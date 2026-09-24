// Atlas OS — CLI dispatch for subcommands.
// Path: `src-tauri/src/cli/bin/opencode.rs` uses this `commands/` dir.

pub mod audit;
pub mod exec;
pub mod fork;
pub mod hud;
pub mod journal;
pub mod mcp;
pub mod mission;
pub mod models;
pub mod plan;
pub mod profile;
#[cfg(feature = "firecrawl")]
pub mod research;
pub mod resume;
pub mod run;
pub mod skill;
pub mod steer;
pub mod swap_model;
#[cfg(feature = "toast")]
pub mod toast;

pub use audit::AuditCmd;
pub use exec::ExecCmd;
pub use fork::ForkCmd;
pub use hud::HudCmd;
pub use journal::JournalCmd;
pub use mcp::McpCmd;
pub use mission::MissionCmd;
pub use models::ModelsCmd;
pub use plan::PlanCmd;
pub use profile::ProfileCmd;
#[cfg(feature = "firecrawl")]
pub use research::ResearchCmd;
pub use resume::ResumeCmd;
pub use run::RunCmd;
pub use skill::SkillCmd;
pub use steer::SteerCmd;
pub use swap_model::SwapModelCmd;
#[cfg(feature = "toast")]
pub use toast::ToastCmd;

use anyhow::Result;

use super::proto::Commands;

pub async fn dispatch(cmd: Commands, profile: &str) -> Result<()> {
    match cmd {
        Commands::Mission(c) => mission::run(c, profile).await,
        Commands::Plan(c) => plan::run(c, profile).await,
        Commands::Run(c) => run::run(c, profile).await,
        Commands::Resume(c) => resume::run(c, profile).await,
        Commands::Fork(c) => fork::run(c, profile).await,
        Commands::Steer(c) => steer::run(c, profile).await,
        Commands::SwapModel(c) => swap_model::run(c, profile).await,
        Commands::Models(c) => models::run(c, profile).await,
        Commands::Exec(c) => exec::run(c, profile).await,
        Commands::Hud(c) => hud::run(c, profile).await,
        Commands::Mcp(c) => mcp::run(c, profile).await,
        Commands::Skill(c) => skill::run(c, profile).await,
        Commands::Profile(c) => profile::run(c, profile).await,
        Commands::Audit(c) => audit::run(c, profile).await,
        Commands::Journal(c) => journal::run(c, profile).await,
        #[cfg(feature = "firecrawl")]
        Commands::Research(c) => research::run(c, profile).await,
        #[cfg(feature = "toast")]
        Commands::Toast(c) => toast::run(c, profile).await,
    }
}
