// Atlas OS — `atlas models` registry management (RFC 04 §8 sub-fase 2.4).
// `refresh` re-crunches the affinity cache from journal telemetry and
// atomically swaps it into the live `Registry::affinity` index — the
// manual arm of the feedback loop (the orchestrator refresh loop is the
// automatic arm).

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::orchestrator::affinity::AffinityRow;
use crate::orchestrator::classifier::TaskType;

#[derive(Args, Debug)]
pub struct ModelsCmd {
    #[command(subcommand)]
    pub sub: ModelsSub,
}

#[derive(Subcommand, Debug)]
pub enum ModelsSub {
    /// Re-crunch the affinity cache from `model_invocations` telemetry
    /// (RFC 04 §8 sub-fase 2.4 — feedback loop).
    Refresh {
        /// Number of most-recent `model_invocations` rows to sample.
        #[arg(short = 'n', long, default_value_t = 500)]
        window: u32,
        /// Print the resulting per-(task_type, model) affinity rows.
        #[arg(long)]
        list: bool,
    },
    /// Show or set the EVAL-informed routing reliability gate (RFC 20 Fase 24 v24.2).
    ReliabilityGate {
        #[arg(long)]
        min_samples: Option<i64>,
        #[arg(long)]
        min_pass_rate: Option<f64>,
        /// Allow models with no eval history (default).
        #[arg(long)]
        allow_unknown: bool,
        /// Deny models with no eval history (overrides `--allow-unknown`).
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        enable: bool,
        #[arg(long)]
        disable: bool,
    },
}

pub async fn run(cmd: ModelsCmd, profile: &str) -> Result<()> {
    match cmd.sub {
        ModelsSub::Refresh { window, list } => refresh(window, list, profile).await,
        ModelsSub::ReliabilityGate {
            min_samples,
            min_pass_rate,
            allow_unknown,
            strict,
            enable,
            disable,
        } => reliability_gate_cmd(
            profile,
            min_samples,
            min_pass_rate,
            allow_unknown,
            strict,
            enable,
            disable,
        ),
    }
}

fn reliability_gate_cmd(
    profile: &str,
    min_samples: Option<i64>,
    min_pass_rate: Option<f64>,
    allow_unknown: bool,
    strict: bool,
    enable: bool,
    disable: bool,
) -> Result<()> {
    use crate::journal::ReliabilityGateRow;

    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;

    let mut policy = journal
        .load_reliability_gate()?
        .unwrap_or_else(ReliabilityGateRow::defaults);
    let writing = min_samples.is_some()
        || min_pass_rate.is_some()
        || allow_unknown
        || strict
        || enable
        || disable;
    if writing {
        if let Some(v) = min_samples {
            policy.min_samples = v.max(0);
        }
        if let Some(v) = min_pass_rate {
            policy.min_pass_rate = v.clamp(0.0, 1.0);
        }
        if allow_unknown {
            policy.allow_unknown = true;
        }
        if strict {
            policy.allow_unknown = false;
        }
        if enable {
            policy.enabled = true;
        }
        if disable {
            policy.enabled = false;
        }
        journal.save_reliability_gate(&policy)?;
        println!("reliability gate updated");
    }
    println!(
        "reliability gate: enabled={} min_samples={} min_pass_rate={:.2} allow_unknown={}",
        policy.enabled, policy.min_samples, policy.min_pass_rate, policy.allow_unknown
    );
    Ok(())
}

async fn refresh(window: u32, list: bool, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    let registry = crate::orchestrator::Registry::from_bundled_seed()?;

    registry.refresh_affinity_from_journal(&journal, window)?;
    println!("affinity cache refreshed from {window} model_invocations rows");

    if list {
        let snap = registry.affinity.load();
        println!("affinity [{pid}] ({} pairs):", snap.len());
        let mut pairs: Vec<(TaskType, String)> = snap.keys().cloned().collect();
        pairs.sort_by_cached_key(|(task, model)| (format!("{task:?}"), model.clone()));
        for k in pairs {
            let row: &AffinityRow = snap
                .get(&k)
                .expect("key came from the same snapshot just loaded");
            let lat = row
                .p95_latency_ms
                .map_or_else(|| "-".to_string(), |v| format!("{v}ms"));
            let cost = row
                .mean_cost_usd
                .map_or_else(|| "-".to_string(), |v| format!("${v:.4}"));
            println!(
                "  {:?}  {:<44} success={:.2}  p95={:<10} cost={:<10} n={}",
                k.0, k.1, row.success_rate, lat, cost, row.n_samples
            );
        }
    }
    Ok(())
}
