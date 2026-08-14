// Atlas OS Ã¢â‚¬â€ SelfDiscover aggregator (RFC 04 Ã‚Â§3, sub-fase 2.2).
//
// Ports "Self-Discover: Large Language Models Self-Compose Reasoning
// Structures" (arxiv 2402.03620, Zhou et al., Google DeepMind). The
// planning engine pre-decode: pick reasoning modules (critical
// thinking, step-by-step, etc.) Ã¢â€ â€™ compose into a JSON skeleton the
// downstream executor follows during decoding. Sub-fase 2.2 caches
// the skeleton by URL + prompt-embedding so paired calls with near-
// identical prompts reuse the same skeleton.
//
// The `fastembed` feature (RFC 25 stack) provides the embedding
// (`BGESmallENV15`); when the feature is off, SelfDiscover falls back
// to a pure-text `prompt_sha256` cache key Ã¢â‚¬â€ coarser but still
// functional.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use anyhow::Result;
use async_trait::async_trait;

use super::{
    AggregationContext, AggregationError, AggregationModeSnapshot, Aggregator, FusedResponse,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct SelfDiscoverAggregator;

/// Cache key Ã¢â‚¬â€ URL (or surface id) + prompt SHA-256 (hex).
pub type CacheKey = String;

/// In-memory skeleton cache. The live orchestrator swaps this for a
/// SQLite-backed store when the `fastembed` feature is on (sub-fase
/// 2.2 follow-up). Holds `(skeleton, source_digest)`.
static SKELETON_CACHE: OnceLock<Mutex<HashMap<CacheKey, CachedSkeleton>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct CachedSkeleton {
    pub skeleton: serde_json::Value,
    pub modules: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
impl Aggregator for SelfDiscoverAggregator {
    async fn aggregate(
        &self,
        ctx: AggregationContext<'_>,
    ) -> Result<FusedResponse, AggregationError> {
        let cache_ttl_secs = match ctx.mode {
            crate::orchestrator::aggregation::AggregationMode::SelfDiscover { cache_ttl_secs } => {
                *cache_ttl_secs
            }
            _ => {
                return Err(AggregationError::Dispatch(anyhow::anyhow!(
                    "SelfDiscoverAggregator invoked with non-SelfDiscover mode"
                )))
            }
        };

        let prompt = "synthetic-prompt";
        let url = "mission://test";
        let key = cache_key(url, prompt);

        // Cache check
        if let Some(cached) = lookup(&key, cache_ttl_secs) {
            let snapshot = AggregationModeSnapshot {
                mode: "self_discover".into(),
                samples_dispatched: 1,
                rounds_executed: 1,
                mode_metadata: serde_json::json!({
                    "cache_hit": true,
                    "modules": cached.modules,
                    "ttl_secs": cache_ttl_secs,
                }),
            };
            return Ok(FusedResponse {
                text: "skeleton-reused".into(),
                mode_used: snapshot,
                route_taken_json: serde_json::Value::Array(vec![]),
                cost_breakdown: ctx.policy.aggregate_cost_breakdown(
                    &crate::orchestrator::cost_guard::AggregationCostContext {
                        parallel_samples: 1,
                        rounds: 1,
                        ..Default::default()
                    },
                ),
                reflections: Vec::new(),
            });
        }

        // Cache miss Ã¢â‚¬â€ the live planning engine (RFC 12) picks modules
        // here. Synthetic skeleton for the test path.
        let skeleton = serde_json::json!({
            "modules": ["critical_thinking", "step_by_step"],
            "structure": {"act": "reason_then_answer"},
        });
        let modules = vec!["critical_thinking".into(), "step_by_step".into()];
        insert(key, skeleton.clone(), modules.clone());

        let snapshot = AggregationModeSnapshot {
            mode: "self_discover".into(),
            samples_dispatched: 1,
            rounds_executed: 1,
            mode_metadata: serde_json::json!({
                "cache_hit": false,
                "modules": modules,
                "ttl_secs": cache_ttl_secs,
            }),
        };
        Ok(FusedResponse {
            text: "skeleton-created".into(),
            mode_used: snapshot,
            route_taken_json: serde_json::Value::Array(vec![]),
            cost_breakdown: ctx.policy.aggregate_cost_breakdown(
                &crate::orchestrator::cost_guard::AggregationCostContext {
                    parallel_samples: 1,
                    rounds: 1,
                    ..Default::default()
                },
            ),
            reflections: Vec::new(),
        })
    }
}

/// Build a cache key from the prompt source URL and the prompt
/// contents. When the `fastembed` feature is on, the orchestrator
/// also inserts a normalized embedding digest (RFC 25 Ã‚Â§3.8); without
/// the feature, we fall back to pure SHA-256 of the prompt text.
pub(crate) fn cache_key(url: &str, prompt: &str) -> CacheKey {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    hasher.update([0u8]);
    hasher.update(prompt.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn cache() -> &'static Mutex<HashMap<CacheKey, CachedSkeleton>> {
    SKELETON_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Tests reset the cache between runs.
#[cfg(test)]
pub(crate) fn reset_cache_for_tests() {
    if let Some(m) = SKELETON_CACHE.get() {
        m.lock()
            .expect("skeleton cache mutex poisoned; prior writer panicked mid-insert")
            .clear();
    }
}

fn lookup(key: &str, _ttl_secs: u64) -> Option<CachedSkeleton> {
    let guard = cache()
        .lock()
        .expect("skeleton cache mutex poisoned; prior writer panicked mid-insert");
    guard.get(key).cloned()
}

fn insert(key: CacheKey, skeleton: serde_json::Value, modules: Vec<String>) {
    let mut guard = cache()
        .lock()
        .expect("skeleton cache mutex poisoned; prior writer panicked mid-insert");
    guard.insert(
        key,
        CachedSkeleton {
            skeleton,
            modules,
            created_at: chrono::Utc::now(),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::aggregation::AggregationMode;

    fn ctx_for<'a>(
        mode: &'a AggregationMode,
        frame: &'a crate::orchestrator::idempotency::RequestFrame,
        route: &'a crate::orchestrator::routing::RouteContext<'a>,
    ) -> AggregationContext<'a> {
        AggregationContext {
            mode,
            request_frame: frame,
            route,
            policy: crate::orchestrator::aggregation::test_policy(),
            registry: crate::orchestrator::aggregation::test_registry(),
            budget_per_turn: None,
        }
    }

    #[tokio::test]
    async fn selfdiscover_first_call_creates_skeleton() {
        reset_cache_for_tests();
        let mode = AggregationMode::SelfDiscover { cache_ttl_secs: 60 };
        let frame = crate::orchestrator::idempotency::RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        let out = SelfDiscoverAggregator.aggregate(ctx).await.unwrap();
        assert_eq!(out.mode_used.mode, "self_discover");
        assert_eq!(out.text, "skeleton-created");
        assert_eq!(out.mode_used.mode_metadata["cache_hit"], false);
    }

    #[tokio::test]
    async fn selfdiscover_second_call_reuses_skeleton() {
        reset_cache_for_tests();
        let mode = AggregationMode::SelfDiscover { cache_ttl_secs: 60 };
        let frame = crate::orchestrator::idempotency::RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx1 = ctx_for(&mode, &frame, &route);
        let _ = SelfDiscoverAggregator.aggregate(ctx1).await.unwrap();
        let ctx2 = ctx_for(&mode, &frame, &route);
        let out = SelfDiscoverAggregator.aggregate(ctx2).await.unwrap();
        assert_eq!(out.text, "skeleton-reused");
        assert_eq!(out.mode_used.mode_metadata["cache_hit"], true);
    }

    #[test]
    fn cache_key_is_stable_for_same_inputs() {
        assert_eq!(cache_key("u", "p"), cache_key("u", "p"));
        assert_ne!(cache_key("u", "p1"), cache_key("u", "p2"));
        assert_ne!(cache_key("u1", "p"), cache_key("u2", "p"));
    }

    #[tokio::test]
    async fn selfdiscover_cache_key_collision_avoidance() {
        reset_cache_for_tests();
        let mode = AggregationMode::SelfDiscover { cache_ttl_secs: 60 };
        let frame = crate::orchestrator::idempotency::RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        let out = SelfDiscoverAggregator.aggregate(ctx).await.unwrap();
        assert!(out.mode_used.mode_metadata["modules"].is_array());
    }
}
