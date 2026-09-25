// Atlas OS — Journal schema migrations (Phase 0, Roadmap Fase 0).
// Each migration is idempotent. KERNEL_BUS event_id is the idempotency key —
// re-publishing the same event is silently dropped (NOT replaced), per
// RFC 02 §3.1.2 (at-least-once delivery with idempotent consumers).

use anyhow::Result;
use rusqlite::Connection;

/// Highest migration version applied by this binary. Single source of truth
/// for "the schema this binary knows how to produce". Tests assert
/// `version >= CURRENT_SCHEMA_VERSION` after `migrate()`; the final migration
/// body writes `params![CURRENT_SCHEMA_VERSION, …]`. Historical migration
/// literals (1..N-1) are frozen — they are part of the idempotent migration
/// log and must never be renumbered. When adding M(N+1): bump this const
/// AND change the final migration's `params![N, …]` to
/// `params![CURRENT_SCHEMA_VERSION, …]` (same value).
pub const CURRENT_SCHEMA_VERSION: i64 = 32;

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
    if current < 13 {
        // M13 — Autoresearch loop (RFC 28 �A, derived from
        //   karpathy/autoresearch program.md, MIT).
        //
        //   autoresearch_runs     : one row per asked `opencode mission
        //     --autoresearch`. Carries the metric_command (deterministic
        //     shell pipeline whose stdout is parsed to a REAL), the
        //     baseline, the best metric observed, git SHA bookends, the
        //     step budget, the wall-clock budget, and the final outcome.
        //   autoresearch_candidates: one row per experiment iteration.
        //     Each candidate has its own git_sha (committed diff_hunk),
        //     the metric recorded BEFORE the candidate ran (so we can
        //     regress against the prior step too), the metric AFTER,
        //     the keep/discarded verdict, and a free-form rationale blob
        //     for the LLM/audit trail.
        //
        // Per RFC 28 �A Riesgos, autoresearch_runs has a hard ceiling
        // of 200 candidates enforced at the supervisor level (the runner
        // aborts with outcome=timeout when step_count hits max_steps or
        // 200, whichever is smaller); the SQL schema does not encode the
        // ceiling to allow future tuning.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS autoresearch_runs (
                id              TEXT PRIMARY KEY,
                mission_id      TEXT NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
                baseline_metric REAL NOT NULL,
                best_metric     REAL,
                git_sha_start   TEXT NOT NULL,
                git_sha_end     TEXT,
                metric_command  TEXT NOT NULL,
                max_steps       INTEGER NOT NULL,
                timebox_seconds INTEGER NOT NULL,
                step_count      INTEGER NOT NULL DEFAULT 0,
                outcome         TEXT NOT NULL CHECK (outcome IN ('running','improved','plateau','timeout','aborted')),
                ts_started      INTEGER NOT NULL,
                ts_ended        INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_ar_runs_mission ON autoresearch_runs(mission_id);

            CREATE TABLE IF NOT EXISTS autoresearch_candidates (
                id                     TEXT PRIMARY KEY,
                run_id                 TEXT NOT NULL REFERENCES autoresearch_runs(id) ON DELETE CASCADE,
                step                   INTEGER NOT NULL,
                git_sha                TEXT NOT NULL,
                diff_hunk              TEXT NOT NULL,
                metric_baseline_at_step REAL NOT NULL,
                metric_after           REAL NOT NULL,
                kept                   INTEGER NOT NULL CHECK (kept IN (0,1)),
                rationale              TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_ar_candidates_run ON autoresearch_candidates(run_id, step);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![13, chrono::Utc::now().to_rfc3339()],
        )?;
    }

    if current < 14 {
        // M14 — Agent session events (RFC 28 §B, derived from Microsoft
        //   Intelligent Terminal `doc/specs/llm-agent-event-integration.md`,
        //   MIT — verbatim spec snapshot copy_uso'd into
        //   `src-tauri/specs/osc-9001.md` during the Phase-0 batch XS).
        //
        //   agent_session_events : one row per in-band OSC 9001 envelope
        //     captured by the `wtcli listen --json` worker (RFC 28 §B
        //     Channel 2). The worker subprocess spawns `wtcli`, parses
        //     the JSON-line stream, and UPSERTs each envelope here.
        //     `pane_id` is nullable because some IT versions emit events
        //     without a pane (e.g. agent.idle on window close). `task_id`
        //     is nullable for the same reason — agent.started/agent.idle
        //     signals carry no task. `payload_json` is the raw `params`
        //     object of the OSC 9001 `agent_event` envelope, kept verbatim
        //     so any future structured query (per-tool dashboards,
        //     replay) has the full payload without a schema migration.
        //
        // The table itself is NOT feature-gated — a default build still
        // creates it (cheap; lets the HUD read whatever pane telemetry
        // exists even on a non-Windows host). The lifecycle that streets
        // the rows (the `wtcli listen` worker) IS gated behind
        // `acp-server` because spawning wtcli is meaningless on Linux.
        //
        // Per RFC 28 §B, the schema intentionally omits `mission_id`:
        // the linkage from agent_session_events to a mission is
        // deduced by the host from `task_id` <-> `steps.id` joins done
        // at query-time by the HUD card, not stored as a hard FK.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS agent_session_events (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                ts           INTEGER NOT NULL,
                pane_id      TEXT,
                event_type   TEXT NOT NULL,
                agent        TEXT NOT NULL,
                task_id      TEXT,
                payload_json TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_ase_ts   ON agent_session_events(ts);
            CREATE INDEX IF NOT EXISTS idx_ase_task ON agent_session_events(task_id);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![14, chrono::Utc::now().to_rfc3339()],
        )?;
    }

    if current < 15 {
        // M15 — Mission graph (RFC 28 §C, derived from
        //   safishamsi/graphify pattern, Apache-2.0). Three tables:
        //
        //   mission_graph_nodes : one row per graph node attached to a
        //     mission. `kind` enumerates the node source: `engine_state`
        //     (RFC 19 state-machine vertex), `mission` (the root), `skill`
        //     (RFC 06 skill node), `external` (something the mission
        //     depends on outside the kernel — a library, a file, a remote
        //     MCP). `provenance` follows graphify's tagged tags:
        //     EXTRACTED (parsed from source/AST, factual), INFERRED (LLM
        //     / Planner deduced), AMBIGUOUS (low-confidence — operator
        //     review pending).
        //   mission_graph_edges : directed edges. `kind` ∈ {calls, imports,
        //     transitions_to, depends_on, references}. `precondition` +
        //     `guard` carry the RFC 19 DFA decorations for the
        //     Execution Supervisor branch selection. `visit_count`
        //     is bumped every time the supervisor traverses the edge —
        //     feeds the doom-loop watcher.
        //   learning_graphs      : cache of successful mission graphs keyed
        //     by an `intent_signature` (a deterministic hash of the
        //     consolidated mission prompt, per RFC 16 §6). Retrieved
        //     top-k by cosine (sqlite-vec + fastembed-rs, RFC 09) to
        //     feed *INFERRED* hints to the Planner on similar future
        //     missions.
        //
        // All three tables are owned by RFC 28 §C migration; they are
        // created unconditionally (no feature flag) because reading the
        // graph is cheap even when the writer is gated. The DAG emitter
        // (Planner) and the AST extractor are the gated parts — see
        // features `dag_mode` and `codebase-graph` in Cargo.toml.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS mission_graph_nodes (
                id          TEXT PRIMARY KEY,
                mission_id  TEXT NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
                kind        TEXT NOT NULL CHECK (kind IN ('engine_state','mission','skill','external')),
                label       TEXT NOT NULL,
                provenance  TEXT NOT NULL CHECK (provenance IN ('EXTRACTED','INFERRED','AMBIGUOUS')),
                attrs_json  TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_mg_nodes_mission ON mission_graph_nodes(mission_id);

            CREATE TABLE IF NOT EXISTS mission_graph_edges (
                id           TEXT PRIMARY KEY,
                mission_id   TEXT NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
                src          TEXT NOT NULL REFERENCES mission_graph_nodes(id) ON DELETE CASCADE,
                dst          TEXT NOT NULL REFERENCES mission_graph_nodes(id) ON DELETE CASCADE,
                kind         TEXT NOT NULL CHECK (kind IN ('calls','imports','transitions_to','depends_on','references')),
                precondition TEXT,
                guard        TEXT,
                visit_count  INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_mg_edges_src ON mission_graph_edges(src);
            CREATE INDEX IF NOT EXISTS idx_mg_edges_dst ON mission_graph_edges(dst);
            CREATE INDEX IF NOT EXISTS idx_mg_edges_mission ON mission_graph_edges(mission_id);

            CREATE TABLE IF NOT EXISTS learning_graphs (
                id               TEXT PRIMARY KEY,
                intent_signature TEXT NOT NULL,
                success          INTEGER NOT NULL CHECK (success IN (0,1)),
                graph_json        TEXT NOT NULL,
                created_ts        INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_lg_sig ON learning_graphs(intent_signature);
            CREATE INDEX IF NOT EXISTS idx_lg_created ON learning_graphs(created_ts);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![15, chrono::Utc::now().to_rfc3339()],
        )?;
    }

    // M16 — Learning graphs embedding columns (RFC 28 §C item 6).
    //   Adds `embedding` (BLOB of little-endian f32, nullable) and
    //   `emb_model` (model id string, nullable) to `learning_graphs`.
    //   Nullable so a Phase-1.5c instance without the optional `fastembed`
    //   feature still inserts rows — the retrieve layer falls back to
    //   exact `intent_signature` match in that case. When the optional
    //   `fastembed` feature is on, the persist layer fills both columns
    //   and the retrieve layer does in-memory cosine over the cached
    //   rows (sqlite-vec load_extension is best-effort per RFC 25 §3.4
    //   and may not be available on every host, so we do not depend on
    //   the vec0 virtual table here).
    if current < 16 {
        conn.execute_batch(
            "ALTER TABLE learning_graphs ADD COLUMN embedding BLOB;
             ALTER TABLE learning_graphs ADD COLUMN emb_model TEXT;",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![16, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    // M17 — Toast notifications queue + history (RFC 28 §F).
    //   `toast_queue` holds scheduled notifications that the driver
    //   loop polls every 5 s. The `kind` column is a stable TEXT
    //   token (`model_ready` | `turn_end` | `validation_failed` |
    //   `calendar_reminder` | `critical` | `info`); status is one of
    //   `pending` / `fired` / `dismissed` / `failed`. `fire_at`,
    //   `fired_at`, `dismissed_at` are unix milliseconds. Deep-link
    //   strings use the `opencode://mission/...` scheme.
    //   `toast_history` is the append-only audit + dedupe ledger
    //   consulted during crash-recovery backfill.
    if current < 17 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS toast_queue (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                kind            TEXT NOT NULL,
                title           TEXT NOT NULL,
                body            TEXT,
                deep_link       TEXT,
                fire_at         INTEGER NOT NULL,
                status          TEXT NOT NULL DEFAULT 'pending',
                fired_at        INTEGER,
                dismissed_at    INTEGER,
                dismiss_reason  TEXT,
                attempts        INTEGER NOT NULL DEFAULT 0,
                created_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
            );

            CREATE INDEX IF NOT EXISTS toast_queue_pending_idx
                ON toast_queue(fire_at)
                WHERE status = 'pending';

            CREATE TABLE IF NOT EXISTS toast_history (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                queue_id        INTEGER NOT NULL,
                kind            TEXT NOT NULL,
                title           TEXT NOT NULL,
                body            TEXT,
                deep_link       TEXT,
                fire_at         INTEGER NOT NULL,
                fired_at        INTEGER,
                dismiss_reason  TEXT,
                outcome         TEXT NOT NULL,
                recorded_at     INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
            );

            CREATE INDEX IF NOT EXISTS toast_history_dedupe_idx
                ON toast_history(kind, title, fire_at);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![17, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    // M18 — Calendar integration (RFC 28 §G).
    //   `calendar_busy_windows` stores busy events consumed by the
    //   Planning engine to decide whether to enqueue a proactive turn
    //   now or wait until the next free slot. `source` enumerates the
    //   provenance: `graph` (MS Graph `/me/calendarView` poller),
    //   `ics_local` (any ICS feed the user subscribed via
    //   `opencode calendar subscribe`), or `manual` (operator-added).
    //   `weight` is 0.0..=1.0 — `graph` defaults to 1.0, an
    //   `OPENCODE`-tagged event defaults to 1.0 (full busy), an
    //   untagged event to 0.5 (soft busy).
    //   `calendar_auth` holds the encrypted refresh token for the
    //   MS Graph reader. `account` is the principal (UPN / email);
    //   `token_ciphertext` is AES-256-GCM(nonce||ciphertext||tag) of
    //   the refresh-token JSON, `key_hint` is a short stable
    //   identifier of the local key (so future multiple-device
    //   setups can support key rotation). `expires_at` is the access
    //   token's expiry (separately tracked so the poller can
    //   silent-refresh before the access token expires). All times
    //   are unix milliseconds. M18 is created unconditionally (no
    //   feature gate): a default build still creates the tables (cheap
    //   schema bump) so a future enablement of `calendar-ics` /
    //   `calendar-graph` doesn't require a migration step.
    if current < 18 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS calendar_busy_windows (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                source      TEXT NOT NULL CHECK (source IN ('graph','ics_local','manual')),
                external_id TEXT NOT NULL,
                subject     TEXT NOT NULL,
                body        TEXT,
                starts_at   INTEGER NOT NULL,
                ends_at     INTEGER NOT NULL,
                weight      REAL NOT NULL DEFAULT 1.0,
                recorded_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
                UNIQUE(source, external_id)
            );

            CREATE INDEX IF NOT EXISTS cal_busy_starts_idx
                ON calendar_busy_windows(starts_at);

            CREATE TABLE IF NOT EXISTS calendar_auth (
                account         TEXT PRIMARY KEY,
                token_ciphertext BLOB NOT NULL,
                key_hint        TEXT NOT NULL,
                expires_at      INTEGER,
                updated_at      INTEGER NOT NULL DEFAULT (unixepoch() * 1000)
            );",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![18, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    // M19 — Model API reset-window tracking (RFC 28 §H).
    //   `model_resets` is the append-only ledger of provider-model
    //   resets observed from upstream error responses (429 rate limit
    //   or 402/403 spend cap). One row per `(provider, model,
    //   resets_at)` triple — UNIQUE de-dupes when the same reset is
    //   re-observed (e.g. poller fetches an unchanged envelope each
    //   minute). `resets_at` is unix-millis absolute (the moment the
    //   provider says the model will be usable again). `observed_at`
    //   is when Atlas OS saw the error. `status_code` is 429/402/403;
    //   `error_type` is `rate_limit | spend_limit | NULL` (the OmniRoute
    //   envelope's `error.type` field when going via OmniRoute, NULL
    //   when the provider is hit directly and didn't classify).
    //   `request_id` is the upstream request id (provider-dependent);
    //   it's not used as a key but kept for forensics and dedupe.
    //   `toast_dismissed_at` is NULL until the `model_ready` Toast
    //   we enqueue into `toast_queue` is dismissed by the user — the
    //   partial index `model_resets_pending_idx` lists only rows where
    //   `toast_dismissed_at IS NULL`, which is the scheduler's working
    //   set. M19 created unconditionally: a default build still
    //   creates the table so a future enablement of §H plumbing doesn't
    //   need a migration step.
    if current < 19 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS model_resets (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                provider        TEXT NOT NULL,
                model           TEXT NOT NULL,
                status_code     INTEGER NOT NULL,
                error_type      TEXT,
                resets_at       INTEGER NOT NULL,
                request_id      TEXT,
                observed_at     INTEGER NOT NULL DEFAULT (unixepoch()),
                toast_id        INTEGER,
                toast_dismissed_at INTEGER
            );

            CREATE UNIQUE INDEX IF NOT EXISTS model_resets_uniq_idx
                ON model_resets(provider, model, resets_at);

            CREATE INDEX IF NOT EXISTS model_resets_pending_idx
                ON model_resets(resets_at)
                WHERE toast_dismissed_at IS NULL;",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![19, chrono::Utc::now().to_rfc3339()],
        )?;
    }

    // M20 — RFC 04 §1 Phase 2 sub-fase 2.0:
    //
    //   * `models`             — overridable cost / context window /
    //                            capability metadata per logical model.
    //                            Overrides take precedence over the JSON
    //                            seed in `orchestrator/assets/`. The seed
    //                            remains the source-of-record for build-
    //                            time-stable values; this table holds
    //                            user/operator overrides and per-deployment
    //                            runtime knobs.
    //   * `deployments`        — RFC 04 §1 multi-key pooling: one
    //                            logical model served by N deployments
    //                            (distinct api_key / api_base / region).
    //                            Cooldown (sub-fase 2.0.5) tracks here.
    //   * `model_aliases`      — alias → model_id reverse map, persistent;
    //                            JSON seed populates it at boot when the
    //                            row is absent (INSERT OR IGNORE).
    //   * `model_groups`       — RFC 04 §1 routing groups (LiteLLM port).
    //                            One group = a set of model_ids sharing a
    //                            routing_strategy. Sub-fase 2.1 plumbs
    //                            the actual routing logic.
    //
    // Phase 2 sub-fase 2.0 only materialises the table layouts +
    // indexes. The INSERT/upsert logic lands with the registry reader
    // in sub-fase 2.1 (alongside `model_invocations`, M21).
    if current < 20 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS models (
                id                      TEXT PRIMARY KEY,
                provider                TEXT NOT NULL,
                display_name            TEXT NOT NULL,
                tier                    TEXT NOT NULL,
                context_window          INTEGER NOT NULL,
                max_output_tokens       INTEGER NOT NULL,
                capabilities_json       TEXT NOT NULL,
                input_cost_per_1m       REAL,
                output_cost_per_1m      REAL,
                cache_read_cost_per_1m  REAL,
                latency_ms_p50          INTEGER,
                updated_at              INTEGER NOT NULL DEFAULT (unixepoch())
            );

            CREATE INDEX IF NOT EXISTS models_tier_idx ON models(tier);

            CREATE TABLE IF NOT EXISTS deployments (
                id                      TEXT PRIMARY KEY,
                model_id                TEXT NOT NULL REFERENCES models(id),
                label                   TEXT NOT NULL DEFAULT '',
                api_base                TEXT NOT NULL,
                region                  TEXT,
                priority                INTEGER NOT NULL DEFAULT 0,
                weight                  REAL NOT NULL DEFAULT 1.0,
                tokens_per_minute       INTEGER,
                requests_per_minute     INTEGER,
                max_parallel            INTEGER,
                api_key_env             TEXT,
                cooldown_until          INTEGER,
                fails_this_minute       INTEGER NOT NULL DEFAULT 0,
                last_fail_at            INTEGER,
                enabled                 INTEGER NOT NULL DEFAULT 1
            );

            CREATE INDEX IF NOT EXISTS deployments_model_idx
                ON deployments(model_id, enabled);
            CREATE INDEX IF NOT EXISTS deployments_cooldown_idx
                ON deployments(cooldown_until)
                WHERE cooldown_until IS NOT NULL;

            CREATE TABLE IF NOT EXISTS model_aliases (
                alias           TEXT PRIMARY KEY,
                model_id        TEXT NOT NULL REFERENCES models(id),
                mutable         INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS model_groups (
                name            TEXT PRIMARY KEY,
                strategy       TEXT NOT NULL,
                strategy_args_json TEXT,
                models_json     TEXT NOT NULL,
                updated_at      INTEGER NOT NULL DEFAULT (unixepoch())
            );",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![20, chrono::Utc::now().to_rfc3339()],
        )?;
    }

    // M21 — RFC 04 §2/§4 Phase 2 sub-fase 2.1:
    //
    //   `model_invocations` — per-request telemetry. Each row = ONE
    //                        model invocation (including fallback
    //                        retries). Drives the affinity learner of
    //                        sub-fase 2.4 (was_correct feedback loop) and
    //                        the LatencyBased / UsageBasedV2 / LeastBusy
    //                        routing strategies of sub-fase 2.1 itself.
    //
    // Columns mirror the gaps flagged in Round 3 audit:
    //   * `cache_read_input_tokens`        — gap G1 (prompt cache accounting).
    //     Without this column the feedback loop reports Anthropic
    //     deployments 3-5× more expensive than reality and the affinity
    //     learner diverges.
    //   * `seed` / `temperature` / `sampling_params_json` — gap G3
    //     (sampling params persist). NULL when unset so the JSON
    //     aggregator of sub-fase 2.2 can omit them.
    //   * `route_taken_json`               — array ordering of the
    //     fallback cascade actually executed (provider, model_id,
    //     outcome) LiteLLM-style. Lets sub-fase 2.4 audit which fallback
    //     bucket caught the request.
    //   * `idempotency_key`                — gap G12. The orchestrator
    //     mints a uuid v4 per logical `RequestFrame` and persists it
    //     here so replayed requests (network retry / pane fork) can
    //     short-circuit before spending another deployment token budget.
    //     UNIQUE-indexed so concurrent enqueues of the same frame collapse
    //     to one row.
    //   * `was_correct`                    — feedback signal, NULL until
    //     the user or the Validation engine (RFC 14) annotates it.
    //     Drives sub-fase 2.4.
    //
    // Foreign-key `model_id` references `models(id)` weakly (no ON DELETE
    // CASCADE) — historical invocations must survive a model being
    // removed from the seed JSON.
    if current < 21 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS model_invocations (
                id                          TEXT PRIMARY KEY,
                mission_id                  TEXT,
                model_id                    TEXT NOT NULL,
                deployment_id              TEXT NOT NULL,
                provider                    TEXT NOT NULL,
                idempotency_key             TEXT NOT NULL,
                started_at                  TEXT NOT NULL,
                finished_at                 TEXT,
                latency_ms                  INTEGER,
                tokens_in                   INTEGER,
                tokens_out                  INTEGER,
                cache_read_input_tokens     INTEGER,
                cost_usd                    REAL,
                seed                        INTEGER,
                temperature                 REAL,
                sampling_params_json        TEXT,
                route_taken_json            TEXT,
                was_correct                 INTEGER,
                error_kind                  TEXT,
                error_message               TEXT,
                FOREIGN KEY (model_id) REFERENCES models(id)
            );

            CREATE INDEX IF NOT EXISTS model_invocations_model_idx
                ON model_invocations(model_id, started_at);
            CREATE INDEX IF NOT EXISTS model_invocations_idem_idx
                ON model_invocations(idempotency_key, started_at);
            CREATE INDEX IF NOT EXISTS model_invocations_mission_idx
                ON model_invocations(mission_id, started_at)
                WHERE mission_id IS NOT NULL;",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![21, chrono::Utc::now().to_rfc3339()],
        )?;
    }

    // M22 — RFC 04 §3 Phase 2 sub-fase 2.2:
    // Aggregation persistence. Two tables:
    //
    // * `reflection_episodes` — Reflexion multi-model verbal
    //   reinforcement (arxiv 2303.11366). Each row is one attempt of
    //   the reflexor upon the executor's failing trace. `attempt_no`
    //   is hard-capped at 3 by a CHECK constraint (paper §3.3
    //   saturation curve flattens after 3 attempts). The UNIQUE
    //   pair `(mission_id, attempt_no)` guarantees a mission cannot
    //   have two concurrent episodes at the same attempt index. The
    //   anti-doom-loop guard in `aggregation/reflexion.rs` queries
    //   the two most recent rows ordered by `created_at` and aborts
    //   when their `failure_signal` hashes are identical (RFC 19
    //   supervisor escalation).
    // * `council_votes` — Multiagent Debate (arxiv 2305.14325). One
    //   row per agent per round. `critique_of_prev` is NULL on
    //   round 1; rounds 2+ populate it with the agent's critique of
    //   the previous round's leading claim. Final fused response is
    //   the most-frequent round-N claim or debater #1's claim when
    //   no majority.
    //
    // Both tables use `TEXT PRIMARY KEY` (uuid v4 string) to keep
    // consistency with the journal's id convention. Foreign keys
    // are intentionally weak (no ON DELETE CASCADE) — historical
    // aggregation rows must survive the removal of their parent
    // mission or model.
    if current < 22 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS reflection_episodes (
                episode_id              TEXT PRIMARY KEY,
                mission_id              TEXT NOT NULL,
                attempt_no              INTEGER NOT NULL CHECK (attempt_no BETWEEN 1 AND 3),
                executor_model          TEXT NOT NULL,
                reflexor_model          TEXT NOT NULL,
                failure_signal          TEXT NOT NULL,
                failure_trace           TEXT,
                verbal_reflection       TEXT NOT NULL,
                injected_prompt_delta   TEXT NOT NULL,
                tokens_in               INTEGER,
                tokens_out              INTEGER,
                created_at              TEXT NOT NULL,
                UNIQUE (mission_id, attempt_no)
            );

            CREATE INDEX IF NOT EXISTS reflection_episodes_mission_idx
                ON reflection_episodes(mission_id, attempt_no);

            CREATE TABLE IF NOT EXISTS council_votes (
                id                      TEXT PRIMARY KEY,
                episode_id              TEXT NOT NULL,
                round                   INTEGER NOT NULL CHECK (round BETWEEN 1 AND 3),
                agent_id                TEXT NOT NULL,
                claim                   TEXT NOT NULL,
                critique_of_prev        TEXT,
                tokens_in               INTEGER,
                tokens_out              INTEGER,
                created_at              TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS council_votes_episode_idx
                ON council_votes(episode_id, round);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![22, chrono::Utc::now().to_rfc3339()],
        )?;
    }

    // M23 — RFC 04 §7 Phase 2 sub-fase 2.3:
    // Auto-routing classifier + MCP tool-capability-aware. Two tables:
    //
    // * `task_classifier_decisions` — append-only audit of every
    //   `TaskTypeClassifier` prediction the orchestrator emits. The
    //   composite UNIQUE over `(prompt_hash, classifier_kind)` lets a
    //   replayed prompt only mint a new row when a different classifier
    //   kind was used (lexical / logreg / embedding). `features_json`
    //   stores the raw feature vector so an offline `opencode
    //   calibrate-classifier` job (G15, Phase 2.5+) can re-fit without
    //   re-tokenising. `confidence` is the softmax-normalised max
    //   probability the classifier emitted (RouteLLM-style threshold
    //   routing uses this value).
    // * `model_affinity_cache` — in-memory cache backed by a SQL mirror
    //   so affinity survives a restart. One row per
    //   `(task_type, model_id)` pair with `success_rate` in [0.0, 1.0],
    //   `p95_latency_ms`, `mean_cost_usd`, and `n_samples` (the rolling
    //   window size used by the reader). The UNIQUE PK over
    //   `(task_type, model_id)` makes the upsert idempotent — sub-fase
    //   2.4 feedback loop will `INSERT OR REPLACE` here when it
    //   re-crunches the affinity window.
    //
    // Foreign keys are intentionally weak. A model row removed from
    // `models` should not invalidate the classifier audit historical
    // record, and the affinity cache is invalidated lazily by the 2.4
    // reader when it re-rebuilds the in-memory map.
    if current < 23 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS task_classifier_decisions (
                id                  TEXT PRIMARY KEY,
                mission_id          TEXT,
                prompt_hash         TEXT NOT NULL,
                predicted_task_type TEXT NOT NULL,
                confidence          REAL NOT NULL CHECK (confidence BETWEEN 0.0 AND 1.0),
                features_json       TEXT,
                classifier_kind     TEXT NOT NULL CHECK (classifier_kind IN ('lexical','logreg','embedding')),
                created_at          TEXT NOT NULL,
                UNIQUE (prompt_hash, classifier_kind)
            );

            CREATE INDEX IF NOT EXISTS task_classifier_decisions_mission_idx
                ON task_classifier_decisions(mission_id, created_at);
            CREATE INDEX IF NOT EXISTS task_classifier_decisions_type_idx
                ON task_classifier_decisions(predicted_task_type, created_at);

            CREATE TABLE IF NOT EXISTS model_affinity_cache (
                task_type           TEXT NOT NULL,
                model_id            TEXT NOT NULL,
                success_rate        REAL NOT NULL CHECK (success_rate BETWEEN 0.0 AND 1.0),
                p95_latency_ms      INTEGER,
                mean_cost_usd       REAL,
                n_samples           INTEGER NOT NULL CHECK (n_samples >= 0),
                updated_at          TEXT NOT NULL,
                PRIMARY KEY (task_type, model_id)
            );",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![23, chrono::Utc::now().to_rfc3339()],
        )?;
    }

    // M24 — sub-fase 2.4. Relax `task_classifier_decisions.classifier_kind`
    // CHECK to accept `'main'` (the primary classifier run, mirroring
    // `ClassifierKind::Lexical` fallback) and `'mf_ab'` (the A/B emission
    // of the experimental `RoutingStrategy::Mf` trained-route selector —
    // RFC 04 §6 sub-fase 2.4 research/29 §248-252). SQLite cannot ALTER
    // a CHECK in-place, so the standard recreate-and-copy idiom applies:
    // create the new table under a temporary name, copy rows in, drop
    // the old, rename, recreate the two indexes (which were dropped
    // when the parent table was dropped). Existing rows (production
    // Phase-1 installs upgraded in-place) keep their `lexical`/`logreg`/
    // `embedding` value unchanged; new rows from 2.4 onward can carry
    // the two new kinds. Idempotent: if `task_classifier_decisions` is
    // absent (fresh install) the outer `IF NOT EXISTS` guards already
    // created it under M23 with a stricter CHECK, and this migration
    // replaces it with the relaxed CHECK.
    if current < 24 {
        conn.execute_batch(
            "BEGIN;

            CREATE TABLE task_classifier_decisions_new (
                id                  TEXT PRIMARY KEY,
                mission_id          TEXT,
                prompt_hash         TEXT NOT NULL,
                predicted_task_type TEXT NOT NULL,
                confidence          REAL NOT NULL CHECK (confidence BETWEEN 0.0 AND 1.0),
                features_json       TEXT,
                classifier_kind     TEXT NOT NULL CHECK (classifier_kind IN
                    ('lexical','logreg','embedding','main','mf_ab')),
                created_at          TEXT NOT NULL,
                UNIQUE (prompt_hash, classifier_kind)
            );

            INSERT INTO task_classifier_decisions_new
                SELECT * FROM task_classifier_decisions;

            DROP TABLE task_classifier_decisions;

            ALTER TABLE task_classifier_decisions_new
                RENAME TO task_classifier_decisions;

            CREATE INDEX IF NOT EXISTS task_classifier_decisions_mission_idx
                ON task_classifier_decisions(mission_id, created_at);
            CREATE INDEX IF NOT EXISTS task_classifier_decisions_type_idx
                ON task_classifier_decisions(predicted_task_type, created_at);

            COMMIT;",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![24, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 25 {
        // M25 — RFC 10 Phase 3 sub-fase 3.0 Foundation:
        //
        //   * `research_runs` gains `status` (`running` | `completed` |
        //     `needing_human`, RFC 10 §10 fail-safe) and `recommended`
        //     (the winning branch id, RFC 10 §7). Both are nullable so
        //     pre-3.0 rows (M1 shape) keep reading untouched; new rows
        //     written by `journal::research` fill them. Added
        //     conditionally via `PRAGMA table_info` so the migration is
        //     idempotent on databases that already carry the columns.
        //   * `research_sources` — one row per evidence hit of a run
        //     (`kind` ∈ documentation | github | paper | web | document
        //     | community, `score` 0..1 pre-filter weight, `fetched_at`
        //     RFC3339). `id` is a uuid v4 string; replays use
        //     `ON CONFLICT DO NOTHING` at the writer layer.
        //   * `research_consensus` — one row per `(run_id, dimension)`
        //     with the 0..100 dimension score plus a free-form `note`
        //     (cited references live in the YAML report payload, RFC 10
        //     §7). Composite PK makes the writer `INSERT OR REPLACE`
        //     idempotent — re-scoring a dimension overwrites, never
        //     duplicates.
        if !table_has_column(conn, "research_runs", "status") {
            conn.execute_batch("ALTER TABLE research_runs ADD COLUMN status TEXT;")?;
        }
        if !table_has_column(conn, "research_runs", "recommended") {
            conn.execute_batch("ALTER TABLE research_runs ADD COLUMN recommended TEXT;")?;
        }
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS research_sources (
                id          TEXT PRIMARY KEY,
                run_id      TEXT NOT NULL REFERENCES research_runs(id),
                kind        TEXT NOT NULL,
                url         TEXT NOT NULL,
                score       REAL,
                fetched_at  TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_research_sources_run
                ON research_sources(run_id);

            CREATE TABLE IF NOT EXISTS research_consensus (
                run_id      TEXT NOT NULL REFERENCES research_runs(id),
                dimension   TEXT NOT NULL CHECK (dimension IN
                    ('community','enterprise','academic','official')),
                score       REAL NOT NULL CHECK (score BETWEEN 0.0 AND 100.0),
                note        TEXT,
                PRIMARY KEY (run_id, dimension)
            );

            CREATE INDEX IF NOT EXISTS idx_research_consensus_run
                ON research_consensus(run_id);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![25, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 26 {
        // M26 — RFC 10 Phase 3 sub-fase 3.4 Hands-on + ramas:
        //
        //   * `research_notes` — one row per operator-attached closed case
        //     (RFC 10 §3 "esto yo lo hice así (profesionalmente)": title,
        //     project, decision, outcome, confidence 0..1, tags as a JSON
        //     array string, attached_at RFC3339, signature). `id` is a
        //     `rn-…` string; replays use `ON CONFLICT DO NOTHING` at the
        //     writer layer (first write wins, RFC 02 §3.1.2). CHECK on
        //     `confidence` keeps the 0..1 invariant at the SQL layer so a
        //     hand-edited DB cannot smuggle an out-of-range weight into
        //     the ×1.5 fold. Application branches (RFC 10 §4 Opción A/B/C)
        //     are DERIVED per run via `research::hands_on::build_branches`
        //     and rendered into the report `proposal:` lines — they are not
        //     a table because they join live consensus + notes + project
        //     hint at query time. The per-run `journal_ref` (`jr-…`,
        //     RFC 10 §7) is a `journal_events` row (`kind='research_run'`,
        //     `event_id=journal_ref`) so `journal tail` audits it.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS research_notes (
                id          TEXT PRIMARY KEY,
                title       TEXT NOT NULL,
                project     TEXT,
                decision    TEXT NOT NULL,
                outcome     TEXT,
                confidence  REAL NOT NULL CHECK (confidence BETWEEN 0.0 AND 1.0),
                tags_json   TEXT NOT NULL DEFAULT '[]',
                attached_at TEXT NOT NULL,
                signature   TEXT NOT NULL,
                created_at  TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_research_notes_attached
                ON research_notes(attached_at);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![26, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 27 {
        // M27 — RFC 10 Phase 3 sub-fase 3.5 probe_feasibility cache:
        //
        //   * `feasibility_cache` — one row per normalised probe
        //     (`cache_key` = lowercased topic + sorted domain tags, see
        //     `research::feasibility::cache_key`). `report_json` carries the
        //     full `FeasibilityReport` so a repeated prompt reuses the
        //     verdict without re-hitting the registries; `created_at`
        //     (RFC3339) drives the 12-day TTL check
        //     (`research::feasibility::is_cache_fresh`, RFC 10 §11.6).
        //     Writes are `INSERT OR REPLACE` — a fresh probe overwrites a
        //     stale row for the same key (first write does NOT win here,
        //     unlike `research_notes`: the cache must converge on the
        //     newest evidence).
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS feasibility_cache (
                cache_key   TEXT PRIMARY KEY,
                topic       TEXT NOT NULL,
                domains     TEXT NOT NULL,
                report_json TEXT NOT NULL,
                created_at  TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_feasibility_cache_created
                ON feasibility_cache(created_at);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![27, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 28 {
        // M28 — RFC 29 §3.C User modeling (Honcho-style).
        //
        //   * `user_profile` — one row per operator (`user_id`). `preferences`
        //     carries `{favorite_models, coding_style, …}` as JSON text;
        //     `knowledge_state` carries `{domains_known, gaps_identified}`
        //     as JSON text. `interaction_history_summary` is a free-form
        //     summary of past interactions. `updated_at` is unix-millis.
        //     Writes are `INSERT … ON CONFLICT DO UPDATE` — the latest
        //     profile snapshot wins (live state, like `step_states` M11),
        //     not the first write. The Prompt Understanding Pipeline
        //     (RFC 23 §2 paso 2) reads this table to resolve
        //     `user_knowledge_gap` (C6): when the prompt matches a
        //     `gaps_identified` entry, the detector offers mentor mode +
        //     shortcuts automatically.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS user_profile (
                user_id TEXT PRIMARY KEY,
                preferences JSON NOT NULL,
                knowledge_state JSON NOT NULL,
                interaction_history_summary TEXT,
                updated_at INTEGER NOT NULL
            );",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![28, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 29 {
        // M29 — RFC 05 Phase 4 sub-fase 4.0 Foundation:
        //
        //   * `swarm_agents` — one row per agent spawned on a mission
        //     (RFC 05 §1 roles). `id` is a uuid v4 string; `mission_id`
        //     is the mission the agent works on. `role` carries the
        //     `Role::as_str` wire name (`swarm::roles::Role`); `model_id`
        //     is the orchestrator-assigned model (NULL until assigned,
        //     RFC 05 §3). `state` is free-form TEXT (`spawned` at
        //     insert; the 4.2 pool lifecycle owns later transitions).
        //     `personality_json` carries the agency-agents-style
        //     personality/processes/deliverables preset (4.1 fills it;
        //     NULL until then). `created_at` is RFC3339. Replays use
        //     `ON CONFLICT DO NOTHING` at the writer layer (first write
        //     wins, RFC 02 §3.1.2).
        //   * `agent_mailbox` — one row per agent-to-agent message
        //     (munder-difflin mailbox, research/31 §A.2). `from_agent`
        //     / `to_agent` are `swarm_agents.id` values (weak FK — the
        //     audit trail must survive agent-row removal). `body_json`
        //     is the message payload. `read_at` is NULL until the
        //     recipient marks the message read (4.3 `mark_read`;
        //     idempotent). Writers/readers land in 4.3; 4.0 only
        //     materialises the table layouts + indexes.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS swarm_agents (
                id               TEXT PRIMARY KEY,
                mission_id       TEXT NOT NULL,
                role             TEXT NOT NULL,
                model_id         TEXT,
                state            TEXT NOT NULL DEFAULT 'spawned',
                personality_json TEXT,
                created_at       TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_swarm_agents_mission
                ON swarm_agents(mission_id);

            CREATE INDEX IF NOT EXISTS idx_swarm_agents_role
                ON swarm_agents(role);

            CREATE TABLE IF NOT EXISTS agent_mailbox (
                id         TEXT PRIMARY KEY,
                from_agent TEXT NOT NULL,
                to_agent   TEXT NOT NULL,
                body_json  TEXT NOT NULL,
                read_at    TEXT,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_agent_mailbox_to
                ON agent_mailbox(to_agent, created_at);

            CREATE INDEX IF NOT EXISTS idx_agent_mailbox_from
                ON agent_mailbox(from_agent);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![29, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 30 {
        // M30 — RFC 32 Phase 5 sub-fase 5.0 Foundation:
        //
        //   * `learned_rules` — one row per promoted `Pattern.rule_id`
        //     (RFC 16 §2 Reflection Loop, §4 auto-reglas). `id` is the
        //     human-readable `r-YYYY-MM-DD-NNN`. `when_trigger` carries
        //     the `RuleWhen` as JSON text; `then_action` the `RuleThen`
        //     as JSON text (same serde shape the `learning::rules` YAML
        //     writer emits, so YAML ↔ Journal round-trips without a
        //     translation layer). `lifecycle` is CHECK-constrained to
        //     the `RuleLifecycle` tags; `priority` mirrors
        //     `RuleLifecycle::default_priority` per stage (0 / 30 / 60).
        //     `was_correct` is nullable (NULL until the 5.1 Reflection
        //     Engine records the first run verdict); `n_applied` counts
        //     persisted promotions. Replays use `ON CONFLICT DO NOTHING`
        //     at the writer layer (first write wins, RFC 02 §3.1.2).
        //   * `compaction_events` — one row per System One compaction
        //     (5.3 fills it; 5.0 only materialises the layout). The
        //     writer lands with `learning::compaction`, so there is no
        //     writer method yet — only the table + indexes.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS learned_rules (
                id           TEXT PRIMARY KEY,
                when_trigger TEXT NOT NULL,
                then_action  TEXT NOT NULL,
                priority     INTEGER NOT NULL DEFAULT 0,
                lifecycle    TEXT NOT NULL DEFAULT 'draft'
                             CHECK (lifecycle IN ('draft','candidate','active','deprecated')),
                was_correct  INTEGER,
                n_applied    INTEGER NOT NULL DEFAULT 0,
                created_at   TEXT NOT NULL,
                updated_at   TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_learned_rules_lifecycle
                ON learned_rules(lifecycle);

            CREATE INDEX IF NOT EXISTS idx_learned_rules_priority
                ON learned_rules(priority);

            CREATE TABLE IF NOT EXISTS compaction_events (
                id             TEXT PRIMARY KEY,
                mission_id     TEXT NOT NULL,
                entries_before INTEGER NOT NULL,
                entries_after  INTEGER NOT NULL,
                summary        TEXT NOT NULL,
                model_id       TEXT NOT NULL,
                created_at     TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_compaction_events_mission
                ON compaction_events(mission_id);",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![30, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    if current < 31 {
        // M31 — RFC 35 Round 7 / research 36 SECTOR B sub-fase 8.0:
        //
        //   * `dir_access` — frecency de worktrees/missions (port del
        //     algoritmo zoxide: aging + ranking, RFC 35 §5). Una fila por
        //     dir (`dir` PK); `record_dir_access` hace UPSERT del
        //     contador + `last_access` (live state como `step_states`
        //     M11, NO first-write-wins: el contador debe converger al
        //     uso real). El score (`count × decay`) se deriva en
        //     `journal::frecency`, no se almacena.
        //   * `journal_events_fts` — virtual table FTS5 sobre
        //     `journal_events` (patrón context-mode, RFC 35 §4). Externa
        //     (`content='journal_events'`) + triggers de sync + backfill
        //     para DBs pre-8.0. Sin crates: `rusqlite` bundled ya trae
        //     FTS5. Cuando el SQLite del host no trae FTS5 (compilación
        //     sin `SQLITE_ENABLE_FTS5`) la parte FTS se omite y
        //     `Journal::search_events` cae a LIKE — la migración nunca
        //     falla por esto.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS dir_access (
                dir           TEXT PRIMARY KEY,
                access_count  INTEGER NOT NULL DEFAULT 0,
                last_access   TEXT NOT NULL,
                created_at    TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_dir_access_last
                ON dir_access(last_access);",
        )?;
        if fts5_available(conn) {
            conn.execute_batch(
                "CREATE VIRTUAL TABLE IF NOT EXISTS journal_events_fts
                    USING fts5(kind, payload, event_id UNINDEXED,
                               content='journal_events', content_rowid='id');

                INSERT INTO journal_events_fts(rowid, kind, payload, event_id)
                    SELECT id, kind, payload, event_id FROM journal_events
                    WHERE id NOT IN (SELECT rowid FROM journal_events_fts);

                CREATE TRIGGER IF NOT EXISTS journal_events_fts_ai
                AFTER INSERT ON journal_events BEGIN
                    INSERT INTO journal_events_fts(rowid, kind, payload, event_id)
                    VALUES (new.id, new.kind, new.payload, new.event_id);
                END;

                CREATE TRIGGER IF NOT EXISTS journal_events_fts_ad
                AFTER DELETE ON journal_events BEGIN
                    INSERT INTO journal_events_fts(
                        journal_events_fts, rowid, kind, payload, event_id)
                    VALUES ('delete', old.id, old.kind, old.payload, old.event_id);
                END;",
            )?;
        }
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![31, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    // M33 — RFC 04 §7 Phase 9 sub-fase 9.1 (Laya classifier backend).
    //   Relax `task_classifier_decisions.classifier_kind` CHECK to
    //   accept `'laya'` (the 4th `ClassifierKind`, RFC 35 §7.1 "System
    //   One" backend). Same recreate-and-copy idiom as M24 (SQLite
    //   cannot ALTER a CHECK in-place). Existing rows keep their
    //   value; new Laya runs record `classifier_kind = 'laya'`.
    //   (Roadmap M32 — findings.json schema + validator — left no DB
    //   change, so schema version 32 carries the M33 change.)
    if current < 32 {
        conn.execute_batch(
            "BEGIN;

            CREATE TABLE task_classifier_decisions_new (
                id                  TEXT PRIMARY KEY,
                mission_id          TEXT,
                prompt_hash         TEXT NOT NULL,
                predicted_task_type TEXT NOT NULL,
                confidence          REAL NOT NULL CHECK (confidence BETWEEN 0.0 AND 1.0),
                features_json       TEXT,
                classifier_kind     TEXT NOT NULL CHECK (classifier_kind IN
                    ('lexical','logreg','embedding','main','mf_ab','laya')),
                created_at          TEXT NOT NULL,
                UNIQUE (prompt_hash, classifier_kind)
            );

            INSERT INTO task_classifier_decisions_new
                SELECT * FROM task_classifier_decisions;

            DROP TABLE task_classifier_decisions;

            ALTER TABLE task_classifier_decisions_new
                RENAME TO task_classifier_decisions;

            CREATE INDEX IF NOT EXISTS task_classifier_decisions_mission_idx
                ON task_classifier_decisions(mission_id, created_at);
            CREATE INDEX IF NOT EXISTS task_classifier_decisions_type_idx
                ON task_classifier_decisions(predicted_task_type, created_at);

            COMMIT;",
        )?;
        conn.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            rusqlite::params![CURRENT_SCHEMA_VERSION, chrono::Utc::now().to_rfc3339()],
        )?;
    }
    Ok(())
}

fn fts5_available(conn: &Connection) -> bool {
    conn.query_row("SELECT sqlite_compileoption_used('ENABLE_FTS5')", [], |r| {
        r.get::<_, i32>(0)
    })
    .map(|v| v != 0)
    .unwrap_or(false)
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("SELECT name FROM pragma_table_info('{table}') WHERE name = ?1");
    conn.query_row(&sql, rusqlite::params![column], |_| Ok(()))
        .is_ok()
}
