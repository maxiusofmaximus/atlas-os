// Atlas OS — Agent capability layer: Tool trait + shared types (RFC 63 §4, Fase 27).
//
// The agent loop stops being "just run_command": every capability is a `Tool`
// with a name, a JSON args schema, a declared `SensitiveAction` (RFC 18 §2) so
// the approval policy can gate it, and a pure-ish `execute` that returns a
// structured `ToolResult`. Tools live in a `ToolRegistry`; the loop calls them
// through the registry, never directly, so permissions and instrumentation are
// enforced in one place.

pub mod browse;
pub mod code;
pub mod exec;
pub mod fs;
pub mod registry;
pub mod web;

use serde::{Deserialize, Serialize};

pub use browse::{BrowseOpenTool, BrowseSnapshotTool};
pub use code::CodeApplyDiffTool;
pub use exec::ExecRunTool;
pub use fs::{FsEditTool, FsListTool, FsReadTool, FsWriteTool};
pub use registry::ToolRegistry;
pub use web::{WebFetchTool, WebSearchTool};

/// Outcome of a tool call. `ok` is the tool-level success (not the process
/// exit code); `exit_code` carries the process code for `exec.*`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub output: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The args the tool was called with (JSON string), for `tool_invocations`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args_json: Option<String>,
    /// Wall-clock time the call took, filled by `ToolRegistry::call` (RFC 07 §8
    /// telemetry). Tools build results without it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

impl ToolResult {
    pub fn ok(tool: &str, output: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            ok: true,
            exit_code: None,
            output: output.into(),
            error: None,
            args_json: None,
            duration_ms: None,
        }
    }

    pub fn err(tool: &str, error: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            ok: false,
            exit_code: None,
            output: String::new(),
            error: Some(error.into()),
            args_json: None,
            duration_ms: None,
        }
    }

    /// Attach the call args (builder for instrumentation).
    pub fn with_args(mut self, args_json: impl Into<String>) -> Self {
        self.args_json = Some(args_json.into());
        self
    }

    /// The `command` field of this call's args, when present (`exec.run`).
    pub fn args_command(&self) -> Option<String> {
        let raw = self.args_json.as_ref()?;
        let v: serde_json::Value = serde_json::from_str(raw).ok()?;
        v.get("command").and_then(|c| c.as_str()).map(String::from)
    }

    /// Render for the model: bounded, human-readable.
    pub fn render(&self) -> String {
        const MAX: usize = 4000;
        let mut s = String::new();
        if let Some(code) = self.exit_code {
            s.push_str(&format!("(exit {code})\n"));
        }
        if self.ok {
            s.push_str(&bound(&self.output, MAX));
        } else {
            s.push_str("error: ");
            s.push_str(&bound(
                self.error.as_deref().unwrap_or("unknown error"),
                MAX,
            ));
        }
        s
    }
}

fn bound(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.len() <= max {
        return t.to_string();
    }
    let head = &t[..max / 2];
    let tail = &t[t.len() - max / 2..];
    format!("{head}\n…(truncated)…\n{tail}")
}

/// Context handed to every tool call: the workspace root, the per-call limits,
/// and the sandbox runtime (RFC 63 §4) the command tools execute in.
pub struct ToolContext {
    pub root: std::path::PathBuf,
    pub max_output_bytes: usize,
    pub default_timeout_ms: u64,
    /// WHERE commands run (`local`, `wsl2`). Shared so the registry and the
    /// verifier use one runtime.
    pub sandbox: std::sync::Arc<dyn crate::orchestrator::sandbox::Sandbox>,
}

impl std::fmt::Debug for ToolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolContext")
            .field("root", &self.root)
            .field("max_output_bytes", &self.max_output_bytes)
            .field("default_timeout_ms", &self.default_timeout_ms)
            .field("sandbox", &self.sandbox.kind())
            .finish()
    }
}

impl ToolContext {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            root: root.into(),
            max_output_bytes: 32_768,
            default_timeout_ms: 120_000,
            sandbox: std::sync::Arc::new(crate::orchestrator::sandbox::LocalSandbox),
        }
    }

    /// Attach a specific sandbox runtime (RFC 63 §4). Builder for the host.
    pub fn with_sandbox(
        mut self,
        sandbox: std::sync::Arc<dyn crate::orchestrator::sandbox::Sandbox>,
    ) -> Self {
        self.sandbox = sandbox;
        self
    }
}

/// Why a tool call was refused before execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolError {
    UnknownTool(String),
    /// The approval policy returned `Forbidden` for the tool's `SensitiveAction`.
    Denied {
        tool: String,
        action: &'static str,
    },
    /// A declared approval is required but the caller had no human in the loop.
    NeedsApproval {
        tool: String,
        action: &'static str,
    },
    BadArgs(String),
    Io(String),
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTool(t) => write!(f, "unknown tool `{t}`"),
            Self::Denied { tool, action } => write!(f, "tool `{tool}` denied ({action})"),
            Self::NeedsApproval { tool, action } => {
                write!(f, "tool `{tool}` needs approval ({action})")
            }
            Self::BadArgs(e) => write!(f, "bad arguments: {e}"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for ToolError {}

/// A capability the agent can invoke. `sensitivity` maps to RFC 18's
/// `SensitiveAction`; the registry checks it against the approval policy
/// before `execute` is ever called.
pub trait Tool: Send + Sync {
    /// Stable name the model uses (`fs.read`, `exec.run`, ...).
    fn name(&self) -> &'static str;
    /// One-line description surfaced in `atlas agent tools` and the prompt.
    fn description(&self) -> &'static str;
    /// RFC 18 sensitivity class for the approval policy.
    fn sensitivity(&self) -> crate::security::sandbox::SensitiveAction;
    /// Execute with `args` (already parsed from the model's JSON). Never panics.
    fn execute(&self, ctx: &ToolContext, args: &serde_json::Value) -> ToolResult;
}
