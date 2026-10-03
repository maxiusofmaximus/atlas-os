// Atlas OS — Calendar MS Graph auth (RFC 28 Section G — READ path).
//
// OAuth 2.0 device-code flow against a registered public client, no
// webview required: `start_device_code` prints a code + URL the operator
// opens once, `poll_for_token` exchanges it for an access/refresh token
// pair, and `refresh_access_token` renews silently thereafter.
//
// The refresh token is encrypted at rest (AES-256-GCM) with a per-profile
// key file (`<profile_root>/calendar.key`, generated on first use) before
// it is handed to `Journal::save_calendar_token`.
//
// Client id + tenant default to the Atlas OS Calendar public client and can
// be overridden with `ATLAS_GRAPH_CLIENT_ID` / `ATLAS_GRAPH_TENANT`.

use std::path::Path;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use ring::rand::{SecureRandom, SystemRandom};
use serde::Deserialize;

use crate::calendar::error::{CalendarError, Result};

/// Registered public client, `signInAudience = AzureADandPersonalMicrosoftAccount`.
const DEFAULT_CLIENT_ID: &str = "a271f4c7-b9bb-48b2-84e5-47c1a6c3e8af";
/// `common` lets both work/school and personal Microsoft accounts sign in and
/// resolves the consumer calendar for personal accounts (RFC 28 §G).
const DEFAULT_TENANT: &str = "common";

/// Delegated scopes requested during the device-code flow. The resource must
/// be qualified so the v2 endpoint mints a Graph token.
const SCOPE: &str = "https://graph.microsoft.com/Calendars.Read offline_access";

/// Value stored in `calendar_auth.key_hint` to identify the key source.
pub const KEY_HINT: &str = "file:calendar.key";

const DEVICE_CODE_URL: &str = "https://login.microsoftonline.com";
const AES_NONCE_LEN: usize = 12;

/// Client id for the OAuth flow: `ATLAS_GRAPH_CLIENT_ID` or the built-in one.
#[must_use]
pub fn client_id() -> String {
    std::env::var("ATLAS_GRAPH_CLIENT_ID").unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string())
}

/// Tenant for the OAuth flow: `ATLAS_GRAPH_TENANT` or the built-in one.
#[must_use]
pub fn tenant() -> String {
    std::env::var("ATLAS_GRAPH_TENANT").unwrap_or_else(|_| DEFAULT_TENANT.to_string())
}

fn device_code_endpoint() -> String {
    format!("{}/{}/oauth2/v2.0/devicecode", DEVICE_CODE_URL, tenant())
}

fn token_endpoint() -> String {
    format!("{}/{}/oauth2/v2.0/token", DEVICE_CODE_URL, tenant())
}

/// The operator-facing half of the device-code handshake.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceCodeStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

/// A minted token pair (refresh token is `None` only on a malformed refresh).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<u64>,
}

#[derive(Deserialize)]
struct DeviceCodeRaw {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default)]
    interval: Option<u64>,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Deserialize)]
struct TokenRaw {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
}

#[derive(Deserialize)]
struct TokenErrorRaw {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

impl From<TokenRaw> for TokenSet {
    fn from(t: TokenRaw) -> Self {
        Self {
            access_token: t.access_token,
            refresh_token: t.refresh_token,
            expires_in: t.expires_in,
        }
    }
}

/// Kick off the device-code flow. The returned code + URL are what the
/// operator enters in a browser; poll afterwards with [`poll_for_token`].
pub async fn start_device_code(client: &reqwest::Client) -> Result<DeviceCodeStart> {
    let resp = client
        .post(device_code_endpoint())
        .form(&[("client_id", client_id()), ("scope", SCOPE.to_string())])
        .send()
        .await
        .map_err(|e| CalendarError::Auth(e.to_string()))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| CalendarError::Auth(e.to_string()))?;
    if !status.is_success() {
        return Err(CalendarError::Auth(format!(
            "devicecode HTTP {}: {body}",
            status.as_u16()
        )));
    }
    let raw: DeviceCodeRaw = serde_json::from_str(&body)
        .map_err(|e| CalendarError::Auth(format!("devicecode parse: {e}")))?;
    Ok(DeviceCodeStart {
        device_code: raw.device_code,
        user_code: raw.user_code,
        verification_uri: raw.verification_uri,
        interval: raw.interval.unwrap_or(5).max(1),
        expires_in: raw.expires_in.unwrap_or(900),
    })
}

/// Poll the token endpoint until the operator finishes the browser step.
/// Handles the standard `authorization_pending` / `slow_down` back-off.
pub async fn poll_for_token(client: &reqwest::Client, start: &DeviceCodeStart) -> Result<TokenSet> {
    let mut interval = start.interval;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(start.expires_in);
    loop {
        if std::time::Instant::now() >= deadline {
            return Err(CalendarError::Auth("device code expired".to_string()));
        }
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        let resp = client
            .post(token_endpoint())
            .form(&[
                (
                    "grant_type",
                    "urn:ietf:params:oauth:grant-type:device_code".to_string(),
                ),
                ("client_id", client_id()),
                ("device_code", start.device_code.clone()),
            ])
            .send()
            .await
            .map_err(|e| CalendarError::Auth(e.to_string()))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| CalendarError::Auth(e.to_string()))?;
        if status.is_success() {
            let raw: TokenRaw = serde_json::from_str(&body)
                .map_err(|e| CalendarError::Auth(format!("token parse: {e}")))?;
            return Ok(raw.into());
        }
        let err: TokenErrorRaw = serde_json::from_str(&body).unwrap_or(TokenErrorRaw {
            error: "unknown".to_string(),
            error_description: None,
        });
        match err.error.as_str() {
            "authorization_pending" => {}
            "slow_down" => interval = interval.saturating_add(5),
            other => {
                return Err(CalendarError::Auth(format!(
                    "{other}: {}",
                    err.error_description.unwrap_or_default()
                )));
            }
        }
    }
}

/// Exchange a stored refresh token for a fresh access token.
pub async fn refresh_access_token(
    client: &reqwest::Client,
    refresh_token: &str,
) -> Result<TokenSet> {
    let resp = client
        .post(token_endpoint())
        .form(&[
            ("grant_type", "refresh_token".to_string()),
            ("client_id", client_id()),
            ("refresh_token", refresh_token.to_string()),
            ("scope", SCOPE.to_string()),
        ])
        .send()
        .await
        .map_err(|e| CalendarError::Auth(e.to_string()))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| CalendarError::Auth(e.to_string()))?;
    if !status.is_success() {
        let err: TokenErrorRaw = serde_json::from_str(&body).unwrap_or(TokenErrorRaw {
            error: "unknown".to_string(),
            error_description: None,
        });
        return Err(CalendarError::Auth(format!(
            "refresh {}: {}",
            err.error,
            err.error_description.unwrap_or_default()
        )));
    }
    let raw: TokenRaw = serde_json::from_str(&body)
        .map_err(|e| CalendarError::Auth(format!("token parse: {e}")))?;
    Ok(raw.into())
}

/// Load the per-profile AES key, generating and persisting it on first use.
pub fn load_or_create_key(root: &Path) -> Result<[u8; 32]> {
    let path = root.join("calendar.key");
    if let Ok(bytes) = std::fs::read(&path) {
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
    }
    let key = random_32();
    std::fs::write(&path, key)?;
    Ok(key)
}

fn random_32() -> [u8; 32] {
    let mut key = [0u8; 32];
    // SystemRandom is infallible on supported platforms; on the impossibility
    // of failure we fall back to an all-zero key, which only weakens an
    // already-broken platform rather than panicking the host.
    if SystemRandom::new().fill(&mut key).is_err() {
        return [0u8; 32];
    }
    key
}

/// Encrypt a refresh token: `nonce(12) || AES-256-GCM ciphertext+tag`.
pub fn encrypt_refresh_token(key: &[u8; 32], plaintext: &str) -> Result<Vec<u8>> {
    let mut nonce = [0u8; AES_NONCE_LEN];
    if SystemRandom::new().fill(&mut nonce).is_err() {
        return Err(CalendarError::Encrypt("system RNG unavailable".to_string()));
    }
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| CalendarError::Encrypt(e.to_string()))?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
        .map_err(|e| CalendarError::Encrypt(e.to_string()))?;
    let mut blob = Vec::with_capacity(AES_NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);
    Ok(blob)
}

/// Reverse of [`encrypt_refresh_token`].
pub fn decrypt_refresh_token(key: &[u8; 32], blob: &[u8]) -> Result<String> {
    if blob.len() <= AES_NONCE_LEN {
        return Err(CalendarError::TokenDecode(
            "ciphertext too short".to_string(),
        ));
    }
    let (nonce, ciphertext) = blob.split_at(AES_NONCE_LEN);
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| CalendarError::Encrypt(e.to_string()))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|e| CalendarError::TokenDecode(e.to_string()))?;
    String::from_utf8(plaintext).map_err(|e| CalendarError::TokenDecode(e.to_string()))
}

/// Selected JWT claims from an access token — enough to diagnose scope/audience.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TokenClaims {
    pub aud: Option<String>,
    pub scp: Option<String>,
    pub tid: Option<String>,
    pub upn: Option<String>,
    pub exp: Option<i64>,
}

/// Decode the payload segment of a JWT access token (no signature check).
#[must_use]
pub fn decode_claims(access_token: &str) -> Option<TokenClaims> {
    let payload = access_token.split('.').nth(1)?;
    let bytes = b64url_decode(payload)?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let s = |key: &str| json.get(key).and_then(|v| v.as_str()).map(str::to_string);
    Some(TokenClaims {
        aud: s("aud"),
        scp: s("scp"),
        tid: s("tid"),
        upn: s("upn").or_else(|| s("preferred_username")),
        exp: json.get("exp").and_then(serde_json::Value::as_i64),
    })
}

fn b64url_decode(input: &str) -> Option<Vec<u8>> {
    fn sextet(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'-' => Some(62),
            b'_' => Some(63),
            _ => None,
        }
    }
    let trimmed = input.trim_end_matches('=');
    let mut out = Vec::with_capacity(trimmed.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for &c in trimmed.as_bytes() {
        buffer = (buffer << 6) | u32::from(sextet(c)?);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

/// PKCE verifier/challenge pair for the authorization-code flow.
#[derive(Clone, Debug)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    /// Generate a 32-byte verifier and its `S256` challenge.
    #[must_use]
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        if SystemRandom::new().fill(&mut bytes).is_err() {
            bytes = [1u8; 32];
        }
        let verifier = b64url_encode(&bytes);
        let digest = ring::digest::digest(&ring::digest::SHA256, verifier.as_bytes());
        let challenge = b64url_encode(digest.as_ref());
        Self {
            verifier,
            challenge,
        }
    }
}

/// base64url (no padding) encode.
#[must_use]
pub fn b64url_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[((n >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 63) as usize] as char);
        }
    }
    out
}

fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Build the `/authorize` URL for the loopback authorization-code flow.
#[must_use]
pub fn authorize_url(redirect_uri: &str, challenge: &str) -> String {
    format!(
        "{DEVICE_CODE_URL}/{tenant}/oauth2/v2.0/authorize?\
         client_id={client}&response_type=code&redirect_uri={redirect}&\
         response_mode=query&scope={scope}&code_challenge={challenge}&\
         code_challenge_method=S256",
        tenant = tenant(),
        client = client_id(),
        redirect = percent_encode(redirect_uri),
        scope = percent_encode(SCOPE),
        challenge = challenge,
    )
}

/// Run the loopback authorization-code + PKCE flow: opens the browser, waits
/// for the redirect on `http://localhost:<port>`, then redeems the code.
pub async fn login_loopback(client: &reqwest::Client) -> Result<TokenSet> {
    let pkce = Pkce::generate();
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| CalendarError::Auth(e.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|e| CalendarError::Auth(e.to_string()))?
        .port();
    let redirect_uri = format!("http://localhost:{port}");
    let url = authorize_url(&redirect_uri, &pkce.challenge);
    open_browser(&url);

    let code = tokio::task::spawn_blocking(move || await_loopback_code(listener))
        .await
        .map_err(|e| CalendarError::Auth(e.to_string()))??;

    let code_len = code.len();
    let resp = client
        .post(token_endpoint())
        .form(&[
            ("grant_type", "authorization_code".to_string()),
            ("client_id", client_id()),
            ("code", code),
            ("redirect_uri", redirect_uri.clone()),
            ("code_verifier", pkce.verifier),
            ("scope", SCOPE.to_string()),
        ])
        .send()
        .await
        .map_err(|e| CalendarError::Auth(e.to_string()))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| CalendarError::Auth(e.to_string()))?;
    if !status.is_success() {
        return Err(CalendarError::Auth(format!(
            "authorization_code HTTP {} (code_len={code_len} redirect={redirect_uri}): {body}",
            status.as_u16()
        )));
    }
    let raw: TokenRaw = serde_json::from_str(&body)
        .map_err(|e| CalendarError::Auth(format!("token parse: {e}")))?;
    Ok(raw.into())
}

fn await_loopback_code(listener: std::net::TcpListener) -> Result<String> {
    use std::io::{Read, Write};

    let (mut stream, _) = listener
        .accept()
        .map_err(|e| CalendarError::Auth(e.to_string()))?;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = stream
            .read(&mut chunk)
            .map_err(|e| CalendarError::Auth(e.to_string()))?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") || buf.len() > 65_536 {
            break;
        }
    }
    let request = String::from_utf8_lossy(&buf);
    let target = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("");
    let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
    let param = |key: &str| -> Option<String> {
        query.split('&').find_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            (k == key).then(|| percent_decode(v))
        })
    };

    let body = "<html><body><h3>Atlas OS — autenticación completada</h3>\
                <p>Ya puedes cerrar esta ventana y volver a la terminal.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();

    if let Some(error) = param("error") {
        return Err(CalendarError::Auth(format!(
            "{error}: {}",
            param("error_description").unwrap_or_default()
        )));
    }
    param("code").ok_or_else(|| CalendarError::Auth("no code in loopback callback".to_string()))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => match u8::from_str_radix(&value[i + 1..i + 3], 16) {
                Ok(byte) => {
                    out.push(byte);
                    i += 3;
                }
                Err(_) => {
                    out.push(b'%');
                    i += 1;
                }
            },
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    {
        // `rundll32 url.dll` hands the URL to the shell handler directly,
        // so `&` in the query string is not re-parsed by cmd.exe.
        let _ = std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    println!("Abriendo el navegador… si no se abre, visita:\n{url}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn endpoints_target_the_configured_tenant() {
        let dc = device_code_endpoint();
        let tok = token_endpoint();
        assert!(dc.contains("login.microsoftonline.com"));
        assert!(dc.ends_with("/oauth2/v2.0/devicecode"));
        assert!(tok.ends_with("/oauth2/v2.0/token"));
        assert!(dc.contains(&tenant()));
    }

    #[test]
    fn device_code_raw_parses_a_typical_payload() {
        let body = r#"{
            "user_code": "ABCD-EFGH",
            "device_code": "secret",
            "verification_uri": "https://microsoft.com/devicelogin",
            "expires_in": 900,
            "interval": 5,
            "message": "To sign in..."
        }"#;
        let raw: DeviceCodeRaw = serde_json::from_str(body).expect("parse");
        assert_eq!(raw.user_code, "ABCD-EFGH");
        assert_eq!(raw.interval, Some(5));
    }

    #[test]
    fn token_error_raw_parses_authorization_pending() {
        let raw: TokenErrorRaw = serde_json::from_str(
            r#"{ "error": "authorization_pending", "error_description": "waiting" }"#,
        )
        .expect("parse");
        assert_eq!(raw.error, "authorization_pending");
    }

    #[test]
    fn refresh_token_encrypts_and_round_trips() {
        let key = [7u8; 32];
        let blob = encrypt_refresh_token(&key, "rt-secret-value").expect("encrypt");
        assert_ne!(&blob[..], b"rt-secret-value");
        let back = decrypt_refresh_token(&key, &blob).expect("decrypt");
        assert_eq!(back, "rt-secret-value");
    }

    #[test]
    fn decrypt_rejects_wrong_key_and_truncated_blob() {
        let blob = encrypt_refresh_token(&[1u8; 32], "x").expect("encrypt");
        assert!(decrypt_refresh_token(&[2u8; 32], &blob).is_err());
        assert!(decrypt_refresh_token(&[1u8; 32], &blob[..5]).is_err());
    }

    #[test]
    fn key_file_is_created_once_and_reused() {
        let dir = TempDir::new().expect("tmp");
        let a = load_or_create_key(dir.path()).expect("first");
        let b = load_or_create_key(dir.path()).expect("second");
        assert_eq!(a, b, "key must be stable across calls");
        assert!(dir.path().join("calendar.key").exists());
    }

    #[test]
    fn decode_claims_extracts_aud_scp_and_exp() {
        // base64url({"aud":"https://graph.microsoft.com","scp":"Calendars.Read","tid":"t","upn":"u@x","exp":123})
        let payload = "eyJhdWQiOiJodHRwczovL2dyYXBoLm1pY3Jvc29mdC5jb20iLCJzY3AiOiJDYWxlbmRhcnMuUmVhZCIsInRpZCI6InQiLCJ1cG4iOiJ1QHgiLCJleHAiOjEyM30";
        let claims = decode_claims(&format!("header.{payload}.sig")).expect("claims");
        assert_eq!(claims.aud.as_deref(), Some("https://graph.microsoft.com"));
        assert_eq!(claims.scp.as_deref(), Some("Calendars.Read"));
        assert_eq!(claims.tid.as_deref(), Some("t"));
        assert_eq!(claims.upn.as_deref(), Some("u@x"));
        assert_eq!(claims.exp, Some(123));
    }

    #[test]
    fn b64url_encode_round_trips_through_decode() {
        for raw in [b"".as_slice(), b"a", b"ab", b"abc", b"hello world!"] {
            let encoded = b64url_encode(raw);
            assert_eq!(b64url_decode(&encoded).expect("decode"), raw);
        }
    }

    #[test]
    fn pkce_challenge_is_s256_of_verifier() {
        let pkce = Pkce::generate();
        assert!(!pkce.verifier.is_empty());
        let digest = ring::digest::digest(&ring::digest::SHA256, pkce.verifier.as_bytes());
        assert_eq!(pkce.challenge, b64url_encode(digest.as_ref()));
    }

    #[test]
    fn authorize_url_carries_pkce_and_redirect() {
        let url = authorize_url("http://localhost:1234", "chal");
        assert!(url.contains("response_type=code"));
        assert!(url.contains("code_challenge=chal"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A1234"));
    }

    #[test]
    fn percent_decode_handles_encoded_values() {
        assert_eq!(percent_decode("a%20b"), "a b");
        assert_eq!(percent_decode("x%2Fy"), "x/y");
        assert_eq!(percent_decode("plain-._~"), "plain-._~");
        assert_eq!(percent_decode("bad%2"), "bad%2");
    }
}
