// Atlas OS — Sandbox runtime trait + backend selection (RFC 63 §4 element 3).
//
// Distinct from `security::sandbox::SandboxLevel` (the *policy* tier). This is
// the *runtime*: WHERE a command runs. The default backend is `Local` (the
// process/container the agent already lives in — which, for a Terminal-Bench
// task, IS an isolated container). `Wsl2` is a lateral backend (external
// process, never bundled — RFC 25 §11): it implements the same trait so the
// agent loop never knows the difference.
//
// The trait carries the file + snapshot ops the RFC specifies so higher layers
// (artifact verification, re-observation, rollback) can go through ONE runtime
// abstraction instead of reaching for `std::fs` directly.

pub mod local;
pub mod wsl2;

use std::path::Path;
use std::time::Duration;

pub use local::LocalSandbox;
pub use wsl2::Wsl2Sandbox;

use crate::orchestrator::agent::CommandResult;

/// Where the agent's commands and file ops execute.
pub trait Sandbox: Send + Sync {
    /// Stable id surfaced in the journal (`local`, `wsl2`).
    fn kind(&self) -> &'static str;
    /// Run `command` with `cwd`, bounded by `timeout`. Never panics.
    fn exec(&self, cwd: &Path, command: &str, timeout: Duration) -> CommandResult;
    /// Read a file's bytes. Default: the local filesystem.
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))
    }
    /// Write bytes to a file (creating parents). Default: the local filesystem.
    fn write_file(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
        std::fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))
    }
    /// A content-addressed snapshot of `root`: `(relative_path, sha256)` sorted
    /// by path, so callers can diff two snapshots (rollback/evidence).
    fn snapshot(&self, root: &Path) -> Result<Vec<(String, String)>, String> {
        snapshot_dir(root)
    }
}

/// Shared directory snapshot: walk `root`, hash every regular file. Bounded to
/// `MAX_SNAPSHOT_FILES` so a huge tree cannot blow memory; the walk is iterative.
pub const MAX_SNAPSHOT_FILES: usize = 10_000;

pub fn snapshot_dir(root: &Path) -> Result<Vec<(String, String)>, String> {
    use sha2::{Digest, Sha256};
    let mut out: Vec<(String, String)> = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            std::fs::read_dir(&dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if ft.is_dir() {
                // Skip VCS internals — noisy and never part of an artifact.
                if path.file_name().map(|n| n == ".git").unwrap_or(false) {
                    continue;
                }
                stack.push(path);
            } else if ft.is_file() {
                if out.len() >= MAX_SNAPSHOT_FILES {
                    return Err(format!("snapshot exceeded {MAX_SNAPSHOT_FILES} files"));
                }
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes = std::fs::read(&path).unwrap_or_default();
                let mut h = Sha256::new();
                h.update(&bytes);
                out.push((rel, hex::encode(h.finalize())));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Select a sandbox from `ATLAS_SANDBOX` (default `local`). `wsl2` uses the real
/// WSL bridge when `wsl.exe` is present; otherwise it falls back to `Local` so a
/// task never breaks because an optional backend is unavailable.
pub fn resolve_sandbox() -> Box<dyn Sandbox> {
    match std::env::var("ATLAS_SANDBOX").ok().as_deref() {
        Some("wsl2") => {
            if wsl2::wsl2_available() {
                Box::new(Wsl2Sandbox::from_env())
            } else {
                Box::new(LocalSandbox)
            }
        }
        // `daytona`/`e2b` remain lateral; until their feature lands the task
        // container itself is the sandbox.
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
        let sb = resolve_sandbox();
        assert_eq!(sb.kind(), "local");
    }

    #[test]
    fn snapshot_hashes_files_sorted() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("b.txt"), "b").unwrap();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        let snap = LocalSandbox.snapshot(dir.path()).unwrap();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].0, "a.txt");
        assert_eq!(snap[1].0, "b.txt");
        assert_eq!(snap[0].1.len(), 64);
    }

    #[test]
    fn read_write_round_trip() {
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path().join("sub/x.txt");
        LocalSandbox.write_file(&p, b"hi").unwrap();
        assert_eq!(LocalSandbox.read_file(&p).unwrap(), b"hi");
    }
}
