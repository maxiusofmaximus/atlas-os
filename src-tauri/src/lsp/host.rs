// Atlas OS — LSP host placeholder.
// Real LSP multiplexer arrives Phase 2 (Roadmap §Fase 2). The host will:
//  * detect LanguageId by extension (RFC 03 §9.3 LanguageIdResolver),
//  * spawn language servers as subprocesses,
//  * proxy JSON-RPC traffic,
//  * expose diagnostics into the Kernel Bus (`02 §3.1 hud.agent.status ...`).
//
// For Phase 0 we only announce readiness and sleep.
use std::sync::Arc;

use crate::core::state::AppState;

pub async fn serve(_state: Arc<AppState>) {
    tracing::info!("LSP host placeholder ready (Phase 0)");
    // Park until cancelled — Tauri process will kill this thread on exit.
    std::future::pending::<()>().await;
}
