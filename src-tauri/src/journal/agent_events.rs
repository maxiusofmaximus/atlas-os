// OpenCode OS — `agent_session_events` row + persistence helpers
// (RFC 28 §B, M14 migration, item 5).
//
// M14 populates this table with one row per OSC 9001 envelope consumed
// from `wtcli listen --json` by the ACP `listen_worker` (Channel 2 per
// `src-tauri/specs/osc-9001.md`). The table itself is NOT feature-gated
// (a default build still creates it — HUD can read whatever telemetry
// exists even on a non-Windows host), so the row type, the query, and
// the insert fn all live in this public module with no `#[cfg(..)]
// shipping-gate`. The lifecycle that poves the rows (the wtcli listen
// subprocess) IS gated behind `acp-server` in `acp/listen_worker.rs`.
//
// Per RFC 28 §B the table deliberately omits `mission_id`: the linkage
// from agent_session_events to a mission is deduced by the host from
// `task_id` <-> `steps.id` joins done at query-time by the HUD card,
// not stored as a hard FK.

use rusqlite::Connection;

/// Canonical OSC 9001 envelope persisted as a row of `agent_session_events`.
/// Mirrors the SQL schema 1:1 so callers (HUD, replay, audit) can read the
/// row directly without an extra translation layer.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentSessionEventRow {
    pub id: i64,
    /// Event wall-clock time in seconds since the UNIX epoch. The OSC 9001
    /// envelope carries ISO-8601 strings in `params.timestamp`; the listener
    /// worker normalises them to `i64` here so range queries on `idx_ase_ts`
    /// stay cheap.
    pub ts: i64,
    /// Source pane id. `None` on events that WT emits without a source pane
    /// (e.g. `agent.idle` on window close).
    pub pane_id: Option<String>,
    /// Namespaced event type, e.g. `agent.task.completed`, `agent.error`,
    /// `copilot.plan.updated`.
    pub event_type: String,
    /// Self-reported agent identity (e.g. `opencode`, `copilot-cli`).
    pub agent: String,
    /// Task id when the event is task-scoped (`agent.task.*`, `agent.tool.*`,
    /// `agent.error` with `task_id`); `None` for `agent.started` /
    /// `agent.idle`.
    pub task_id: Option<String>,
    /// Raw `params` object of the OSC 9001 `agent_event` envelope kept
    /// verbatim so any future structured query has the full payload without
    /// a schema migration.
    pub payload_json: String,
}

/// Insert one envelope into `agent_session_events`. Returns the row id.
/// Caller holds the connection lock via `Journal::conn` (or the
/// `conn: &Connection` of a fresh test journal). The fn does NOT publish
/// a BusEvent — the host loop that spawns the listener worker is the
/// only writer; consumers read via `agent_session_events_tail` /
/// `agent_session_events_for_pane`.
pub fn insert_agent_session_event(
    conn: &Connection,
    ts: i64,
    pane_id: Option<&str>,
    event_type: &str,
    agent: &str,
    task_id: Option<&str>,
    payload_json: &str,
) -> anyhow::Result<i64> {
    let id = conn.execute(
        "INSERT INTO agent_session_events
            (ts, pane_id, event_type, agent, task_id, payload_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![ts, pane_id, event_type, agent, task_id, payload_json],
    )?;
    let rowid = conn.last_insert_rowid();
    debug_assert!(id == 1, "exactly one row must be inserted per call");
    Ok(rowid)
}

/// Read the most recent `last` rows of `agent_session_events`, newest
/// first. Mirrors the shape of `Journal::audit_tail` /
/// `Journal::step_state_tail` so the HUD card can swap in the same
/// rendering code path.
pub fn agent_session_events_tail(
    conn: &Connection,
    last: i64,
) -> anyhow::Result<Vec<AgentSessionEventRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, ts, pane_id, event_type, agent, task_id, payload_json
         FROM agent_session_events
         ORDER BY ts DESC, id DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(rusqlite::params![last.max(0)], |row| {
        Ok(AgentSessionEventRow {
            id: row.get(0)?,
            ts: row.get(1)?,
            pane_id: row.get(2)?,
            event_type: row.get(3)?,
            agent: row.get(4)?,
            task_id: row.get(5)?,
            payload_json: row.get(6)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// Read all rows for a given `pane_id`, newest first. Backs the
/// per-pane HUD card that lets an operator scrub agent.tool.invoked /
/// agent.tool.completed pairs in chronological order.
pub fn agent_session_events_for_pane(
    conn: &Connection,
    pane_id: &str,
) -> anyhow::Result<Vec<AgentSessionEventRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, ts, pane_id, event_type, agent, task_id, payload_json
         FROM agent_session_events
         WHERE pane_id = ?1
         ORDER BY ts DESC, id DESC",
    )?;
    let rows = stmt.query_map(rusqlite::params![pane_id], |row| {
        Ok(AgentSessionEventRow {
            id: row.get(0)?,
            ts: row.get(1)?,
            pane_id: row.get(2)?,
            event_type: row.get(3)?,
            agent: row.get(4)?,
            task_id: row.get(5)?,
            payload_json: row.get(6)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::schema::migrate;
    use tempfile::TempDir;

    fn fresh_conn() -> (TempDir, Connection) {
        let tmp = TempDir::new().expect("tmp");
        let conn = Connection::open(tmp.path().join("j.db")).expect("open");
        migrate(&conn).expect("migrate");
        (tmp, conn)
    }

    #[test]
    fn insert_emits_a_row_and_returns_rowid() {
        let (_tmp, conn) = fresh_conn();
        let id = insert_agent_session_event(
            &conn,
            1_700_000_000,
            Some("42"),
            "agent.task.started",
            "opencode",
            Some("abc-123"),
            r#"{"task_id":"abc-123","description":"build auth"}"#,
        )
        .expect("insert");
        assert!(id > 0);
        let tail = agent_session_events_tail(&conn, 10).expect("tail");
        assert_eq!(tail.len(), 1);
        let row = &tail[0];
        assert_eq!(row.id, id);
        assert_eq!(row.pane_id.as_deref(), Some("42"));
        assert_eq!(row.event_type, "agent.task.started");
        assert_eq!(row.agent, "opencode");
        assert_eq!(row.task_id.as_deref(), Some("abc-123"));
        assert!(row.payload_json.contains("abc-123"));
    }

    #[test]
    fn tail_returns_newest_first() {
        let (_tmp, conn) = fresh_conn();
        insert_agent_session_event(&conn, 1, None, "agent.started", "opencode", None, "{}")
            .expect("i1");
        insert_agent_session_event(&conn, 3, None, "agent.idle", "opencode", None, "{}")
            .expect("i3");
        insert_agent_session_event(&conn, 2, None, "agent.error", "opencode", None, "{}")
            .expect("i2");
        let tail = agent_session_events_tail(&conn, 5).expect("tail");
        let ts_sequence: Vec<i64> = tail.iter().map(|r| r.ts).collect();
        assert_eq!(ts_sequence, vec![3, 2, 1]);
    }

    #[test]
    fn tail_respects_last_limit_clamped_to_zero() {
        let (_tmp, conn) = fresh_conn();
        insert_agent_session_event(&conn, 1, None, "agent.started", "opencode", None, "{}")
            .expect("i");
        let tail = agent_session_events_tail(&conn, 0).expect("tail");
        assert!(tail.is_empty(), "LIMIT 0 must return zero rows");
    }

    #[test]
    fn for_pane_filters_to_just_one_pane() {
        let (_tmp, conn) = fresh_conn();
        insert_agent_session_event(
            &conn,
            1,
            Some("3"),
            "agent.task.started",
            "copilot",
            Some("t1"),
            "{}",
        )
        .expect("i1");
        insert_agent_session_event(
            &conn,
            2,
            Some("4"),
            "agent.task.started",
            "copilot",
            Some("t2"),
            "{}",
        )
        .expect("i2");
        insert_agent_session_event(
            &conn,
            3,
            Some("3"),
            "agent.tool.invoked",
            "copilot",
            Some("t1"),
            "{}",
        )
        .expect("i3");
        let rows = agent_session_events_for_pane(&conn, "3").expect("rows");
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.pane_id.as_deref() == Some("3")));
    }

    #[test]
    fn insert_preserves_payload_json_verbatim() {
        let (_tmp, conn) = fresh_conn();
        let payload =
            r#"{"task_id":"abc-123","tool":"bash","exit_code":0,"weird":"' \\\n unicode: ñ"}"#;
        insert_agent_session_event(
            &conn,
            1700000000,
            Some("3"),
            "agent.tool.completed",
            "copilot-cli",
            Some("abc-123"),
            payload,
        )
        .expect("insert");
        let rows = agent_session_events_tail(&conn, 10).expect("tail");
        assert_eq!(rows[0].payload_json, payload);
    }

    #[test]
    fn none_pane_and_none_task_round_trip() {
        let (_tmp, conn) = fresh_conn();
        insert_agent_session_event(&conn, 1, None, "agent.idle", "opencode", None, "{}")
            .expect("i");
        let rows = agent_session_events_tail(&conn, 5).expect("tail");
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert!(row.pane_id.is_none());
        assert!(row.task_id.is_none());
    }
}
