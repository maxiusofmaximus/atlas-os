// Atlas OS — Toast facade errors (RFC 28 Section F).
//
// `ToastFacadeError` is the single error type surfaced out of the
// `crate::toast` module. Every fallible call (queue CRUD, AUMID
// registration, WinRT dispatch, deep-link parsing) maps its underlying
// error into one of the variants below so consumers (the scheduler
// driver, the CLI `atlas toast` subcommand, the AppState boot
// sequence) don't have to know about `winrt_toast_reborn::Error`,
// `rusqlite::Error`, or `chrono::ParseError` individually.
//
// Uses `winrt-toast-reborn = "0.3.8"` (MIT) by Md. Iftakhar Awal
// Chowdhury (AtifChy), fork maintained of winrt-toast 0.1.1.
// https://github.com/AtifChy/winrt-toast

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToastFacadeError {
    #[error("AUMID registration failed: {0}")]
    AumidRegister(String),

    #[error("journal SQLite error: {0}")]
    Journal(#[from] rusqlite::Error),

    #[error("backend error: {0}")]
    Backend(#[from] anyhow::Error),

    #[error("WinRT toast dispatch failed: {0}")]
    Dispatch(String),

    #[error("invalid deep-link `{0}` — expected `opencode://...`")]
    InvalidDeepLink(String),

    #[error("toast kind `{0}` is not recognised")]
    InvalidKind(String),

    #[error("scheduler driver already running")]
    DriverAlreadyRunning,

    #[error("scheduler driver not running")]
    DriverNotRunning,

    #[error("platform unsupported — the `toast` feature is Windows-only")]
    UnsupportedPlatform,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ToastFacadeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_carries_underlying_message() {
        let e = ToastFacadeError::AumidRegister("no Start Menu access".into());
        assert!(format!("{e}").contains("Start Menu"));
    }

    #[test]
    fn journal_variant_from_underlying() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let raw = conn
            .prepare("SELECT * FROM definitely_not_a_table")
            .err()
            .unwrap();
        let lifted = ToastFacadeError::from(raw);
        assert!(matches!(lifted, ToastFacadeError::Journal(_)));
        assert!(format!("{lifted}").contains("definitely_not_a_table"));
    }

    #[test]
    fn invalid_deep_link_message_keeps_the_input() {
        let e = ToastFacadeError::InvalidDeepLink("https://example.com".into());
        assert!(format!("{e}").contains("https://example.com"));
    }
}
