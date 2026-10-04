// Atlas OS — Rust entry point (Tauri 2 desktop).
// Atlas OS is a Tauri 2 desktop app whose Rust core also serves a local
// axum WebSocket server for the HUD Mission Control (RFC 24) and a sidecar CLI
// (`atlas`) for headless use (RFC 25 §3.9, RFC 08).
// The core survives webview crashes: the axum server runs in a separate Tokio
// task and stays up even if the Tauri window is destroyed (RFC 25 §2, RFC 19).
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::missing_errors_doc)]

use std::sync::Arc;

use atlas_os::AppState;
use tauri::Manager;
#[cfg(feature = "hud")]
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

/// Boot the `Atlas OS` desktop shell.
///
/// Spawns the HUD axum server and the LSP host on dedicated Tokio runtimes
/// kept alive independently of the Tauri webview, then enters the Tauri
/// event loop. The function returns only after the desktop shell exits
/// and the HUD thread has joined.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::missing_panics_doc)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,atlas_os=debug")),
        )
        .with_target(true)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .init();

    let state = match AppState::bootstrap() {
        Ok(s) => Arc::new(s),
        Err(err) => {
            tracing::error!(error = %err, "failed to bootstrap Atlas OS; aborting");
            std::process::exit(1);
        }
    };

    // Cancellation token shared with the HUD thread so the Tauri
    // window-close path can trigger axum graceful shutdown and the HUD
    // runtime can join cleanly before process exit (RFC 25 §2, RFC 19).
    #[cfg(feature = "hud")]
    let hud_shutdown = CancellationToken::new();

    #[cfg(feature = "hud")]
    let hud_state = Arc::clone(&state);
    #[cfg(feature = "hud")]
    let hud_shutdown_clone = hud_shutdown.clone();
    #[cfg(feature = "hud")]
    let hud_handle = std::thread::Builder::new()
        .name("oc-hud".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("oc-hud-worker")
                .build()
                .expect("failed to build HUD runtime");
            runtime.block_on(atlas_os::hud::serve(hud_state, hud_shutdown_clone));
        })
        .expect("failed to spawn HUD thread");

    #[cfg(feature = "lsp")]
    let lsp_state = Arc::clone(&state);
    #[cfg(feature = "lsp")]
    let _lsp_handle = std::thread::Builder::new()
        .name("oc-lsp".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("oc-lsp-worker")
                .build()
                .expect("failed to build LSP runtime");
            runtime.block_on(atlas_os::lsp::host::serve(lsp_state));
        })
        .expect("failed to spawn LSP thread");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(state.clone())
        .setup(|app| {
            tracing::info!(
                version = env!("CARGO_PKG_VERSION"),
                "Atlas OS desktop started"
            );
            let state = app.state::<Arc<AppState>>();
            tracing::info!(hud_url = %state.hud_url(), "HUD Mission Control reachable");
            Ok(())
        })
        .on_window_event(move |window, event| {
            // Trigger HUD graceful shutdown when the main window closes.
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                tracing::info!("window closed; signalling HUD shutdown");
                #[cfg(feature = "hud")]
                hud_shutdown.cancel();
                let _ = window.app_handle().try_state::<Arc<AppState>>();
            }
        })
        .invoke_handler(tauri::generate_handler![
            atlas_os::core::ipc::mission_new,
            atlas_os::core::ipc::mission_list,
            atlas_os::core::ipc::hud_url,
            atlas_os::core::ipc::journal_tail,
            atlas_os::core::ipc::audit_tail,
            atlas_os::core::ipc::approval_decide,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri shell failed to start");

    #[cfg(feature = "hud")]
    let _ = hud_handle.join();
}

fn main() {
    run();
}
