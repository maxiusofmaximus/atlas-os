// Atlas OS — `GET /hud/cost` (RFC 65 §3, Cost & Res view).
//
// Powers `<CostDashboard>`: window totals + per-model roll-up from
// `model_invocations`, the cumulative spend with its pressure level, and the
// pending `model_resets` windows (RFC 28 §H) so the operator sees both what has
// been spent and which providers are rate-limited right now. Read-only.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::core::state::AppState;
use crate::monitor::{classify_cost, Pressure, MONITOR_COST_CRIT_USD, MONITOR_COST_WARN_USD};

#[derive(Debug, Deserialize)]
pub struct CostQuery {
    /// Number of most-recent invocations to aggregate (1..=5000, default 200).
    pub window: Option<i64>,
}

fn pressure_label(p: Pressure) -> &'static str {
    match p {
        Pressure::Ok => "ok",
        Pressure::Warn => "warn",
        Pressure::Critical => "critical",
    }
}

pub async fn get_cost(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CostQuery>,
) -> Json<serde_json::Value> {
    let window = q.window.unwrap_or(200).clamp(1, 5000);
    let journal = state.journal();
    let totals = journal.cost_totals(window).unwrap_or_default();
    let by_model = journal.cost_by_model(window).unwrap_or_default();
    let cumulative_usd = journal.total_model_cost_usd().unwrap_or(0.0);
    let pending = journal
        .pending_model_resets(chrono::Utc::now())
        .unwrap_or_default();
    drop(journal);

    let pending_json: Vec<serde_json::Value> = pending
        .iter()
        .map(|r| {
            json!({
                "provider": r.provider,
                "model": r.model,
                "status_code": r.status_code,
                "error_type": r.error_type,
                "resets_at_ms": r.resets_at,
            })
        })
        .collect();

    Json(json!({
        "window": window,
        "totals": totals,
        "by_model": by_model,
        "cumulative_usd": cumulative_usd,
        "pressure": {
            "level": pressure_label(classify_cost(cumulative_usd)),
            "warn_usd": MONITOR_COST_WARN_USD,
            "crit_usd": MONITOR_COST_CRIT_USD,
        },
        "pending_resets": pending_json,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::model_invocation::ModelInvocationRow;

    fn invocation(id: &str, model: &str, ts: &str, cost: f64) -> ModelInvocationRow {
        ModelInvocationRow {
            id: id.into(),
            mission_id: None,
            model_id: model.into(),
            deployment_id: "dep".into(),
            provider: "p".into(),
            idempotency_key: format!("idem-{id}"),
            started_at: ts.into(),
            finished_at: None,
            latency_ms: Some(50),
            tokens_in: Some(10),
            tokens_out: Some(2),
            cache_read_input_tokens: None,
            cost_usd: Some(cost),
            seed: None,
            temperature: None,
            sampling_params_json: None,
            route_taken_json: None,
            was_correct: None,
            error_kind: None,
            error_message: None,
        }
    }

    #[tokio::test]
    async fn reports_window_totals_and_models() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = crate::journal::Journal::open(dir.path()).unwrap();
        journal
            .record_model_invocation(&invocation("a", "m1", "2026-01-01T00:00:01Z", 0.25))
            .unwrap();
        journal
            .record_model_invocation(&invocation("b", "m2", "2026-01-01T00:00:02Z", 0.75))
            .unwrap();
        let state = Arc::new(AppState::from_journal_in(journal, dir.path().to_path_buf()));

        let Json(v) = get_cost(State(state), Query(CostQuery { window: Some(50) })).await;
        assert_eq!(v["window"], 50);
        assert_eq!(v["totals"]["invocations"], 2);
        assert!((v["totals"]["cost_usd"].as_f64().unwrap() - 1.0).abs() < 1e-9);
        assert_eq!(v["by_model"].as_array().unwrap().len(), 2);
        // Costliest model first.
        assert_eq!(v["by_model"][0]["model_id"], "m2");
        assert!((v["cumulative_usd"].as_f64().unwrap() - 1.0).abs() < 1e-9);
        assert_eq!(v["pressure"]["level"], "ok");
        assert!(v["pending_resets"].as_array().unwrap().is_empty());
    }
}
