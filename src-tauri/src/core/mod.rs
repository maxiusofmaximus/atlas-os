// OpenCode OS — modular core submodule.
// Holds the AppState (single shared state for the kernel bus + journal),
// the IPC command handlers invoked from the Tauri frontend, and the
// kernel-bus event types (RFC 02 §3.1).

pub mod bus;
pub mod ipc;
pub mod state;
