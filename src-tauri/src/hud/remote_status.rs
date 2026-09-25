// Atlas OS — `GET /remote/status` (RFC 20 Phase 8 sub-fase 8.3,
// research 36 SECTOR B 8.3, RFC 24 §16).
//
// Always-mounted informational route: reports whether the remote bearer
// is configured and whether OIDC discovery is set, without ever leaking
// the token itself. When the `remote-ui` feature is enabled the same
// route enforces the bearer (header or `atlas_session` cookie) and
// answers 401 otherwise — the behavioral gate the future OIDC layer
// reuses, with zero new crates either way.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::core::state::AppState;
use crate::remote_auth;

pub async fn get_remote_status(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let root = state.profile_root();
    let expected = remote_auth::read_token(&root).unwrap_or_default();
    let config = remote_auth::RemoteAuthConfig::from_env_with_token(!expected.is_empty());

    #[cfg(feature = "remote-ui")]
    {
        let presented = remote_auth::extract_request_token(
            headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok()),
            headers
                .get(axum::http::header::COOKIE)
                .and_then(|v| v.to_str().ok()),
        );
        let authorized = match presented {
            Some(token) => remote_auth::verify_bearer(Some(&format!("Bearer {token}")), &expected),
            None => false,
        };
        if !authorized {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "unauthorized",
                    "hint": "send `Authorization: Bearer <token>` (see `atlas hud --auth-status`)",
                })),
            )
                .into_response();
        }
    }
    #[cfg(not(feature = "remote-ui"))]
    let _ = &headers;

    let snap = remote_auth::status_snapshot(&config, Some(state.hud_port()));
    (StatusCode::OK, Json(snap)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::AUTHORIZATION;

    fn state_in(dir: &std::path::Path) -> Arc<AppState> {
        let journal = crate::journal::Journal::open(dir).unwrap();
        let token = remote_auth::ensure_token(dir).unwrap();
        assert!(!token.is_empty());
        let state = AppState::from_journal_in(journal, dir.to_path_buf());
        state.set_hud_port(1420);
        Arc::new(state)
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in pairs {
            map.insert(
                k.parse::<axum::http::HeaderName>().unwrap(),
                v.parse().unwrap(),
            );
        }
        map
    }

    fn file_token(dir: &std::path::Path) -> String {
        std::fs::read_to_string(dir.join(remote_auth::REMOTE_TOKEN_FILE))
            .unwrap()
            .trim()
            .to_string()
    }

    #[tokio::test]
    async fn status_never_leaks_the_token() {
        let dir = tempfile::TempDir::new().unwrap();
        let state = state_in(dir.path());
        let token = file_token(dir.path());
        let resp = get_remote_status(
            State(state),
            headers(&[(AUTHORIZATION.as_str(), &format!("Bearer {token}"))]),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 64)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(!text.contains(&token));
        assert!(text.contains("1420"));
    }

    #[cfg(feature = "remote-ui")]
    #[tokio::test]
    async fn enforced_mode_rejects_anonymous_but_accepts_cookie() {
        let dir = tempfile::TempDir::new().unwrap();
        let state = state_in(dir.path());
        let token = file_token(dir.path());
        let anon = get_remote_status(State(state.clone()), headers(&[])).await;
        assert_eq!(anon.status(), StatusCode::UNAUTHORIZED);
        let wrong = get_remote_status(
            State(state.clone()),
            headers(&[(AUTHORIZATION.as_str(), "Bearer wrong")]),
        )
        .await;
        assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);
        let cookie = get_remote_status(
            State(state),
            headers(&[(
                axum::http::header::COOKIE.as_str(),
                &format!("other=1; atlas_session={token}"),
            )]),
        )
        .await;
        assert_eq!(cookie.status(), StatusCode::OK);
    }
}
