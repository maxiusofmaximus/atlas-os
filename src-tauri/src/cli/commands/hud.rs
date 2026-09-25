// Atlas OS — `atlas hud` subcommand: prints HUD URL for the profile.
// Phase 8.3 (research 36 SECTOR B 8.3): `--auth-status` reports the
// remote-access posture (bearer configured? OIDC discovery set?) and
// `--rotate-token` rotates the remote bearer — both thin verbs over
// `crate::remote_auth`, never printing the token itself.
use anyhow::Result;
use clap::Args;

use crate::remote_auth;

#[derive(Args, Debug)]
pub struct HudCmd {
    /// Print the remote-access status (token configured, OIDC issuer,
    /// latency KPI) without revealing the token.
    #[arg(long, default_value_t = false)]
    pub auth_status: bool,
    /// Generate + persist a fresh remote bearer token, printing only a
    /// redacted preview. Implies no other output besides the rotation.
    #[arg(long, default_value_t = false)]
    pub rotate_token: bool,
}

pub async fn run(cmd: HudCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    if cmd.rotate_token {
        let token = remote_auth::rotate_token(&root)?;
        println!(
            "hud [{pid}] remote bearer rotated (preview {})",
            remote_auth::redacted_preview(&token)
        );
        println!("hint: send `Authorization: Bearer <token>` or the `atlas_session` cookie to /remote/status (remote-ui builds).");
        return Ok(());
    }
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
    if cmd.auth_status {
        let expected = remote_auth::read_token(&root).unwrap_or_default();
        let config = remote_auth::RemoteAuthConfig::from_env_with_token(!expected.is_empty());
        let snap = remote_auth::status_snapshot(&config, port);
        println!(
            "remote [{pid}] local_only={} token_configured={} oidc_configured={} issuer={} status=http://localhost:{}/remote/status",
            snap.local_only,
            snap.token_configured,
            snap.oidc_configured,
            snap.oidc_issuer.as_deref().unwrap_or("<unset>"),
            port.unwrap_or(0),
        );
        if !snap.token_configured {
            println!(
                "hint: `atlas hud --rotate-token` mints the bearer; expose remotely only over an authenticated tunnel (Tailscale / SSH / Cloudflare)."
            );
        }
        if let Some(issuer) = snap.oidc_issuer {
            println!(
                "oidc discovery: {}",
                remote_auth::discovery_url(&issuer).unwrap_or_default()
            );
        }
    }
    Ok(())
}
