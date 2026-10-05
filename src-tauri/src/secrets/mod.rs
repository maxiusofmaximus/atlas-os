// Atlas OS — OS keychain secret store (RFC 25 §3.10, RFC 18).
//
// Provider API keys and other secrets live in the **OS credential store**
// (Windows Credential Manager / macOS Keychain / Linux Secret Service), keyed
// by `(service="OpenCodeOS", account=<slot>)`. They are never written to
// `.env`, `journal.db`, or logs (RFC 18 §"Secrets").
//
// The `account` is the *key slot* name — by convention the provider's
// `api_key_env` name (e.g. `OPENAI_API_KEY`), so the orchestrator can resolve
// it transparently. The orchestrator reads env vars first and falls back here
// (`orchestrator::client::resolve_api_key`), so existing env-var setups keep
// working unchanged.
//
// No new crate: `keyring` is already a workspace dependency (its per-OS
// native backend is selected in `Cargo.toml` target sections).

use keyring::Entry;

/// Keychain service namespace. Fixed so secrets are namespaced to Atlas OS.
pub const SERVICE: &str = "OpenCodeOS";

#[derive(Debug, thiserror::Error)]
pub enum SecretsError {
    /// The underlying credential store refused the operation.
    #[error("secure storage error: {0}")]
    Keyring(String),
    /// Account or value was blank.
    #[error("{0} must not be empty")]
    Empty(&'static str),
}

fn entry(account: &str) -> Result<Entry, SecretsError> {
    let account = account.trim();
    if account.is_empty() {
        return Err(SecretsError::Empty("account"));
    }
    Entry::new(SERVICE, account).map_err(|e| SecretsError::Keyring(e.to_string()))
}

/// Store (or replace) a secret under `account`.
pub fn set(account: &str, value: &str) -> Result<(), SecretsError> {
    if value.is_empty() {
        return Err(SecretsError::Empty("secret value"));
    }
    entry(account)?
        .set_password(value)
        .map_err(|e| SecretsError::Keyring(e.to_string()))
}

/// Read a secret, or `None` when the account has no entry.
pub fn get(account: &str) -> Result<Option<String>, SecretsError> {
    match entry(account)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(SecretsError::Keyring(e.to_string())),
    }
}

/// Delete a secret. `false` when there was nothing to delete.
pub fn delete(account: &str) -> Result<bool, SecretsError> {
    match entry(account)?.delete_credential() {
        Ok(()) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(SecretsError::Keyring(e.to_string())),
    }
}

/// Mask a secret for display: keep a short prefix, hide the rest. Never
/// reveals enough to reconstruct the key (RFC 18 — no secret in logs/output).
pub fn preview(value: &str) -> String {
    let n = value.chars().count();
    if n <= 4 {
        return "*".repeat(n.max(1));
    }
    let head: String = value.chars().take(4).collect();
    format!("{head}{}", "*".repeat((n - 4).min(12)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A random, definitely-absent account: lets the unit tests exercise the
    /// keyring `NoEntry` path read-only, never writing to the real OS keychain.
    /// The write/roundtrip path is covered by `tools/secrets-smoke.ps1` (the
    /// repo's pattern for OS-integration surfaces).
    fn missing_account() -> String {
        format!("atlas-absent-{}", uuid::Uuid::new_v4())
    }

    #[test]
    fn get_missing_returns_none() {
        assert!(matches!(get(&missing_account()), Ok(None)));
    }

    #[test]
    fn delete_missing_is_a_noop() {
        assert!(!delete(&missing_account()).unwrap());
    }

    #[test]
    fn empty_account_or_value_is_rejected_before_touching_the_store() {
        assert!(matches!(set("", "x"), Err(SecretsError::Empty("account"))));
        assert!(matches!(
            set("acct", ""),
            Err(SecretsError::Empty("secret value"))
        ));
        assert!(matches!(get("   "), Err(SecretsError::Empty("account"))));
    }

    #[test]
    fn preview_hides_the_tail() {
        assert_eq!(preview("sk-1234567890"), "sk-1*********");
        assert_eq!(preview("abc"), "***");
        assert_eq!(preview(""), "*");
    }
}
