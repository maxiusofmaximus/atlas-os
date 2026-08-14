// Atlas OS — `opencode run <mission_id>` subcommand (RFC 25 §3.9).
// Drives Coding → Validation → Repair against the latest persisted Plan
// for a mission. The Prompt Understanding and Planning phases have
// already run (`atlas mission new` / `atlas plan`); `run` re-uses
// their Journal rows and never regenerates them.
//
// Phase 1 — no Execution Supervisor loop / heartbeats: the host process
// drives a single Coding → Validation → Repair pass and exits. Phase 6
// will replace this with a real Supervisor tick loop.

use std::time::Instant;

use anyhow::{Context, Result};
use clap::Args;
use uuid::Uuid;

use super::mission::run_steps_loop;

#[derive(Args, Debug)]
pub struct RunCmd {
    /// Mission UUID whose latest Plan should be executed.
    pub mission_id: String,
    /// Execution mode (RFC 21). Phase 1 only honours `human_in_loop`;
    /// `autonomous` and `yolo` arrive with the full Supervisor (Phase 6).
    #[arg(long, default_value = "human_in_loop")]
    pub mode: String,
}

pub async fn run(cmd: RunCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    let mid = Uuid::parse_str(&cmd.mission_id).context("invalid mission_id")?;

    let plan_row = journal
        .latest_plan_for_mission(mid)?
        .with_context(|| format!("no plan found for mission {mid}"))?;
    let plan_payload = journal
        .plan_payload(plan_row.plan_id)?
        .with_context(|| format!("plan {} payload not found", plan_row.plan_id))?;
    let plan: crate::planning::types::Plan = serde_json::from_str(&plan_payload)?;

    if !plan.blocked.is_empty() {
        anyhow::bail!(
            "plan {} is blocked ({} blocker(s)) — fix gaps and re-run `opencode plan {mid}`",
            plan.plan_id,
            plan.blocked.len()
        );
    }
    if plan.confidence < crate::planning::types::PLAN_CONFIDENCE_THRESHOLD {
        anyhow::bail!(
            "plan {pid} confidence {c:.2} <threshold {t:.2} — Coding Engine blocked (RFC 12 §7)",
            pid = plan.plan_id,
            c = plan.confidence,
            t = crate::planning::types::PLAN_CONFIDENCE_THRESHOLD
        );
    }

    let agent_id = Uuid::new_v4();
    let started = Instant::now();
    let steps = run_steps_loop(&journal, &plan, agent_id, &started)?;
    debug_assert_eq!(steps.mission_id, mid, "steps mission_id mismatch");
    debug_assert_eq!(steps.plan_id, plan.plan_id, "steps plan_id mismatch");

    println!(
        "run[{cmd_mode}] mission={mid} plan={plan_id}  steps_executed — diff={diff}  validation_ok={ok}  repairs={repairs}  elapsed_ms={ms}",
        cmd_mode = cmd.mode,
        plan_id = steps.plan_id,
        diff = steps
            .last_diff_id
            .map(|u| u.to_string())
            .unwrap_or_else(|| "—".into()),
        ok = steps.report_ok,
        repairs = steps.repair_count,
        ms = started.elapsed().as_millis(),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use tempfile::TempDir;
    use uuid::Uuid;

    use super::run_steps_loop;
    use crate::journal::Journal;
    use crate::planning::runner::PlanningInput;
    use crate::prompt::runner::{run as run_prompt, PipelineOptions};

    /// Force-lock a sanitized prompt and hand the Plan back to the caller.
    fn plan_from_forced_prompt(journal: &Journal, prompt: &str) -> crate::planning::types::Plan {
        let (verdict, consolidated_opt) =
            run_prompt(prompt, &PipelineOptions::default_force()).expect("prompt::run");
        let c = consolidated_opt.expect("forced prompt yields a consolidated");
        journal
            .save_verdict(&verdict, Some(c.mission_id))
            .expect("save_verdict");
        journal
            .create_mission(c.mission_id, &c.mission_statement)
            .expect("create_mission");
        journal.save_consolidated(&c).expect("save_consolidated");
        crate::planning::runner::run(&PlanningInput {
            consolidated: &c,
            verdict: &verdict,
            clarification_answers: vec![],
            research_runs: vec![],
        })
        .expect("planning::run")
    }

    #[test]
    fn run_steps_loop_empty_plan_yields_no_diffs() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let mut plan = plan_from_forced_prompt(&journal, "make hello function");
        plan.steps = Vec::new();
        let r =
            run_steps_loop(&journal, &plan, Uuid::new_v4(), &Instant::now()).expect("steps loop");
        assert!(r.last_diff_id.is_none());
        assert!(r.report_ok);
        assert_eq!(r.repair_count, 0);
    }

    #[test]
    fn run_steps_loop_executes_steps_and_persists_tail() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let plan = plan_from_forced_prompt(&journal, "build auth folder and add tests");
        let r =
            run_steps_loop(&journal, &plan, Uuid::new_v4(), &Instant::now()).expect("steps loop");
        let tail = journal.diff_tail(50).expect("tail");
        if let Some(diff_id) = r.last_diff_id {
            assert!(tail.iter().any(|d| d.diff_id == diff_id));
        } else {
            assert!(tail.is_empty());
        }
    }

    #[test]
    fn run_steps_loop_persists_executing_and_done_checkpoints() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let plan = plan_from_forced_prompt(&journal, "create readme file");
        let mid = plan.mission_id;
        let r =
            run_steps_loop(&journal, &plan, Uuid::new_v4(), &Instant::now()).expect("steps loop");
        let ckpts = journal.checkpoint_tail(50).expect("tail");
        let for_mission: Vec<_> = ckpts.iter().filter(|c| c.mission_id == mid).collect();
        assert!(
            for_mission.len() >= 2,
            "expected at least 2 checkpoints (Executing entry + Done/Halted exit) for mission, got {}",
            for_mission.len()
        );
        assert_eq!(r.mission_id, mid);
        assert_eq!(r.plan_id, plan.plan_id);
    }
}
