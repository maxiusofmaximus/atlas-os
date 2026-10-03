// Atlas OS — shared `git` process launcher.
//
// Resolves the `git` executable to an absolute path **once** and caches it.
// On Windows, `Command::new("git")` performs a PATH search inside
// `CreateProcess`, which can transiently fail with `NotFound` ("program not
// found") under high process-creation concurrency — a flaky failure unrelated
// to the code under test that made the swarm worktree/rebase tests
// intermittently red. Pinning the resolved absolute path removes that race.
// Falls back to the bare name when the lookup fails, so behaviour is unchanged
// when git is genuinely absent (the caller's `git_available()` path handles it).

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

fn resolved_git() -> Option<PathBuf> {
    static CACHE: OnceLock<Option<PathBuf>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let exe = if cfg!(windows) { "git.exe" } else { "git" };
            let path = std::env::var_os("PATH")?;
            std::env::split_paths(&path)
                .map(|dir| dir.join(exe))
                .find(|cand| cand.is_file())
        })
        .clone()
}

/// A `git` [`Command`] pinned to the resolved absolute path when available.
pub fn git() -> Command {
    match resolved_git() {
        Some(path) => Command::new(path),
        None => Command::new("git"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_launcher_resolves_and_runs_version() {
        match git().arg("--version").output() {
            Ok(out) => assert!(out.status.success()),
            // git genuinely absent on PATH is acceptable; a flaky NotFound under
            // load is not (that is the bug this module fixes).
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => panic!("unexpected git spawn error: {e}"),
        }
    }
}
