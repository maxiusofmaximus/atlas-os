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
    /// Enable the network tools (`web.fetch`/`web.search`, RFC 63 §4). Off by
    /// default: they are `NetworkEgress` (RFC 18) and need a backend.
    #[arg(long, default_value_t = false)]
    pub web: bool,
    /// Sandbox runtime the commands run in: `local` (default) or `wsl2`
    /// (RFC 63 §4). `wsl2` falls back to `local` when WSL is unavailable.
    #[arg(long, default_value = "local")]
    pub sandbox: String,
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
            "no deployments: set ATLAS_LLM_BASE_URL and ATLAS_LLM_MODEL, then store the API key \
             (Settings view or `atlas secrets set <slot>`)"
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

    // RFC 63 §5: ground execution and verification on the SAME absolute root.
    // A relative `--root` (e.g. ".") breaks `verify_artifacts`' `starts_with`
    // check and lets the model's cwd drift from the verifier's — canonicalize.
    let work =
        std::fs::canonicalize(&cmd.root).with_context(|| format!("resolve --root {}", cmd.root))?;
    let success_predicate = match &cmd.verify {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("read verify file {path}"))?;
            serde_json::from_str::<Vec<crate::orchestrator::artifacts::ArtifactCheck>>(&text)
                .with_context(|| format!("parse verify file {path}"))?
        }
        None => Vec::new(),
    };
    // RFC 63 §4: resolve the sandbox runtime from `--sandbox` (honoured by
    // `resolve_sandbox` when it builds the tool context). Validated so a typo
    // fails loudly instead of silently running on the host.
    match cmd.sandbox.as_str() {
        "local" | "wsl2" | "daytona" | "e2b" => {
            std::env::set_var("ATLAS_SANDBOX", &cmd.sandbox);
        }
        other => anyhow::bail!("unknown --sandbox `{other}` (local|wsl2|daytona|e2b)"),
    }

    // RFC 63 §4: build the live tool registry (web tools opt-in via --web).
    let registry = if cmd.web {
        crate::orchestrator::ToolRegistry::with_web_tools()
    } else {
        crate::orchestrator::ToolRegistry::with_core_tools()
    };
    let tools: Vec<(String, String)> = registry
        .catalog()
        .into_iter()
        .map(|(n, d)| (n.to_string(), d.to_string()))
        .collect();

    let cfg = AgentConfig {
        max_steps: cmd.max_steps,
        timeout: std::time::Duration::from_secs(cmd.command_timeout),
        max_attempts: cmd.max_attempts,
        success_predicate,
        tools,
    };

    println!(
        "agent: model={} root={} max_steps={} tools={} sandbox={}",
        model,
        work.display(),
        cfg.max_steps,
        registry.catalog().len(),
        cmd.sandbox
    );

    let client = HttpProviderClient::new();
    let run_id = format!("ar_{}", uuid::Uuid::new_v4());
    let ts_started = chrono::Utc::now().timestamp_millis();

    // RFC 63 §6 (M49): open the run row before the loop so a crash still leaves
    // a `running` record, then finalize it below.
    let _ = journal.create_agent_run(&crate::journal::AgentRunRow {
        id: run_id.clone(),
        mission_id: None,
        goal: task.clone(),
        success_predicate: serde_json::to_string(&cfg.success_predicate)
            .unwrap_or_else(|_| "[]".into()),
        sandbox: std::env::var("ATLAS_SANDBOX").unwrap_or_else(|_| "local".into()),
        status: "running".into(),
        steps: 0,
        tokens_in: 0,
        tokens_out: 0,
        cost_usd: 0.0,
        ts_started,
        ts_ended: None,
    });

    let outcome = run_agent_real(
        &client,
        RoutingConfig::default(),
        &model,
        &deployments,
        &task,
        &work,
        &cfg,
        &denied,
        Some(registry),
    )
    .await
    .context("agent loop failed")?;

    // RFC 63 §6: persist every turn + tool call. Tokens are summed for the run;
    // artifacts are recorded from the success predicate that was verified.
    let (mut tokens_in, mut tokens_out) = (0i64, 0i64);
    for rec in &outcome.turn_records {
        tokens_in += rec.tokens_in;
        tokens_out += rec.tokens_out;
        let step_id = format!("{run_id}_s{}", rec.turn);
        let _ = journal.record_agent_step(&crate::journal::AgentStepRow {
            id: step_id.clone(),
            run_id: run_id.clone(),
            step: rec.turn as i64,
            thought: Some(rec.thought.clone()),
            action: Some(rec.action.clone()),
            observation: rec.observation.clone(),
            evidence_json: None,
            verdict: rec.verdict.clone(),
            tokens_in: rec.tokens_in,
            tokens_out: rec.tokens_out,
            cost_usd: 0.0,
            ts: chrono::Utc::now().timestamp_millis(),
        });
        // RFC 63 §7/§9: stream each turn on the Kernel Bus for the HUD AgentCard.
        let event = crate::core::bus::BusEvent::new(crate::core::bus::BusEventKind::AgentStep {
            run_id: run_id.clone(),
            step: rec.turn,
            action: rec.action.clone(),
            observation: rec.observation.clone(),
            verdict: rec.verdict.clone(),
            tokens_in: rec.tokens_in.max(0) as u64,
            tokens_out: rec.tokens_out.max(0) as u64,
            cost_usd: 0.0,
        });
        let _ = journal.publish(&event);
    }
    // RFC 63 §6 (M50): one tool_invocation row per tool call, with its real args.
    for (i, tr) in outcome.tool_results.iter().enumerate() {
        let _ = journal.record_tool_invocation(&crate::journal::ToolInvocationRow {
            id: format!("{run_id}_t{i}"),
            run_id: run_id.clone(),
            step: i as i64,
            tool: tr.tool.clone(),
            args_json: tr.args_json.clone().unwrap_or_else(|| "{}".into()),
            result_json: Some(
                serde_json::to_string(&crate::orchestrator::tools::ToolResult {
                    args_json: None,
                    ..tr.clone()
                })
                .unwrap_or_default(),
            ),
            exit_code: tr.exit_code.map(|c| c as i64),
            duration_ms: None,
            tokens: None,
            cost_usd: None,
            ts: chrono::Utc::now().timestamp_millis(),
        });
    }
    let ts_ended = chrono::Utc::now().timestamp_millis();
    let status = if outcome.done { "done" } else { "failed" };
    let _ = journal.finish_agent_run(
        &run_id,
        status,
        outcome.turns as i64,
        tokens_in,
        tokens_out,
        0.0,
        ts_ended,
    );

    for step in &outcome.steps() {
        println!(
            "  $ {}  → exit {}",
            step.command.lines().next().unwrap_or(""),
            step.exit_code
        );
    }
    println!(
        "agent done={} turns={} tools={} tokens_in={} tokens_out={} run_id={} summary={}",
        outcome.done,
        outcome.turns,
        outcome.tool_results.len(),
        tokens_in,
        tokens_out,
        run_id,
        outcome.summary
    );
    if !outcome.done {
        anyhow::bail!("agent did not complete the task");
    }
    Ok(())
}
