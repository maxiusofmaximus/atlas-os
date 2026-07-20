// OpenCode OS — Journal schema migrations (Phase 0, Roadmap Fase 0).
// Each migration is idempotent. KERNEL_BUS event_id is the idempotency key —
// re-publishing the same event is silently dropped (NOT replaced), per
// RFC 02 §3.1.2 (at-least-once delivery with idempotent consumers).

use anyhow::Result;
use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> Result<()> {
    // M0 — Schema versioning.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version   INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )?;
    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if current < 1 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS journal_events (
                id                INTEGER PRIMARY KEY AUTOINCREMENT,
                event_id          TEXT NOT NULL UNIQUE,
                idempotency_key   TEXT NOT NULL,
                ts                TEXT NOT NULL,
                kind              TEXT NOT NULL,
                payload           TEXT NOT NULL
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_journal_events_event_id  ON journal_events(event_id);
            CREATE INDEX IF NOT EXISTS idx_journal_events_ts   ON journal_events(ts);
            CREATE INDEX IF NOT EXISTS idx_journal_events_kind ON journal_events(kind);

            CREATE TABLE IF NOT EXISTS missions (
                id          TEXT PRIMARY KEY,
                label       TEXT NOT NULL,
                status      TEXT NOT NULL DEFAULT 'received',
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS subagents (
                id           TEXT PRIMARY KEY,
                mission_id   TEXT NOT NULL REFERENCES missions(id),
                role         TEXT NOT NULL,
                model_id     TEXT,
                status       TEXT NOT NULL,
                worktree     TEXT,
                tokens_in    INTEGER NOT NULL DEFAULT 0,
                tokens_out   INTEGER NOT NULL DEFAULT 0,
                cost_usd     REAL NOT NULL DEFAULT 0,
                heartbeat_ts TEXT,
                created_at   TEXT NOT NULL,
                updated_at   TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS approvals (
                id         TEXT PRIMARY KEY,
                agent_id   TEXT NOT NULL,
                action     TEXT NOT NULL,
                status     TEXT NOT NULL DEFAULT 'pending',
                decision_by TEXT,
                decision_at TEXT,
                reason     TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audit_log (
                seq           INTEGER PRIMARY KEY AUTOINCREMENT,
                ts            TEXT NOT NULL,
                actor         TEXT NOT NULL,
                action        TEXT NOT NULL,
                inputs_json   TEXT,
                outputs_json  TEXT,
                previous_hash BLOB,
                this_hash     BLOB NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_audit_log_ts ON audit_log(ts);

            CREATE TABLE IF NOT EXISTS research_runs (
                id          TEXT PRIMARY KEY,
                query       TEXT NOT NULL,
                confidence  REAL,
                outcome     TEXT,
                created_at  TEXT NOT NULL,
                completed_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_research_runs_query ON research_runs(query);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![1, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    Ok(())
}
