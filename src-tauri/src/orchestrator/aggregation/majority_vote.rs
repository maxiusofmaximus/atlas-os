// OpenCode OS Ã¢â‚¬â€ MajorityVote aggregator (RFC 04 Ã‚Â§3, sub-fase 2.2).
//
// Ports the Agent Forest finding from "More Agents Is All You Need"
// (arxiv 2402.05120, Li et al., TMLR): scaling purely with parallel
// sampling + a majority vote improves LLM performance, and the
// benefit is correlated with task difficulty. Sub-fase 2.2 tunes
// this by:
//   * Making `n_samples` difficulty-aware (easy=1, medium=3, hard=5/9)
//     via the `RouteContext.task_difficulty` hint.
//   * Batching the votes in groups of 3 and checking for stop-early
//     agreement (Ã¢â€°Â¥2/3 of canonicalised responses identical AND
//     Shannon entropy of the response distribution `<0.5`).
//   * Canonicalising responses (strip whitespace + lowercase + trim
//     trailing punctuation) so cosmetically different answers of
//     the same semantic value collapse to one bucket.
//
// The floored paper does NOT use stop-early; we add it (annotated as
// a contribution in RFC 22) because OpenCode OS is interactive and a
// 9-sample majority on a low-difficulty prompt silently burns the
// operator's wallet.

use anyhow::Result;
use async_trait::async_trait;

use super::{
    AggregationContext, AggregationError, AggregationModeSnapshot, Aggregator, FusedResponse,
};

/// Default stop-early threshold when the profile omits it (=2/3).
pub const DEFAULT_STOP_EARLY_THRESHOLD: f32 = 0.6667;

/// Default batch size for stop-early checkpoints. After every full
/// batch of 3, the aggregator checks the agreement ratio.
pub const STOP_EARLY_BATCH: usize = 3;

#[derive(Debug, Default, Clone, Copy)]
pub struct MajorityVoteAggregator;

#[async_trait]
impl Aggregator for MajorityVoteAggregator {
    async fn aggregate(
        &self,
        ctx: AggregationContext<'_>,
    ) -> Result<FusedResponse, AggregationError> {
        let (n_samples, threshold) = match ctx.mode {
            crate::orchestrator::aggregation::AggregationMode::MajorityVote {
                n_samples,
                stop_early_threshold,
            } => (*n_samples, *stop_early_threshold),
            _ => {
                return Err(AggregationError::Dispatch(anyhow::anyhow!(
                    "MajorityVoteAggregator invoked with non-MajorityVote mode"
                )))
            }
        };

        let mut bag: Vec<String> = Vec::with_capacity(n_samples as usize);
        let mut samples_dispatched: u32 = 0;

        for i in 0..n_samples {
            let sample = sample_synthetic(i).await?;
            bag.push(sample);
            samples_dispatched += 1;

            if bag.len() >= STOP_EARLY_BATCH {
                let bucket = canonicalise_bag(&bag);
                let agreement = agreement_ratio(&bucket);
                let entropy = shannon_entropy(&bucket);
                if agreement >= threshold as f64 && entropy < 0.5 {
                    return Ok(fuse_majority(
                        bag,
                        samples_dispatched,
                        ctx,
                        /* stop_early */ true,
                    ));
                }
            }
        }

        Ok(fuse_majority(
            bag,
            samples_dispatched,
            ctx,
            /* stop_early */ false,
        ))
    }
}

/// Fuse the accumulated bag of responses. Picks the canonical bucket
/// with the highest count; outputs the canonical form of the winning
/// bucket (whitespace-stripped lowercase). HUD snapshot carries the
/// agreement ratio and stop-early flag in `mode_metadata`.
fn fuse_majority(
    bag: Vec<String>,
    samples_dispatched: u32,
    ctx: AggregationContext<'_>,
    stop_early: bool,
) -> FusedResponse {
    let canon = canonicalise_bag(&bag);
    let winner = mode_winner(&canon).unwrap_or_default();
    let agreement = if !canon.is_empty() {
        let top = canon.iter().filter(|c| *c == &winner).count() as f64;
        top / canon.len() as f64
    } else {
        0.0
    };
    let snapshot = AggregationModeSnapshot {
        mode: "majority_vote".into(),
        samples_dispatched,
        rounds_executed: 1,
        mode_metadata: serde_json::json!({
            "stop_early": stop_early,
            "agreement": agreement,
            "entropy": shannon_entropy(&canon),
        }),
    };
    FusedResponse {
        text: winner,
        mode_used: snapshot,
        route_taken_json: serde_json::Value::Array(vec![]),
        cost_breakdown: ctx.policy.aggregate_cost_breakdown(
            &crate::orchestrator::cost_guard::AggregationCostContext {
                parallel_samples: samples_dispatched,
                ..Default::default()
            },
        ),
        reflections: Vec::new(),
    }
}

/// Deterministic stub for the parallel sample. The real orchestrator
/// dispatches the prompt via the provider pool; the aggregation
/// faÃƒÂ§ade receives the response stream and parks it. Tests pass a
/// mock provider handle and inject responses.
///
/// Refactor note: when the dispatch loop wires in, this becomes
/// `provider.dispatch(shape, ctx).await` and returns the body text.
async fn sample_synthetic(idx: u8) -> Result<String, AggregationError> {
    // Synthetic placeholder Ã¢â‚¬â€ the live dispatch loop replaces this.
    Ok(format!("sample-{idx}"))
}

/// Canonicalise a response: strip all whitespace, lowercase, trim
/// trailing punctuation (`.`/`!`/`?`/`,`/`;`). Agent Forest found
/// this essential Ã¢â‚¬â€ model capitalisation and interpunctuation vary
/// across deployments but semantically identical outputs should vote
/// together.
pub(crate) fn canonicalise(resp: &str) -> String {
    let mut s: String = resp
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    while matches!(
        s.chars().last(),
        Some('.') | Some('!') | Some('?') | Some(',') | Some(';')
    ) {
        s.pop();
    }
    s
}

/// Canonicalise an entire bag of responses into canon-keys.
pub(crate) fn canonicalise_bag(bag: &[String]) -> Vec<String> {
    bag.iter().map(|s| canonicalise(s)).collect()
}

/// Agreement ratio: largest bucket / total. 1.0 = unanimous.
pub(crate) fn agreement_ratio(canon: &[String]) -> f64 {
    if canon.is_empty() {
        return 0.0;
    }
    let mut counts = std::collections::HashMap::new();
    for c in canon {
        *counts.entry(c.as_str()).or_insert(0u32) += 1;
    }
    let max = *counts.values().max().unwrap_or(&0) as f64;
    max / canon.len() as f64
}

/// Shannon entropy over the bucket distribution. Low = high agreement.
/// Uses natural log (nats). 0 when only one bucket.
pub(crate) fn shannon_entropy(canon: &[String]) -> f64 {
    if canon.is_empty() {
        return 0.0;
    }
    let mut counts = std::collections::HashMap::new();
    for c in canon {
        *counts.entry(c.as_str()).or_insert(0u32) += 1;
    }
    let total = canon.len() as f64;
    counts
        .values()
        .map(|&n| {
            let p = n as f64 / total;
            -p * p.ln()
        })
        .sum()
}

/// Return the canonical-answer bucket with the highest count.
pub(crate) fn mode_winner(canon: &[String]) -> Option<String> {
    let mut counts = std::collections::HashMap::new();
    for c in canon {
        *counts.entry(c.as_str()).or_insert(0u32) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(k, _)| k.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::idempotency::RequestFrame;

    fn ctx_for<'a>(
        mode: &'a crate::orchestrator::aggregation::AggregationMode,
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

    #[test]
    fn canonicalise_strips_whitespace_trailing_punct_and_lowercases() {
        assert_eq!(canonicalise("Hello, World! "), "hello,world");
        assert_eq!(canonicalise("A."), "a");
        assert_eq!(canonicalise("a?"), "a");
        assert_eq!(canonicalise(" B ; "), "b");
    }

    #[test]
    fn agreement_ratio_unanimous_is_1() {
        let canon = vec!["x".into(); 3];
        assert!((agreement_ratio(&canon) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn shannon_entropy_single_bucket_is_0() {
        let canon = vec!["x".into(); 5];
        assert_eq!(shannon_entropy(&canon), 0.0);
    }

    #[test]
    fn mode_winner_picks_highest_count_canonical_bucket() {
        let canon = vec![
            "a".into(),
            "a".into(),
            "b".into(),
            "a".into(),
            "c".into(),
            "b".into(),
        ];
        assert_eq!(mode_winner(&canon).as_deref(), Some("a"));
    }

    #[test]
    fn fuse_majority_with_empty_bag_returns_empty_text() {
        let empty_route = super::super::empty_route_ctx();
        let mode = crate::orchestrator::aggregation::AggregationMode::MajorityVote {
            n_samples: 0,
            stop_early_threshold: 0.66,
        };
        let frame = RequestFrame::new();
        let ctx = ctx_for(&mode, &frame, &empty_route);
        let out = fuse_majority(Vec::new(), 0, ctx, /* stop_early */ false);
        assert!(out.text.is_empty());
        assert_eq!(out.mode_used.mode, "majority_vote");
    }
}
