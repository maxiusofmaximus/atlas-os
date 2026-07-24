// RFC 14 §2 Type-check (cargo check / tsc / mypy / go vet).
//
// Phase 1 heuristic: scan the `Diff` for obvious syntax holes the Coding
// Engine heuristic might emit (unbalanced braces, missing `fn`/`func`, a
// `=>` in a position that doesn't match Rust/TS). The real toolchains
// land in Phase 2; the heuristic catches the regression-class mistakes.

use super::{pass, Stage, StageContext};
use crate::validation::types::{Finding, StageKind, StageStatus, StageSummary};

pub struct TypeCheck;

impl Stage for TypeCheck {
    fn kind(&self) -> StageKind {
        StageKind::TypeCheck
    }
    fn run(&self, ctx: &StageContext) -> StageSummary {
        let started = std::time::Instant::now();
        let mut findings = Vec::new();
        for f in &ctx.diff.files {
            for h in &f.hunks {
                let joined: String = h.new_lines.join("\n");
                let opens = joined.matches('{').count() as i32;
                let closes = joined.matches('}').count() as i32;
                if opens != closes {
                    findings.push(Finding {
                        file: f.path.clone(),
                        rule: "unbalanced_braces".into(),
                        suggestion: Some(format!("opened {opens} `{{` but closed {closes} `}}`")),
                        auto_fixable: false,
                    });
                }
                // Detect Rust snippet left without `fn` keyword (heuristic
                // Coding Engine sometimes drops it on Create).
                let first_non_doc = h
                    .new_lines
                    .iter()
                    .find(|l| !l.trim().is_empty() && !l.starts_with("//!"));
                if let Some(line) = first_non_doc {
                    let t = line.trim();
                    if t.contains('(')
                        && t.contains(')')
                        && t.contains("->")
                        && !t.contains("fn ")
                        && !t.contains("function ")
                        && !t.contains("const ")
                    {
                        findings.push(Finding {
                            file: f.path.clone(),
                            rule: "missing_fn_keyword".into(),
                            suggestion: Some("Rust call signature needs `fn`.".into()),
                            auto_fixable: false,
                        });
                    }
                }
            }
        }
        let elapsed = started.elapsed().as_millis() as u64;
        if findings.is_empty() {
            return pass(StageKind::TypeCheck, elapsed, "type-check: clean");
        }
        StageSummary {
            stage: StageKind::TypeCheck,
            status: StageStatus::Fail,
            elapsed_ms: elapsed,
            findings,
            summary: "type-check: heuristic found blocking holes".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::types::{Diff, FileEdit, Hunk};
    use crate::validation::types::{StageStatus, ValidationMode};

    fn ctx<'a>(diff: &'a Diff) -> StageContext<'a> {
        StageContext {
            diff,
            touched_paths: vec![],
            mode: ValidationMode::Strict,
        }
    }

    fn diff_with(files: Vec<FileEdit>) -> Diff {
        Diff {
            diff_id: uuid::Uuid::new_v4(),
            plan_id: uuid::Uuid::new_v4(),
            mission_id: uuid::Uuid::new_v4(),
            step_id: "S1".into(),
            agent_id: uuid::Uuid::new_v4(),
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
    fn balanced_diff_passes() {
        let d = diff_with(vec![FileEdit {
            path: "src/lib.rs".into(),
            is_new_file: false,
            is_delete: false,
            hunks: vec![Hunk {
                old_start: 0,
                old_end: 0,
                new_lines: vec!["fn x() {}".into()],
                rationale: "r".into(),
            }],
        }]);
        let s = TypeCheck.run(&ctx(&d));
        assert_eq!(s.status, StageStatus::Pass);
    }

    #[test]
    fn unbalanced_braces_fail() {
        let d = diff_with(vec![FileEdit {
            path: "src/lib.rs".into(),
            is_new_file: false,
            is_delete: false,
            hunks: vec![Hunk {
                old_start: 0,
                old_end: 0,
                new_lines: vec!["fn x() {".into()],
                rationale: "r".into(),
            }],
        }]);
        let s = TypeCheck.run(&ctx(&d));
        assert_eq!(s.status, StageStatus::Fail);
        assert_eq!(s.findings[0].rule, "unbalanced_braces");
    }

    #[test]
    fn missing_fn_keyword_detected() {
        let d = diff_with(vec![FileEdit {
            path: "src/lib.rs".into(),
            is_new_file: true,
            is_delete: false,
            hunks: vec![Hunk {
                old_start: 0,
                old_end: 0,
                new_lines: vec![
                    "//! doc".into(),
                    "add(a: u32, b: u32) -> u32 { a + b }".into(),
                ],
                rationale: "r".into(),
            }],
        }]);
        let s = TypeCheck.run(&ctx(&d));
        assert_eq!(s.status, StageStatus::Fail);
        assert_eq!(s.findings[0].rule, "missing_fn_keyword");
    }
}
