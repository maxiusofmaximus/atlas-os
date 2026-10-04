// Atlas OS — `GET /hud/reliability` (RFC 20 Fase 24 v24.2).
//
// Exposes the persisted reliability-gate policy plus the per-model reliability
// map (from the EVAL store) to the HUD. Read-only.

use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::core::state::AppState;

pub async fn get_reliability(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let journal = state.journal();
    let policy = journal
        .load_reliability_gate()
        .ok()
        .flatten()
        .unwrap_or_default();
    let models = journal.eval_models().unwrap_or_default();
    let reliabilities =
        crate::orchestrator::reliability_gate::reliabilities_from_journal(&journal, &models, 20);
    Json(json!({ "policy": policy, "models": reliabilities }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{EvalCaseInput, EvalRunStart, ReliabilityGateRow};

    #[tokio::test]
    async fn reports_policy_and_model_reliabilities() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = crate::journal::Journal::open(dir.path()).unwrap();
        journal
            .save_reliability_gate(&ReliabilityGateRow {
                min_samples: 5,
                min_pass_rate: 0.7,
                allow_unknown: false,
                enabled: true,
            })
            .unwrap();
        let run = journal
            .eval_run_start(&EvalRunStart {
                suite: "golden",
                agent: "atlas",
                harness: "atlas-local",
                model: Some("m1"),
                metadata_json: None,
            })
            .unwrap();
        journal
            .eval_run_record_case(&EvalCaseInput {
                run_id: &run,
                case_id: "c1",
                category: Some("SYS"),
                status: "pass",
                duration_ms: 0,
                turns: 0,
                no_action_turns: 0,
                tokens_in: 0,
                tokens_out: 0,
                cost_usd: 0.0,
                failure_kind: None,
                detail: None,
            })
            .unwrap();

        let state = Arc::new(AppState::from_journal_in(journal, dir.path().to_path_buf()));
        let Json(value) = get_reliability(State(state)).await;
        assert_eq!(value["policy"]["enabled"], true);
        assert_eq!(value["policy"]["min_pass_rate"], 0.7);
        assert_eq!(value["models"]["m1"]["n"], 1);
    }
}
