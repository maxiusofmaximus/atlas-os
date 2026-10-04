// Atlas OS — exec.run tool (RFC 63 §4: exec.run via Sandbox).
//
// Wraps the terminal executor used by the agent loop. Today the backend is the
// local shell (the task container itself); the `Sandbox` trait (RFC 63 element
// 3) makes swapping in WSL2/Daytona a config change, not a rewrite. The command
// runs in the workspace root with a bounded timeout.

use std::time::Duration;

use serde::Deserialize;

use crate::security::sandbox::SensitiveAction;

use super::{Tool, ToolContext, ToolResult};

#[derive(Deserialize)]
struct RunArg {
    command: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

pub struct ExecRunTool;

impl Tool for ExecRunTool {
    fn name(&self) -> &'static str {
        "exec.run"
    }
    fn description(&self) -> &'static str {
        "Run a shell command in the workspace. Args: {\"command\": \"...\", \"timeout_ms\"?}"
    }
    fn sensitivity(&self) -> SensitiveAction {
        SensitiveAction::ShellSandbox
    }
    fn execute(&self, ctx: &ToolContext, args: &serde_json::Value) -> ToolResult {
        let a: RunArg = match serde_json::from_value(args.clone()) {
            Ok(a) => a,
            Err(e) => return ToolResult::err(self.name(), format!("bad arguments: {e}")),
        };
        if a.command.trim().is_empty() {
            return ToolResult::err(self.name(), "empty command");
        }
        let timeout = Duration::from_millis(a.timeout_ms.unwrap_or(ctx.default_timeout_ms));
        let res = crate::orchestrator::agent::run_command(&ctx.root, &a.command, timeout);
        let mut out = ToolResult {
            tool: self.name().to_string(),
            ok: res.exit_code == 0,
            exit_code: Some(res.exit_code),
            output: res.stdout,
            error: None,
            args_json: None,
        };
        if !res.stderr.trim().is_empty() {
            if out.output.is_empty() {
                out.output = res.stderr;
            } else {
                out.output.push_str("\n[stderr]\n");
                out.output.push_str(&res.stderr);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_and_captures_output() {
        let dir = tempfile::TempDir::new().unwrap();
        let c = ToolContext::new(dir.path());
        let r = ExecRunTool.execute(&c, &serde_json::json!({"command": "echo hi"}));
        assert!(r.ok);
        assert_eq!(r.exit_code, Some(0));
        assert!(r.output.contains("hi"));
    }

    #[test]
    fn nonzero_exit_is_not_ok() {
        let dir = tempfile::TempDir::new().unwrap();
        let c = ToolContext::new(dir.path());
        let r = ExecRunTool.execute(&c, &serde_json::json!({"command": "exit 4"}));
        assert!(!r.ok);
        assert_eq!(r.exit_code, Some(4));
    }

    #[test]
    fn empty_command_is_rejected() {
        let dir = tempfile::TempDir::new().unwrap();
        let c = ToolContext::new(dir.path());
        let r = ExecRunTool.execute(&c, &serde_json::json!({"command": "   "}));
        assert!(!r.ok);
    }
}
