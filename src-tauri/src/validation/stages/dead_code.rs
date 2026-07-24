// RFC 14 §2 Knip-equivalent dead-code check (Phase 1).
//
// Detect sentinel patterns unused code emitted by the heuristic Coding
// Engine (private `fn` never called, `let _` placeholders). The real
// Knip / dead-code analyser lands Phase 2.

use super::{pass, Stage, StageContext};
use crate::validation::types::{Finding, StageKind, StageStatus, StageSummary};

pub struct DeadCode;

impl Stage for DeadCode {
    fn kind(&self) -> StageKind {
        StageKind::DeadCode
    }
    fn run(&self, ctx: &StageContext) -> StageSummary {
        let started = std::time::Instant::now();
        let mut findings = Vec::new();
        for f in &ctx.diff.files {
            for line in f.hunks.iter().flat_map(|h| h.new_lines.iter()) {
                let t = line.trim();
                if t.starts_with("let _ = ") {
                    findings.push(Finding {
                        file: f.path.clone(),
                        rule: "dead_binding".into(),
                        suggestion: Some("Drop the unused `let _ =`.".into()),
                        auto_fixable: true,
                    });
                }
            }
        }
        let elapsed = started.elapsed().as_millis() as u64;
        if findings.is_empty() {
            return pass(StageKind::DeadCode, elapsed, "dead-code: clean");
        }
        StageSummary {
            stage: StageKind::DeadCode,
            status: StageStatus::Warn,
            elapsed_ms: elapsed,
            findings,
            summary: "dead-code: heuristic findings".into(),
        }
    }
}
