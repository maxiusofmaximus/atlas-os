// Atlas OS — `atlas serve` headless daemon (RFC 29 §3.A, research/61 gap C).
//
// Runs the Kernel Bus + HUD axum server WITHOUT the Tauri webview, so the
// runtime survives closing the desktop — the "AI Employee" posture. The
// operator connects from a desktop/mobile over an authenticated tunnel
// (SSH / Tailscale / Cloudflare). Binding beyond loopback exposes the
// HUD, so a non-loopback host prints an explicit warning (RFC 18).
//
// This is the foundation for the multi-channel gateway (RFC 29 §3.B):
// a daemon that outlives the laptop is what makes "steer from anywhere"
// meaningful. No new crate — it reuses the axum server the desktop
// already runs (`hud::serve_on`).

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Args;
use tokio_util::sync::CancellationToken;

use crate::core::state::AppState;
use crate::profiles::ProfileId;

#[derive(Args, Debug)]
pub struct ServeCmd {
    /// Interface to bind. Defaults to loopback (local only).
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// Port to bind. 0 = ephemeral (chosen by the OS, printed on boot).
    #[arg(long, default_value_t = 0)]
    pub port: u16,
}

pub async fn run(cmd: ServeCmd, profile: &str) -> Result<()> {
    let bind = parse_bind(&cmd.host, cmd.port)?;

    if !bind.ip().is_loopback() {
        eprintln!(
            "warning: binding {bind} exposes the HUD beyond loopback — only do this \
             behind an authenticated tunnel (SSH / Tailscale / Cloudflare) with the \
             remote bearer configured (`atlas hud --rotate-token`)."
        );
    }

    let pid = ProfileId::new(profile);
    let state =
        Arc::new(AppState::bootstrap_for(pid.clone()).context("bootstrapping Atlas kernel")?);
    let shutdown = CancellationToken::new();

    // Ctrl-C → graceful shutdown so the SQLite WAL flushes before exit.
    let signal_token = shutdown.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal_token.cancel();
        }
    });

    let server_state = Arc::clone(&state);
    let server = tokio::spawn(async move {
        crate::hud::serve_on(server_state, bind, shutdown).await;
    });

    // `serve_on` publishes the resolved port after bind; wait briefly so
    // the operator sees the URL while the daemon is still running.
    let mut port = 0u16;
    for _ in 0..250 {
        tokio::time::sleep(Duration::from_millis(20)).await;
        port = state.hud_port();
        if port != 0 {
            break;
        }
    }

    let host = cmd.host;
    if port == 0 {
        eprintln!("atlas serve [{pid}] did not publish a port; check the bind address ({bind})");
    } else {
        println!("atlas serve [{pid}] — HUD Mission Control live (Ctrl-C to stop)");
        println!("  HUD:       http://{host}:{port}/");
        println!("  WebSocket: ws://{host}:{port}/ws");
        println!("  Attach:    `atlas hud` (same profile) or the desktop app");
    }

    let _ = server.await;
    println!("atlas serve [{pid}] stopped");
    Ok(())
}

/// Parse `host:port` into a socket address, failing with the offending
/// string rather than a bare `AddrParseError`.
fn parse_bind(host: &str, port: u16) -> Result<SocketAddr> {
    format!("{host}:{port}")
        .parse()
        .with_context(|| format!("invalid bind address `{host}:{port}`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bind_accepts_loopback_and_wildcard() {
        assert_eq!(
            parse_bind("127.0.0.1", 8080).unwrap(),
            "127.0.0.1:8080".parse::<SocketAddr>().unwrap()
        );
        assert!(!parse_bind("0.0.0.0", 8787).unwrap().ip().is_loopback());
    }

    #[test]
    fn parse_bind_rejects_garbage_with_context() {
        let err = parse_bind("not a host", 0).unwrap_err();
        assert!(format!("{err:#}").contains("invalid bind address"));
    }
}
