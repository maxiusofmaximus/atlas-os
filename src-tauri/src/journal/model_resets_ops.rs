use crate::journal::*;
// Section H â€” model reset-window helpers. `model_reset_upsert`
// and `link_model_reset_toast` are NOT feature-gated: the
// `model_resets` table is created unconditionally (M19), and the
// reset path persists the row even when the `toast` feature is off
// (the HUD tail surfaces the next pending reset regardless). Only
// the Toast enqueue is gated (`enqueue_model_ready_toast`).
impl Journal {
    /// RFC 28 Â§H.4 â€” persist a `SpendLimitError` observation into
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

    /// RFC 28 Â§H.4 â€” enqueue the `kind='model_ready'` Toast for a
    /// fresh `SpendLimitError` observation. Returns the new Toast
    /// queue row id; caller links it back via `link_model_reset_toast`.
    /// Feature-gated behind `toast` (the `toast_queue` table exists
    /// regardless of the feature, but Â§H ships Toast-only).
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

    /// RFC 28 Â§H.4 â€” link a `model_resets` row to the Toast queue row
    /// enqueued for it, so the scheduler can correlate dismissals.
    pub fn link_model_reset_toast(&self, reset_id: i64, toast_id: i64) -> anyhow::Result<bool> {
        let conn = self.conn.lock();
        crate::journal::model_resets::link_toast_id(&conn, reset_id, toast_id)
    }

    /// RFC 28 Â§H.4 â€” mark a `model_resets` row's Toast as dismissed,
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
    /// to assert idempotency of `handle_spend_limit_error` â€” production
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
    // exists â€” no migration is needed for 2.4.
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
    /// `kind = "mf_ab"`). Phase 9 sub-fase 9.1 (M33) admits
    /// `kind = "laya"` for the System One backend. The
    /// `(prompt_hash, classifier_kind)` UNIQUE
    /// constraint means a replay is silently ignored by `INSERT OR
    /// IGNORE` â€” A/B replay on the same prompt is a no-op, not an
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

    /// Record one `ast_symbols` row (Phase 9 sub-fase 9.2, M34).
    /// Returns the row id â€” the existing id on replay thanks to the
    /// `(file, name, kind, line)` UNIQUE constraint plus
    /// `INSERT OR IGNORE` (RFC 02 Â§3.1.2 idempotent consumer).
    pub fn record_ast_symbol(
        &self,
        file: &str,
        name: &str,
        kind: &str,
        line: i64,
        lang: &str,
    ) -> anyhow::Result<String> {
        let conn = self.conn.lock();
        ast_symbols::upsert_ast_symbol(&conn, file, name, kind, line, lang)
    }

    /// Symbols extracted from one file, ordered by line (M34).
    pub fn ast_symbols_for_file(&self, file: &str) -> anyhow::Result<Vec<AstSymbolRow>> {
        let conn = self.conn.lock();
        ast_symbols::ast_symbols_for_file(&conn, file)
    }

    /// RFC 04 Â§6 sub-fase 2.4 â€” rolling-window means of
    /// `model_invocations.tokens_in`, `tokens_out`, and a blended
    /// input+output cost-per-1M figure for `cost_guard`'s
    /// `AggregationCostContext::from_journal`. Used by the orchestrator
    /// to estimate pre-aggregation spend based on what this model has
    /// historically consumed per request.
    ///
    /// When no telemetry exists for `model_id`, returns zeros â€” the
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
