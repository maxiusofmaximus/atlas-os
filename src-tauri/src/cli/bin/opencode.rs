// OpenCode OS — `opencode` CLI entry point (RFC 08, RFC 25 §3.9).
// Shares the same Rust core as the Tauri desktop binary but runs headless.
// Useful for CI, headless Linux servers, and scripting.
// Subcommands declared here mirror the table in RFC 25 §3.9.

use clap::Parser;
use opencode_os::cli::{commands, proto::Cli};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = match cli.verbose {
        0 => "warn,opencode_os=info",
        1 => "info,opencode_os=debug",
        2 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    let active = cli.active_profile();
    match cli.command {
        Some(cmd) => commands::dispatch(cmd, &active).await,
        None => {
            println!(
                "opencode {} — run `opencode --help` for usage",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
    }
}
