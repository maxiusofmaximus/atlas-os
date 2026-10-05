// Atlas OS — MCP runtime (RFC 07 materialization, Fase 29).
//
// RFC 07 specifies a *secure* MCP client: a registry with per-server
// trust/sandbox/tool-allowlist, a stdio JSON-RPC transport, and an
// execution policy that never exposes a tool to the agent unless the
// operator listed it. Before Fase 29 this was a 23-line CLI stub and the
// HUD's `GET /hud/mcp` was the only honest read side.
//
// Fase 29.0 (this module) delivers the client core:
//   * `config`   — registry loading for both the opencode-compatible
//                  `{ "mcp": {...} }` shape Atlas already uses and the
//                  RFC 07 `{ "mcpServers": {...} }` shape;
//   * `protocol` — JSON-RPC 2.0 shapes + MCP method payloads (pure);
//   * `client`   — stdio transport + `initialize`/`tools/list`/
//                  `tools/call`, timeout-bounded, allowlist-enforced.
//
// Sandbox *policy* (RFC 07 §2) is applied in `config::McpServerConfig`
// (`effective_sandbox`/`sandbox_finding`): an unsigned server is forced to
// `container`, a signed one floored at `vuOnly`, and `atlas mcp list/add/probe/
// call` surface the finding (warn by default, refuse under `--strict`).
// Real isolation (running under vuOnly/container), supply-chain verification
// (§3), the RFC 63 `ToolRegistry` bridge and telemetry (§8) are later sub-phases.
//
// No new crate: transport rides `tokio::process`, framing rides
// `serde_json`, errors ride `thiserror` — all already in the tree
// (RFC 25 §11, single-binary safe).

pub mod client;
pub mod config;
pub mod protocol;

pub use client::{McpClient, McpError, McpTransport, StdioTransport};
pub use config::{McpRegistry, McpSandbox, McpServerConfig};
pub use protocol::{ServerInfo, ToolCallOutcome, ToolInfo, MCP_PROTOCOL_VERSION};
