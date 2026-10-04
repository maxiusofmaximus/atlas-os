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
use crate::orchestrator::execute::{
    execute_coding_step_denied, execute_steps, ExecuteConfig, ExecuteStep, ModelPrice,
};
use crate::orchestrator::provider::Deployment;
use crate::orchestrator::reliability_gate::{filter_deployments, ReliabilityGate};
use crate::orchestrator::routing::RoutingConfig;
use crate::planning::types::Plan;

const SYSTEM_PROMPT: &str =
    "You are Atlas OS executing one plan step. Reply with the concrete change or action.";

/// Load `.env` from the current dir into the process env once (no dependency:
/// simple `KEY=VALUE` parser, `#` comments, optional quotes). Existing env vars
/// win so a shell override is never clobbered.
fn load_dotenv_once() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let Ok(text) = std::fs::read_to_string(".env") else {
            return;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let k = k.trim();
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if !k.is_empty() && std::env::var_os(k).is_none() {
                    std::env::set_var(k, v);
                }
            }
        }
    });
}

/// Build a `ResearchContext` from explicit `--research-ref` flags (Fase 36). The
/// operator passes refs (e.g. a past `atlas research query` result) so a coding
/// Diff carries evidence the EvidenceGate accepts. Empty → behaves as before.
fn research_context_from(cmd: &ExecuteCmd) -> crate::orchestrator::code::ResearchContext {
    use crate::orchestrator::code::ResearchContext;
    let mut refs = Vec::new();
    let mut notes = Vec::new();
    for raw in &cmd.research_refs {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        match Uuid::parse_str(raw) {
            Ok(u) => {
                refs.push(u);
                notes.push(raw.to_string());
            }
            Err(_) => {
                refs.push(uuid_from_text(raw));
                notes.push(raw.to_string());
            }
        }
    }
    ResearchContext {
        run_id: cmd.research_run.clone().unwrap_or_else(|| "-".to_string()),
        refs,
        notes,
    }
}

/// Deterministic UUID for a free-text ref (so the same string yields the same
/// `research_refs` entry across runs) without needing the `v5` uuid feature.
fn uuid_from_text(s: &str) -> Uuid {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    let a = h.finish();
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    (s, a).hash(&mut h2);
    let b = h2.finish();
    Uuid::from_u128((a as u128) << 64 | b as u128)
}

/// Persist a checkpoint, apply `diff` to `root` (filesystem), and return the
/// number of files written. Pure application logic lives in `coding::apply`; this
/// only does the I/O + pre-write audit (Fase 35).
fn apply_coding_diff(
    journal: &Journal,
    diff: &crate::coding::types::Diff,
    mission_id: Uuid,
    plan_id: Uuid,
    root: &std::path::Path,
) -> Result<usize> {
    use std::collections::BTreeMap;

    // Pre-write checkpoint so the state before the edit is auditable.
    let checkpoint = crate::supervisor::types::MissionCheckpoint {
        checkpoint_id: Uuid::new_v4(),
        mission_id,
        phase: crate::supervisor::types::MissionPhase::Executing,
        current_plan_id: Some(plan_id),
        last_validation_report_id: None,
        last_repair_id: None,
        budget_tally: crate::supervisor::types::BudgetTally::default(),
        caps: None,
        mode: None,
        generated_at: chrono::Utc::now().to_rfc3339(),
    };
    journal.save_checkpoint(&checkpoint)?;

    // Read the current contents of the files the diff touches.
    let mut workspace: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for edit in &diff.files {
        let abs = root.join(&edit.path);
        if let Ok(text) = std::fs::read_to_string(&abs) {
            let lines = text.split('\n').map(|l| l.to_string()).collect();
            workspace.insert(edit.path.clone(), lines);
        }
    }

    let applied = crate::coding::apply::apply_diff(diff, &workspace)
        .map_err(|e| anyhow::anyhow!("apply diff: {e}"))?;

    let mut written = 0usize;
    for file in &applied {
        let abs = root.join(&file.path);
        if file.deleted {
            if abs.exists() {
                std::fs::remove_file(&abs).with_context(|| format!("delete {}", abs.display()))?;
            }
            written += 1;
            continue;
        }
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("mkdir {}", parent.display()))?;
        }
        let body = file.lines.join("\n");
        std::fs::write(&abs, body).with_context(|| format!("write {}", abs.display()))?;
        written += 1;
    }
    Ok(written)
}

#[derive(Args, Debug)]
pub struct ExecuteCmd {
    /// Mission UUID whose latest Plan should be executed through the orchestrator.
    pub mission_id: String,
    /// Max cascade attempts per step.
    #[arg(long, default_value_t = 5)]
    pub max_attempts: u8,
    /// Drive each step through the LLM-coding bridge (structured Diff →
    /// Validation → Repair) instead of the plain chat loop. Persists the diff,
    /// the validation report and any repair (Fase 26).
    #[arg(long, default_value_t = false)]
    pub coding: bool,
    /// With `--coding`, write each validated Diff to the workspace under `--root`
    /// (default: the current directory) after a checkpoint. Fase 35.
    #[arg(long, default_value_t = false)]
    pub apply: bool,
    /// Workspace root that `--apply` reads/writes. Defaults to the current dir.
    #[arg(long, default_value = ".")]
    pub root: String,
    /// Research refs (repeatable) attached to each coding Diff as evidence
    /// (Fase 36). Accepts a UUID or any free text (hashed deterministically).
    #[arg(long = "research-ref")]
    pub research_refs: Vec<String>,
    /// Optional research run id shown in the report (e.g. `rr-...`).
    #[arg(long)]
    pub research_run: Option<String>,
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
    // Load `.env` (if present) BEFORE resolving the endpoint, so a real run uses
    // the operator's ATLAS_LLM_* vars. Seed deployments are empty; env is the
    // runtime backend for an OpenAI-compatible provider (NIM/Cerebras/Groq/...).
    load_dotenv_once();
    let mut deployments: Vec<Deployment> = registry
        .deployments
        .values()
        .flatten()
        .map(|d| (**d).clone())
        .collect();
    if deployments.is_empty() {
        deployments = crate::orchestrator::Registry::deployments_from_env();
    }
    if deployments.is_empty() {
        anyhow::bail!(
            "no deployments: set ATLAS_LLM_BASE_URL + ATLAS_LLM_MODEL + the provider key in .env"
        );
    }
    println!(
        "execute: deployment={} base={} key_env={}",
        deployments[0].model_id,
        deployments[0].api_base,
        deployments[0].api_key_env.as_deref().unwrap_or("-")
    );
    for (i, d) in deployments.iter().enumerate() {
        eprintln!(
            "  [deployment {i}] id={} model_id={} weight={}",
            d.id, d.model_id, d.weight
        );
    }

    let steps: Vec<ExecuteStep> = plan
        .steps
        .iter()
        .map(|s| {
            let plan_model = s
                .models
                .first()
                .map(|m| m.model_id.clone())
                .unwrap_or_else(|| plan.model_id.clone());
            // If the plan's model has no deployment (e.g. a curated plan naming
            // claude-opus-4 while the runtime endpoint is a single NIM model),
            // route through the deployment we actually have — otherwise the
            // cascade would exhaust with zero attempts.
            let model_id = if deployments.iter().any(|d| d.model_id == plan_model) {
                plan_model
            } else {
                deployments[0].model_id.clone()
            };
            ExecuteStep {
                id: s.id.clone(),
                model_id,
                statement: s.statement.clone(),
            }
        })
        .collect();
    if steps.is_empty() {
        println!("plan {} has no steps", plan.plan_id);
        return Ok(());
    }
    for s in &steps {
        eprintln!("  [step {}] model_id={}", s.id, s.model_id);
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

    if cmd.coding {
        let agent_id = Uuid::new_v4();
        // Fase 24/33: if the reliability gate is enabled, deny models whose
        // historical pass rate is below threshold (or too few samples) so the
        // coding loop never routes a Diff to an unreliable model.
        let policy = journal.load_reliability_gate().ok().flatten();
        let (gate_enabled, denied): (bool, Vec<String>) = match policy {
            Some(p) if p.enabled => {
                let models: Vec<String> = {
                    let mut seen = std::collections::BTreeSet::new();
                    deployments
                        .iter()
                        .filter(|d| seen.insert(d.model_id.clone()))
                        .map(|d| d.model_id.clone())
                        .collect()
                };
                let reliabilities =
                    crate::orchestrator::reliability_gate::reliabilities_from_journal(
                        &journal, &models, 200,
                    );
                let gate = ReliabilityGate {
                    min_samples: p.min_samples,
                    min_pass_rate: p.min_pass_rate,
                    allow_unknown: p.allow_unknown,
                };
                let (_allowed, denied) = filter_deployments(&deployments, &reliabilities, &gate);
                (
                    true,
                    denied
                        .into_iter()
                        .map(|(d, _)| d.model_id.clone())
                        .collect(),
                )
            }
            _ => (false, Vec::new()),
        };
        println!(
            "execute --coding reliability_gate={} denied={denied:?}",
            if gate_enabled { "on" } else { "off" }
        );

        // Fase 36 — attach recent Research Engine refs so a code Diff can carry
        // evidence the EvidenceGate accepts (RFC 30 §2.1), instead of always
        // failing to repair for lack of a research artefact.
        let research = research_context_from(&cmd);
        if !research.is_empty() {
            println!(
                "execute --coding research={} refs={}",
                research.run_id,
                research.refs.len()
            );
        }

        let now = chrono::Utc::now().to_rfc3339();
        let mut ok = 0usize;
        for step in &steps {
            let out = if denied.is_empty() {
                crate::orchestrator::execute::execute_coding_step_denied(
                    &client,
                    RoutingConfig::default(),
                    step,
                    &deployments,
                    mission_id,
                    plan.plan_id,
                    agent_id,
                    cmd.max_attempts,
                    &price_of,
                    &[],
                    &research,
                )
                .await
            } else {
                execute_coding_step_denied(
                    &client,
                    RoutingConfig::default(),
                    step,
                    &deployments,
                    mission_id,
                    plan.plan_id,
                    agent_id,
                    cmd.max_attempts,
                    &price_of,
                    &denied,
                    &research,
                )
                .await
            }
            .with_context(|| format!("coding step `{}` failed", step.id))?;
            journal.save_diff(&out.diff)?;
            journal.save_report(&out.report)?;
            if let Some(repair) = &out.repair {
                journal.save_repair(repair)?;
            }

            // F34 — one auditable model_invocation per Diff, carrying the gate
            // decision (route_taken_json) so a routing choice can be reviewed
            // after the fact (the "measure the decision, not only the outcome"
            // rule from research/56).
            let provider = registry
                .get(&out.model_id)
                .map(|d| format!("{:?}", d.provider))
                .unwrap_or_else(|| "unknown".into());
            let route_taken = serde_json::json!({
                "coding": true,
                "step_id": out.step_id,
                "diff_id": out.diff.diff_id.to_string(),
                "reliability_gate": if gate_enabled { "on" } else { "off" },
                "denied": denied,
                "deployment_id": out.deployment_id,
                "attempts": out.attempts,
            })
            .to_string();
            let row = ModelInvocationRow {
                id: Uuid::new_v4().to_string(),
                mission_id: Some(mission_id.to_string()),
                model_id: out.model_id.clone(),
                deployment_id: out.deployment_id.clone(),
                provider,
                idempotency_key: format!("{mission_id}:{}:diff", out.step_id),
                started_at: now.clone(),
                finished_at: Some(now.clone()),
                latency_ms: Some(out.latency_ms),
                tokens_in: out.usage.map(|u| u.prompt_tokens as i64),
                tokens_out: out.usage.map(|u| u.completion_tokens as i64),
                cache_read_input_tokens: None,
                cost_usd: Some(out.cost_usd),
                seed: None,
                temperature: None,
                sampling_params_json: None,
                route_taken_json: Some(route_taken),
                was_correct: Some(if out.report.is_pass() { 1 } else { 0 }),
                error_kind: None,
                error_message: None,
            };
            journal.record_model_invocation(&row)?;
            let _ = journal.publish(&BusEvent::new(BusEventKind::AgentTokens {
                agent_id: mission_id,
                tokens_in: out.usage.map(|u| u.prompt_tokens).unwrap_or(0),
                tokens_out: out.usage.map(|u| u.completion_tokens).unwrap_or(0),
                cost_usd: out.cost_usd,
            }));

            println!(
                "  step {} -> {} diff={} files={} validation={:?} repair={:?} ${:.6}",
                step.id,
                out.deployment_id,
                out.diff.diff_id,
                out.diff.files.len(),
                out.report.outcome,
                out.repair.as_ref().map(|r| r.outcome),
                out.cost_usd,
            );

            // F35 — apply the validated Diff to the workspace under `--root`,
            // after a checkpoint so the pre-edit state is auditable.
            if cmd.apply {
                let root = std::path::Path::new(&cmd.root);
                let applied =
                    apply_coding_diff(&journal, &out.diff, mission_id, plan.plan_id, root)?;
                println!(
                    "    apply: {} file(s) written under {}",
                    applied,
                    root.display()
                );
            }
            ok += 1;
        }
        println!("execute --coding mission={mission_id} steps_ok={ok}");
        return Ok(());
    }

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
