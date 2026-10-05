// RFC 64 §8 — Domain validation stage.
//
// Runs the check commands a domain pack declares (`[validation] commands` in
// `domain.toml`), e.g. `openscad --check model.scad` or `trimesh check out.stl`.
// Atlas does not invent per-domain checkers: the pack owns the command. When a
// pack declares no commands the stage is `Skipped` (never a fake `Pass`).
//
// Commands run through the platform shell in the process cwd — the same lateral
// boundary as `static_analysis` (RFC 25 §11: external process, never bundled).

use std::path::Path;
use std::process::Command;
use std::time::Instant;

use super::{skipped, Stage, StageContext};
use crate::validation::types::{Finding, StageKind, StageStatus, StageSummary};

pub struct DomainStage {
    commands: Vec<String>,
}

impl DomainStage {
    pub fn new(commands: Vec<String>) -> Self {
        Self { commands }
    }
}

fn run_shell(command: &str, cwd: &Path) -> std::io::Result<std::process::ExitStatus> {
    #[cfg(windows)]
    {
        Command::new("cmd")
            .arg("/C")
            .arg(command)
            .current_dir(cwd)
            .status()
    }
    #[cfg(not(windows))]
    {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(cwd)
            .status()
    }
}

impl Stage for DomainStage {
    fn kind(&self) -> StageKind {
        StageKind::Domain
    }

    fn run(&self, _ctx: &StageContext) -> StageSummary {
        if self.commands.iter().all(|c| c.trim().is_empty()) {
            return skipped(
                StageKind::Domain,
                "no domain validation commands declared ([validation] commands)",
            );
        }
        let started = Instant::now();
        let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let mut findings: Vec<Finding> = Vec::new();
        let mut ran = 0usize;
        for command in &self.commands {
            if command.trim().is_empty() {
                continue;
            }
            ran += 1;
            match run_shell(command, &cwd) {
                Ok(status) if status.success() => {}
                Ok(status) => findings.push(Finding {
                    file: "domain".into(),
                    rule: format!("domain.check: `{command}` exited {status}"),
                    suggestion: None,
                    auto_fixable: false,
                }),
                Err(e) => findings.push(Finding {
                    file: "domain".into(),
                    rule: format!("domain.check: `{command}` failed to launch: {e}"),
                    suggestion: None,
                    auto_fixable: false,
                }),
            }
        }
        let failed = findings.len();
        let elapsed_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
        if failed == 0 {
            StageSummary {
                stage: StageKind::Domain,
                status: StageStatus::Pass,
                elapsed_ms,
                findings,
                summary: format!("{ran} domain check(s) passed"),
            }
        } else {
            StageSummary {
                stage: StageKind::Domain,
                status: StageStatus::Fail,
                elapsed_ms,
                findings,
                summary: format!("{failed} of {ran} domain check(s) failed"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::types::Diff;
    use crate::validation::types::ValidationMode;

    fn diff() -> Diff {
        Diff {
            diff_id: uuid::Uuid::new_v4(),
            plan_id: uuid::Uuid::new_v4(),
            mission_id: uuid::Uuid::new_v4(),
            step_id: "S1".into(),
            agent_id: uuid::Uuid::new_v4(),
            generated_at: "2026-10-05T00:00:00Z".into(),
            files: vec![],
            narrative: String::new(),
            research_refs: vec![],
            risk_decision: None,
            model_id: "m".into(),
            elapsed_ms: 0,
        }
    }

    fn ctx<'a>(d: &'a Diff) -> StageContext<'a> {
        StageContext {
            diff: d,
            touched_paths: vec![],
            mode: ValidationMode::Strict,
        }
    }

    #[test]
    fn empty_commands_skip_never_pass() {
        let d = diff();
        let s = DomainStage::new(vec![]).run(&ctx(&d));
        assert_eq!(s.status, StageStatus::Skipped);
        assert!(s.summary.contains("no domain validation commands"));
    }

    #[test]
    fn passing_and_failing_commands_are_reported() {
        let d = diff();
        let pass = DomainStage::new(vec!["echo domain-ok".into()]).run(&ctx(&d));
        assert_eq!(pass.status, StageStatus::Pass);

        let fail = DomainStage::new(vec!["exit 1".into()]).run(&ctx(&d));
        assert_eq!(fail.status, StageStatus::Fail);
        assert_eq!(fail.findings.len(), 1);
        assert!(fail.findings[0].rule.contains("exit 1"));
    }
}
