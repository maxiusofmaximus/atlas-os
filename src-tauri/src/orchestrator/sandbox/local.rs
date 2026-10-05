// Atlas OS — Local sandbox backend (RFC 63 §4; default).
//
// The ambient shell: for a Harbor task container this is already an isolated
// environment, so `Local` is the correct default. It also enforces the RFC 28
// §I-style **network allowlist** declaratively: when `ATLAS_NET_ALLOWLIST` is
// set (comma-separated hosts), a command that references an `http(s)://host`
// whose host is not on the list is refused as a `CommandResult` error. This is
// a heuristic guard, not kernel-level egress control — the honest contract is
// "refuse obvious un-allowed fetches", which is what a shell-command allowlist
// can actually promise. Kernel-grade isolation is the job of an external
// backend (WSL2/Daytona), per RFC 25 §11.

use std::path::Path;
use std::time::Duration;

use super::Sandbox;
use crate::orchestrator::agent::{run_command, CommandResult};

pub struct LocalSandbox;

impl LocalSandbox {
    /// Hosts permitted when `ATLAS_NET_ALLOWLIST` is set. Empty = no restriction.
    pub fn network_allowlist() -> Vec<String> {
        std::env::var("ATLAS_NET_ALLOWLIST")
            .ok()
            .map(|s| {
                s.split(',')
                    .map(|h| h.trim().to_ascii_lowercase())
                    .filter(|h| !h.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// True when the command is allowed under the current network allowlist.
    /// Extraction is deliberately conservative: every `http(s)://host` token
    /// found must be permitted. With no allowlist set, everything is allowed.
    pub fn command_allowed(command: &str, allow: &[String]) -> bool {
        if allow.is_empty() {
            return true;
        }
        for host in extract_hosts(command) {
            if !allow
                .iter()
                .any(|a| host == *a || host.ends_with(&format!(".{a}")))
            {
                return false;
            }
        }
        true
    }
}

/// Pull the host out of every `http://` / `https://` URL in `cmd`.
fn extract_hosts(cmd: &str) -> Vec<String> {
    let mut out = Vec::new();
    for scheme in ["http://", "https://"] {
        let mut rest = cmd;
        while let Some(i) = rest.find(scheme) {
            rest = &rest[i + scheme.len()..];
            let host: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
                .collect();
            if !host.is_empty() {
                out.push(host.to_ascii_lowercase());
            }
        }
    }
    out
}

impl Sandbox for LocalSandbox {
    fn kind(&self) -> &'static str {
        "local"
    }
    fn exec(&self, cwd: &Path, command: &str, timeout: Duration) -> CommandResult {
        let allow = Self::network_allowlist();
        if !Self::command_allowed(command, &allow) {
            return CommandResult {
                command: command.to_string(),
                exit_code: 126,
                stdout: String::new(),
                stderr: format!(
                    "blocked by network allowlist (ATLAS_NET_ALLOWLIST = {})",
                    allow.join(", ")
                ),
            };
        }
        run_command(cwd, command, timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_when_no_allowlist() {
        assert!(LocalSandbox::command_allowed("curl http://evil.test", &[]));
    }

    #[test]
    fn blocks_unallowed_host() {
        let allow = vec!["example.com".to_string()];
        assert!(!LocalSandbox::command_allowed(
            "curl https://evil.test/x",
            &allow
        ));
        assert!(LocalSandbox::command_allowed(
            "curl https://example.com/x",
            &allow
        ));
        assert!(LocalSandbox::command_allowed(
            "curl https://api.example.com/x",
            &allow
        ));
    }

    #[test]
    fn extracts_hosts() {
        let h = extract_hosts("wget http://a.b/c && curl https://d-e.f");
        assert_eq!(h, vec!["a.b".to_string(), "d-e.f".to_string()]);
    }

    #[test]
    fn blocked_exec_reports_126() {
        // Set the allowlist only for this check; restore after.
        std::env::set_var("ATLAS_NET_ALLOWLIST", "good.test");
        let dir = tempfile::TempDir::new().unwrap();
        let r = LocalSandbox.exec(dir.path(), "curl http://bad.test", Duration::from_secs(5));
        std::env::remove_var("ATLAS_NET_ALLOWLIST");
        assert_eq!(r.exit_code, 126);
        assert!(r.stderr.contains("allowlist"));
    }
}
