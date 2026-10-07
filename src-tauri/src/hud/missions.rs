// Atlas OS — `POST /hud/missions` (RFC 67 §20 H-09, RFC 24 §2 Mission
// Rail).
//
// Seeds a mission row from a raw prompt so the HUD `+ New` action has a
// REST surface (v1 shelled out to `atlas mission new`). The handler is a
// thin transport: it validates the prompt, mints a mission id, persists
// the row, and returns `{ mission_id, status }`. Driving the pipeline
// stays the supervisor's job; this only creates the mission record.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::core::state::AppState;

const MAX_PROMPT_LEN: usize = 20_000;

#[derive(Debug, Deserialize)]
pub struct NewMissionRequest {
    pub prompt: String,
}

fn bad_request(message: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message })))
}

pub async fn post_mission(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NewMissionRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let prompt = req.prompt.trim();
    if prompt.is_empty() {
        return Err(bad_request("prompt must not be empty"));
    }
    if prompt.len() > MAX_PROMPT_LEN {
        return Err(bad_request("prompt too long"));
    }

    let mission_id = uuid::Uuid::new_v4();
    state
        .journal()
        .create_mission(mission_id, prompt)
        .map_err(|e| {
            tracing::error!(error = %e, "HUD post_mission: create_mission failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "mission create failed" })),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "mission_id": mission_id.to_string(), "status": "received" })),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::Journal;

    fn fresh_state(tmp: &tempfile::TempDir) -> Arc<AppState> {
        let journal = Journal::open(tmp.path()).expect("open");
        Arc::new(AppState::from_journal(journal))
    }

    #[tokio::test]
    async fn creates_a_mission_from_a_prompt() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let (status, Json(v)) = post_mission(
            State(state.clone()),
            Json(NewMissionRequest {
                prompt: "add a rate limiter".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(v["status"], "received");
        let id = v["mission_id"].as_str().unwrap();
        let missions = state.journal().list_missions().unwrap();
        assert!(missions.iter().any(|m| m.id.to_string() == id));
    }

    #[tokio::test]
    async fn rejects_blank_prompt() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let err = post_mission(
            State(state),
            Json(NewMissionRequest {
                prompt: "   ".into(),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }
}
