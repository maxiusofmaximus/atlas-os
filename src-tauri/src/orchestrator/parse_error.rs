// Atlas OS — OmniRoute envelope + HTTP header parser (RFC 28 §H.2 / §H.5).
//
// Translates the three known sources of a reset-window hint into the
// single in-memory `SpendLimitError` envelope used downstream by the
// orchestrator:
//
// 1. **OmniRoute envelope** — JSON body returned by OmniRoute gateway
//    (the OpenAI-compatible endpoint Atlas OS uses) when an
//    upstream provider rate-limits or spend-limits a request. The
//    envelope normalizes upstream-specific headers into a single
//    `error.resets_at` RFC 3339 timestamp.
//
// 2. **`Retry-After`** — HTTP response header, either a delay (the
//    number of seconds until the resource is usable) or an absolute
//    RFC 1123 date. Defined by RFC 7231 §7.1.3; every HTTP/1.1 client
//    already speaks it. Provider-side distinction:
//    - OpenAI returns no `Retry-After` on 429 (use
//      `x-ratelimit-reset-requests` / `x-ratelimit-reset-tokens`).
//    - Anthropic returns `Retry-After` as a delay-seconds on 429.
//    - OmniRoute surfaces it as the envelope's `resets_at` field.
//
// 3. **`x-ratelimit-reset-requests` / `x-ratelimit-reset-tokens`** —
//    OpenAI-specific headers (delay in seconds). Parsed only when
//    OmniRoute is bypassed (`direct_provider` mode in `Profile`).
//
// The parser is strict — a malformed envelope / header returns
// `ParseError::Malformed` and the caller falls back to a default
// `RetryPolicy` exponential-backoff delay (no Toast fires). This
// mirrors Cline #10963's "no-parse, no-Toast" fallback.
//
// Pattern adapted from Cline (Apache-2.0) PR #10963
// https://github.com/cline/cline/pull/10963 — jitter ±25% retry
// middleware syntax retained, but jitter lives in `retry.rs` to keep
// the parser deterministic.

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use serde::Deserialize;

use crate::orchestrator::error::{ResetKind, SpendLimitError};

/// Error returned by every parser in this module. The parser surface
/// is small and the caller can match on these to decide whether to
/// fallback to default retry behavior (`Malformed`) or surface an
/// `Unauthorised` (401/`invalid_grant` refresh failure) to the user.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("malformed body or header: {0}")]
    Malformed(String),
    #[error("parse of timestamp failed: {0}")]
    BadTimestamp(String),
    #[error("header value not a valid number: {0}")]
    BadNumeric(String),
    #[error("missing required field in OmniRoute envelope: {0}")]
    Missing(&'static str),
}

/// Typed view of the OmniRoute error envelope (`RFC 28 §H.2`).
/// Only the fields Atlas OS consults are deserialized; the rest
/// pass through `#[serde(other)]` of the `kind` enum intact.
#[derive(Debug, Deserialize)]
pub struct OmniRouteEnvelope {
    pub error: OmniRouteErrorBody,
}

#[derive(Debug, Deserialize)]
pub struct OmniRouteErrorBody {
    /// `"rate_limit" | "spend_limit"`. Other tokens (e.g.
    /// `"invalid_request"`) signal a non-reset error: the parser
    /// returns `ParseError::Malformed` for unknown kinds so the
    /// orchestrator falls through to default retry.
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub message: Option<String>,
    pub status: Option<u16>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub resets_at: Option<String>,
    pub request_id: Option<String>,
}

/// Parse the OmniRoute JSON envelope body. Returns `Ok(Some(...))`
/// when the envelope carries a `resets_at` future timestamp,
/// `Ok(None)` when the envelope doesn't carry a reset (e.g. a 5xx
/// propagate or a malformed 4xx the gateway opted to surface), and
/// `Err(ParseError)` when the JSON is unparseable or the timestamp
/// doesn't round-trip — that's an upgrade mismatch between OmniRoute
/// and Atlas OS, and the caller should log it + fall back.
pub fn parse_omniroute(body: &[u8]) -> Result<Option<SpendLimitError>, ParseError> {
    let env: OmniRouteEnvelope = serde_json::from_slice(body)
        .map_err(|e| ParseError::Malformed(format!("json decode: {e}")))?;

    let Some(resets_at_str) = env.error.resets_at.as_deref() else {
        return Ok(None);
    };
    let resets_at = parse_rfc3339(resets_at_str)?;

    let kind = match env.error.kind.as_deref() {
        Some("rate_limit") => ResetKind::RateLimit,
        Some("spend_limit") => ResetKind::SpendLimit,
        Some(other) => {
            return Err(ParseError::Malformed(format!(
                "unknown error.type: {other}"
            )));
        }
        None => {
            // No kind on the envelope: infer from status; if no
            // status either, assume rate_limit (the safer default —
            // SpendLimit buttons route to the dashboard which on a
            // misclassified rate limit is a no-op annoyance).
            match env.error.status {
                Some(402) | Some(403) => ResetKind::SpendLimit,
                _ => ResetKind::RateLimit,
            }
        }
    };

    let status = env.error.status.unwrap_or(match kind {
        ResetKind::RateLimit => 429,
        ResetKind::SpendLimit => 402,
    });

    let provider = env.error.provider.ok_or(ParseError::Missing("provider"))?;
    let model = env.error.model.ok_or(ParseError::Missing("model"))?;

    // Stale-reset guard (RFC 28 §H.8 risk 2): if `resets_at` is more
    // than 60 s in the past, the envelope was a replay or our clock
    // is skewed — caller should not enqueue a Toast. Surface as
    // `Ok(None)` so the orchestrator treats it as "no reset info";
    // the upper layer (scheduler) decides what to do.
    let now = Utc::now();
    if resets_at < now - Duration::seconds(60) {
        return Ok(None);
    }

    Ok(Some(SpendLimitError {
        provider,
        model,
        status_code: status,
        kind,
        resets_at,
        request_id: env.error.request_id,
        message: env.error.message,
    }))
}

/// Parse a `Retry-After` HTTP header (RFC 7231 §7.1.3). Two forms:
/// - delta-seconds (integer): `Retry-After: 120`
/// - HTTP-date (RFC 1123): `Retry-After: Wed, 03 Aug 2026 12:34:56 GMT`
///
/// The numeric form is the common case for rate-limits; the date
/// form is more typical for "Retry-After: <date>" maintenance windows.
/// `now` is passed explicitly so tests are deterministic.
pub fn parse_retry_after(header: &str, now: DateTime<Utc>) -> Result<DateTime<Utc>, ParseError> {
    let trimmed = header.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Malformed("empty Retry-After".into()));
    }
    if let Ok(secs) = trimmed.parse::<i64>() {
        if secs < 0 {
            return Err(ParseError::BadNumeric(format!(
                "negative Retry-After: {secs}"
            )));
        }
        return Ok(now + Duration::seconds(secs));
    }
    parse_rfc1123(trimmed, now)
}

/// Parse `x-ratelimit-reset-requests` / `x-ratelimit-reset-tokens`
/// (OpenAI-specific). The header value is always a delay in seconds
/// (may be fractional in newer APIs, but historically an integer).
/// `now` is explicit for deterministic tests.
pub fn parse_x_ratelimit_reset(
    header: &str,
    now: DateTime<Utc>,
) -> Result<DateTime<Utc>, ParseError> {
    let trimmed = header.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Malformed("empty x-ratelimit-reset".into()));
    }
    let secs: f64 = trimmed
        .parse()
        .map_err(|e| ParseError::BadNumeric(format!("x-ratelimit-reset `{trimmed}`: {e}")))?;
    if secs < 0.0 {
        return Err(ParseError::BadNumeric(format!(
            "negative x-ratelimit-reset: {secs}"
        )));
    }
    let whole_secs = secs.trunc() as i64;
    Ok(now + Duration::seconds(whole_secs))
}

fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, ParseError> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| ParseError::BadTimestamp(format!("rfc3339 `{s}`: {e}")))
}

/// Parse an RFC 1123 date string as used in HTTP headers
/// (`Wed, 03 Aug 2026 12:34:56 GMT`). chrono's `DateTime::parse_from_rfc2822`
/// accepts this format directly. `now` is unused here but reserved for
/// boundary rounding (treat "now" as 0-second delay).
fn parse_rfc1123(s: &str, _now: DateTime<Utc>) -> Result<DateTime<Utc>, ParseError> {
    // RFC 2822 is a superset that accepts the structured form used
    // by RFC 1123 dates in HTTP headers.
    DateTime::parse_from_rfc2822(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| ParseError::BadTimestamp(format!("rfc2822 `{s}`: {e}")))
}

/// Convenience used by tests: parse an RFC 1123 date without dragging
/// `chrono::NaiveDateTime` into the public surface. Not exported.
#[allow(dead_code)]
fn _parse_naive(s: &str) -> Result<NaiveDateTime, ParseError> {
    NaiveDateTime::parse_from_str(s, "%a, %d %b %Y %H:%M:%S")
        .map_err(|e| ParseError::BadTimestamp(format!("naive `{s}`: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn in_n_minutes(mins: i64) -> String {
        let t = Utc::now() + Duration::minutes(mins);
        t.to_rfc3339()
    }

    #[test]
    fn omniroute_envelope_with_resets_at_and_full_metadata() {
        let resets = in_n_minutes(5);
        let body = format!(
            r#"{{
            "error": {{
                "type": "rate_limit",
                "message": "Too many requests",
                "status": 429,
                "provider": "anthropic",
                "model": "claude-3-5-sonnet",
                "resets_at": "{resets}",
                "request_id": "req_abc"
            }}
        }}"#
        );
        let parsed = parse_omniroute(body.as_bytes()).expect("well-formed envelope should parse");
        let e = parsed.expect("envelope carries resets_at");
        assert_eq!(e.provider, "anthropic");
        assert_eq!(e.model, "claude-3-5-sonnet");
        assert_eq!(e.status_code, 429);
        assert_eq!(e.kind, ResetKind::RateLimit);
        assert_eq!(e.resets_at, ts(&resets));
        assert_eq!(e.request_id.as_deref(), Some("req_abc"));
        assert_eq!(e.message.as_deref(), Some("Too many requests"));
    }

    #[test]
    fn omniroute_envelope_with_spend_limit_kind_maps_to_spend_limit() {
        let resets = in_n_minutes(10);
        let body = format!(
            r#"{{
            "error": {{
                "type": "spend_limit",
                "status": 402,
                "provider": "openai",
                "model": "gpt-5",
                "resets_at": "{resets}"
            }}
        }}"#
        );
        let parsed = parse_omniroute(body.as_bytes()).unwrap().unwrap();
        assert_eq!(parsed.kind, ResetKind::SpendLimit);
        assert_eq!(parsed.status_code, 402);
    }

    #[test]
    fn omniroute_envelope_with_no_kind_infers_from_status() {
        let resets = in_n_minutes(15);
        let body = format!(
            r#"{{"error":{{"status":403,"provider":"openai","model":"gpt-5","resets_at":"{resets}"}}}}"#
        );
        let parsed = parse_omniroute(body.as_bytes()).unwrap().unwrap();
        assert_eq!(parsed.kind, ResetKind::SpendLimit);
        assert_eq!(parsed.status_code, 403);
    }

    #[test]
    fn omniroute_envelope_with_stale_resets_at_returns_ok_none() {
        let body = br#"{"error":{"type":"rate_limit","status":429,"provider":"anthropic","model":"claude-3-5-sonnet","resets_at":"2020-01-01T00:00:00Z"}}"#;
        let parsed = parse_omniroute(body).expect("should parse");
        assert!(parsed.is_none(), "stale resets_at should yield Ok(None)");
    }

    #[test]
    fn omniroute_envelope_without_resets_at_returns_ok_none() {
        let body = br#"{"error":{"type":"invalid_request","status":400,"message":"bad"}}"#;
        let parsed = parse_omniroute(body).expect("should parse");
        assert!(parsed.is_none(), "no resets_at should yield Ok(None)");
    }

    #[test]
    fn retry_after_numeric_seconds_form() {
        let now = ts("2026-08-03T12:00:00Z");
        let r = parse_retry_after("120", now).unwrap();
        assert_eq!(r, ts("2026-08-03T12:02:00Z"));
    }

    #[test]
    fn retry_after_rfc1123_date_form() {
        let now = ts("2026-08-03T12:00:00Z");
        // 2026-08-03 is a Monday; RFC 2822 refuses mismatched day-of-week.
        let r = parse_retry_after("Mon, 03 Aug 2026 12:05:00 GMT", now).unwrap();
        assert_eq!(r, ts("2026-08-03T12:05:00Z"));
    }

    #[test]
    fn retry_after_negative_seconds_returns_err() {
        let now = ts("2026-08-03T12:00:00Z");
        let err = parse_retry_after("-30", now).unwrap_err();
        assert!(matches!(err, ParseError::BadNumeric(_)));
    }

    #[test]
    fn x_ratelimit_reset_accepts_integer_seconds() {
        let now = ts("2026-08-03T12:00:00Z");
        let r = parse_x_ratelimit_reset("900", now).unwrap();
        assert_eq!(r, ts("2026-08-03T12:15:00Z"));
    }

    #[test]
    fn x_ratelimit_reset_accepts_fractional_seconds() {
        let now = ts("2026-08-03T12:00:00Z");
        let r = parse_x_ratelimit_reset("1.5", now).unwrap();
        // 1.5 s truncates to 1 second (item-level delay).
        assert_eq!(r, ts("2026-08-03T12:00:01Z"));
    }

    #[test]
    fn x_ratelimit_reset_negative_returns_err() {
        let now = ts("2026-08-03T12:00:00Z");
        let err = parse_x_ratelimit_reset("-1", now).unwrap_err();
        assert!(matches!(err, ParseError::BadNumeric(_)));
    }

    #[test]
    fn rfc3339_parser_rejects_malformed() {
        let err = parse_rfc3339("2026-13-99T99").unwrap_err();
        assert!(matches!(err, ParseError::BadTimestamp(_)));
    }

    #[test]
    fn unknown_envelope_kind_treated_as_malformed() {
        let body = br#"{"error":{"type":"unknown","status":500,"provider":"x","model":"y","resets_at":"2026-08-03T12:00:00Z"}}"#;
        let err = parse_omniroute(body).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(_)));
    }

    #[test]
    fn envelope_missing_required_fields_returns_err() {
        // No provider/model.
        let body = br#"{"error":{"resets_at":"2026-08-03T12:00:00Z"}}"#;
        let err = parse_omniroute(body).unwrap_err();
        assert!(matches!(err, ParseError::Missing(_)));
    }
}
