// Atlas OS — `opencode steer <mission_id> "msg"` subcommand
// (RFC 25 §3.9). Injects a steering message mid-run.
//
// Phase 1 has no live supervisor process; we persist the message to the
// Journal as a `SteerAgent` BusEvent so the next `atlas resume` /
// desktop-binary run will see it via `journal.publish`. The subcommand
// returns immediately (no agent is spawned headless — that is Phase 6
// supervisor territory).

use anyhow::{Context, Result};
use clap::Args;
use uuid::Uuid;

#[derive(Args, Debug)]
pub struct SteerCmd {
    /// Mission UUID whose most-recent agent should receive the steer.
    pub mission_id: String,
    /// Free-form steering message.
    pub message: String,
}

pub async fn run(cmd: SteerCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    let mid = Uuid::parse_str(&cmd.mission_id).context("invalid mission_id")?;

    // Phase 1: there is no register of "agent_id → mission_id" yet, so
    // we publish the steer as a `MissionSteered` BusEvent tagged with
    // the mission id only. Phase 6 (live Supervisor) will own the real
    // agent_id ↔ mission_id mapping and subscribe to these events.
    let event = crate::core::bus::BusEvent::new(crate::core::bus::BusEventKind::MissionSteered {
        mission_id: mid,
        message: cmd.message.clone(),
    });
    journal.publish(&event)?;

    println!("steer mission={mid}  message={msg:?}", msg = cmd.message);
    println!(
        "  persisted to journal_events — the next `opencode resume {mid}` or live desktop HUD will surface it."
    );
    Ok(())
}
