// OpenCode OS — `opencode mission` subcommand (Phase 0 stub).
// Full Prompt Understanding Pipeline arrives Phase 1 (Roadmap Fase 1).

use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct MissionCmd {
    #[command(subcommand)]
    pub action: MissionAction,
}

#[derive(Subcommand, Debug)]
pub enum MissionAction {
    /// Create a mission from a raw prompt. Phase 0: persists to journal only.
    New { prompt: String },
    /// List recent missions.
    List,
}

pub async fn run(cmd: MissionCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    match cmd.action {
        MissionAction::New { prompt } => {
            let id = uuid::Uuid::new_v4();
            let session_id = uuid::Uuid::new_v4();
            let now = chrono::Utc::now().to_rfc3339();

            journal.create_mission(id, &prompt)?;
            let event =
                crate::core::bus::BusEvent::new(crate::core::bus::BusEventKind::TaskReceived {
                    raw_prompt: prompt.clone(),
                    session_id,
                });
            // Persist to journal only — the CLI has no broadcast subscribers
            // (the desktop shell and HUD are separate processes when running
            // `opencode mission new` headless).
            journal.publish(&event)?;
            println!(
                "mission created:
  id        = {id}
  profile   = {pid}
  prompt    = {prompt:?}
  created_at= {now}"
            );
        }
        MissionAction::List => {
            let missions = journal.list_missions()?;
            if missions.is_empty() {
                println!("(no missions yet for profile {pid})");
            } else {
                println!("missions [{pid}] ({}):", missions.len());
                for m in missions {
                    println!(
                        "  {id}  status={status}  {label}",
                        id = m.id,
                        status = m.status,
                        label = m.label
                    );
                }
            }
        }
    }
    Ok(())
}
