// Atlas OS — E2B sandbox preset (RFC 63 §4, item 4; lateral, never bundled).
//
// Thin wrapper over the shared CLI bridge. `atlas_sandbox=e2b` selects it when
// the `e2b` CLI is on PATH; otherwise `resolve_sandbox` falls back to `Local`.

use super::bridge::{self, CliBridgeSandbox};

/// Default binary; `ATLAS_E2B_BIN` overrides.
const BIN: &str = "e2b";

pub fn sandbox() -> CliBridgeSandbox {
    CliBridgeSandbox::e2b()
}

/// `true` when the E2B CLI is usable on this host.
pub fn available() -> bool {
    let bin = std::env::var("ATLAS_E2B_BIN")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| BIN.to_string());
    bridge::available(&bin)
}
