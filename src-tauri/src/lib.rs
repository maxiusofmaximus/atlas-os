// Atlas OS — shared library crate (workspace).
// Exposes `AppState` consumed by both the Tauri desktop binary and the
// headless `atlas` CLI (RFC 25 §3.9). See RFC 25 §2 for the topology.

#[cfg(feature = "acp-server")]
pub mod acp;
pub mod calendar;
pub mod cli;
pub mod coding;
pub mod core;
#[cfg(feature = "firecrawl")]
pub mod firecrawl;
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
pub mod research;
pub mod security;
pub mod skills;
pub mod supervisor;
pub mod swarm;
#[cfg(feature = "toast")]
pub mod toast;
pub mod validation;

pub use core::state::AppState;
