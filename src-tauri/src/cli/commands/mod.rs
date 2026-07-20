// OpenCode OS — CLI dispatch for subcommands.
// Path: `src-tauri/src/cli/bin/opencode.rs` uses this `commands/` dir.

pub mod audit;
pub mod hud;
pub mod journal;
pub mod mcp;
pub mod mission;
pub mod profile;
pub mod skill;

pub use audit::AuditCmd;
pub use hud::HudCmd;
pub use journal::JournalCmd;
pub use mcp::McpCmd;
pub use mission::MissionCmd;
pub use profile::ProfileCmd;
pub use skill::SkillCmd;

use anyhow::Result;

use super::proto::Commands;

pub async fn dispatch(cmd: Commands, profile: &str) -> Result<()> {
    match cmd {
        Commands::Mission(c) => mission::run(c, profile).await,
        Commands::Hud(c) => hud::run(c, profile).await,
        Commands::Mcp(c) => mcp::run(c, profile).await,
        Commands::Skill(c) => skill::run(c, profile).await,
        Commands::Profile(c) => profile::run(c, profile).await,
        Commands::Audit(c) => audit::run(c, profile).await,
        Commands::Journal(c) => journal::run(c, profile).await,
    }
}
