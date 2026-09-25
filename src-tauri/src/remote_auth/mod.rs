// Atlas OS — Command Center web remoto: auth propia mínima (RFC 20 Phase 8
// sub-fase 8.3, research 36 SECTOR B 8.3 + A.3, RFC 24 §16, RFC 17 §6).
//
// Audit decision (RFC 25 §11, AGENTS.md §4): `axum-oidc-layer` +
// `openidconnect` are DEFERRED — no new crates in this sub-fase.
// `openidconnect` drags the `oauth2` + blocking `reqwest` surface and a
// wide transitive tree onto the single-binary budget; `axum-oidc-layer`
// is a young API wrapper over the same stack. The LAN-first threat model
// (RFC 24 §16, RFC 25 topology: remote reaches the HUD over an SSH
// tunnel / Tailscale / Cloudflare tunnel that is already authenticated
// at transport) only needs an owned bearer gate plus the OIDC discovery
// shape fixed here, so a future `remote-ui` feature (default off) can
// swap the real OIDC layer without changing callers. Session cookies
// (`SESSION_COOKIE`) ride the same token: the cookie value IS the bearer.
// Poll cadence 5s is enough for the MVP (research 36 SECTOR C).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Env var carrying the remote bearer token (operator override; beats the
/// `remote_token.txt` file persisted per profile).
pub const REMOTE_TOKEN_ENV: &str = "ATLAS_REMOTE_TOKEN";
/// File in the profile root persisting the bearer token (same pattern as
/// `hud_port.txt`: the headless CLI has no AppState to ask).
pub const REMOTE_TOKEN_FILE: &str = "remote_token.txt";
/// Env var with the OIDC issuer URL (e.g. `https://accounts.google.com`).
/// When set (plus `OIDC_CLIENT_ID_ENV`), `/remote/status` reports
/// `oidc_configured: true` and the future OIDC layer knows where to
/// discover from. No network call happens in this sub-fase.
pub const OIDC_ISSUER_ENV: &str = "ATLAS_OIDC_ISSUER";
/// Env var with the OIDC client id registered at the issuer.
pub const OIDC_CLIENT_ID_ENV: &str = "ATLAS_OIDC_CLIENT_ID";
/// Session cookie name binding the browser to the bearer token.
pub const SESSION_COOKIE: &str = "atlas_session";
/// Always-mounted informational route on the HUD axum server.
pub const REMOTE_STATUS_ROUTE: &str = "/remote/status";
/// End-to-end UI latency target shared with the Phase 8 entregable.
pub const REMOTE_AUTH_LATENCY_TARGET_MS: u64 = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteAuthConfig {
    pub token_configured: bool,
    pub oidc_issuer: Option<String>,
    pub oidc_client_id: Option<String>,
}

impl RemoteAuthConfig {
    pub fn from_env_with_token(token_configured: bool) -> Self {
        let oidc_issuer = std::env::var(OIDC_ISSUER_ENV)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let oidc_client_id = std::env::var(OIDC_CLIENT_ID_ENV)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        Self {
            token_configured,
            oidc_issuer,
            oidc_client_id,
        }
    }

    pub fn oidc_configured(&self) -> bool {
        self.oidc_issuer.is_some() && self.oidc_client_id.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteAccessStatus {
    pub local_only: bool,
    pub token_configured: bool,
    pub oidc_configured: bool,
    pub oidc_issuer: Option<String>,
    pub hud_port: Option<u16>,
    pub latency_target_ms: u64,
}

pub fn status_snapshot(config: &RemoteAuthConfig, hud_port: Option<u16>) -> RemoteAccessStatus {
    RemoteAccessStatus {
        local_only: true,
        token_configured: config.token_configured,
        oidc_configured: config.oidc_configured(),
        oidc_issuer: config.oidc_issuer.clone(),
        hud_port: hud_port.filter(|p| *p != 0),
        latency_target_ms: REMOTE_AUTH_LATENCY_TARGET_MS,
    }
}

/// OIDC discovery document URL for an issuer (RFC 8414 §3 / OIDC
/// Discovery §4). Pure string shaping — no network. Returns `None` for
/// blank input so callers fail safe to bearer-only.
pub fn discovery_url(issuer: &str) -> Option<String> {
    let base = issuer.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        return None;
    }
    Some(format!("{base}/.well-known/openid-configuration"))
}

pub fn token_path(profile_root: &Path) -> PathBuf {
    profile_root.join(REMOTE_TOKEN_FILE)
}

pub fn read_token(profile_root: &Path) -> Option<String> {
    if let Ok(env) = std::env::var(REMOTE_TOKEN_ENV) {
        let env = env.trim().to_string();
        if !env.is_empty() {
            return Some(env);
        }
    }
    std::fs::read_to_string(token_path(profile_root))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Return the persisted bearer token, generating and persisting one when
/// absent. Generation rides `Uuid::new_v4` (already a direct dep via the
/// Kernel Bus) — no new randomness crate.
pub fn ensure_token(profile_root: &Path) -> std::io::Result<String> {
    if let Some(t) = read_token(profile_root) {
        return Ok(t);
    }
    let token = uuid::Uuid::new_v4().simple().to_string();
    std::fs::write(token_path(profile_root), &token)?;
    Ok(token)
}

/// Unconditionally generate + persist a fresh token (rotation after a
/// suspected leak). Never prints the old value.
pub fn rotate_token(profile_root: &Path) -> std::io::Result<String> {
    let token = uuid::Uuid::new_v4().simple().to_string();
    std::fs::write(token_path(profile_root), &token)?;
    Ok(token)
}

/// Pull the raw token out of an `Authorization` header value
/// (`Bearer <token>`, scheme case-insensitive). Returns `None` for any
/// other scheme or blank input.
pub fn extract_bearer(header_value: &str) -> Option<&str> {
    let (scheme, token) = header_value.trim().split_once(char::is_whitespace)?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    Some(token)
}

/// Bearer check with a length + xor-fold comparison so a wrong token of
/// the right length does not early-exit on the first byte. Empty expected
/// never verifies: without a configured token every remote call is 401.
pub fn verify_bearer(header_value: Option<&str>, expected: &str) -> bool {
    if expected.is_empty() {
        return false;
    }
    let presented = header_value.and_then(extract_bearer).unwrap_or("");
    if presented.len() != expected.len() {
        return false;
    }
    presented
        .bytes()
        .zip(expected.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// Pull the raw bearer out of an HTTP request: `Authorization: Bearer`
/// wins, otherwise the `atlas_session` session cookie. Pure string work
/// so the axum route stays a thin shell over it.
pub fn extract_request_token(authorization: Option<&str>, cookie: Option<&str>) -> Option<String> {
    if let Some(token) = authorization.and_then(extract_bearer) {
        return Some(token.to_string());
    }
    let jar = cookie?;
    for pair in jar.split(';') {
        let mut kv = pair.trim().splitn(2, '=');
        if kv.next()?.trim() == SESSION_COOKIE {
            let value = kv.next().unwrap_or("").trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Safe token preview for CLI output (`abcd…wxyz`): proves which token is
/// active without leaking it into shells, logs, or screenshots.
pub fn redacted_preview(token: &str) -> String {
    if token.len() <= 8 {
        return "…".to_string();
    }
    format!("{}…{}", &token[..4], &token[token.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_clean_env<F: FnOnce()>(f: F) {
        let _g = ENV_LOCK.lock().unwrap();
        let token_b = std::env::var_os(REMOTE_TOKEN_ENV);
        let iss_b = std::env::var_os(OIDC_ISSUER_ENV);
        let cid_b = std::env::var_os(OIDC_CLIENT_ID_ENV);
        std::env::remove_var(REMOTE_TOKEN_ENV);
        std::env::remove_var(OIDC_ISSUER_ENV);
        std::env::remove_var(OIDC_CLIENT_ID_ENV);
        f();
        restore(REMOTE_TOKEN_ENV, token_b);
        restore(OIDC_ISSUER_ENV, iss_b);
        restore(OIDC_CLIENT_ID_ENV, cid_b);
    }

    fn restore(key: &str, val: Option<std::ffi::OsString>) {
        if let Some(v) = val {
            std::env::set_var(key, v);
        } else {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn bearer_accepts_exact_token_case_insensitive_scheme() {
        assert!(verify_bearer(Some("Bearer secret123"), "secret123"));
        assert!(verify_bearer(Some("bearer secret123"), "secret123"));
        assert!(verify_bearer(Some("BEARER secret123"), "secret123"));
        assert!(verify_bearer(Some("  Bearer   secret123  "), "secret123"));
    }

    #[test]
    fn bearer_rejects_wrong_missing_and_unconfigured() {
        assert!(!verify_bearer(Some("Bearer wrong"), "secret123"));
        assert!(!verify_bearer(Some("Bearer secrets"), "secret123"));
        assert!(!verify_bearer(Some("Basic c2VjcmV0"), "secret123"));
        assert!(!verify_bearer(Some("Bearer "), "secret123"));
        assert!(!verify_bearer(None, "secret123"));
        assert!(!verify_bearer(Some("Bearer secret123"), ""));
        assert!(!verify_bearer(None, ""));
    }

    #[test]
    fn discovery_shapes_url_and_rejects_blank() {
        assert_eq!(
            discovery_url("https://accounts.google.com"),
            Some("https://accounts.google.com/.well-known/openid-configuration".to_string())
        );
        assert_eq!(
            discovery_url("https://issuer.example/oidc/"),
            Some("https://issuer.example/oidc/.well-known/openid-configuration".to_string())
        );
        assert_eq!(discovery_url("   "), None);
        assert_eq!(discovery_url(""), None);
    }

    #[test]
    fn token_file_round_trips_and_rotation_changes_it() {
        with_clean_env(|| {
            let dir = tempfile::TempDir::new().unwrap();
            assert_eq!(read_token(dir.path()), None);
            let first = ensure_token(dir.path()).unwrap();
            assert!(!first.is_empty());
            assert_eq!(ensure_token(dir.path()).unwrap(), first);
            assert_eq!(read_token(dir.path()).as_deref(), Some(first.as_str()));
            let rotated = rotate_token(dir.path()).unwrap();
            assert_ne!(rotated, first);
            assert_eq!(read_token(dir.path()).as_deref(), Some(rotated.as_str()));
            assert!(verify_bearer(Some(&format!("Bearer {rotated}")), &rotated));
            assert!(!verify_bearer(Some(&format!("Bearer {first}")), &rotated));
        });
    }

    #[test]
    fn env_token_beats_file_without_touching_disk() {
        with_clean_env(|| {
            let dir = tempfile::TempDir::new().unwrap();
            std::env::set_var(REMOTE_TOKEN_ENV, "env-token-abc");
            assert_eq!(read_token(dir.path()).as_deref(), Some("env-token-abc"));
            assert!(!token_path(dir.path()).exists());
        });
    }

    #[test]
    fn status_flags_thread_through_without_leaking_secret() {
        with_clean_env(|| {
            let cfg = RemoteAuthConfig::from_env_with_token(false);
            assert!(!cfg.oidc_configured());
            let snap = status_snapshot(&cfg, Some(1420));
            assert!(snap.local_only);
            assert!(!snap.token_configured);
            assert_eq!(snap.hud_port, Some(1420));
            assert_eq!(snap.latency_target_ms, REMOTE_AUTH_LATENCY_TARGET_MS);
            let json = serde_json::to_value(&snap).unwrap();
            assert!(!json.to_string().contains("secret"));

            std::env::set_var(OIDC_ISSUER_ENV, "https://issuer.example");
            std::env::set_var(OIDC_CLIENT_ID_ENV, "atlas-hud");
            let cfg = RemoteAuthConfig::from_env_with_token(true);
            assert!(cfg.oidc_configured());
            let snap = status_snapshot(&cfg, Some(0));
            assert_eq!(snap.hud_port, None);
        });
    }

    #[test]
    fn request_token_prefers_header_then_cookie() {
        assert_eq!(
            extract_request_token(Some("Bearer hdr"), Some("atlas_session=ck")),
            Some("hdr".to_string())
        );
        assert_eq!(
            extract_request_token(None, Some("other=1; atlas_session=ck-token ; x=2")),
            Some("ck-token".to_string())
        );
        assert_eq!(extract_request_token(Some("Basic eA"), None), None);
        assert_eq!(
            extract_request_token(None, Some("atlas_session= ; x=1")),
            None
        );
        assert_eq!(extract_request_token(None, None), None);
    }

    #[test]
    fn preview_hides_middle_but_stays_stable() {
        let p = redacted_preview("abcdefghijklmnop");
        assert!(p.starts_with("abcd"));
        assert!(p.ends_with("mnop"));
        assert!(!p.contains("efghijkl"));
        assert_eq!(redacted_preview("short"), "…");
    }
}
