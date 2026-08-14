// Atlas OS — single entry-point for the "create a mission from a
// raw prompt" flow. Sits between IPC / CLI and the engine stack so
// both surfaces call the same code path (RFC 25 §3.1.1 SOP).
//
// Phase 1: heuristic-only pipeline (Prompt → Planning → Coding →
// Validation → Repair). Future phases swap engines via Phase 2
// trait-objects without touching this file's public surface.

use std::time::Instant;

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::planning::runner::{ClarificationAnswer, PlanningInput};
use crate::prompt::runner::{run as run_prompt, PipelineOptions};

/// Light-weight per-run summary surfaced to callers. The full
/// step-by-step artefacts live in the Journal (`prompt_verdicts`,
/// `mission_consolidated`, `plans`, `diffs`, `validation_reports`,
/// `repair_runs`, `mission_checkpoints`) — this summary just points
/// to them so the caller knows where to look next.
#[derive(Clone, Debug)]
pub struct MissionSummary {
    pub mission_id: Uuid,
    pub verdict_id: Uuid,
    pub plan_id: Option<Uuid>,
    pub diff_id: Option<Uuid>,
    pub report_ok: bool,
    pub repair_count: u32,
    pub elapsed_ms: u128,
}

/// Run the full prompt → planning → coding → validation → repair loop
/// for a single raw prompt and return a `MissionSummary`. All
/// artefacts are persisted to the Journal so the HUD (RFC 24), CLI
/// `journal`, and `audit` commands can replay them. Idempotent —
/// re-publishing the same prompt produces a fresh mission each call.
///
/// This is the single contract the IPC and CLI surfaces share. The
/// IPC returns `summary.mission_id`; the CLI prints it via
/// `print_summary`.
pub fn run_mission(
    journal: &crate::journal::Journal,
    prompt: &str,
    force: bool,
) -> Result<MissionSummary> {
    let started = Instant::now();
    let _session_id = Uuid::new_v4();

    // 1) prompt → verdict + (optional) consolidated mission
    let (verdict, consolidated_opt) = run_prompt(
        prompt,
        &PipelineOptions {
            force,
            ..PipelineOptions::default()
        },
    )?;

    let (mission_id, consolidated): (Uuid, Option<_>) = match consolidated_opt {
        Some(c) => (c.mission_id, Some(c)),
        None => (Uuid::new_v4(), None),
    };

    journal.save_verdict(&verdict, Some(mission_id))?;
    journal.create_mission(mission_id, prompt)?;

    let Some(consolidated) = consolidated else {
        return Ok(MissionSummary {
            mission_id,
            verdict_id: verdict.verdict_id,
            plan_id: None,
            diff_id: None,
            report_ok: false,
            repair_count: 0,
            elapsed_ms: started.elapsed().as_millis(),
        });
    };
    journal.save_consolidated(&consolidated)?;
    if !consolidated.locked {
        return Ok(MissionSummary {
            mission_id,
            verdict_id: verdict.verdict_id,
            plan_id: None,
            diff_id: None,
            report_ok: false,
            repair_count: 0,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    // 2) planning → plan
    let plan = crate::planning::runner::run(&PlanningInput {
        consolidated: &consolidated,
        verdict: &verdict,
        clarification_answers: Vec::<ClarificationAnswer>::new(),
        research_runs: Vec::new(),
    })?;
    journal.save_plan(&plan)?;
    if !plan.blocked.is_empty() {
        return Ok(MissionSummary {
            mission_id,
            verdict_id: verdict.verdict_id,
            plan_id: Some(plan.plan_id),
            diff_id: None,
            report_ok: false,
            repair_count: 0,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }
    let agent_id = Uuid::new_v4();

    // 3..5) coding → diff → validation → repair (per step). The
    // shared loop lives in `crate::cli::commands::mission::run_steps_loop`
    // for now; Phase 2 will generalize it through an `Engine` trait
    // so this module stops depending on the CLI crate (currently
    // re-exported via crate root).
    let steps = crate::cli::commands::mission::run_steps_loop(journal, &plan, agent_id, &started)
        .context("steps loop")?;

    Ok(MissionSummary {
        mission_id,
        verdict_id: verdict.verdict_id,
        plan_id: Some(plan.plan_id),
        diff_id: steps.last_diff_id,
        report_ok: steps.report_ok,
        repair_count: steps.repair_count,
        elapsed_ms: started.elapsed().as_millis(),
    })
}
