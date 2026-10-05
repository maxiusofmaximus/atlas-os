// Atlas OS — `GET /hud/health` (RFC 65 §3, Health KPIs view).
//
// Powers `<HealthKPIs>`: agent-session telemetry (`agent_session_events`, M14)
// with its `event_type` histogram and a recent tail, the capability-layer run
// states (`agent_runs`, M49) and the live swarm registry states
// (`swarm_agents`, M29). Read-only. Supervisor heartbeat liveness is a live WS
// signal (`agent_heartbeat`) and is projected in the component, not persisted.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::core::state::AppState;

#[derive(Debug, Deserialize)]
pub struct HealthQuery {
    /// Number of most-recent agent-session events to include (1..=200, default 20).
    pub last: Option<i64>,
}

pub async fn get_health(
    State(state): State<Arc<AppState>>,
    Query(q): Query<HealthQuery>,
) -> Json<Value> {
    let last = q.last.unwrap_or(20).clamp(1, 200);
    let journal = state.journal();
    let total = journal.agent_session_event_count().unwrap_or(0);
    let by_type = journal
        .agent_session_event_type_counts()
        .unwrap_or_default();
    let recent = journal.agent_session_events_tail(last).unwrap_or_default();
    let run_states = journal.agent_run_status_counts().unwrap_or_default();
    let swarm_states = journal.swarm_agent_state_counts().unwrap_or_default();
    drop(journal);

    let recent_json: Vec<Value> = recent
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "ts": r.ts,
                "pane_id": r.pane_id,
                "event_type": r.event_type,
                "agent": r.agent,
                "task_id": r.task_id,
            })
        })
        .collect();
    let runs_json: Vec<Value> = run_states
        .iter()
        .map(|(status, count)| json!({ "status": status, "count": count }))
        .collect();
    let swarm_json: Vec<Value> = swarm_states
        .iter()
        .map(|(state_name, count)| json!({ "state": state_name, "count": count }))
        .collect();

    Json(json!({
        "agent_events": { "total": total, "by_type": by_type, "recent": recent_json },
        "agent_runs": runs_json,
        "swarm_agents": swarm_json,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reports_event_counts_and_run_states() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = crate::journal::Journal::open(dir.path()).unwrap();
        journal
            .insert_agent_session_event(1, None, "agent.started", "opencode", None, "{}")
            .unwrap();
        journal
            .insert_agent_session_event(2, None, "agent.started", "opencode", None, "{}")
            .unwrap();
        journal
            .insert_agent_session_event(3, None, "agent.error", "opencode", None, "{}")
            .unwrap();
        journal
            .create_agent_run(&crate::journal::AgentRunRow {
                id: "r1".into(),
                mission_id: None,
                goal: "g".into(),
                success_predicate: "[]".into(),
                sandbox: "local".into(),
                status: "done".into(),
                steps: 1,
                tokens_in: 0,
                tokens_out: 0,
                cost_usd: 0.0,
                ts_started: 1,
                ts_ended: Some(2),
            })
            .unwrap();
        let state = Arc::new(AppState::from_journal_in(journal, dir.path().to_path_buf()));

        let Json(v) = get_health(State(state), Query(HealthQuery { last: Some(10) })).await;
        assert_eq!(v["agent_events"]["total"], 3);
        assert_eq!(
            v["agent_events"]["by_type"][0]["event_type"],
            "agent.started"
        );
        assert_eq!(v["agent_events"]["by_type"][0]["count"], 2);
        assert_eq!(v["agent_events"]["recent"].as_array().unwrap().len(), 3);
        assert_eq!(v["agent_runs"][0]["status"], "done");
        assert_eq!(v["agent_runs"][0]["count"], 1);
    }
}
