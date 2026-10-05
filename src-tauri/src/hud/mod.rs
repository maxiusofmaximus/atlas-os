// Atlas OS — HUD Mission Control server (RFC 24).
// Served locally by the Rust core via axum + WebSocket (RFC 25 §3.8).
// Survives webview crashes (RFC 25 §2): runs in a dedicated Tokio runtime
// spawned by the desktop binary.

pub mod annotate;
pub mod approvals;
pub mod autoresearch;
pub mod availability;
pub mod cards;
pub mod eval;
pub mod export;
#[cfg(feature = "dag_mode")]
pub mod graph;
pub mod observer;
pub mod reliability;
pub mod remote_status;
pub mod server;
pub mod tail;
pub mod ws;

pub use server::serve;
