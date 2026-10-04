// Atlas OS — ICS feed access token (RFC 28 §G.2).
//
// The WRITE feed `GET /atlas-calendar.ics` is gated by an opaque subscrip-
// tion token so the `webcal://127.0.0.1:…/atlas-calendar.ics?token=…` URL a
// user pastes into Outlook / Apple Calendar / Google Calendar is not
// guessable by another local process.
//
// The token is `base64url(16 bytes)` (RFC 4648 §5, no padding) over the 16
// random bytes of a v4 UUID — no new randomness or base64 crate (mirrors
// `remote_auth`'s "ride `Uuid`" approach). Persisted at
// `<profile_root>/calendar_ics_token.txt` so the URL is stable across restarts.

use std::path::{Path, PathBuf};

use uuid::Uuid;

/// File name under the profile root holding the persisted ICS token.
pub const ICS_TOKEN_FILE: &str = "calendar_ics_token.txt";

pub fn token_path(profile_root: &Path) -> PathBuf {
    profile_root.join(ICS_TOKEN_FILE)
}

/// `base64url(16 bytes)` of a fresh v4 UUID.
pub fn generate_token() -> String {
    base64url(Uuid::new_v4().as_bytes())
}

/// Read the persisted token, if any (blank/missing → `None`).
pub fn read_token(profile_root: &Path) -> Option<String> {
    std::fs::read_to_string(token_path(profile_root))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Return the persisted token, generating and persisting one when absent.
pub fn ensure_token(profile_root: &Path) -> std::io::Result<String> {
    if let Some(t) = read_token(profile_root) {
        return Ok(t);
    }
    let token = generate_token();
    std::fs::write(token_path(profile_root), &token)?;
    Ok(token)
}

/// Constant-time-ish comparison (length check + xor fold) so a wrong token of
/// the right length does not early-exit on the first byte. Empty expected
/// never verifies.
pub fn verify(expected: &str, presented: Option<&str>) -> bool {
    if expected.is_empty() {
        return false;
    }
    let presented = presented.unwrap_or("");
    if presented.len() != expected.len() {
        return false;
    }
    presented
        .bytes()
        .zip(expected.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// RFC 4648 §5 base64url without padding.
fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[((n >> 6) & 63) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(n & 63) as usize] as char);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn base64url_matches_rfc4648_vectors() {
        assert_eq!(base64url(b""), "");
        assert_eq!(base64url(b"f"), "Zg");
        assert_eq!(base64url(b"fo"), "Zm8");
        assert_eq!(base64url(b"foo"), "Zm9v");
        assert_eq!(base64url(b"foob"), "Zm9vYg");
        assert_eq!(base64url(b"fooba"), "Zm9vYmE");
        assert_eq!(base64url(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn generate_token_is_22_url_safe_chars() {
        let t = generate_token();
        assert_eq!(t.len(), 22, "16 bytes → 22 base64url chars: {t}");
        assert!(
            t.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "token must be url-safe: {t}"
        );
    }

    #[test]
    fn ensure_token_persists_and_is_stable() {
        let dir = TempDir::new().unwrap();
        assert!(read_token(dir.path()).is_none());
        let a = ensure_token(dir.path()).unwrap();
        let b = ensure_token(dir.path()).unwrap();
        assert_eq!(a, b, "token stable across calls");
        assert_eq!(read_token(dir.path()).as_deref(), Some(a.as_str()));
    }

    #[test]
    fn verify_only_accepts_exact_match() {
        let t = generate_token();
        assert!(verify(&t, Some(&t)));
        assert!(!verify(&t, None));
        assert!(!verify(&t, Some("")));
        assert!(!verify(&t, Some("short")));
        let mut wrong = t.clone();
        wrong.pop();
        wrong.push(if t.ends_with('A') { 'B' } else { 'A' });
        assert!(!verify(&t, Some(&wrong)));
        assert!(!verify("", Some(&t)));
    }
}
