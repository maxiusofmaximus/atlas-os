// Atlas OS — `atlas plan` subcommand (RFC 25 §3.9).
// Generates (or regenerates) a Plan from an existing locked mission. The
// upstream `PublicUnderstandingVerdict` and `MissionConsolidated` are
// fetched from the Journal (the CLI never holds engine artefacts in
// memory between invocations — each subcommand is stateless).
use anyhow::{Context, Result};
use clap::Args;

use crate::planning::runner::{ClarificationAnswer, PlanningInput};

#[derive(Args, Debug)]
pub struct PlanCmd {
    /// Mission UUID whose consolidated mission is locked.
    pub mission_id: String,
}

pub async fn run(cmd: PlanCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;

    let mid = uuid::Uuid::parse_str(&cmd.mission_id).context("invalid mission_id")?;

    let consolidated = journal
        .latest_consolidated_for_mission(mid)?
        .with_context(|| format!("no consolidated mission found for {mid}"))?;
    if !consolidated.locked {
        anyhow::bail!("mission {mid} is not locked — the Planning Engine cannot run (RFC 12 §2)");
    }

    let verdict = journal
        .latest_verdict_for_mission(mid)?
        .with_context(|| format!("no verdict linked to mission {mid}"))?;

    let plan = crate::planning::runner::run(&PlanningInput {
        consolidated: &consolidated,
        verdict: &verdict,
        clarification_answers: Vec::<ClarificationAnswer>::new(),
        research_runs: Vec::new(),
    })?;
    journal.save_plan(&plan)?;

    println!(
        "plan generated: {plan_id} (mission {mid}, confidence={conf:.2}, steps={n})",
        plan_id = plan.plan_id,
        conf = plan.confidence,
        n = plan.steps.len()
    );
    if !plan.blocked.is_empty() {
        println!("  blocked ({} blocker(s)):", plan.blocked.len());
        for b in &plan.blocked {
            println!("    - {:?}: {}", b.kind, b.message);
        }
    } else {
        println!("  next: `opencode run {mid}` once the Coding Engine is wired (Phase 1.5).");
    }
    let grill = crate::planning::grill_plan(&plan);
    if grill.question_count == 0 {
        println!("  grill: pass, no open questions (skill grill-me).");
    } else {
        println!(
            "  grill: {} question(s), {} blocking (skill grill-me, can_lock={}):",
            grill.question_count, grill.blocking_count, grill.can_lock
        );
        for q in &grill.questions {
            let gate = if q.blocks_lock {
                "blocks-lock"
            } else {
                "advisory"
            };
            println!("    - {} [{}:{}] {}", q.id, q.focus, gate, q.question);
        }
    }
    Ok(())
}
