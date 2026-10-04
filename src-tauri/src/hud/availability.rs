// Atlas OS — `GET /hud/availability` (RFC 20 Fase 23 v3.1.2.2).
//
// Exposes the persisted proactive-turn policy, the current availability and the
// pending mission to the HUD. Read-only.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::core::state::AppState;

pub async fn get_availability(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    // Scope each Journal lock so we never hold it across a second lock
    // (`context_availability` locks the Journal itself).
    let row = {
        let journal = state.journal();
        journal
            .load_proactive_policy()
            .ok()
            .flatten()
            .unwrap_or_default()
    };
    let pending = {
        let journal = state.journal();
        journal.next_pending_mission().ok().flatten()
    };
    let availability = if row.enabled {
        let policy = crate::planning::availability::TurnPolicy {
            eta_ms: row.eta_ms,
            weight_threshold: row.weight_threshold,
            horizon_ms: row.horizon_ms,
        };
        state.context_availability(&policy).ok()
    } else {
        None
    };

    Json(json!({
        "enabled": row.enabled,
        "policy": row,
        "availability": availability,
        "pending_mission": pending,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::ProactivePolicyRow;

    #[tokio::test]
    async fn reports_policy_and_pending_mission() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = crate::journal::Journal::open(dir.path()).unwrap();
        journal
            .save_proactive_policy(&ProactivePolicyRow {
                eta_ms: 600_000,
                weight_threshold: 1.0,
                horizon_ms: 3_600_000,
                enabled: true,
            })
            .unwrap();
        let mission = uuid::Uuid::new_v4();
        journal.create_mission(mission, "queued").unwrap();
        let state = Arc::new(AppState::from_journal_in(journal, dir.path().to_path_buf()));

        let Json(value) = get_availability(State(state)).await;
        assert_eq!(value["enabled"], true);
        assert_eq!(value["policy"]["eta_ms"], 600_000);
        assert_eq!(value["pending_mission"], mission.to_string());
        assert!(value["availability"].is_string() || value["availability"].is_object());
    }
}
