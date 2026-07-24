// RFC 14 §2 Lint + Format (rápido).
//
// Phase 1 heuristic: scan the `Diff` for sentinel markers the Coding
// Engine emits (heuristic todo markers, debug `dbg!`, `println!`,
// `console.log`, `fmt.Println`). The real Biome / Ruff / clippy / etc.
// integration lands in Phase 2 once the Model Orchestrator (RFC 04) is
// available — the heuristic is enough to drive the round-trip tests.

use super::{pass, Stage, StageContext};
use crate::validation::types::{Finding, StageKind, StageStatus, StageSummary};

pub struct LintFormat;

impl Stage for LintFormat {
    fn kind(&self) -> StageKind {
        StageKind::LintFormat
    }
    fn run(&self, ctx: &StageContext) -> StageSummary {
        let started = std::time::Instant::now();
        let mut findings = Vec::new();
        for f in &ctx.diff.files {
            for h in &f.hunks {
                for (idx, line) in h.new_lines.iter().enumerate() {
                    let line_no = h.old_start + idx as u32 + 1;
                    if let Some(rule) = detect_smell(line) {
                        findings.push(Finding {
                            file: format!("{}:{}", f.path, line_no),
                            rule: rule.to_string(),
                            suggestion: suggestion_for(rule),
                            auto_fixable: matches!(rule, "no_todo" | "no_dbg"),
                        });
                    }
                }
            }
        }
        let elapsed = started.elapsed().as_millis() as u64;
        if findings.is_empty() {
            return pass(StageKind::LintFormat, elapsed, "lint: no findings");
        }
        let critical = findings.iter().any(|f| !f.auto_fixable);
        StageSummary {
            stage: StageKind::LintFormat,
            status: if critical {
                StageStatus::Fail
            } else {
                StageStatus::Warn
            },
            elapsed_ms: elapsed,
            findings,
            summary: "lint: heuristic smells found".into(),
        }
    }
}

fn detect_smell(line: &str) -> Option<&'static str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("// todo") || trimmed.starts_with("TODO") || trimmed.starts_with("FIXME")
    {
        return Some("no_todo");
    }
    if trimmed.starts_with("dbg!") || trimmed.contains("dbg!(") {
        return Some("no_dbg");
    }
    if trimmed.starts_with("println!") || trimmed.contains("println!(") {
        return Some("no_println");
    }
    if trimmed.contains("console.log(") {
        return Some("no_console_log");
    }
    if trimmed.contains("fmt.Println(") || trimmed.contains("fmt.Printf(") {
        return Some("no_fmt_println");
    }
    None
}

fn suggestion_for(rule: &str) -> Option<String> {
    match rule {
        "no_todo" => Some("Resolve the TODO before commit or move it to the issue tracker.".into()),
        "no_dbg" => Some("Remove the `dbg!` call.".into()),
        "no_println" => Some("Replace `println!` with `tracing::info!`.".into()),
        "no_console_log" => Some("Remove the `console.log` (use a logger).".into()),
        "no_fmt_println" => Some("Replace `fmt.Println` with a structured logger.".into()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::types::{Diff, FileEdit, Hunk};
    use crate::validation::types::{StageStatus, ValidationMode};

    fn ctx<'a>(diff: &'a Diff, touched: Vec<String>) -> StageContext<'a> {
        StageContext {
            diff,
            touched_paths: touched,
            mode: ValidationMode::Strict,
        }
    }

    fn diff_with_lines(lines: &[&str]) -> Diff {
        Diff {
            diff_id: uuid::Uuid::new_v4(),
            plan_id: uuid::Uuid::new_v4(),
            mission_id: uuid::Uuid::new_v4(),
            step_id: "S1".into(),
            agent_id: uuid::Uuid::new_v4(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            files: vec![FileEdit {
                path: "src/lib.rs".into(),
                is_new_file: false,
                is_delete: false,
                hunks: vec![Hunk {
                    old_start: 0,
                    old_end: 0,
                    new_lines: lines.iter().map(|s| s.to_string()).collect(),
                    rationale: "r".into(),
                }],
            }],
            narrative: "n".into(),
            research_refs: vec![],
            risk_decision: None,
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        }
    }

    #[test]
    fn clean_diff_passes() {
        let d = diff_with_lines(&["fn add(a: u32, u32) -> u32 { a + b }"]);
        let s = LintFormat.run(&ctx(&d, vec![]));
        assert_eq!(s.status, StageStatus::Pass);
    }

    #[test]
    fn todo_marker_emits_no_todo_finding() {
        let d = diff_with_lines(&["// todo(S1): ship it", "fn x() {}"]);
        let s = LintFormat.run(&ctx(&d, vec![]));
        assert_eq!(
            s.status,
            StageStatus::Warn,
            "todo is auto-fixable → warn not fail"
        );
        assert_eq!(s.findings.len(), 1);
        assert_eq!(s.findings[0].rule, "no_todo");
        assert!(s.findings[0].auto_fixable);
    }

    #[test]
    fn println_emits_no_println_finding_and_fails() {
        let d = diff_with_lines(&["println!(\"hi\");"]);
        let s = LintFormat.run(&ctx(&d, vec![]));
        assert_eq!(
            s.status,
            StageStatus::Fail,
            "println is not auto-fixable → fail"
        );
        assert_eq!(s.findings[0].rule, "no_println");
        assert!(!s.findings[0].auto_fixable);
    }
}
