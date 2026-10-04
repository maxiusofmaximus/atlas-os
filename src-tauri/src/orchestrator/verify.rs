// Atlas OS — Drive a `Diff` through the pure Validation→Repair engines while
// advancing the Execution Supervisor state machine (Fase 26 v26.2; RFC 14 §8,
// RFC 15 §4).
//
// This is the second half of the LLM-driven-coding bridge: v26.1 turns a routed
// model reply into a `Diff`; this runs that `Diff` through the SAME engines the
// heuristic CLI loop uses (`validation::runner::run`, `repair::runner::run`) and
// walks the supervisor: `ValidationStarted → ValidationPassed|ValidationFailed|
// CriticalValidation`, and on any failure `RepairAttempted`. The engines are
// pure (they inspect the diff, they do not touch the filesystem), so the whole
// cycle is unit-testable offline.

use crate::coding::types::Diff;
use crate::repair::runner::{run as run_repair, RepairInput};
use crate::repair::types::RepairReport;
use crate::supervisor::runner::{tick, TickContext};
use crate::supervisor::types::{MissionPhase, SupervisorAction, SupervisorEvent, SupervisorState};
use crate::validation::runner::{run as run_validation, ValidationInput};
use crate::validation::types::{StageKind, ValidationMode, ValidationOutcome, ValidationReport};

/// Inputs the verification pass needs (mode + escalation counters). Mirrors
/// `ValidationInput`/`RepairInput` so the caller owns the policy.
#[derive(Clone, Debug)]
pub struct VerifyConfig {
    pub mode: ValidationMode,
    pub previous_critical_failure: Option<StageKind>,
    pub mission_failure_count: u32,
    pub previous_attempts: u32,
    pub model_id: String,
}

impl Default for VerifyConfig {
    fn default() -> Self {
        Self {
            mode: ValidationMode::default(),
            previous_critical_failure: None,
            mission_failure_count: 0,
            previous_attempts: 0,
            model_id: "heuristic-v0".into(),
        }
    }
}

/// What one verification pass produced.
#[derive(Clone, Debug)]
pub struct VerifyOutcome {
    pub report: ValidationReport,
    /// Present iff the report was not `Pass` (a repair was attempted).
    pub repair: Option<RepairReport>,
    pub phase: MissionPhase,
    pub actions: Vec<SupervisorAction>,
}

/// Run `diff` through Validation and, when it does not pass, Repair — feeding
/// the supervisor's validation/repair events. Returns the advanced `state`
/// alongside the reports so the caller can persist them and render the
/// supervisor's actions.
pub fn verify_diff(
    mut state: SupervisorState,
    ctx: &mut TickContext,
    diff: &Diff,
    cfg: &VerifyConfig,
) -> (SupervisorState, VerifyOutcome) {
    state = tick(ctx, state, SupervisorEvent::ValidationStarted).state;

    let report = run_validation(&ValidationInput {
        diff,
        mode: cfg.mode,
        previous_critical_failure: cfg.previous_critical_failure,
        model_id: cfg.model_id.clone(),
    });

    let mut actions: Vec<SupervisorAction> = Vec::new();
    match report.outcome {
        ValidationOutcome::Pass => {
            let out = tick(
                ctx,
                state,
                SupervisorEvent::ValidationPassed {
                    report_id: report.report_id,
                },
            );
            state = out.state;
            actions.extend(out.actions);
        }
        ValidationOutcome::Fail => {
            let out = tick(
                ctx,
                state,
                SupervisorEvent::ValidationFailed {
                    report_id: report.report_id,
                },
            );
            state = out.state;
            actions.extend(out.actions);
        }
        ValidationOutcome::Critical => {
            let out = tick(
                ctx,
                state,
                SupervisorEvent::CriticalValidation {
                    report_id: report.report_id,
                },
            );
            state = out.state;
            actions.extend(out.actions);
        }
    }

    let mut repair: Option<RepairReport> = None;
    if report.outcome != ValidationOutcome::Pass {
        let rr = run_repair(RepairInput {
            source_diff: diff,
            triggered_by_report: &report,
            previous_attempts: cfg.previous_attempts,
            mission_failure_count: cfg.mission_failure_count,
            model_id: cfg.model_id.clone(),
        });
        let out = tick(
            ctx,
            state,
            SupervisorEvent::RepairAttempted {
                repair_id: rr.repair_id,
                outcome: rr.outcome,
            },
        );
        state = out.state;
        actions.extend(out.actions);
        repair = Some(rr);
    }

    let outcome = VerifyOutcome {
        report,
        repair,
        phase: state.phase,
        actions,
    };
    (state, outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::types::{FileEdit, Hunk};
    use crate::supervisor::types::BudgetCaps;
    use crate::supervisor::types::ExecutionMode;
    use uuid::Uuid;

    fn diff(lines: Vec<String>, path: &str) -> Diff {
        Diff {
            diff_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            mission_id: Uuid::new_v4(),
            step_id: "S1".into(),
            agent_id: Uuid::new_v4(),
            generated_at: "2026-10-03T00:00:00Z".into(),
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
            model_id: "m".into(),
            elapsed_ms: 0,
        }
    }

    /// Bring a fresh supervisor to `Executing` (MissionStarted + PlanGenerated).
    fn executing(ctx: &mut TickContext) -> SupervisorState {
        let state = SupervisorState::new(BudgetCaps::DEFAULT, ExecutionMode::HumanInLoop);
        let state = tick(
            ctx,
            state,
            SupervisorEvent::MissionStarted {
                mission_id: Uuid::new_v4(),
            },
        )
        .state;
        tick(
            ctx,
            state,
            SupervisorEvent::PlanGenerated {
                plan_id: Uuid::new_v4(),
            },
        )
        .state
    }

    #[test]
    fn passing_diff_marks_validation_passed_and_done() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let mut d = diff(
            vec!["pub fn add(a: u32, b: u32) -> u32 { a + b }".into()],
            "src/lib.rs",
        );
        d.narrative = "adds add(), covered by a unit test".into();
        d.research_refs = vec![Uuid::new_v4()];
        let (_state, out) =
            verify_diff(executing(&mut ctx), &mut ctx, &d, &VerifyConfig::default());
        assert!(out.report.is_pass(), "report={:?}", out.report.outcome);
        assert!(out.repair.is_none(), "a passing diff needs no repair");
        assert_eq!(out.phase, MissionPhase::Done);
        assert!(out
            .actions
            .iter()
            .any(|a| matches!(a, SupervisorAction::MarkDone)));
    }

    #[test]
    fn failing_diff_triggers_repair() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let d = diff(vec!["fn x() {".into()], "src/lib.rs");
        let (_state, out) =
            verify_diff(executing(&mut ctx), &mut ctx, &d, &VerifyConfig::default());
        assert_eq!(out.report.outcome, ValidationOutcome::Fail);
        assert_eq!(out.report.failed_stage(), Some(StageKind::TypeCheck));
        assert!(out.repair.is_some(), "a failing diff must attempt repair");
    }

    #[test]
    fn repeat_failure_escalates_to_critical() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let d = diff(vec!["fn x() {".into()], "src/lib.rs");
        let cfg = VerifyConfig {
            previous_critical_failure: Some(StageKind::TypeCheck),
            ..VerifyConfig::default()
        };
        let (_state, out) = verify_diff(executing(&mut ctx), &mut ctx, &d, &cfg);
        assert_eq!(out.report.outcome, ValidationOutcome::Critical);
        assert!(out.repair.is_some());
    }
}
