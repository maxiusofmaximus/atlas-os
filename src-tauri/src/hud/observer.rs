// Atlas OS — HUD journal observer route (RFC 19 §10, research 33 SECTOR B 6.1).
//
// Single route attached to the HUD axum server:
//
//   GET /hud/journal?limit=N&offset=M&kind=K
//     → 200 { entries, total, limit, offset }
//
// The Phase 1 `/tail/*` routes return newest-N projections (several
// without their payload column at all). The Observer is the full
// inspection view: complete `JournalEntry` payloads, newest-first
// pagination via `limit`/`offset`, and an optional `kind` filter so
// the operator can isolate one event stream (stall events, restarts,
// checkpoints). The Svelte `<JournalObserver>` component consumes
// this through `fetchJournalPage` in `src/lib/stores/hud.ts`.
//
// The handler is a thin transport: it validates the query, delegates
// to `Journal::journal_page`, and maps failures to 4xx (caller error)
// or 500 (journal error). No DB write happens here.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::core::state::AppState;
use crate::journal::JournalEntry;

/// Defaults mirror `tail.rs` (`DEFAULT_LAST`/`MAX_LAST`) so the
/// observer and the tail boxes page the same window sizes.
const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 200;
const MAX_KIND_LEN: usize = 64;

/// `?limit=N&offset=M&kind=K`. All three are optional: absent means
/// "first page of the default window, every kind".
#[derive(Debug, Deserialize)]
pub struct JournalPageQuery {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
    #[serde(default)]
    pub kind: Option<String>,
}

/// Envelope returned to the HUD. `total` is the count of matching
/// rows (before `limit`/`offset`) so the UI can render prev/next
/// without a follow-up count query.
#[derive(Debug, Serialize)]
pub struct JournalPageResponse {
    pub entries: Vec<JournalEntry>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

pub async fn get_journal_page(
    State(state): State<Arc<AppState>>,
    Query(q): Query<JournalPageQuery>,
) -> Result<Json<JournalPageResponse>, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("limit must be 1..={MAX_LIMIT}"),
        ));
    }
    let offset = q.offset.unwrap_or(0);
    if offset < 0 {
        return Err((StatusCode::BAD_REQUEST, "offset must be >= 0".to_string()));
    }
    let kind = q.kind.as_deref().map(str::trim).filter(|k| !k.is_empty());
    if let Some(k) = kind {
        if k.len() > MAX_KIND_LEN {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("kind must be 1..={MAX_KIND_LEN} chars"),
            ));
        }
    }

    let journal = state.journal();
    let (entries, total) = journal
        .journal_page(limit, offset, kind)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(JournalPageResponse {
        entries,
        total,
        limit,
        offset,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    use crate::core::bus::{AgentStatus, BusEvent, BusEventKind};
    use crate::core::state::AppState;
    use crate::journal::Journal;

    fn fresh_state(tmp: &tempfile::TempDir) -> Arc<AppState> {
        let journal = Journal::open(tmp.path()).expect("open");
        Arc::new(AppState::from_journal(journal))
    }

    fn app(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/hud/journal", get(get_journal_page))
            .with_state(state)
    }

    fn task_received(raw: &str) -> BusEvent {
        BusEvent::new(BusEventKind::TaskReceived {
            raw_prompt: raw.into(),
            session_id: uuid::Uuid::new_v4(),
        })
    }

    fn agent_status() -> BusEvent {
        BusEvent::new(BusEventKind::AgentStatusChanged {
            agent_id: uuid::Uuid::new_v4(),
            status: AgentStatus::Coding,
        })
    }

    fn seed(state: &Arc<AppState>, tasks: usize, statuses: usize) {
        let journal = state.journal();
        for i in 0..tasks {
            journal
                .publish(&task_received(&format!("prompt {i}")))
                .expect("publish task");
        }
        for _ in 0..statuses {
            journal.publish(&agent_status()).expect("publish status");
        }
    }

    async fn get_page(app: Router, uri: &str) -> (StatusCode, serde_json::Value) {
        let req = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        let status = res.status();
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or_default();
        (status, json)
    }

    #[tokio::test]
    async fn happy_path_returns_full_payload_with_total() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        seed(&state, 1, 0);
        let (status, json) = get_page(app(state), "/hud/journal").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["total"], 1);
        assert_eq!(json["limit"], DEFAULT_LIMIT);
        assert_eq!(json["offset"], 0);
        let entries = json["entries"].as_array().expect("entries array");
        assert_eq!(entries.len(), 1);
        let payload = &entries[0]["payload"];
        assert_eq!(payload["type"], "task_received");
        assert!(payload["raw_prompt"].as_str().unwrap().contains("prompt 0"));
    }

    #[tokio::test]
    async fn pagination_slices_newest_first() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        seed(&state, 5, 0);
        let (status, json) = get_page(app(state), "/hud/journal?limit=2&offset=1").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["total"], 5);
        assert_eq!(json["limit"], 2);
        assert_eq!(json["offset"], 1);
        let entries = json["entries"].as_array().expect("entries array");
        assert_eq!(entries.len(), 2);
        let first: i64 = entries[0]["id"].as_i64().expect("id");
        let second: i64 = entries[1]["id"].as_i64().expect("id");
        assert!(first > second, "newest-first ordering");
    }

    #[tokio::test]
    async fn offset_past_total_returns_empty_page_with_total() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        seed(&state, 2, 0);
        let (status, json) = get_page(app(state), "/hud/journal?limit=10&offset=99").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["total"], 2);
        assert!(json["entries"].as_array().expect("entries").is_empty());
    }

    #[tokio::test]
    async fn kind_filter_isolates_one_stream() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        seed(&state, 2, 1);
        let (status, json) =
            get_page(app(Arc::clone(&state)), "/hud/journal?kind=task_received").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["total"], 2);
        assert_eq!(json["entries"].as_array().expect("entries").len(), 2);
        let (status, json) = get_page(app(state), "/hud/journal?kind=agent_status").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["total"], 1);
        assert_eq!(json["entries"].as_array().expect("entries").len(), 1);
    }

    #[tokio::test]
    async fn zero_limit_is_400() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let (status, _) = get_page(app(state), "/hud/journal?limit=0").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn oversized_limit_is_400() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let uri = format!("/hud/journal?limit={}", MAX_LIMIT + 1);
        let (status, _) = get_page(app(state), &uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn negative_offset_is_400() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let (status, _) = get_page(app(state), "/hud/journal?offset=-1").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn oversized_kind_is_400() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let long = "k".repeat(MAX_KIND_LEN + 1);
        let uri = format!("/hud/journal?kind={long}");
        let (status, _) = get_page(app(state), &uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn malformed_limit_is_rejected_with_4xx() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let (status, _) = get_page(app(state), "/hud/journal?limit=abc").await;
        let code = status.as_u16();
        assert!(
            code == 400 || code == 422,
            "expected 4xx rejection, got {code}"
        );
    }
}
