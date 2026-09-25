// Atlas OS — `atlas remote` Remote-live dual-PC (RFC 20 Phase 8
// sub-fase 8.5, research 36 SECTOR B 8.5).
//
// Thin verb over `crate::remote`: detects the external RustDesk binary
// (lateral AGPL — never bundled/linked), prints the Nate Gentile
// dual-PC checklists, and spawns RustDesk via `std::process` only on
// explicit `--launch`. Without RustDesk every verb degrades to the
// useful-message path instead of failing.

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::remote::{
    client_checklist, missing_message, nate_gentile_guide, normalize_peer_id, peer_id_looks_valid,
    resolve, server_checklist, spawn_session, RemoteStatus,
};

#[derive(Args, Debug)]
pub struct RemoteCmd {
    #[command(subcommand)]
    pub sub: RemoteSub,
}

#[derive(Subcommand, Debug)]
pub enum RemoteSub {
    /// Detect RustDesk in PATH / ATLAS_RUSTDESK_BIN (never installs it).
    Status,
    /// Print the Nate Gentile dual-PC model + AGPL lateral note.
    Guide,
    /// Server-side checklist (PC potente); `--launch` spawns RustDesk share screen.
    Serve {
        /// Spawn the external RustDesk binary after printing the checklist.
        #[arg(long)]
        launch: bool,
        /// Explicit RustDesk binary path (overrides PATH + ATLAS_RUSTDESK_BIN).
        #[arg(long)]
        bin: Option<String>,
    },
    /// Client-side checklist (PC thin); `--launch` spawns the viewer.
    Connect {
        /// RustDesk peer id of the server PC (optional for the checklist).
        #[arg(long)]
        peer_id: Option<String>,
        /// Spawn the external RustDesk viewer after printing the checklist.
        #[arg(long)]
        launch: bool,
        /// Explicit RustDesk binary path (overrides PATH + ATLAS_RUSTDESK_BIN).
        #[arg(long)]
        bin: Option<String>,
    },
}

pub async fn run(cmd: RemoteCmd, profile: &str) -> Result<()> {
    match cmd.sub {
        RemoteSub::Status => status(profile),
        RemoteSub::Guide => guide(),
        RemoteSub::Serve { launch, bin } => serve(launch, bin.as_deref(), profile),
        RemoteSub::Connect {
            peer_id,
            launch,
            bin,
        } => connect(peer_id.as_deref(), launch, bin.as_deref()),
    }
}

fn status(profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    match resolve(None) {
        RemoteStatus::Available { bin } => {
            println!("remote [{pid}] rustdesk available: {}", bin.display());
            println!(
                "role model: server (PC potente) + client (PC thin) — see `atlas remote guide`."
            );
            Ok(())
        }
        RemoteStatus::Missing => {
            println!("remote [{pid}] rustdesk missing");
            println!("{}", missing_message());
            Ok(())
        }
    }
}

fn guide() -> Result<()> {
    println!("{}", nate_gentile_guide());
    Ok(())
}

fn serve(launch: bool, bin_override: Option<&str>, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let hud_port: Option<u16> = std::fs::read_to_string(root.join("hud_port.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok());
    println!("{}", server_checklist(hud_port));
    if !launch {
        return Ok(());
    }
    match resolve(bin_override) {
        RemoteStatus::Available { bin } => {
            let child = spawn_session(&bin, crate::remote::RemoteRole::Server, None)?;
            println!(
                "remote [{pid}] rustdesk share screen spawned (pid {child}): {}",
                bin.display()
            );
            Ok(())
        }
        RemoteStatus::Missing => {
            println!("{}", missing_message());
            Ok(())
        }
    }
}

fn connect(peer_id: Option<&str>, launch: bool, bin_override: Option<&str>) -> Result<()> {
    let normalized = peer_id.and_then(normalize_peer_id);
    if let Some(ref id) = normalized {
        if !peer_id_looks_valid(id) {
            anyhow::bail!(
                "--peer-id `{id}` looks invalid (alphanumerics, `-`, `_` only, max 64 chars)"
            );
        }
    }
    println!("{}", client_checklist(normalized.as_deref()));
    if !launch {
        return Ok(());
    }
    match resolve(bin_override) {
        RemoteStatus::Available { bin } => {
            let child = spawn_session(
                &bin,
                crate::remote::RemoteRole::Client,
                normalized.as_deref(),
            )?;
            println!(
                "remote rustdesk viewer spawned (pid {child}): {}",
                bin.display()
            );
            Ok(())
        }
        RemoteStatus::Missing => {
            println!("{}", missing_message());
            Ok(())
        }
    }
}
