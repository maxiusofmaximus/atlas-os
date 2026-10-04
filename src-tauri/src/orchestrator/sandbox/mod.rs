// Atlas OS — Sandbox runtime trait (RFC 63 §4 element 3).
//
// Distinct from `security::sandbox::SandboxLevel` (the *policy* tier). This is
// the *runtime*: WHERE a command runs. The default backend is `Local` (the
// process/container the agent already lives in — which, for a Terminal-Bench
// task, IS an isolated container). WSL2 / Daytona / E2B are lateral backends
// (external process, feature-gated, never bundled — RFC 25 §11): they implement
// the same trait so the agent loop never knows the difference.

use std::path::Path;
use std::time::Duration;

use crate::orchestrator::agent::{run_command, CommandResult};

/// Where the agent's commands execute.
pub trait Sandbox: Send + Sync {
    /// Stable id surfaced in the journal (`local`, `wsl2`, `daytona`).
    fn kind(&self) -> &'static str;
    /// Run `command` with `cwd`, bounded by `timeout`. Never panics.
    fn exec(&self, cwd: &Path, command: &str, timeout: Duration) -> CommandResult;
}

/// Default backend: the ambient shell (the agent's own environment). For a
/// task container this is already isolated; for a local dev run it is the host.
pub struct LocalSandbox;

impl Sandbox for LocalSandbox {
    fn kind(&self) -> &'static str {
        "local"
    }
    fn exec(&self, cwd: &Path, command: &str, timeout: Duration) -> CommandResult {
        run_command(cwd, command, timeout)
    }
}

/// Select a sandbox from `ATLAS_SANDBOX` (default `local`). Lateral backends
/// (`wsl2`, `daytona`) fall back to `Local` until their feature lands, rather
/// than failing — a task never breaks because an optional backend is missing.
pub fn resolve_sandbox() -> Box<dyn Sandbox> {
    match std::env::var("ATLAS_SANDBOX").ok().as_deref() {
        Some("wsl2") | Some("daytona") | Some("e2b") | Some("container") => {
            // Feature-gated lateral backends land in a later sub-phase; until
            // then the task container itself is the sandbox.
            Box::new(LocalSandbox)
        }
        _ => Box::new(LocalSandbox),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_sandbox_runs_a_command() {
        let dir = tempfile::TempDir::new().unwrap();
        let sb = LocalSandbox;
        assert_eq!(sb.kind(), "local");
        let r = sb.exec(dir.path(), "echo sandboxed", Duration::from_secs(20));
        assert_eq!(r.exit_code, 0);
        assert!(r.stdout.contains("sandboxed"));
    }

    #[test]
    fn resolve_defaults_to_local() {
        // No ATLAS_SANDBOX set in the test env by default.
        let sb = resolve_sandbox();
        assert_eq!(sb.kind(), "local");
    }
}
