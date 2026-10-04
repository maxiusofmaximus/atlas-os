// Atlas OS — `atlas execute <mission_id>` (RFC 20 Fase 25 v25.2; research/54).
//
// Drives the mission's latest Plan through the orchestrator loop: routes each
// step to a model via `call_with_cascade`, feeding the Execution Supervisor so
// budgets apply. This is the first command that exercises the routing/cascade/
// reliability path end-to-end (vs the Phase-1 single pass in `run`).

use std::sync::atomic::AtomicBool;

use anyhow::{Context, Result};
use clap::Args;
use uuid::Uuid;

use crate::journal::Journal;
use crate::orchestrator::client::HttpProviderClient;
use crate::orchestrator::execute::{execute_steps, ExecuteConfig, ExecuteStep};
use crate::orchestrator::provider::Deployment;
use crate::orchestrator::routing::RoutingConfig;
use crate::planning::types::Plan;

const SYSTEM_PROMPT: &str =
    "You are Atlas OS executing one plan step. Reply with the concrete change or action.";

#[derive(Args, Debug)]
pub struct ExecuteCmd {
    /// Mission UUID whose latest Plan should be executed through the orchestrator.
    pub mission_id: String,
    /// Max cascade attempts per step.
    #[arg(long, default_value_t = 5)]
    pub max_attempts: u8,
}

pub async fn run(cmd: ExecuteCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = Journal::open(&root)?;
    let mission_id = Uuid::parse_str(&cmd.mission_id).context("invalid mission_id")?;

    let plan_row = journal
        .latest_plan_for_mission(mission_id)?
        .with_context(|| format!("no plan for mission {mission_id}"))?;
    let payload = journal
        .plan_payload(plan_row.plan_id)?
        .with_context(|| format!("plan {} payload missing", plan_row.plan_id))?;
    let plan: Plan = serde_json::from_str(&payload)?;

    let registry = crate::orchestrator::Registry::from_bundled_seed()?;
    let deployments: Vec<Deployment> = registry
        .deployments
        .values()
        .flatten()
        .map(|d| (**d).clone())
        .collect();
    if deployments.is_empty() {
        anyhow::bail!("no deployments in the registry seed");
    }

    let steps: Vec<ExecuteStep> = plan
        .steps
        .iter()
        .map(|s| ExecuteStep {
            id: s.id.clone(),
            model_id: s
                .models
                .first()
                .map(|m| m.model_id.clone())
                .unwrap_or_else(|| plan.model_id.clone()),
            statement: s.statement.clone(),
        })
        .collect();
    if steps.is_empty() {
        println!("plan {} has no steps", plan.plan_id);
        return Ok(());
    }

    let cfg = ExecuteConfig {
        max_attempts: cmd.max_attempts,
        ..ExecuteConfig::default()
    };
    let cancel = AtomicBool::new(false);
    let client = HttpProviderClient::new();

    let report = execute_steps(
        &client,
        &deployments,
        RoutingConfig::default(),
        mission_id,
        plan.plan_id,
        &steps,
        SYSTEM_PROMPT,
        &cfg,
        &cancel,
    )
    .await;

    println!(
        "execute mission={mission_id} steps_ok={ok} halted={halted:?} iterations={iters} phase={phase:?} elapsed_ms={ms}",
        ok = report.outcomes.len(),
        halted = report.halted,
        iters = report.tally.iterations,
        phase = report.phase,
        ms = report.elapsed_ms,
    );
    for outcome in &report.outcomes {
        println!(
            "  step {} -> {} (attempts {})",
            outcome.step_id, outcome.deployment_id, outcome.attempts
        );
    }
    if let Some(reason) = report.halted {
        anyhow::bail!("execute halted: {reason}");
    }
    Ok(())
}
