// Atlas OS — CLI top-level arguments (RFC 08).
// Declared in the library so both the `atlas` headless binary and any
// future embedder reuse the same `clap` schema.

use clap::{Parser, Subcommand};

use super::commands::ResearchCmd;
#[cfg(feature = "toast")]
use super::commands::ToastCmd;
use super::commands::{
    AuditCmd, ExecCmd, ForkCmd, HudCmd, JournalCmd, LearnCmd, McpCmd, MissionCmd, MobileCmd,
    ModelsCmd, MonitorCmd, PlanCmd, ProfileCmd, RemoteCmd, ResumeCmd, RunCmd, SecurityCmd,
    SisterCmd, SkillCmd, SteerCmd, SwapModelCmd, SwarmCmd,
};
#[derive(Parser, Debug)]
#[command(name = "atlas", version, propagate_version = true)]
#[command(about = "Atlas OS — Agent Engineering Operating System", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Profile to use. Defaults to the value persisted at
    /// `~/.opencode/current`, or `default` on a fresh install. Use
    /// `atlas profile switch <name>` to change it.
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
    /// Model registry management — affinity refresh (RFC 04 §8).
    Models(ModelsCmd),
    /// VRAM/RAM/cost snapshot — prints + publishes `hardware_snapshot` (RFC 20 Phase 8.2).
    Monitor(MonitorCmd),
    /// Mobile testing via external artemis (lateral Python/uv, never bundled) — status/guide/run/mcp-template (RFC 20 Fase 11.0/11.1).
    Mobile(MobileCmd),
    /// LLM-driver Phase 2 entrypoints (RFC 27 §F).
    Exec(ExecCmd),
    /// HUD server control.
    Hud(HudCmd),
    /// List/refresh MCP servers (RFC 07).
    Mcp(McpCmd),
    /// Skill management (RFC 06).
    Skill(SkillCmd),
    /// Sister IDE-in-a-terminal frame — text Document Model over the same
    /// Kernel Bus data the HUD serves (RFC 20 Phase 8.4).
    Sister(SisterCmd),
    /// Security supply-chain install gate (RFC 18 §4).
    Security(SecurityCmd),
    /// Swarm role presets — list and spawn (RFC 05 Phase 4.1).
    Swarm(SwarmCmd),
    /// Remote-live dual-PC via external RustDesk (lateral AGPL) + Nate Gentile model (RFC 20 Phase 8.5).
    Remote(RemoteCmd),
    /// Profile switcher (RFC 25 §4).
    Profile(ProfileCmd),
    /// Audit log (RFC 24 §10).
    Audit(AuditCmd),
    /// Journal tail (RFC 19).
    Journal(JournalCmd),
    /// Learning Engine rules — list, promote, deprecate, export/import (RFC 16 §2, RFC 32 Phase 5.1, M40).
    Learn(LearnCmd),
    /// Research: live library docs via the docs gateway (Context7Max →
    /// Context7 MCP shape → official docs, always available), collective
    /// engineering intelligence (`query`, always available — four
    /// dimension scorers + fail-safe, RFC 10 §5/§7), local
    /// document ingestion into Markdown (`ingest`, only compiled with
    /// the `doc-ingest` feature), plus web ingestion via Firecrawl —
    /// scrape, search, crawl, extract (RFC 28 §E, only compiled with
    /// the `firecrawl` feature).
    Research(ResearchCmd),
    /// Toast queue management — enqueue, list, cancel (RFC 28 §F).
    #[cfg(feature = "toast")]
    Toast(ToastCmd),
}
