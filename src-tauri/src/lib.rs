// OpenCode OS — shared library crate (workspace).
// Exposes `AppState` consumed by both the Tauri desktop binary and the
// headless `opencode` CLI (RFC 25 §3.9). See RFC 25 §2 for the topology.

pub mod cli;
pub mod core;
pub mod hud;
pub mod journal;
pub mod lsp;
pub mod profiles;
pub mod skills;

pub use core::state::AppState;
