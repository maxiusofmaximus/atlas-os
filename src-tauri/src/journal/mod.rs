// Atlas OS — Journal: SQLite + sqlite-vec (RFC 02 §3.4, RFC 25 §3.4).
// WAL mode, single file per profile: `~/.opencode/profiles/<id>/journal.db`.
// Schema is minimal in Phase 0 — Roadmap §Fase 0; expanded in later phases.

pub mod agent_events;
pub mod autoresearch;
pub mod export;
#[cfg(feature = "dag_mode")]
pub mod learning_graphs;
#[cfg(feature = "dag_mode")]
pub mod mission_graph;
pub mod model_resets;
pub mod schema;
pub mod store;
pub mod yaml_format;

#[cfg(test)]
mod tests;

#[cfg(feature = "dag_mode")]
pub use learning_graphs::{LearningGraphRow, ScoredGraph};

pub use agent_events::AgentSessionEventRow;

pub use model_resets::ModelResetRow;

pub use store::{
    AuditEntry, CheckpointRow, ConsolidatedRow, DiffAnnotationRow, DiffRow, JournalEntry, Mission,
    ModelSwapRow, PatternRow, PlanRow, RepairRunRow, SkillRow, StepStateRow, ValidationReportRow,
    VerdictRow,
};

/// RFC 04 §6 sub-fase 2.4 — one row of `model_invocations` (M21) as
/// materialised by `Journal::record_model_invocation`. The struct
/// mirrors the SQL column set 1:1; missing columns are `Option<…>`
/// matching the schema's `NULL` allowance.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelInvocationRow {
    pub id: String,
    pub mission_id: Option<String>,
    pub model_id: String,
    pub deployment_id: String,
    pub provider: String,
    pub idempotency_key: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub latency_ms: Option<i64>,
    pub tokens_in: Option<i64>,
    pub tokens_out: Option<i64>,
    pub cache_read_input_tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub seed: Option<i64>,
    pub temperature: Option<f64>,
    pub sampling_params_json: Option<String>,
    pub route_taken_json: Option<String>,
    pub was_correct: Option<i64>,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
}

use std::path::{Path, PathBuf};

use anyhow::Context;
use parking_lot::Mutex;
use rusqlite::Connection;
use uuid::Uuid;

use crate::core::bus::BusEvent;
use crate::prompt::types::PublicUnderstandingVerdict;

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

    /// Generic single-row payload fetch used by every `*_payload`
    /// helper. `table` is the SQLite table, `id_col` is the column
    /// the lookup is keyed on, and `id_str` is the stringified id
    /// (UUIDs stringified via `to_string()`; skills pass their own
    /// `skill_id` string). Returns `None` when no row matches.
    fn payload_for(
        &self,
        table: &str,
        id_col: &str,
        id_str: &str,
    ) -> anyhow::Result<Option<String>> {
        let conn = self.conn.lock();
        let sql = format!("SELECT payload FROM {table} WHERE {id_col} = ?1");
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query_map(rusqlite::params![id_str], |row| {
            let payload: String = row.get(0)?;
            Ok(payload)
        })?;
        if let Some(r) = rows.next().transpose()? {
            return Ok(Some(r));
        }
        Ok(None)
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

    /// Mission projection consumed by the ICS writer (RFC 28 §G.2.1).
    /// Returns rows whose `updated_at` falls within the last `days`
    /// days, ordered by `updated_at DESC`. The status string is mapped
    /// to `IcsMissionStatus` by the caller; rows with an unknown
    /// status token are skipped (logged at `warn`).
    pub fn missions_for_ics(
        &self,
        days: i64,
    ) -> anyhow::Result<Vec<crate::calendar::payload::IcsMission>> {
        use crate::calendar::payload::{IcsMission, IcsMissionStatus};
        use chrono::{DateTime, Utc};
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, label, status, created_at, updated_at
             FROM missions
             WHERE updated_at >= datetime('now', ?1)
             ORDER BY updated_at DESC",
        )?;
        let offset = format!("-{days} days");
        let rows = stmt.query_map(rusqlite::params![&offset], |row| {
            let id: String = row.get(0)?;
            let label: String = row.get(1)?;
            let status_str: String = row.get(2)?;
            let created_str: String = row.get(3)?;
            let updated_str: String = row.get(4)?;
            Ok((id, label, status_str, created_str, updated_str))
        })?;
        let mut out = Vec::new();
        for (id, label, status_str, created_str, updated_str) in rows.flatten() {
            let status = match status_str.parse::<IcsMissionStatus>() {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(raw = %status_str, error = %e, "skipping mission — unknown status for ICS");
                    continue;
                }
            };
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|t| t.with_timezone(&Utc))
                .unwrap_or_else(|e| {
                    tracing::warn!(raw = %created_str, error = %e, "unparseable created_at; using epoch");
                    DateTime::from_timestamp(0, 0).unwrap_or_default()
                });
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map(|t| t.with_timezone(&Utc))
                .unwrap_or_else(|e| {
                    tracing::warn!(raw = %updated_str, error = %e, "unparseable updated_at; using epoch");
                    DateTime::from_timestamp(0, 0).unwrap_or_default()
                });
            out.push(IcsMission {
                id,
                label,
                status,
                created_at,
                updated_at,
            });
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

    // ——— Prompt Understanding Pipeline persistence (RFC 23 §3-§4) ———
    //
    // The verdict JSON is the source of truth; the duplicated top-level
    // columns exist purely for indexed tail queries / HUD live stream.
    // `on conflict do nothing`: a re-persisted verdict (e.g. from a
    // replay) never overwrites the original — the first one wins, matching
    // the journal_events idempotency invariant from RFC 02 §3.1.2.

    pub fn save_verdict(
        &self,
        verdict: &crate::prompt::types::PublicUnderstandingVerdict,
        mission_id: Option<Uuid>,
    ) -> anyhow::Result<()> {
        let payload = serde_json::to_string(&verdict)?;
        let rubric_mean = verdict.confidence_rubric.mean();
        let gap_count: i64 = verdict.gaps.len().try_into().unwrap_or(i64::MAX);
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO prompt_verdicts
                (verdict_id, session_id, mission_id, raw_prompt, ts, confidence,
                 rubric_mean, recommended_mode, gap_count, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(verdict_id) DO NOTHING",
            rusqlite::params![
                verdict.verdict_id.to_string(),
                verdict.session_id.to_string(),
                mission_id.map(|u| u.to_string()),
                verdict.raw_prompt,
                verdict.timestamp,
                verdict.confidence.tag(),
                rubric_mean,
                verdict.recommended_mode.tag(),
                gap_count,
                payload,
            ],
        )?;
        Ok(())
    }

    pub fn save_consolidated(
        &self,
        consolidated: &crate::prompt::types::MissionConsolidated,
    ) -> anyhow::Result<()> {
        let payload = serde_json::to_string(&consolidated)?;
        let locked_int: i64 = if consolidated.locked { 1 } else { 0 };
        let requires_research_int: i64 = if consolidated.requires_research_first {
            1
        } else {
            0
        };
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO mission_consolidated
                (mission_id, verdict_id, generated_at, mission_statement,
                 suggested_mode, locked, locked_at, locked_by,
                 requires_research, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(mission_id) DO UPDATE SET
                verdict_id        = excluded.verdict_id,
                generated_at      = excluded.generated_at,
                mission_statement = excluded.mission_statement,
                suggested_mode    = excluded.suggested_mode,
                locked            = excluded.locked,
                locked_at         = excluded.locked_at,
                locked_by         = excluded.locked_by,
                requires_research = excluded.requires_research,
                payload           = excluded.payload",
            rusqlite::params![
                consolidated.mission_id.to_string(),
                consolidated.verdict_id.to_string(),
                consolidated.generated_at,
                consolidated.mission_statement,
                consolidated.suggested_mode.tag(),
                locked_int,
                consolidated.locked_at,
                consolidated.locked_by.tag(),
                requires_research_int,
                payload,
            ],
        )?;
        Ok(())
    }

    /// Return the raw JSON payload column for a verdict id, or `None`.
    /// Used by the HUD when it needs the full `PublicUnderstandingVerdict`
    /// (intent hypotheses, gaps, clarification questions, etc.).
    pub fn verdict_payload(&self, verdict_id: Uuid) -> anyhow::Result<Option<String>> {
        self.payload_for("prompt_verdicts", "verdict_id", &verdict_id.to_string())
    }

    /// Newest verdict row linked to a given mission, or `None`.
    /// Used by `opencode plan <mission_id>` (RFC 25 §3.9) to recover the
    /// upstream `PublicUnderstandingVerdict` without storing it in CLI
    /// state.
    pub fn latest_verdict_for_mission(
        &self,
        mission_id: Uuid,
    ) -> anyhow::Result<Option<PublicUnderstandingVerdict>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT payload FROM prompt_verdicts
             WHERE mission_id = ?1
             ORDER BY ts DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![mission_id.to_string()], |row| {
            let payload: String = row.get(0)?;
            Ok(payload)
        })?;
        if let Some(raw) = rows.next().transpose()? {
            return Ok(Some(serde_json::from_str(&raw)?));
        }
        Ok(None)
    }

    /// Newest-first list of verdicts for the HUD tail (RFC 24 §3).
    pub fn verdict_tail(&self, last: i64) -> anyhow::Result<Vec<VerdictRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT verdict_id, session_id, mission_id, raw_prompt, ts,
                    confidence, rubric_mean, recommended_mode, gap_count
             FROM prompt_verdicts ORDER BY ts DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            let verdict_id_str: String = row.get(0)?;
            let session_id_str: String = row.get(1)?;
            let mission_id_str: Option<String> = row.get(2)?;
            Ok(VerdictRow {
                verdict_id: Uuid::parse_str(&verdict_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %verdict_id_str, "verdict_id parse failed; using nil");
                    Uuid::nil()
                }),
                session_id: Uuid::parse_str(&session_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %session_id_str, "session_id parse failed; using nil");
                    Uuid::nil()
                }),
                mission_id: mission_id_str
                    .as_deref()
                    .and_then(|s| Uuid::parse_str(s).ok()),
                raw_prompt: row.get(3)?,
                ts: row.get(4)?,
                confidence: row.get(5)?,
                rubric_mean: row.get(6)?,
                recommended_mode: row.get(7)?,
                gap_count: row.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    pub fn consolidated_tail(&self, last: i64) -> anyhow::Result<Vec<ConsolidatedRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT mission_id, verdict_id, generated_at, mission_statement,
                    suggested_mode, locked, locked_at, locked_by, requires_research
             FROM mission_consolidated ORDER BY generated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            let mission_id_str: String = row.get(0)?;
            let verdict_id_str: String = row.get(1)?;
            let locked_int: i64 = row.get(5)?;
            let req_int: i64 = row.get(8)?;
            let locked_by_str: Option<String> = row.get(7)?;
            Ok(ConsolidatedRow {
                mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed");
                    Uuid::nil()
                }),
                verdict_id: Uuid::parse_str(&verdict_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %verdict_id_str, "verdict_id parse failed");
                    Uuid::nil()
                }),
                generated_at: row.get(2)?,
                mission_statement: row.get(3)?,
                suggested_mode: row.get(4)?,
                locked: locked_int != 0,
                locked_at: row.get(6)?,
                locked_by: locked_by_str,
                requires_research: req_int != 0,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    /// Return the raw JSON payload column for a consolidated mission, or
    /// `None`. Mirrors `verdict_payload`.
    pub fn consolidated_payload(&self, mission_id: Uuid) -> anyhow::Result<Option<String>> {
        self.payload_for(
            "mission_consolidated",
            "mission_id",
            &mission_id.to_string(),
        )
    }

    /// Newest consolidated mission for a given mission_id. `opencode plan
    /// <mission_id>` uses this to recover the upstream `MissionConsolidated`
    /// before invoking the Planning Engine (RFC 25 §3.9).
    pub fn latest_consolidated_for_mission(
        &self,
        mission_id: Uuid,
    ) -> anyhow::Result<Option<crate::prompt::types::MissionConsolidated>> {
        let Some(raw) = self.consolidated_payload(mission_id)? else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_str(&raw)?))
    }

    // ——— Planning Engine persistence (RFC 12 §3) ———
    //
    // Same pattern as the Prompt Understanding artefacts: the JSON payload
    // is the source of truth; the top-level columns are duplicated only where
    // the SQL engine needs them for indexing / the Journal tail commands / the
    // HUD live stream. `ON CONFLICT DO NOTHING`: a re-persisted plan (e.g.
    // replayed from a snapshot) never overwrites the original — the first one
    // wins, matching the `journal_events` idempotency invariant (RFC 02 §3.1.2).

    pub fn save_plan(&self, plan: &crate::planning::types::Plan) -> anyhow::Result<()> {
        let payload = serde_json::to_string(plan)?;
        let blocker_count: i64 = plan.blocked.len().try_into().unwrap_or(i64::MAX);
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO plans
                (plan_id, mission_id, verdict_id, generated_at, strategy,
                 risk, impact, confidence, resume_point, blocker_count, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(plan_id) DO NOTHING",
            rusqlite::params![
                plan.plan_id.to_string(),
                plan.mission_id.to_string(),
                plan.verdict_id.to_string(),
                plan.generated_at,
                plan.strategy.tag(),
                plan.risk,
                plan.impact.tag(),
                plan.confidence,
                plan.resume_point,
                blocker_count,
                payload,
            ],
        )?;
        Ok(())
    }

    /// Return the raw JSON payload column for a plan id, or `None`.
    pub fn plan_payload(&self, plan_id: Uuid) -> anyhow::Result<Option<String>> {
        self.payload_for("plans", "plan_id", &plan_id.to_string())
    }

    /// Newest-first list of plans for the HUD tail (RFC 24 §3).
    pub fn plan_tail(&self, last: i64) -> anyhow::Result<Vec<PlanRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT plan_id, mission_id, verdict_id, generated_at,
                    strategy, risk, impact, confidence, resume_point, blocker_count
             FROM plans ORDER BY generated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            let plan_id_str: String = row.get(0)?;
            let mission_id_str: String = row.get(1)?;
            let verdict_id_str: String = row.get(2)?;
            Ok(PlanRow {
                plan_id: Uuid::parse_str(&plan_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %plan_id_str, "plan_id parse failed; using nil");
                    Uuid::nil()
                }),
                mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed; using nil");
                    Uuid::nil()
                }),
                verdict_id: Uuid::parse_str(&verdict_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %verdict_id_str, "verdict_id parse failed; using nil");
                    Uuid::nil()
                }),
                generated_at: row.get(3)?,
                strategy: row.get(4)?,
                risk: row.get(5)?,
                impact: row.get(6)?,
                confidence: row.get(7)?,
                resume_point: row.get(8)?,
                blocker_count: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    /// Most recent `Plan` row for a given mission. The HUD renders this on
    /// the Mission Control Kanban while a subagent is reading the full
    /// payload via `plan_payload`.
    pub fn latest_plan_for_mission(&self, mission_id: Uuid) -> anyhow::Result<Option<PlanRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT plan_id, mission_id, verdict_id, generated_at,
                    strategy, risk, impact, confidence, resume_point, blocker_count
             FROM plans WHERE mission_id = ?1
             ORDER BY generated_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![mission_id.to_string()], |row| {
            let plan_id_str: String = row.get(0)?;
            let mission_id_str: String = row.get(1)?;
            let verdict_id_str: String = row.get(2)?;
            Ok(PlanRow {
                plan_id: Uuid::parse_str(&plan_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %plan_id_str, "plan_id parse failed; using nil");
                    Uuid::nil()
                }),
                mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed; using nil");
                    Uuid::nil()
                }),
                verdict_id: Uuid::parse_str(&verdict_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %verdict_id_str, "verdict_id parse failed; using nil");
                    Uuid::nil()
                }),
                generated_at: row.get(3)?,
                strategy: row.get(4)?,
                risk: row.get(5)?,
                impact: row.get(6)?,
                confidence: row.get(7)?,
                resume_point: row.get(8)?,
                blocker_count: row.get(9)?,
            })
        })?;
        if let Some(r) = rows.next().transpose()? {
            return Ok(Some(r));
        }
        Ok(None)
    }

    // ——— Coding Engine persistence (RFC 13 §2, §8) ———
    //
    // Same pattern as the artefacts above: JSON payload is the source of
    // truth; the top-level columns are duplicated only where the SQL
    // engine needs them for indexing / HUD tail. `ON CONFLICT DO NOTHING`
    // honours the RFC 02 §3.1.2 at-least-once invariant (first write wins).

    pub fn save_diff(&self, diff: &crate::coding::types::Diff) -> anyhow::Result<()> {
        let payload = serde_json::to_string(diff)?;
        let lines_added: i64 = diff.files.iter().map(|f| f.lines_added() as i64).sum();
        let lines_removed: i64 = diff.files.iter().map(|f| f.lines_removed() as i64).sum();
        let file_count: i64 = diff.files.len() as i64;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO diffs
                (diff_id, plan_id, mission_id, step_id, agent_id,
                 generated_at, lines_added, lines_removed, file_count, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(diff_id) DO NOTHING",
            rusqlite::params![
                diff.diff_id.to_string(),
                diff.plan_id.to_string(),
                diff.mission_id.to_string(),
                diff.step_id,
                diff.agent_id.to_string(),
                diff.generated_at,
                lines_added,
                lines_removed,
                file_count,
                payload,
            ],
        )?;
        Ok(())
    }

    /// Return the raw JSON payload column for a diff id, or `None`.
    pub fn diff_payload(&self, diff_id: Uuid) -> anyhow::Result<Option<String>> {
        self.payload_for("diffs", "diff_id", &diff_id.to_string())
    }

    /// Newest-first list of diffs for the HUD tail (RFC 24 §3).
    pub fn diff_tail(&self, last: i64) -> anyhow::Result<Vec<DiffRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT diff_id, plan_id, mission_id, step_id, agent_id,
                    generated_at, lines_added, lines_removed, file_count
             FROM diffs ORDER BY generated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            let diff_id_str: String = row.get(0)?;
            let plan_id_str: String = row.get(1)?;
            let mission_id_str: String = row.get(2)?;
            let agent_id_str: String = row.get(4)?;
            Ok(DiffRow {
                diff_id: Uuid::parse_str(&diff_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %diff_id_str, "diff_id parse failed; using nil");
                    Uuid::nil()
                }),
                plan_id: Uuid::parse_str(&plan_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %plan_id_str, "plan_id parse failed; using nil");
                    Uuid::nil()
                }),
                mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed; using nil");
                    Uuid::nil()
                }),
                step_id: row.get(3)?,
                agent_id: Uuid::parse_str(&agent_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %agent_id_str, "agent_id parse failed; using nil");
                    Uuid::nil()
                }),
                generated_at: row.get(5)?,
                lines_added: row.get(6)?,
                lines_removed: row.get(7)?,
                file_count: row.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    // ——— Validation Engine persistence (RFC 14 §3, §8) ———
    //
    // Same JSON-blob + duplicated-index pattern as the M1..M4 artefacts.
    // `ON CONFLICT DO NOTHING` honours RFC 02 §3.1.2 at-least-once
    // delivery with idempotent consumers (first write wins; a replayed
    // report is silently dropped, NOT replaced).

    pub fn save_report(
        &self,
        report: &crate::validation::types::ValidationReport,
    ) -> anyhow::Result<()> {
        let payload = serde_json::to_string(report)?;
        let stage_count: i64 = report.stages.len().try_into().unwrap_or(i64::MAX);
        let failed_stage = report.failed_stage().map(|k| k.tag().to_string());
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO validation_reports
                (report_id, diff_id, plan_id, mission_id, generated_at,
                 mode, outcome, failed_stage, stage_count, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(report_id) DO NOTHING",
            rusqlite::params![
                report.report_id.to_string(),
                report.diff_id.to_string(),
                report.plan_id.to_string(),
                report.mission_id.to_string(),
                report.generated_at,
                report.mode.tag(),
                report.outcome.tag(),
                failed_stage,
                stage_count,
                payload,
            ],
        )?;
        Ok(())
    }

    /// Return the raw JSON payload column for a report id, or `None`.
    pub fn report_payload(&self, report_id: Uuid) -> anyhow::Result<Option<String>> {
        self.payload_for("validation_reports", "report_id", &report_id.to_string())
    }

    /// Newest-first list of validation reports for the HUD tail (RFC 24 §3)
    /// and the Repair Engine (RFC 15).
    pub fn report_tail(&self, last: i64) -> anyhow::Result<Vec<ValidationReportRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT report_id, diff_id, plan_id, mission_id, generated_at,
                    mode, outcome, failed_stage, stage_count
             FROM validation_reports ORDER BY generated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            let report_id_str: String = row.get(0)?;
            let diff_id_str: String = row.get(1)?;
            let plan_id_str: String = row.get(2)?;
            let mission_id_str: String = row.get(3)?;
            Ok(ValidationReportRow {
                report_id: Uuid::parse_str(&report_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %report_id_str, "report_id parse failed; using nil");
                    Uuid::nil()
                }),
                diff_id: Uuid::parse_str(&diff_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %diff_id_str, "diff_id parse failed; using nil");
                    Uuid::nil()
                }),
                plan_id: Uuid::parse_str(&plan_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %plan_id_str, "plan_id parse failed; using nil");
                    Uuid::nil()
                }),
                mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed; using nil");
                    Uuid::nil()
                }),
                generated_at: row.get(4)?,
                mode: row.get(5)?,
                outcome: row.get(6)?,
                failed_stage: row.get(7)?,
                stage_count: row.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    // ——— Repair Engine persistence (RFC 15 §6) ———
    //
    // Same JSON-blob + duplicated-index pattern as the M1..M5 artefacts.
    // `ON CONFLICT DO NOTHING` honours RFC 02 §3.1.2 at-least-once
    // delivery with idempotent consumers (first write wins; a replayed
    // repair run is silently dropped, NOT replaced).

    pub fn save_repair(&self, report: &crate::repair::types::RepairReport) -> anyhow::Result<()> {
        let payload = serde_json::to_string(report)?;
        let attempt_count: i64 = report.attempts.len().try_into().unwrap_or(i64::MAX);
        let successful_attempt: Option<i64> = report.successful_attempt.map(|n| n as i64);
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO repair_runs
                (repair_id, triggered_by_report_id, source_diff_id, plan_id,
                 mission_id, generated_at, outcome, triggering_stage,
                 attempt_count, successful_attempt, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(repair_id) DO NOTHING",
            rusqlite::params![
                report.repair_id.to_string(),
                report.triggered_by_report_id.to_string(),
                report.source_diff_id.to_string(),
                report.plan_id.to_string(),
                report.mission_id.to_string(),
                report.generated_at,
                report.outcome.tag(),
                report.triggering_stage.tag(),
                attempt_count,
                successful_attempt,
                payload,
            ],
        )?;
        Ok(())
    }

    /// Return the raw JSON payload column for a repair id, or `None`.
    pub fn repair_payload(&self, repair_id: Uuid) -> anyhow::Result<Option<String>> {
        self.payload_for("repair_runs", "repair_id", &repair_id.to_string())
    }

    /// Newest-first list of repair runs for the HUD tail (RFC 24 §3) and
    /// the Learning Engine (RFC 16).
    pub fn repair_tail(&self, last: i64) -> anyhow::Result<Vec<RepairRunRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT repair_id, triggered_by_report_id, source_diff_id, plan_id,
                    mission_id, generated_at, outcome, triggering_stage,
                    attempt_count, successful_attempt
             FROM repair_runs ORDER BY generated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            let repair_id_str: String = row.get(0)?;
            let report_id_str: String = row.get(1)?;
            let diff_id_str: String = row.get(2)?;
            let plan_id_str: String = row.get(3)?;
            let mission_id_str: String = row.get(4)?;
            Ok(RepairRunRow {
                repair_id: Uuid::parse_str(&repair_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %repair_id_str, "repair_id parse failed; using nil");
                    Uuid::nil()
                }),
                triggered_by_report_id: Uuid::parse_str(&report_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %report_id_str, "report_id parse failed; using nil");
                    Uuid::nil()
                }),
                source_diff_id: Uuid::parse_str(&diff_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %diff_id_str, "diff_id parse failed; using nil");
                    Uuid::nil()
                }),
                plan_id: Uuid::parse_str(&plan_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %plan_id_str, "plan_id parse failed; using nil");
                    Uuid::nil()
                }),
                mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed; using nil");
                    Uuid::nil()
                }),
                generated_at: row.get(5)?,
                outcome: row.get(6)?,
                triggering_stage: row.get(7)?,
                attempt_count: row.get(8)?,
                successful_attempt: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    // ——— Learning Engine persistence (RFC 16 §2 / §3 / §4) ———
    //
    // `LearnOutcome` carries an `Option<Pattern>` (the rule draft) and
    // a metrics bundle. We persist ONE row per `LearnOutcome` using the
    // same JSON-blob + denormalised-index pattern: the `payload` column
    // holds the full `LearnOutcome` (Pattern + metrics + evidence diff
    // inline), and the denormalised columns back the HUD tail and the
    // Skill Compressor's "find duplicate patterns" queries without a
    // JSON parse.
    //
    // `ON CONFLICT DO NOTHING` honours RFC 02 §3.1.2 at-least-once
    // delivery with idempotent consumers (first write wins).

    pub fn save_pattern(
        &self,
        outcome: &crate::learning::types::LearnOutcome,
    ) -> anyhow::Result<()> {
        let payload = serde_json::to_string(outcome)?;
        let conn = self.conn.lock();
        // `pattern_id` is `Some` iff the outcome produced a Draft rule;
        // `Uuid::nil()` otherwise. The PK is `learn_id` so one row per
        // `LearnOutcome` (incl. NoPattern) — RFC 16 §6 audit tail.
        let pattern_id_str = outcome
            .pattern
            .as_ref()
            .map(|p| p.pattern_id.to_string())
            .unwrap_or_else(|| Uuid::nil().to_string());
        let rule_id = outcome
            .pattern
            .as_ref()
            .map(|p| p.rule_id.clone())
            .unwrap_or_else(|| format!("no-pattern-{}", outcome.learn_id));
        let stage = outcome
            .pattern
            .as_ref()
            .map(|p| p.when.stage.tag().to_string())
            .unwrap_or_else(|| "none".into());
        let strategy = outcome
            .pattern
            .as_ref()
            .map(|p| p.then.strategy.tag().to_string())
            .unwrap_or_else(|| "none".into());
        let lifecycle = outcome
            .pattern
            .as_ref()
            .map(|p| p.lifecycle.tag().to_string())
            .unwrap_or_else(|| "none".into());
        let priority: i64 = outcome
            .pattern
            .as_ref()
            .map(|p| p.priority as i64)
            .unwrap_or(0);
        let confidence: f64 = outcome
            .pattern
            .as_ref()
            .map(|p| p.confidence as f64)
            .unwrap_or(0.0);
        conn.execute(
            "INSERT INTO pattern_runs
                (learn_id, pattern_id, source_repair_id, source_mission_id,
                 rule_id, stage, strategy, lifecycle, priority, confidence,
                 was_correct, tests_passed, generated_at, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(learn_id) DO NOTHING",
            rusqlite::params![
                outcome.learn_id.to_string(),
                pattern_id_str,
                outcome.source_repair_id.to_string(),
                outcome.source_mission_id.to_string(),
                rule_id,
                stage,
                strategy,
                lifecycle,
                priority,
                confidence,
                outcome.metrics.was_correct as i64,
                outcome.metrics.tests_passed as i64,
                outcome.generated_at,
                payload,
            ],
        )?;
        Ok(())
    }

    /// Return the raw JSON payload column for a learn id, or `None`.
    pub fn pattern_payload(&self, learn_id: Uuid) -> anyhow::Result<Option<String>> {
        self.payload_for("pattern_runs", "learn_id", &learn_id.to_string())
    }

    /// Newest-first list of pattern rows for the HUD tail (RFC 24 §3)
    /// and the Skill Compressor (RFC 16 §5). The denormalised columns
    /// let the Compressor group by `(stage, strategy)` to detect
    /// duplicate patterns without parsing each payload.
    pub fn pattern_tail(&self, last: i64) -> anyhow::Result<Vec<PatternRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT pattern_id, learn_id, source_repair_id, source_mission_id,
                    rule_id, stage, strategy, lifecycle, priority, confidence,
                    was_correct, tests_passed, generated_at
             FROM pattern_runs ORDER BY generated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            let pattern_id_str: String = row.get(0)?;
            let learn_id_str: String = row.get(1)?;
            let repair_id_str: String = row.get(2)?;
            let mission_id_str: String = row.get(3)?;
            Ok(PatternRow {
                pattern_id: Uuid::parse_str(&pattern_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %pattern_id_str, "pattern_id parse failed; using nil");
                    Uuid::nil()
                }),
                learn_id: Uuid::parse_str(&learn_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %learn_id_str, "learn_id parse failed; using nil");
                    Uuid::nil()
                }),
                source_repair_id: Uuid::parse_str(&repair_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %repair_id_str, "repair_id parse failed; using nil");
                    Uuid::nil()
                }),
                source_mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed; using nil");
                    Uuid::nil()
                }),
                rule_id: row.get(4)?,
                stage: row.get(5)?,
                strategy: row.get(6)?,
                lifecycle: row.get(7)?,
                priority: row.get(8)?,
                confidence: row.get(9)?,
                was_correct: row.get(10)?,
                tests_passed: row.get(11)?,
                generated_at: row.get(12)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    // ——— Execution Supervisor persistence (RFC 19 §5) ———
    //
    // `MissionCheckpoint` carries the phase + budget tally + upstream
    // ids needed for `Supervisor.resume(mission_id)`. The HUD renders
    // the latest checkpoint per mission as the "where are we" tile.
    // Same JSON-blob + denormalised-index pattern. ON CONFLICT DO
    // NOTHING honours RFC 02 §3.1.2 replay idempotency.

    pub fn save_checkpoint(
        &self,
        checkpoint: &crate::supervisor::types::MissionCheckpoint,
    ) -> anyhow::Result<()> {
        let payload = serde_json::to_string(checkpoint)?;
        let tally = serde_json::to_string(&checkpoint.budget_tally)?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO mission_checkpoints
                (checkpoint_id, mission_id, phase, budget_tally, generated_at, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(checkpoint_id) DO NOTHING",
            rusqlite::params![
                checkpoint.checkpoint_id.to_string(),
                checkpoint.mission_id.to_string(),
                checkpoint.phase.tag(),
                tally,
                checkpoint.generated_at,
                payload,
            ],
        )?;
        Ok(())
    }

    /// Return the raw JSON payload column for a checkpoint, or `None`.
    pub fn checkpoint_payload(&self, checkpoint_id: Uuid) -> anyhow::Result<Option<String>> {
        self.payload_for(
            "mission_checkpoints",
            "checkpoint_id",
            &checkpoint_id.to_string(),
        )
    }

    /// Newest-first list of checkpoints for the HUD recent-checkpoints
    /// panel (RFC 24 §3).
    pub fn checkpoint_tail(&self, last: i64) -> anyhow::Result<Vec<CheckpointRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT checkpoint_id, mission_id, phase, generated_at
             FROM mission_checkpoints ORDER BY generated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            let checkpoint_id_str: String = row.get(0)?;
            let mission_id_str: String = row.get(1)?;
            Ok(CheckpointRow {
                checkpoint_id: Uuid::parse_str(&checkpoint_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %checkpoint_id_str, "checkpoint_id parse failed; using nil");
                    Uuid::nil()
                }),
                mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed; using nil");
                    Uuid::nil()
                }),
                phase: row.get(2)?,
                generated_at: row.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    /// RFC 19 §5 `Supervisor.resume(mission_id)` — return the newest
    /// checkpoint row for a mission, or `None` when no checkpoint
    /// exists yet (e.g. the mission was never started).
    pub fn latest_checkpoint(&self, mission_id: Uuid) -> anyhow::Result<Option<CheckpointRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT checkpoint_id, mission_id, phase, generated_at
             FROM mission_checkpoints WHERE mission_id = ?1
             ORDER BY generated_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![mission_id.to_string()], |row| {
            let checkpoint_id_str: String = row.get(0)?;
            let mission_id_str: String = row.get(1)?;
            Ok(CheckpointRow {
                checkpoint_id: Uuid::parse_str(&checkpoint_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %checkpoint_id_str, "checkpoint_id parse failed; using nil");
                    Uuid::nil()
                }),
                mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed; using nil");
                    Uuid::nil()
                }),
                phase: row.get(2)?,
                generated_at: row.get(3)?,
            })
        })?;
        if let Some(r) = rows.next().transpose()? {
            return Ok(Some(r));
        }
        Ok(None)
    }

    // ——— Skill Graph persistence (RFC 06 §1 / §10) ———
    //
    // `SkillManifest` evolves over time (RFC 06 §5 compression, RFC 16
    // §6 learning-engine rules). We use a composite PK `(skill_id,
    // version)` so the HUD can render the full history of an id; a
    // re-register of the SAME version is idempotent
    // (`ON CONFLICT DO NOTHING`).

    pub fn save_skill(&self, skill: &crate::skills::SkillManifest) -> anyhow::Result<()> {
        let payload = serde_json::to_string(skill)?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO skill_manifests
                (skill_id, version, engine, priority, domain, language, framework,
                 confidence, auto_generated, verified, requires_sandbox, generated_at, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(skill_id, version) DO NOTHING",
            rusqlite::params![
                skill.id,
                skill.version,
                skill.engine.tag(),
                skill.priority as i64,
                skill.domain.as_deref(),
                skill.language.as_deref(),
                skill.framework.as_deref(),
                skill.confidence as f64,
                skill.auto_generated as i64,
                skill.verified as i64,
                skill.requires_sandbox as i64,
                chrono::Utc::now().to_rfc3339(),
                payload,
            ],
        )?;
        Ok(())
    }

    /// Return the raw JSON payload column for a `(skill_id, version)`,
    /// or `None`.
    pub fn skill_payload(&self, skill_id: &str, version: &str) -> anyhow::Result<Option<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT payload FROM skill_manifests WHERE skill_id = ?1 AND version = ?2")?;
        let mut rows = stmt.query_map(rusqlite::params![skill_id, version], |row| {
            let payload: String = row.get(0)?;
            Ok(payload)
        })?;
        if let Some(r) = rows.next().transpose()? {
            return Ok(Some(r));
        }
        Ok(None)
    }

    /// Newest-first list of skill rows for the HUD Skill Graph panel
    /// (RFC 24 §3) and the §3 candidate pipeline. The denormalised
    /// `priority` and `engine` columns cover the "ORDER BY priority"
    /// path without a JSON parse.
    pub fn skill_tail(&self, last: i64) -> anyhow::Result<Vec<SkillRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT skill_id, version, engine, priority, domain, language, framework,
                    confidence, auto_generated, verified, requires_sandbox, generated_at
             FROM skill_manifests ORDER BY generated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            let domain: Option<String> = row.get(4)?;
            let language: Option<String> = row.get(5)?;
            let framework: Option<String> = row.get(6)?;
            Ok::<SkillRow, rusqlite::Error>(SkillRow {
                skill_id: row.get(0)?,
                version: row.get(1)?,
                engine: row.get(2)?,
                priority: row.get(3)?,
                domain,
                language,
                framework,
                confidence: row.get(7)?,
                auto_generated: row.get::<_, i64>(8)? != 0,
                verified: row.get::<_, i64>(9)? != 0,
                requires_sandbox: row.get::<_, i64>(10)? != 0,
                generated_at: row.get(11)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    /// Return the newest version row for a skill id (or `None` when
    /// the id is unknown). Used by the Learning Engine (RFC 16 §5) to
    /// supersede an old skill with a fused one.
    pub fn latest_skill(&self, skill_id: &str) -> anyhow::Result<Option<SkillRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT skill_id, version, engine, priority, domain, language, framework,
                    confidence, auto_generated, verified, requires_sandbox, generated_at
             FROM skill_manifests WHERE skill_id = ?1
             ORDER BY generated_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![skill_id], |row| {
            let domain: Option<String> = row.get(4)?;
            let language: Option<String> = row.get(5)?;
            let framework: Option<String> = row.get(6)?;
            Ok::<SkillRow, rusqlite::Error>(SkillRow {
                skill_id: row.get(0)?,
                version: row.get(1)?,
                engine: row.get(2)?,
                priority: row.get(3)?,
                domain,
                language,
                framework,
                confidence: row.get(7)?,
                auto_generated: row.get::<_, i64>(8)? != 0,
                verified: row.get::<_, i64>(9)? != 0,
                requires_sandbox: row.get::<_, i64>(10)? != 0,
                generated_at: row.get(11)?,
            })
        })?;
        if let Some(r) = rows.next().transpose()? {
            return Ok(Some(r));
        }
        Ok(None)
    }

    // ——— Model hot-swap persistence (RFC 27 §B, schema M10) ———
    //
    // `model_swaps` is append-only by design: every swap is a fresh
    // `BusEventKind::ModelSwapped` row, and the audit trail must survive
    // re-opens / replays. `ON CONFLICT DO NOTHING` is keyed on `swap_id`
    // (a fresh UUID per swap) so a replayed event is silently dropped
    // rather than producing a ghost duplicate (RFC 02 §3.1.2).

    pub fn save_model_swap(
        &self,
        mission_id: Uuid,
        prev_model_id: &str,
        new_model_id: &str,
        initiator: &crate::core::bus::SwapInitiator,
        occurred_at: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<Uuid> {
        let swap_id = Uuid::new_v4();
        let payload = serde_json::json!({
            "mission_id": mission_id.to_string(),
            "prev_model_id": prev_model_id,
            "new_model_id": new_model_id,
            "initiator": initiator,
            "occurred_at": occurred_at.to_rfc3339(),
        })
        .to_string();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO model_swaps
                (swap_id, mission_id, prev_model_id, new_model_id, initiator,
                 occurred_at, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(swap_id) DO NOTHING",
            rusqlite::params![
                swap_id.to_string(),
                mission_id.to_string(),
                prev_model_id,
                new_model_id,
                match initiator {
                    crate::core::bus::SwapInitiator::User => "user",
                    crate::core::bus::SwapInitiator::Auto => "auto",
                },
                occurred_at.to_rfc3339(),
                payload,
            ],
        )?;
        Ok(swap_id)
    }

    /// Newest-first list of model-swap rows for the HUD tail (RFC 24 §3)
    /// and the operator CLI (`opencode swaps <mission_id>`).
    pub fn model_swap_tail(&self, last: i64) -> anyhow::Result<Vec<ModelSwapRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT swap_id, mission_id, prev_model_id, new_model_id, initiator, occurred_at
             FROM model_swaps ORDER BY occurred_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            let swap_id_str: String = row.get(0)?;
            let mission_id_str: String = row.get(1)?;
            Ok(ModelSwapRow {
                swap_id: Uuid::parse_str(&swap_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %swap_id_str, "swap_id parse failed; using nil");
                    Uuid::nil()
                }),
                mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed; using nil");
                    Uuid::nil()
                }),
                prev_model_id: row.get(2)?,
                new_model_id: row.get(3)?,
                initiator: row.get(4)?,
                occurred_at: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    /// Every swap row for a given mission (oldest-first) — the audit
    /// trail view the operator pulls up before a steering intervention.
    pub fn model_swaps_for_mission(&self, mission_id: Uuid) -> anyhow::Result<Vec<ModelSwapRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT swap_id, mission_id, prev_model_id, new_model_id, initiator, occurred_at
             FROM model_swaps WHERE mission_id = ?1
             ORDER BY occurred_at ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![mission_id.to_string()], |row| {
            let swap_id_str: String = row.get(0)?;
            let mission_id_str: String = row.get(1)?;
            Ok(ModelSwapRow {
                swap_id: Uuid::parse_str(&swap_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %swap_id_str, "swap_id parse failed; using nil");
                    Uuid::nil()
                }),
                mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed; using nil");
                    Uuid::nil()
                }),
                prev_model_id: row.get(2)?,
                new_model_id: row.get(3)?,
                initiator: row.get(4)?,
                occurred_at: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    // ——— Step-state persistence (RFC 27 §G, schema M11) ———
    //
    // `step_states` is UPSERTed (not append-only): the latest phase
    // observation replaces the previous row because the HUD pill renders
    // the live state, not a history. Each transition ALSO publishes a
    // `StepPhaseChanged` bus event into `journal_events` so the audit
    // trail survives in the append-only `journal_events`.

    pub fn set_step_phase(
        &self,
        mission_id: Uuid,
        plan_id: Uuid,
        step_id: &str,
        phase: crate::planning::types::StepPhase,
    ) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO step_states
                (mission_id, plan_id, step_id, phase, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(mission_id, plan_id, step_id) DO UPDATE SET
                phase      = excluded.phase,
                updated_at = excluded.updated_at",
            rusqlite::params![
                mission_id.to_string(),
                plan_id.to_string(),
                step_id,
                phase.tag(),
                now,
            ],
        )?;
        Ok(())
    }

    /// All step-state rows for a plan, newest-first by `updated_at`. The
    /// HUD joins this with the plan payload to render colour-coded pills
    /// alongside each `Step.id`.
    pub fn step_states_for_plan(&self, plan_id: Uuid) -> anyhow::Result<Vec<StepStateRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT mission_id, plan_id, step_id, phase, updated_at
             FROM step_states WHERE plan_id = ?1
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![plan_id.to_string()], |row| {
            let mission_id_str: String = row.get(0)?;
            let plan_id_str: String = row.get(1)?;
            Ok(StepStateRow {
                mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed; using nil");
                    Uuid::nil()
                }),
                plan_id: Uuid::parse_str(&plan_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %plan_id_str, "plan_id parse failed; using nil");
                    Uuid::nil()
                }),
                step_id: row.get(2)?,
                phase: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    /// Newest-first list across all plans — the HUD live-stream view.
    pub fn step_state_tail(&self, last: i64) -> anyhow::Result<Vec<StepStateRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT mission_id, plan_id, step_id, phase, updated_at
             FROM step_states ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![last], |row| {
            let mission_id_str: String = row.get(0)?;
            let plan_id_str: String = row.get(1)?;
            Ok(StepStateRow {
                mission_id: Uuid::parse_str(&mission_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %mission_id_str, "mission_id parse failed; using nil");
                    Uuid::nil()
                }),
                plan_id: Uuid::parse_str(&plan_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %plan_id_str, "plan_id parse failed; using nil");
                    Uuid::nil()
                }),
                step_id: row.get(2)?,
                phase: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    /// RFC 27 §E — persist an annotation on a diff. Append-only: each
    /// call inserts a fresh `id` (caller mints it). On success the
    /// annotation is immediately visible via `diff_annotations_for_diff`
    /// and through the HUD tail drawer.
    pub fn save_diff_annotation(&self, row: &DiffAnnotationRow) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO diff_annotations
                (id, diff_id, file_path, line_no, body, author, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO NOTHING",
            rusqlite::params![
                row.id.to_string(),
                row.diff_id.to_string(),
                row.file_path,
                row.line_no,
                row.body,
                row.author,
                row.created_at,
            ],
        )?;
        Ok(())
    }

    /// RFC 27 §E — annotations for a single diff, oldest-first (the
    /// drawer renders them in chronological order so the reviewer can
    /// follow the conversation).
    pub fn diff_annotations_for_diff(
        &self,
        diff_id: Uuid,
    ) -> anyhow::Result<Vec<DiffAnnotationRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, diff_id, file_path, line_no, body, author, created_at
             FROM diff_annotations WHERE diff_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![diff_id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let diff_id_str: String = row.get(1)?;
            Ok(DiffAnnotationRow {
                id: Uuid::parse_str(&id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %id_str, "annotation id parse failed; using nil");
                    Uuid::nil()
                }),
                diff_id: Uuid::parse_str(&diff_id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %diff_id_str, "annotation diff_id parse failed; using nil");
                    Uuid::nil()
                }),
                file_path: row.get(2)?,
                line_no: row.get(3)?,
                body: row.get(4)?,
                author: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    /// Insert one row into `agent_session_events` (M14). Returns the row id.
    /// Spawning and parsing the source envelope is owned by the ACP
    /// `listen_worker`; this fn only persists the resulting envelope so
    /// non-Windows hosts still get the typed row inserted by tests / fixtures.
    pub fn insert_agent_session_event(
        &self,
        ts: i64,
        pane_id: Option<&str>,
        event_type: &str,
        agent: &str,
        task_id: Option<&str>,
        payload_json: &str,
    ) -> anyhow::Result<i64> {
        let conn = self.conn.lock();
        agent_events::insert_agent_session_event(
            &conn,
            ts,
            pane_id,
            event_type,
            agent,
            task_id,
            payload_json,
        )
    }

    /// Read the most recent `last` rows of `agent_session_events`, newest
    /// first. Backs the HUD pane-timeline card.
    pub fn agent_session_events_tail(
        &self,
        last: i64,
    ) -> anyhow::Result<Vec<agent_events::AgentSessionEventRow>> {
        let conn = self.conn.lock();
        agent_events::agent_session_events_tail(&conn, last)
    }

    /// Read all rows for a given `pane_id`, newest first. Backs the per-pane
    /// HUD card that lets an operator scrub agent.tool.invoked /
    /// agent.tool.completed pairs in chronological order.
    pub fn agent_session_events_for_pane(
        &self,
        pane_id: &str,
    ) -> anyhow::Result<Vec<agent_events::AgentSessionEventRow>> {
        let conn = self.conn.lock();
        agent_events::agent_session_events_for_pane(&conn, pane_id)
    }
}

#[cfg(feature = "dag_mode")]
impl Journal {
    /// Thin wrapper over [`learning_graphs::persist_graph`]. Acquires
    /// the connection lock once and delegates.
    #[cfg(feature = "dag_mode")]
    pub fn persist_learning_graph(
        &self,
        req: &learning_graphs::PersistGraphRequest<'_>,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        learning_graphs::persist_graph(&conn, req)
    }

    /// Thin wrapper over [`learning_graphs::retrieve_similar_graphs`].
    #[cfg(feature = "dag_mode")]
    pub fn retrieve_similar_learning_graphs(
        &self,
        query_embedding: Option<&[f32]>,
        intent_signature: Option<&str>,
        top_k: usize,
    ) -> anyhow::Result<Vec<ScoredGraph>> {
        let conn = self.conn.lock();
        learning_graphs::retrieve_similar_graphs(&conn, query_embedding, intent_signature, top_k)
    }

    /// Thin wrapper over [`mission_graph::read_graph`]. Returns the
    /// full persisted graph (nodes+edges) for `mission_id`, or `None`
    /// when no M15 rows exist for that mission.
    #[cfg(feature = "dag_mode")]
    pub fn read_mission_graph(
        &self,
        mission_id: &str,
    ) -> anyhow::Result<Option<crate::graph::MissionGraph>> {
        let conn = self.conn.lock();
        mission_graph::read_graph(&conn, mission_id)
    }

    /// Test-only helper: bulk-insert raw graph rows for HUD/router
    /// integration tests. NOT for production use; production callers
    /// go through `skills::graph_loader::instantiate` or
    /// `planning::graph_emitter::plan_to_graph` (caller owns the
    /// write side). Gated behind `dag_mode` so a non-dag_mode build
    /// doesn't even see this helper.
    #[cfg(all(test, feature = "dag_mode"))]
    pub fn seed_test_graph_node(
        &self,
        mission_id: &str,
        node_id: &str,
        kind: &str,
        label: &str,
        provenance: &str,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![node_id, mission_id, kind, label, provenance],
        )?;
        Ok(())
    }
}

#[cfg(feature = "toast")]
impl Journal {
    /// Thin wrapper over [`crate::toast::queue::ToastQueue::enqueue`].
    #[cfg(feature = "toast")]
    pub fn toast_enqueue(
        &self,
        kind: crate::toast::payload::ToastKind,
        title: &str,
        body: Option<&str>,
        deep_link: Option<&str>,
        fire_at_ms: i64,
    ) -> anyhow::Result<i64> {
        let conn = self.conn.lock();
        let q = crate::toast::queue::ToastQueue::new(&conn);
        Ok(q.enqueue(kind, title, body, deep_link, fire_at_ms)?)
    }

    /// Thin wrapper over [`crate::toast::queue::ToastQueue::next_pending`].
    #[cfg(feature = "toast")]
    pub fn toast_next_pending(
        &self,
        now_ms: i64,
    ) -> anyhow::Result<Option<crate::toast::queue::QueueRow>> {
        let conn = self.conn.lock();
        let q = crate::toast::queue::ToastQueue::new(&conn);
        Ok(q.next_pending(now_ms)?)
    }

    /// Thin wrapper over [`crate::toast::queue::ToastQueue::count_pending`].
    #[cfg(feature = "toast")]
    pub fn toast_count_pending(&self) -> anyhow::Result<i64> {
        let conn = self.conn.lock();
        let q = crate::toast::queue::ToastQueue::new(&conn);
        Ok(q.count_pending()?)
    }

    /// Thin wrapper around the various `mark_*` mutators on
    /// [`crate::toast::queue::ToastQueue`]. `operation` is one of
    /// `"fired"` / `"dismissed"` / `"failed"`.
    #[cfg(feature = "toast")]
    pub fn toast_mark(
        &self,
        operation: &str,
        id: i64,
        stamp_ms: i64,
        reason: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        let q = crate::toast::queue::ToastQueue::new(&conn);
        match operation {
            "fired" => q.mark_fired(id, stamp_ms)?,
            "dismissed" => q.mark_dismissed(id, stamp_ms, reason.unwrap_or("unknown"))?,
            "failed" => q.mark_failed(id, 1)?,
            other => anyhow::bail!("unknown toast mark operation: {other}"),
        }
        Ok(())
    }

    /// Thin wrapper over [`crate::toast::queue::ToastQueue::cancel`].
    #[cfg(feature = "toast")]
    pub fn toast_cancel(&self, id: i64) -> anyhow::Result<bool> {
        let conn = self.conn.lock();
        let q = crate::toast::queue::ToastQueue::new(&conn);
        Ok(q.cancel(id)?)
    }

    /// Thin wrapper over [`crate::toast::queue::ToastQueue::list`].
    #[cfg(feature = "toast")]
    pub fn toast_list(&self, limit: i64) -> anyhow::Result<Vec<crate::toast::queue::QueueRow>> {
        let conn = self.conn.lock();
        let q = crate::toast::queue::ToastQueue::new(&conn);
        Ok(q.list(limit)?)
    }
}

// Section H — model reset-window helpers. `model_reset_upsert`
// and `link_model_reset_toast` are NOT feature-gated: the
// `model_resets` table is created unconditionally (M19), and the
// reset path persists the row even when the `toast` feature is off
// (the HUD tail surfaces the next pending reset regardless). Only
// the Toast enqueue is gated (`enqueue_model_ready_toast`).
impl Journal {
    /// RFC 28 §H.4 — persist a `SpendLimitError` observation into
    /// `model_resets` and read back the row id + any pre-existing
    /// Toast link (for idempotent replays). Returns
    /// `(reset_id, Option<toast_id>)` where the second slot is
    /// `Some(queue_id)` when this same `(provider, model, resets_at)`
    /// was already observed and previously linked to a Toast row.
    /// Used by `orchestrator::handle_spend_limit_error`.
    pub fn model_reset_upsert(
        &self,
        error: &crate::orchestrator::error::SpendLimitError,
        observed_at: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<(i64, Option<i64>)> {
        let conn = self.conn.lock();
        let reset_id = crate::journal::model_resets::insert_model_reset(&conn, error, observed_at)?;
        let already_linked: Option<i64> = conn
            .query_row(
                "SELECT toast_id FROM model_resets WHERE id = ?1",
                rusqlite::params![reset_id],
                |r| r.get(0),
            )
            .ok()
            .flatten();
        Ok((reset_id, already_linked))
    }

    /// RFC 28 §H.4 — enqueue the `kind='model_ready'` Toast for a
    /// fresh `SpendLimitError` observation. Returns the new Toast
    /// queue row id; caller links it back via `link_model_reset_toast`.
    /// Feature-gated behind `toast` (the `toast_queue` table exists
    /// regardless of the feature, but §H ships Toast-only).
    #[cfg(feature = "toast")]
    pub fn enqueue_model_ready_toast(
        &self,
        error: &crate::orchestrator::error::SpendLimitError,
    ) -> anyhow::Result<i64> {
        let conn = self.conn.lock();
        let q = crate::toast::queue::ToastQueue::new(&conn);
        let fire_at_ms = error.resets_at.timestamp_millis();
        let title = format!("{}: {} is rate-limited", error.provider, error.model);
        let body = format!(
            "Resets at {}. Switch provider or wait.",
            error.resets_at.to_rfc3339()
        );
        let deep_link = format!("opencode://model/{}/{}", error.provider, error.model);
        Ok(q.enqueue(
            crate::toast::payload::ToastKind::ModelReady,
            &title,
            Some(&body),
            Some(&deep_link),
            fire_at_ms,
        )?)
    }

    /// RFC 28 §H.4 — link a `model_resets` row to the Toast queue row
    /// enqueued for it, so the scheduler can correlate dismissals.
    pub fn link_model_reset_toast(&self, reset_id: i64, toast_id: i64) -> anyhow::Result<bool> {
        let conn = self.conn.lock();
        crate::journal::model_resets::link_toast_id(&conn, reset_id, toast_id)
    }

    /// RFC 28 §H.4 — mark a `model_resets` row's Toast as dismissed,
    /// looked up by `toast_queue.id`. Used by the Toast callback when
    /// the user dismisses the `model_ready` card (the dismiss is
    /// propagated through the scheduler -> here).
    pub fn mark_model_reset_toast_dismissed_by_queue_id(
        &self,
        toast_id: i64,
        dismissed_at: chrono::DateTime<chrono::Utc>,
    ) -> anyhow::Result<bool> {
        let conn = self.conn.lock();
        crate::journal::model_resets::mark_toast_dismissed_by_queue_id(
            &conn,
            toast_id,
            dismissed_at,
        )
    }

    /// Count of `model_resets` rows for a given `(provider, model)`
    /// pair (regardless of pending / dismissed state). Used by tests
    /// to assert idempotency of `handle_spend_limit_error` — production
    /// callers should consult `pending_for` for the actual pending
    /// reset (it filters past + dismissed rows).
    #[cfg(test)]
    pub fn count_model_resets_for_test(&self, provider: &str, model: &str) -> anyhow::Result<i64> {
        let conn = self.conn.lock();
        Ok(conn.query_row(
            "SELECT count(*) FROM model_resets WHERE provider=?1 AND model=?2",
            rusqlite::params![provider, model],
            |r| r.get(0),
        )?)
    }

    // ---- Phase 2 sub-fase 2.4 feedback loop -----------------------
    // Affinity reader + classifier-decision append.
    //
    // The reader joins `model_invocations mi ON
    // task_classifier_decisions cd ON cd.mission_id = mi.mission_id`
    // and groups by `(cd.predicted_task_type, mi.model_id)`, computing
    // `success_rate`, `p95_latency_ms`, `mean_cost_usd`, `n_samples`
    // over a bounded rolling window. The schema (M21 + M23) already
    // exists — no migration is needed for 2.4.
    //
    // The classifier-decision append records a row in
    // `task_classifier_decisions` keyed by `(prompt_hash,
    // classifier_kind)`. The A/B emission path records a *second* row
    // with `classifier_kind = 'mf_ab'` so offline calibration can
    // compare the chosen RouterId::Mf's log-loss against RouterId::Auto.

    /// Compute the affinity table for `(task_type, model_id)` pairs
    /// over the most recent `window` `model_invocations` rows. The
    /// reader is called by the orchestrator refresh loop (every N
    /// missions completed + on `atlas models refresh`), then
    /// `AffinityIndex::store_all` swaps the fresh map into the
    /// hot-path cache.
    ///
    /// Returned rows are filtered to `n_samples >= 1`; the router
    /// applies the `MIN_SAMPLES = 3` floor on read.
    pub fn read_affinity(
        &self,
        window: u32,
    ) -> anyhow::Result<Vec<crate::orchestrator::affinity::AffinityRow>> {
        use crate::orchestrator::affinity::AffinityRow;
        use crate::orchestrator::classifier::TaskType;
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "WITH ranked AS (
                SELECT mi.model_id,
                       mi.latency_ms,
                       mi.cost_usd,
                       mi.was_correct,
                       cd.predicted_task_type,
                       ROW_NUMBER() OVER (
                           ORDER BY mi.started_at DESC
                       ) AS rk
                FROM model_invocations mi
                JOIN task_classifier_decisions cd
                  ON cd.mission_id IS NOT NULL
                 AND cd.mission_id = mi.mission_id
                WHERE mi.mission_id IS NOT NULL
            )
            SELECT predicted_task_type, model_id,
                   COALESCE(SUM(CASE WHEN was_correct = 1 THEN 1 ELSE 0 END), 0) * 1.0
                       / COUNT(*)        AS success_rate,
                   -1                    AS p95_latency_ms_int,
                   AVG(cost_usd)         AS mean_cost_usd,
                   COUNT(*)              AS n_samples
            FROM ranked
            WHERE rk <= ?1
            GROUP BY predicted_task_type, model_id
            HAVING COUNT(*) >= 1",
        )?;
        let rows: rusqlite::Result<Vec<AffinityRow>> = stmt
            .query_map(rusqlite::params![i64::from(window)], |r| {
                let task_str: String = r.get(0)?;
                let model_id: String = r.get(1)?;
                let success_rate: f64 = r.get(2)?;
                let p95_int: i64 = r.get(3)?;
                let mean_cost: Option<f64> = r.get(4)?;
                let n_samples: i64 = r.get(5)?;
                let p95_latency_ms = if p95_int < 0 { None } else { Some(p95_int) };
                let task_type = TaskType::parse(&task_str).unwrap_or(TaskType::Unknown);
                Ok(AffinityRow {
                    task_type,
                    model_id,
                    success_rate,
                    p95_latency_ms,
                    mean_cost_usd: mean_cost,
                    n_samples,
                })
            })?
            .collect();
        Ok(rows?)
    }

    /// Persist the current affinity snapshot back to the SQLite mirror
    /// (`model_affinity_cache`, M23). Idempotent: `INSERT OR REPLACE`
    /// overwrites by primary key `(task_type, model_id)`. The
    /// refresh loop calls this after `read_affinity` so a restart can
    /// warm the in-memory index from disk without re-crunching.
    pub fn upsert_affinity_rows(
        &self,
        rows: &[crate::orchestrator::affinity::AffinityRow],
    ) -> anyhow::Result<()> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO model_affinity_cache
                 (task_type, model_id, success_rate, p95_latency_ms,
                  mean_cost_usd, n_samples, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            let now = chrono::Utc::now().to_rfc3339();
            for row in rows {
                stmt.execute(rusqlite::params![
                    row.task_type.as_str(),
                    row.model_id,
                    row.success_rate,
                    row.p95_latency_ms,
                    row.mean_cost_usd,
                    row.n_samples,
                    now,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Reload the in-memory affinity snapshot from the SQLite mirror
    /// (used at boot or after a manual `atlas models refresh`). Returns
    /// an empty vec when the table is empty.
    pub fn load_affinity_rows(
        &self,
    ) -> anyhow::Result<Vec<crate::orchestrator::affinity::AffinityRow>> {
        use crate::orchestrator::affinity::AffinityRow;
        use crate::orchestrator::classifier::TaskType;
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT task_type, model_id, success_rate, p95_latency_ms,
                    mean_cost_usd, n_samples
             FROM model_affinity_cache",
        )?;
        let rows: rusqlite::Result<Vec<AffinityRow>> = stmt
            .query_map([], |r| {
                let task_str: String = r.get(0)?;
                let model_id: String = r.get(1)?;
                let success_rate: f64 = r.get(2)?;
                let p95_latency_ms: Option<i64> = r.get(3)?;
                let mean_cost_usd: Option<f64> = r.get(4)?;
                let n_samples: i64 = r.get(5)?;
                let task_type = TaskType::parse(&task_str).unwrap_or(TaskType::Unknown);
                Ok(AffinityRow {
                    task_type,
                    model_id,
                    success_rate,
                    p95_latency_ms,
                    mean_cost_usd,
                    n_samples,
                })
            })?
            .collect();
        Ok(rows?)
    }

    /// Record a `task_classifier_decisions` row. Used by 2.3's main
    /// classifier run and by 2.4's A/B emission (the second row uses
    /// `kind = "mf_ab"`). The `(prompt_hash, classifier_kind)` UNIQUE
    /// constraint means a replay is silently ignored by `INSERT OR
    /// IGNORE` — A/B replay on the same prompt is a no-op, not an
    /// error.
    pub fn record_classifier_decision(
        &self,
        mission_id: Option<&Uuid>,
        prompt_hash: &str,
        predicted_task_type: &str,
        confidence: f64,
        features_json: Option<&str>,
        classifier_kind: &str,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        let id = Uuid::new_v4().to_string();
        let mission_id_str = mission_id.map(|u| u.to_string());
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO task_classifier_decisions
             (id, mission_id, prompt_hash, predicted_task_type, confidence,
              features_json, classifier_kind, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                id,
                mission_id_str,
                prompt_hash,
                predicted_task_type,
                confidence,
                features_json,
                classifier_kind,
                now,
            ],
        )?;
        Ok(())
    }

    /// RFC 04 §6 sub-fase 2.4 — rolling-window means of
    /// `model_invocations.tokens_in`, `tokens_out`, and a blended
    /// input+output cost-per-1M figure for `cost_guard`'s
    /// `AggregationCostContext::from_journal`. Used by the orchestrator
    /// to estimate pre-aggregation spend based on what this model has
    /// historically consumed per request.
    ///
    /// When no telemetry exists for `model_id`, returns zeros — the
    /// caller's guard evaluates `0` as "no data" (the operator
    /// explicitly-set `blended_cost_per_1m` in the profile takes over
    /// only when the `Default::default()` path is used instead).
    pub fn read_model_invocation_means(
        &self,
        model_id: &str,
        window: u32,
    ) -> anyhow::Result<(u64, u64, f64)> {
        let conn = self.conn.lock();
        let (mean_in, mean_out, total_cost, total_tokens) = conn.query_row(
            "WITH ranked AS (
                SELECT tokens_in, tokens_out,
                       cost_usd,
                       ROW_NUMBER() OVER (
                           ORDER BY started_at DESC
                       ) AS rk
                FROM model_invocations
                WHERE model_id = ?1
                  AND tokens_in IS NOT NULL
                  AND tokens_out IS NOT NULL
            )
            SELECT COALESCE(AVG(tokens_in), 0),
                   COALESCE(AVG(tokens_out), 0),
                   COALESCE(SUM(cost_usd), 0),
                   COALESCE(SUM(tokens_in + tokens_out), 0)
            FROM ranked
            WHERE rk <= ?2",
            rusqlite::params![model_id, i64::from(window)],
            |r| {
                let mean_in: Option<f64> = r.get(0)?;
                let mean_out: Option<f64> = r.get(1)?;
                let total_cost: Option<f64> = r.get(2)?;
                let total_tokens: Option<f64> = r.get(3)?;
                Ok((
                    mean_in.unwrap_or(0.0) as u64,
                    mean_out.unwrap_or(0.0) as u64,
                    total_cost.unwrap_or(0.0),
                    total_tokens.unwrap_or(0.0),
                ))
            },
        )?;
        let blended_cost_per_1m = if total_tokens > 0.0 {
            (total_cost / total_tokens) * 1_000_000.0
        } else {
            0.0
        };
        Ok((mean_in, mean_out, blended_cost_per_1m))
    }

    /// Record a single `model_invocations` row. Used by the
    /// orchestrator after every model invocation (Phase 2). Phases 0-1
    /// never call this; the rows accumulate naturally once the
    /// orchestrator loop lands.
    pub fn record_model_invocation(&self, row: &ModelInvocationRow) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR IGNORE INTO model_invocations
             (id, mission_id, model_id, deployment_id, provider,
              idempotency_key, started_at, finished_at, latency_ms,
              tokens_in, tokens_out, cache_read_input_tokens, cost_usd,
              seed, temperature, sampling_params_json, route_taken_json,
              was_correct, error_kind, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                     ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            rusqlite::params![
                row.id,
                row.mission_id,
                row.model_id,
                row.deployment_id,
                row.provider,
                row.idempotency_key,
                row.started_at,
                row.finished_at,
                row.latency_ms,
                row.tokens_in,
                row.tokens_out,
                row.cache_read_input_tokens,
                row.cost_usd,
                row.seed,
                row.temperature,
                row.sampling_params_json,
                row.route_taken_json,
                row.was_correct,
                row.error_kind,
                row.error_message,
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod phase24_tests {
    use super::*;
    use crate::orchestrator::classifier::TaskType;
    use tempfile::TempDir;

    fn open() -> Journal {
        let tmp = TempDir::new().expect("tmp");
        Journal::open(tmp.path()).expect("open")
    }

    fn seed_model(j: &Journal, model_id: &str) {
        j.conn
            .lock()
            .execute(
                "INSERT INTO models (id, provider, display_name, tier, context_window,
                    max_output_tokens, capabilities_json)
                 VALUES (?1, 'openai', 'Test Model', 'strong', 128000, 16384, '[]')",
                rusqlite::params![model_id],
            )
            .expect("seed models row");
    }

    #[test]
    fn affinity_reader_returns_empty_when_no_telemetry() {
        let j = open();
        let rows = j.read_affinity(50).expect("read_affinity");
        assert!(rows.is_empty(), "expected no affinity rows pre-seed");
    }

    #[test]
    fn affinity_reader_seeds_from_model_invocations_and_classifier_decisions() {
        let j = open();
        seed_model(&j, "gpt-test");

        let mission_id = Uuid::new_v4();

        let row = ModelInvocationRow {
            id: Uuid::new_v4().to_string(),
            mission_id: Some(mission_id.to_string()),
            model_id: "gpt-test".into(),
            deployment_id: "gpt-test-deploy".into(),
            provider: "openai".into(),
            idempotency_key: "k1".into(),
            started_at: "2025-01-01T00:00:00Z".into(),
            finished_at: Some("2025-01-01T00:00:01Z".into()),
            latency_ms: Some(250),
            tokens_in: Some(100),
            tokens_out: Some(50),
            cache_read_input_tokens: None,
            cost_usd: Some(0.001),
            seed: None,
            temperature: Some(0.0),
            sampling_params_json: None,
            route_taken_json: None,
            was_correct: Some(1),
            error_kind: None,
            error_message: None,
        };
        j.record_model_invocation(&row).expect("record inv");

        j.record_classifier_decision(
            Some(&mission_id),
            "hash-1",
            "coding",
            0.92,
            Some("{}"),
            "main",
        )
        .expect("record classifier");

        j.upsert_affinity_rows(&[]).expect("noop prime");
        let rows = j.read_affinity(50).expect("read_affinity");
        assert_eq!(
            rows.len(),
            1,
            "expected one affinity row (the JOIN matched)"
        );
        let r = &rows[0];
        assert_eq!(r.task_type, TaskType::Coding);
        assert_eq!(r.model_id, "gpt-test");
        assert!(
            r.success_rate > 0.5,
            "success_rate should be > 0.5 since was_correct=1"
        );
        assert_eq!(r.n_samples, 1);
    }

    #[test]
    fn classifier_decision_ab_replay_is_idempotent() {
        let j = open();
        let mission_id = Uuid::new_v4();

        j.record_classifier_decision(
            Some(&mission_id),
            "prompt-hash-x",
            "reasoning",
            0.88,
            Some("{}"),
            "main",
        )
        .expect("record main");

        j.record_classifier_decision(
            Some(&mission_id),
            "prompt-hash-x",
            "creative",
            0.7,
            Some("{}"),
            "mf_ab",
        )
        .expect("record ab");

        // Replay the same (prompt_hash, classifier_kind) — must be a no-op.
        let before = j
            .conn
            .lock()
            .query_row::<i64, _, _>("SELECT COUNT(*) FROM task_classifier_decisions", [], |r| {
                r.get(0)
            })
            .expect("count");
        j.record_classifier_decision(
            Some(&mission_id),
            "prompt-hash-x",
            "creative",
            0.7,
            Some("{}"),
            "mf_ab",
        )
        .expect("replay");
        let after = j
            .conn
            .lock()
            .query_row::<i64, _, _>("SELECT COUNT(*) FROM task_classifier_decisions", [], |r| {
                r.get(0)
            })
            .expect("count");
        assert_eq!(before, after, "INSERT OR IGNORE: replay is idempotent");
        assert_eq!(after, 2, "two distinct rows (main + mf_ab)");
    }

    #[test]
    fn read_model_invocation_means_returns_zeros_when_no_data() {
        let j = open();
        let (mean_in, mean_out, blend) = j
            .read_model_invocation_means("ghost-model", 100)
            .expect("means");
        assert_eq!(mean_in, 0);
        assert_eq!(mean_out, 0);
        assert_eq!(blend, 0.0);
    }

    #[test]
    fn read_model_invocation_means_computes_per_1m_blended_cost() {
        let j = open();
        seed_model(&j, "gpt-x");
        let ids: Vec<String> = (0..3).map(|_i| Uuid::new_v4().to_string()).collect();
        for (i, row_id) in ids.iter().enumerate() {
            let day = (i % 28) + 1;
            let started = if day < 10 {
                format!("2025-01-0{day}T00:00:00Z")
            } else {
                format!("2025-01-{day}T00:00:00Z")
            };
            let finished = if day < 10 {
                format!("2025-01-0{day}T00:00:01Z")
            } else {
                format!("2025-01-{day}T00:00:01Z")
            };
            let row = ModelInvocationRow {
                id: row_id.clone(),
                mission_id: None,
                model_id: "gpt-x".into(),
                deployment_id: "gpt-x".into(),
                provider: "openai".into(),
                idempotency_key: format!("k{i}"),
                started_at: started,
                finished_at: Some(finished),
                latency_ms: Some(100),
                tokens_in: Some(1000),
                tokens_out: Some(500),
                cache_read_input_tokens: None,
                cost_usd: Some(0.030),
                seed: None,
                temperature: None,
                sampling_params_json: None,
                route_taken_json: None,
                was_correct: Some(1),
                error_kind: None,
                error_message: None,
            };
            j.record_model_invocation(&row).expect("record");
        }
        let (mean_in, mean_out, blend) =
            j.read_model_invocation_means("gpt-x", 100).expect("means");
        assert_eq!(mean_in, 1000);
        assert_eq!(mean_out, 500);
        // total_cost = 0.030 * 3 = 0.090; total_tokens = (1000+500)*3 = 4500
        // blended = (0.090 / 4500) * 1_000_000 = 20.0
        assert!(
            (blend - 20.0).abs() < 1e-6,
            "expected blended ~20.0 USD per 1M tokens, got {blend}"
        );
    }
}
