// Atlas OS — `atlas security` supply-chain gate (RFC 18 §4, Phase 7 sub-fase 7.3).
// Deterministic pre-install check: typosquatting heuristic, install-script
// detection and env-var access detection. Blocks with a non-zero exit on
// `Block`; warns on `Warn`. Socket/Snyk SaaS APIs stay a documented
// follow-up (research/34 SECTOR A.3).

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::security::{evaluate_package, SupplyVerdict};

#[derive(Args, Debug)]
pub struct SecurityCmd {
    #[command(subcommand)]
    pub action: SecurityAction,
}

#[derive(Subcommand, Debug)]
pub enum SecurityAction {
    /// Evaluate a package name (+ optional manifest JSON) through the
    /// deterministic supply-chain gate (RFC 18 §4).
    Gate {
        /// Package name to evaluate (e.g. `lodash`).
        name: String,
        /// Optional package manifest JSON (package.json contents) to scan
        /// for install scripts and env-var access.
        #[arg(long)]
        manifest: Option<String>,
    },
}

pub async fn run(cmd: SecurityCmd, _profile: &str) -> Result<()> {
    match cmd.action {
        SecurityAction::Gate { name, manifest } => gate(&name, manifest.as_deref()),
    }
}

fn gate(name: &str, manifest_json: Option<&str>) -> Result<()> {
    let report = evaluate_package(name, manifest_json);
    println!(
        "supply gate [{pkg}]: {verdict}",
        pkg = report.package,
        verdict = report.verdict.as_str(),
    );
    for reason in &report.reasons {
        println!("  [-] {reason}");
    }
    if report.verdict == SupplyVerdict::Block {
        anyhow::bail!("supply gate Block: refusing install of `{name}` (RFC 18 §4)");
    }
    Ok(())
}
