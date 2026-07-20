// OpenCode OS — CLI module (RFC 08, RFC 25 §3.9).
// Exposes subcommands usable from both the `opencode` headless binary and
// the Tauri shell when the user wants a local terminal experience.

pub mod commands;
pub mod proto;

pub use commands::*;
pub use proto::Cli;
