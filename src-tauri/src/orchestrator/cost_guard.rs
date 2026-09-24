// Atlas OS — Cost guard / aggregation policy trait (RFC 04 §6,
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

impl AggregationCostContext {
    /// RFC 04 §6 sub-fase 2.4 — fill `tokens_per_sample` and
    /// `blended_cost_per_1m` from the journal's rolling-window
    /// `model_invocations` means for `(model_id, sample_window)`.
    /// This is the "real cost_guard impl" (research/29 line 252).
    ///
    /// Used by the orchestrator loop before dispatching an aggregation:
    /// if the historical mean tokens for this model exceed the budget
    /// allocation, the guard rejects and the orchestrator degrades to
    /// single-model invocation. The default `Default::default()`
    /// still works for unit tests that don't want to seed telemetry.
    ///
    /// Returns a fully-formed `AggregationCostContext` ready for
    /// `AggregationPolicy::pre_cost_estimate`. Caller controls the
    /// `parallel_samples`, `rounds`, and `reflexion_memory_tokens`
    /// fields via the parameter list — those don't come from the
    /// journal.
    pub fn from_journal(
        journal: &crate::journal::Journal,
        model_id: &str,
        window: u32,
        parallel_samples: u32,
        rounds: u32,
        reflexion_memory_tokens: u64,
    ) -> anyhow::Result<Self> {
        let (mean_tokens_in, mean_tokens_out, blended_cost_per_1m) =
            journal.read_model_invocation_means(model_id, window)?;
        let tokens_per_sample = mean_tokens_in.saturating_add(mean_tokens_out);
        Ok(Self {
            parallel_samples,
            tokens_per_sample,
            blended_cost_per_1m,
            rounds,
            reflexion_memory_tokens,
        })
    }
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

/// Cost guard for `MajorityVote` (RFC 04 §3, sub-fase 2.2). Linear
/// cost model plus a stop-early discount — when the profile sets
/// `stop_early_threshold`, the effective sample count is `min(
/// parallel_samples, estimated_samples)`. By default we assume the
/// full `parallel_samples` will be dispatched; the orchestrator
/// updates `estimated_samples` post-hoc.
#[derive(Debug, Default, Clone, Copy)]
pub struct MajorityVoteCostGuard;

impl AggregationPolicy for MajorityVoteCostGuard {
    fn pre_cost_estimate(&self, ctx: &AggregationCostContext) -> f64 {
        let effective = ctx.parallel_samples.max(1) as f64;
        effective * ctx.tokens_per_sample as f64 * (ctx.blended_cost_per_1m / 1_000_000.0)
    }
    fn aggregate_cost_breakdown(&self, ctx: &AggregationCostContext) -> AggregationCostBreakdown {
        let total = self.pre_cost_estimate(ctx);
        let per = if ctx.parallel_samples > 0 {
            total / ctx.parallel_samples as f64
        } else {
            0.0
        };
        AggregationCostBreakdown {
            total_usd: total,
            per_sample_usd: per,
            coordinator_usd: 0.0,
            estimated_samples: ctx.parallel_samples,
        }
    }
}

/// Cost guard for `MoA` (RFC 04 §3). MoA's layered architecture
/// means each layer-N aggregator consumes every layer-(N-1) output
/// as auxiliary input — the cost grows quadratically with the number
/// of layers, not linearly with `parallel_samples` alone. We model
/// this as `sum(layer_sizes) × tokens_per_sample × blended_cost`
/// with a `layers` multiplier exposed via `rounds` (the orchestrator
/// sets `rounds = layers.len()`).
#[derive(Debug, Default, Clone, Copy)]
pub struct MoACostGuard;

impl AggregationPolicy for MoACostGuard {
    fn pre_cost_estimate(&self, ctx: &AggregationCostContext) -> f64 {
        let layers = ctx.rounds.max(1) as f64;
        ctx.parallel_samples as f64
            * layers
            * ctx.tokens_per_sample as f64
            * (ctx.blended_cost_per_1m / 1_000_000.0)
    }
}

/// Cost guard for `Council` (RFC 04 §3). Debaters run every round
/// (`parallel_samples × rounds`), plus a coordinator model synthesises
/// the final fused response. The coordinator uses the same blended
/// cost but operates on the concatenated claim text — modelled here
/// as a small coordinator surcharge proportional to `parallel_samples
/// × rounds` (the number of claims it must fuse).
#[derive(Debug, Default, Clone, Copy)]
pub struct CouncilCostGuard;

impl AggregationPolicy for CouncilCostGuard {
    fn pre_cost_estimate(&self, ctx: &AggregationCostContext) -> f64 {
        let debater_cost = ctx.parallel_samples as f64
            * ctx.rounds.max(1) as f64
            * ctx.tokens_per_sample as f64
            * (ctx.blended_cost_per_1m / 1_000_000.0);
        let coordinator_cost = (ctx.parallel_samples as f64 * ctx.rounds.max(1) as f64)
            * 200.0
            * (ctx.blended_cost_per_1m / 1_000_000.0);
        debater_cost + coordinator_cost
    }
    fn aggregate_cost_breakdown(&self, ctx: &AggregationCostContext) -> AggregationCostBreakdown {
        let debater_cost = ctx.parallel_samples as f64
            * ctx.rounds.max(1) as f64
            * ctx.tokens_per_sample as f64
            * (ctx.blended_cost_per_1m / 1_000_000.0);
        let coordinator_cost = (ctx.parallel_samples as f64 * ctx.rounds.max(1) as f64)
            * 200.0
            * (ctx.blended_cost_per_1m / 1_000_000.0);
        AggregationCostBreakdown {
            total_usd: debater_cost + coordinator_cost,
            per_sample_usd: if ctx.parallel_samples > 0 {
                debater_cost / ctx.parallel_samples as f64
            } else {
                0.0
            },
            coordinator_usd: coordinator_cost,
            estimated_samples: ctx.parallel_samples,
        }
    }
}

/// Cost guard for `Reflexion` (RFC 04 §3). Executor cost (caro)
/// scales with attempts; reflexor cost (barato) scales with
/// attempts × memory buffer tokens. The orchestrator fills
/// `tokens_per_sample` with the executor's blended cost and
/// `reflexion_memory_tokens` with the reflexor's per-attempt
/// memory cost. We approximate reflexor-blended cost as 1/4 the
/// executor's (a common ratio in deployed profiles); the live
/// orchestrator can override.
#[derive(Debug, Default, Clone, Copy)]
pub struct ReflexionCostGuard;

impl AggregationPolicy for ReflexionCostGuard {
    fn pre_cost_estimate(&self, ctx: &AggregationCostContext) -> f64 {
        let attempts = ctx.rounds.max(1) as f64;
        let executor_cost =
            attempts * ctx.tokens_per_sample as f64 * (ctx.blended_cost_per_1m / 1_000_000.0);
        let reflexor_rate = ctx.blended_cost_per_1m / 4.0;
        let reflexor_cost =
            attempts * ctx.reflexion_memory_tokens as f64 * (reflexor_rate / 1_000_000.0);
        executor_cost + reflexor_cost
    }
    fn aggregate_cost_breakdown(&self, ctx: &AggregationCostContext) -> AggregationCostBreakdown {
        let attempts = ctx.rounds.max(1) as f64;
        let executor_cost =
            attempts * ctx.tokens_per_sample as f64 * (ctx.blended_cost_per_1m / 1_000_000.0);
        let reflexor_rate = ctx.blended_cost_per_1m / 4.0;
        let reflexor_cost =
            attempts * ctx.reflexion_memory_tokens as f64 * (reflexor_rate / 1_000_000.0);
        AggregationCostBreakdown {
            total_usd: executor_cost + reflexor_cost,
            per_sample_usd: if ctx.parallel_samples > 0 {
                executor_cost / ctx.parallel_samples as f64
            } else {
                executor_cost
            },
            coordinator_usd: reflexor_cost,
            estimated_samples: ctx.parallel_samples.max(1),
        }
    }
}

/// Cost guard for `SelfRefine` (RFC 04 §3). The same LLM plays
/// generator, feedback, and refiner — so the cost is
/// `max_iterations × tokens_per_sample × blended_cost`. No
/// separate coordinator; the model refines in-place.
#[derive(Debug, Default, Clone, Copy)]
pub struct SelfRefineCostGuard;

impl AggregationPolicy for SelfRefineCostGuard {
    fn pre_cost_estimate(&self, ctx: &AggregationCostContext) -> f64 {
        let iters = ctx.rounds.max(1) as f64;
        iters * ctx.tokens_per_sample as f64 * (ctx.blended_cost_per_1m / 1_000_000.0)
    }
}

/// Cost guard for `SelfDiscover` (RFC 04 §3). One planning call
/// per cache miss (charged once per TTL window), then a single
/// executor call. `parallel_samples` is 1 here; `rounds` is 1
/// except on cache miss (rounds=2 — plan + execute). The live
/// orchestrator estimates cache hit-rate from rolling stats; the
/// default assumes worst-case (always miss).
#[derive(Debug, Default, Clone, Copy)]
pub struct SelfDiscoverCostGuard;

impl AggregationPolicy for SelfDiscoverCostGuard {
    fn pre_cost_estimate(&self, ctx: &AggregationCostContext) -> f64 {
        let calls = ctx.rounds.max(1) as f64;
        calls * ctx.tokens_per_sample as f64 * (ctx.blended_cost_per_1m / 1_000_000.0)
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

    fn ctx_full() -> AggregationCostContext {
        AggregationCostContext {
            parallel_samples: 3,
            tokens_per_sample: 1_000_000,
            blended_cost_per_1m: 10.0,
            rounds: 2,
            reflexion_memory_tokens: 50_000,
        }
    }

    #[test]
    fn majority_vote_cost_guard_matches_linear_for_single_round() {
        let p = MajorityVoteCostGuard;
        let ctx = AggregationCostContext {
            rounds: 1,
            ..ctx_full()
        };
        // 3 × 1M × 10/1M = 30 USD.
        assert!((p.pre_cost_estimate(&ctx) - 30.0).abs() < 1e-9);
        let bd = p.aggregate_cost_breakdown(&ctx);
        assert_eq!(bd.estimated_samples, 3);
        assert!((bd.per_sample_usd - 10.0).abs() < 1e-9);
        assert_eq!(bd.coordinator_usd, 0.0);
    }

    #[test]
    fn moa_cost_guard_scales_with_layer_count() {
        let p = MoACostGuard;
        let ctx = AggregationCostContext {
            rounds: 3,
            ..ctx_full()
        };
        // 3 × 3 × 1M × 10/1M = 90 USD.
        assert!((p.pre_cost_estimate(&ctx) - 90.0).abs() < 1e-9);
    }

    #[test]
    fn council_cost_guard_charges_coordinator() {
        let p = CouncilCostGuard;
        let ctx = AggregationCostContext {
            parallel_samples: 3,
            tokens_per_sample: 1_000_000,
            blended_cost_per_1m: 10.0,
            rounds: 2,
            reflexion_memory_tokens: 0,
        };
        let est = p.pre_cost_estimate(&ctx);
        let bd = p.aggregate_cost_breakdown(&ctx);
        assert!(bd.coordinator_usd > 0.0);
        // debater_cost = 3 × 2 × 1M × 10/1M = 60 USD; coordinator_cost
        // = 6 × 200 × 10/1M = 0.012 USD. Total ≈ 60.012.
        assert!((est - bd.total_usd).abs() < 1e-12);
        assert!((bd.total_usd - 60.012).abs() < 1e-3);
    }

    #[test]
    fn reflexion_cost_guard_charges_reflexor_memory() {
        let p = ReflexionCostGuard;
        let ctx = AggregationCostContext {
            parallel_samples: 1,
            tokens_per_sample: 1_000_000,
            blended_cost_per_1m: 10.0,
            rounds: 2,
            reflexion_memory_tokens: 50_000,
        };
        // executor_cost = 2 × 1M × 10/1M = 20 USD.
        // reflexor_cost = 2 × 50_000 × (10/4)/1M = 0.25 USD.
        // Total = 20.25.
        assert!((p.pre_cost_estimate(&ctx) - 20.25).abs() < 1e-9);
        let bd = p.aggregate_cost_breakdown(&ctx);
        assert!((bd.coordinator_usd - 0.25).abs() < 1e-9);
    }

    #[test]
    fn self_refine_cost_guard_scales_iterations() {
        let p = SelfRefineCostGuard;
        let ctx1 = AggregationCostContext {
            rounds: 1,
            ..ctx_full()
        };
        let ctx2 = AggregationCostContext {
            rounds: 2,
            ..ctx_full()
        };
        let r1 = p.pre_cost_estimate(&ctx1);
        let r2 = p.pre_cost_estimate(&ctx2);
        assert!((r2 - r1 * 2.0).abs() < 1e-9);
    }

    #[test]
    fn self_discover_cost_guard_charges_planning_on_cache_miss() {
        let p = SelfDiscoverCostGuard;
        let ctx_miss = AggregationCostContext {
            rounds: 2,
            ..ctx_full()
        };
        let ctx_hit = AggregationCostContext {
            rounds: 1,
            ..ctx_full()
        };
        let miss = p.pre_cost_estimate(&ctx_miss);
        let hit = p.pre_cost_estimate(&ctx_hit);
        assert!(miss > hit);
        assert!((miss - 2.0 * hit).abs() < 1e-9);
    }
}
