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
}
