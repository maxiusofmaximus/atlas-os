// Atlas OS — HUD autoresearch control routes (RFC 28 §A — Phase 1.5b §A-3).
//
// Single route attached to the HUD axum server:
//
//   POST /autoresearch/cancel   body: { run_id: string, outcome: string }
//                              → 204 No Content
//
// The HUD frontend (`AutoresearchCard.svelte`) calls this when the user
// clicks "Stop" on the autoresearch card. The handler is intentionally a
// thin transport: it validates the request, publishes an
// `AutoresearchCancelled` event on the Kernel Bus, and returns 204. The
// Execution Supervisor host (Phase 2) subscribes to the bus, breaks the
// experiment loop, persists `autoresearch_runs.outcome = 'aborted'`,
// and emits the downstream `JournalCheckpoint`. No DB write happens
// here — keeps the HUD side effect-free w.r.t. the journal and avoids
// racing the supervisor's own transition.
//
// `outcome` is accepted as an open string for forward-compat with the
// future `/autoresearch/pause` route (`outcome: 'paused'`); the front-
// end only ever sends `'aborted'` today, and the supervisor's
// `Outcome::Aborted` is the only side that reads this field for now.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::core::bus::{BusEvent, BusEventKind};
use crate::core::state::AppState;

/// POST body. The frontend `AutoresearchCancel` type pins `outcome` to
/// the literal `'aborted'`; we accept an open string here so the same
/// route can host `/pause` later without a schema migration.
#[derive(Debug, Deserialize)]
pub struct AutoresearchCancelRequest {
    pub run_id: String,
    pub outcome: String,
}

/// Validation ceiling for the incoming `run_id`. UUIDs are 36 chars;
/// we accept a small margin for non-UUID host-generated ids (the RFC
/// leaves the run id opaque to the front-end) but reject anything that
/// would make a poor audit-trail key.
const MAX_RUN_ID_LEN: usize = 64;
const MAX_OUTCOME_LEN: usize = 32;

pub async fn post_autoresearch_cancel(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AutoresearchCancelRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let trimmed_run = body.run_id.trim();
    if trimmed_run.is_empty() || trimmed_run.len() > MAX_RUN_ID_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            "run_id must be 1..=64 chars".to_string(),
        ));
    }
    let trimmed_outcome = body.outcome.trim();
    if trimmed_outcome.is_empty() || trimmed_outcome.len() > MAX_OUTCOME_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            "outcome must be 1..=32 chars".to_string(),
        ));
    }

    let event = BusEvent::new(BusEventKind::AutoresearchCancelled {
        run_id: trimmed_run.to_string(),
        outcome: trimmed_outcome.to_string(),
    });
    state
        .publish_and_broadcast(&event)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
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
            .route("/autoresearch/cancel", post(post_autoresearch_cancel))
            .with_state(state)
    }

    fn cancel_req(body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/autoresearch/cancel")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn happy_path_returns_204() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let body = serde_json::json!({
            "run_id": "00000000-0000-0000-0000-000000000abc",
            "outcome": "aborted",
        });
        let res = app.oneshot(cancel_req(body)).await.unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn empty_run_id_is_400() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let body = serde_json::json!({"run_id": "   ", "outcome": "aborted"});
        let res = app.oneshot(cancel_req(body)).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn oversized_run_id_is_400() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let long = "x".repeat(MAX_RUN_ID_LEN + 1);
        let body = serde_json::json!({"run_id": long, "outcome": "aborted"});
        let res = app.oneshot(cancel_req(body)).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn empty_outcome_is_400() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let body = serde_json::json!({"run_id": "r1", "outcome": ""});
        let res = app.oneshot(cancel_req(body)).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn oversized_outcome_is_400() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let long = "y".repeat(MAX_OUTCOME_LEN + 1);
        let body = serde_json::json!({"run_id": "r1", "outcome": long});
        let res = app.oneshot(cancel_req(body)).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn malformed_json_is_rejected_with_4xx() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let req = Request::builder()
            .method("POST")
            .uri("/autoresearch/cancel")
            .header("content-type", "application/json")
            .body(Body::from("{not json"))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        let status = res.status().as_u16();
        assert!(
            status == 400 || status == 422,
            "expected 4xx rejection, got {status}"
        );
    }

    #[tokio::test]
    async fn paused_outcome_is_accepted_for_forward_compat() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let body = serde_json::json!({
            "run_id": "00000000-0000-0000-0000-000000000abc",
            "outcome": "paused",
        });
        let res = app.oneshot(cancel_req(body)).await.unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
    }
}
