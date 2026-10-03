// Atlas OS — LSP host entry (Phase 19).
// Serves `AtlasLspBackend` over stdio when the host is a pipe or the
// operator forces it (`ATLAS_LSP_STDIO=1`); otherwise parks so the desktop
// webview thread never blocks on a pipe.
use std::io::IsTerminal;
use std::sync::Arc;

use crate::core::state::AppState;

use super::server::serve_stdio;

pub async fn serve(state: Arc<AppState>) {
    let forced = std::env::var_os("ATLAS_LSP_STDIO").is_some();
    let piped = !std::io::stdout().is_terminal();
    if forced || piped {
        tracing::info!("lsp: stdio host starting");
        serve_stdio(state).await;
        return;
    }
    tracing::info!("lsp: desktop embedding — no stdio host, parked");
    std::future::pending::<()>().await;
}

#[cfg(test)]
mod tests {
    #[test]
    fn desktop_park_termination_is_reasonable() {
        // The default desktop path parks; nothing to assert on behaviorally
        // without a pty. The pure capability assertion lives in `server::tests`.
    }
}
