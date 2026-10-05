// Atlas OS — Daytona sandbox preset (RFC 63 §4, item 4; lateral, never bundled).
//
// Thin wrapper over the shared CLI bridge. `atlas_sandbox=daytona` selects it
// when the `daytona` CLI is on PATH; otherwise `resolve_sandbox` falls back to
// `Local`, so a task never breaks because an optional backend is missing.

use super::bridge::{self, CliBridgeSandbox};

/// Default binary; `ATLAS_DAYTONA_BIN` overrides.
const BIN: &str = "daytona";

pub fn sandbox() -> CliBridgeSandbox {
    CliBridgeSandbox::daytona()
}

/// `true` when the Daytona CLI is usable on this host.
pub fn available() -> bool {
    let bin = std::env::var("ATLAS_DAYTONA_BIN")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| BIN.to_string());
    bridge::available(&bin)
}
