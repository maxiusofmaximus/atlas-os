// Atlas OS — EvidenceGate terminal stage (RFC 30 §2.1, Canny done-gate).
//
// Runs last in the RFC 14 cascade: when code changed, the diff itself
// must carry evidence (narrative plus test delta, research refs, or an
// explicit risk decision). A bare "done" with empty hands fails here,
// so no commit and no supervisor MarkDone can follow.

use super::{pass, skipped, Stage, StageContext};
use crate::validation::evidence::{diff_touches_code, diff_touches_tests, MIN_NARRATIVE_LEN};
use crate::validation::types::{Finding, StageKind, StageStatus, StageSummary, ValidationMode};

pub struct EvidenceGate;

impl Stage for EvidenceGate {
    fn kind(&self) -> StageKind {
        StageKind::EvidenceGate
    }

    fn run(&self, ctx: &StageContext) -> StageSummary {
        let started = std::time::Instant::now();
        if !diff_touches_code(ctx.diff) {
            return pass(
                StageKind::EvidenceGate,
                started.elapsed().as_millis() as u64,
                "evidence-gate: no code touched, no check required",
            );
        }
        if matches!(ctx.mode, ValidationMode::Loose) {
            return skipped(
                StageKind::EvidenceGate,
                "loose mode: exploratory commit, gate advisory only",
            );
        }
        let narrative_ok = ctx.diff.narrative.trim().len() >= MIN_NARRATIVE_LEN;
        let artefact_ok = diff_touches_tests(ctx.diff)
            || !ctx.diff.research_refs.is_empty()
            || ctx.diff.risk_decision.is_some();
        let elapsed = started.elapsed().as_millis() as u64;
        if narrative_ok && artefact_ok {
            return pass(
                StageKind::EvidenceGate,
                elapsed,
                "evidence-gate: narrative + test/research artefact present",
            );
        }
        let mut findings = Vec::new();
        if !narrative_ok {
            findings.push(Finding {
                file: "<diff>".into(),
                rule: "missing_narrative".into(),
                suggestion: Some("State what changed and why in the diff narrative.".into()),
                auto_fixable: false,
            });
        }
        if !artefact_ok {
            findings.push(Finding {
                file: "<diff>".into(),
                rule: "missing_check_evidence".into(),
                suggestion: Some(
                    "Touch a test file, cite research refs, or record a risk decision.".into(),
                ),
                auto_fixable: false,
            });
        }
        let both_missing = !narrative_ok && !artefact_ok;
        StageSummary {
            stage: StageKind::EvidenceGate,
            status: if both_missing {
                StageStatus::Fail
            } else {
                StageStatus::Warn
            },
            elapsed_ms: elapsed,
            findings,
            summary: "evidence-gate: done claim without evidence".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::types::{Diff, FileEdit, Hunk};

    fn diff_with(path: &str, narrative: &str, research_refs: Vec<uuid::Uuid>) -> Diff {
        Diff {
            diff_id: uuid::Uuid::new_v4(),
            plan_id: uuid::Uuid::new_v4(),
            mission_id: uuid::Uuid::new_v4(),
            step_id: "S1".into(),
            agent_id: uuid::Uuid::new_v4(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            files: vec![FileEdit {
                path: path.into(),
                is_new_file: false,
                is_delete: false,
                hunks: vec![Hunk {
                    old_start: 0,
                    old_end: 0,
                    new_lines: vec!["fn add(a: u32, b: u32) -> u32 { a + b }".into()],
                    rationale: "r".into(),
                }],
            }],
            narrative: narrative.into(),
            research_refs,
            risk_decision: None,
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        }
    }

    fn ctx<'a>(diff: &'a Diff, mode: ValidationMode) -> StageContext<'a> {
        StageContext {
            diff,
            touched_paths: vec![],
            mode,
        }
    }

    #[test]
    fn docs_only_passes_without_evidence() {
        let d = diff_with("docs/guide.md", "", vec![]);
        let s = EvidenceGate.run(&ctx(&d, ValidationMode::Strict));
        assert_eq!(s.status, StageStatus::Pass);
    }

    #[test]
    fn bare_done_on_code_fails() {
        let d = diff_with("src/lib.rs", "", vec![]);
        let s = EvidenceGate.run(&ctx(&d, ValidationMode::Strict));
        assert_eq!(s.status, StageStatus::Fail);
        assert!(s.findings.iter().any(|f| f.rule == "missing_narrative"));
        assert!(s
            .findings
            .iter()
            .any(|f| f.rule == "missing_check_evidence"));
    }

    #[test]
    fn narrative_plus_test_file_passes() {
        let d = diff_with(
            "tests/s1_heuristic_v0.rs",
            "adds add() with a green unit test",
            vec![],
        );
        let s = EvidenceGate.run(&ctx(&d, ValidationMode::Strict));
        assert_eq!(s.status, StageStatus::Pass);
    }

    #[test]
    fn narrative_alone_warns_instead_of_failing() {
        let d = diff_with("src/lib.rs", "pure rename, no behaviour change", vec![]);
        let s = EvidenceGate.run(&ctx(&d, ValidationMode::Strict));
        assert_eq!(s.status, StageStatus::Warn);
        assert_eq!(s.findings.len(), 1);
    }

    #[test]
    fn loose_mode_never_blocks_on_evidence() {
        let d = diff_with("src/lib.rs", "", vec![]);
        let s = EvidenceGate.run(&ctx(&d, ValidationMode::Loose));
        assert_eq!(s.status, StageStatus::Skipped);
    }
}
