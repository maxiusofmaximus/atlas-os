// OpenCode OS — `opencode mcp` stub (RFC 07; full impl Phase 5).
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct McpCmd {
    #[command(subcommand)]
    pub action: McpAction,
}

#[derive(Subcommand, Debug)]
pub enum McpAction {
    List,
    Refresh,
}

pub async fn run(cmd: McpCmd, _profile: &str) -> Result<()> {
    match cmd.action {
        McpAction::List => println!("(MCP list not implemented yet — Phase 5 RFC 07)"),
        McpAction::Refresh => println!("(MCP refresh not implemented yet — Phase 5 RFC 07)"),
    }
    Ok(())
}
