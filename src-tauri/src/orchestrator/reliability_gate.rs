// Atlas OS — Model reliability gate (RFC 20 Fase 24 v24.0; research/53).
//
// Pure gate over historical eval reliability (`crate::eval::metrics::Reliability`):
// before routing to a model, drop deployments whose model has enough eval
// samples but a pass rate below a threshold. The `n >= min_samples` guard is
// deliberate (research/51 §1 — "trust calibration mismatch"): act only on
// statistically meaningful history, never degrade on noise.
//
// The gate is opt-in and not wired into routing yet (that is v24.1); this module
// is pure and unit- + golden-tested so the wiring lands on a proven core.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::eval::metrics::Reliability;
use crate::journal::Journal;
use crate::orchestrator::provider::Deployment;

/// Policy for gating deployments by historical eval reliability.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReliabilityGate {
    /// Minimum eval cases before a model's reliability is actionable.
    pub min_samples: i64,
    /// Drop a model whose `pass_rate` is below this (0..=1).
    pub min_pass_rate: f64,
    /// When true, a model with no eval history is allowed (default).
    pub allow_unknown: bool,
}

impl Default for ReliabilityGate {
    fn default() -> Self {
        Self {
            min_samples: 20,
            min_pass_rate: 0.5,
            allow_unknown: true,
        }
    }
}

/// Decision for one model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateDecision {
    Allow,
    Deny(&'static str),
}

/// Decide whether `model_id` may be routed to, given its historical reliability.
pub fn gate_model(
    model_id: &str,
    reliabilities: &HashMap<String, Reliability>,
    gate: &ReliabilityGate,
) -> GateDecision {
    match reliabilities.get(model_id) {
        None => {
            if gate.allow_unknown {
                GateDecision::Allow
            } else {
                GateDecision::Deny("no eval history")
            }
        }
        // Not enough history to act on — do not degrade on noise.
        Some(r) if r.n < gate.min_samples => GateDecision::Allow,
        Some(r) if r.pass_rate < gate.min_pass_rate => {
            GateDecision::Deny("pass_rate below threshold")
        }
        Some(_) => GateDecision::Allow,
    }
}

/// Partition deployments into (allowed, denied-with-reason) using `gate_model`.
pub fn filter_deployments<'a>(
    deployments: &'a [Deployment],
    reliabilities: &HashMap<String, Reliability>,
    gate: &ReliabilityGate,
) -> (Vec<&'a Deployment>, Vec<(&'a Deployment, &'static str)>) {
    let mut allowed = Vec::new();
    let mut denied = Vec::new();
    for deployment in deployments {
        match gate_model(&deployment.model_id, reliabilities, gate) {
            GateDecision::Allow => allowed.push(deployment),
            GateDecision::Deny(reason) => denied.push((deployment, reason)),
        }
    }
    (allowed, denied)
}

/// Build the `model_id → Reliability` map from the eval store for `models`.
/// This is the host glue that turns the Fase-22 EVAL metrics into routing
/// signal; models never evaluated are simply absent from the map.
pub fn reliabilities_from_journal(
    journal: &Journal,
    models: &[String],
    limit: i64,
) -> HashMap<String, Reliability> {
    let mut out = HashMap::new();
    for model in models {
        if let Ok(Some(r)) = crate::eval::metrics::model_reliability(journal, model, limit) {
            out.insert(model.clone(), r);
        }
    }
    out
}

/// Gate a borrowed candidate set, returning `(kept, denied-with-reason)`.
/// Unlike [`filter_deployments`] this takes references — the shape the
/// cascade's `healthy_for` closure produces.
pub fn gate_refs_with_denied<'a>(
    eligible: Vec<&'a Deployment>,
    reliabilities: &HashMap<String, Reliability>,
    gate: &ReliabilityGate,
) -> (Vec<&'a Deployment>, Vec<(&'a Deployment, &'static str)>) {
    let mut kept = Vec::new();
    let mut denied = Vec::new();
    for deployment in eligible {
        match gate_model(&deployment.model_id, reliabilities, gate) {
            GateDecision::Allow => kept.push(deployment),
            GateDecision::Deny(reason) => denied.push((deployment, reason)),
        }
    }
    (kept, denied)
}

/// Fail-safe gated view: if the gate would drop **every** candidate, the
/// original set is returned so the cascade never loses its route.
pub fn gate_refs<'a>(
    eligible: &[&'a Deployment],
    reliabilities: &HashMap<String, Reliability>,
    gate: &ReliabilityGate,
) -> Vec<&'a Deployment> {
    let (kept, _denied) = gate_refs_with_denied(eligible.to_vec(), reliabilities, gate);
    if kept.is_empty() {
        eligible.to_vec()
    } else {
        kept
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(model: &str, n: i64, pass_rate: f64) -> Reliability {
        Reliability {
            model: model.to_string(),
            n,
            pass_rate,
            tokens_per_solved: 0.0,
            cost_per_solved: 0.0,
        }
    }

    fn map(entries: &[(&str, Reliability)]) -> HashMap<String, Reliability> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn unknown_model_allowed_by_default_denied_when_strict() {
        let empty = HashMap::new();
        let gate = ReliabilityGate::default();
        assert_eq!(gate_model("mystery", &empty, &gate), GateDecision::Allow);
        let strict = ReliabilityGate {
            allow_unknown: false,
            ..ReliabilityGate::default()
        };
        assert_eq!(
            gate_model("mystery", &empty, &strict),
            GateDecision::Deny("no eval history")
        );
    }

    #[test]
    fn thin_history_is_never_gated() {
        let rels = map(&[("weak", rel("weak", 3, 0.0))]);
        assert_eq!(
            gate_model("weak", &rels, &ReliabilityGate::default()),
            GateDecision::Allow,
            "3 samples < min_samples(20) → allowed despite 0% pass rate"
        );
    }

    #[test]
    fn enough_history_below_threshold_is_denied() {
        let rels = map(&[("bad", rel("bad", 50, 0.1)), ("ok", rel("ok", 50, 0.9))]);
        let gate = ReliabilityGate::default();
        assert_eq!(
            gate_model("bad", &rels, &gate),
            GateDecision::Deny("pass_rate below threshold")
        );
        assert_eq!(gate_model("ok", &rels, &gate), GateDecision::Allow);
    }

    #[test]
    fn filter_partitions_deployments() {
        let good = Deployment::new("good", "http://a");
        let bad = Deployment::new("bad", "http://b");
        let unknown = Deployment::new("who", "http://c");
        let deploys = vec![good, bad, unknown];
        let rels = map(&[
            ("bad", rel("bad", 100, 0.2)),
            ("good", rel("good", 100, 0.95)),
        ]);

        let (allowed, denied) = filter_deployments(&deploys, &rels, &ReliabilityGate::default());
        let allowed_models: Vec<&str> = allowed.iter().map(|d| d.model_id.as_str()).collect();
        assert!(allowed_models.contains(&"good"));
        assert!(allowed_models.contains(&"who"));
        assert_eq!(denied.len(), 1);
        assert_eq!(denied[0].0.model_id, "bad");
        assert_eq!(denied[0].1, "pass_rate below threshold");
    }

    #[test]
    fn gate_refs_falls_back_when_everything_is_denied() {
        let only = Deployment::new("bad", "http://b");
        let refs = vec![&only];
        let rels = map(&[("bad", rel("bad", 100, 0.0))]);
        let gated = gate_refs(&refs, &rels, &ReliabilityGate::default());
        assert_eq!(gated.len(), 1, "never leave the cascade without a route");
        assert_eq!(gated[0].model_id, "bad");
    }

    #[test]
    fn gate_refs_keeps_allowed_and_drops_denied() {
        let good = Deployment::new("good", "http://a");
        let bad = Deployment::new("bad", "http://b");
        let refs = vec![&good, &bad];
        let rels = map(&[("bad", rel("bad", 100, 0.1))]);
        let gated = gate_refs(&refs, &rels, &ReliabilityGate::default());
        let ids: Vec<&str> = gated.iter().map(|d| d.model_id.as_str()).collect();
        assert_eq!(ids, vec!["good"]);
    }

    #[test]
    fn reliabilities_from_journal_populates_evaluated_models() {
        use crate::journal::{EvalCaseInput, EvalRunStart, EvalTotals};
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let journal = Journal::open(dir.path()).unwrap();
        let run = journal
            .eval_run_start(&EvalRunStart {
                suite: "golden",
                agent: "atlas",
                harness: "atlas-local",
                model: Some("m1"),
                metadata_json: None,
            })
            .unwrap();
        for (id, status) in [("a", "pass"), ("b", "pass"), ("c", "fail")] {
            journal
                .eval_run_record_case(&EvalCaseInput {
                    run_id: &run,
                    case_id: id,
                    category: Some("SYS"),
                    status,
                    duration_ms: 0,
                    turns: 0,
                    no_action_turns: 0,
                    tokens_in: 0,
                    tokens_out: 0,
                    cost_usd: 0.0,
                    failure_kind: if status == "pass" {
                        None
                    } else {
                        Some("VERIFY")
                    },
                    detail: None,
                })
                .unwrap();
        }
        journal
            .eval_run_finish(
                &run,
                &EvalTotals {
                    status: "completed",
                    total: 3,
                    passed: 2,
                    failed: 1,
                    tokens_total: 0,
                    cost_usd: 0.0,
                },
            )
            .unwrap();

        let models = vec!["m1".to_string(), "never-evaluated".to_string()];
        let map = reliabilities_from_journal(&journal, &models, 20);
        assert!(map.contains_key("m1"));
        assert!(!map.contains_key("never-evaluated"));
        assert_eq!(map["m1"].n, 3);
    }
}
