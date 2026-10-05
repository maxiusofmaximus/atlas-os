// Atlas OS — lateral CLI-bridge sandbox (RFC 63 §4, item 4).
//
// `Daytona` and `E2B` are cloud runtimes reached through their own CLI. Like
// `Wsl2`, these are *external processes* (RFC 25 §11): nothing is bundled, and
// if the CLI is absent the caller falls back to `Local`. Because each vendor's
// CLI surface can change, the argv is a template with `{cwd}` and `{cmd}`
// placeholders, overridable per backend via env:
//   ATLAS_DAYTONA_BIN / ATLAS_DAYTONA_TEMPLATE
//   ATLAS_E2B_BIN     / ATLAS_E2B_TEMPLATE
// The binary and template resolve once, at construction.

use std::path::Path;
use std::time::Duration;

use super::Sandbox;
use crate::orchestrator::agent::CommandResult;

/// A sandbox that forwards commands to an external runtime CLI.
pub struct CliBridgeSandbox {
    kind: &'static str,
    program: String,
    template: Vec<String>,
}

impl CliBridgeSandbox {
    pub fn new(kind: &'static str, program: impl Into<String>, template: Vec<String>) -> Self {
        Self {
            kind,
            program: program.into(),
            template,
        }
    }

    /// Daytona preset (`ATLAS_DAYTONA_BIN` / `ATLAS_DAYTONA_TEMPLATE` override).
    pub fn daytona() -> Self {
        Self::from_env(
            "daytona",
            "ATLAS_DAYTONA_BIN",
            "daytona",
            "ATLAS_DAYTONA_TEMPLATE",
            &["exec", "--cwd", "{cwd}", "--", "sh", "-lc", "{cmd}"],
        )
    }

    /// E2B preset (`ATLAS_E2B_BIN` / `ATLAS_E2B_TEMPLATE` override).
    pub fn e2b() -> Self {
        Self::from_env(
            "e2b",
            "ATLAS_E2B_BIN",
            "e2b",
            "ATLAS_E2B_TEMPLATE",
            &["sandbox", "exec", "--", "sh", "-lc", "cd {cwd} && {cmd}"],
        )
    }

    fn from_env(
        kind: &'static str,
        bin_key: &str,
        bin_default: &str,
        tpl_key: &str,
        tpl_default: &[&str],
    ) -> Self {
        let program = std::env::var(bin_key)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| bin_default.to_string());
        let template: Vec<String> =
            match std::env::var(tpl_key).ok().filter(|s| !s.trim().is_empty()) {
                Some(joined) => joined.split_whitespace().map(str::to_string).collect(),
                None => tpl_default.iter().map(|s| s.to_string()).collect(),
            };
        Self {
            kind,
            program,
            template,
        }
    }

    /// Substitute `{cwd}` / `{cmd}` per template element (each stays one argv).
    fn expand(&self, cwd: &Path, command: &str) -> Vec<String> {
        let cwd = cwd.to_string_lossy().to_string();
        self.template
            .iter()
            .map(|item| item.replace("{cwd}", &cwd).replace("{cmd}", command))
            .collect()
    }
}

impl Sandbox for CliBridgeSandbox {
    fn kind(&self) -> &'static str {
        self.kind
    }
    fn exec(&self, cwd: &Path, command: &str, timeout: Duration) -> CommandResult {
        let argv = self.expand(cwd, command);
        run_external(&self.program, &argv, command, timeout)
    }
}

/// True when `bin` is on PATH (probed via `--version`, never fatal).
pub fn available(bin: &str) -> bool {
    std::process::Command::new(bin)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Spawn `program argv...`, capturing output with the same bounded-wait shape as
/// `run_command` (never panics; timeout → exit 124). Kept local so lateral
/// backends share one spawn contract.
pub fn run_external(
    program: &str,
    argv: &[String],
    label: &str,
    timeout: Duration,
) -> CommandResult {
    let child = std::process::Command::new(program)
        .args(argv)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            return CommandResult {
                command: label.to_string(),
                exit_code: 127,
                stdout: String::new(),
                stderr: format!("failed to spawn {program}: {e}"),
            }
        }
    };
    let start = std::time::Instant::now();
    let timed_out = loop {
        match child.try_wait() {
            Ok(Some(_)) => break false,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    break true;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break true,
        }
    };
    let out = child.wait_with_output().ok();
    let (stdout, stderr, code) = match out {
        Some(o) => (
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
            o.status.code().unwrap_or(-1),
        ),
        None => (String::new(), String::new(), -1),
    };
    CommandResult {
        command: label.to_string(),
        exit_code: if timed_out { 124 } else { code },
        stdout,
        stderr: if timed_out {
            format!("{stderr}\n[timeout after {}s]", timeout.as_secs())
        } else {
            stderr
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daytona_expands_template_placeholders() {
        let sb = CliBridgeSandbox::new(
            "daytona",
            "daytona",
            vec![
                "exec".into(),
                "--cwd".into(),
                "{cwd}".into(),
                "--".into(),
                "sh".into(),
                "-lc".into(),
                "{cmd}".into(),
            ],
        );
        let argv = sb.expand(Path::new("/work"), "echo hi");
        assert_eq!(argv[0], "exec");
        assert_eq!(argv[2], "/work");
        assert_eq!(argv[6], "echo hi");
    }

    #[test]
    fn kind_is_stable() {
        assert_eq!(CliBridgeSandbox::daytona().kind(), "daytona");
        assert_eq!(CliBridgeSandbox::e2b().kind(), "e2b");
    }

    #[test]
    fn missing_binary_probe_is_false_not_panic() {
        assert!(!available("atlas-definitely-not-a-real-binary-xyz"));
    }
}
