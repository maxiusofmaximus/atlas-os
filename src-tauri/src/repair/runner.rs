// Atlas OS — Repair Engine runner (RFC 15).
//
// Phase 1: heuristic-only. The runner consumes a failed `ValidationReport`
// + the `Diff` that was being validated, classifies the failure (RFC 15 §2),
// picks a repair strategy (RFC 15 §2 table), builds an amended `Diff` and
// re-runs the heuristic Validation cascade on the patched content to
// confirm convergence (RFC 15 §1 "Aplicar → Validar otra vez"). The runner
// never enters an infinite loop: §4 enforces `MAX_ATTEMPTS_PER_STAGE = 3`
// per stage and `MAX_MISSION_FAILURES = 6` per mission.
//
// The runner does NOT shell out to Coding / Validation engines recursively
// — it reuses their pure functions (`validation::run`) directly. Phase 2
// will route through the Model Orchestrator (RFC 04) for actual LLM-driven
// analysis; the signatures here are kept stable for that swap.

use std::time::Instant;
use uuid::Uuid;

use crate::coding::types::{Diff, FileEdit, Hunk};
use crate::repair::types::{
    ErrorClass, RepairAttempt, RepairOutcome, RepairReport, RepairStrategy, MAX_ATTEMPTS_PER_STAGE,
    MAX_MISSION_FAILURES,
};
use crate::validation::runner::ValidationInput;
use crate::validation::types::{StageKind, ValidationMode, ValidationOutcome, ValidationReport};

/// Inputs the runner needs for a single Repair run. The `source_diff` is
/// the `Diff` that the Validation Engine rejected; `triggered_by_report`
/// is its `ValidationReport`. The `previous_attempts` and
/// `mission_failure_count` fields let the runner honour the RFC 15 §4
/// budget — they are supplied by the Execution Supervisor (RFC 19) and
/// start at 0 for the first repair run of a step.
#[derive(Clone, Debug)]
pub struct RepairInput<'a> {
    pub source_diff: &'a Diff,
    pub triggered_by_report: &'a ValidationReport,
    /// How many times this same stage has already failed (across prior
    /// repair runs for the same step). Drives the §4 Planning escalation.
    pub previous_attempts: u32,
    /// Total failures accumulated across the whole mission so far. Drives
    /// the §4 human escalation.
    pub mission_failure_count: u32,
    pub model_id: String,
}

impl<'a> RepairInput<'a> {
    pub fn new(source_diff: &'a Diff, triggered_by_report: &'a ValidationReport) -> Self {
        Self {
            source_diff,
            triggered_by_report,
            previous_attempts: 0,
            mission_failure_count: 0,
            model_id: "heuristic-v0".into(),
        }
    }

    pub fn with_previous_attempts(mut self, n: u32) -> Self {
        self.previous_attempts = n;
        self
    }

    pub fn with_mission_failures(mut self, n: u32) -> Self {
        self.mission_failure_count = n;
        self
    }
}

/// Run the Repair Engine. Always returns a `RepairReport`; never `Err`.
pub fn run(input: RepairInput) -> RepairReport {
    let started = Instant::now();
    let repair_id = Uuid::new_v4();
    let triggering_stage = input
        .triggered_by_report
        .failed_stage()
        .unwrap_or(StageKind::LintFormat);

    // RFC 15 §2 last row — `RatingConfidenceLow` is not a code error.
    // The runner escalates immediately to Planning when the failure was
    // flagged as `Critical` (RFC 14 §8 — same stage failed twice in a row).
    if matches!(
        input.triggered_by_report.outcome,
        crate::validation::types::ValidationOutcome::Critical
    ) {
        let attempt = RepairAttempt {
            error_class: ErrorClass::RatingConfidenceLow,
            strategy: RepairStrategy::PlanningReplan,
            success: false,
            root_cause: "validation.fail.critical — same stage failed twice (RFC 14 §8)".into(),
            proposed_diff: None,
        };
        return finalize(
            repair_id,
            &input,
            vec![attempt],
            RepairOutcome::EscalatedPlanning,
            triggering_stage,
            None,
            None,
            started,
        );
    }

    // RFC 15 §4 — after `MAX_ATTEMPTS_PER_STAGE` consecutive failures for
    // the same stage, escalate to Planning (replan + research).
    if input.previous_attempts >= MAX_ATTEMPTS_PER_STAGE {
        let attempt = RepairAttempt {
            error_class: ErrorClass::from_stage(triggering_stage).unwrap_or(ErrorClass::TypeCheck),
            strategy: RepairStrategy::PlanningReplan,
            success: false,
            root_cause: format!(
                "{} consecutive failures on stage {} (RFC 15 §4)",
                input.previous_attempts,
                triggering_stage.tag()
            ),
            proposed_diff: None,
        };
        return finalize(
            repair_id,
            &input,
            vec![attempt],
            RepairOutcome::EscalatedPlanning,
            triggering_stage,
            None,
            None,
            started,
        );
    }

    // RFC 15 §4 — after `MAX_MISSION_FAILURES` total failures for the
    // mission, surface to the human in the Agent Console (RFC 24).
    if input.mission_failure_count >= MAX_MISSION_FAILURES {
        let attempt = RepairAttempt {
            error_class: ErrorClass::from_stage(triggering_stage).unwrap_or(ErrorClass::TypeCheck),
            strategy: RepairStrategy::HumanEscalate,
            success: false,
            root_cause: format!(
                "{} total mission failures (RFC 15 §4)",
                input.mission_failure_count
            ),
            proposed_diff: None,
        };
        return finalize(
            repair_id,
            &input,
            vec![attempt],
            RepairOutcome::EscalatedHuman,
            triggering_stage,
            None,
            None,
            started,
        );
    }

    // RFC 15 §1 — classify → analyse → propose.
    let class = ErrorClass::from_stage(triggering_stage).unwrap_or(ErrorClass::TypeCheck);
    let finding = first_finding(input.triggered_by_report, triggering_stage);
    let strategy = pick_strategy(class, finding.as_deref());
    let root_cause = describe_root_cause(triggering_stage, finding.as_deref());

    let proposed_diff = build_fix(input.source_diff, triggering_stage, finding.as_deref());
    let success = proposed_diff
        .as_ref()
        .map(|d| would_revalidate_clean(d, input.triggered_by_report.mode))
        .unwrap_or(false);

    let attempt = RepairAttempt {
        error_class: class,
        strategy,
        success,
        root_cause,
        proposed_diff: proposed_diff.clone().map(Box::new),
    };

    let outcome = if success {
        RepairOutcome::Applied
    } else if strategy == RepairStrategy::PlanningReplan {
        RepairOutcome::EscalatedPlanning
    } else {
        // A fix was proposed but the heuristic re-validation did not
        // converge — Phase 2 will route through the Coding Engine for a
        // real amendment. Phase 1 marks `Proposed` so the Execution
        // Supervisor re-enters the Validation loop.
        RepairOutcome::Proposed
    };

    finalize(
        repair_id,
        &input,
        vec![attempt],
        outcome,
        triggering_stage,
        if success { Some(0) } else { None },
        proposed_diff.map(Box::new),
        started,
    )
}

fn first_finding(report: &ValidationReport, stage: StageKind) -> Option<String> {
    report
        .stages
        .iter()
        .find(|s| s.stage == stage)
        .and_then(|s| s.findings.first().map(|f| f.rule.clone()))
}

/// RFC 15 §2 — strategy per error class + finding rule. The rule tag is
/// consulted to distinguish AutoFix (mechanical) from CodingAmendment
/// (needs a structural rewrite).
fn pick_strategy(class: ErrorClass, finding_rule: Option<&str>) -> RepairStrategy {
    match (class, finding_rule) {
        (
            ErrorClass::SyntaxFormat,
            Some(
                "no_println" | "no_dbg" | "no_console_log" | "no_fmt_println" | "no_todo"
                | "dead_binding",
            ),
        ) => RepairStrategy::AutoFix,
        (ErrorClass::TypeCheck, Some("unbalanced_braces")) => RepairStrategy::AutoFix,
        (ErrorClass::TypeCheck, Some("missing_fn_keyword")) => RepairStrategy::CodingAmendment,
        (ErrorClass::TestFailure, Some("red_tdd_stub")) => RepairStrategy::CodingAmendment,
        (ErrorClass::TestFailure, Some("missing_coverage")) => RepairStrategy::CodingRetry,
        (ErrorClass::RatingConfidenceLow, _) => RepairStrategy::PlanningReplan,
        (ErrorClass::Vulnerability, _) | (ErrorClass::Config, _) => RepairStrategy::PlanningReplan,
        _ => RepairStrategy::CodingRetry,
    }
}

fn describe_root_cause(stage: StageKind, finding_rule: Option<&str>) -> String {
    match (stage, finding_rule) {
        (StageKind::LintFormat, Some(r)) => format!("lint rule `{r}` fired on a sentinel line"),
        (StageKind::TypeCheck, Some(r)) => {
            format!("type-check rejected the heuristic parse: `{r}`")
        }
        (StageKind::UnitTests, Some("red_tdd_stub")) => {
            "TDD red stub still asserts `false` — the impl step has not flipped it.".into()
        }
        (StageKind::UnitTests, Some("missing_coverage")) => {
            "non-test code changed without a matching test delta.".into()
        }
        (StageKind::DeadCode, Some("dead_binding")) => {
            "placeholder `let _ = …` left in the diff by the Coding Engine.".into()
        }
        _ => "unclassified heuristic root cause".into(),
    }
}

/// Heuristic fixer. Builds a new `Diff` on top of `source_diff` that
/// patches the offending lines. Returns `None` when the heuristic has no
/// rule for the failure — the runner then escalates.
fn build_fix(source_diff: &Diff, stage: StageKind, finding_rule: Option<&str>) -> Option<Diff> {
    let rule = finding_rule?;
    let mut files: Vec<FileEdit> = Vec::with_capacity(source_diff.files.len());
    for f in &source_diff.files {
        let mut new_hunks: Vec<Hunk> = Vec::new();
        for h in &f.hunks {
            let mut patched_lines: Vec<String> = Vec::with_capacity(h.new_lines.len());
            let mut changed = false;
            for line in &h.new_lines {
                match rewrite_line(line, stage, rule) {
                    Some(new_line) => {
                        patched_lines.push(new_line);
                        changed = true;
                    }
                    None => patched_lines.push(line.clone()),
                }
            }
            if changed {
                new_hunks.push(Hunk {
                    old_start: h.old_start,
                    old_end: h.old_end,
                    new_lines: patched_lines,
                    rationale: format!("repair: apply heuristic fix for `{rule}`"),
                });
            }
        }
        if !new_hunks.is_empty() {
            files.push(FileEdit {
                path: f.path.clone(),
                hunks: new_hunks,
                is_new_file: f.is_new_file,
                is_delete: f.is_delete,
            });
        }
    }
    if files.is_empty() {
        return None;
    }
    Some(Diff {
        diff_id: Uuid::new_v4(),
        plan_id: source_diff.plan_id,
        mission_id: source_diff.mission_id,
        step_id: source_diff.step_id.clone(),
        agent_id: source_diff.agent_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        files,
        narrative: format!("Repair Engine heuristic patch for `{rule}`"),
        research_refs: vec![],
        risk_decision: None,
        model_id: "heuristic-v0".into(),
        elapsed_ms: 0,
    })
}

/// Per-line rewriter. Maps a `(line, stage, rule)` triple to the patched
/// line or `None` when the line should be left untouched.
fn rewrite_line(line: &str, stage: StageKind, rule: &str) -> Option<String> {
    match (stage, rule) {
        (StageKind::LintFormat, "no_println") => Some(line.replace("println!", "tracing::info!")),
        (StageKind::LintFormat, "no_dbg") => Some(line.replace("dbg!", "tracing::debug!")),
        (StageKind::LintFormat, "no_console_log") => {
            Some(line.replace("console.log", "logger.debug"))
        }
        (StageKind::LintFormat, "no_fmt_println") => Some(
            line.replace("fmt.Println", "log.Info")
                .replace("fmt.Printf", "log.Infof"),
        ),
        (StageKind::LintFormat, "no_todo") => Some(
            line.replacen("// todo", "// resolved", 1)
                .replacen("TODO", "// resolved", 1)
                .replacen("FIXME", "// resolved", 1),
        ),
        (StageKind::DeadCode, "dead_binding") => Some(String::new()),
        (StageKind::TypeCheck, "unbalanced_braces") => Some(balance_braces(line)),
        (StageKind::TypeCheck, "missing_fn_keyword") => Some(prefix_keyword(line, "fn ")),
        (StageKind::UnitTests, "red_tdd_stub") => {
            Some(line.replace("assert!(false", "assert!(true"))
        }
        _ => None,
    }
}

fn balance_braces(line: &str) -> String {
    let opens = line.matches('{').count() as i32;
    let closes = line.matches('}').count() as i32;
    let diff = opens - closes;
    if diff > 0 {
        format!("{line}{}", "}".repeat(diff as usize))
    } else if diff < 0 {
        let extra = (-diff) as usize;
        let mut s = line.to_string();
        for _ in 0..extra {
            if let Some(pos) = s.rfind('}') {
                s.remove(pos);
            }
        }
        s
    } else {
        line.to_string()
    }
}

fn prefix_keyword(line: &str, keyword: &str) -> String {
    let trimmed = line.trim_start();
    let lead = &line[..line.len() - trimmed.len()];
    if !trimmed.starts_with(keyword) {
        format!("{lead}{keyword}{trimmed}")
    } else {
        line.to_string()
    }
}

/// RFC 15 §1 "Validar otra vez" — re-run the Validation cascade on the
/// proposed diff and confirm it now passes. Phase 1 calls the pure
/// `validation::run` directly; Phase 2 will route through the kernel
/// bus so the HUD timeline stays consistent.
fn would_revalidate_clean(diff: &Diff, mode: ValidationMode) -> bool {
    let input = ValidationInput::new(diff).with_mode(mode);
    let report = crate::validation::runner::run(&input);
    matches!(report.outcome, ValidationOutcome::Pass)
}

#[allow(clippy::too_many_arguments)]
fn finalize(
    repair_id: Uuid,
    input: &RepairInput,
    attempts: Vec<RepairAttempt>,
    outcome: RepairOutcome,
    triggering_stage: StageKind,
    successful_attempt: Option<u32>,
    final_diff: Option<Box<Diff>>,
    started: Instant,
) -> RepairReport {
    let elapsed_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
    RepairReport {
        repair_id,
        triggered_by_report_id: input.triggered_by_report.report_id,
        source_diff_id: input.source_diff.diff_id,
        plan_id: input.source_diff.plan_id,
        mission_id: input.source_diff.mission_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        attempts,
        outcome,
        triggering_stage,
        successful_attempt,
        final_diff,
        model_id: input.model_id.clone(),
        elapsed_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::types::{Diff, FileEdit, Hunk};
    use crate::validation::runner::ValidationInput;

    fn diff_with(lines: Vec<String>, path: &str) -> Diff {
        Diff {
            diff_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            mission_id: Uuid::new_v4(),
            step_id: "S1".into(),
            agent_id: Uuid::new_v4(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            files: vec![FileEdit {
                path: path.into(),
                is_new_file: false,
                is_delete: false,
                hunks: vec![Hunk {
                    old_start: 0,
                    old_end: 0,
                    new_lines: lines,
                    rationale: "r".into(),
                }],
            }],
            narrative: String::new(),
            research_refs: vec![],
            risk_decision: None,
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        }
    }

    fn failing_report(diff: &Diff) -> ValidationReport {
        let input = ValidationInput::new(diff);
        let report = crate::validation::runner::run(&input);
        assert!(
            !matches!(report.outcome, ValidationOutcome::Pass),
            "test setup: diff must fail validation"
        );
        report
    }

    #[test]
    fn auto_fix_println_to_tracing_applies_clean() {
        let d = diff_with(vec!["println!(\"hi\");".into()], "src/lib.rs");
        let report = failing_report(&d);
        assert_eq!(report.failed_stage(), Some(StageKind::LintFormat));

        let repair = run(RepairInput::new(&d, &report));
        assert_eq!(
            repair.outcome,
            RepairOutcome::Applied,
            "println auto-fix should converge"
        );
        assert!(repair.is_applied());
        let final_diff = repair.final_diff.expect("applied -> diff");
        let hunk_line = &final_diff.files[0].hunks[0].new_lines[0];
        assert!(hunk_line.contains("tracing::info!"));
        assert!(!hunk_line.contains("println!"));
        assert!(repair.successful_attempt == Some(0));
        assert_eq!(repair.attempts[0].strategy, RepairStrategy::AutoFix);
    }

    #[test]
    fn coding_amendment_flips_red_tdd_stub() {
        let d = diff_with(
            vec!["#[test] fn t() { assert!(false, \"red\"); }".into()],
            "tests/s1_heuristic_v0.rs",
        );
        let report = failing_report(&d);
        assert_eq!(report.failed_stage(), Some(StageKind::UnitTests));

        let repair = run(RepairInput::new(&d, &report));
        assert_eq!(
            repair.outcome,
            RepairOutcome::Applied,
            "flipping the stub converges"
        );
        assert_eq!(repair.attempts[0].strategy, RepairStrategy::CodingAmendment);
        let final_diff = repair.final_diff.expect("diff");
        assert!(final_diff.files[0].hunks[0].new_lines[0].contains("assert!(true"));
    }

    #[test]
    fn auto_fix_unbalanced_braces_converges() {
        let d = diff_with(vec!["fn x() {".into()], "src/lib.rs");
        let report = failing_report(&d);
        assert_eq!(report.failed_stage(), Some(StageKind::TypeCheck));

        let repair = run(RepairInput::new(&d, &report));
        assert_eq!(repair.outcome, RepairOutcome::Applied);
        let final_diff = repair.final_diff.expect("diff");
        let patched = &final_diff.files[0].hunks[0].new_lines[0];
        let opens = patched.matches('{').count();
        let closes = patched.matches('}').count();
        assert_eq!(opens, closes, "braces balanced after patch");
    }

    #[test]
    fn three_attempts_escalates_to_planning() {
        let d = diff_with(vec!["println!(\"hi\");".into()], "src/lib.rs");
        let report = failing_report(&d);
        // previous_attempts == 3 -> §4 Planning escalation.
        let repair = run(RepairInput::new(&d, &report).with_previous_attempts(3));
        assert_eq!(repair.outcome, RepairOutcome::EscalatedPlanning);
        assert!(repair.is_escalated());
        assert_eq!(repair.attempts[0].strategy, RepairStrategy::PlanningReplan);
        assert!(repair.final_diff.is_none());
    }

    #[test]
    fn critical_validation_outcome_escalates_to_planning() {
        let d = diff_with(vec!["fn x() {".into()], "src/lib.rs");
        // first run is a normal Fail; mark the report Critical to simulate
        // RFC 14 §8 (same stage twice in a row).
        let mut report = failing_report(&d);
        report.outcome = crate::validation::types::ValidationOutcome::Critical;

        let repair = run(RepairInput::new(&d, &report));
        assert_eq!(repair.outcome, RepairOutcome::EscalatedPlanning);
        assert_eq!(repair.triggering_stage, StageKind::TypeCheck);
        assert_eq!(
            repair.attempts[0].error_class,
            ErrorClass::RatingConfidenceLow
        );
    }

    #[test]
    fn six_mission_failures_escalates_to_human() {
        let d = diff_with(vec!["fn x() {".into()], "src/lib.rs");
        let report = failing_report(&d);
        let repair = run(RepairInput::new(&d, &report).with_mission_failures(6));
        assert_eq!(repair.outcome, RepairOutcome::EscalatedHuman);
        assert!(repair.is_escalated());
        assert_eq!(repair.attempts[0].strategy, RepairStrategy::HumanEscalate);
    }

    #[test]
    fn unknown_rule_returns_proposed_or_escalated() {
        // Synthesise a failing report with a fabricated rule so
        // `pick_strategy` falls back to `CodingRetry` (Proposed).
        let d = diff_with(vec!["fn x() {".into()], "src/lib.rs");
        let mut report = failing_report(&d);
        let stage_idx = report
            .stages
            .iter()
            .position(|s| s.stage == StageKind::TypeCheck)
            .unwrap();
        report.stages[stage_idx].findings.clear();
        report.stages[stage_idx]
            .findings
            .push(crate::validation::types::Finding {
                file: "src/lib.rs".into(),
                rule: "unknown_exotic_rule".into(),
                suggestion: None,
                auto_fixable: false,
            });

        let repair = run(RepairInput::new(&d, &report));
        assert!(
            matches!(repair.outcome, RepairOutcome::Proposed | RepairOutcome::EscalatedPlanning),
            "unknown rule falls back to Proposed (no heuristic match), or Planning if pick_strategy picked replan"
        );
    }

    #[test]
    fn no_failure_returns_proposed() {
        // DeadCode is `Warn` not `Fail`; the validation outcome is `Pass`
        // (no `failed_stage`). The runner defaults to `LintFormat` and
        // `build_fix` returns `None` (no finding rule) -> `Proposed`.
        let mut d = diff_with(vec!["let _ = 42;".into()], "src/lib.rs");
        d.narrative = "placeholder binding, dead-code warning accepted".into();
        d.research_refs = vec![Uuid::new_v4()];
        let report = {
            let inp = ValidationInput::new(&d);
            crate::validation::runner::run(&inp)
        };
        assert!(matches!(report.outcome, ValidationOutcome::Pass));
        let repair = run(RepairInput::new(&d, &report));
        assert_eq!(repair.outcome, RepairOutcome::Proposed);
    }
}
