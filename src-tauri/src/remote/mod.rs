// Atlas OS — Remote-live dual-PC (RFC 20 Phase 8 sub-fase 8.5,
// research 36 SECTOR B 8.5 + SECTOR C + A.3).
//
// RustDesk is AGPL-3.0: Atlas OS NEVER links, bundles, embeds, or
// vendors it. Integration is lateral-only: when the operator already
// installed RustDesk, `atlas remote` detects the external binary in
// `PATH` (or `ATLAS_RUSTDESK_BIN`) and spawns it as a child process
// via `std::process::Command`. Without RustDesk installed, every verb
// degrades to a useful message (download link + Nate Gentile checklist)
// and never fabricates a session.
//
// Modelo Nate Gentile (RFC 20 Sector B): PC servidor (potente, corre
// Atlas OS + HUD + modelos) + PC cliente thin (RustDesk viewer, programa
// en vivo contra los recursos del servidor). El HUD web remoto (8.3) y
// la Sister TUI (8.4) son las superficies de Atlas; RustDesk es solo el
// transporte de pantalla/teclado lateral.
//
// No new crate (RFC 25 §11): detection rides `std::env::split_paths`,
// launch rides `std::process::Command`, output is `serde`-shaped only
// for the CLI print path.

use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// Env var pinning the RustDesk binary (operator override for installs
/// outside `PATH`; the `PATH` lookup remains the default).
pub const RUSTDESK_BIN_ENV: &str = "ATLAS_RUSTDESK_BIN";
/// Binary name probed in `PATH` when no explicit override is set.
pub const RUSTDESK_BIN: &str = "rustdesk";
/// Where to get RustDesk when it is missing (never fetched by Atlas).
pub const RUSTDESK_DOWNLOAD_URL: &str = "https://rustdesk.com/";
/// Lateral-integration license guardrail (research 36 A.3 + SECTOR C).
pub const RUSTDESK_LICENSE_NOTE: &str =
    "RustDesk is AGPL-3.0 — Atlas OS never bundles or links it; lateral external-process launch only.";
/// End-to-end UI latency target shared with the Phase 8 entregable.
pub const REMOTE_LIVE_LATENCY_TARGET_MS: u64 = 100;

/// Dual-PC role in the Nate Gentile model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteRole {
    Server,
    Client,
}

impl RemoteRole {
    pub fn as_str(self) -> &'static str {
        match self {
            RemoteRole::Server => "server",
            RemoteRole::Client => "client",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "server" | "servidor" | "host" => Some(RemoteRole::Server),
            "client" | "cliente" | "viewer" | "thin" => Some(RemoteRole::Client),
            _ => None,
        }
    }
}

/// What `atlas remote status` found on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemoteStatus {
    Available { bin: PathBuf },
    Missing,
}

impl RemoteStatus {
    pub fn available(&self) -> bool {
        matches!(self, RemoteStatus::Available { .. })
    }
}

/// Resolve the RustDesk binary: explicit override wins, then `PATH`.
/// Returns `Missing` instead of erroring so the CLI can print the
/// useful-message path (research 36 §8.5 "Sin RustDesk → mensaje útil").
pub fn resolve(bin_override: Option<&str>) -> RemoteStatus {
    if let Some(b) = bin_override.map(str::trim).filter(|s| !s.is_empty()) {
        return RemoteStatus::Available {
            bin: PathBuf::from(b),
        };
    }
    if let Ok(b) = env::var(RUSTDESK_BIN_ENV) {
        let b = b.trim().to_string();
        if !b.is_empty() {
            return RemoteStatus::Available {
                bin: PathBuf::from(b),
            };
        }
    }
    match find_rustdesk_in_path() {
        Some(bin) => RemoteStatus::Available { bin },
        None => RemoteStatus::Missing,
    }
}

fn find_rustdesk_in_path() -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let names: &[&str] = if cfg!(windows) {
        &[RUSTDESK_BIN, "rustdesk.exe"]
    } else {
        &[RUSTDESK_BIN]
    };
    for dir in env::split_paths(&path) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Args used to spawn the external RustDesk process. Best-effort and
/// lateral: `server` opens the share screen locally (no args — the ID
/// shown in the RustDesk window is what the thin PC dials); `client`
/// passes the server peer id positionally when given so stock RustDesk
/// builds open straight into the session, otherwise opens the viewer
/// for manual dial. Never shells out here — the caller owns `Command`.
pub fn launch_args(role: RemoteRole, peer_id: Option<&str>) -> Vec<String> {
    match role {
        RemoteRole::Server => Vec::new(),
        RemoteRole::Client => match peer_id.map(str::trim).filter(|s| !s.is_empty()) {
            Some(id) => vec![id.to_string()],
            None => Vec::new(),
        },
    }
}

/// Validate a RustDesk peer id (digits, 9-11 chars on stock builds).
/// Lenient on purpose: returns the trimmed id or `None` for blank.
pub fn normalize_peer_id(raw: &str) -> Option<String> {
    let id = raw.trim().replace(' ', "");
    if id.is_empty() {
        return None;
    }
    Some(id)
}

pub fn peer_id_looks_valid(id: &str) -> bool {
    let id = id.trim();
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Spawn the external RustDesk binary. Returns the child PID on success.
/// The AGPL boundary lives here: `Command::new` on an operator-owned
/// path, no link/bundle/embed of RustDesk code or assets.
pub fn spawn_session(
    bin: &PathBuf,
    role: RemoteRole,
    peer_id: Option<&str>,
) -> std::io::Result<u32> {
    let args = launch_args(role, peer_id);
    let child = std::process::Command::new(bin).args(&args).spawn()?;
    let pid = child.id();
    std::mem::forget(child);
    Ok(pid)
}

/// Server-side (PC potente) checklist printed by `atlas remote serve`.
pub fn server_checklist(hud_port: Option<u16>) -> String {
    let hud = match hud_port {
        Some(p) if p != 0 => format!("http://localhost:{p}/ (WS ws://localhost:{p}/ws)"),
        _ => {
            "not running yet — start `atlas-os-desktop` (Tauri) to publish hud_port.txt".to_string()
        }
    };
    format!(
        "PC SERVIDOR (potente) — Nate Gentile model:\n\
         1. Atlas OS + HUD corriendo: {hud}\n\
         2. RustDesk instalado y abierto en MODO compartir (anota el ID de 9 digitos + password de un solo uso).\n\
         3. Red: mismo LAN o RustDesk relay; objetivo latencia UI <100ms (RFC 20 KPI).\n\
         4. Seguridad: password rotativo por sesion, jamas expongas el ID en canales publicos.\n\
         {RUSTDESK_LICENSE_NOTE}"
    )
}

/// Client-side (PC thin) checklist printed by `atlas remote connect`.
pub fn client_checklist(peer_id: Option<&str>) -> String {
    let target = peer_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("<ID-del-servidor>");
    format!(
        "PC CLIENTE (thin) — Nate Gentile model:\n\
         1. RustDesk viewer instalado (lateral, AGPL — {RUSTDESK_DOWNLOAD_URL}).\n\
         2. Conecta al servidor: ID `{target}` (+ password de un solo uso).\n\
         3. Abre el HUD del servidor en el navegador remoto o trabaja directo sobre la sesion RustDesk.\n\
         4. Programa en vivo: el codigo corre en el servidor; el thin solo transporta pantalla/teclado.\n\
         {RUSTDESK_LICENSE_NOTE}"
    )
}

/// Full dual-PC model text printed by `atlas remote guide`.
pub fn nate_gentile_guide() -> String {
    format!(
        "Remote-live dual-PC — modelo Nate Gentile (RFC 20 Sector B, research 36 B 8.5):\n\n\
         - PC SERVIDOR (potente): corre Atlas OS, HUD Mission Control, modelos y worktrees.\n\
         - PC CLIENTE (thin): RustDesk viewer + navegador; programa en vivo contra el servidor.\n\
         - Transporte: RustDesk como proceso EXTERNO (std::process::Command si esta en PATH).\n\
         - {RUSTDESK_LICENSE_NOTE}\n\
         - Sin RustDesk: instala desde {RUSTDESK_DOWNLOAD_URL} y reintenta `atlas remote status`.\n\
         - KPI: latencia end-to-end UI <{ms}ms en LAN (RFC 20); SECTOR C: sin embed del host, sin streaming propio (poll 5s MVP en 8.2)."
        ,
        ms = REMOTE_LIVE_LATENCY_TARGET_MS
    )
}

/// Message printed when RustDesk is not installed (research 36 §8.5).
pub fn missing_message() -> String {
    format!(
        "RustDesk no detectado en PATH ni en ${RUSTDESK_BIN_ENV}.\n\
         Instala RustDesk desde {RUSTDESK_DOWNLOAD_URL} (AGPL-3.0, lateral — Atlas OS jamas lo empaqueta) \
         y reintenta `atlas remote status`.\n\
         Mientras tanto: `atlas remote guide` imprime el modelo Nate Gentile dual-PC.\n\
         {RUSTDESK_LICENSE_NOTE}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_clean_env<F: FnOnce()>(f: F) {
        let _g = ENV_LOCK.lock().unwrap();
        let bin_b = env::var_os(RUSTDESK_BIN_ENV);
        let path_b = env::var_os("PATH");
        env::remove_var(RUSTDESK_BIN_ENV);
        env::set_var(
            "PATH",
            std::env::temp_dir().join("atlas-os-no-rustdesk-xyz"),
        );
        f();
        if let Some(v) = bin_b {
            env::set_var(RUSTDESK_BIN_ENV, v);
        } else {
            env::remove_var(RUSTDESK_BIN_ENV);
        }
        if let Some(v) = path_b {
            env::set_var("PATH", v);
        } else {
            env::remove_var("PATH");
        }
    }

    #[test]
    fn explicit_override_wins_over_path() {
        let st = resolve(Some("/opt/rustdesk/rustdesk"));
        assert_eq!(
            st,
            RemoteStatus::Available {
                bin: PathBuf::from("/opt/rustdesk/rustdesk")
            }
        );
    }

    #[test]
    fn blank_override_falls_through_to_missing() {
        with_clean_env(|| {
            assert_eq!(resolve(Some("   ")), RemoteStatus::Missing);
        });
    }

    #[test]
    fn env_override_selects_available() {
        with_clean_env(|| {
            env::set_var(RUSTDESK_BIN_ENV, "/usr/local/bin/rustdesk");
            assert!(resolve(None).available());
        });
    }

    #[test]
    fn empty_env_resolves_to_missing() {
        with_clean_env(|| {
            assert_eq!(resolve(None), RemoteStatus::Missing);
        });
    }

    #[test]
    fn role_parse_round_trips_including_spanish_aliases() {
        assert_eq!(RemoteRole::parse("server"), Some(RemoteRole::Server));
        assert_eq!(RemoteRole::parse("SERVIDOR"), Some(RemoteRole::Server));
        assert_eq!(RemoteRole::parse("host"), Some(RemoteRole::Server));
        assert_eq!(RemoteRole::parse("client"), Some(RemoteRole::Client));
        assert_eq!(RemoteRole::parse("thin"), Some(RemoteRole::Client));
        assert_eq!(RemoteRole::parse("viewer"), Some(RemoteRole::Client));
        assert_eq!(RemoteRole::parse("nonsense"), None);
        assert_eq!(RemoteRole::Server.as_str(), "server");
        assert_eq!(RemoteRole::Client.as_str(), "client");
    }

    #[test]
    fn launch_args_server_is_bare_and_client_carries_peer_id() {
        assert!(launch_args(RemoteRole::Server, Some("123456789")).is_empty());
        assert!(launch_args(RemoteRole::Client, None).is_empty());
        assert_eq!(
            launch_args(RemoteRole::Client, Some("123456789")),
            vec!["123456789".to_string()]
        );
        assert!(launch_args(RemoteRole::Client, Some("   ")).is_empty());
    }

    #[test]
    fn missing_message_points_to_download_and_env() {
        let msg = missing_message();
        assert!(msg.contains(RUSTDESK_DOWNLOAD_URL));
        assert!(msg.contains(RUSTDESK_BIN_ENV));
        assert!(msg.contains("AGPL"));
        assert!(msg.contains("atlas remote guide"));
    }

    #[test]
    fn guide_names_both_pcs_and_latency_kpi() {
        let guide = nate_gentile_guide();
        assert!(guide.contains("SERVIDOR"));
        assert!(guide.contains("CLIENTE"));
        assert!(guide.contains("Nate Gentile"));
        assert!(guide.contains("100ms"));
        assert!(guide.contains("AGPL"));
    }

    #[test]
    fn peer_id_validation_rejects_shaped_input() {
        assert!(peer_id_looks_valid("123456789"));
        assert!(peer_id_looks_valid("abc-123_X"));
        assert!(!peer_id_looks_valid(""));
        assert!(!peer_id_looks_valid("   "));
        assert!(!peer_id_looks_valid("id with spaces inside!"));
        assert!(!peer_id_looks_valid("semi;colon"));
        assert_eq!(normalize_peer_id("  123 456 "), Some("123456".to_string()));
        assert_eq!(normalize_peer_id("   "), None);
    }

    #[test]
    fn checklists_mention_their_own_side_and_license() {
        let s = server_checklist(Some(1420));
        assert!(s.contains("SERVIDOR"));
        assert!(s.contains("1420"));
        assert!(s.contains("AGPL"));
        let s_off = server_checklist(None);
        assert!(s_off.contains("hud_port.txt"));
        let c = client_checklist(Some("123456789"));
        assert!(c.contains("CLIENTE"));
        assert!(c.contains("123456789"));
        assert!(c.contains("AGPL"));
        let c_blank = client_checklist(None);
        assert!(c_blank.contains("<ID-del-servidor>"));
    }

    #[test]
    fn spawn_missing_binary_surfaces_io_error_without_panic() {
        let bin = PathBuf::from("/nonexistent-atlas-rustdesk-bin-xyz");
        let err = spawn_session(&bin, RemoteRole::Server, None).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
