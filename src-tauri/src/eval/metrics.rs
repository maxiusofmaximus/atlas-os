// Atlas OS — Evaluation metrics (RFC 20 Fase 22, EVAL.2; plan research/51).
//
// Normalizes the raw `eval_cases` rows into the field's converged metrics so
// the same numbers drive the CLI, the HUD card, and the orchestrator feedback
// accessor. Definitions (research/51 §5):
//   pass_rate          = passed / total
//   tokens_per_solved  = total tokens / solved   (KDD "Scaffold Effect")
//   cost_per_solved    = total cost  / solved
//   failure_kinds      = histogram over non-pass cases (REASON/VERIFY/…)
//
// The unit of measure is the harness × model pair (see `summarize_by_harness_model`).

use std::collections::BTreeMap;

use serde::Serialize;

use crate::journal::{EvalCaseRow, EvalRunRow, Journal};

/// Aggregated evaluation metrics.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct EvalSummary {
    pub runs: i64,
    pub total: i64,
    pub passed: i64,
    pub failed: i64,
    pub errored: i64,
    pub pass_rate: f64,
    pub tokens_total: i64,
    pub tokens_per_solved: f64,
    pub no_action_turns: i64,
    pub cost_usd: f64,
    pub cost_per_solved: f64,
    pub failure_kinds: BTreeMap<String, i64>,
}

/// One harness × model group of `EvalSummary`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GroupSummary {
    pub key: String,
    pub summary: EvalSummary,
}

/// A model's historical reliability, for the orchestrator feedback loop.
#[derive(Clone, Debug, PartialEq)]
pub struct Reliability {
    pub model: String,
    pub n: i64,
    pub pass_rate: f64,
    pub tokens_per_solved: f64,
    pub cost_per_solved: f64,
}

/// Aggregate the cases of the given runs into one `EvalSummary`.
pub fn summarize_runs(journal: &Journal, runs: &[EvalRunRow]) -> anyhow::Result<EvalSummary> {
    let mut summary = EvalSummary {
        runs: runs.len() as i64,
        ..Default::default()
    };
    for run in runs {
        for case in journal.eval_cases(&run.id)? {
            accumulate(&mut summary, &case);
        }
    }
    finalize(&mut summary);
    Ok(summary)
}

/// Aggregate over the newest `limit` runs.
pub fn summarize_recent(journal: &Journal, limit: i64) -> anyhow::Result<EvalSummary> {
    let runs = journal.eval_run_list(limit)?;
    summarize_runs(journal, &runs)
}

/// Aggregate per harness × model group over the newest `limit` runs.
pub fn summarize_by_harness_model(
    journal: &Journal,
    limit: i64,
) -> anyhow::Result<Vec<GroupSummary>> {
    let runs = journal.eval_run_list(limit)?;
    let mut groups: BTreeMap<String, Vec<EvalRunRow>> = BTreeMap::new();
    for run in runs {
        let key = format!("{}/{}", run.harness, run.model.as_deref().unwrap_or("-"));
        groups.entry(key).or_default().push(run);
    }
    let mut out = Vec::new();
    for (key, runs) in groups {
        out.push(GroupSummary {
            key,
            summary: summarize_runs(journal, &runs)?,
        });
    }
    Ok(out)
}

/// Historical reliability for a specific model, or `None` if never evaluated.
pub fn model_reliability(
    journal: &Journal,
    model: &str,
    limit: i64,
) -> anyhow::Result<Option<Reliability>> {
    let runs: Vec<EvalRunRow> = journal
        .eval_run_list(limit)?
        .into_iter()
        .filter(|r| r.model.as_deref() == Some(model))
        .collect();
    if runs.is_empty() {
        return Ok(None);
    }
    let summary = summarize_runs(journal, &runs)?;
    Ok(Some(Reliability {
        model: model.to_string(),
        n: summary.total,
        pass_rate: summary.pass_rate,
        tokens_per_solved: summary.tokens_per_solved,
        cost_per_solved: summary.cost_per_solved,
    }))
}

fn accumulate(summary: &mut EvalSummary, case: &EvalCaseRow) {
    summary.total += 1;
    match case.status.as_str() {
        "pass" => summary.passed += 1,
        "fail" => {
            summary.failed += 1;
            *summary
                .failure_kinds
                .entry(
                    case.failure_kind
                        .clone()
                        .unwrap_or_else(|| "UNKNOWN".into()),
                )
                .or_default() += 1;
        }
        "error" => {
            summary.errored += 1;
            *summary
                .failure_kinds
                .entry(case.failure_kind.clone().unwrap_or_else(|| "REASON".into()))
                .or_default() += 1;
        }
        _ => {}
    }
    summary.tokens_total += case.tokens_in + case.tokens_out;
    summary.no_action_turns += case.no_action_turns;
    summary.cost_usd += case.cost_usd;
}

fn finalize(summary: &mut EvalSummary) {
    summary.pass_rate = if summary.total > 0 {
        summary.passed as f64 / summary.total as f64
    } else {
        0.0
    };
    summary.tokens_per_solved = if summary.passed > 0 {
        summary.tokens_total as f64 / summary.passed as f64
    } else {
        0.0
    };
    summary.cost_per_solved = if summary.passed > 0 {
        summary.cost_usd / summary.passed as f64
    } else {
        0.0
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{EvalCaseInput, EvalRunStart, EvalTotals};

    fn fresh_journal() -> (tempfile::TempDir, Journal) {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        (tmp, journal)
    }

    fn case<'a>(run_id: &'a str, id: &'a str, status: &'a str) -> EvalCaseInput<'a> {
        EvalCaseInput {
            run_id,
            case_id: id,
            category: Some("SYS"),
            status,
            duration_ms: 10,
            turns: 3,
            no_action_turns: 1,
            tokens_in: 100,
            tokens_out: 50,
            cost_usd: 0.01,
            failure_kind: if status == "pass" {
                None
            } else {
                Some("VERIFY")
            },
            detail: None,
        }
    }

    fn seed(journal: &Journal, harness: &str, model: &str) -> String {
        journal
            .eval_run_start(&EvalRunStart {
                suite: "golden",
                agent: "atlas",
                harness,
                model: Some(model),
                metadata_json: None,
            })
            .unwrap()
    }

    #[test]
    fn summarize_recent_computes_rates_and_tokens_per_solved() {
        let (_dir, journal) = fresh_journal();
        let run = seed(&journal, "atlas-local", "m1");
        for (id, status) in [("a", "pass"), ("b", "pass"), ("c", "fail"), ("d", "error")] {
            journal
                .eval_run_record_case(&case(&run, id, status))
                .unwrap();
        }
        journal
            .eval_run_finish(
                &run,
                &EvalTotals {
                    status: "completed",
                    total: 4,
                    passed: 2,
                    failed: 2,
                    tokens_total: 600,
                    cost_usd: 0.04,
                },
            )
            .unwrap();

        let s = summarize_recent(&journal, 10).unwrap();
        assert_eq!(s.runs, 1);
        assert_eq!(s.total, 4);
        assert_eq!(s.passed, 2);
        assert_eq!(s.failed, 1);
        assert_eq!(s.errored, 1);
        assert!((s.pass_rate - 0.5).abs() < 1e-9);
        // tokens_total = 4 cases * 150; per solved = 600 / 2 solved = 300.
        assert_eq!(s.tokens_total, 600);
        assert!((s.tokens_per_solved - 300.0).abs() < 1e-9);
        assert!((s.cost_per_solved - 0.02).abs() < 1e-9);
        assert_eq!(s.no_action_turns, 4);
        assert_eq!(s.failure_kinds.get("VERIFY"), Some(&2));
    }

    #[test]
    fn groups_by_harness_and_model() {
        let (_dir, journal) = fresh_journal();
        let a = seed(&journal, "atlas-local", "m1");
        journal
            .eval_run_record_case(&case(&a, "x", "pass"))
            .unwrap();
        let b = seed(&journal, "harbor", "m2");
        journal
            .eval_run_record_case(&case(&b, "y", "fail"))
            .unwrap();

        let groups = summarize_by_harness_model(&journal, 10).unwrap();
        let keys: Vec<&str> = groups.iter().map(|g| g.key.as_str()).collect();
        assert!(keys.contains(&"atlas-local/m1"));
        assert!(keys.contains(&"harbor/m2"));

        let rel = model_reliability(&journal, "m1", 10).unwrap().unwrap();
        assert_eq!(rel.n, 1);
        assert!((rel.pass_rate - 1.0).abs() < 1e-9);
        assert!(model_reliability(&journal, "nope", 10).unwrap().is_none());
    }
}
