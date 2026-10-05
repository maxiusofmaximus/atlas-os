// Atlas OS — model_invocations aggregate (RFC 04 §6 sub-fase 2.4).
// Struct + write/record/read path extracted from journal/mod.rs in Phase 20.2.

use serde::{Deserialize, Serialize};

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

/// RFC 65 §3 — per-model cost/token roll-up over a bounded window of the
/// most recent `model_invocations` rows. Powers the HUD `<CostDashboard>`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CostByModelRow {
    pub model_id: String,
    pub provider: String,
    pub invocations: i64,
    pub cost_usd: f64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub mean_latency_ms: f64,
}

/// RFC 65 §3 — window totals matching the sum of the per-model roll-up.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CostTotals {
    pub invocations: i64,
    pub cost_usd: f64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub mean_latency_ms: f64,
}

impl Journal {
    /// RFC 65 §3 — window totals over the most recent `window`
    /// `model_invocations` rows (newest first). Empty journal → all zeros.
    pub fn cost_totals(&self, window: i64) -> anyhow::Result<CostTotals> {
        let conn = self.conn.lock();
        let totals = conn.query_row(
            "WITH ranked AS (
                SELECT cost_usd, tokens_in, tokens_out, latency_ms,
                       ROW_NUMBER() OVER (ORDER BY started_at DESC) AS rk
                FROM model_invocations
            )
            SELECT COUNT(*),
                   COALESCE(SUM(cost_usd), 0),
                   COALESCE(SUM(tokens_in), 0),
                   COALESCE(SUM(tokens_out), 0),
                   COALESCE(AVG(latency_ms), 0)
            FROM ranked
            WHERE rk <= ?1",
            rusqlite::params![window.max(0)],
            |r| {
                Ok(CostTotals {
                    invocations: r.get(0)?,
                    cost_usd: r.get(1)?,
                    tokens_in: r.get(2)?,
                    tokens_out: r.get(3)?,
                    mean_latency_ms: r.get(4)?,
                })
            },
        )?;
        Ok(totals)
    }

    /// RFC 65 §3 — per-model roll-up over the most recent `window`
    /// `model_invocations` rows, costliest first.
    pub fn cost_by_model(&self, window: i64) -> anyhow::Result<Vec<CostByModelRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "WITH ranked AS (
                SELECT model_id, provider, cost_usd, tokens_in, tokens_out, latency_ms,
                       ROW_NUMBER() OVER (ORDER BY started_at DESC) AS rk
                FROM model_invocations
            )
            SELECT model_id,
                   provider,
                   COUNT(*),
                   COALESCE(SUM(cost_usd), 0),
                   COALESCE(SUM(tokens_in), 0),
                   COALESCE(SUM(tokens_out), 0),
                   COALESCE(AVG(latency_ms), 0)
            FROM ranked
            WHERE rk <= ?1
            GROUP BY model_id, provider
            ORDER BY SUM(cost_usd) DESC, COUNT(*) DESC, model_id ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![window.max(0)], |r| {
            Ok(CostByModelRow {
                model_id: r.get(0)?,
                provider: r.get(1)?,
                invocations: r.get(2)?,
                cost_usd: r.get(3)?,
                tokens_in: r.get(4)?,
                tokens_out: r.get(5)?,
                mean_latency_ms: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn invocation(
        id: &str,
        model: &str,
        provider: &str,
        ts: &str,
        cost: f64,
    ) -> ModelInvocationRow {
        ModelInvocationRow {
            id: id.into(),
            mission_id: None,
            model_id: model.into(),
            deployment_id: "dep".into(),
            provider: provider.into(),
            idempotency_key: format!("idem-{id}"),
            started_at: ts.into(),
            finished_at: Some(ts.into()),
            latency_ms: Some(100),
            tokens_in: Some(10),
            tokens_out: Some(5),
            cache_read_input_tokens: None,
            cost_usd: Some(cost),
            seed: None,
            temperature: None,
            sampling_params_json: None,
            route_taken_json: None,
            was_correct: None,
            error_kind: None,
            error_message: None,
        }
    }

    #[test]
    fn cost_totals_is_zero_on_empty_journal() {
        let dir = TempDir::new().unwrap();
        let j = Journal::open(dir.path()).unwrap();
        let totals = j.cost_totals(200).unwrap();
        assert_eq!(totals, CostTotals::default());
    }

    #[test]
    fn cost_rollup_groups_by_model_and_respects_window() {
        let dir = TempDir::new().unwrap();
        let j = Journal::open(dir.path()).unwrap();
        j.record_model_invocation(&invocation("a", "m1", "p1", "2026-01-01T00:00:01Z", 0.10))
            .unwrap();
        j.record_model_invocation(&invocation("b", "m1", "p1", "2026-01-01T00:00:03Z", 0.20))
            .unwrap();
        j.record_model_invocation(&invocation("c", "m2", "p2", "2026-01-01T00:00:02Z", 0.05))
            .unwrap();

        let totals = j.cost_totals(200).unwrap();
        assert_eq!(totals.invocations, 3);
        assert![(totals.cost_usd - 0.35).abs() < 1e-9];
        assert_eq!(totals.tokens_in, 30);
        assert_eq!(totals.tokens_out, 15);

        let by_model = j.cost_by_model(200).unwrap();
        assert_eq!(by_model.len(), 2);
        assert_eq!(by_model[0].model_id, "m1");
        assert_eq!(by_model[0].invocations, 2);
        assert![(by_model[0].cost_usd - 0.30).abs() < 1e-9];
        assert_eq!(by_model[1].model_id, "m2");

        // Window of the 2 newest rows drops the oldest (m1 @ :01, $0.10).
        let windowed = j.cost_totals(2).unwrap();
        assert_eq!(windowed.invocations, 2);
        assert![(windowed.cost_usd - 0.25).abs() < 1e-9];
    }
}
