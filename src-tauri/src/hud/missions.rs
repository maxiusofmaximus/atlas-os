// Atlas OS — `POST /hud/missions` (RFC 67 §20 H-09, RFC 24 §2 Mission
// Rail).
//
// Seeds a mission row from a raw prompt so the HUD `+ New` action has a
// REST surface (v1 shelled out to `atlas mission new`). The handler is a
// thin transport: it validates the prompt, mints a mission id, persists
// the row, and returns `{ mission_id, status }`. Driving the pipeline
// stays the supervisor's job; this only creates the mission record.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Query, State};
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

#[derive(Debug, Deserialize)]
pub struct MissionsQuery {
    pub sort: Option<String>,
}

pub async fn get_missions(
    State(state): State<Arc<AppState>>,
    Query(q): Query<MissionsQuery>,
) -> Json<Value> {
    let sort = q.sort.as_deref().unwrap_or("recent");
    let missions = state.journal().list_missions().unwrap_or_default();
    let mut rows: Vec<(crate::journal::Mission, f64)> = if sort == "frecency" {
        let scores: HashMap<String, f64> = state
            .journal()
            .mission_frecency(200)
            .unwrap_or_default()
            .into_iter()
            .collect();
        missions
            .into_iter()
            .map(|m| {
                let score = scores.get(&m.id.to_string()).copied().unwrap_or(0.0);
                (m, score)
            })
            .collect()
    } else {
        missions.into_iter().map(|m| (m, 0.0)).collect()
    };
    if sort == "frecency" {
        rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    }
    let out: Vec<Value> = rows
        .iter()
        .map(|(m, score)| {
            json!({
                "id": m.id.to_string(),
                "label": m.label,
                "status": m.status,
                "frecency": score,
            })
        })
        .collect();
    Json(json!({ "sort": sort, "count": out.len(), "missions": out }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bus::{BusEvent, BusEventKind, Confidence};
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

    fn consolidate(state: &Arc<AppState>, mission_id: uuid::Uuid) {
        state
            .publish_and_broadcast(&BusEvent::new(BusEventKind::MissionConsolidated {
                mission_id,
                verdict_id: uuid::Uuid::new_v4(),
                confidence: Confidence::High,
            }))
            .unwrap();
    }

    #[tokio::test]
    async fn frecency_orders_missions_by_activity() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let busy = uuid::Uuid::new_v4();
        let quiet = uuid::Uuid::new_v4();
        state.journal().create_mission(busy, "busy").unwrap();
        state.journal().create_mission(quiet, "quiet").unwrap();
        for _ in 0..3 {
            consolidate(&state, busy);
        }
        consolidate(&state, quiet);

        let Json(v) = get_missions(
            State(state),
            Query(MissionsQuery {
                sort: Some("frecency".into()),
            }),
        )
        .await;
        assert_eq!(v["count"], 2);
        assert_eq!(v["sort"], "frecency");
        assert_eq!(v["missions"][0]["id"], busy.to_string());
    }

    #[tokio::test]
    async fn empty_journal_keeps_a_stable_order() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let a = uuid::Uuid::new_v4();
        let b = uuid::Uuid::new_v4();
        state.journal().create_mission(a, "first").unwrap();
        state.journal().create_mission(b, "second").unwrap();
        let expected: Vec<String> = state
            .journal()
            .list_missions()
            .unwrap()
            .iter()
            .map(|m| m.id.to_string())
            .collect();

        let Json(v) = get_missions(
            State(state),
            Query(MissionsQuery {
                sort: Some("frecency".into()),
            }),
        )
        .await;
        let got: Vec<String> = v["missions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(got, expected);
    }
}
