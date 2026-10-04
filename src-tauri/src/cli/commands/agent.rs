// Atlas OS — `atlas agent <task>` (RFC 20 Fase 39).
//
// The terminal agent loop: the model runs shell commands in `--root` to complete
// a task (the Terminal-Bench paradigm). Reuses the same endpoint resolution,
// reliability gate and `.env` loading as `atlas execute`.

use anyhow::{Context, Result};
use clap::Args;

use crate::orchestrator::agent::{run_agent_real, AgentConfig};
use crate::orchestrator::client::HttpProviderClient;
use crate::orchestrator::provider::Deployment;
use crate::orchestrator::routing::RoutingConfig;

#[derive(Args, Debug)]
pub struct AgentCmd {
    /// The task for the agent to complete by running shell commands.
    /// Omit it (with `--list-tools`) to print the tool catalog instead.
    pub task: Option<String>,
    /// List the ToolRegistry catalog and exit (RFC 63).
    #[arg(long, default_value_t = false)]
    pub list_tools: bool,
    /// Working directory the commands run in.
    #[arg(long, default_value = ".")]
    pub root: String,
    /// Max model↔terminal turns before giving up.
    #[arg(long, default_value_t = 25)]
    pub max_steps: u32,
    /// Per-command timeout in seconds.
    #[arg(long, default_value_t = 120)]
    pub command_timeout: u64,
    /// Max cascade attempts per model call.
    #[arg(long, default_value_t = 5)]
    pub max_attempts: u8,
    /// Path to a JSON file with the success predicate (array of ArtifactCheck).
    /// `done` is only accepted once every check holds (RFC 63).
    #[arg(long)]
    pub verify: Option<String>,
}

pub async fn run(cmd: AgentCmd, profile: &str) -> Result<()> {
    if cmd.list_tools {
        let reg = crate::orchestrator::ToolRegistry::with_core_tools();
        println!("atlas agent tools ({}):", reg.catalog().len());
        for (name, desc) in reg.catalog() {
            println!("  {name:<12} {desc}");
        }
        return Ok(());
    }
    let task = cmd
        .task
        .clone()
        .context("missing <TASK> (or pass --list-tools)")?;

    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;

    let registry = crate::orchestrator::Registry::from_bundled_seed()?;
    crate::cli::commands::execute::load_dotenv_once();
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
    let model = deployments[0].model_id.clone();

    // Reliability gate (same opt-in policy as `execute`).
    let denied: Vec<String> = match journal.load_reliability_gate().ok().flatten() {
        Some(p) if p.enabled => {
            let models: Vec<String> = {
                let mut seen = std::collections::BTreeSet::new();
                deployments
                    .iter()
                    .filter(|d| seen.insert(d.model_id.clone()))
                    .map(|d| d.model_id.clone())
                    .collect()
            };
            let rel = crate::orchestrator::reliability_gate::reliabilities_from_journal(
                &journal, &models, 200,
            );
            let gate = crate::orchestrator::reliability_gate::ReliabilityGate {
                min_samples: p.min_samples,
                min_pass_rate: p.min_pass_rate,
                allow_unknown: p.allow_unknown,
            };
            crate::orchestrator::reliability_gate::filter_deployments(&deployments, &rel, &gate)
                .1
                .into_iter()
                .map(|(d, _)| d.model_id.clone())
                .collect()
        }
        _ => Vec::new(),
    };

    let work = std::path::PathBuf::from(&cmd.root);
    let success_predicate = match &cmd.verify {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("read verify file {path}"))?;
            serde_json::from_str::<Vec<crate::orchestrator::artifacts::ArtifactCheck>>(&text)
                .with_context(|| format!("parse verify file {path}"))?
        }
        None => Vec::new(),
    };
    let cfg = AgentConfig {
        max_steps: cmd.max_steps,
        timeout: std::time::Duration::from_secs(cmd.command_timeout),
        max_attempts: cmd.max_attempts,
        success_predicate,
    };

    println!(
        "agent: model={} root={} max_steps={}",
        model,
        work.display(),
        cfg.max_steps
    );

    let client = HttpProviderClient::new();
    let outcome = run_agent_real(
        &client,
        RoutingConfig::default(),
        &model,
        &deployments,
        &task,
        &work,
        &cfg,
        &denied,
    )
    .await
    .context("agent loop failed")?;

    for step in &outcome.steps {
        println!(
            "  $ {}  → exit {}",
            step.command.lines().next().unwrap_or(""),
            step.exit_code
        );
    }
    println!(
        "agent done={} turns={} commands={} summary={}",
        outcome.done,
        outcome.turns,
        outcome.steps.len(),
        outcome.summary
    );
    if !outcome.done {
        anyhow::bail!("agent did not complete the task");
    }
    Ok(())
}
