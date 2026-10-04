// Atlas OS — `atlas execute <mission_id>` (RFC 20 Fase 25 v25.3; research/54).
//
// Drives the mission's latest Plan through the orchestrator loop: routes each
// step to a model via `call_with_cascade`, feeds the Execution Supervisor so
// budgets apply, persists a `model_invocations` row + publishes `AgentTokens`
// per step, and supports Ctrl-C cancellation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Args;
use uuid::Uuid;

use crate::core::bus::{BusEvent, BusEventKind};
use crate::journal::{Journal, ModelInvocationRow};
use crate::orchestrator::client::HttpProviderClient;
use crate::orchestrator::execute::{execute_steps, ExecuteConfig, ExecuteStep, ModelPrice};
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

    // Ctrl-C sets the cancellation flag.
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let cancel = Arc::clone(&cancel);
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                cancel.store(true, Ordering::Relaxed);
            }
        });
    }

    let cfg = ExecuteConfig {
        max_attempts: cmd.max_attempts,
        ..ExecuteConfig::default()
    };
    let client = HttpProviderClient::new();
    let price_of = |model_id: &str| {
        registry.get(model_id).map(|d| ModelPrice {
            input_per_1m: d.input_cost_per_1m_tokens,
            output_per_1m: d.output_cost_per_1m_tokens,
        })
    };

    let report = execute_steps(
        &client,
        &deployments,
        RoutingConfig::default(),
        mission_id,
        plan.plan_id,
        &steps,
        SYSTEM_PROMPT,
        &cfg,
        cancel.as_ref(),
        price_of,
    )
    .await;

    // Persist one `model_invocations` row + publish `AgentTokens` per step.
    let now = chrono::Utc::now().to_rfc3339();
    for outcome in &report.outcomes {
        let provider = registry
            .get(&outcome.model_id)
            .map(|d| format!("{:?}", d.provider))
            .unwrap_or_else(|| "unknown".into());
        let row = ModelInvocationRow {
            id: Uuid::new_v4().to_string(),
            mission_id: Some(mission_id.to_string()),
            model_id: outcome.model_id.clone(),
            deployment_id: outcome.deployment_id.clone(),
            provider,
            idempotency_key: format!("{mission_id}:{}:{}", outcome.step_id, outcome.deployment_id),
            started_at: now.clone(),
            finished_at: Some(now.clone()),
            latency_ms: Some(outcome.latency_ms),
            tokens_in: outcome.usage.map(|u| u.prompt_tokens as i64),
            tokens_out: outcome.usage.map(|u| u.completion_tokens as i64),
            cache_read_input_tokens: None,
            cost_usd: Some(outcome.cost_usd),
            seed: None,
            temperature: None,
            sampling_params_json: None,
            route_taken_json: None,
            was_correct: None,
            error_kind: None,
            error_message: None,
        };
        journal.record_model_invocation(&row)?;

        let event = BusEvent::new(BusEventKind::AgentTokens {
            agent_id: mission_id,
            tokens_in: outcome.usage.map(|u| u.prompt_tokens).unwrap_or(0),
            tokens_out: outcome.usage.map(|u| u.completion_tokens).unwrap_or(0),
            cost_usd: outcome.cost_usd,
        });
        let _ = journal.publish(&event);
    }

    println!(
        "execute mission={mission_id} steps_ok={ok} halted={halted:?} iterations={iters} cost_usd={cost:.6} phase={phase:?} elapsed_ms={ms}",
        ok = report.outcomes.len(),
        halted = report.halted,
        iters = report.tally.iterations,
        cost = report.tally.cost_usd,
        phase = report.phase,
        ms = report.elapsed_ms,
    );
    for outcome in &report.outcomes {
        println!(
            "  step {} -> {} (attempts {}, {} ms, ${:.6})",
            outcome.step_id,
            outcome.deployment_id,
            outcome.attempts,
            outcome.latency_ms,
            outcome.cost_usd,
        );
    }
    if let Some(reason) = report.halted {
        anyhow::bail!("execute halted: {reason}");
    }
    Ok(())
}
