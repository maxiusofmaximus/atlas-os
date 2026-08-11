// OpenCode OS — Cascade failover (RFC 04 §6, sub-fase 2.1).
//
// The cascade is the orchestrator's escalation policy: when the
// primary `Deployment` for a request returns a non-retryable error
// (401/404/408), overflows its context window, or refuses on content
// policy, the orchestrator walks an ordered list of fallback
// deployments.
//
// The policy is the three-bucket LiteLLM taxonomy plus a final
// `default_fallbacks` list:
//
// 1. **Weighted failover within the same `model_id` group** — when a
//    `Deployment` 429s (cooldown) or saturates (`max_parallel`), we
//    try other healthy `Deployment`s serving the same `model_id`
//    first. The exclusion set accumulates the IDs already tried so
//    they're filtered out on the next round.
// 2. **Bucket escalation** — once the same-group failover is
//    exhausted (or the error was non-retryable, bypassing step 1),
//    we pick the next fallback `model_id` from the bucket matching
//    the failure mode:
//    - `FallbackBucket::Generic` for 401/404/408/network.
//    - `FallbackBucket::ContextWindow` for prompt-overflow.
//    - `FallbackBucket::ContentPolicy` for provider refusals.
// 3. **`default_fallbacks`** — when no bucket-specific list matches,
//    we consult `RoutingConfig::default_fallbacks` as the catch-all.
// 4. **`max_fallbacks` cap** (default 5). After
//    `max_fallbacks` attempts the orchestrator gives up and surfaces
//    a paused-mission HUD card (RFC 19).
//
// The `Cascade` is intentionally stateless across calls: each call to
// `next_target` consults the supplied healthy-for closure (so the
// caller can layer cooldown + back-pressure filtering), appends to
// the `excluded` set it carries, and increments `attempt`. The
// driving state is the caller's; `Cascade` is a thin policy shim.

use std::collections::HashSet;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::orchestrator::provider::Deployment;
use crate::orchestrator::routing::{
    FallbackBucket, FallbackMap, RouteContext, RouteDecision, RoutingConfig, RoutingStrategy,
};

/// Outcome of a single cascade step.
#[derive(Debug, Clone, PartialEq)]
pub enum CascadeStep<'a> {
    /// The orchestrator should dispatch to this deployment next. The
    /// `model_id` is included separately so the orchestrator can
    /// switch model groups even when the deployment belongs to a
    /// different group than the primary (it always does — same-group
    /// lookups use the primary `model_id` directly).
    TryNext {
        deployment: &'a Deployment,
        model_id: String,
        bucket: FallbackBucket,
        attempt_index: u8,
    },
    /// The orchestrator has exhausted all fallbacks. It should
    /// surface a paused-mission HUD card (RFC 19) and stop.
    Exhausted {
        reason: ExhaustionReason,
        attempts: u8,
    },
}

/// Why the cascade gave up.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExhaustionReason {
    /// All fallback buckets (`fallbacks`/`context_window_fallbacks`/
    /// `content_policy_fallbacks`/`default_fallbacks`) are empty for
    /// the primary.
    NoFallbackConfigured,
    /// `max_fallbacks` reached (default 5).
    MaxFallbacksReached,
    /// All configured fallback `model_id`s had no healthy `Deployment`
    /// (every candidate is in cooldown, drained, or saturated).
    AllFallbacksUnhealthy,
}

/// Discriminator of the current failure mode — picks which bucket to
/// consult. The orchestrator maps an upstream provider error to one
/// of these before calling `Cascade::next_target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureMode {
    /// 401/403/404/408/network — go to `fallbacks` bucket.
    BadConfigOrNetwork,
    /// 429 / 5xx storm — falls back to same-group weighted failover
    /// first (not a bucket escalation).
    RateLimited,
    /// Prompt overflow — `context_window_fallbacks` bucket.
    ContextWindowOverflow,
    /// Provider content-policy refusal — `content_policy_fallbacks`.
    ContentPolicyRefusal,
}

impl FailureMode {
    pub fn bucket(self) -> FallbackBucket {
        match self {
            // Rate-limited uses same-group failover; bucket is
            // only consulted if the entire group is drained.
            Self::RateLimited | Self::BadConfigOrNetwork => FallbackBucket::Generic,
            Self::ContextWindowOverflow => FallbackBucket::ContextWindow,
            Self::ContentPolicyRefusal => FallbackBucket::ContentPolicy,
        }
    }
}

/// Pure cascade driver. Stateless across calls — pass in a closure
/// that resolves a `model_id` to its healthy `Deployment`s, the
/// routing config, the primary `model_id`, an accumulation of
/// already-tried `Deployment::id`s, and the count of fallback
/// attempts already performed (callers track this to honour
/// `max_fallbacks`).
pub struct Cascade {
    config: RoutingConfig,
    primary_model_id: String,
    excluded: HashSet<String>,
    attempt: u8,
}

impl Cascade {
    pub fn new(config: RoutingConfig, primary_model_id: impl Into<String>) -> Self {
        Self {
            config,
            primary_model_id: primary_model_id.into(),
            excluded: HashSet::new(),
            attempt: 0,
        }
    }

    /// Mark this `Deployment::id` as already-tried so future
    /// `next_target` calls skip it.
    pub fn exclude(&mut self, deployment_id: impl Into<String>) {
        self.excluded.insert(deployment_id.into());
    }

    /// Current attempt index (0 on the first primary call, increments
    /// on every fallback).
    pub fn attempt_index(&self) -> u8 {
        self.attempt
    }

    /// Pick the next target. The orchestrator calls this:
    /// - Once for the primary (mode irrelevant — it just builds the
    ///   `RouteContext` and runs `RoutingStrategy::select`).
    /// - Then on every subsequent failure with the `FailureMode` that
    ///   describes the just-failed response.
    ///
    /// `healthy_for` is a closure that returns healthy deployments
    /// for a `model_id` — lifted from the cascade because, in
    /// production, "healthy" requires consulting the cooldown store
    /// and backpressure state which the cascade deliberately does
    /// not own. Tests pass a passthrough closure.
    pub fn next_target<'a, F>(&mut self, mode: FailureMode, healthy_for: F) -> CascadeStep<'a>
    where
        F: Fn(&str) -> Vec<&'a Deployment>,
    {
        // 1. Increment attempt counter.
        self.attempt += 1;
        if self.attempt > self.config.max_fallbacks {
            return CascadeStep::Exhausted {
                reason: ExhaustionReason::MaxFallbacksReached,
                attempts: self.attempt - 1,
            };
        }
        // Seed our own RNG — the cascade is deterministic per call so
        // we don't want caller-side state to leak in.
        let mut rng = StdRng::from_entropy();
        // 2. Same-group weighted failover when the failure is
        //    transient (RateLimited). Other failure modes skip to
        //    the bucket escalation directly.
        if mode == FailureMode::RateLimited {
            let primary_healthy = healthy_for(&self.primary_model_id);
            let eligible: Vec<&Deployment> = primary_healthy
                .into_iter()
                .filter(|d| !self.excluded.contains(&d.id))
                .filter(|d| d.weight > 0.0)
                .collect();
            if let Some(d) = pick_via_strategy(&self.config.strategy, &eligible, &mut rng) {
                self.excluded.insert(d.id.clone());
                return CascadeStep::TryNext {
                    deployment: d,
                    model_id: self.primary_model_id.clone(),
                    bucket: FallbackBucket::Generic,
                    attempt_index: self.attempt,
                };
            }
        }
        // 3. Bucket escalation — collect candidate model_ids ordered.
        let candidates = ordered_fallback_models(
            &self.config.fallback,
            mode,
            &self.primary_model_id,
            &self.config.default_fallbacks,
        );
        if candidates.is_empty() {
            return CascadeStep::Exhausted {
                reason: ExhaustionReason::NoFallbackConfigured,
                attempts: self.attempt - 1,
            };
        }
        // 4. Walk candidates; for each model_id, look up its healthy
        //    deployments, filter by exclusion set, pick with strategy.
        let mut any_unhealthy = false;
        for model_id in &candidates {
            let deps = healthy_for(model_id);
            let eligible: Vec<&Deployment> = deps
                .into_iter()
                .filter(|d| !self.excluded.contains(&d.id))
                .filter(|d| d.weight > 0.0)
                .collect();
            if eligible.is_empty() {
                any_unhealthy = true;
                continue;
            }
            if let Some(d) = pick_via_strategy(&self.config.strategy, &eligible, &mut rng) {
                self.excluded.insert(d.id.clone());
                return CascadeStep::TryNext {
                    deployment: d,
                    model_id: model_id.clone(),
                    bucket: mode.bucket(),
                    attempt_index: self.attempt,
                };
            }
            any_unhealthy = true;
        }
        let _ = any_unhealthy;
        // 5. All fallback models are unhealthy.
        CascadeStep::Exhausted {
            reason: ExhaustionReason::AllFallbacksUnhealthy,
            attempts: self.attempt - 1,
        }
    }
}

/// Build the ordered list of fallback `model_id`s for `mode`,
/// combining bucket-specific list and the catch-all.
fn ordered_fallback_models(
    map: &FallbackMap,
    mode: FailureMode,
    primary: &str,
    default_fallbacks: &[String],
) -> Vec<String> {
    let mut out = Vec::new();
    let bucket = map.lookup(mode.bucket(), primary);
    for m in bucket {
        if !out.contains(m) {
            out.push(m.clone());
        }
    }
    for m in default_fallbacks {
        if !out.contains(m) {
            out.push(m.clone());
        }
    }
    out
}

/// Run `RoutingStrategy::select` on a slice of `&Deployment` references and
/// return the chosen one, if any. We materialise a minimal
/// `RouteContext` (all telemetry empty) because the cascade's role
/// is healthy-filter-and-pick, not full latency/usage dispatch — the
/// orchestrator loop does the latter on the primary call.
fn pick_via_strategy<'a, R: Rng + ?Sized>(
    strategy: &RoutingStrategy,
    eligible: &[&'a Deployment],
    rng: &mut R,
) -> Option<&'a Deployment> {
    if eligible.is_empty() {
        return None;
    }
    let empty_inflight: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let empty_lat: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let empty_err: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    let empty_tpm: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let empty_budget: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let empty_excluded: HashSet<String> = HashSet::new();
    // We clone to satisfy `RouteContext`'s borrowed `&[Deployment]`
    // (the strategy only reads the slice; the clones are local).
    let healthy_refs: Vec<Deployment> = eligible.iter().map(|d| (*d).clone()).collect();
    let ctx = RouteContext {
        healthy: &healthy_refs,
        in_flight: &empty_inflight,
        latency_p50_ms: &empty_lat,
        error_rate: &empty_err,
        tokens_this_minute: &empty_tpm,
        tpm_budget: &empty_budget,
        prompt_tokens: 0,
        required_capabilities: &[],
        pre_cost_estimate_usd: 0.0,
        has_tool_calls: false,
        high_stakes: false,
        excluded: &empty_excluded,
    };
    match strategy.select(&ctx, rng) {
        RouteDecision::Deploy(d) => {
            // `d` borrows from `healthy_refs` which is local — we need
            // to find the same Deployment by id from `eligible`.
            eligible.iter().find(|x| x.id == d.id).copied()
        }
        RouteDecision::NoHealthy => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn dep(id: &str, model_id: &str, weight: f32) -> Deployment {
        let mut d = Deployment::new(model_id, format!("https://{id}"));
        d.weight = weight;
        d
    }

    fn dep_full(model_id: &str, suffix: &str, weight: f32) -> Deployment {
        let mut d = Deployment::new(model_id, format!("https://{model_id}-{suffix}"));
        d.weight = weight;
        d
    }

    fn healthy_for_static<'a>(deps: &'a [Deployment]) -> impl Fn(&str) -> Vec<&'a Deployment> + 'a {
        move |model_id: &str| deps.iter().filter(|d| d.model_id == model_id).collect()
    }

    #[test]
    fn rate_limited_falls_back_within_same_group_first() {
        let deps = vec![
            dep("a", "claude-opus", 1.0),
            dep("b", "claude-opus", 1.0),
            dep("c", "gpt-4o", 1.0),
        ];
        let rf = RoutingConfig::default();
        let mut cascade = Cascade::new(rf, "claude-opus");
        let step = cascade.next_target(FailureMode::RateLimited, healthy_for_static(&deps));
        match step {
            CascadeStep::TryNext {
                model_id, bucket, ..
            } => {
                assert_eq!(model_id, "claude-opus", "same group");
                assert_eq!(bucket, FallbackBucket::Generic);
            }
            other => panic!("expected TryNext, got {other:?}"),
        }
    }

    #[test]
    fn exhausted_when_max_fallbacks_reached() {
        let deps = vec![dep("a", "m1", 1.0)];
        let rf = RoutingConfig {
            max_fallbacks: 0,
            ..Default::default()
        };
        let mut cascade = Cascade::new(rf, "m1");
        let step = cascade.next_target(FailureMode::BadConfigOrNetwork, healthy_for_static(&deps));
        assert!(matches!(
            step,
            CascadeStep::Exhausted {
                reason: ExhaustionReason::MaxFallbacksReached,
                ..
            }
        ));
    }

    #[test]
    fn no_fallback_configured_when_buckets_empty() {
        let deps = vec![dep("a", "m1", 1.0)];
        let rf = RoutingConfig::default();
        let mut cascade = Cascade::new(rf, "m1");
        let step = cascade.next_target(FailureMode::BadConfigOrNetwork, healthy_for_static(&deps));
        assert!(matches!(
            step,
            CascadeStep::Exhausted {
                reason: ExhaustionReason::NoFallbackConfigured,
                ..
            }
        ));
    }

    #[test]
    fn context_window_overflow_uses_context_window_bucket() {
        let deps = vec![dep("a", "m_small", 1.0), dep("b", "m_big", 1.0)];
        let rf = RoutingConfig {
            fallback: {
                let mut f = FallbackMap::default();
                f.context_window_fallbacks
                    .insert("m_small".into(), vec!["m_big".into()]);
                f
            },
            ..Default::default()
        };
        let mut cascade = Cascade::new(rf, "m_small");
        let step = cascade.next_target(
            FailureMode::ContextWindowOverflow,
            healthy_for_static(&deps),
        );
        match step {
            CascadeStep::TryNext {
                model_id, bucket, ..
            } => {
                assert_eq!(model_id, "m_big");
                assert_eq!(bucket, FallbackBucket::ContextWindow);
            }
            other => panic!("expected TryNext, got {other:?}"),
        }
    }

    #[test]
    fn content_policy_refusal_uses_content_policy_bucket() {
        let deps = vec![dep("a", "p1", 1.0), dep("b", "p2", 1.0)];
        let rf = RoutingConfig {
            fallback: {
                let mut f = FallbackMap::default();
                f.content_policy_fallbacks
                    .insert("p1".into(), vec!["p2".into()]);
                f
            },
            ..Default::default()
        };
        let mut cascade = Cascade::new(rf, "p1");
        let step =
            cascade.next_target(FailureMode::ContentPolicyRefusal, healthy_for_static(&deps));
        match step {
            CascadeStep::TryNext { model_id, .. } => assert_eq!(model_id, "p2"),
            other => panic!("expected TryNext, got {other:?}"),
        }
    }

    #[test]
    fn default_fallbacks_consulted_when_no_bucket_match() {
        let deps = vec![dep("a", "primary", 1.0), dep("b", "catchall", 1.0)];
        let rf = RoutingConfig {
            default_fallbacks: vec!["catchall".into()],
            ..Default::default()
        };
        let mut cascade = Cascade::new(rf, "primary");
        let step = cascade.next_target(FailureMode::BadConfigOrNetwork, healthy_for_static(&deps));
        match step {
            CascadeStep::TryNext { model_id, .. } => assert_eq!(model_id, "catchall"),
            other => panic!("expected TryNext, got {other:?}"),
        }
    }

    #[test]
    fn exclusion_set_accumulates() {
        let mut d_a = dep_full("m1", "a", 1.0);
        let mut d_b = dep_full("m1", "b", 1.0);
        let mut d_c = dep_full("m1", "c", 1.0);
        // Force deterministic ordering of the model group by giving
        // them alphabetic ids (`Deployment::new` hashes api_base into
        // the id, so we cannot rely on stable order — but the exclusion
        // set means we cannot pick the same one twice in a row).
        let _ = (&mut d_a, &mut d_b, &mut d_c);
        let deps = vec![d_a, d_b, d_c];
        let rf = RoutingConfig::default();
        let mut cascade = Cascade::new(rf, "m1");
        // First fallback → some m1 deployment.
        let step1 = cascade.next_target(FailureMode::RateLimited, healthy_for_static(&deps));
        let d1 = match step1 {
            CascadeStep::TryNext { deployment, .. } => deployment,
            other => panic!("expected TryNext, got {other:?}"),
        };
        // Third fallback when all m1 deployments are excluded → falls
        // to "no fallback configured" since buckets are empty. We
        // execute two and assert IDs differ.
        let step2 = cascade.next_target(FailureMode::RateLimited, healthy_for_static(&deps));
        let d2 = match step2 {
            CascadeStep::TryNext { deployment, .. } => deployment,
            other => panic!("expected TryNext, got {other:?}"),
        };
        assert_ne!(d1.id, d2.id, "exclusion set should have filtered the first");
        // The third would need a third healthy deployment to succeed —
        // we have exactly three so step 3 should also pick a new one.
        let step3 = cascade.next_target(FailureMode::RateLimited, healthy_for_static(&deps));
        let d3 = match step3 {
            CascadeStep::TryNext { deployment, .. } => deployment,
            other => panic!("expected TryNext, got {other:?}"),
        };
        assert_ne!(d3.id, d1.id);
        assert_ne!(d3.id, d2.id);
    }

    #[test]
    fn all_fallbacks_unhealthy_when_candidates_have_no_deployments() {
        let deps = vec![dep("a", "m1", 1.0)];
        let rf = RoutingConfig {
            fallback: {
                let mut f = FallbackMap::default();
                f.fallbacks.insert("m1".into(), vec!["m2".into()]);
                f
            },
            ..Default::default()
        };
        // Note: no Deployment for "m2".
        let mut cascade = Cascade::new(rf, "m1");
        let step = cascade.next_target(FailureMode::BadConfigOrNetwork, healthy_for_static(&deps));
        assert!(matches!(
            step,
            CascadeStep::Exhausted {
                reason: ExhaustionReason::AllFallbacksUnhealthy,
                ..
            }
        ));
    }

    #[test]
    fn drained_weight_zero_filtered_out() {
        let d = dep("a", "m2", 0.0);
        let deps = vec![dep("x", "m1", 1.0), d.clone()];
        let rf = RoutingConfig {
            fallback: {
                let mut f = FallbackMap::default();
                f.fallbacks.insert("m1".into(), vec!["m2".into()]);
                f
            },
            ..Default::default()
        };
        let mut cascade = Cascade::new(rf, "m1");
        let step = cascade.next_target(FailureMode::BadConfigOrNetwork, healthy_for_static(&deps));
        // m2 has 0 weight → no healthy → AllFallbacksUnhealthy.
        assert!(matches!(
            step,
            CascadeStep::Exhausted {
                reason: ExhaustionReason::AllFallbacksUnhealthy,
                ..
            }
        ));
        // Silence unused mut warning while keeping the mut pattern
        // for symmetry with other tests that mutate.
        let _ = d;
    }

    #[test]
    fn failure_mode_bucket_mapping() {
        assert_eq!(FailureMode::RateLimited.bucket(), FallbackBucket::Generic);
        assert_eq!(
            FailureMode::BadConfigOrNetwork.bucket(),
            FallbackBucket::Generic
        );
        assert_eq!(
            FailureMode::ContextWindowOverflow.bucket(),
            FallbackBucket::ContextWindow
        );
        assert_eq!(
            FailureMode::ContentPolicyRefusal.bucket(),
            FallbackBucket::ContentPolicy
        );
    }

    #[test]
    fn ordered_fallback_models_dedupes() {
        let mut map = FallbackMap::default();
        map.fallbacks
            .insert("p".into(), vec!["a".into(), "b".into()]);
        let defaults = vec!["b".into(), "c".into()];
        let got = ordered_fallback_models(&map, FailureMode::BadConfigOrNetwork, "p", &defaults);
        assert_eq!(got, vec!["a", "b", "c"]);
    }

    #[test]
    fn pick_via_strategy_returns_none_for_empty() {
        let mut rng = StdRng::seed_from_u64(1);
        let strat = RoutingStrategy::SimpleShuffle;
        assert!(pick_via_strategy(&strat, &[], &mut rng).is_none());
    }

    #[test]
    fn pick_via_strategy_picks_one_for_simple_shuffle() {
        let d = dep("a", "m1", 1.0);
        let eligible: Vec<&Deployment> = vec![&d];
        let mut rng = StdRng::seed_from_u64(1);
        let strat = RoutingStrategy::SimpleShuffle;
        let got = pick_via_strategy(&strat, &eligible, &mut rng);
        assert_eq!(got.map(|x| x.id.clone()), Some(d.id));
    }
}
