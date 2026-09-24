// Atlas OS — Evidence-gated done (RFC 30 §2.1, Canny pattern).
//
// Stops agents from landing "done" on a claim: every DoneClaim must cite
// machine-checkable evidence (a green ValidationReport plus at least one
// executed check), and the terminal EvidenceGate pipeline stage blocks
// diffs that carry neither narrative nor test/research artefacts.
// Facts block; advisory judgments never do — the gate is fully
// deterministic (no model call, no IO), so `replay` over the Journal
// always re-derives the same verdict.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::coding::types::Diff;
use crate::validation::types::{StageKind, StageStatus, ValidationMode, ValidationReport};

pub const MIN_NARRATIVE_LEN: usize = 10;
pub const MIN_CLAIM_LEN: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    ValidationReport,
    TestOutput,
    TypecheckOutput,
    LintOutput,
    E2eOutput,
    ManualNote,
}

impl EvidenceKind {
    pub fn tag(&self) -> &'static str {
        match self {
            EvidenceKind::ValidationReport => "validation_report",
            EvidenceKind::TestOutput => "test_output",
            EvidenceKind::TypecheckOutput => "typecheck_output",
            EvidenceKind::LintOutput => "lint_output",
            EvidenceKind::E2eOutput => "e2e_output",
            EvidenceKind::ManualNote => "manual_note",
        }
    }

    pub fn is_executed_check(&self) -> bool {
        matches!(
            self,
            EvidenceKind::TestOutput
                | EvidenceKind::TypecheckOutput
                | EvidenceKind::LintOutput
                | EvidenceKind::E2eOutput
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub kind: EvidenceKind,
    pub reference: String,
}

impl EvidenceItem {
    pub fn new(kind: EvidenceKind, reference: impl Into<String>) -> Self {
        Self {
            kind,
            reference: reference.into(),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.reference.trim().is_empty()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DoneClaim {
    pub diff_id: Uuid,
    pub mission_id: Uuid,
    pub report_id: Uuid,
    pub claim: String,
    pub evidence: Vec<EvidenceItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DoneGateVerdict {
    Allowed {
        summary: String,
    },
    Blocked {
        reason: String,
        missing: Vec<String>,
    },
}

impl DoneGateVerdict {
    pub fn allowed(&self) -> bool {
        matches!(self, DoneGateVerdict::Allowed { .. })
    }

    pub fn missing(&self) -> &[String] {
        match self {
            DoneGateVerdict::Allowed { .. } => &[],
            DoneGateVerdict::Blocked { missing, .. } => missing,
        }
    }
}

pub fn is_test_path(path: &str) -> bool {
    let p = path.to_lowercase();
    p.ends_with("_heuristic_v0.rs")
        || p.ends_with("_test.rs")
        || p.ends_with(".test.ts")
        || p.ends_with(".spec.ts")
        || p.ends_with("test.rs")
}

pub fn is_code_path(path: &str) -> bool {
    let p = path.replace('\\', "/").to_lowercase();
    let name = p.rsplit('/').next().unwrap_or(&p);
    if matches!(
        name,
        "cargo.lock" | "pnpm-lock.yaml" | "package-lock.json" | "yarn.lock" | "flake.lock"
    ) {
        return false;
    }
    let non_code_exts = [
        ".md",
        ".mdx",
        ".png",
        ".jpg",
        ".jpeg",
        ".gif",
        ".svg",
        ".ico",
        ".webp",
        ".lock",
        ".drawio",
        ".excalidraw",
    ];
    if non_code_exts.iter().any(|e| p.ends_with(e)) {
        return false;
    }
    true
}

pub fn diff_touches_code(diff: &Diff) -> bool {
    diff.files.iter().any(|f| is_code_path(&f.path))
}

pub fn diff_touches_tests(diff: &Diff) -> bool {
    diff.files.iter().any(|f| is_test_path(&f.path))
}

pub fn collect_diff_evidence(diff: &Diff, report: &ValidationReport) -> Vec<EvidenceItem> {
    let mut out = vec![EvidenceItem::new(
        EvidenceKind::ValidationReport,
        report.report_id.to_string(),
    )];
    for s in &report.stages {
        if s.status != StageStatus::Pass {
            continue;
        }
        let kind = match s.stage {
            StageKind::UnitTests => EvidenceKind::TestOutput,
            StageKind::TypeCheck => EvidenceKind::TypecheckOutput,
            StageKind::LintFormat => EvidenceKind::LintOutput,
            StageKind::E2e => EvidenceKind::E2eOutput,
            _ => continue,
        };
        out.push(EvidenceItem::new(
            kind,
            format!("{}:{}", report.report_id, s.stage.tag()),
        ));
    }
    let _ = diff;
    out
}

pub fn evaluate_done_claim(report: &ValidationReport, claim: &DoneClaim) -> DoneGateVerdict {
    if claim.report_id != report.report_id
        || claim.diff_id != report.diff_id
        || claim.mission_id != report.mission_id
    {
        return DoneGateVerdict::Blocked {
            reason: "claim cites a different report/diff/mission than the one under review".into(),
            missing: vec!["report_identity".into()],
        };
    }
    if !report.is_pass() {
        let stage = report
            .failed_stage()
            .map(|k| k.tag().to_string())
            .unwrap_or_else(|| report.outcome.tag().to_string());
        return DoneGateVerdict::Blocked {
            reason: format!(
                "report is not green (outcome={}) at stage {stage}",
                report.outcome.tag()
            ),
            missing: vec!["green_report".into()],
        };
    }
    if claim.claim.trim().len() < MIN_CLAIM_LEN {
        return DoneGateVerdict::Blocked {
            reason: "done claim states no verifiable outcome".into(),
            missing: vec!["claim_text".into()],
        };
    }
    if claim.evidence.is_empty() || claim.evidence.iter().any(|e| !e.is_valid()) {
        return DoneGateVerdict::Blocked {
            reason: "done claim cites no usable evidence reference".into(),
            missing: vec!["evidence_reference".into()],
        };
    }
    let report_ref = report.report_id.to_string();
    if !claim
        .evidence
        .iter()
        .any(|e| e.kind == EvidenceKind::ValidationReport && e.reference.trim() == report_ref)
    {
        return DoneGateVerdict::Blocked {
            reason: "done claim does not cite the green validation report".into(),
            missing: vec!["validation_report_ref".into()],
        };
    }
    let has_check = claim.evidence.iter().any(|e| e.kind.is_executed_check());
    if !has_check {
        return DoneGateVerdict::Blocked {
            reason: "code changed but no executed check (test/typecheck/lint/e2e) is cited".into(),
            missing: vec!["check_evidence".into()],
        };
    }
    if matches!(report.mode, ValidationMode::Pr)
        && !claim
            .evidence
            .iter()
            .any(|e| e.kind == EvidenceKind::E2eOutput)
    {
        return DoneGateVerdict::Blocked {
            reason: "PR mode requires an e2e evidence item".into(),
            missing: vec!["e2e_evidence".into()],
        };
    }
    DoneGateVerdict::Allowed {
        summary: format!(
            "done allowed: {} evidence items over {} report {}",
            claim.evidence.len(),
            report.mode.tag(),
            &report_ref[..8.min(report_ref.len())]
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::types::{FileEdit, Hunk};
    use crate::validation::types::{StageStatus, StageSummary, ValidationOutcome};

    fn file(path: &str, lines: Vec<&str>) -> FileEdit {
        FileEdit {
            path: path.into(),
            is_new_file: false,
            is_delete: false,
            hunks: vec![Hunk {
                old_start: 0,
                old_end: 0,
                new_lines: lines.into_iter().map(|s| s.to_string()).collect(),
                rationale: "r".into(),
            }],
        }
    }

    fn report_with(outcome: ValidationOutcome, mode: ValidationMode) -> ValidationReport {
        ValidationReport {
            report_id: Uuid::new_v4(),
            diff_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            mission_id: Uuid::new_v4(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            mode,
            stages: vec![],
            outcome,
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        }
    }

    fn claim_for(report: &ValidationReport, kinds: Vec<EvidenceKind>) -> DoneClaim {
        DoneClaim {
            diff_id: report.diff_id,
            mission_id: report.mission_id,
            report_id: report.report_id,
            claim: "unit tests pass, lint clean — done".into(),
            evidence: kinds
                .into_iter()
                .map(|k| {
                    let reference = if k == EvidenceKind::ValidationReport {
                        report.report_id.to_string()
                    } else {
                        format!("{}:{}", report.report_id, k.tag())
                    };
                    EvidenceItem::new(k, reference)
                })
                .collect(),
        }
    }

    #[test]
    fn green_report_with_check_evidence_is_allowed() {
        let r = report_with(ValidationOutcome::Pass, ValidationMode::Strict);
        let c = claim_for(
            &r,
            vec![EvidenceKind::ValidationReport, EvidenceKind::TestOutput],
        );
        let v = evaluate_done_claim(&r, &c);
        assert!(v.allowed(), "expected allowed, got {v:?}");
        assert!(v.missing().is_empty());
    }

    #[test]
    fn failed_report_blocks_even_with_evidence() {
        let r = report_with(ValidationOutcome::Fail, ValidationMode::Strict);
        let c = claim_for(
            &r,
            vec![EvidenceKind::ValidationReport, EvidenceKind::TestOutput],
        );
        let v = evaluate_done_claim(&r, &c);
        assert!(!v.allowed());
        assert_eq!(v.missing(), &["green_report".to_string()]);
    }

    #[test]
    fn bare_done_without_check_evidence_is_blocked() {
        let r = report_with(ValidationOutcome::Pass, ValidationMode::Strict);
        let c = claim_for(&r, vec![EvidenceKind::ValidationReport]);
        let v = evaluate_done_claim(&r, &c);
        assert!(!v.allowed());
        assert_eq!(v.missing(), &["check_evidence".to_string()]);
    }

    #[test]
    fn manual_note_alone_never_counts_as_check() {
        let r = report_with(ValidationOutcome::Pass, ValidationMode::Strict);
        let c = claim_for(
            &r,
            vec![EvidenceKind::ValidationReport, EvidenceKind::ManualNote],
        );
        assert!(!evaluate_done_claim(&r, &c).allowed());
    }

    #[test]
    fn pr_mode_requires_e2e_evidence() {
        let r = report_with(ValidationOutcome::Pass, ValidationMode::Pr);
        let without_e2e = claim_for(
            &r,
            vec![EvidenceKind::ValidationReport, EvidenceKind::TestOutput],
        );
        let v = evaluate_done_claim(&r, &without_e2e);
        assert!(!v.allowed());
        assert_eq!(v.missing(), &["e2e_evidence".to_string()]);
        let with_e2e = claim_for(
            &r,
            vec![
                EvidenceKind::ValidationReport,
                EvidenceKind::TestOutput,
                EvidenceKind::E2eOutput,
            ],
        );
        assert!(evaluate_done_claim(&r, &with_e2e).allowed());
    }

    #[test]
    fn stale_report_identity_is_blocked() {
        let r = report_with(ValidationOutcome::Pass, ValidationMode::Strict);
        let mut c = claim_for(
            &r,
            vec![EvidenceKind::ValidationReport, EvidenceKind::TestOutput],
        );
        c.report_id = Uuid::new_v4();
        let v = evaluate_done_claim(&r, &c);
        assert!(!v.allowed());
        assert_eq!(v.missing(), &["report_identity".to_string()]);
    }

    #[test]
    fn docs_only_diff_touches_no_code() {
        let d = crate::coding::types::Diff {
            diff_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            mission_id: Uuid::new_v4(),
            step_id: "S1".into(),
            agent_id: Uuid::new_v4(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            files: vec![file("docs/guide.md", vec!["# hi"])],
            narrative: String::new(),
            research_refs: vec![],
            risk_decision: None,
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        };
        assert!(!diff_touches_code(&d));
        assert!(is_code_path("src/lib.rs"));
        assert!(!is_code_path("Cargo.lock"));
    }

    #[test]
    fn collect_diff_evidence_maps_passing_stages() {
        let mut r = report_with(ValidationOutcome::Pass, ValidationMode::Strict);
        r.stages = vec![
            StageSummary {
                stage: StageKind::UnitTests,
                status: StageStatus::Pass,
                elapsed_ms: 1,
                findings: vec![],
                summary: "ok".into(),
            },
            StageSummary {
                stage: StageKind::TypeCheck,
                status: StageStatus::Fail,
                elapsed_ms: 1,
                findings: vec![],
                summary: "bad".into(),
            },
        ];
        let d = crate::coding::types::Diff {
            diff_id: r.diff_id,
            plan_id: r.plan_id,
            mission_id: r.mission_id,
            step_id: "S1".into(),
            agent_id: Uuid::new_v4(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            files: vec![],
            narrative: String::new(),
            research_refs: vec![],
            risk_decision: None,
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        };
        let items = collect_diff_evidence(&d, &r);
        assert!(items
            .iter()
            .any(|e| e.kind == EvidenceKind::ValidationReport));
        assert!(items.iter().any(|e| e.kind == EvidenceKind::TestOutput));
        assert!(!items
            .iter()
            .any(|e| e.kind == EvidenceKind::TypecheckOutput));
    }
}
