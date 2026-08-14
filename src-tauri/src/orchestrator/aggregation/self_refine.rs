// Atlas OS Ã¢â‚¬â€ SelfRefine aggregator (RFC 04 Ã‚Â§3, sub-fase 2.2).
//
// Ports "Self-Refine: Iterative Refinement with Self-Feedback" (arxiv
// 2303.17651, Madaan et al.). A single LLM acts as generator,
// feedback-provider, and refiner in sequence. Each iteration produces
// `feedback Ã¢â€ â€™ refined_output`. Sub-fase 2.2 caps iterations at 2
// (research/29 line 234) and aborts early when the line-diff between
// consecutive iterations drops below `StopCondition::MinDeltaLines.lines`
// Ã¢â‚¬â€ the model has converged and further refinement would burn budget.

use anyhow::Result;
use async_trait::async_trait;

use super::{
    AggregationContext, AggregationError, AggregationModeSnapshot, Aggregator, FusedResponse,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct SelfRefineAggregator;

#[async_trait]
impl Aggregator for SelfRefineAggregator {
    async fn aggregate(
        &self,
        ctx: AggregationContext<'_>,
    ) -> Result<FusedResponse, AggregationError> {
        let (max_iterations, stop_condition) = match ctx.mode {
            crate::orchestrator::aggregation::AggregationMode::SelfRefine {
                max_iterations,
                stop_condition,
            } => (*max_iterations, stop_condition.clone()),
            _ => {
                return Err(AggregationError::Dispatch(anyhow::anyhow!(
                    "SelfRefineAggregator invoked with non-SelfRefine mode"
                )))
            }
        };
        if max_iterations == 0 || max_iterations > 5 {
            return Err(AggregationError::Dispatch(anyhow::anyhow!(
                "SelfRefine max_iterations must be in 1..=5, got {}",
                max_iterations
            )));
        }

        let threshold = match stop_condition {
            super::StopCondition::MinDeltaLines { lines } => lines,
            super::StopCondition::MaxIterations { iters: _ } => u32::MAX,
            super::StopCondition::ConvergenceThreshold { threshold: _ } => 0,
        };

        // Synthetic refinement loop Ã¢â‚¬â€ the live loop wires the provider
        // dispatch. Each iteration emits a synthetic text whose line
        // count is constant, so the diff threshold triggers on the
        // first check when threshold > 0.
        let mut samples_dispatched: u32 = 0;
        let mut last_text: String = String::new();
        let mut rounds_executed: u32 = 0;
        for iter in 1..=max_iterations {
            let candidate = sample_synthetic_selfrefine(iter, &last_text).await?;
            samples_dispatched += 1;
            rounds_executed = iter as u32;
            if iter > 1 {
                let delta = line_delta(&last_text, &candidate);
                if delta < threshold {
                    return Err(AggregationError::SelfRefineStalled { delta, threshold });
                }
            }
            last_text = candidate;
        }

        let snapshot = AggregationModeSnapshot {
            mode: "self_refine".into(),
            samples_dispatched,
            rounds_executed,
            mode_metadata: serde_json::json!({
                "max_iterations": max_iterations,
                "stop_condition": format!("{:?}", stop_condition),
                "final_lines": last_text.lines().count(),
            }),
        };
        Ok(FusedResponse {
            text: last_text,
            mode_used: snapshot,
            route_taken_json: serde_json::Value::Array(vec![]),
            cost_breakdown: ctx.policy.aggregate_cost_breakdown(
                &crate::orchestrator::cost_guard::AggregationCostContext {
                    parallel_samples: 1,
                    rounds: rounds_executed,
                    ..Default::default()
                },
            ),
            reflections: Vec::new(),
        })
    }
}

async fn sample_synthetic_selfrefine(iter: u8, prev: &str) -> Result<String, AggregationError> {
    if prev.is_empty() {
        Ok("line A\nline B\nline C\nline D\nline E\nline F\nline G\nline H\nline I\nline J".into())
    } else {
        Ok(format!("{prev}\nrefine iter {iter}"))
    }
}

/// Line diff Ã¢â‚¬â€ symmetrically added/removed lines between `a` and `b`.
pub(crate) fn line_delta(a: &str, b: &str) -> u32 {
    let set_a: std::collections::HashSet<&str> = a.lines().collect();
    let set_b: std::collections::HashSet<&str> = b.lines().collect();
    let removed = set_a.difference(&set_b).count() as u32;
    let added = set_b.difference(&set_a).count() as u32;
    removed + added
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::aggregation::{AggregationMode, StopCondition};

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
    async fn selfrefine_2_iterations_happy_path() {
        let mode = AggregationMode::SelfRefine {
            max_iterations: 2,
            stop_condition: StopCondition::MinDeltaLines { lines: 0 },
        };
        let frame = crate::orchestrator::idempotency::RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        let out = SelfRefineAggregator.aggregate(ctx).await.unwrap();
        assert_eq!(out.mode_used.mode, "self_refine");
        assert_eq!(out.mode_used.samples_dispatched, 2);
        assert_eq!(out.mode_used.rounds_executed, 2);
    }

    #[tokio::test]
    async fn selfrefine_stalls_when_delta_below_threshold() {
        let mode = AggregationMode::SelfRefine {
            max_iterations: 3,
            stop_condition: StopCondition::MinDeltaLines { lines: 100 },
        };
        let frame = crate::orchestrator::idempotency::RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        let err = SelfRefineAggregator.aggregate(ctx).await.unwrap_err();
        match err {
            AggregationError::SelfRefineStalled { delta, threshold } => {
                assert_eq!(threshold, 100);
                assert!(delta < threshold);
            }
            other => panic!("expected SelfRefineStalled, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn selfrefine_rejects_zero_iterations() {
        let mode = AggregationMode::SelfRefine {
            max_iterations: 0,
            stop_condition: StopCondition::MinDeltaLines { lines: 0 },
        };
        let frame = crate::orchestrator::idempotency::RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        assert!(SelfRefineAggregator.aggregate(ctx).await.is_err());
    }

    #[test]
    fn line_delta_counts_added_and_removed() {
        let a = "alpha\nbeta\ngamma";
        let b = "beta\ngamma\ndelta";
        // Removed: "alpha" (1), Added: "delta" (1) Ã¢â‚¬â€ total 2.
        assert_eq!(line_delta(a, b), 2);
        assert_eq!(line_delta(a, a), 0);
    }
}
