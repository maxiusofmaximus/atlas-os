// OpenCode OS — Cost guard / aggregation policy trait (RFC 04 §6,
// sub-fase 2.1, G11).
//
// When the orchestrator routes to an aggregation mode (Council, MoA,
// MajorityVote — sub-fase 2.2), it must verify the *expected* cost of
// the aggregation will not exceed the profile's per-turn budget
// *before* dispatching the parallel sample. Aggregation multiplies the
// per-request cost by N (number of debaters / voters / layers), so a
// naive "split budget equally" approach silently burns the operator's
// wallet when a profile has Council enabled for a HighStakes mission.
//
// G11 ("cost guard pre-aggregation") shapes this concern as a trait:
//
// - `pre_cost_estimate(ctx) -> f64` returns the estimated USD spend
//   for the aggregation, including every parallel sample, layer, and
//   round. The orchestrator compares this to `Profile.budget_per_turn`
//   and refuses the aggregation when it would overflow, falling back
//   to single-model invocation.
// - `aggregate_cost_breakdown(ctx) -> AggregationCostBreakdown` returns
//   the per-component breakdown so the HUD can surface it on the
//   cost-guard card (RFC 19 §3.4 "Aggregation rejected: $X > budget $Y").
//
// The trait is **shaped here** in sub-fase 2.1 so `RoutingConfig` can
// reference `Option<Arc<dyn AggregationPolicy>>` without pulling the
// implementation in; the six aggregation strategies from research/29
// (lines 226-237) implement it in sub-fase 2.2 — see
// `orchestrator/aggregation/*.rs`.

use serde::{Deserialize, Serialize};

/// Context handed to `AggregationPolicy::pre_cost_estimate`. The
/// orchestrator fills this from the routing decision and the journal's
/// `model_invocations` rolling-window rates.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AggregationCostContext {
    /// Number of parallel samples the aggregation will dispatch.
    pub parallel_samples: u32,
    /// Estimated tokens per sample (prompt + expected output).
    pub tokens_per_sample: u64,
    /// Cost per 1M tokens (input + output blended) of the chosen model,
    /// in USD. Filled from `ModelDescriptor::input_cost_per_1m_tokens`
    /// + `ModelDescriptor::output_cost_per_1m_tokens`.
    pub blended_cost_per_1m: f64,
    /// Number of rounds (Council, Reflexion). 1 for stateless modes
    /// (MoA, MajorityVote).
    pub rounds: u32,
    /// Reflexion memory buffer size — placeholder used by sub-fase 2.2
    /// to estimate the reflexor model's extra tokens. 0 in modes that
    /// don't use it.
    pub reflexion_memory_tokens: u64,
}

/// Breakdown of the pre-cost estimate, surfaced to the HUD when the
/// guard rejects an aggregation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AggregationCostBreakdown {
    /// Total estimated USD spend for the aggregation.
    pub total_usd: f64,
    /// Per-sample cost (parallel_samples × tokens_per_sample ×
    /// blended_cost_per_1m / 1_000_000).
    pub per_sample_usd: f64,
    /// Cross-round coordinator cost (Council, Reflexion reflexor) —
    /// separate so the HUD can show it next to the per-sample line.
    pub coordinator_usd: f64,
    /// Number of samples that would actually be dispatched (after
    /// stop-early estimation in MajorityVote).
    pub estimated_samples: u32,
}

/// Trait shaped in 2.1, implemented in 2.2 by the six aggregation
/// strategies. Methods return `f64`/structs only — no `async` so the
/// pure estimate can be unit-tested without a runtime.
pub trait AggregationPolicy: Send + Sync {
    /// Return the estimated total USD spend for the aggregation given
    /// `ctx`. Implementations should be deterministic and side-effect-
    /// free; the orchestrator compares the result to
    /// `Profile.budget_per_turn` and refuses the aggregation on
    /// overflow.
    fn pre_cost_estimate(&self, ctx: &AggregationCostContext) -> f64;

    /// Return the breakdown for HUD display. Default impl derives
    /// from `pre_cost_estimate` — strategies with richer breakdowns
    /// (Council, Reflexion) override.
    fn aggregate_cost_breakdown(&self, ctx: &AggregationCostContext) -> AggregationCostBreakdown {
        let total = self.pre_cost_estimate(ctx);
        let per_sample = if ctx.parallel_samples > 0 {
            total / ctx.parallel_samples as f64
        } else {
            0.0
        };
        AggregationCostBreakdown {
            total_usd: total,
            per_sample_usd: per_sample,
            coordinator_usd: 0.0,
            estimated_samples: ctx.parallel_samples,
        }
    }
}

/// No-op policy used by profiles without aggregation — `pre_cost_estimate`
/// always returns 0.0 so the guard never rejects. The orchestrator
/// uses this when `Profile.aggregation` is `None`.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoAggregation;

impl AggregationPolicy for NoAggregation {
    fn pre_cost_estimate(&self, _ctx: &AggregationCostContext) -> f64 {
        0.0
    }
}

/// Linear cost model: `parallel_samples × rounds × tokens_per_sample ×
/// blended_cost_per_1m / 1_000_000`. Used as the default cost guard
/// for stateless aggregations (MoA, MajorityVote).
#[derive(Debug, Default, Clone, Copy)]
pub struct LinearCostGuard;

impl AggregationPolicy for LinearCostGuard {
    fn pre_cost_estimate(&self, ctx: &AggregationCostContext) -> f64 {
        let base = ctx.parallel_samples as f64
            * ctx.tokens_per_sample as f64
            * (ctx.blended_cost_per_1m / 1_000_000.0);
        base * ctx.rounds.max(1) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_aggregation_returns_zero() {
        let p = NoAggregation;
        let ctx = AggregationCostContext {
            parallel_samples: 5,
            tokens_per_sample: 1_000,
            blended_cost_per_1m: 10.0,
            rounds: 3,
            reflexion_memory_tokens: 0,
        };
        assert_eq!(p.pre_cost_estimate(&ctx), 0.0);
    }

    #[test]
    fn linear_cost_guard_baseline() {
        let p = LinearCostGuard;
        let ctx = AggregationCostContext {
            parallel_samples: 3,
            tokens_per_sample: 1_000_000,
            blended_cost_per_1m: 10.0,
            rounds: 1,
            reflexion_memory_tokens: 0,
        };
        // 3 × 1_000_000 × 10/1M = 30
        assert!((p.pre_cost_estimate(&ctx) - 30.0).abs() < 1e-9);
    }

    #[test]
    fn linear_cost_guard_scales_with_rounds() {
        let p = LinearCostGuard;
        let ctx1 = AggregationCostContext {
            parallel_samples: 3,
            tokens_per_sample: 1_000_000,
            blended_cost_per_1m: 10.0,
            rounds: 1,
            reflexion_memory_tokens: 0,
        };
        let ctx3 = AggregationCostContext {
            rounds: 3,
            ..ctx1.clone()
        };
        let r1 = p.pre_cost_estimate(&ctx1);
        let r3 = p.pre_cost_estimate(&ctx3);
        assert!((r3 - r1 * 3.0).abs() < 1e-9);
    }

    #[test]
    fn default_breakdown_splits_per_sample() {
        let p = LinearCostGuard;
        let ctx = AggregationCostContext {
            parallel_samples: 4,
            tokens_per_sample: 500_000,
            blended_cost_per_1m: 8.0,
            rounds: 1,
            reflexion_memory_tokens: 0,
        };
        // 4 × 500_000 × 8/1M = 16 USD total, 4 per sample.
        let bd = p.aggregate_cost_breakdown(&ctx);
        assert!((bd.total_usd - 16.0).abs() < 1e-9);
        assert!((bd.per_sample_usd - 4.0).abs() < 1e-9);
        assert_eq!(bd.estimated_samples, 4);
    }

    #[test]
    fn default_breakdown_handles_zero_samples() {
        let p = LinearCostGuard;
        let ctx = AggregationCostContext {
            parallel_samples: 0,
            ..Default::default()
        };
        let bd = p.aggregate_cost_breakdown(&ctx);
        assert_eq!(bd.total_usd, 0.0);
        assert_eq!(bd.per_sample_usd, 0.0);
        assert_eq!(bd.estimated_samples, 0);
    }

    #[test]
    fn linear_cost_guard_with_zero_tokens_is_zero() {
        let p = LinearCostGuard;
        let ctx = AggregationCostContext {
            parallel_samples: 5,
            tokens_per_sample: 0,
            blended_cost_per_1m: 100.0,
            rounds: 3,
            reflexion_memory_tokens: 0,
        };
        assert_eq!(p.pre_cost_estimate(&ctx), 0.0);
    }

    #[test]
    fn cost_context_roundtrips_through_serde() {
        let ctx = AggregationCostContext {
            parallel_samples: 5,
            tokens_per_sample: 10_000,
            blended_cost_per_1m: 3.5,
            rounds: 2,
            reflexion_memory_tokens: 1_000,
        };
        let json = serde_json::to_string(&ctx).unwrap();
        let back: AggregationCostContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx, back);
    }

    #[test]
    fn cost_breakdown_roundtrips_through_serde() {
        let bd = AggregationCostBreakdown {
            total_usd: 1.5,
            per_sample_usd: 0.5,
            coordinator_usd: 0.1,
            estimated_samples: 3,
        };
        let json = serde_json::to_string(&bd).unwrap();
        let back: AggregationCostBreakdown = serde_json::from_str(&json).unwrap();
        assert_eq!(bd, back);
    }
}
