// OpenCode OS — `opencode fork <mission_id>` subcommand (RFC 25 §3.9).
// Cursor's fork pattern: clone an existing mission under a new UUID,
// copy its verdict + consolidated rows, and return immediately so the
// operator can `opencode plan <new_id>` / `opencode run <new_id>` with a
// clean timeline. The original mission is untouched.

use anyhow::{Context, Result};
use clap::Args;
use uuid::Uuid;

#[derive(Args, Debug)]
pub struct ForkCmd {
    /// Mission UUID to fork from.
    pub mission_id: String,
}

pub async fn run(cmd: ForkCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    let src = Uuid::parse_str(&cmd.mission_id).context("invalid mission_id")?;

    let mut consolidated = journal
        .latest_consolidated_for_mission(src)?
        .with_context(|| format!("no consolidated mission found for {src}"))?;
    let mut verdict = journal
        .latest_verdict_for_mission(src)?
        .with_context(|| format!("no verdict linked to mission {src}"))?;

    let new_mission = Uuid::new_v4();
    let new_verdict = Uuid::new_v4();

    // Re-key the artefacts under the new mission/verdict ids and reset
    // the timestamp so the cloned rows land below the originals in the
    // tail (newer). Phase 1 does not persist a `lock` row, so there is
    // no `locked_by` field to reset here — a forked plan is therefore
    // pick-up-able by `opencode plan <new>` as-is.
    consolidated.mission_id = new_mission;
    consolidated.verdict_id = new_verdict;
    consolidated.generated_at = chrono::Utc::now().to_rfc3339();
    verdict.verdict_id = new_verdict;
    verdict.timestamp = consolidated.generated_at.clone();

    journal.create_mission(new_mission, &consolidated.mission_statement)?;
    journal.save_verdict(&verdict, Some(new_mission))?;
    journal.save_consolidated(&consolidated)?;

    println!(
        "fork mission: {src} → {new}  (verdict {vnew})",
        new = new_mission,
        vnew = new_verdict
    );
    println!("next: `opencode plan {new_mission}` to regenerate the plan with the forked context.");
    Ok(())
}
