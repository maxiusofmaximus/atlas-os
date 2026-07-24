// OpenCode OS — modular core submodule.
// Holds the AppState (single shared state for the kernel bus + journal),
// the IPC command handlers invoked from the Tauri frontend, the
// kernel-bus event types (RFC 02 §3.1), and the unified mission
// pipeline entry-point (RFC 25 §3.1.1 SOP).

pub mod bus;
pub mod ipc;
pub mod pipeline;
pub mod state;
