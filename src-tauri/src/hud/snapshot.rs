// Atlas OS — HUD agent snapshot route (RFC 67 §20 H-08).
//
// Bridges `Sandbox::snapshot` (RFC 63 §4 item 4) to the HUD so the Agent Card
// can show a run's filesystem frame. The route looks up the run, resolves the
// sandbox runtime the run recorded, and hashes the profile root with it. A run
// that is unknown, references an unavailable runtime, or whose root cannot be
// read surfaces 404 with a reason — the card never claims a frame it could not
// produce. Read-only; nothing is written back.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::core::state::AppState;
use crate::orchestrator::sandbox::sandbox_for_kind;

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotFile {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotResponse {
    pub run_id: String,
    pub sandbox: String,
    pub root: String,
    pub file_count: usize,
    pub files: Vec<SnapshotFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotError {
    pub reason: String,
}

fn err(status: StatusCode, reason: String) -> (StatusCode, Json<SnapshotError>) {
    (status, Json(SnapshotError { reason }))
}

pub async fn get_agent_snapshot(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<Json<SnapshotResponse>, (StatusCode, Json<SnapshotError>)> {
    let run = {
        let journal = state.journal();
        journal
            .get_agent_run(&run_id)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    };
    let Some(run) = run else {
        return Err(err(
            StatusCode::NOT_FOUND,
            format!("unknown agent run {run_id}"),
        ));
    };
    let Some(sandbox) = sandbox_for_kind(&run.sandbox) else {
        return Err(err(
            StatusCode::NOT_FOUND,
            format!("no sandbox runtime for kind '{}'", run.sandbox),
        ));
    };
    let root = state.profile_root();
    let entries = sandbox
        .snapshot(&root)
        .map_err(|reason| err(StatusCode::NOT_FOUND, reason))?;
    let files: Vec<SnapshotFile> = entries
        .into_iter()
        .map(|(path, sha256)| SnapshotFile { path, sha256 })
        .collect();
    Ok(Json(SnapshotResponse {
        run_id,
        sandbox: sandbox.kind().to_string(),
        root: root.display().to_string(),
        file_count: files.len(),
        files,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    use crate::core::state::AppState;
    use crate::journal::{AgentRunRow, Journal};

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/hud/agent/:run_id/snapshot", get(get_agent_snapshot))
            .with_state(state)
    }

    fn run(id: &str, sandbox: &str) -> AgentRunRow {
        AgentRunRow {
            id: id.into(),
            mission_id: None,
            goal: "make out.txt".into(),
            success_predicate: "[]".into(),
            sandbox: sandbox.into(),
            status: "running".into(),
            steps: 0,
            tokens_in: 0,
            tokens_out: 0,
            cost_usd: 0.0,
            ts_started: 1,
            ts_ended: None,
        }
    }

    fn state_with_run(tmp: &tempfile::TempDir, sandbox: &str) -> Arc<AppState> {
        let journal = Journal::open(tmp.path()).expect("open");
        journal
            .create_agent_run(&run("r1", sandbox))
            .expect("create run");
        Arc::new(AppState::from_journal_in(journal, tmp.path().to_path_buf()))
    }

    #[tokio::test]
    async fn returns_the_frame_for_a_known_run() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("out.txt"), "hello").unwrap();
        let app = app(state_with_run(&tmp, "local"));

        let req = Request::builder()
            .method("GET")
            .uri("/hud/agent/r1/snapshot")
            .body(axum::body::Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = to_bytes(res.into_body(), 65536).await.unwrap();
        let frame: SnapshotResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(frame.run_id, "r1");
        assert_eq!(frame.sandbox, "local");
        assert!(frame.file_count >= 1);
        assert!(frame.files.iter().any(|f| f.path == "out.txt"));
        assert!(frame.files.iter().any(|f| f.sha256.len() == 64));
    }

    #[tokio::test]
    async fn unknown_run_is_404_with_reason() {
        let tmp = tempfile::TempDir::new().unwrap();
        let journal = Journal::open(tmp.path()).expect("open");
        let app = app(Arc::new(AppState::from_journal_in(
            journal,
            tmp.path().to_path_buf(),
        )));

        let req = Request::builder()
            .method("GET")
            .uri("/hud/agent/nope/snapshot")
            .body(axum::body::Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let bytes = to_bytes(res.into_body(), 8192).await.unwrap();
        let error: SnapshotError = serde_json::from_slice(&bytes).unwrap();
        assert!(error.reason.contains("unknown agent run"));
    }

    #[tokio::test]
    async fn run_without_a_resolvable_sandbox_is_404_with_reason() {
        let tmp = tempfile::TempDir::new().unwrap();
        let app = app(state_with_run(&tmp, "mystery-runtime"));

        let req = Request::builder()
            .method("GET")
            .uri("/hud/agent/r1/snapshot")
            .body(axum::body::Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let bytes = to_bytes(res.into_body(), 8192).await.unwrap();
        let error: SnapshotError = serde_json::from_slice(&bytes).unwrap();
        assert!(error.reason.contains("no sandbox runtime"));
    }
}
