// Atlas OS — shared library crate (workspace).
// Exposes `AppState` consumed by both the Tauri desktop binary and the
// headless `atlas` CLI (RFC 25 §3.9). See RFC 25 §2 for the topology.

use tauri::Manager;
use tracing_subscriber::EnvFilter;

#[cfg(feature = "acp-server")]
pub mod acp;
pub mod calendar;
pub mod cli;
pub mod coding;
pub mod context;
pub mod core;
#[cfg(feature = "firecrawl")]
pub mod firecrawl;
pub mod graph;
#[cfg(feature = "hud")]
pub mod hud;
pub mod journal;
pub mod learning;
pub mod lsp;
pub mod mobile;
pub mod monitor;
pub mod orchestrator;
pub mod planning;
pub mod profiles;
pub mod prompt;
pub mod remote;
pub mod remote_auth;
pub mod repair;
pub mod research;
pub mod security;
pub mod sister;
pub mod skills;
pub mod supervisor;
pub mod swarm;
#[cfg(feature = "toast")]
pub mod toast;
pub mod validation;

pub use core::state::AppState;

/// Mobile entry point (Tauri 2 Android, RFC 41 Fase 14 — mobile companion).
/// Boots the same core as the desktop shell: tracing, `AppState::bootstrap`,
/// the HUD/LSP threads (feature-gated), then the Tauri event loop. The macro
/// exports the runtime symbols the Android packaging validates — they live in
/// this cdylib, not in the `atlas-os` desktop bin (whose own entry is
/// `main.rs::run`).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::missing_panics_doc)]
pub fn run_app() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,atlas_os=debug")),
        )
        .with_target(true)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .init();

    let state = match AppState::bootstrap() {
        Ok(s) => std::sync::Arc::new(s),
        Err(err) => {
            tracing::error!(error = %err, "failed to bootstrap Atlas OS; aborting");
            std::process::exit(1);
        }
    };

    #[cfg(feature = "hud")]
    let hud_shutdown = tokio_util::sync::CancellationToken::new();

    #[cfg(feature = "hud")]
    let hud_state = std::sync::Arc::clone(&state);
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
            runtime.block_on(hud::serve(hud_state, hud_shutdown_clone));
        })
        .expect("failed to spawn HUD thread");

    let lsp_state = std::sync::Arc::clone(&state);
    let _lsp_handle = std::thread::Builder::new()
        .name("oc-lsp".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("oc-lsp-worker")
                .build()
                .expect("failed to build LSP runtime");
            runtime.block_on(lsp::host::serve(lsp_state));
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
        .manage(std::sync::Arc::clone(&state))
        .setup(|app| {
            tracing::info!(
                version = env!("CARGO_PKG_VERSION"),
                "Atlas OS mobile started"
            );
            let state = app.state::<std::sync::Arc<AppState>>();
            tracing::info!(hud_url = %state.hud_url(), "HUD Mission Control reachable");
            Ok(())
        })
        .on_window_event(move |window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                tracing::info!("window closed; signalling HUD shutdown");
                #[cfg(feature = "hud")]
                hud_shutdown.cancel();
                let _ = window.app_handle().try_state::<std::sync::Arc<AppState>>();
            }
        })
        .invoke_handler(tauri::generate_handler![
            core::ipc::mission_new,
            core::ipc::mission_list,
            core::ipc::hud_url,
            core::ipc::journal_tail,
            core::ipc::audit_tail,
            core::ipc::approval_decide,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri shell failed to start");

    #[cfg(feature = "hud")]
    let _ = hud_handle.join();
}
