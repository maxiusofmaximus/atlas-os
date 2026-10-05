// Atlas OS — Validation Engine runner (RFC 14).
//
// Phase 1: heuristic-only. The runner takes a `Diff` (from the Coding
// Engine) and threads it through the stage cascade in `StageKind::pipeline_order()`.
// Each stage returns a `StageSummary`. The runner stops on the first
// `StageStatus::Fail` (RFC 14 §4 "Throttling") and marks the remaining
// stages as `Skipped`. The same stage failing twice across two runs
// triggers `ValidationOutcome::Critical` (RFC 14 §8), which prompts the
// Repair Engine (RFC 15) to take over.
//
// The runner never short-circuits on `Err`; everything flows back as
// structured types so the Kernel Bus / HUD can render failures without
// a separate error channel (RFC 14 §3 "structured summaries, not raw
// logs").

use std::time::Instant;
use uuid::Uuid;

use crate::coding::types::Diff;
use crate::validation::stages::{
    dead_code::DeadCode, domain::DomainStage, e2e::E2e, evidence_gate::EvidenceGate, iac::Iac,
    layer_boundary::LayerBoundary, lint_format::LintFormat, security_scan::SecurityScan,
    supply_chain::SupplyChain, type_check::TypeCheck, unit_tests::UnitTests, Stage, StageContext,
};
use crate::validation::types::{
    StageKind, StageStatus, StageSummary, ValidationMode, ValidationOutcome, ValidationReport,
};

/// Inputs the runner needs to execute a single pass of the validation
/// pipeline over a `Diff`. All RFC 14 §7 mode variations and the §8
/// "critical escalation" last-failed stage are taken from this struct.
#[derive(Clone, Debug)]
pub struct ValidationInput<'a> {
    pub diff: &'a Diff,
    pub mode: ValidationMode,
    /// The stage that failed on the previous run of validation for this
    /// `Diff`. Used by §8 critical-escalation: if the same stage fails
    /// twice in a row, `outcome` becomes `Critical`.
    pub previous_critical_failure: Option<StageKind>,
    pub model_id: String,
    /// RFC 64 §8 — domain-pack check commands; empty → the `Domain` stage is
    /// `Skipped` (no domain pack active).
    pub domain_commands: Vec<String>,
}

impl<'a> ValidationInput<'a> {
    /// Convenience constructor with the RFC 14 §7 default mode (`Strict`),
    /// no previous failure, and the `heuristic-v0` model id.
    pub fn new(diff: &'a Diff) -> Self {
        Self {
            diff,
            mode: ValidationMode::Strict,
            previous_critical_failure: None,
            model_id: "heuristic-v0".into(),
            domain_commands: Vec::new(),
        }
    }

    pub fn with_mode(mut self, mode: ValidationMode) -> Self {
        self.mode = mode;
        self
    }

    /// RFC 64 §8 — attach the active domain pack's validation commands.
    pub fn with_domain_commands(mut self, commands: Vec<String>) -> Self {
        self.domain_commands = commands;
        self
    }

    pub fn with_previous_failure(mut self, stage: StageKind) -> Self {
        self.previous_critical_failure = Some(stage);
        self
    }
}

/// Run the Validation Engine over `diff`. Returns a `ValidationReport`
/// regardless of outcome; never returns `Err` (RFC 14 §3 contract).
pub fn run(input: &ValidationInput) -> ValidationReport {
    let started = Instant::now();
    let touched_paths: Vec<String> = input
        .diff
        .files
        .iter()
        .map(|f| f.path.replace('\\', "/"))
        .collect();

    let ctx = StageContext {
        diff: input.diff,
        touched_paths,
        mode: input.mode,
    };

    // Build the stage cascade in RFC 14 §1 order. The vector owns boxed
    // trait objects so the cascade can iterate uniformly without a giant
    // `match`.
    let stages: Vec<Box<dyn Stage>> = vec![
        Box::new(LintFormat),
        Box::new(TypeCheck),
        Box::new(UnitTests),
        Box::new(E2e),
        Box::new(DeadCode),
        Box::new(SupplyChain),
        Box::new(SecurityScan),
        Box::new(LayerBoundary),
        Box::new(Iac),
        Box::new(DomainStage::new(input.domain_commands.clone())),
        Box::new(EvidenceGate),
    ];

    debug_assert!(
        stages
            .iter()
            .zip(StageKind::pipeline_order().iter())
            .all(|(s, k)| s.kind() == *k),
        "stage cascade must match StageKind::pipeline_order() — guard test below"
    );

    let mut summaries: Vec<StageSummary> = Vec::with_capacity(stages.len());
    let mut first_fail: Option<StageKind> = None;

    for stage in &stages {
        let kind = stage.kind();
        if let Some(failed) = first_fail {
            // RFC 14 §4 throttling — downstream of the first `Fail` is `Skipped`.
            tracing::debug!(
                stage = kind.tag(),
                blocked_by = failed.tag(),
                "skipping stage"
            );
            summaries.push(StageSummary {
                stage: kind,
                status: StageStatus::Skipped,
                elapsed_ms: 0,
                findings: vec![],
                summary: format!("skipped after stage {} failed", failed.tag()),
            });
            continue;
        }
        let summary = stage.run(&ctx);
        let status = summary.status;
        summaries.push(summary);
        if status == StageStatus::Fail {
            first_fail = Some(kind);
            tracing::info!(
                stage = kind.tag(),
                "validation stage failed; downstream throttled"
            );
        }
    }

    let outcome = resolve_outcome(first_fail, input.previous_critical_failure);
    let elapsed_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);

    ValidationReport {
        report_id: Uuid::new_v4(),
        diff_id: input.diff.diff_id,
        plan_id: input.diff.plan_id,
        mission_id: input.diff.mission_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        mode: input.mode,
        stages: summaries,
        outcome,
        model_id: input.model_id.clone(),
        elapsed_ms,
    }
}

/// RFC 14 §8 — `Critical` happens iff a stage failed now AND it was the
/// same stage that failed on the previous run; otherwise `Fail` for any
/// failure and `Pass` when there were no `Fail`s.
fn resolve_outcome(
    first_fail: Option<StageKind>,
    previous_critical_failure: Option<StageKind>,
) -> ValidationOutcome {
    match (first_fail, previous_critical_failure) {
        (Some(stage), Some(prev)) if stage == prev => ValidationOutcome::Critical,
        (Some(_), _) => ValidationOutcome::Fail,
        (None, _) => ValidationOutcome::Pass,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::types::{Diff, FileEdit, Hunk};

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

    fn diff_with_files(files: Vec<FileEdit>) -> Diff {
        Diff {
            diff_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            mission_id: Uuid::new_v4(),
            step_id: "S1".into(),
            agent_id: Uuid::new_v4(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            files,
            narrative: String::new(),
            research_refs: vec![],
            risk_decision: None,
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        }
    }

    #[test]
    fn stage_cascade_matches_pipeline_order() {
        // Belt-and-braces. The runner's cascade must match
        // `StageKind::pipeline_order()` exactly; if anyone reorders the
        // `stages` vector in `run` the debug_assert fires.
        let stages: Vec<Box<dyn Stage>> = vec![
            Box::new(LintFormat),
            Box::new(TypeCheck),
            Box::new(UnitTests),
            Box::new(E2e),
            Box::new(DeadCode),
            Box::new(SupplyChain),
            Box::new(SecurityScan),
            Box::new(LayerBoundary),
            Box::new(Iac),
            Box::new(DomainStage::new(vec![])),
            Box::new(EvidenceGate),
        ];
        for (s, k) in stages.iter().zip(StageKind::pipeline_order().iter()) {
            assert_eq!(s.kind(), *k, "cascade order must match RFC 14 §1");
        }
    }

    #[test]
    fn domain_stage_runs_declared_commands() {
        // RFC 64 §8 — when a pack's commands are attached, the Domain stage
        // executes them; a failing command blocks (Fail) like any stage.
        let mut d = diff_with(
            vec!["fn add(a: u32, b: u32) -> u32 { a + b }".into()],
            "src/lib.rs",
        );
        d.narrative = "adds pure add() helper, covered by unit test".into();
        d.research_refs = vec![Uuid::new_v4()];

        let ok = run(&ValidationInput::new(&d).with_domain_commands(vec!["echo ok".into()]));
        let dom = ok
            .stages
            .iter()
            .find(|s| s.stage == StageKind::Domain)
            .expect("domain stage present");
        assert_eq!(dom.status, StageStatus::Pass);

        let bad = run(&ValidationInput::new(&d).with_domain_commands(vec!["exit 1".into()]));
        assert_eq!(bad.outcome, ValidationOutcome::Fail);
        let dom = bad
            .stages
            .iter()
            .find(|s| s.stage == StageKind::Domain)
            .expect("domain stage present");
        assert_eq!(dom.status, StageStatus::Fail);
    }

    #[test]
    fn clean_diff_passes_all_stages_or_skips_them() {
        let mut d = diff_with(
            vec!["fn add(a: u32, b: u32) -> u32 { a + b }".into()],
            "src/lib.rs",
        );
        d.narrative = "adds pure add() helper, covered by unit test".into();
        d.research_refs = vec![Uuid::new_v4()];
        let report = run(&ValidationInput::new(&d));
        assert_eq!(report.outcome, ValidationOutcome::Pass);
        assert!(report.is_pass());
        assert!(report.failed_stage().is_none());
        assert_eq!(report.stages.len(), StageKind::pipeline_order().len());

        let lint = &report.stages[0];
        assert_eq!(lint.stage, StageKind::LintFormat);
        assert_eq!(lint.status, StageStatus::Pass);

        // E2E sits at index 3; no UI touched → it should be Skipped.
        let e2e = report
            .stages
            .iter()
            .find(|s| s.stage == StageKind::E2e)
            .unwrap();
        assert_eq!(e2e.status, StageStatus::Skipped);
    }

    #[test]
    fn fail_on_type_check_throttles_downstream() {
        // Unbalanced braces trip `TypeCheck` (Fail). Every subsequent stage
        // should be Skipped, and `UnitTests` (idx 2) should not have been
        // executed for real — its summary is the throttled `Skipped` line.
        let d = diff_with(vec!["fn x() {".into()], "src/lib.rs");
        let report = run(&ValidationInput::new(&d));
        assert_eq!(report.outcome, ValidationOutcome::Fail);
        assert_eq!(report.failed_stage(), Some(StageKind::TypeCheck));
        assert_eq!(
            report.stages[0].status,
            StageStatus::Pass,
            "LintFormat still passes"
        );

        // Lint ran (Pass), TypeCheck ran (Fail), the rest Skipped.
        for (i, s) in report.stages.iter().enumerate() {
            match s.stage {
                StageKind::LintFormat => assert_eq!(s.status, StageStatus::Pass, "lint idx {i}"),
                StageKind::TypeCheck => {
                    assert_eq!(s.status, StageStatus::Fail, "type-check idx {i}")
                }
                _ => assert_eq!(
                    s.status,
                    StageStatus::Skipped,
                    "downstream idx {i} should be skipped"
                ),
            }
        }
    }

    #[test]
    fn loose_mode_skips_unit_tests() {
        let d = diff_with(
            vec!["fn x() { assert!(false, \"red\"); }".into()],
            "tests/s1_heuristic_v0.rs",
        );
        let report = run(&ValidationInput::new(&d).with_mode(ValidationMode::Loose));
        // Lint + TypeCheck only on Strict gate, per RFC 14 §7. `UnitTests`
        // itself short-circuits to `Skipped` when mode == Loose.
        let unit = report
            .stages
            .iter()
            .find(|s| s.stage == StageKind::UnitTests)
            .unwrap();
        assert_eq!(
            unit.status,
            StageStatus::Skipped,
            "Loose mode must skip UnitTests"
        );
        // RFC 14 §4 throttle must NOT propagate from a `Skipped` stage
        // (only from a `Fail`). `E2E` will also end up `Skipped` here,
        // but because of its OWN gate ("no UI file touched and not PR
        // mode"), NOT because UnitTests was skipped. We detect the
        // difference by checking `summary`: the throttle-branch writes
        // "skipped after stage <X> failed" while the stage-local branch
        // writes its own reason.
        assert!(
            !unit.summary.contains("skipped after stage"),
            "throttle must NOT fire on Skipped stages — UnitTests self-skipped, not throttled"
        );
    }

    #[test]
    fn pr_mode_forces_e2e_on_non_ui_changes() {
        // Even without UI files in the diff, PR mode forces E2E to actually
        // run (E2e stage's own gate relies on `ctx.mode == Pr`).
        let d = diff_with(
            vec!["fn add(a: u32, b: u32) -> u32 { a + b }".into()],
            "src/lib.rs",
        );
        let report = run(&ValidationInput::new(&d).with_mode(ValidationMode::Pr));
        let e2e = report
            .stages
            .iter()
            .find(|s| s.stage == StageKind::E2e)
            .unwrap();
        assert_eq!(
            e2e.status,
            StageStatus::Pass,
            "PR mode forces E2E to run even on non-UI"
        );
    }

    #[test]
    fn same_stage_fail_twice_becomes_critical() {
        // First pass — TypeCheck fails (unbalanced braces).
        let d = diff_with(vec!["fn x() {".into()], "src/lib.rs");
        let first = run(&ValidationInput::new(&d));
        assert_eq!(first.outcome, ValidationOutcome::Fail);
        assert_eq!(first.failed_stage(), Some(StageKind::TypeCheck));

        // Second pass with the previous failure recorded — RFC 14 §8 says
        // "twice → Critical". Repair Engine takes over.
        let second = run(&ValidationInput::new(&d).with_previous_failure(StageKind::TypeCheck));
        assert_eq!(second.outcome, ValidationOutcome::Critical);
        assert_eq!(second.failed_stage(), Some(StageKind::TypeCheck));
    }

    #[test]
    fn different_stage_fail_does_not_become_critical() {
        let d = diff_with(vec!["fn x() {".into()], "src/lib.rs");
        // Previous failed stage was LintFormat; now TypeCheck fails — not a
        // repeat, so outcome is Fail not Critical.
        let report = run(&ValidationInput::new(&d).with_previous_failure(StageKind::LintFormat));
        assert_eq!(report.outcome, ValidationOutcome::Fail);
    }

    #[test]
    fn supply_chain_fires_when_lockfile_touched() {
        let d = diff_with_files(vec![FileEdit {
            path: "Cargo.lock".into(),
            is_new_file: false,
            is_delete: false,
            hunks: vec![Hunk {
                old_start: 0,
                old_end: 0,
                new_lines: vec!["version = 1".into()],
                rationale: "r".into(),
            }],
        }]);
        let report = run(&ValidationInput::new(&d));
        let sc = report
            .stages
            .iter()
            .find(|s| s.stage == StageKind::SupplyChain)
            .unwrap();
        assert_eq!(
            sc.status,
            StageStatus::Pass,
            "SupplyChain runs when Cargo.lock touched"
        );
    }

    #[test]
    fn iac_fires_when_terraform_touched() {
        let d = diff_with_files(vec![FileEdit {
            path: "infra/main.tf".into(),
            is_new_file: false,
            is_delete: false,
            hunks: vec![Hunk {
                old_start: 0,
                old_end: 0,
                new_lines: vec!["resource \"foo\" \"bar\" {}".into()],
                rationale: "r".into(),
            }],
        }]);
        let report = run(&ValidationInput::new(&d));
        let iac = report
            .stages
            .iter()
            .find(|s| s.stage == StageKind::Iac)
            .unwrap();
        assert_eq!(iac.status, StageStatus::Pass, "IaC runs when .tf touched");
    }

    #[test]
    fn bare_done_on_code_is_blocked_by_evidence_gate() {
        let d = diff_with(
            vec!["fn add(a: u32, b: u32) -> u32 { a + b }".into()],
            "src/lib.rs",
        );
        let report = run(&ValidationInput::new(&d));
        assert_eq!(report.outcome, ValidationOutcome::Fail);
        assert_eq!(report.failed_stage(), Some(StageKind::EvidenceGate));
        let gate = report
            .stages
            .iter()
            .find(|s| s.stage == StageKind::EvidenceGate)
            .unwrap();
        assert_eq!(gate.status, StageStatus::Fail);
    }

    #[test]
    fn dead_code_warns_on_let_underscore_binding() {
        let d = diff_with(vec!["let _ = 42;".into()], "src/lib.rs");
        let report = run(&ValidationInput::new(&d));
        let dc = report
            .stages
            .iter()
            .find(|s| s.stage == StageKind::DeadCode)
            .unwrap();
        assert_eq!(dc.status, StageStatus::Warn);
        assert_eq!(dc.findings.len(), 1);
        assert_eq!(dc.findings[0].rule, "dead_binding");
        assert!(dc.findings[0].auto_fixable);
    }
}
