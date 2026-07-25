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
    if current < 2 {
        // M2 — Prompt Understanding Pipeline artefacts (RFC 23 §3-§4).
        //   prompt_verdicts     : one row per `PublicUnderstandingVerdict`
        //   mission_consolidated : one row per `MissionConsolidated` (locked mission before Planning)
        //
        // Both tables accept the canonical JSON blob unchanged (serde form is
        // the cross-RFC contract, RFC 23 §3-§4) so a future SvelteKit UI or
        // external MCP consumer can read the JSON verbatim. Top-level columns
        // are duplicated only where the SQL engine needs them for indexing /
        // the Journal tail commands / the HUD live stream.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS prompt_verdicts (
                verdict_id  TEXT PRIMARY KEY,
                session_id  TEXT NOT NULL,
                mission_id TEXT,
                raw_prompt  TEXT NOT NULL,
                ts          TEXT NOT NULL,
                confidence  TEXT NOT NULL,
                rubric_mean REAL NOT NULL,
                recommended_mode TEXT NOT NULL,
                gap_count   INTEGER NOT NULL DEFAULT 0,
                payload     TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_prompt_verdicts_session  ON prompt_verdicts(session_id);
            CREATE INDEX IF NOT EXISTS idx_prompt_verdicts_ts        ON prompt_verdicts(ts);
            CREATE INDEX IF NOT EXISTS idx_prompt_verdicts_confidence ON prompt_verdicts(confidence);

            CREATE TABLE IF NOT EXISTS mission_consolidated (
                mission_id        TEXT PRIMARY KEY,
                verdict_id        TEXT NOT NULL REFERENCES prompt_verdicts(verdict_id),
                generated_at      TEXT NOT NULL,
                mission_statement TEXT NOT NULL,
                suggested_mode    TEXT NOT NULL,
                locked            INTEGER NOT NULL DEFAULT 0,
                locked_at         TEXT,
                locked_by         TEXT,
                requires_research INTEGER NOT NULL DEFAULT 0,
                payload           TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_mission_consolidated_verdict ON mission_consolidated(verdict_id);
            CREATE INDEX IF NOT EXISTS idx_mission_consolidated_locked ON mission_consolidated(locked);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![2, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 3 {
        // M3 — Planning Engine artefacts (RFC 12 §3).
        //   plans : one row per `Plan` produced by `planning::run`.
        //
        // As with the prompt_verdicts/mission_consolidated tables, we store
        // the canonical JSON payload unchanged plus a handful of top-level
        // columns the SQL engine needs for indexing and the Journal tail /
        // HUD live-stream. The `blocked` count is denormalised so the HUD
        // tail can filter "blocked plans" without touching the JSON blob.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS plans (
                plan_id      TEXT PRIMARY KEY,
                mission_id   TEXT NOT NULL,
                verdict_id   TEXT NOT NULL,
                generated_at TEXT NOT NULL,
                strategy     TEXT NOT NULL,
                risk         REAL NOT NULL,
                impact       TEXT NOT NULL,
                confidence   REAL NOT NULL,
                resume_point TEXT NOT NULL,
                blocker_count INTEGER NOT NULL DEFAULT 0,
                payload      TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_plans_mission   ON plans(mission_id);
            CREATE INDEX IF NOT EXISTS idx_plans_verdict  ON plans(verdict_id);
            CREATE INDEX IF NOT EXISTS idx_plans_generated ON plans(generated_at);
            CREATE INDEX IF NOT EXISTS idx_plans_confidence ON plans(confidence);
            CREATE INDEX IF NOT EXISTS idx_plans_blockers  ON plans(blocker_count);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![3, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 4 {
        // M4 — Coding Engine artefacts (RFC 13 §2, §8).
        //   diffs : one row per `Diff` produced by `coding::run`.
        //
        // The canonical JSON payload is the source of truth; the duplicated
        // top-level columns index the HUD tail by plan_id / step_id /
        // generated_at, and the `lines_added` / `lines_removed` pair
        // powers the `agent.diff` re-projection (RFC 02 §3.1 / RFC 24 §4.1)
        // without re-parsing the JSON blob.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS diffs (
                diff_id       TEXT PRIMARY KEY,
                plan_id       TEXT NOT NULL,
                mission_id    TEXT NOT NULL,
                step_id       TEXT NOT NULL,
                agent_id      TEXT NOT NULL,
                generated_at  TEXT NOT NULL,
                lines_added   INTEGER NOT NULL DEFAULT 0,
                lines_removed INTEGER NOT NULL DEFAULT 0,
                file_count    INTEGER NOT NULL DEFAULT 0,
                payload       TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_diffs_plan    ON diffs(plan_id);
            CREATE INDEX IF NOT EXISTS idx_diffs_mission ON diffs(mission_id);
            CREATE INDEX IF NOT EXISTS idx_diffs_step    ON diffs(step_id);
            CREATE INDEX IF NOT EXISTS idx_diffs_generated ON diffs(generated_at);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![4, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 5 {
        // M5 — Validation Engine artefacts (RFC 14 §3).
        //   validation_reports : one row per `ValidationReport` produced by
        //                        `validation::run`.
        //
        // Same JSON-blob + duplicated-index pattern as the M1..M4 tables.
        // The `outcome` and `mode` columns are denormalised so the HUD
        // tail / the Repair Engine (RFC 15) can filter `critical` /
        // `loose` rows without touching the JSON blob. `failed_stage` is
        // `NULL` when the report is `Pass`; the surrounding Repair Engine
        // joins back to `diffs` via `diff_id` when it needs the offending
        // `Diff` to feed back to the Coding Engine.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS validation_reports (
                report_id      TEXT PRIMARY KEY,
                diff_id        TEXT NOT NULL REFERENCES diffs(diff_id),
                plan_id        TEXT NOT NULL,
                mission_id     TEXT NOT NULL,
                generated_at   TEXT NOT NULL,
                mode           TEXT NOT NULL,
                outcome        TEXT NOT NULL,
                failed_stage   TEXT,
                stage_count    INTEGER NOT NULL DEFAULT 0,
                payload        TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_validation_reports_diff     ON validation_reports(diff_id);
            CREATE INDEX IF NOT EXISTS idx_validation_reports_mission  ON validation_reports(mission_id);
            CREATE INDEX IF NOT EXISTS idx_validation_reports_plan     ON validation_reports(plan_id);
            CREATE INDEX IF NOT EXISTS idx_validation_reports_outcome  ON validation_reports(outcome);
            CREATE INDEX IF NOT EXISTS idx_validation_reports_generated ON validation_reports(generated_at);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![5, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 6 {
        // M6 — Repair Engine artefacts (RFC 15 §6).
        //   repair_runs : one row per `RepairReport` produced by `repair::run`.
        //
        // Same JSON-blob + duplicated-index pattern. `outcome` and
        // `triggering_stage` are denormalised so the HUD tail and the
        // Learning Engine (RFC 16) can filter "applied / escalated" /
        // "type_check-triggered" rows without parsing the JSON. The
        // `triggered_by_report_id` and `source_diff_id` columns anchor
        // the repair run in the timeline (RFC 02 §3.1).
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS repair_runs (
                repair_id                TEXT PRIMARY KEY,
                triggered_by_report_id   TEXT NOT NULL REFERENCES validation_reports(report_id),
                source_diff_id           TEXT NOT NULL REFERENCES diffs(diff_id),
                plan_id                  TEXT NOT NULL,
                mission_id               TEXT NOT NULL,
                generated_at             TEXT NOT NULL,
                outcome                  TEXT NOT NULL,
                triggering_stage         TEXT NOT NULL,
                attempt_count            INTEGER NOT NULL DEFAULT 0,
                successful_attempt       INTEGER,
                payload                  TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_repair_runs_report   ON repair_runs(triggered_by_report_id);
            CREATE INDEX IF NOT EXISTS idx_repair_runs_diff      ON repair_runs(source_diff_id);
            CREATE INDEX IF NOT EXISTS idx_repair_runs_mission   ON repair_runs(mission_id);
            CREATE INDEX IF NOT EXISTS idx_repair_runs_outcome    ON repair_runs(outcome);
            CREATE INDEX IF NOT EXISTS idx_repair_runs_generated ON repair_runs(generated_at);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![6, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 7 {
        // M7 — Learning Engine artefacts (RFC 16 §2 / §3 / §4).
        //   pattern_runs : one row per `LearnOutcome` produced by
        //   `learning::run`. The full JSON blob (Pattern + metrics +
        //   evidence_diff inline) is in `payload`; denormalised columns
        //   (`lifecycle`, `stage`, `strategy`, `confidence`) cover the
        //   HUD tail and the Skill Compressor's "find duplicate
        //   patterns" queries without a JSON parse.
        //
        // The `pattern_id` column carries the pattern's UUID when the
        // outcome produced a `Draft`; `Uuid::nil()` otherwise (one
        // row per `learn_id` either way — RFC 16 §6 "mantiene off-line
        // el diff en el Journal para auditar"). `rule_id` carries the
        // human-readable `r-YYYY-MM-DD-NNN` for direct display in the
        // Agent Console (RFC 24 §3).
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS pattern_runs (
                learn_id             TEXT PRIMARY KEY,
                pattern_id           TEXT,
                source_repair_id     TEXT NOT NULL REFERENCES repair_runs(repair_id),
                source_mission_id    TEXT NOT NULL,
                rule_id              TEXT NOT NULL,
                stage                TEXT NOT NULL,
                strategy             TEXT NOT NULL,
                lifecycle            TEXT NOT NULL,
                priority             INTEGER NOT NULL DEFAULT 0,
                confidence           REAL NOT NULL DEFAULT 0,
                was_correct          INTEGER NOT NULL DEFAULT 0,
                tests_passed         INTEGER NOT NULL DEFAULT 0,
                generated_at         TEXT NOT NULL,
                payload              TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_pattern_runs_repair    ON pattern_runs(source_repair_id);
            CREATE INDEX IF NOT EXISTS idx_pattern_runs_mission   ON pattern_runs(source_mission_id);
            CREATE INDEX IF NOT EXISTS idx_pattern_runs_lifecycle ON pattern_runs(lifecycle);
            CREATE INDEX IF NOT EXISTS idx_pattern_runs_stage     ON pattern_runs(stage);
            CREATE INDEX IF NOT EXISTS idx_pattern_runs_generated ON pattern_runs(generated_at);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![7, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 8 {
        // M8 — Execution Supervisor artefacts (RFC 19 §5).
        //   mission_checkpoints : one row per `PersistCheckpoint`
        //   action emitted by the supervisor's state machine.
        //
        // The full JSON `MissionCheckpoint` (phase, plan/report
        // references, budget tally snapshot) lives in `payload`; the
        // denormalised `phase` column lets the HUD "recent
        // checkpoints" panel avoid a JSON parse. The FK to `missions`
        // anchors the resume chain (RFC 19 §5 "Reanudación").
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS mission_checkpoints (
                checkpoint_id   TEXT PRIMARY KEY,
                mission_id      TEXT NOT NULL,
                phase           TEXT NOT NULL,
                budget_tally    TEXT NOT NULL,
                generated_at    TEXT NOT NULL,
                payload         TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_checkpoints_mission   ON mission_checkpoints(mission_id);
            CREATE INDEX IF NOT EXISTS idx_checkpoints_phase      ON mission_checkpoints(phase);
            CREATE INDEX IF NOT EXISTS idx_checkpoints_generated ON mission_checkpoints(generated_at);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![8, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 9 {
        // M9 — Skill Graph artefacts (RFC 06 §1 / §10).
        //   skill_manifests : one row per `(id, version)` pair.
        //
        // Skills evolve over time (RFC 06 §5 compression, RFC 16 §6
        // learning); keeping the `version` in the PK lets the HUD show
        // the full history of a skill id. The denormalised `engine`
        // and `priority` columns cover the §3 candidate pipeline's
        // "ORDER BY priority" path without a JSON parse. When a skill
        // is deprecated (RFC 06 §5) it is re-saved with
        // `verified=false, priority=0` rather than deleted — the audit
        // trail must survive.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS skill_manifests (
                skill_id          TEXT NOT NULL,
                version           TEXT NOT NULL,
                engine            TEXT NOT NULL,
                priority          INTEGER NOT NULL DEFAULT 0,
                domain            TEXT,
                language          TEXT,
                framework         TEXT,
                confidence        REAL NOT NULL DEFAULT 0,
                auto_generated    INTEGER NOT NULL DEFAULT 0,
                verified          INTEGER NOT NULL DEFAULT 0,
                requires_sandbox  INTEGER NOT NULL DEFAULT 0,
                generated_at      TEXT NOT NULL,
                payload           TEXT NOT NULL,
                PRIMARY KEY (skill_id, version)
            );

            CREATE INDEX IF NOT EXISTS idx_skills_engine      ON skill_manifests(engine);
            CREATE INDEX IF NOT EXISTS idx_skills_priority    ON skill_manifests(priority);
            CREATE INDEX IF NOT EXISTS idx_skills_generated   ON skill_manifests(generated_at);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![9, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 10 {
        // M10 — Model hot-swap artefacts (RFC 27 §B).
        //   model_swaps : one row per `ModelSwapped` bus event. Each swap
        //   is an INSERT (append-only); the audit trail must survive
        //   re-opens so we never UPDATE or DELETE. The denormalised
        //   `initiator` column lets the HUD tail filter "auto fail-over"
        //   vs "user-initiated" swaps without parsing the JSON payload.
        //
        // `prev_model_id` is denormalised so the operator can grep "which
        // missions ran on gpt-4o before they were swapped off it" with
        // a single index scan.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS model_swaps (
                swap_id        TEXT PRIMARY KEY,
                mission_id     TEXT NOT NULL,
                prev_model_id  TEXT NOT NULL,
                new_model_id   TEXT NOT NULL,
                initiator      TEXT NOT NULL,
                occurred_at    TEXT NOT NULL,
                payload        TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_model_swaps_mission   ON model_swaps(mission_id);
            CREATE INDEX IF NOT EXISTS idx_model_swaps_prev      ON model_swaps(prev_model_id);
            CREATE INDEX IF NOT EXISTS idx_model_swaps_new       ON model_swaps(new_model_id);
            CREATE INDEX IF NOT EXISTS idx_model_swaps_occurred  ON model_swaps(occurred_at);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![10, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 11 {
        // M11 — Step-state artefacts (RFC 27 §G).
        //   step_states : one row per `(mission_id, plan_id, step_id)`
        //   phase observation. The PK is `(mission_id, plan_id, step_id)`
        //   and `phase` is UPSERTed (`ON CONFLICT DO UPDATE`) because the
        //   latest phase is the live state — the HUD colour-coded pill
        //   always reflects the most recent observation, not the first
        //   one (in contrast with the append-only `model_swaps`).
        //
        // Each phase transition ALSO publishes a `StepPhaseChanged` bus
        // event (append-only in `journal_events`) for the audit trail;
        // `step_states` is the rolled-up live view.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS step_states (
                mission_id  TEXT NOT NULL,
                plan_id     TEXT NOT NULL,
                step_id     TEXT NOT NULL,
                phase       TEXT NOT NULL,
                updated_at  TEXT NOT NULL,
                PRIMARY KEY (mission_id, plan_id, step_id)
            );

            CREATE INDEX IF NOT EXISTS idx_step_states_mission  ON step_states(mission_id);
            CREATE INDEX IF NOT EXISTS idx_step_states_plan     ON step_states(plan_id);
            CREATE INDEX IF NOT EXISTS idx_step_states_phase    ON step_states(phase);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![11, chrono::Utc::now().to_rfc3339()],
        )?;
    }

    if current < 12 {
        // M12 — Diff annotations (RFC 27 §E).
        //   diff_annotations : one append-only row per annotation posted
        //   against a diff. The HUD drawer renders them inline next to
        //   the changed line; the next Coding Engine run reads them as
        //   additional context so the human reviewer's comments steer
        //   the agent's next attempt (cursor-style).
        //
        // `line_no` is 1-indexed within the file the annotation refers
        // to; `file_path` is the in-repo path of that file (NULL is
        // allowed for diff-level comments).
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS diff_annotations (
                id          TEXT PRIMARY KEY,
                diff_id     TEXT NOT NULL,
                file_path   TEXT,
                line_no     INTEGER,
                body        TEXT NOT NULL,
                author      TEXT NOT NULL,
                created_at  TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_diff_annotations_diff   ON diff_annotations(diff_id);
            CREATE INDEX IF NOT EXISTS idx_diff_annotations_author ON diff_annotations(author);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![12, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    Ok(())
}
