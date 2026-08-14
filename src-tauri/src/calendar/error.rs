// Atlas OS — Calendar facade errors (RFC 28 Section G).
//
// `CalendarError` is the single error type surfaced out of the
// `crate::calendar` module. Every fallible call (queue CRUD, ICS
// serialisation, MS Graph fetch / parse, encrypted token storage,
// deep-link URL building) maps its underlying error into one of the
// variants below so consumers (the axum HUD route, the CLI
// `opencode calendar` subcommand, the AppState boot sequence, the
// Planning engine's busy-window lookup) don't have to know about
// `ics::Error`, `rusqlite::Error`, `graph_rs_sdk::error::Error`,
// `aes_gcm::Error`, or `chrono::ParseError` individually.
//
// Uses:
// * `ics 0.5.8` (MIT OR Apache-2.0) by hummingly.
//   https://github.com/hummingly/ics
// * `graph-rs-sdk 3.0.1` (MIT) by sreeise.
//   https://github.com/sreeise/graph-rs-sdk
// * `aes-gcm 0.10` (MIT OR Apache-2.0) for token-at-rest encryption.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CalendarError {
    #[error("journal SQLite error: {0}")]
    Journal(#[from] rusqlite::Error),

    #[error("backend error: {0}")]
    Backend(#[from] anyhow::Error),

    #[error("iCalendar serialisation failed: {0}")]
    Ics(String),

    #[error("Microsoft Graph request failed: {0}")]
    Graph(String),

    #[error("Microsoft Graph auth failed: {0}")]
    Auth(String),

    #[error("refresh token decode failed: {0}")]
    TokenDecode(String),

    #[error("token encryption failed: {0}")]
    Encrypt(String),

    #[error("invalid busy-window id `{0}` — not found")]
    NotFound(i64),

    #[error("invalid busy source `{0}` — expected graph|ics_local|manual")]
    InvalidSource(String),

    #[error("invalid time range — starts_at ({starts}) must be < ends_at ({ends})")]
    InvalidRange { starts: i64, ends: i64 },

    #[error("platform unsupported — the `calendar-graph` feature is desktop-only")]
    UnsupportedPlatform,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CalendarError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_carries_underlying_message() {
        let e = CalendarError::Ics("fold overflow".to_string());
        assert!(format!("{e}").contains("fold overflow"));
    }

    #[test]
    fn journal_variant_from_underlying() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let raw = conn
            .prepare("SELECT * FROM definitely_not_a_table")
            .err()
            .unwrap();
        let lifted = CalendarError::from(raw);
        assert!(matches!(lifted, CalendarError::Journal(_)));
        assert!(format!("{lifted}").contains("definitely_not_a_table"));
    }

    #[test]
    fn invalid_range_carries_both_timestamps() {
        let e = CalendarError::InvalidRange {
            starts: 200,
            ends: 100,
        };
        let msg = format!("{e}");
        assert!(msg.contains("200"));
        assert!(msg.contains("100"));
    }
}
