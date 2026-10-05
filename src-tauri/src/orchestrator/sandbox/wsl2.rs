// Atlas OS — WSL2 sandbox backend (RFC 63 §4; lateral, never bundled).
//
// A command backend that runs inside a WSL2 distro via the `wsl.exe` bridge —
// an *external process*, per RFC 25 §11. On Windows this gives the agent a real
// Linux environment without Docker; on other hosts `wsl2_available()` is false
// and `resolve_sandbox` stays `Local`. Nothing is installed or bundled: if the
// user has WSL, this works; if not, the default `Local` is used.

use std::path::Path;
use std::time::Duration;

use super::Sandbox;
use crate::orchestrator::agent::CommandResult;

/// Default distro when `ATLAS_WSL_DISTRO` is unset.
const DEFAULT_DISTRO: &str = "Ubuntu-24.04";

pub struct Wsl2Sandbox {
    distro: String,
}

impl Wsl2Sandbox {
    pub fn new(distro: impl Into<String>) -> Self {
        Self {
            distro: distro.into(),
        }
    }

    /// `ATLAS_WSL_DISTRO`, else `Ubuntu-24.04`.
    pub fn from_env() -> Self {
        let distro = std::env::var("ATLAS_WSL_DISTRO")
            .ok()
            .filter(|d| !d.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_DISTRO.to_string());
        Self::new(distro)
    }

    /// Build the `wsl.exe` argv for a command run in `cwd`.
    fn wsl_argv(&self, cwd: &Path, command: &str) -> Vec<String> {
        let wsl_dir = windows_path_to_wsl(cwd);
        let script = format!("cd {} && {}", shell_quote(&wsl_dir), command);
        vec![
            "-d".into(),
            self.distro.clone(),
            "--".into(),
            "bash".into(),
            "-lc".into(),
            script,
        ]
    }
}

/// True when a WSL bridge is usable on this host.
pub fn wsl2_available() -> bool {
    if !cfg!(windows) {
        return false;
    }
    std::process::Command::new("wsl")
        .args(["--status"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// `C:\Users\x` → `/mnt/c/Users/x`. Non-Windows-style paths pass through.
pub fn windows_path_to_wsl(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let bytes = s.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        format!("/mnt/{drive}{}", &s[2..])
    } else {
        s
    }
}

/// Minimal POSIX single-quote escaping for embedding in `bash -lc`.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

impl Sandbox for Wsl2Sandbox {
    fn kind(&self) -> &'static str {
        "wsl2"
    }
    fn exec(&self, cwd: &Path, command: &str, timeout: Duration) -> CommandResult {
        if !cfg!(windows) {
            return CommandResult {
                command: command.to_string(),
                exit_code: 127,
                stdout: String::new(),
                stderr: "wsl2 sandbox is only available on Windows".into(),
            };
        }
        // Run the wsl bridge as the "shell"; the Linux `cd` is inside the
        // `bash -lc` script, so the host process must NOT set `current_dir` to a
        // Linux path (Windows rejects it: os error 267). It inherits the parent
        // Windows cwd, which is always valid.
        let argv = self.wsl_argv(cwd, command);
        run_external("wsl", &argv, command, timeout)
    }
}

/// Spawn `program argv...`, capturing output with the same bounded-wait shape as
/// `run_command` (never panics; timeout → exit 124). `current_dir` is inherited
/// from the parent (safe on Windows, where the caller may pass a Linux path).
fn run_external(program: &str, argv: &[String], label: &str, timeout: Duration) -> CommandResult {
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
    // Reuse run_command's timeout exit code (124) for consistency.
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
    fn windows_path_converts_drive() {
        assert_eq!(
            windows_path_to_wsl(Path::new(r"C:\Users\Max\proj")),
            "/mnt/c/Users/Max/proj"
        );
        assert_eq!(windows_path_to_wsl(Path::new("/home/x")), "/home/x");
    }

    #[test]
    fn argv_has_distro_and_bash() {
        let sb = Wsl2Sandbox::new("Ubuntu-24.04");
        let argv = sb.wsl_argv(Path::new(r"C:\proj"), "echo hi");
        assert_eq!(argv[0], "-d");
        assert_eq!(argv[1], "Ubuntu-24.04");
        assert!(argv.contains(&"bash".to_string()));
        assert!(argv.last().unwrap().contains("echo hi"));
        assert!(argv.last().unwrap().contains("/mnt/c/proj"));
    }

    #[test]
    fn shell_quote_escapes_single_quotes() {
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    }

    /// Live bridge check: only meaningful on a Windows host with WSL. Ignored by
    /// default (CI has no WSL); run with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn wsl2_executes_in_a_distro() {
        if !wsl2_available() {
            eprintln!("skipping: wsl2 not available");
            return;
        }
        let sb = Wsl2Sandbox::from_env();
        assert_eq!(sb.kind(), "wsl2");
        let r = sb.exec(
            Path::new("/tmp"),
            "echo from-wsl && uname -s",
            Duration::from_secs(60),
        );
        assert_eq!(r.exit_code, 0, "stderr: {}", r.stderr);
        assert!(r.stdout.contains("from-wsl"));
        assert!(r.stdout.contains("Linux"));
    }
}
