// Atlas OS — `atlas` CLI entry point (RFC 08, RFC 25 §3.9).
// Shares the same Rust core as the Tauri desktop binary but runs headless.
// Useful for CI, headless Linux servers, and scripting.
// Subcommands declared here mirror the table in RFC 25 §3.9.
//
// RFC 28 §B item 4 — Microsoft Intelligent Terminal ACP host detection:
// when the build enables the `acp-server` feature AND the
// `WT_COM_CLSID` env var is present in the process environment, the
// binary takes over the stdin/stdout pair and runs the ACP JSON-RPC
// server in place of the headless CLI dispatch. The detection honours
// RFC 28 §B line 143 (`WT_COM_CLSID` is the discovery env var IT sets
// when it spawns the agent process). On any other host (macOS, Linux,
// or Windows without IT) the binary falls through to the legacy CLI.
//
// To force the ACP host loop without IT installed (e.g. for manual ACP
// client smoke tests), set environment variable `ATLAS_ACP_FORCE=1`
// alongside a build with `--features acp-server`. This is documented for
// operators only and is not part of the production flow.
// `ATLAS_ACP_FORCE` is also honoured as a legacy fallback for existing
// users who already have it set in their shell environment.

use atlas_os::cli::{commands, proto::Cli};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if should_run_acp_server() {
        return run_acp_server().await;
    }

    let cli = Cli::parse();

    let filter = match cli.verbose {
        0 => "warn,atlas_os=info",
        1 => "info,atlas_os=debug",
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
                "atlas {} — run `atlas --help` for usage",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
    }
}

/// Return `true` when the binary should hand stdin/stdout to the ACP
/// JSON-RPC server instead of the clap CLI dispatcher. See the
/// module-level doc for the two triggers.
#[cfg(feature = "acp-server")]
fn should_run_acp_server() -> bool {
    std::env::var_os("WT_COM_CLSID").is_some()
        || std::env::var_os("ATLAS_ACP_FORCE")
            .or_else(|| std::env::var_os("ATLAS_ACP_FORCE"))
            .is_some()
}

#[cfg(not(feature = "acp-server"))]
fn should_run_acp_server() -> bool {
    false
}

#[cfg(feature = "acp-server")]
async fn run_acp_server() -> anyhow::Result<()> {
    // tracing logs must NOT pollute stdout — stdout is the ACP JSON-RPC
    // transport. Route logs to stderr so the IT host (or a smoke-test
    // harness) sees them out-of-band.
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info,atlas_os=debug")
        .with_target(true)
        .with_writer(std::io::stderr)
        .try_init();
    tracing::info!("acp-server: handing stdin/stdout to ACP JSON-RPC host loop");
    atlas_os::acp::run_server()
        .await
        .map_err(|e| anyhow::anyhow!("acp server error: {e:?}"))
}

#[cfg(not(feature = "acp-server"))]
async fn run_acp_server() -> anyhow::Result<()> {
    // Unreachable — `should_run_acp_server` always returns false when the
    // feature is disabled. The stub exists so the function-level `cfg`
    // branches compile without dragging `atlas_os::acp` into the bin.
    anyhow::bail!("acp-server feature is not enabled")
}
