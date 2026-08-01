// OpenCode OS — Firecrawl facade errors (RFC 28 Section E).
//
// `firecrawl::FirecrawlError` is the SDK's transport-level enum. We
// lift it into a stable in-process enum so callers (CLI `research`,
// future graph_ingest step, future orchestrator `webfetch` fallback,
// future MCP server) program against a fixed surface even if the SDK
// adds new variants or splits `HttpError`/`HttpRequestFailed` (it
// already carries `action`/`reqwest::Error` redundantly, so the SDK
// is likely to merge them in a future minor). Stable facade = no
// breaking-change ripple in OpenCode OS consumers.
//
// `thiserror::Error` keeps `Display` and `From` impls derived so
// consumers can `?` up the stack, including from `anyhow::Result`.

use thiserror::Error;

/// Error returned by every `firecrawl::facade` call. Translates the
/// SDK's enum into OpenCode-OS-shaped variants; nothing here leaks a
/// `reqwest::Error` or other transient SDK type to the caller.
#[derive(Debug, Error)]
pub enum FirecrawlFacadeError {
    /// No `FIRECRAWL_API_KEY` env var and no self-hosted URL either, so
    /// the keyless cloud endpoint would be used. Callers MAY downgrade
    /// to the keyless free tier (`scrape`/`search` only, rate-limited),
    /// or surface this to the operator as "configure a key".
    #[error("firecrawl key missing: set FIRECRAWL_API_KEY or OPENCODE_FIRECRAWL_URL to use this surface")]
    MissingApiKey,

    /// The caller-supplied URL failed `url::Url::parse`. Isolation from
    /// reqwest lets us classify the failure before the HTTP layer.
    #[error("invalid url: {0}")]
    InvalidUrl(String),

    /// The SDK returned an HTTP-layer error (network, TLS, DNS, etc.).
    /// Preserves the human-readable `action` string from the SDK so
    /// logs line up.
    #[error("network error during {action}: {source}")]
    Network {
        action: String,
        source: firecrawl::FirecrawlError,
    },

    /// The API surfaced an explicit failure envelope (status != 2xx
    /// with `success: false`). Often a keyless-tier rate-limit (429)
    /// or a missing-key rejection (401/402/403). Discernible via the
    /// `status_code`/`error_type` fields for the orchestrator's retry
    /// policy (RFC 28 §H may consume variants like `RateLimited`).
    #[error("api error during {action}: {source}")]
    Api {
        action: String,
        source: firecrawl::FirecrawlError,
    },

    /// Background job (`crawl`/`batch-scrape`) reported `cancelled`
    /// or `failed`. The SDK returns an empty `data` vec in that case;
    /// we turn it into an explicit error so callers don't silently
    /// receive zero docs when a crawl die mid-flight.
    #[error("job failed: {0}")]
    JobFailed(firecrawl::FirecrawlError),

    /// The SDK refused to build (`Client::new`) for a reason not
    /// covered above (e.g. URL normalisation panic, future SDK misuse
    /// variants). Bubble it through so the operator sees the cause.
    #[error("firecrawl client build failed: {0}")]
    ClientBuild(firecrawl::FirecrawlError),

    /// Decomposing or redacting the SDK's structured response failed.
    /// Treat as a parse path failure, not a network failure.
    #[error("response parse error: {0}")]
    Parse(#[from] serde_json::Error),
}

/// Helper used by the facade to bucket any `firecrawl::FirecrawlError`
/// into one of our variants. Centralised so each `facade` method
/// doesn't reimplement the match arms.
pub(crate) fn lift_sdk_error(
    action: impl Into<String>,
    e: firecrawl::FirecrawlError,
) -> FirecrawlFacadeError {
    use firecrawl::FirecrawlError as S;
    let action = action.into();
    match e {
        S::HttpRequestFailed(_, _, _)
        | S::HttpError(_, _)
        | S::ResponseParseError(_)
        | S::ResponseParseErrorText(_) => FirecrawlFacadeError::Network { action, source: e },
        S::APIError(_, _) => FirecrawlFacadeError::Api { action, source: e },
        S::JobFailed(_, _) => FirecrawlFacadeError::JobFailed(e),
        S::Misuse(_) => FirecrawlFacadeError::ClientBuild(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_api_key_display_is_human_friendly() {
        let e = FirecrawlFacadeError::MissingApiKey;
        assert!(format!("{e}").contains("FIRECRAWL_API_KEY"));
    }

    #[test]
    fn invalid_url_display_carries_the_url() {
        let e = FirecrawlFacadeError::InvalidUrl("ht!tp://bad".into());
        assert!(format!("{e}").contains("ht!tp://bad"));
    }

    #[test]
    fn from_serde_json_error_yields_parse_variant() {
        let json_err = serde_json::from_str::<serde_json::Value>("bad").unwrap_err();
        let e: FirecrawlFacadeError = json_err.into();
        assert!(matches!(e, FirecrawlFacadeError::Parse(_)));
        assert!(format!("{e}").to_lowercase().contains("parse"));
    }
}
