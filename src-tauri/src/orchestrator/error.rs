// OpenCode OS — Orchestrator error types (RFC 28 §H).
//
// `SpendLimitError` is the typed envelope for the two failure modes
// that carry a `resets_at` future timestamp:
//
// 1. rate-limit (HTTP 429) — temporary throttling by provider.
// 2. spend-limit (HTTP 402 / 403) — pay-per-use cap exhausted or
//    permission/quota gate tripped; the model is unavailable until
//    the account reloads or the operator raises the cap.
//
// Both share the lifecycle: observed from a provider/omniroute error
// body (or HTTP headers), persisted in the `model_resets` SQLite table
// (M19), and surfaced to the user via a `model_ready` Toast at
// `resets_at`. The struct is **distinct from `anyhow::Error`** because
// it's first-class data, not a fatality: the orchestrator catches it,
// records the reset, marks the in-flight mission as paused, and stops
// auto-retrying (the card's "Request Increase" + "Switch Provider"
// buttons + 5-min localStorage cooldown are downstream of this).
//
// Pattern adapted from Cline (Apache-2.0) PRs:
// * #10207 — SpendLimitError card with `resets_at`, exempted from
//   auto-retry; the user-not-the-loop decides the outcome.
// * #10963 — jitter retry + parse Retry-After / x-ratelimit-reset.
// * #10141 — bail-out when Retry-After > threshold (default 60 s).
// https://github.com/cline/cline

use chrono::{DateTime, Utc};

/// Discriminator between the two reset-bearing failure modes. Mirrors
/// the OmniRoute envelope's `error.type` field (`rate_limit | spend_limit`).
/// When the error is observed from a direct provider hit (no OmniRoute
/// normalization), the parser infers the kind from the HTTP status:
/// 429 → `RateLimit`, 402 / 403 → `SpendLimit`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResetKind {
    RateLimit,
    SpendLimit,
}

impl ResetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RateLimit => "rate_limit",
            Self::SpendLimit => "spend_limit",
        }
    }

    /// Map an HTTP status code to a `ResetKind`. 429 → RateLimit,
    /// 402 | 403 → SpendLimit; any other value returns `None`.
    pub fn from_status_code(status: u16) -> Option<Self> {
        match status {
            429 => Some(Self::RateLimit),
            402 | 403 => Some(Self::SpendLimit),
            _ => None,
        }
    }
}

/// Typed error envelope lifted from a provider / OmniRoute response.
/// Lifetimes are owned so `SpendLimitError` can cross `await` points
/// and survive in `model_resets` row precedent.
#[derive(Clone, Debug, PartialEq)]
pub struct SpendLimitError {
    /// Provider id as known to OpenCode OS: `openai`, `anthropic`,
    /// `omniroute`, `local`, etc.
    pub provider: String,
    /// Model id as known to OpenCode OS: e.g. `claude-3-5-sonnet`.
    pub model: String,
    /// HTTP status observed (`429`, `402`, `403`).
    pub status_code: u16,
    /// Discriminator — `RateLimit` or `SpendLimit`. Inferred from
    /// the envelope when OmniRoute sets `error.type`; from the HTTP
    /// status otherwise (RFC 28 §H.2).
    pub kind: ResetKind,
    /// Absolute moment the provider says the model will be usable
    /// again. Parser guarantees `resets_at >= now()` at observation
    /// time — see `parse_error::parse_omniroute` / `parse_retry_after`
    /// for the source-truth rules.
    pub resets_at: DateTime<Utc>,
    /// Upstream request id (provider-dependent). Kept for forensics
    /// and dedupe against the same response replaying; not used as a
    /// key.
    pub request_id: Option<String>,
    /// Human-readable provider message, surfaced in the HUD card.
    pub message: Option<String>,
}

impl SpendLimitError {
    /// Convenience: number of seconds from `now` until `resets_at`.
    /// Returns `0` when `resets_at` has already passed (caller
    /// decides whether that means "fire the Toast immediately" — see
    /// `scheduler`).
    pub fn secs_until_reset(&self, now: DateTime<Utc>) -> i64 {
        (self.resets_at - now).num_seconds().max(0)
    }

    /// True if `resets_at` is in the past relative to `now` (the
    /// scheduler should fire the `model_ready` Toast immediately
    /// rather than sleep until the timestamp — the resume button is
    /// now actionable).
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.resets_at <= now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn make(resets_at: &str) -> SpendLimitError {
        SpendLimitError {
            provider: "anthropic".into(),
            model: "claude-3-5-sonnet".into(),
            status_code: 429,
            kind: ResetKind::RateLimit,
            resets_at: ts(resets_at),
            request_id: Some("req_abc".into()),
            message: Some("rate limited".into()),
        }
    }

    #[test]
    fn reset_kind_from_status_code_maps_known_codes() {
        assert_eq!(ResetKind::from_status_code(429), Some(ResetKind::RateLimit));
        assert_eq!(
            ResetKind::from_status_code(402),
            Some(ResetKind::SpendLimit)
        );
        assert_eq!(
            ResetKind::from_status_code(403),
            Some(ResetKind::SpendLimit)
        );
        assert_eq!(ResetKind::from_status_code(500), None);
        assert_eq!(ResetKind::from_status_code(200), None);
    }

    #[test]
    fn secs_until_reset_returns_seconds_remaining() {
        let e = make("2026-08-02T12:00:00Z");
        let now = ts("2026-08-02T11:59:30Z");
        assert_eq!(e.secs_until_reset(now), 30);
    }

    #[test]
    fn secs_until_reset_clamps_negative_to_zero() {
        let e = make("2026-08-02T12:00:00Z");
        let now = ts("2026-08-02T12:01:00Z");
        assert_eq!(
            e.secs_until_reset(now),
            0,
            "expired reset should clamp to 0"
        );
        assert!(e.is_expired(now));
    }

    #[test]
    fn is_expired_returns_true_when_resets_at_eq_now() {
        let e = make("2026-08-02T12:00:00Z");
        let now = ts("2026-08-02T12:00:00Z");
        assert!(e.is_expired(now), "boundary: resets_at == now is expired");
    }

    #[test]
    fn build_from_direct_provider_status_inference() {
        let kind = ResetKind::from_status_code(402).unwrap();
        let e = SpendLimitError {
            provider: "openai".into(),
            model: "gpt-5".into(),
            status_code: 402,
            kind,
            resets_at: ts("2026-08-02T13:00:00Z"),
            request_id: None,
            message: None,
        };
        assert_eq!(e.kind, ResetKind::SpendLimit);
    }
}
