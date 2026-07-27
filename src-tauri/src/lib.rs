// OpenCode OS — shared library crate (workspace).
// Exposes `AppState` consumed by both the Tauri desktop binary and the
// headless `opencode` CLI (RFC 25 §3.9). See RFC 25 §2 for the topology.

pub mod cli;
pub mod coding;
pub mod core;
pub mod graph;
pub mod hud;
pub mod journal;
pub mod learning;
pub mod lsp;
pub mod orchestrator;
pub mod planning;
pub mod profiles;
pub mod prompt;
pub mod repair;
pub mod skills;
pub mod supervisor;
pub mod validation;

pub use core::state::AppState;
