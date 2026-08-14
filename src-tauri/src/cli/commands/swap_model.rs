// Atlas OS — `opencode swap-model <mission_id> <model_id>` subcommand
// (RFC 25 §3.9 + RFC 27 §B). Hot-swaps the model driving a mission
// mid-flight: rewrites the latest verdict/plan via the Orchestrator and
// records a `model_swaps` row + `BusEventKind::ModelSwapped` so the HUD
// and the next supervisor run observe the new model.

use anyhow::{Context, Result};
use clap::Args;
use uuid::Uuid;

#[derive(Args, Debug)]
pub struct SwapModelCmd {
    /// Mission UUID whose model should be swapped.
    pub mission_id: String,
    /// New model id (e.g. `claude-sonnet-4`, `llama-3.1-70b@llama-server`).
    pub model_id: String,
}

pub async fn run(cmd: SwapModelCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    let mid = Uuid::parse_str(&cmd.mission_id).context("invalid mission_id")?;

    let outcome = crate::orchestrator::swap_model(
        &journal,
        mid,
        &cmd.model_id,
        crate::core::bus::SwapInitiator::User,
    )
    .context("model swap failed")?;

    println!(
        "swap mission={mid}  {prev} -> {new}  plan_touched={plan_touched}  swap_id={swap_id}",
        prev = outcome.prev_model_id,
        new = outcome.new_model_id,
        plan_touched = outcome.plan_touched,
        swap_id = outcome.swap_id,
    );
    if outcome.plan_touched {
        println!("  latest Plan re-emitted with the new model_id");
    } else {
        println!("  no Plan exists yet — next run will pick the new model");
    }
    println!(
        "  persisted to model_swaps + journal_events — the next `opencode resume {mid}` or live HUD will surface it."
    );
    Ok(())
}
