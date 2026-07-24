// OpenCode OS — `opencode resume <mission_id>` subcommand (RFC 25 §3.9 /
// RFC 19). Loads the latest `MissionCheckpoint`, re-runs the Planning
// Engine from the upstream consolidated/verdict, and drives the
// Coding → Validation → Repair loop afresh using the supervisor-
// supplied `last_*_id` hints.
//
// Phase 1 — no real supervisor heartbeat; we replay with a fresh
// agent_id and zeroed budget tally. Phase 6 wires the live Supervisor
// state machine.

use std::time::Instant;

use anyhow::{Context, Result};
use clap::Args;
use uuid::Uuid;

use super::mission::run_steps_loop;
use crate::planning::runner::{ClarificationAnswer, PlanningInput};

#[derive(Args, Debug)]
pub struct ResumeCmd {
    /// Mission UUID whose latest checkpoint will be resumed from.
    pub mission_id: String,
}

pub async fn run(cmd: ResumeCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    let mid = Uuid::parse_str(&cmd.mission_id).context("invalid mission_id")?;

    let ckpt_row = journal
        .latest_checkpoint(mid)?
        .with_context(|| format!("no checkpoint for mission {mid}"))?;
    let ckpt_payload = journal
        .checkpoint_payload(ckpt_row.checkpoint_id)?
        .with_context(|| format!("checkpoint {} payload not found", ckpt_row.checkpoint_id))?;
    let ckpt: crate::supervisor::types::MissionCheckpoint = serde_json::from_str(&ckpt_payload)?;

    let consolidated = journal
        .latest_consolidated_for_mission(mid)?
        .with_context(|| format!("no consolidated mission for {mid}"))?;
    let verdict = journal
        .latest_verdict_for_mission(mid)?
        .with_context(|| format!("no verdict linked to mission {mid}"))?;

    println!(
        "resuming mission={mid} from checkpoint {cid} in phase {phase}, last plan {plan:?}",
        cid = ckpt_row.checkpoint_id,
        phase = ckpt_row.phase,
        plan = ckpt.current_plan_id,
    );

    let plan = crate::planning::runner::run(&PlanningInput {
        consolidated: &consolidated,
        verdict: &verdict,
        clarification_answers: Vec::<ClarificationAnswer>::new(),
        research_runs: Vec::new(),
    })?;
    journal.save_plan(&plan)?;
    if !plan.blocked.is_empty() {
        anyhow::bail!(
            "re-planned {mid} is still blocked ({n} blocker(s))",
            n = plan.blocked.len()
        );
    }

    let agent_id = Uuid::new_v4();
    let started = Instant::now();
    let steps = run_steps_loop(&journal, &plan, agent_id, &started)?;

    println!(
        "resume mission={mid} from ckpt={cid} (phase {phase})  new_plan={pid}  validation_ok={ok}  repairs={rep}  elapsed_ms={ms}",
        cid = ckpt_row.checkpoint_id,
        phase = ckpt_row.phase,
        pid = plan.plan_id,
        ok = steps.report_ok,
        rep = steps.repair_count,
        ms = started.elapsed().as_millis(),
    );
    Ok(())
}
