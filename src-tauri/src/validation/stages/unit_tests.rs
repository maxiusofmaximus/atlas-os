// RFC 14 §2 Unit tests affected.
//
// Phase 1 heuristic: a test file is "affected" if the diff touches a file
// whose name matches a `*_heuristic_v0.rs` / `*.test.ts` / `*.spec.ts`
// pattern (the TDD-test stubs the Coding Engine emits, RFC 12 §6). If
// the diff touches an existing test file we PASS (the human or coder
// added a test); if it touches non-test code AND no matching test file
// exists, we WARN ("coverage delta missing"); if a heuristic `#[test]`
// stub still asserts `false` (TDD red), we FAIL — the Coding Engine's
// Create action does this and the impl step is expected to flip it.

use super::{pass, Stage, StageContext};
use crate::validation::types::{Finding, StageKind, StageStatus, StageSummary};

pub struct UnitTests;

impl Stage for UnitTests {
    fn kind(&self) -> StageKind {
        StageKind::UnitTests
    }
    fn run(&self, ctx: &StageContext) -> StageSummary {
        let started = std::time::Instant::now();
        // RFC 14 §7 Loose mode skips Tests entirely.
        if matches!(ctx.mode, crate::validation::types::ValidationMode::Loose) {
            return super::skipped(StageKind::UnitTests, "loose mode → tests skipped");
        }
        let mut findings = Vec::new();
        for f in &ctx.diff.files {
            let is_test_file = is_test_path(&f.path);
            for h in &f.hunks {
                for (idx, line) in h.new_lines.iter().enumerate() {
                    let line_no = h.old_start + idx as u32 + 1;
                    if is_test_file && line.contains("assert!(false") {
                        findings.push(Finding {
                            file: format!("{}:{}", f.path, line_no),
                            rule: "red_tdd_stub".into(),
                            suggestion: Some(
                                "Flip the failing `assert!(false)` to a real assertion.".into(),
                            ),
                            auto_fixable: false,
                        });
                    }
                    if !is_test_file && line.contains("TODO(") {
                        findings.push(Finding {
                            file: format!("{}:{}", f.path, line_no),
                            rule: "missing_coverage".into(),
                            suggestion: Some(
                                "Non-test code change lacks a matching test delta.".into(),
                            ),
                            auto_fixable: false,
                        });
                    }
                }
            }
        }
        let elapsed = started.elapsed().as_millis() as u64;
        if findings.is_empty() {
            return pass(
                StageKind::UnitTests,
                elapsed,
                "unit-tests: no red stubs / missing coverage",
            );
        }
        let any_red = findings.iter().any(|f| f.rule == "red_tdd_stub");
        StageSummary {
            stage: StageKind::UnitTests,
            status: if any_red {
                StageStatus::Fail
            } else {
                StageStatus::Warn
            },
            elapsed_ms: elapsed,
            findings,
            summary: "unit-tests: see findings".into(),
        }
    }
}

fn is_test_path(path: &str) -> bool {
    let p = path.to_lowercase();
    p.ends_with("_heuristic_v0.rs")
        || p.ends_with("_test.rs")
        || p.ends_with(".test.ts")
        || p.ends_with(".spec.ts")
        || p.ends_with("test.rs")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::types::{Diff, FileEdit, Hunk};
    use crate::validation::types::ValidationMode;

    fn ctx<'a>(diff: &'a Diff, mode: ValidationMode) -> StageContext<'a> {
        StageContext {
            diff,
            touched_paths: vec![],
            mode,
        }
    }

    fn diff_with_files(files: Vec<FileEdit>) -> Diff {
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
    fn red_tdd_stub_fails() {
        let d = diff_with_files(vec![FileEdit {
            path: "tests/s1_heuristic_v0.rs".into(),
            is_new_file: true,
            is_delete: false,
            hunks: vec![Hunk {
                old_start: 0,
                old_end: 0,
                new_lines: vec!["#[test] fn t() { assert!(false, \"red\"); }".into()],
                rationale: "r".into(),
            }],
        }]);
        let s = UnitTests.run(&ctx(&d, ValidationMode::Strict));
        assert_eq!(s.status, StageStatus::Fail);
        assert_eq!(s.findings[0].rule, "red_tdd_stub");
    }

    #[test]
    fn non_test_code_with_todo_warns() {
        let d = diff_with_files(vec![FileEdit {
            path: "src/lib.rs".into(),
            is_new_file: false,
            is_delete: false,
            hunks: vec![Hunk {
                old_start: 1,
                old_end: 1,
                new_lines: vec!["// TODO(S1): ship".into()],
                rationale: "r".into(),
            }],
        }]);
        let s = UnitTests.run(&ctx(&d, ValidationMode::Strict));
        assert_eq!(s.status, StageStatus::Warn);
        assert_eq!(s.findings[0].rule, "missing_coverage");
    }

    #[test]
    fn loose_mode_skips() {
        let d = diff_with_files(vec![FileEdit {
            path: "tests/s1_heuristic_v0.rs".into(),
            is_new_file: true,
            is_delete: false,
            hunks: vec![Hunk {
                old_start: 0,
                old_end: 0,
                new_lines: vec!["#[test] fn t() { assert!(false); }".into()],
                rationale: "r".into(),
            }],
        }]);
        let s = UnitTests.run(&ctx(&d, ValidationMode::Loose));
        assert_eq!(s.status, StageStatus::Skipped);
    }

    #[test]
    fn clean_test_diff_passes() {
        let d = diff_with_files(vec![FileEdit {
            path: "tests/s1_heuristic_v0.rs".into(),
            is_new_file: true,
            is_delete: false,
            hunks: vec![Hunk {
                old_start: 0,
                old_end: 0,
                new_lines: vec!["#[test] fn t() { assert!(true); }".into()],
                rationale: "r".into(),
            }],
        }]);
        let s = UnitTests.run(&ctx(&d, ValidationMode::Strict));
        assert_eq!(s.status, StageStatus::Pass);
    }
}
