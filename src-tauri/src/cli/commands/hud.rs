// Atlas OS — `atlas hud` subcommand: prints HUD URL for the profile.
use anyhow::Result;
use clap::Args;

#[derive(Args, Debug)]
pub struct HudCmd {}

pub async fn run(_: HudCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    // The HUD port is owned by AppState, not the Journal, but the headless
    // CLI has no AppState running. The port lives only while the desktop
    // binary is alive; we persist it to `hud_port.txt` so the CLI can
    // recover it without needing to attach to the live process.
    let port_file = root.join("hud_port.txt");
    let port: Option<u16> = std::fs::read_to_string(&port_file)
        .ok()
        .and_then(|s| s.trim().parse().ok());
    match port {
        Some(port) if port != 0 => {
            println!("HUD Mission Control reachable at http://localhost:{port}/  (profile {pid})");
            println!("WebSocket: ws://localhost:{port}/ws");
        }
        _ => {
            println!("HUD not running for profile {pid} (no port published yet).");
            println!("Hint: start `atlas-os-desktop` (Tauri) which spawns the HUD server.");
        }
    }
    Ok(())
}
