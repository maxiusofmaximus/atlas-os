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
}

pub async fn run(cmd: ModelsCmd, profile: &str) -> Result<()> {
    match cmd.sub {
        ModelsSub::Refresh { window, list } => refresh(window, list, profile).await,
    }
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
