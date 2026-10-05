// Atlas OS — CLI dispatch for subcommands.
// Path: `src-tauri/src/cli/bin/opencode.rs` uses this `commands/` dir.

pub mod agent;
pub mod audit;
pub mod browser;
pub mod calendar;
pub mod domain;
pub mod eval;
pub mod exec;
pub mod execute;
pub mod fork;
pub mod hud;
pub mod journal;
pub mod learn;
pub mod mcp;
pub mod mission;
pub mod mobile;
pub mod models;
pub mod monitor;
pub mod plan;
pub mod profile;
pub mod remote;
pub mod research;
pub mod resume;
pub mod run;
pub mod security;
#[cfg(feature = "hud")]
pub mod serve;
pub mod sister;
pub mod skill;
pub mod steer;
pub mod swap_model;
pub mod swarm;
#[cfg(feature = "toast")]
pub mod toast;
pub mod validate;

pub use agent::AgentCmd;
pub use audit::AuditCmd;
pub use browser::BrowserCmd;
pub use calendar::CalendarCmd;
pub use domain::DomainCmd;
pub use eval::EvalCmd;
pub use exec::ExecCmd;
pub use execute::ExecuteCmd;
pub use fork::ForkCmd;
pub use hud::HudCmd;
pub use journal::JournalCmd;
pub use learn::LearnCmd;
pub use mcp::McpCmd;
pub use mission::MissionCmd;
pub use mobile::MobileCmd;
pub use models::ModelsCmd;
pub use monitor::MonitorCmd;
pub use plan::PlanCmd;
pub use profile::ProfileCmd;
pub use remote::RemoteCmd;
pub use research::ResearchCmd;
pub use resume::ResumeCmd;
pub use run::RunCmd;
pub use security::SecurityCmd;
#[cfg(feature = "hud")]
pub use serve::ServeCmd;
pub use sister::SisterCmd;
pub use skill::SkillCmd;
pub use steer::SteerCmd;
pub use swap_model::SwapModelCmd;
pub use swarm::SwarmCmd;
#[cfg(feature = "toast")]
pub use toast::ToastCmd;
pub use validate::ValidateCmd;

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
        Commands::Monitor(c) => monitor::run(c, profile).await,
        Commands::Mobile(c) => mobile::run(c, profile).await,
        Commands::Browser(c) => browser::run(c, profile).await,
        Commands::Exec(c) => exec::run(c, profile).await,
        Commands::Execute(c) => execute::run(c, profile).await,
        Commands::Agent(c) => agent::run(c, profile).await,
        Commands::Hud(c) => hud::run(c, profile).await,
        #[cfg(feature = "hud")]
        Commands::Serve(c) => serve::run(c, profile).await,
        Commands::Mcp(c) => mcp::run(c, profile).await,
        Commands::Skill(c) => skill::run(c, profile).await,
        Commands::Domain(c) => domain::run(c, profile).await,
        Commands::Security(c) => security::run(c, profile).await,
        Commands::Sister(c) => sister::run(c, profile).await,
        Commands::Swarm(c) => swarm::run(c, profile).await,
        Commands::Profile(c) => profile::run(c, profile).await,
        Commands::Remote(c) => remote::run(c, profile).await,
        Commands::Audit(c) => audit::run(c, profile).await,
        Commands::Calendar(c) => calendar::run(c, profile).await,
        Commands::Eval(c) => eval::run(c, profile).await,
        Commands::Validate(c) => validate::run(c, profile).await,
        Commands::Journal(c) => journal::run(c, profile).await,
        Commands::Learn(c) => learn::run(c, profile).await,
        Commands::Research(c) => research::run(c, profile).await,
        #[cfg(feature = "toast")]
        Commands::Toast(c) => toast::run(c, profile).await,
    }
}
