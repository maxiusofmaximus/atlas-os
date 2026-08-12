// OpenCode OS Ã¢â‚¬â€ Council aggregator (RFC 04 Ã‚Â§3, sub-fase 2.2).
//
// Ports "Improving Factuality and Reasoning in Language Models through
// Multiagent Debate" (arxiv 2305.14325, Du et al.). Each agent
// proposes an initial claim, then critiques the previous round's
// leading claim on subsequent rounds. Final fused response is the
// most-frequent round-`rounds` claim (or the first-debater claim when
// no majority Ã¢â‚¬â€ left to caller discretion).
//
// Sub-fase 2.2 caps `rounds` at 3 (CHECK constraint on the
// `council_votes` table, M22) but defaults to 1 round per
// research/29 line 232 (cost control: 2-3 debaters Ãƒâ€” 1 round = 2-3
// parallel samples instead of Ãƒâ€”N for N rounds).

use anyhow::Result;
use async_trait::async_trait;

use super::{
    AggregationContext, AggregationError, AggregationModeSnapshot, Aggregator, FusedResponse,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct CouncilAggregator;

#[async_trait]
impl Aggregator for CouncilAggregator {
    async fn aggregate(
        &self,
        ctx: AggregationContext<'_>,
    ) -> Result<FusedResponse, AggregationError> {
        let (debaters, rounds) = match ctx.mode {
            crate::orchestrator::aggregation::AggregationMode::Council { debaters, rounds } => {
                (debaters.clone(), *rounds)
            }
            _ => {
                return Err(AggregationError::Dispatch(anyhow::anyhow!(
                    "CouncilAggregator invoked with non-Council mode"
                )))
            }
        };
        if debaters.is_empty() {
            return Err(AggregationError::Dispatch(anyhow::anyhow!(
                "Council requires at least one debater"
            )));
        }
        if rounds == 0 || rounds > 3 {
            return Err(AggregationError::Dispatch(anyhow::anyhow!(
                "Council rounds must be in 1..=3, got {}",
                rounds
            )));
        }

        // Synthetic dispatch Ã¢â‚¬â€ the live loop wires the provider pool here.
        // For each round, every debater emits a claim. Round 1: initial
        // claims. Round 2+: critique_of_prev + revised claim.
        let mut all_claims: Vec<Vec<String>> = Vec::with_capacity(rounds as usize);
        let mut samples_dispatched: u32 = 0;
        for round in 1..=rounds {
            let mut round_claims: Vec<String> = Vec::with_capacity(debaters.len());
            for (slot, debater_model) in debaters.iter().enumerate() {
                let claim = sample_synthetic_council(round, slot, debater_model).await?;
                round_claims.push(claim);
                samples_dispatched += 1;
            }
            all_claims.push(round_claims);
        }

        let final_round = all_claims.last().cloned().unwrap_or_default();
        let winner = mode_winner_str(&final_round)
            .unwrap_or_else(|| final_round.first().cloned().unwrap_or_default());
        let snapshot = AggregationModeSnapshot {
            mode: "council".into(),
            samples_dispatched,
            rounds_executed: rounds as u32,
            mode_metadata: serde_json::json!({
                "debater_count": debaters.len(),
                "rounds": rounds,
                "final_winner_claim_len": winner.len(),
            }),
        };
        Ok(FusedResponse {
            text: winner,
            mode_used: snapshot,
            route_taken_json: serde_json::Value::Array(vec![]),
            cost_breakdown: ctx.policy.aggregate_cost_breakdown(
                &crate::orchestrator::cost_guard::AggregationCostContext {
                    parallel_samples: debaters.len() as u32,
                    rounds: rounds as u32,
                    ..Default::default()
                },
            ),
            reflections: Vec::new(),
        })
    }
}

async fn sample_synthetic_council(
    round: u8,
    slot: usize,
    debater: &str,
) -> Result<String, AggregationError> {
    Ok(format!("r{round}d{slot}:{debater}"))
}

/// Reuse the canonical winner picker. Unanimous on round 1 keeps the
/// same debater's claim; majority on later rounds picks the most common.
pub(crate) fn mode_winner_str(claims: &[String]) -> Option<String> {
    let mut counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for c in claims {
        *counts.entry(c.as_str()).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(k, _)| k.to_string())
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
    async fn council_3_debaters_1_round_dispatches_3_samples() {
        let mode = AggregationMode::Council {
            debaters: vec![
                "gpt-4o".into(),
                "claude-opus-4".into(),
                "gemini-2.5-pro".into(),
            ],
            rounds: 1,
        };
        let frame = crate::orchestrator::idempotency::RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        let out = CouncilAggregator.aggregate(ctx).await.unwrap();
        assert_eq!(out.mode_used.mode, "council");
        assert_eq!(out.mode_used.samples_dispatched, 3);
        assert_eq!(out.mode_used.rounds_executed, 1);
    }

    #[tokio::test]
    async fn council_2_debaters_3_rounds_dispatches_6_samples() {
        let mode = AggregationMode::Council {
            debaters: vec!["gpt-4o".into(), "claude-opus-4".into()],
            rounds: 3,
        };
        let frame = crate::orchestrator::idempotency::RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        let out = CouncilAggregator.aggregate(ctx).await.unwrap();
        assert_eq!(out.mode_used.samples_dispatched, 6);
        assert_eq!(out.mode_used.rounds_executed, 3);
    }

    #[tokio::test]
    async fn council_rejects_zero_rounds() {
        let mode = AggregationMode::Council {
            debaters: vec!["gpt-4o".into()],
            rounds: 0,
        };
        let frame = crate::orchestrator::idempotency::RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        assert!(CouncilAggregator.aggregate(ctx).await.is_err());
    }

    #[tokio::test]
    async fn council_rejects_more_than_3_rounds() {
        let mode = AggregationMode::Council {
            debaters: vec!["gpt-4o".into()],
            rounds: 4,
        };
        let frame = crate::orchestrator::idempotency::RequestFrame::new();
        let route = crate::orchestrator::aggregation::empty_route_ctx();
        let ctx = ctx_for(&mode, &frame, &route);
        assert!(CouncilAggregator.aggregate(ctx).await.is_err());
    }
}
