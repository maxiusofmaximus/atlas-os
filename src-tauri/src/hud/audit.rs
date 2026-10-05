// Atlas OS — `GET /hud/audit` (RFC 65 §3, Audit view; RFC 24 §10).
//
// Powers `<AuditTimeline>`: the append-only `audit_log` chain, newest first.
// Read-only. The rows carry `previous_hash`/`this_hash` in the schema; this
// endpoint exposes the decoded entry (seq/ts/actor/action/inputs/outputs) — the
// hash-chain verification pass (RFC 24 §10) is not implemented yet and is NOT
// claimed here.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::core::state::AppState;

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    /// Number of most-recent audit entries to return (1..=1000, default 50).
    pub last: Option<i64>,
}

pub async fn get_audit(
    State(state): State<Arc<AppState>>,
    Query(q): Query<AuditQuery>,
) -> Json<Value> {
    let last = q.last.unwrap_or(50).clamp(1, 1000);
    let journal = state.journal();
    let entries = journal.audit_tail(last).unwrap_or_default();
    drop(journal);
    Json(json!({ "rows": entries, "count": entries.len() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_audit_log_returns_no_rows() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = crate::journal::Journal::open(dir.path()).unwrap();
        let state = Arc::new(AppState::from_journal_in(journal, dir.path().to_path_buf()));

        let Json(v) = get_audit(State(state), Query(AuditQuery { last: Some(10) })).await;
        assert_eq!(v["count"], 0);
        assert!(v["rows"].as_array().unwrap().is_empty());
    }
}
