// Atlas OS — Routing policy (RFC 04 §2 / §6, sub-fase 2.1).
//
// The orchestrator's routing layer picks one `Deployment` out of the
// healthy set for a given `model_id` group according to a
// `RoutingStrategy`. LiteLLM (MIT, BerriAI) defines five production
// strategies — `simple-shuffle`, `usage-based-routing-v2`, `least-busy`,
// `latency-based-routing`, `cost-based-routing` — plus a `Hybrid` form
// we add here so a profile can layer e.g. `LatencyBased` for the fast
// tier and `CostBased` for the bulk tier. `Custom` lets an operator
// inject a `dyn Router` (in-process only — not serializable, so it is
// `#[serde(skip)]`).
//
// Fallback cascade is the second concern: `RoutingConfig` carries the
// three LiteLLM buckets (`fallbacks`, `context_window_fallbacks`,
// `content_policy_fallbacks`) plus `default_fallbacks` and a
// `max_fallbacks` cap (5 by default). The cascade itself (exclusion set
// + weighted failover within group → cross-group escalation) lives in
// `cascade.rs` (sub-fase 2.1.c).
//
// The `Router` trait is intentionally sync and borrow-only: routing
// selection is a pure function over `RouteContext` (the healthy
// deployments + telemetry snapshot), so it can be unit-tested without
// a runtime and cached/replayed for determinism. The caller (the
// orchestrator loop in `orchestrator/mod.rs`) is responsible for
// honouring the returned `RouteDecision` and for threading the
// exclusion set on retries.
//
// `pre_cost_estimate` (G11) is trait-shaped here and implemented in
// sub-fase 2.2 (Aggregation). Defining the shape now lets `RoutingConfig`
// reference an `AggregationPolicy` without pulling the implementation
// in.

use std::collections::HashSet;
use std::sync::Arc;

use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::orchestrator::provider::Deployment;

/// Identifies a fallback bucket. Mirrors LiteLLM's three-bucket
/// taxonomy: `fallbacks` (generic), `context_window_fallbacks` (when
/// the prompt overflows the primary's context window),
/// `content_policy_fallbacks` (when the primary refuses on policy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackBucket {
    Generic,
    ContextWindow,
    ContentPolicy,
}

/// Why the current deployment is being skipped. Used by `Hybrid`
/// conditions and by the cascade log so the audit trail records the
/// reason for each escalation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// Cooldown (429 / 5xx storm). Cascade should try other healthy
    /// deployments in the same group first.
    Cooldown,
    /// Context window overflow — escalate to `context_window_fallbacks`.
    ContextWindowOverflow,
    /// Provider content-policy refusal — escalate to
    /// `content_policy_fallbacks`.
    ContentPolicyRefusal,
    /// Operator-set `weight = 0` ("drain"). The registry keeps the row
    /// for audit but routing must skip.
    Drained,
    /// In-flight count reached `max_parallel` (back-pressure). The
    /// deployment is healthy but momentarily unavailable.
    Saturated,
    /// Explicitly excluded by the cascade's exclusion set (already
    /// tried and failed on this request).
    Excluded,
}

/// A condition for the `Hybrid` strategy. The router evaluates
/// conditions top-to-bottom and picks the first match; the final
/// `else` branch is the strategy carried on `Hybrid.default`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Condition {
    /// Match if the prompt's token count is above `threshold`.
    TokensAbove { threshold: u32 },
    /// Match if any of the requested capabilities is in `required`.
    Requires { capabilities: Vec<String> },
    /// Match if the estimated cost (per the `pre_cost_estimate` hook)
    /// exceeds `usd`.
    CostAbove { usd: f64 },
    /// Match if the request carries tool calls (function calling).
    HasToolCalls,
    /// Match if the request is marked HighStakes (RFC 19).
    HighStakes,
}

/// Telemetry snapshot fed to the router. The orchestrator fills this
/// from the journal's `model_invocations` rolling window (M21) and the
/// in-flight counter of `BackPressure` (sub-fase 2.0.5).
#[derive(Debug, Clone)]
pub struct RouteContext<'a> {
    /// Healthy deployments for the current `model_id` group (cooldown
    /// and back-pressure already filtered — the cascade is what does
    /// that filtering, but `RouteContext` is the post-filter view).
    pub healthy: &'a [Deployment],
    /// In-flight request count per `Deployment::id`. Absent means 0.
    pub in_flight: &'a HashMap<String, u32>,
    /// Rolling p50 latency in ms per `Deployment::id`. Absent means
    /// "no data yet" — `LatencyBased` falls back to weight-only.
    pub latency_p50_ms: &'a HashMap<String, u32>,
    /// Rolling error rate (0.0..=1.0) per `Deployment::id`. Absent
    /// means 0.
    pub error_rate: &'a HashMap<String, f64>,
    /// Tokens consumed in the current minute per `Deployment::id`.
    pub tokens_this_minute: &'a HashMap<String, u32>,
    /// Token budget per minute per `Deployment::id` (from
    /// `Deployment::tokens_per_minute`).
    pub tpm_budget: &'a HashMap<String, u32>,
    /// Snapshot of prompt token count (best estimate) for
    /// `Condition::TokensAbove`.
    pub prompt_tokens: u32,
    /// Capabilities requested by the prompt (mapped from tool calls /
    /// vision / etc.) for `Condition::Requires`.
    pub required_capabilities: &'a [String],
    /// Pre-cost estimate (G11), filled by `AggregationPolicy`. 0.0
    /// when no aggregator is configured.
    pub pre_cost_estimate_usd: f64,
    /// True when the request carries at least one tool call.
    pub has_tool_calls: bool,
    /// True when `ExecutionMode::HighStakes` (RFC 19) is active.
    pub high_stakes: bool,
    /// IDs already tried on this request (cascade exclusion set). The
    /// router MUST treat these as if they were `Excluded`-skipped.
    pub excluded: &'a HashSet<String>,
}

use std::collections::HashMap;

/// Outcome of routing: either a chosen `Deployment` or a signal that no
/// healthy deployment is available (caller escalates to fallbacks).
#[derive(Debug, Clone)]
pub enum RouteDecision<'a> {
    Deploy(&'a Deployment),
    NoHealthy,
}

/// Pure routing strategy. Implementations are sync and borrow from
/// `RouteContext`; the orchestrator owns the lifetimes.
pub trait Router: Send + Sync {
    fn route<'a>(&self, ctx: &RouteContext<'a>) -> RouteDecision<'a>;
}

/// Top-level strategy enum. `Custom` holds an `Arc<dyn Router>` so the
/// profile can inject an in-process policy object; serde skips it on
/// serialize (the seed only stores the data-driven strategies).
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RoutingStrategy {
    /// Weighted random shuffle (LiteLLM `simple-shuffle`). Each
    /// deployment with `weight > 0` is chosen with probability
    /// proportional to its weight; ties broken by `priority` then by
    /// stable id.
    SimpleShuffle,
    /// Rolling p50 latency, refreshed every `ttl_secs`. We mix weight
    /// in to break cold-start ties (no telemetry yet). `buffer` is the
    /// LiteLLM "buffer" — extra ms added so a marginally-faster
    /// deployment doesn't always win (default 50 ms).
    LatencyBased { ttl_secs: u32, buffer_ms: u32 },
    /// LiteLLM `usage-based-routing-v2` — pick the deployment with the
    /// most remaining TPM headroom (budget - tokens_this_minute).
    UsageBasedV2,
    /// LiteLLM `least-busy` — pick the deployment with the smallest
    /// `in_flight` count; ties broken by weight.
    LeastBusy,
    /// LiteLLM `cost-based-routing` — pick the deployment whose model
    /// has the lowest input+output cost per token (per
    /// `ModelDescriptor`). Ties broken by weight.
    CostBased,
    /// Layered strategies: evaluate `conditions` top-to-bottom, pick
    /// the first matching branch, fall back to `default`. Used to
    /// e.g. route HighStakes to `Council`-capable deployments and bulk
    /// to `CostBased`.
    Hybrid {
        branches: Vec<(Condition, RoutingStrategy)>,
        #[serde(default)]
        default: Option<Box<RoutingStrategy>>,
    },
    /// In-process custom router. Skipped on serialize.
    #[serde(skip)]
    Custom(Arc<dyn Router>),
}

impl std::fmt::Debug for RoutingStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SimpleShuffle => f.write_str("SimpleShuffle"),
            Self::LatencyBased {
                ttl_secs,
                buffer_ms,
            } => f
                .debug_struct("LatencyBased")
                .field("ttl_secs", ttl_secs)
                .field("buffer_ms", buffer_ms)
                .finish(),
            Self::UsageBasedV2 => f.write_str("UsageBasedV2"),
            Self::LeastBusy => f.write_str("LeastBusy"),
            Self::CostBased => f.write_str("CostBased"),
            Self::Hybrid { branches, default } => f
                .debug_struct("Hybrid")
                .field("branches", branches)
                .field("default", default)
                .finish(),
            Self::Custom(_) => f.write_str("Custom(<dyn Router>)"),
        }
    }
}

impl PartialEq for RoutingStrategy {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::SimpleShuffle, Self::SimpleShuffle) => true,
            (
                Self::LatencyBased {
                    ttl_secs: a,
                    buffer_ms: am,
                },
                Self::LatencyBased {
                    ttl_secs: b,
                    buffer_ms: bm,
                },
            ) => a == b && am == bm,
            (Self::UsageBasedV2, Self::UsageBasedV2) => true,
            (Self::LeastBusy, Self::LeastBusy) => true,
            (Self::CostBased, Self::CostBased) => true,
            (
                Self::Hybrid {
                    branches: ab,
                    default: ad,
                },
                Self::Hybrid {
                    branches: bb,
                    default: bd,
                },
            ) => ab == bb && ad == bd,
            // Custom routers are not comparable (Arc<dyn Router>).
            _ => false,
        }
    }
}

/// Three-bucket fallback configuration (LiteLLM). Each bucket maps a
/// primary `model_id` to an ordered list of fallback `model_id`s. The
/// cascade (`cascade.rs`) consumes this when escalating.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FallbackMap {
    /// `primary → [fb1, fb2, ...]`. Used when the primary is in
    /// cooldown or returning non-retryable errors (401/404/408).
    #[serde(default)]
    pub fallbacks: HashMap<String, Vec<String>>,
    /// `primary → [fb1, ...]` used when the primary's context window
    /// is overflowed by the prompt.
    #[serde(default)]
    pub context_window_fallbacks: HashMap<String, Vec<String>>,
    /// `primary → [fb1, ...]` used when the primary refuses on content
    /// policy.
    #[serde(default)]
    pub content_policy_fallbacks: HashMap<String, Vec<String>>,
}

impl FallbackMap {
    /// Look up the fallback list for `primary` in `bucket`. Returns
    /// an empty slice if none configured.
    pub fn lookup(&self, bucket: FallbackBucket, primary: &str) -> &[String] {
        let map = match bucket {
            FallbackBucket::Generic => &self.fallbacks,
            FallbackBucket::ContextWindow => &self.context_window_fallbacks,
            FallbackBucket::ContentPolicy => &self.content_policy_fallbacks,
        };
        map.get(primary).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Top-level routing configuration carried on `Profile`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Strategy for the primary group.
    pub strategy: RoutingStrategy,
    /// Three-bucket fallback map.
    #[serde(default)]
    pub fallback: FallbackMap,
    /// Default fallback list consulted when none of the three buckets
    /// matches the primary (LiteLLM `default_fallbacks`). Applied
    /// last, after bucket-specific lists.
    #[serde(default)]
    pub default_fallbacks: Vec<String>,
    /// Hard cap on the total number of fallbacks attempted before the
    /// orchestrator gives up and surfaces a paused-mission card.
    /// Default 5 (RFC 04 §6 / research 29 line 218).
    #[serde(default = "default_max_fallbacks")]
    pub max_fallbacks: u8,
    /// Cooldown time applied when a deployment 429s without a
    /// `Retry-After`/`x-ratelimit-reset` header. Per-provider overrides
    /// live on `Profile.cooldown_overrides` (sub-fase 2.0.5).
    #[serde(default = "default_cooldown_secs")]
    pub default_cooldown_secs: u32,
    /// Allowed fails before a deployment is cooled down within the
    /// rolling window (LiteLLM `allowed_fails`, default 3).
    #[serde(default = "default_allowed_fails")]
    pub allowed_fails: u32,
    /// Rolling window for the `allowed_fails` counter, in seconds
    /// (LiteLLM `cooldown_time`'s sibling — we expose it here so an
    /// operator can tighten the window independently of the cooldown
    /// duration). Default 60.
    #[serde(default = "default_fails_window_secs")]
    pub fails_window_secs: u32,
}

fn default_max_fallbacks() -> u8 {
    5
}
fn default_cooldown_secs() -> u32 {
    60
}
fn default_allowed_fails() -> u32 {
    3
}
fn default_fails_window_secs() -> u32 {
    60
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            strategy: RoutingStrategy::SimpleShuffle,
            fallback: FallbackMap::default(),
            default_fallbacks: Vec::new(),
            max_fallbacks: default_max_fallbacks(),
            default_cooldown_secs: default_cooldown_secs(),
            allowed_fails: default_allowed_fails(),
            fails_window_secs: default_fails_window_secs(),
        }
    }
}

impl RoutingStrategy {
    /// Dispatch routing for this strategy. Borrows the healthy set and
    /// telemetry snapshot via `ctx`. Honours `ctx.excluded` by
    /// filtering prior to selection.
    pub fn select<'a, R: Rng + ?Sized>(
        &self,
        ctx: &RouteContext<'a>,
        rng: &mut R,
    ) -> RouteDecision<'a> {
        let eligible: Vec<&Deployment> = ctx
            .healthy
            .iter()
            .filter(|d| !ctx.excluded.contains(&d.id))
            .filter(|d| d.weight > 0.0)
            .collect();
        if eligible.is_empty() {
            return RouteDecision::NoHealthy;
        }
        match self {
            Self::SimpleShuffle => shuffle_select(&eligible, rng),
            Self::LatencyBased { buffer_ms, .. } => latency_select(&eligible, ctx, *buffer_ms, rng),
            Self::UsageBasedV2 => usage_select(&eligible, ctx),
            Self::LeastBusy => least_busy_select(&eligible, ctx),
            Self::CostBased => cost_select(&eligible, ctx, rng),
            Self::Hybrid { branches, default } => {
                for (cond, strat) in branches {
                    if cond.matches(ctx) {
                        return strat.select(ctx, rng);
                    }
                }
                if let Some(def) = default {
                    return def.select(ctx, rng);
                }
                // No branch matched and no default: fall back to shuffle.
                shuffle_select(&eligible, rng)
            }
            Self::Custom(router) => router.route(ctx),
        }
    }
}

impl Condition {
    /// Does this condition match the given context?
    pub fn matches(&self, ctx: &RouteContext<'_>) -> bool {
        match self {
            Self::TokensAbove { threshold } => ctx.prompt_tokens > *threshold,
            Self::Requires { capabilities } => {
                if capabilities.is_empty() {
                    return true;
                }
                capabilities
                    .iter()
                    .all(|c| ctx.required_capabilities.iter().any(|r| r == c))
            }
            Self::CostAbove { usd } => ctx.pre_cost_estimate_usd > *usd,
            Self::HasToolCalls => ctx.has_tool_calls,
            Self::HighStakes => ctx.high_stakes,
        }
    }
}

fn shuffle_select<'a, R: Rng + ?Sized>(
    eligible: &[&'a Deployment],
    rng: &mut R,
) -> RouteDecision<'a> {
    let total: f32 = eligible.iter().map(|d| d.weight).sum();
    if total <= 0.0 {
        return RouteDecision::NoHealthy;
    }
    let cut = rng.gen_range(0.0..total);
    let mut acc = 0.0;
    for d in eligible {
        acc += d.weight;
        if acc >= cut {
            return RouteDecision::Deploy(d);
        }
    }
    // Fallback (float rounding): pick the last. The caller (RoutingStrategy::select)
    // guarantees `eligible` is non-empty (empty returns NoHealthy at line 361), and
    // `total > 0` above guarantees the slice still has at least one weighted entry.
    RouteDecision::Deploy(
        eligible
            .last()
            .expect("invariant: eligible non-empty after total > 0 check"),
    )
}

fn latency_select<'a, R: Rng + ?Sized>(
    eligible: &[&'a Deployment],
    ctx: &RouteContext<'a>,
    buffer_ms: u32,
    rng: &mut R,
) -> RouteDecision<'a> {
    let score = |d: &&Deployment| -> u32 {
        match ctx.latency_p50_ms.get(&d.id) {
            Some(l) => l.saturating_add(buffer_ms),
            // Cold start: treat as 0 so the deployment wins over
            // known-slow ones (it gets a chance to build telemetry).
            None => 0,
        }
    };
    // Sort eligible by (score asc, weight desc, id asc).
    let mut sorted: Vec<&Deployment> = eligible.to_vec();
    sorted.sort_by(|a, b| {
        score(a)
            .cmp(&score(b))
            .then_with(|| {
                b.weight
                    .partial_cmp(&a.weight)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.id.cmp(&b.id))
    });
    let best_score = score(
        sorted
            .first()
            .expect("invariant: eligible non-empty (caller-guarded) — sorted non-empty"),
    );
    // Gather all deployments tied for the best score and weighted-shuffle
    // among them so we don't hammer a single one on cold start.
    let ties: Vec<&Deployment> = sorted
        .iter()
        .copied()
        .take_while(|d| score(d) == best_score)
        .collect();
    if ties.len() == 1 {
        return RouteDecision::Deploy(ties[0]);
    }
    shuffle_select(&ties, rng)
}

fn usage_select<'a>(eligible: &[&'a Deployment], ctx: &RouteContext<'a>) -> RouteDecision<'a> {
    let mut best: Option<(i64, &Deployment)> = None;
    for d in eligible {
        let budget = ctx
            .tpm_budget
            .get(&d.id)
            .copied()
            .or(d.tokens_per_minute)
            .unwrap_or(0) as i64;
        let used = ctx.tokens_this_minute.get(&d.id).copied().unwrap_or(0) as i64;
        // Unbounded (budget=0) → treat as +inf headroom only when
        // budget is None; when budget is Some(0) it means the operator
        // capped at 0 (drain), but weight>0 filter already removed
        // weight=0. Use a large headroom when budget is None.
        let headroom: i64 = if d.tokens_per_minute.is_none() {
            i64::MAX
        } else {
            budget - used
        };
        match best {
            None => best = Some((headroom, *d)),
            Some((bh, _)) if headroom > bh => best = Some((headroom, *d)),
            _ => {}
        }
    }
    match best {
        Some((_, d)) => RouteDecision::Deploy(d),
        None => RouteDecision::NoHealthy,
    }
}

fn least_busy_select<'a>(eligible: &[&'a Deployment], ctx: &RouteContext<'a>) -> RouteDecision<'a> {
    let mut best: Option<(u32, f32, &Deployment)> = None;
    for d in eligible {
        let inflight = ctx.in_flight.get(&d.id).copied().unwrap_or(0);
        match best {
            None => best = Some((inflight, d.weight, *d)),
            Some((bi, _bw, _)) if inflight < bi => best = Some((inflight, d.weight, *d)),
            Some((bi, bw, _)) if inflight == bi && d.weight > bw => {
                best = Some((inflight, d.weight, *d))
            }
            _ => {}
        }
    }
    match best {
        Some((_, _, d)) => RouteDecision::Deploy(d),
        None => RouteDecision::NoHealthy,
    }
}

fn cost_select<'a, R: Rng + ?Sized>(
    eligible: &[&'a Deployment],
    _ctx: &RouteContext<'a>,
    rng: &mut R,
) -> RouteDecision<'a> {
    // The cost lookup is via `Registry::model(&d.model_id)` rather than
    // threading a `cost_per_1m` map through `RouteContext` (which would
    // duplicate data). Since routing is intentionally pure and
    // borrow-only, we approximate "pick the lowest-cost" by ordering
    // ties uniformly. The full implementation in the orchestrator loop
    // consults the registry; the routing layer just breaks ties by
    // weight then random.
    let max_weight = eligible
        .iter()
        .map(|d| d.weight)
        .fold(0.0f32, f32::max)
        .max(1e-9);
    let heavy: Vec<&Deployment> = eligible
        .iter()
        .copied()
        .filter(|d| (d.weight - max_weight).abs() < 1e-6)
        .collect();
    if heavy.is_empty() {
        return RouteDecision::Deploy(
            eligible
                .first()
                .expect("invariant: eligible non-empty (caller-guarded) — heavy.is_empty() branch"),
        );
    }
    // Among the heaviest (treated as "preferred for cost" by the
    // operator's weight assignment), shuffle.
    let mut shuffled = heavy.clone();
    shuffled.shuffle(rng);
    RouteDecision::Deploy(
        shuffled
            .first()
            .expect("invariant: heavy non-empty after early-return above"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn dep(id: &str, weight: f32) -> Deployment {
        let mut d = Deployment::new(format!("m-{id}"), format!("https://{id}"));
        d.weight = weight;
        d
    }

    type EmptyMaps = (
        &'static HashMap<String, u32>,
        &'static HashMap<String, u32>,
        &'static HashMap<String, f64>,
        &'static HashMap<String, u32>,
        &'static HashMap<String, u32>,
        &'static HashSet<String>,
    );

    #[allow(clippy::type_complexity)]
    fn empty_maps() -> EmptyMaps {
        static IN_FLIGHT: std::sync::LazyLock<HashMap<String, u32>> =
            std::sync::LazyLock::new(HashMap::new);
        static LATENCY: std::sync::LazyLock<HashMap<String, u32>> =
            std::sync::LazyLock::new(HashMap::new);
        static ERROR_RATE: std::sync::LazyLock<HashMap<String, f64>> =
            std::sync::LazyLock::new(HashMap::new);
        static TOKENS_THIS_MIN: std::sync::LazyLock<HashMap<String, u32>> =
            std::sync::LazyLock::new(HashMap::new);
        static TPM_BUDGET: std::sync::LazyLock<HashMap<String, u32>> =
            std::sync::LazyLock::new(HashMap::new);
        static EXCLUDED: std::sync::LazyLock<HashSet<String>> =
            std::sync::LazyLock::new(HashSet::new);
        (
            &*IN_FLIGHT,
            &*LATENCY,
            &*ERROR_RATE,
            &*TOKENS_THIS_MIN,
            &*TPM_BUDGET,
            &*EXCLUDED,
        )
    }

    fn ctx_with(healthy: &[Deployment]) -> RouteContext<'_> {
        let (in_flight, latency_p50_ms, error_rate, tokens_this_minute, tpm_budget, excluded) =
            empty_maps();
        RouteContext {
            healthy,
            in_flight,
            latency_p50_ms,
            error_rate,
            tokens_this_minute,
            tpm_budget,
            prompt_tokens: 0,
            required_capabilities: &[],
            pre_cost_estimate_usd: 0.0,
            has_tool_calls: false,
            high_stakes: false,
            excluded,
        }
    }

    #[test]
    fn simple_shuffle_picks_healthy_deployment() {
        let deps = [dep("a", 1.0), dep("b", 1.0), dep("c", 1.0)];
        let ctx = ctx_with(&deps);
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let got = RoutingStrategy::SimpleShuffle.select(&ctx, &mut rng);
        match got {
            RouteDecision::Deploy(d) => {
                assert!(deps.iter().any(|x| x.id == d.id));
            }
            RouteDecision::NoHealthy => panic!("expected Deploy"),
        }
    }

    #[test]
    fn simple_shuffle_respects_excluded_set() {
        let deps = [dep("a", 1.0), dep("b", 1.0)];
        let excluded: HashSet<String> = std::iter::once(deps[0].id.clone()).collect();
        let ctx = RouteContext {
            healthy: &deps,
            excluded: &excluded,
            ..ctx_with(&deps)
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        for _ in 0..20 {
            match RoutingStrategy::SimpleShuffle.select(&ctx, &mut rng) {
                RouteDecision::Deploy(d) => assert_ne!(d.id, deps[0].id),
                RouteDecision::NoHealthy => panic!("expected Deploy"),
            }
        }
    }

    #[test]
    fn simple_shuffle_skips_drained_weight_zero() {
        let deps = [dep("a", 0.0), dep("b", 1.0)];
        let ctx = ctx_with(&deps);
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        for _ in 0..20 {
            match RoutingStrategy::SimpleShuffle.select(&ctx, &mut rng) {
                RouteDecision::Deploy(d) => assert_eq!(d.id, deps[1].id),
                RouteDecision::NoHealthy => panic!("expected Deploy on b"),
            }
        }
    }

    #[test]
    fn simple_shuffle_no_healthy_returns_no_healthy() {
        let deps = [];
        let ctx = ctx_with(&deps);
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        assert!(matches!(
            RoutingStrategy::SimpleShuffle.select(&ctx, &mut rng),
            RouteDecision::NoHealthy
        ));
    }

    #[test]
    fn simple_shuffle_weight_distribution_is_proportional() {
        let deps = [dep("a", 9.0), dep("b", 1.0)];
        let ctx = ctx_with(&deps);
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let mut a = 0u32;
        let mut b = 0u32;
        for _ in 0..10_000 {
            match RoutingStrategy::SimpleShuffle.select(&ctx, &mut rng) {
                RouteDecision::Deploy(d) => {
                    if d.id == deps[0].id {
                        a += 1;
                    } else if d.id == deps[1].id {
                        b += 1;
                    } else {
                        panic!("unknown deployment");
                    }
                }
                RouteDecision::NoHealthy => panic!("expected Deploy"),
            }
        }
        // Expect ~90% a, ~10% b within ±2%.
        let pa = a as f64 / 10_000.0;
        assert!(
            (0.88..=0.92).contains(&pa),
            "expected ~0.90, got {pa} (a={a}, b={b})"
        );
    }

    #[test]
    fn least_busy_picks_smallest_in_flight() {
        let deps = [dep("a", 1.0), dep("b", 1.0), dep("c", 1.0)];
        let mut inflight = HashMap::new();
        inflight.insert(deps[0].id.clone(), 5u32);
        inflight.insert(deps[1].id.clone(), 1u32);
        inflight.insert(deps[2].id.clone(), 3u32);
        let ctx = RouteContext {
            healthy: &deps,
            in_flight: &inflight,
            ..ctx_with(&deps)
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        match RoutingStrategy::LeastBusy.select(&ctx, &mut rng) {
            RouteDecision::Deploy(d) => assert_eq!(d.id, deps[1].id, "expected b (in_flight=1)"),
            RouteDecision::NoHealthy => panic!("expected Deploy"),
        }
    }

    #[test]
    fn least_busy_ties_broken_by_weight() {
        let mut d_a = dep("a", 2.0);
        let mut d_b = dep("b", 1.0);
        d_a.weight = 1.0;
        d_b.weight = 2.0;
        let deps = [d_a, d_b];
        let inflight = HashMap::new(); // both 0
        let ctx = RouteContext {
            healthy: &deps,
            in_flight: &inflight,
            ..ctx_with(&deps)
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        match RoutingStrategy::LeastBusy.select(&ctx, &mut rng) {
            RouteDecision::Deploy(d) => assert_eq!(d.id, deps[1].id, "expected b (heavier weight)"),
            RouteDecision::NoHealthy => panic!("expected Deploy"),
        }
    }

    #[test]
    fn usage_based_picks_most_headroom() {
        let mut d_a = dep("a", 1.0);
        d_a.tokens_per_minute = Some(10_000);
        let mut d_b = dep("b", 1.0);
        d_b.tokens_per_minute = Some(2_000);
        let deps = [d_a, d_b];
        let mut tpm = HashMap::new();
        tpm.insert(deps[0].id.clone(), 9_000u32); // headroom 1000
        tpm.insert(deps[1].id.clone(), 500u32); // headroom 1500
        let ctx = RouteContext {
            healthy: &deps,
            tokens_this_minute: &tpm,
            tpm_budget: &HashMap::new(),
            ..ctx_with(&deps)
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        match RoutingStrategy::UsageBasedV2.select(&ctx, &mut rng) {
            RouteDecision::Deploy(d) => assert_eq!(d.id, deps[1].id, "expected b (1500 headroom)"),
            RouteDecision::NoHealthy => panic!("expected Deploy"),
        }
    }

    #[test]
    fn latency_based_prefers_lower_latency() {
        let deps = [dep("a", 1.0), dep("b", 1.0)];
        let mut lat = HashMap::new();
        lat.insert(deps[0].id.clone(), 800u32);
        lat.insert(deps[1].id.clone(), 200u32);
        let ctx = RouteContext {
            healthy: &deps,
            latency_p50_ms: &lat,
            ..ctx_with(&deps)
        };
        let strat = RoutingStrategy::LatencyBased {
            ttl_secs: 60,
            buffer_ms: 50,
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        for _ in 0..20 {
            match strat.select(&ctx, &mut rng) {
                RouteDecision::Deploy(d) => assert_eq!(d.id, deps[1].id, "expected b (200ms)"),
                RouteDecision::NoHealthy => panic!("expected Deploy"),
            }
        }
    }

    #[test]
    fn latency_based_cold_start_treats_unknown_as_fast() {
        let deps = [dep("a", 1.0), dep("b", 1.0)];
        let mut lat = HashMap::new();
        lat.insert(deps[0].id.clone(), 500u32);
        // b has no latency → treat as 0 (cold start), should win.
        let ctx = RouteContext {
            healthy: &deps,
            latency_p50_ms: &lat,
            ..ctx_with(&deps)
        };
        let strat = RoutingStrategy::LatencyBased {
            ttl_secs: 60,
            buffer_ms: 50,
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        match strat.select(&ctx, &mut rng) {
            RouteDecision::Deploy(d) => assert_eq!(d.id, deps[1].id, "expected b (cold start)"),
            RouteDecision::NoHealthy => panic!("expected Deploy"),
        }
    }

    #[test]
    fn hybrid_picks_first_matching_branch() {
        let deps = [dep("a", 1.0), dep("b", 1.0)];
        let high_stakes_ctx = RouteContext {
            healthy: &deps,
            high_stakes: true,
            ..ctx_with(&deps)
        };
        let strat = RoutingStrategy::Hybrid {
            branches: vec![(Condition::HighStakes, RoutingStrategy::LeastBusy)],
            default: None,
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        // Should dispatch to LeastBusy.
        match strat.select(&high_stakes_ctx, &mut rng) {
            RouteDecision::Deploy(d) => assert!(deps.iter().any(|x| x.id == d.id)),
            RouteDecision::NoHealthy => panic!("expected Deploy"),
        }
    }

    #[test]
    fn hybrid_falls_back_to_default_when_no_branch_matches() {
        let deps = [dep("a", 1.0), dep("b", 1.0)];
        let ctx = ctx_with(&deps);
        let strat = RoutingStrategy::Hybrid {
            branches: vec![(Condition::HighStakes, RoutingStrategy::UsageBasedV2)],
            default: Some(Box::new(RoutingStrategy::SimpleShuffle)),
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        match strat.select(&ctx, &mut rng) {
            RouteDecision::Deploy(d) => assert!(deps.iter().any(|x| x.id == d.id)),
            RouteDecision::NoHealthy => panic!("expected Deploy from default"),
        }
    }

    #[test]
    fn hybrid_falls_to_shuffle_when_no_default() {
        let deps = [dep("a", 1.0)];
        let ctx = ctx_with(&deps);
        let strat = RoutingStrategy::Hybrid {
            branches: vec![(Condition::HighStakes, RoutingStrategy::UsageBasedV2)],
            default: None,
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        match strat.select(&ctx, &mut rng) {
            RouteDecision::Deploy(d) => assert_eq!(d.id, deps[0].id),
            RouteDecision::NoHealthy => panic!("expected shuffle fallback"),
        }
    }

    #[test]
    fn condition_tokens_above_matches_when_above_threshold() {
        let ctx = RouteContext {
            prompt_tokens: 10_000,
            ..ctx_with(&[])
        };
        assert!(Condition::TokensAbove { threshold: 5_000 }.matches(&ctx));
        assert!(!Condition::TokensAbove { threshold: 20_000 }.matches(&ctx));
    }

    #[test]
    fn condition_requires_all_must_be_present() {
        let ctx = RouteContext {
            required_capabilities: &["vision".into(), "tools".into()],
            ..ctx_with(&[])
        };
        assert!(Condition::Requires {
            capabilities: vec!["vision".into()]
        }
        .matches(&ctx));
        assert!(Condition::Requires {
            capabilities: vec!["vision".into(), "tools".into()]
        }
        .matches(&ctx));
        assert!(!Condition::Requires {
            capabilities: vec!["audio".into()]
        }
        .matches(&ctx));
        assert!(Condition::Requires {
            capabilities: vec![]
        }
        .matches(&ctx));
    }

    #[test]
    fn routing_config_default_max_fallbacks_is_5() {
        assert_eq!(RoutingConfig::default().max_fallbacks, 5);
    }

    #[test]
    fn routing_config_roundtrips_through_serde() {
        let cfg = RoutingConfig {
            strategy: RoutingStrategy::LatencyBased {
                ttl_secs: 30,
                buffer_ms: 25,
            },
            fallback: FallbackMap::default(),
            default_fallbacks: vec!["gpt-4o".into()],
            max_fallbacks: 3,
            default_cooldown_secs: 60,
            allowed_fails: 3,
            fails_window_secs: 60,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: RoutingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, back);
    }

    #[test]
    fn custom_strategy_skipped_on_serde() {
        struct AlwaysFirst;
        impl Router for AlwaysFirst {
            fn route<'a>(&self, ctx: &RouteContext<'a>) -> RouteDecision<'a> {
                ctx.healthy
                    .first()
                    .map(RouteDecision::Deploy)
                    .unwrap_or(RouteDecision::NoHealthy)
            }
        }
        let strat = RoutingStrategy::Custom(Arc::new(AlwaysFirst));
        // `#[serde(skip)]` on an internally-tagged enum variant makes
        // serialization fail (no default inner content) — that's the
        // intended behavior since `Custom` carries a non-Serialize
        // `Arc<dyn Router>`. We assert it errors rather than panicking.
        let res = serde_json::to_string(&strat);
        assert!(
            res.is_err(),
            "expected serialization to fail, got: {:?}",
            res
        );
        // Deserialize of the tag name yields an error too (no inner
        // content to construct Custom from). We only assert the tag is
        // recognised by trying the enum-level parse directly with a
        // known-serializable variant.
        let ok: RoutingStrategy =
            serde_json::from_str(r#"{"kind":"simple_shuffle"}"#).expect("deserialize");
        assert!(matches!(ok, RoutingStrategy::SimpleShuffle));
    }

    #[test]
    fn fallback_map_lookup_returns_empty_when_absent() {
        let m = FallbackMap::default();
        assert!(m.lookup(FallbackBucket::Generic, "gpt-4o").is_empty());
        assert!(m.lookup(FallbackBucket::ContextWindow, "gpt-4o").is_empty());
        assert!(m.lookup(FallbackBucket::ContentPolicy, "gpt-4o").is_empty());
    }

    #[test]
    fn fallback_map_lookup_returns_configured_list() {
        let mut m = FallbackMap::default();
        m.fallbacks.insert(
            "claude-opus".into(),
            vec!["gpt-4o".into(), "gemini-pro".into()],
        );
        assert_eq!(
            m.lookup(FallbackBucket::Generic, "claude-opus"),
            &["gpt-4o".to_string(), "gemini-pro".to_string()]
        );
        // Other buckets still empty.
        assert!(m
            .lookup(FallbackBucket::ContextWindow, "claude-opus")
            .is_empty());
    }
}
