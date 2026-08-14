// Atlas OS — HUD graph read route (RFC 28 §C item 7, RFC 24 §3.3
// graph cards).
//
//   GET /graph/:mission_id   200 { mission_id, nodes, edges }
//                          | 404 when no M15 rows exist for the mission
//                          | 400 when mission_id is malformed
//
// The handler is intentionally a thin transport: it validates the
// path parameter, delegates to `Journal::read_mission_graph`, and
// serialises the canonical `MissionGraph` (already Serialize via
// `graph::mod`). The HUD frontend `<GraphView>` consumes this JSON.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::core::state::AppState;

/// UUIDs are 36 chars; accept a small margin for non-UUID host ids but
/// reject anything that would make a poor audit-trail key. Mirrors the
/// autoresearch route's policy.
const MAX_MISSION_ID_LEN: usize = 64;

/// Response body — canonical `MissionGraph` already serialises to
/// `{ mission_id, nodes, edges }`; this is just the type alias.
pub type GraphResponse = crate::graph::MissionGraph;

pub async fn get_graph(
    State(state): State<Arc<AppState>>,
    Path(mission_id): Path<String>,
) -> Result<Json<GraphResponse>, (StatusCode, String)> {
    let trimmed = mission_id.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_MISSION_ID_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            "mission_id must be 1..=64 chars".to_string(),
        ));
    }
    let graph = state
        .journal()
        .read_mission_graph(trimmed)
        .map_err(|e| {
            tracing::error!(error = %e, mission_id = %trimmed, "HUD get_graph: read_mission_graph failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "graph read failed".to_string(),
            )
        })?;
    match graph {
        Some(g) => Ok(Json(g)),
        None => Err((StatusCode::NOT_FOUND, "no graph for mission".to_string())),
    }
}

// NOTE: HTTP-level integration tests for this route live below.
// The unit-level read path is covered in `journal::mission_graph::tests`.

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
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
            .route("/graph/:mission_id", get(get_graph))
            .with_state(state)
    }

    fn get_req(id: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(format!("/graph/{id}"))
            .body(Body::empty())
            .unwrap()
    }

    fn seed_mission(state: &AppState, id: &str) {
        let j = state.journal();
        j.create_mission(uuid::Uuid::parse_str(id).unwrap(), "test mission")
            .unwrap();
        j.seed_test_graph_node(id, "n-root", "mission", "root", "EXTRACTED")
            .unwrap();
    }

    #[test]
    fn mission_id_policy_constant_allows_uuid() {
        // 36-char UUID fits; pin the policy so a careless edit doesn't
        // silently narrow the input. We use a non-assert equality so
        // clippy:assertions_on_constants stays quiet while still failing
        // to link if the const flips to something unsafe.
        const _: () = {
            assert!(MAX_MISSION_ID_LEN >= 36);
        };
        let _ = MAX_MISSION_ID_LEN;
    }

    #[tokio::test]
    async fn missing_mission_returns_404() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let res = app
            .oneshot(get_req("00000000-0000-0000-0000-000000000001"))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn empty_mission_id_is_400() {
        // axum's `:mission_id` matcher never matches the empty string
        // (it requires at least one char), so this is a sanity check
        // that the handler-policy itself is wired — exercised via the
        // space-padded URI below which DOES match the segment.
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let res = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/graph/%20")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn oversized_mission_id_is_400() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let app = app(state);
        let long = "x".repeat(MAX_MISSION_ID_LEN + 1);
        let res = app.oneshot(get_req(&long)).await.unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn existing_mission_returns_200_with_graph_json() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let id = "00000000-0000-0000-0000-000000000001";
        seed_mission(&state, id);
        let app = app(state);
        let res = app.oneshot(get_req(id)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["mission_id"], id);
        assert!(parsed["nodes"].is_array());
        assert!(!parsed["nodes"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn graph_payload_roundtrips_node_kind_and_provenance() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let id = "00000000-0000-0000-0000-000000000002";
        seed_mission(&state, id);
        let app = app(state);
        let res = app.oneshot(get_req(id)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let first = &parsed["nodes"][0];
        assert_eq!(first["kind"], "mission");
        assert_eq!(first["provenance"], "EXTRACTED");
        assert_eq!(first["label"], "root");
    }
}
