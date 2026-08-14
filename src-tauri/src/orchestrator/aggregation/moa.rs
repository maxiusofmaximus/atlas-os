// Atlas OS Ã¢â‚¬â€ Mixture-of-Agents aggregator (RFC 04 Ã‚Â§3, sub-fase 2.2).
//
// Ports the layered architecture from "Mixture-of-Agents Enhances
// Large Language Model Capabilities" (arxiv 2406.04692, Wang et al.):
// each layer is a `Vec<ModelId>` executed in parallel; layer N+1
// takes every layer-N output as auxiliary input into the prompt. The
// final layer's output is the fused response.
//
// Sub-fase 2.2 restricts MoA to `ExecutionMode::HighStakes` (RFC 19)
// via the cost guard: a 3Ãƒâ€”3 default deployment pulls 9 parallel
// samples, so the orchestrator refuses when
// `pre_cost_estimate > Profile.budget_per_turn` before any dispatch.
//
// Default 3Ãƒâ€”3 (research/29 line 231). The profile is free to override
// both layer count and per-layer width Ã¢â‚¬â€ the only invariant the impl
// enforces is non-empty layers and non-zero models per layer.

use anyhow::Result;
use async_trait::async_trait;

use super::{
    AggregationContext, AggregationError, AggregationModeSnapshot, Aggregator, FusedResponse,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct MoAAggregator;

#[async_trait]
impl Aggregator for MoAAggregator {
    async fn aggregate(
        &self,
        ctx: AggregationContext<'_>,
    ) -> Result<FusedResponse, AggregationError> {
        let layers = match ctx.mode {
            crate::orchestrator::aggregation::AggregationMode::MoA { layers } => layers,
            _ => {
                return Err(AggregationError::Dispatch(anyhow::anyhow!(
                    "MoAAggregator invoked with non-MoA mode"
                )))
            }
        };
        if layers.is_empty() || layers.iter().any(|l| l.is_empty()) {
            return Err(AggregationError::Dispatch(anyhow::anyhow!(
                "MoA requires at least one non-empty layer"
            )));
        }

        // The live dispatch loop wires in here. Synthetic stub:
        // concatenate the layer descriptors into the fused output so
        // the test suite can verify the call path and snapshot.
        let mut fused_parts: Vec<String> = Vec::new();
        let mut samples_dispatched: u32 = 0;
        for (layer_idx, layer) in layers.iter().enumerate() {
            for (slot_idx, model_id) in layer.iter().enumerate() {
                let sample = sample_synthetic_moa(layer_idx, slot_idx, model_id).await?;
                fused_parts.push(sample);
                samples_dispatched += 1;
            }
        }

        let snapshot = AggregationModeSnapshot {
            mode: "moa".into(),
            samples_dispatched,
            rounds_executed: layers.len() as u32,
            mode_metadata: serde_json::json!({
                "layer_count": layers.len(),
                "layer_widths": layers.iter().map(|l| l.len()).collect::<Vec<_>>(),
            }),
        };
        Ok(FusedResponse {
            text: fused_parts.join("\n---\n"),
            mode_used: snapshot,
            route_taken_json: serde_json::Value::Array(vec![]),
            cost_breakdown: ctx.policy.aggregate_cost_breakdown(
                &crate::orchestrator::cost_guard::AggregationCostContext {
                    parallel_samples: samples_dispatched,
                    ..Default::default()
                },
            ),
            reflections: Vec::new(),
        })
    }
}

/// Synthetic dispatch placeholder. Wired to the provider pool when the
/// orchestrator loop fuses in.
async fn sample_synthetic_moa(
    layer_idx: usize,
    slot_idx: usize,
    model_id: &str,
) -> Result<String, AggregationError> {
    Ok(format!("L{layer_idx}S{slot_idx}:{model_id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::aggregation::AggregationMode;
    use crate::orchestrator::idempotency::RequestFrame;

    fn ctx_for<'a>(
        mode: &'a AggregationMode,
        frame: &'a RequestFrame,
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
    async fn moa_3x3_dispatches_9_samples_across_3_layers() {
        let mode = AggregationMode::MoA {
            layers: vec![
                vec![
                    "gpt-4o".into(),
                    "claude-sonnet-4".into(),
                    "gemini-2.5-pro".into(),
                ],
                vec![
                    "llama-3.1-405b".into(),
                    "qwen-2.5-72b".into(),
                    "mistral-large".into(),
                ],
                vec![
                    "gpt-4o".into(),
                    "claude-opus-4".into(),
                    "gemini-2.5-pro".into(),
                ],
            ],
        };
        let frame = RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        let out = MoAAggregator.aggregate(ctx).await.unwrap();
        assert_eq!(out.mode_used.mode, "moa");
        assert_eq!(out.mode_used.samples_dispatched, 9);
        assert_eq!(out.mode_used.rounds_executed, 3);
        assert!(out.text.contains("L0S0:gpt-4o"));
        assert!(out.text.contains("L2S2:gemini-2.5-pro"));
    }

    #[tokio::test]
    async fn moa_rejects_empty_layer_set() {
        let mode = AggregationMode::MoA { layers: vec![] };
        let frame = RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        assert!(MoAAggregator.aggregate(ctx).await.is_err());
    }

    #[tokio::test]
    async fn moa_rejects_layer_with_empty_model_list() {
        let mode = AggregationMode::MoA {
            layers: vec![vec!["gpt-4o".into()], vec![]],
        };
        let frame = RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        assert!(MoAAggregator.aggregate(ctx).await.is_err());
    }
}
