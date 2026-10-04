// Atlas OS — model_invocations aggregate (RFC 04 §6 sub-fase 2.4).
// Struct + write/record/read path extracted from journal/mod.rs in Phase 20.2.

use super::Journal;

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

impl Journal {
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
        // `model_invocations.model_id` has an FK to `models(id)`. A runtime
        // endpoint (e.g. a NIM/Groq model) is not necessarily in the curated
        // seed, so ensure a minimal row exists before recording telemetry.
        conn.execute(
            "INSERT OR IGNORE INTO models
             (id, provider, display_name, tier, context_window, max_output_tokens, capabilities_json)
             VALUES (?1, ?2, ?1, 'strong', 128000, 16384, '[]')",
            rusqlite::params![row.model_id, row.provider],
        )?;
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
