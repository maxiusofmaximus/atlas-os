// Atlas OS — `GET /hud/eval/summary` (RFC 20 Fase 22, EVAL.2).
//
// Exposes the normalized evaluation metrics (`crate::eval::metrics`) to the
// HUD `<EvalCard>`: overall pass rate, tokens/solved, cost/solved and the
// failure-kind histogram, plus the per harness × model breakdown.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::core::state::AppState;

#[derive(Debug, Deserialize)]
pub struct EvalQuery {
    /// Number of most-recent runs to aggregate (1..=512, default 20).
    pub last: Option<i64>,
}

pub async fn get_eval_summary(
    State(state): State<Arc<AppState>>,
    Query(q): Query<EvalQuery>,
) -> Json<serde_json::Value> {
    let limit = q.last.unwrap_or(20).clamp(1, 512);
    let journal = state.journal();
    let summary = crate::eval::metrics::summarize_recent(&journal, limit).unwrap_or_default();
    let groups =
        crate::eval::metrics::summarize_by_harness_model(&journal, limit).unwrap_or_default();
    Json(json!({ "summary": summary, "groups": groups }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{EvalCaseInput, EvalRunStart};

    #[tokio::test]
    async fn summary_reports_seeded_cases() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = crate::journal::Journal::open(dir.path()).unwrap();
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
                duration_ms: 1,
                turns: 1,
                no_action_turns: 0,
                tokens_in: 10,
                tokens_out: 5,
                cost_usd: 0.0,
                failure_kind: None,
                detail: None,
            })
            .unwrap();

        let state = Arc::new(AppState::from_journal_in(journal, dir.path().to_path_buf()));
        let Json(value) = get_eval_summary(State(state), Query(EvalQuery { last: Some(10) })).await;
        assert_eq!(value["summary"]["total"], 1);
        assert_eq!(value["summary"]["passed"], 1);
        assert_eq!(value["groups"][0]["key"], "atlas-local/m1");
    }
}
