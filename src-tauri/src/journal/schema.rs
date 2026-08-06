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
    //   is when OpenCode OS saw the error. `status_code` is 429/402/403;
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
    Ok(())
}
