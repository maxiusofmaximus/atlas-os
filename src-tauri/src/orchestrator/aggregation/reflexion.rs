// Atlas OS Ã¢â‚¬â€ Reflexion aggregator (RFC 04 Ã‚Â§3, sub-fase 2.2).
//
// Ports "Reflexion: Language Agents with Verbal Reinforcement
// Learning" (arxiv 2303.11366, Shinn et al.). The paper uses a single
// LLM as both executor and reflexor; sub-fase 2.2 extends this to
// multi-model (executor caro + reflexor barato) as an Atlas OS
// contribution Ã¢â‚¬â€ annotated in RFC 22 Ã‚Â§2 because the paper does not
// validate this variant.
//
// Anti-doom-loop guard (RFC 19): on every episode, the orchestrator
// queries the two most recent `reflection_episodes` rows for the same
// `mission_id`; if both have the same `failure_signal` hash, the
// aggregator aborts with `AggregationError::DoomLoop` and escalates
// to a human via the Supervisor (RFC 19 Ã‚Â§6 idempotency + supervisor
// escalation chain).
//
// `attempt_no` is hard-capped at 3 by the M22 CHECK constraint.

use anyhow::Result;
use async_trait::async_trait;

use super::{
    AggregationContext, AggregationError, AggregationModeSnapshot, Aggregator, FusedResponse,
    ReflectionEpisodeOut,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct ReflexionAggregator;

#[async_trait]
impl Aggregator for ReflexionAggregator {
    async fn aggregate(
        &self,
        ctx: AggregationContext<'_>,
    ) -> Result<FusedResponse, AggregationError> {
        let memory_buffer_size = match ctx.mode {
            crate::orchestrator::aggregation::AggregationMode::Reflexion { memory_buffer_size } => {
                *memory_buffer_size
            }
            _ => {
                return Err(AggregationError::Dispatch(anyhow::anyhow!(
                    "ReflexionAggregator invoked with non-Reflexion mode"
                )))
            }
        };
        if memory_buffer_size == 0 {
            return Err(AggregationError::Dispatch(anyhow::anyhow!(
                "Reflexion memory_buffer_size must be > 0"
            )));
        }

        // Synthetic 1-episode dispatch Ã¢â‚¬â€ the live loop wires the
        // provider pool and writes to `reflection_episodes` (M22).
        let episode = ReflectionEpisodeOut {
            episode_id: uuid::Uuid::new_v4(),
            mission_id: ctx.request_frame.idempotency_key, // reframe as mission in the live loop
            attempt_no: 1,
            executor_model: "gpt-4o".into(),
            reflexor_model: "haiku".into(),
            failure_signal: "sig-1".into(),
            verbal_reflection: "Need to reconsider edge cases.".into(),
        };
        let snapshot = AggregationModeSnapshot {
            mode: "reflexion".into(),
            samples_dispatched: 1,
            rounds_executed: 1,
            mode_metadata: serde_json::json!({
                "memory_buffer_size": memory_buffer_size,
                "max_attempts": 3,
            }),
        };
        Ok(FusedResponse {
            text: "reflexion-fused-output".into(),
            mode_used: snapshot,
            route_taken_json: serde_json::Value::Array(vec![]),
            cost_breakdown: ctx.policy.aggregate_cost_breakdown(
                &crate::orchestrator::cost_guard::AggregationCostContext {
                    parallel_samples: 1,
                    rounds: 1,
                    reflexion_memory_tokens: memory_buffer_size as u64,
                    ..Default::default()
                },
            ),
            reflections: vec![episode],
        })
    }
}

/// Anti-doom-loop detector. Returns `Some(signal)` when the same
/// `failure_signal` appears in 2 consecutive episodes for the same
/// mission; returns `None` otherwise. The orchestrator calls this
/// before dispatching `attempt_no + 1` and aborts with
/// `AggregationError::DoomLoop` when it returns `Some`.
pub fn detect_doom_loop(history: &[&str]) -> Option<String> {
    if history.len() < 2 {
        return None;
    }
    let last_two: Vec<&str> = history.iter().rev().take(2).copied().collect();
    if last_two[0] == last_two[1] {
        Some(last_two[0].to_string())
    } else {
        None
    }
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
    async fn reflexion_1_attempt_succeeds_with_episode_record() {
        let mode = AggregationMode::Reflexion {
            memory_buffer_size: 2,
        };
        let frame = crate::orchestrator::idempotency::RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        let out = ReflexionAggregator.aggregate(ctx).await.unwrap();
        assert_eq!(out.mode_used.mode, "reflexion");
        assert_eq!(out.reflections.len(), 1);
        assert_eq!(out.reflections[0].attempt_no, 1);
    }

    #[tokio::test]
    async fn reflexion_rejects_zero_memory_buffer() {
        let mode = AggregationMode::Reflexion {
            memory_buffer_size: 0,
        };
        let frame = crate::orchestrator::idempotency::RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        assert!(ReflexionAggregator.aggregate(ctx).await.is_err());
    }

    #[test]
    fn detect_doom_loop_returns_none_for_empty_history() {
        assert!(detect_doom_loop(&[]).is_none());
    }

    #[test]
    fn detect_doom_loop_returns_some_when_two_consecutive_match() {
        let history = vec!["sig-a", "sig-b", "sig-b"];
        assert_eq!(detect_doom_loop(&history), Some("sig-b".into()));
    }

    #[test]
    fn detect_doom_loop_returns_none_when_alternating() {
        let history = vec!["sig-a", "sig-b", "sig-a"];
        // Last two are "sig-b" and "sig-a" Ã¢â‚¬â€ different, no doom loop.
        assert!(detect_doom_loop(&history).is_none());
    }
}
