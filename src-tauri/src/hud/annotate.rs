// OpenCode OS — HUD diff-annotation routes (RFC 27 §E).
//
// Two routes attached to the HUD axum server:
//
//   POST /diff/{id}/annotation    body: { body, author, file_path?, line_no? }
//   GET  /diff/{id}/annotations
//
// Persistence is append-only in `diff_annotations` (M12). The HUD
// drawer shows them inline next to the changed line; the next Coding
// Engine run reads them as additional context so the reviewer's notes
// steer the agent's next attempt (cursor-pattern).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::state::AppState;
use crate::journal::DiffAnnotationRow;

/// POST /diff/{id}/annotation — body shape received from HUD (or any
/// reviewer UI). All but `body` are optional: a diff-level comment
/// uses `file_path=None, line_no=None`; a line comment must pin both.
#[derive(Debug, Deserialize)]
pub struct AnnotationPost {
    /// Markdown/plain text of the comment.
    pub body: String,
    /// Human-readable author handle (e.g. "max", "agent-Planner-7f3a").
    pub author: String,
    /// Optional intra-file path (e.g. `src/lib.rs`).
    pub file_path: Option<String>,
    /// 1-indexed line number in `file_path` (None for diff-level notes).
    pub line_no: Option<i64>,
}

/// Echoed after `POST` so the caller can render the row immediately
/// without a follow-up GET.
#[derive(Debug, Serialize, Deserialize)]
pub struct AnnotationPosted {
    pub id: Uuid,
    pub diff_id: Uuid,
    pub created_at: String,
}

pub async fn post_annotation(
    State(state): State<Arc<AppState>>,
    Path(diff_id): Path<String>,
    Json(body): Json<AnnotationPost>,
) -> Result<Json<AnnotationPosted>, StatusCode> {
    let diff_uuid = Uuid::parse_str(&diff_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    if body.body.trim().is_empty() {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let id = Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();
    let row = DiffAnnotationRow {
        id,
        diff_id: diff_uuid,
        file_path: body.file_path.filter(|s| !s.trim().is_empty()),
        line_no: body.line_no,
        body: body.body,
        author: body.author,
        created_at: now.clone(),
    };
    let journal = state.journal();
    journal
        .save_diff_annotation(&row)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(AnnotationPosted {
        id,
        diff_id: diff_uuid,
        created_at: now,
    }))
}

pub async fn get_annotations(
    State(state): State<Arc<AppState>>,
    Path(diff_id): Path<String>,
) -> Result<Json<Vec<DiffAnnotationRow>>, StatusCode> {
    let diff_uuid = Uuid::parse_str(&diff_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let journal = state.journal();
    journal
        .diff_annotations_for_diff(diff_uuid)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .map(Json)
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
            .route(
                "/diff/:id/annotation",
                post(post_annotation).get(get_annotations),
            )
            .with_state(state)
    }

    #[tokio::test]
    async fn post_then_get_round_trips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);

        let diff_id = Uuid::new_v4();
        let req_body = serde_json::json!({
            "body": "use `?` instead of unwrap",
            "author": "max",
            "file_path": "src/lib.rs",
            "line_no": 42,
        });
        let req = Request::builder()
            .method("POST")
            .uri(format!("/diff/{diff_id}/annotation"))
            .header("content-type", "application/json")
            .body(axum::body::Body::from(req_body.to_string()))
            .unwrap();
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = to_bytes(res.into_body(), 8192).await.unwrap();
        let posted: AnnotationPosted = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(posted.diff_id, diff_id);

        let req = Request::builder()
            .method("GET")
            .uri(format!("/diff/{diff_id}/annotation"))
            .body(axum::body::Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = to_bytes(res.into_body(), 8192).await.unwrap();
        let rows: Vec<DiffAnnotationRow> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].body, "use `?` instead of unwrap");
        assert_eq!(rows[0].author, "max");
        assert_eq!(rows[0].line_no, Some(42));
    }

    #[tokio::test]
    async fn empty_body_is_unprocessable() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let diff_id = Uuid::new_v4();
        let req_body = serde_json::json!({
            "body": "   ",
            "author": "anyone",
        });
        let req = Request::builder()
            .method("POST")
            .uri(format!("/diff/{diff_id}/annotation"))
            .header("content-type", "application/json")
            .body(axum::body::Body::from(req_body.to_string()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn malformed_diff_id_is_bad_request() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let req = Request::builder()
            .method("GET")
            .uri("/diff/not-a-uuid/annotation")
            .body(axum::body::Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}
