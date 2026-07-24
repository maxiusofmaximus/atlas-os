// OpenCode OS — `opencode mission` subcommand (Phase 1).
// Drives a prompt end-to-end through the heuristic pipeline so the CLI
// matches the desktop binary's surface area:
//
//   opencode mission new "prompt"   → Prompt → Planning → Coding →
//                                     Validation → (Repair if needed)
//   opencode mission list           → recent missions
//
// `opencode plan <mission_id>` lives in `plan.rs` (RFC 25 §3.9) and
// reuses the same planning engine; it is wired into the CLI dispatch
// via its own subcommand rather than nesting under `mission`.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use uuid::Uuid;

#[derive(Args, Debug)]
pub struct MissionCmd {
    #[command(subcommand)]
    pub action: MissionAction,
}

#[derive(Subcommand, Debug)]
pub enum MissionAction {
    /// Create a mission from a raw prompt and run the heuristic pipeline.
    New {
        prompt: String,
        /// Force-lock the mission even when the heuristic verdict confidence
        /// is below the auto-lock threshold (RFC 23 §7). Surfaces a human
        /// override that is auditable in the Journal as `locked_by = user`.
        #[arg(long)]
        force: bool,
    },
    /// List recent missions.
    List,
}

/// Light-weight per-run summary surfaced to the CLI operator. The full
/// step-by-step artefacts live in the Journal (`prompt_verdicts`,
/// `mission_consolidated`, `plans`, `diffs`, `validation_reports`,
/// `repair_runs`) — this summary just points to them so the operator
/// knows where to look next.
struct PipeSummary {
    mission_id: Uuid,
    verdict_id: Uuid,
    plan_id: Option<Uuid>,
    diff_id: Option<Uuid>,
    report_ok: bool,
    repair_count: u32,
    elapsed_ms: u128,
}

/// (crate-visible) result of `run_steps_loop` so sibling subcommands
/// (`opencode run`, `opencode resume`) and `core::pipeline` can drive
/// the Coding → Validation → Repair loop against an already-loaded
/// Plan without re-thinking the orchestration.
pub(crate) struct StepsResult {
    pub last_diff_id: Option<Uuid>,
    pub report_ok: bool,
    pub repair_count: u32,
    pub plan_id: Uuid,
    pub mission_id: Uuid,
}

fn build_checkpoint(
    plan: &crate::planning::types::Plan,
    phase: crate::supervisor::types::MissionPhase,
    last_validation_report_id: Option<Uuid>,
    last_repair_id: Option<Uuid>,
) -> crate::supervisor::types::MissionCheckpoint {
    use crate::supervisor::types::{BudgetTally, MissionCheckpoint};
    MissionCheckpoint {
        checkpoint_id: Uuid::new_v4(),
        mission_id: plan.mission_id,
        phase,
        current_plan_id: Some(plan.plan_id),
        last_validation_report_id,
        last_repair_id,
        budget_tally: BudgetTally::default(),
        generated_at: chrono::Utc::now().to_rfc3339(),
    }
}

/// Execute one non-read-only step of `plan`: Coding → Validation →
/// (optional) Repair. Persists the diff, report, repair run, and the
/// step-phase pills (`Executing` → `Verifying` → `Done`/`Blocked`).
///
/// Returns the produced `StepOutcome` so both `run_steps_loop` and the
/// Phase 2 LLM-driver `opencode exec step` sub-command share the same
/// code path (RFC 27 §3.F).
pub(crate) struct StepOutcome {
    pub diff_id: Option<Uuid>,
    pub report_ok: bool,
    pub repair_id: Option<Uuid>,
    /// True when Coding was rejected OR Repair produced no diff. The
    /// full `run_steps_loop` ignores this (it derives the same signal
    /// from `report_ok`) but `opencode exec step` surfaces it to the
    /// LLM driver so Phase 2 can decide whether to branch or retry.
    #[allow(dead_code)]
    pub blocked: bool,
}

pub(crate) fn run_single_step(
    journal: &crate::journal::Journal,
    plan: &crate::planning::types::Plan,
    step: &crate::planning::types::Step,
    agent_id: Uuid,
    mission_failures: u32,
) -> anyhow::Result<StepOutcome> {
    use crate::coding::runner::run as run_coding;
    use crate::coding::runner::{CodingInput, WorkspaceFile};
    use crate::coding::types::CodingOutcome;
    use crate::planning::types::StepPhase;
    use crate::repair::runner::run as run_repair;
    use crate::repair::runner::RepairInput;
    use crate::validation::runner::run as run_validation;
    use crate::validation::runner::ValidationInput;
    use crate::validation::types::ValidationMode;

    publish_step_phase(journal, plan, &step.id, StepPhase::Executing)?;

    let coding_outcome = run_coding(&CodingInput {
        plan,
        step,
        agent_id,
        approved_skills: Vec::new(),
        workspace_files: Vec::<WorkspaceFile>::new(),
        narrative: format!("heuristic step {} ({})", step.id, step.action.tag()),
        research_refs: Vec::new(),
    })
    .context("coding::run failed")?;
    let diff = match coding_outcome {
        CodingOutcome::Emitted { diff } => *diff,
        CodingOutcome::Rejected { reason } => {
            println!("step {} rejected: {reason:?} — skipping", step.id);
            publish_step_phase(journal, plan, &step.id, StepPhase::Blocked)?;
            return Ok(StepOutcome {
                diff_id: None,
                report_ok: false,
                repair_id: None,
                blocked: true,
            });
        }
    };
    journal.save_diff(&diff).context("save_diff")?;

    publish_step_phase(journal, plan, &step.id, StepPhase::Verifying)?;

    let report = run_validation(&ValidationInput::new(&diff).with_mode(ValidationMode::default()));
    journal.save_report(&report).context("save_report")?;
    if report.is_pass() {
        publish_step_phase(journal, plan, &step.id, StepPhase::Done)?;
        return Ok(StepOutcome {
            diff_id: Some(diff.diff_id),
            report_ok: true,
            repair_id: None,
            blocked: false,
        });
    }

    let repair =
        run_repair(RepairInput::new(&diff, &report).with_mission_failures(mission_failures));
    journal.save_repair(&repair).context("save_repair")?;
    let blocked = repair.final_diff.is_none();
    let phase = if blocked {
        StepPhase::Blocked
    } else {
        StepPhase::Done
    };
    publish_step_phase(journal, plan, &step.id, phase)?;
    if blocked {
        println!(
            "step {} repair yielded no diff — review `opencode journal -n 20`",
            step.id
        );
    } else {
        println!(
            "step {} repaired after 1 attempt — Phase 1 heuristic did not re-validate yet (RFC 15 §4)",
            step.id
        );
    }
    Ok(StepOutcome {
        diff_id: Some(diff.diff_id),
        report_ok: false,
        repair_id: Some(repair.repair_id),
        blocked,
    })
}

/// Drive Coding → Validation → Repair over every edit-capable step of
/// `plan`. Read-only steps are skipped (RFC 13 §2). Diffs, validation
/// reports, and repair runs are all persisted to the Journal so the HUD
/// / `opencode journal` can replay them. Two `MissionCheckpoint`s are
/// persisted per call (RFC 19 §5) — one at `Executing` entry and one at
/// `Done`/`Halted` exit — so `opencode resume <mission_id>` finds a row
/// to load.
pub(crate) fn run_steps_loop(
    journal: &crate::journal::Journal,
    plan: &crate::planning::types::Plan,
    agent_id: Uuid,
    _started: &std::time::Instant,
) -> anyhow::Result<StepsResult> {
    use crate::planning::types::StepPhase;
    use crate::supervisor::types::MissionPhase;

    journal.save_checkpoint(&build_checkpoint(plan, MissionPhase::Executing, None, None))?;

    // Mark every non-read-only step as Pending so the HUD has a baseline
    // before the loop flips each one to Executing. Read-only steps are
    // skipped entirely (they produce no diff to validate).
    for step in &plan.steps {
        if step.read_only {
            continue;
        }
        journal.set_step_phase(plan.mission_id, plan.plan_id, &step.id, StepPhase::Pending)?;
    }

    let mut last_diff_id: Option<Uuid> = None;
    let mut last_validation_report_id: Option<Uuid> = None;
    let mut last_repair_id: Option<Uuid> = None;
    let mut report_ok = true;
    let mut repair_count: u32 = 0u32;
    let mut mission_failures: u32 = 0u32;

    for step in &plan.steps {
        if step.read_only {
            continue;
        }

        let outcome = run_single_step(journal, plan, step, agent_id, mission_failures)?;
        if let Some(id) = outcome.diff_id {
            last_diff_id = Some(id);
        }
        if !outcome.report_ok {
            report_ok = false;
            mission_failures = mission_failures.saturating_add(1);
        }
        if let Some(id) = outcome.repair_id {
            last_repair_id = Some(id);
            repair_count = repair_count.saturating_add(1);
        }
        // Single-step diff_id is the implicit report row-key; collect the
        // validation report id via load from journal so the closing
        // checkpoint pins it.
        if let Some(diff_id) = outcome.diff_id {
            if let Some(r) = latest_report_for_diff(journal, diff_id)? {
                last_validation_report_id = Some(r);
            }
        }
    }

    let exit_phase = if report_ok {
        MissionPhase::Done
    } else {
        MissionPhase::Halted
    };
    journal.save_checkpoint(&build_checkpoint(
        plan,
        exit_phase,
        last_validation_report_id,
        last_repair_id,
    ))?;

    Ok(StepsResult {
        last_diff_id,
        report_ok,
        repair_count,
        plan_id: plan.plan_id,
        mission_id: plan.mission_id,
    })
}

/// RFC 27 §G — persist a step phase change to the Journal's live-state
/// table and publish a `StepPhaseChanged` bus event so the HUD renders
/// the colour-coded pill. Best-effort: a publish failure logs but does
/// NOT abort the supervisor loop — losing the live pill is preferable
/// to discarding an in-flight step's diff.
fn publish_step_phase(
    journal: &crate::journal::Journal,
    plan: &crate::planning::types::Plan,
    step_id: &str,
    phase: crate::planning::types::StepPhase,
) -> anyhow::Result<()> {
    journal.set_step_phase(plan.mission_id, plan.plan_id, step_id, phase)?;
    let _ = journal.publish(&crate::core::bus::BusEvent::new(
        crate::core::bus::BusEventKind::StepPhaseChanged {
            mission_id: plan.mission_id,
            plan_id: plan.plan_id,
            step_id: step_id.to_string(),
            phase,
        },
    ));
    Ok(())
}

/// Best-effort: find the most-recent validation report row whose
/// `diff_id` matches `target`. Linear scan of the report tail because
/// the Journal does not expose a `report_for_diff` accessor yet (Phase
/// 2 will add a real index when `exec wait` needs to poll hot rows).
fn latest_report_for_diff(
    journal: &crate::journal::Journal,
    target: Uuid,
) -> anyhow::Result<Option<Uuid>> {
    let tail = journal.report_tail(64)?;
    Ok(tail
        .into_iter()
        .rev()
        .find(|r| r.diff_id == target)
        .map(|r| r.report_id))
}

pub async fn run(cmd: MissionCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    match cmd.action {
        MissionAction::New { prompt, force } => {
            let summary = run_pipeline(&journal, &prompt, force).context("pipeline failed")?;
            print_summary(&summary, &pid, &prompt);
        }
        MissionAction::List => {
            let missions = journal.list_missions()?;
            if missions.is_empty() {
                println!("(no missions yet for profile {pid})");
            } else {
                println!("missions [{pid}] ({}):", missions.len());
                for m in missions {
                    println!(
                        "  {id}  status={status}  {label}",
                        id = m.id,
                        status = m.status,
                        label = m.label
                    );
                }
            }
        }
    }
    Ok(())
}

fn print_summary(s: &PipeSummary, pid: &crate::profiles::ProfileId, prompt: &str) {
    println!("mission {id}", id = s.mission_id);
    println!("  profile        = {pid}");
    println!("  prompt         = {prompt:?}");
    println!("  verdict_id     = {vid}", vid = s.verdict_id);
    if let Some(plan_id) = s.plan_id {
        println!("  plan_id        = {plan_id}");
    }
    if let Some(diff_id) = s.diff_id {
        println!("  diff_id        = {diff_id}");
    }
    println!("  validation_ok  = {ok}", ok = s.report_ok);
    println!("  repair_runs    = {n}", n = s.repair_count);
    println!("  elapsed_ms     = {ms}", ms = s.elapsed_ms);
    println!();
    println!("next: `opencode journal -n 20`   /   HUD: `opencode hud` for live view");
}

/// Drive the prompt → planning → coding → validation → repair loop for
/// a single prompt. All artefacts are persisted to the Journal so the
/// HUD (RFC 24), CLI `journal`, and `audit` commands can replay them.
///
/// Thin shim that forwards to the unified `core::pipeline::run_mission`
/// so the IPC and CLI surfaces share the same code path (RFC 25 §3.1.1
/// SOP, Fix #3 of the thermo-nuclear audit).
fn run_pipeline(
    journal: &crate::journal::Journal,
    prompt: &str,
    force: bool,
) -> Result<PipeSummary> {
    let s =
        crate::core::pipeline::run_mission(journal, prompt, force).context("pipeline failed")?;
    Ok(PipeSummary {
        mission_id: s.mission_id,
        verdict_id: s.verdict_id,
        plan_id: s.plan_id,
        diff_id: s.diff_id,
        report_ok: s.report_ok,
        repair_count: s.repair_count,
        elapsed_ms: s.elapsed_ms,
    })
}
