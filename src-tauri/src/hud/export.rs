// Atlas OS — HUD audit export routes (RFC 28 §D — Phase 1.5a §D-5).
//
// Single route attached to the HUD axum server:
//
//   POST /audit/export-posting  body: { last?: u32, output_dir?: string }
//                               → { files_written: string[], entries_packed: u32,
//                                   entries_purged: u32, snapshot_root: string }
//
// The HUD frontend calls this when the user clicks "Export as posting" on the
// Audit card. The snapshot files are written under the supplied `output_dir`
// or `<profile_root>/snapshots/` by default. The frontend surfaces a Tauri
// save dialog elsewhere — here we just take a path string for cross-platform
// determinism (Tauri webview is not always available in headless runs).
//
// Persistence path used: `journal::export::retention::snapshot_entries`.
// Per RFC 28 §D we ignore the posting `scripts` field on emit (security
// boundary AGENTS.md §6).

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::core::state::AppState;
use crate::journal::export::retention::{RetentionConfig, SnapshotResult};

/// POST body. All fields optional:
/// * `last` — last N audit entries to export. Default 50 (matches CLI default).
/// * `output_dir` — destination dir. Default `<profile_root>/snapshots/`.
#[derive(Debug, Deserialize)]
pub struct ExportPostingRequest {
    pub last: Option<u32>,
    pub output_dir: Option<String>,
}

/// Successful POST response.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportPostingResponse {
    pub files_written: Vec<String>,
    pub entries_packed: u32,
    pub entries_purged: u32,
    pub snapshot_root: String,
}

pub async fn post_export_posting(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ExportPostingRequest>,
) -> Result<Json<ExportPostingResponse>, (StatusCode, String)> {
    let last = body.last.unwrap_or(50).min(10_000) as i64;
    let snapshot_root = match body.output_dir.as_deref() {
        Some(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => state.profile_root().join("snapshots"),
    };

    let journal = state.journal();
    let entries = journal
        .audit_tail(last)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    drop(journal);

    let cfg = RetentionConfig::new(snapshot_root.clone());
    let SnapshotResult {
        files_written,
        entries_packed,
        entries_purged,
    } = crate::journal::export::retention::snapshot_entries(&entries, &cfg)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ExportPostingResponse {
        files_written: files_written
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
        entries_packed: entries_packed as u32,
        entries_purged: entries_purged as u32,
        snapshot_root: snapshot_root.to_string_lossy().into_owned(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::{Request, StatusCode};
    use axum::routing::post;
    use axum::Router;
    use tower::ServiceExt;

    use crate::core::state::AppState;
    use crate::journal::Journal;

    fn fresh_state(tmp: &tempfile::TempDir) -> Arc<AppState> {
        let journal = Journal::open(tmp.path()).expect("open");
        Arc::new(AppState::from_journal(journal))
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/audit/export-posting", post(post_export_posting))
            .with_state(state)
    }

    #[tokio::test]
    async fn empty_audit_log_returns_zero_packed() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);

        let req_body = serde_json::json!({"last": 50});
        let req = Request::builder()
            .method("POST")
            .uri("/audit/export-posting")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(req_body.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), 8192).await.unwrap();
        let resp: ExportPostingResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.entries_packed, 0);
        assert!(resp.files_written.is_empty());
        assert!(resp.snapshot_root.contains("snapshots"));
    }

    #[tokio::test]
    async fn custom_output_dir_is_honored() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);

        let arb_dir = tmp.path().join("custom-snapshots");
        let req_body = serde_json::json!({
            "last": 50,
            "output_dir": arb_dir.to_string_lossy()
        });
        let req = Request::builder()
            .method("POST")
            .uri("/audit/export-posting")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(req_body.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), 8192).await.unwrap();
        let resp: ExportPostingResponse = serde_json::from_slice(&body).unwrap();
        assert!(resp.snapshot_root.ends_with("custom-snapshots"));
    }

    #[tokio::test]
    async fn last_param_clamps_to_10000_max() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let req_body = serde_json::json!({"last": 999999999});
        let req = Request::builder()
            .method("POST")
            .uri("/audit/export-posting")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(req_body.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn empty_output_dir_falls_back_to_profile_root_snapshots() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let expected = state.profile_root().join("snapshots");
        let expected_str = expected.to_string_lossy().into_owned();
        let app = app(state);
        let req_body = serde_json::json!({"output_dir": ""});
        let req = Request::builder()
            .method("POST")
            .uri("/audit/export-posting")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(req_body.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), 8192).await.unwrap();
        let resp: ExportPostingResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.snapshot_root, expected_str);
    }

    #[tokio::test]
    async fn whitespace_only_output_dir_falls_back_to_profile_root() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let expected = state.profile_root().join("snapshots");
        let expected_str = expected.to_string_lossy().into_owned();
        let app = app(state);
        let req_body = serde_json::json!({"output_dir": "   "});
        let req = Request::builder()
            .method("POST")
            .uri("/audit/export-posting")
            .header("content-type", "application/json")
            .body(axum::body::Body::from(req_body.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), 8192).await.unwrap();
        let resp: ExportPostingResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(resp.snapshot_root, expected_str);
    }
}
