// OpenCode OS — Journal: SQLite + sqlite-vec (RFC 02 §3.4, RFC 25 §3.4).
// WAL mode, single file per profile: `~/.opencode/profiles/<id>/journal.db`.
// Schema is minimal in Phase 0 — Roadmap §Fase 0; expanded in later phases.

pub mod schema;
pub mod store;

#[cfg(test)]
mod tests;

pub use store::{AuditEntry, JournalEntry, Mission};

use std::path::{Path, PathBuf};

use anyhow::Context;
use parking_lot::Mutex;
use rusqlite::Connection;

use crate::core::bus::BusEvent;

pub struct Journal {
    conn: Mutex<Connection>,
    /// Snapshot path kept for diagnostics; written once at open time.
    #[allow(dead_code)]
    db_path: PathBuf,
}

impl Journal {
    pub fn open(profile_root: &Path) -> anyhow::Result<Self> {
        let db_path = profile_root.join("journal.db");
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating journal dir {}", parent.display()))?;
        }
        let conn = Connection::open(&db_path)?;
        // RFC 25 §3.4: WAL + tuned pragmas.
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = 10000;
             PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;",
        )?;
        // Load sqlite-vec extension if available (RFC 25 §3.4).
        // Phase 0: best-effort — provides optional vector table for embeddings.
        // SAFETY: extension lives in the same dir as journal.db or `$(pwd)`.
        #[cfg(feature = "fastembed")]
        {
            let extension_path = std::env::var("OC_SQLITE_VEC_PATH").ok();
            if let Some(p) = extension_path {
                // sqlite-vec's default entry-point symbol: sqlite3_sqlitevec_init
                // (see https://github.com/asg017/sqlite-vec)
                let entry_point: Option<&'static str> = Some("sqlite3_sqlitevec_init");
                match unsafe { conn.load_extension(&p, entry_point) } {
                    Ok(()) => tracing::info!(path = %p, "sqlite-vec loaded"),
                    Err(e) => {
                        tracing::warn!(error = %e, "sqlite-vec load failed; vector queries will fail")
                    }
                }
            }
        }
        schema::migrate(&conn).context("schema migration failed")?;
        Ok(Self {
            conn: Mutex::new(conn),
            db_path,
        })
    }

    pub fn publish(&self, event: &BusEvent) -> anyhow::Result<()> {
        let kind = event.kind.tag();
        let payload = serde_json::to_string(&event.kind)?;
        let conn = self.conn.lock();
        // ON CONFLICT DO NOTHING — RFC 02 §3.1.2 at-least-once delivery with
        // idempotent consumers. Re-publishing the same event_id is silently
        // dropped (NOT replaced — the first copy wins).
        conn.execute(
            "INSERT INTO journal_events (event_id, idempotency_key, ts, kind, payload)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(event_id) DO NOTHING",
            rusqlite::params![
                event.id.to_string(),
                event.idempotency_key,
                event.ts.to_rfc3339(),
                kind,
                payload,
            ],
        )?;
        Ok(())
    }

    /// Persist a new mission row. Idempotent on `id` (will overwrite label/status,
    /// preserve created_at). Used by the `mission new` CLI and the IPC `mission_new`.
    pub fn create_mission(&self, id: uuid::Uuid, label: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO missions (id, label, status, created_at, updated_at)
             VALUES (?1, ?2, 'received', ?3, ?3)
             ON CONFLICT(id) DO UPDATE SET label=excluded.label, updated_at=excluded.updated_at",
            rusqlite::params![id.to_string(), label, now],
        )?;
        Ok(())
    }

    pub fn list_missions(&self) -> anyhow::Result<Vec<Mission>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, label, status FROM missions ORDER BY created_at DESC LIMIT 200")?;
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let parsed = uuid::Uuid::parse_str(&id_str).unwrap_or_else(|e| {
                tracing::warn!(error = %e, raw = %id_str, "missions.id is not a valid UUID; using nil");
                uuid::Uuid::nil()
            });
            Ok(Mission {
                id: parsed,
                label: row.get(1)?,
                status: row.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    pub fn tail(&self, last: i64) -> anyhow::Result<Vec<JournalEntry>> {
        let conn = self.conn.lock();
        // Subquery picks newest N; outer query reorders oldest-first without
        // a Vec::reverse() in Rust.
        let mut stmt = conn.prepare(
            "SELECT id, ts, kind, payload FROM (
                SELECT id, ts, kind, payload FROM journal_events
                ORDER BY id DESC LIMIT ?1
             ) ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            Ok(JournalEntry {
                id: row.get(0)?,
                ts: row.get(1)?,
                kind: row.get(2)?,
                payload: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    /// Tail the append-only audit log (RFC 24 §10). Phase 0 returns the raw
    /// rows; Phase 6 will verify the hash chain when `--verify` is passed.
    pub fn audit_tail(&self, last: i64) -> anyhow::Result<Vec<AuditEntry>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT seq, ts, actor, action, inputs_json, outputs_json
             FROM audit_log ORDER BY seq DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            let inputs_json: Option<String> = row.get(4)?;
            let outputs_json: Option<String> = row.get(5)?;
            Ok(AuditEntry {
                seq: row.get(0)?,
                ts: row.get(1)?,
                actor: row.get(2)?,
                action: row.get(3)?,
                inputs: inputs_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default(),
                outputs: outputs_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default(),
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }
}
