// Atlas OS — Aggregation engine (RFC 04 §3, sub-fase 2.2).
//
// When the orchestrator routes to an aggregation mode for a mission
// running in `ExecutionMode::HighStakes` (RFC 19), it must fuse the
// responses of multiple model deployments into a single output string
// before the Coding/Planning engine consumes it. Sub-fase 2.2 ports
// six peer-reviewed strategies into the orchestrator:
//
// 1. `MajorityVote` — Agent Forest (arxiv 2402.05120, Li et al.). N
//    difficulty-aware parallel samples + stop-early on 2/3 agreement.
// 2. `MoA`       — Mixture-of-Agents (arxiv 2406.04692, Wang et al.).
//    Layered architecture, 3 layers × 3 models default. Restricted
//    to HighStakes by the cost guard.
// 3. `Council`   — Multiagent Debate (arxiv 2305.14325, Du et al.).
//    2-3 debaters + 1 round by default (cost control); votes persist
//    to the `council_votes` table (M22).
// 4. `Reflexion` — Verbal Reinforcement Learning (arxiv 2303.11366,
//    Shinn et al.). Multi-model: executor caro + reflexor barato is
//    an Atlas OS contribution (the paper uses a single model;
//    see RFC 22 §2 for the attribution note). Anti-doom-loop guard
//    aborts when 2 consecutive episodes share the same `failure_signal`.
// 5. `SelfRefine` — Iterative Refinement with Self-Feedback (arxiv
//    2303.17651, Madaan et al.). Single-LLM generator→feedback→refiner
//    loop capped at 2 iterations; abort when `delta_lines < 10`.
// 6. `SelfDiscover` — Self-Compose Reasoning Structures (arxiv
//    2402.03620, Zhou et al.). Planning engine pre-decode: pick
//    reasoning modules + emit a JSON skeleton cached by
//    `URL + prompt_embedding`.
//
// `AggregationMode` is the profile-facing enum selected by `Profile
// .aggregation` (sub-fase 2.2). The `Aggregator` trait carries the
// single `aggregate` method; each mode implements it. The trait is
// `async_trait` (dtolnay) so we can `Arc<dyn Aggregator>` the mode
// for runtime dispatch. A `BoxedAggregator::for_mode` factory returns
// the matching `Aggregator` impl.
//
// The `cost_guard` (sub-fase 2.1, `AggregationPolicy` trait) sits in
// front of every mode: the orchestrator calls `pre_cost_estimate`
// before dispatch and refuses with `AggregationError::BudgetExceeded`
// when the estimate overshoots `Profile.budget_per_turn`. The HUD
// surfaces a "Aggregation rejected: $X > budget $Y" card (RFC 19 §3.4).
//
// `RequestFrame` from sub-fase 2.1 (G12) threads the idempotency key
// across every parallel sample so the provider deduplicates replays.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::orchestrator::cost_guard::{AggregationCostBreakdown, AggregationPolicy};
use crate::orchestrator::idempotency::RequestFrame;
use crate::orchestrator::registry::Registry;
use crate::orchestrator::routing::RouteContext;

pub mod council;
pub mod majority_vote;
pub mod moa;
pub mod reflexion;
pub mod self_discover;
pub mod self_refine;

pub use council::CouncilAggregator;
pub use majority_vote::MajorityVoteAggregator;
pub use moa::MoAAggregator;
pub use reflexion::ReflexionAggregator;
pub use self_discover::SelfDiscoverAggregator;
pub use self_refine::SelfRefineAggregator;

/// Empty `RouteContext` builder shared by aggregation tests. Exposes
/// static-scoped empty maps so the lifetime is `'static` and the
/// call-site is zero-cost.
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn empty_route_ctx() -> RouteContext<'static> {
    use std::collections::{HashMap, HashSet};
    use std::sync::LazyLock;
    static IN_FLIGHT: LazyLock<HashMap<String, u32>> = LazyLock::new(HashMap::new);
    static LATENCY: LazyLock<HashMap<String, u32>> = LazyLock::new(HashMap::new);
    static ERROR_RATE: LazyLock<HashMap<String, f64>> = LazyLock::new(HashMap::new);
    static TOKENS_THIS_MIN: LazyLock<HashMap<String, u32>> = LazyLock::new(HashMap::new);
    static TPM_BUDGET: LazyLock<HashMap<String, u32>> = LazyLock::new(HashMap::new);
    static EXCLUDED: LazyLock<HashSet<String>> = LazyLock::new(HashSet::new);
    static STRONG: LazyLock<HashSet<String>> = LazyLock::new(HashSet::new);
    static HEALTHY: LazyLock<Vec<crate::orchestrator::provider::Deployment>> =
        LazyLock::new(Vec::new);
    RouteContext {
        healthy: &HEALTHY,
        in_flight: &IN_FLIGHT,
        latency_p50_ms: &LATENCY,
        error_rate: &ERROR_RATE,
        tokens_this_minute: &TOKENS_THIS_MIN,
        tpm_budget: &TPM_BUDGET,
        prompt_tokens: 0,
        required_capabilities: &[],
        pre_cost_estimate_usd: 0.0,
        has_tool_calls: false,
        high_stakes: false,
        excluded: &EXCLUDED,
        strong_ids: &STRONG,
        classifier_confidence: 0.0,
    }
}

/// Static `&'static Registry` for aggregation tests — the
/// `AggregationContext.registry` field borrows the registry, so a
/// temporary `&Registry::default()` would not live long enough.
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_registry() -> &'static Registry {
    use std::sync::LazyLock;
    static REG: LazyLock<Registry> = LazyLock::new(Registry::default);
    &REG
}

/// Static `&'static dyn AggregationPolicy` for aggregation tests —
/// the `AggregationContext.policy` field borrows the policy, so a
/// temporary `&NoAggregation` would not live long enough.
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_policy() -> &'static dyn crate::orchestrator::cost_guard::AggregationPolicy {
    use std::sync::LazyLock;
    static POL: LazyLock<crate::orchestrator::cost_guard::NoAggregation> =
        LazyLock::new(crate::orchestrator::cost_guard::NoAggregation::default);
    &*POL
}

/// Aggregation mode selected by the profile. `Single` is the default
/// and short-circuits aggregation entirely — the orchestrator passes
/// through the single routed response untouched.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AggregationMode {
    /// No aggregation — pass-through. Default.
    #[default]
    Single,
    /// Agent Forest majority vote (arxiv 2402.05120). N parallel
    /// samples with stop-early when agreement ≥ `stop_early_threshold`
    /// after each batch of 3 votes. `n_samples` is difficulty-aware
    /// (1 for easy, 3 for medium, 5/9 for hard).
    MajorityVote {
        n_samples: u8,
        stop_early_threshold: f32,
    },
    /// Mixture-of-Agents layered architecture (arxiv 2406.04692). Each
    /// layer is a `Vec<ModelId>` executed in parallel; layer N+1 takes
    /// every layer-N output as auxiliary input. Restricted to
    /// `ExecutionMode::HighStakes` by the cost guard. Default 3×3.
    MoA { layers: Vec<Vec<String>> },
    /// Multiagent Debate (arxiv 2305.14325). 2-3 debaters + 1 round by
    /// default (cost control). Votes persist to `council_votes` (M22).
    Council { debaters: Vec<String>, rounds: u8 },
    /// Self-Refine iterative refinement (arxiv 2303.17651). Single-LLM
    /// generator→feedback→refiner loop. Capped at 2 iterations by
    /// default; abort when `delta_lines < 10`.
    SelfRefine {
        max_iterations: u8,
        stop_condition: StopCondition,
    },
    /// Reflexion verbal reinforcement learning (arxiv 2303.11366).
    /// Multi-model: executor caro + reflexor barato. Atlas OS
    /// contribution (paper uses same model). Anti-doom-loop guard
    /// aborts on 2 consecutive identical `failure_signal` (RFC 19).
    Reflexion { memory_buffer_size: u8 },
    /// Self-Discover reasoning structures (arxiv 2402.03620, Zhou et
    /// al.). Planning engine pre-decode selects reasoning modules →
    /// JSON skeleton cached by `URL + prompt_embedding`.
    SelfDiscover { cache_ttl_secs: u64 },
}

/// Stop condition for `SelfRefine`. `MinDeltaLines(u32)` aborts when
/// the line-diff between consecutive refinement iterations drops below
/// the threshold (the model stopped making meaningful changes).
/// `MaxIterations(u8)` is equivalent to the enum field but allows
/// override per-mission without rewriting the profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StopCondition {
    MinDeltaLines { lines: u32 },
    MaxIterations { iters: u8 },
    ConvergenceThreshold { threshold: f32 },
}

/// Snapshot of what the aggregation actually executed — surfaced to
/// the HUD and persisted into `model_invocations.route_taken_json`
/// (M21) for the audit trail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AggregationModeSnapshot {
    /// Mode name as selected by the profile.
    pub mode: String,
    /// Number of parallel samples dispatched (may differ from
    /// `n_samples` when stop-early triggered in MajorityVote or
    /// Reflexion aborted before exhausting `attempt_no`).
    pub samples_dispatched: u32,
    /// Number of rounds executed (Council / SelfRefine / Reflexion).
    /// 1 for stateless modes (MajorityVote single batch, MoA layers).
    pub rounds_executed: u32,
    /// Mode-specific JSON blob for forensics (debatER claims, reflexor
    /// verbal_reflection text hashes, self-discover skeleton id).
    pub mode_metadata: serde_json::Value,
}

/// Aggregated output returned to the orchestrator. `text` is the
/// fused response the downstream engine consumes; `mode_used` and
/// `route_taken_json` flow into the journal for audit; `cost_breakdown`
/// is emitted to the HUD cost-guard card (RFC 19 §3.4).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FusedResponse {
    /// Fused text the downstream engine consumes.
    pub text: String,
    /// Snapshot of how the aggregation ran.
    pub mode_used: AggregationModeSnapshot,
    /// JSON array of the cascade actually executed (`provider`,
    /// `model_id`, `outcome` per step). Mirrors the M21 column.
    pub route_taken_json: serde_json::Value,
    /// Cost breakdown surfaced to the HUD.
    pub cost_breakdown: AggregationCostBreakdown,
    /// Reflection episodes created (Reflexion mode only). Empty for
    /// other modes. Persisted to `reflection_episodes` (M22) by the
    /// Reflexion impl itself.
    pub reflections: Vec<ReflectionEpisodeOut>,
}

/// Lightweight handle to a persisted reflection episode. Only
/// populated when the mode was `Reflexion`. The orchestrator surfaces
/// these in the HUD audit card so the user can inspect the verbal
/// reflection chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ReflectionEpisodeOut {
    pub episode_id: Uuid,
    pub mission_id: Uuid,
    pub attempt_no: u8,
    pub executor_model: String,
    pub reflexor_model: String,
    pub failure_signal: String,
    pub verbal_reflection: String,
}

/// Aggregate failure modes surfaced to the orchestrator. The
/// orchestrator maps these to HUD cards / supervisor signals.
#[derive(Debug, thiserror::Error)]
pub enum AggregationError {
    /// Cost guard rejection — `pre_cost_estimate > budget_per_turn`.
    /// The HUD renders a "$X > budget $Y" card (RFC 19 §3.4).
    #[error("aggregation rejected by cost guard: ${est_usd:.4} > budget ${budget_usd:.4}")]
    BudgetExceeded { est_usd: f64, budget_usd: f64 },
    /// Reflexion anti-doom-loop abort — 2 consecutive identical
    /// `failure_signal`. RFC 19 supervisor escalation.
    #[error(
        "reflexion doom loop detected: failure_signal {signal} repeated in 2 consecutive episodes"
    )]
    DoomLoop { signal: String },
    /// SelfRefine stall — `delta_lines < stop_condition` threshold.
    #[error("self-refine stalled: delta_lines {delta} < threshold {threshold}")]
    SelfRefineStalled { delta: u32, threshold: u32 },
    /// SelfDiscover skeleton parse failure.
    #[error("self-discover skeleton parse: {0}")]
    SkeletonParse(String),
    /// Provider dispatch failure during aggregation (sample round).
    #[error(transparent)]
    Dispatch(#[from] anyhow::Error),
}

/// Read-only context handed to every `Aggregator::aggregate` call.
/// Holds references the impls need to query the registry, persist to
/// the journal, read the route snapshot, and check the cost guard.
/// Lifetime-parameterised to avoid cloning snapshots — aggregators
/// run inside a single orchestrator tick.
pub struct AggregationContext<'a> {
    /// Final selected aggregation mode and parameters.
    pub mode: &'a AggregationMode,
    /// Idempotency frame (G12) — the same `idempotency_key` is sent
    /// with every parallel sample so the provider deduplicates
    /// replays.
    pub request_frame: &'a RequestFrame,
    /// Route snapshot from the routing layer (2.1). Used to describe
    /// the chosen deployments / fallback map to the aggregator.
    pub route: &'a RouteContext<'a>,
    /// Cost guard (G11) — `pre_cost_estimate` must be checked before
    /// dispatching the parallel sample.
    pub policy: &'a dyn AggregationPolicy,
    /// Model registry handle for resolving model descriptors
    /// (cost-per-1M, context window, capabilities).
    pub registry: &'a Registry,
    /// Per-turn USD budget ceiling. `None` means unlimited (rare —
    /// HighStakes missions always set a budget).
    pub budget_per_turn: Option<f64>,
}

impl AggregationMode {
    /// Maximum number of parallel samples this mode could dispatch.
    /// Used by the orchestrator pre-flight to size the backpressure
    /// semaphore (sub-fase 2.0.5).
    pub fn max_parallel_samples(&self) -> u32 {
        match self {
            Self::Single => 1,
            Self::MajorityVote { n_samples, .. } => (*n_samples).max(1) as u32,
            Self::MoA { layers } => layers.iter().map(|l| l.len()).max().unwrap_or(1) as u32,
            Self::Council { debaters, .. } => debaters.len().max(1) as u32,
            Self::SelfRefine { max_iterations, .. } => (*max_iterations).max(1) as u32,
            Self::Reflexion { .. } => 3,
            Self::SelfDiscover { .. } => 1,
        }
    }

    /// Display name — used in `AggregationModeSnapshot.mode` and HUD
    /// card labels.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::MajorityVote { .. } => "majority_vote",
            Self::MoA { .. } => "moa",
            Self::Council { .. } => "council",
            Self::SelfRefine { .. } => "self_refine",
            Self::Reflexion { .. } => "reflexion",
            Self::SelfDiscover { .. } => "self_discover",
        }
    }
}

/// Aggregator trait — single entry point the orchestrator invokes.
/// Implementations are stateful only when persistence is required
/// (Council writes to `council_votes`; Reflexion writes to
/// `reflection_episodes`). Stateless modes (MajorityVote single
/// batch, MoA layers, SelfDiscover skeleton) hold only config.
#[async_trait]
pub trait Aggregator: Send + Sync {
    async fn aggregate(
        &self,
        ctx: AggregationContext<'_>,
    ) -> Result<FusedResponse, AggregationError>;
}

/// Factory: pick the matching `Aggregator` impl for an
/// `AggregationMode`. Returns `None` for `Single` (no-op pass-through
/// handled by the orchestrator itself).
pub fn aggregator_for(mode: &AggregationMode) -> Option<Arc<dyn Aggregator>> {
    match mode {
        AggregationMode::Single => None,
        AggregationMode::MajorityVote { .. } => Some(Arc::new(MajorityVoteAggregator)),
        AggregationMode::MoA { .. } => Some(Arc::new(MoAAggregator)),
        AggregationMode::Council { .. } => Some(Arc::new(CouncilAggregator)),
        AggregationMode::SelfRefine { .. } => Some(Arc::new(SelfRefineAggregator)),
        AggregationMode::Reflexion { .. } => Some(Arc::new(ReflexionAggregator)),
        AggregationMode::SelfDiscover { .. } => Some(Arc::new(SelfDiscoverAggregator)),
    }
}

/// Sentinel surfaced when `AggregationMode::Single` would have been
/// routed but the caller invoked `aggregate` directly. The
/// orchestrator handles `Single` before reaching the Aggregator
/// trait, so this is a defensive guard.
pub fn passthrough_single(text: String) -> FusedResponse {
    FusedResponse {
        text,
        mode_used: AggregationModeSnapshot {
            mode: "single".into(),
            samples_dispatched: 1,
            rounds_executed: 1,
            mode_metadata: serde_json::Value::Null,
        },
        route_taken_json: serde_json::Value::Array(vec![]),
        cost_breakdown: AggregationCostBreakdown::default(),
        reflections: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_mode_max_parallel_samples_is_1() {
        assert_eq!(AggregationMode::Single.max_parallel_samples(), 1);
        assert_eq!(AggregationMode::Single.display_name(), "single");
    }

    #[test]
    fn moa_max_parallel_samples_takes_widest_layer() {
        let mode = AggregationMode::MoA {
            layers: vec![
                vec![
                    "gpt-4o".into(),
                    "claude-sonnet-4".into(),
                    "gemini-2.5-pro".into(),
                ],
                vec!["llama-3.1-405b".into()],
                vec!["gpt-4o".into(), "claude-opus-4".into()],
            ],
        };
        assert_eq!(mode.max_parallel_samples(), 3);
        assert_eq!(mode.display_name(), "moa");
    }

    #[test]
    fn aggregation_mode_serde_roundtrip_preserves_tag_and_fields() {
        let mode = AggregationMode::MajorityVote {
            n_samples: 7,
            stop_early_threshold: 0.66,
        };
        let json = serde_json::to_string(&mode).unwrap();
        let back: AggregationMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, back);
        let j2 = serde_json::to_string(&AggregationMode::Council {
            debaters: vec!["gpt-4o".into(), "claude-opus-4".into()],
            rounds: 1,
        })
        .unwrap();
        let parsed: AggregationMode = serde_json::from_str(&j2).unwrap();
        assert_eq!(parsed.display_name(), "council");
    }
}
