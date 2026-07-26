// OpenCode OS — HUD axum server entry point.
// RFC 24 §1 layout, §4 events, §16 mobile remote access.
// Bare bones for Phase 0; expanded Roadmap Fase 8 — UI v2.

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::core::state::AppState;

/// Serve the HUD axum server until `shutdown` is cancelled or the runtime is
/// torn down. The caller (Tauri desktop binary) holds a clone of `shutdown`
/// and cancels it from `on_window_event(close_requested)` so the HUD thread
/// joins cleanly and the SQLite WAL flushes before the process exits.
pub async fn serve(state: Arc<AppState>, shutdown: CancellationToken) {
    // Bind to a kernel-supplied ephemeral port so collisions are impossible.
    let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
        Ok(l) => l,
        Err(err) => {
            tracing::error!(error = %err, "HUD: failed to bind; HUD unreachable");
            return;
        }
    };
    let addr = listener
        .local_addr()
        .expect("HUD: listener has no local_addr");
    tracing::info!(addr = %addr, "HUD Mission Control listening");
    state.set_hud_port(addr.port());

    // Persist the port to `~/.opencode/profiles/<id>/hud_port.txt` so the
    // headless `opencode hud` CLI (which has no AppState) can recover it.
    {
        let port_file = state.profile_root().join("hud_port.txt");
        if let Err(e) = std::fs::write(&port_file, addr.port().to_string()) {
            tracing::warn!(error = %e, path = %port_file.display(), "failed to persist hud_port");
        }
    }

    // Announce to the Kernel Bus so any HUD consumer can find this port.
    let _ = state.bus().send(crate::core::bus::BusEvent::new(
        crate::core::bus::BusEventKind::HudServed {
            hud_port: addr.port(),
        },
    ));

    let app = Router::new()
        .route("/health", get(health))
        .route("/ws", get(super::ws::ws_handler))
        // Phase 1 — nine tail endpointes (RFC 24 §2/§3) plus the raw
        // payload drill-down used by the Audit view. Each tail returns
        // the latest N rows as JSON; defaults configured in `tail.rs`.
        .route("/tail/journal", get(super::tail::tail_journal))
        .route("/tail/missions", get(super::tail::tail_missions))
        .route("/tail/verdicts", get(super::tail::tail_verdicts))
        .route("/tail/consolidated", get(super::tail::tail_consolidated))
        .route("/tail/plans", get(super::tail::tail_plans))
        .route("/tail/diffs", get(super::tail::tail_diffs))
        .route(
            "/tail/validation_reports",
            get(super::tail::tail_validation_reports),
        )
        .route("/tail/repairs", get(super::tail::tail_repairs))
        .route("/tail/patterns", get(super::tail::tail_patterns))
        .route("/tail/checkpoints", get(super::tail::tail_checkpoints))
        .route("/tail/skills", get(super::tail::tail_skills))
        .route("/tail/model_swaps", get(super::tail::tail_model_swaps))
        .route("/tail/step_states", get(super::tail::tail_step_states))
        .route(
            "/diff/:id/annotation",
            post(super::annotate::post_annotation).get(super::annotate::get_annotations),
        )
        .route(
            "/audit/export-posting",
            post(super::export::post_export_posting),
        )
        .route("/payload/:kind/:id", get(super::tail::payload))
        .route(
            "/payload/skill/:skill_id/:version",
            get(super::tail::payload_skill),
        )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Graceful shutdown — when the Tauri window closes, `shutdown.cancel()`
    // fires, axum stops accepting new connections and finishes active ones,
    // the serve future resolves, and `main.rs`' `hud_handle.join()` returns.
    let serve_fut = axum::serve(listener, app).with_graceful_shutdown(async move {
        shutdown.cancelled().await;
    });
    if let Err(err) = serve_fut.await {
        tracing::error!(error = %err, "HUD axum server terminated");
    }
}

async fn health() -> &'static str {
    "ok"
}
