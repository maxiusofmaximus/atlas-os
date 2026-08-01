// OpenCode OS — CLI top-level arguments (RFC 08).
// Declared in the library so both the `opencode` headless binary and any
// future embedder reuse the same `clap` schema.

use clap::{Parser, Subcommand};

#[cfg(feature = "firecrawl")]
use super::commands::ResearchCmd;
use super::commands::{
    AuditCmd, ExecCmd, ForkCmd, HudCmd, JournalCmd, McpCmd, MissionCmd, PlanCmd, ProfileCmd,
    ResumeCmd, RunCmd, SkillCmd, SteerCmd, SwapModelCmd,
};

#[derive(Parser, Debug)]
#[command(name = "opencode", version, propagate_version = true)]
#[command(about = "OpenCode OS — Agent Engineering Operating System", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Profile to use. Defaults to the value persisted at
    /// `~/.opencode/current`, or `default` on a fresh install. Use
    /// `opencode profile switch <name>` to change it.
    #[arg(long, global = true, env = "OC_PROFILE")]
    pub profile: Option<String>,

    /// Increase verbosity (-v info, -vv debug, -vvv trace).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,
}

impl Cli {
    /// Resolve the active profile id for this invocation: explicit `--profile`
    /// wins, then the persisted pointer at `~/.opencode/current`, then
    /// `default` on a fresh install. Centralising it here means every
    /// subcommand sees the same answer.
    pub fn active_profile(&self) -> String {
        self.profile
            .clone()
            .unwrap_or_else(|| crate::profiles::current().0)
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new mission from a raw prompt.
    Mission(MissionCmd),
    /// Generate Plan from a locked Mission (RFC 25 §3.9).
    Plan(PlanCmd),
    /// Execute the latest Plan for a mission (Coding → Validation → Repair).
    Run(RunCmd),
    /// Resume a mission from its latest checkpoint (RFC 19).
    Resume(ResumeCmd),
    /// Fork an existing session under a new mission id (Cursor pattern).
    Fork(ForkCmd),
    /// Inject a steer message mid-run.
    Steer(SteerCmd),
    /// Hot-swap the model driving a mission (RFC 27 §B).
    SwapModel(SwapModelCmd),
    /// LLM-driver Phase 2 entrypoints (RFC 27 §F).
    Exec(ExecCmd),
    /// HUD server control.
    Hud(HudCmd),
    /// List/refresh MCP servers (RFC 07).
    Mcp(McpCmd),
    /// Skill management (RFC 06).
    Skill(SkillCmd),
    /// Profile switcher (RFC 25 §4).
    Profile(ProfileCmd),
    /// Audit log (RFC 24 §10).
    Audit(AuditCmd),
    /// Journal tail (RFC 19).
    Journal(JournalCmd),
    /// Web ingestion via Firecrawl — scrape, search, crawl, extract
    /// (RFC 28 §E).
    #[cfg(feature = "firecrawl")]
    Research(ResearchCmd),
}
