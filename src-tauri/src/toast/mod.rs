// OpenCode OS — Toast notifications module (RFC 28 Section F).
//
// Provides a Windows Toast notification scheduler backed by a SQLite
// queue so notifications survive process crashes. The two building
// blocks are independent:
//
// 1. `ToastQueue` — the journal-backed queue CRUD. Platform-agnostic
//    Rust + SQLite; compiles on any target the rest of OpenCode OS
//    does. The M17 migration in `journal::schema` creates the
//    `toast_queue` + `toast_history` tables.
//
// 2. `ToastManager` (Windows) / `StubManager` (other OSes) — the
//    dispatcher. On Windows (`cfg(windows)` + feature `toast` enabled)
//    it wraps `winrt_toast_reborn::ToastManager` and honours
//    `on_activated`/`on_dismissed`/`on_failed` callbacks so the HUD
//    can deep-link back to a Mission. On non-Windows targets the
//    stub simply logs via `tracing::info!` so callers can verify
//    ordering without any OS surface.
//
// The `ToastDriver` ties the two: a single `tokio::spawn` task polls
// `ToastQueue::next_pending(now)` every 5 s, dispatches via the
//    manager, and persists the outcome back into the queue/history.
//
// Uses `winrt-toast-reborn = "0.3.8"` (MIT) by Md. Iftakhar Awal
// Chowdhury (AtifChy), fork maintained of winrt-toast 0.1.1.
// https://github.com/AtifChy/winrt-toast
//
// The module is gated behind the `toast` feature flag (default OFF,
// RFC 25 §11 — no behaviour change when off). On non-Windows targets,
// the feature still compiles (the WinRT dep isn't even pulled in),
// but dispatch becomes a logging no-op.

pub mod error;
pub mod manager;
pub mod payload;
pub mod queue;
pub mod scheduler;

pub use error::ToastFacadeError;
pub use manager::ToastDispatcher;
pub use payload::{ToastKind, ToastPayload, ToastStatus};
pub use queue::{QueueRow, ToastQueue};
pub use scheduler::ToastDriver;
