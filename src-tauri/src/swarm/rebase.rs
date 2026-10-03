// Atlas OS — Swarm auto-rebase post-merge (RFC 05 §4, Phase 4 sub-fase 4.4).
// Conductor CN-003 port (research/28 §A.4, research/31 §A.3): after the merger
// accepts the merge, every live workspace rebases onto the updated main
// (`git fetch` + `git rebase` via the `git` CLI through
// `std::process::Command` — no `git2` dependency, RFC 25 §11 single-binary
// audit, same fail-safe pattern as `swarm::worktrees::WorktreeManager`).
// Conflicts fail safe to manual resolution via `RebaseError::Conflict`.

use std::path::{Path, PathBuf};
// (git spawns go through `crate::gitcmd::git()`, which pins an absolute path)

use crate::swarm::worktrees::WorktreeManager;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RebaseStatus {
    UpToDate,
    Rebased,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebaseOutcome {
    pub path: PathBuf,
    pub upstream: String,
    pub status: RebaseStatus,
}

#[derive(Debug)]
pub struct RebaseResult {
    pub path: PathBuf,
    pub result: Result<RebaseOutcome, RebaseError>,
}

#[derive(Debug, thiserror::Error)]
pub enum RebaseError {
    #[error("git not found on PATH — install git to use swarm auto-rebase")]
    GitMissing,
    #[error("not a git repository: {0}")]
    NotARepo(String),
    #[error("not a swarm worktree: {0}")]
    NotAWorktree(String),
    #[error("invalid upstream branch: {0}")]
    InvalidUpstream(String),
    #[error("git fetch failed: {0}")]
    FetchFailed(String),
    #[error("rebase conflict — resolve manually in {0}: {1} (abort with `git rebase --abort`)")]
    Conflict(String, String),
    #[error("git rebase failed: {0}")]
    RebaseFailed(String),
    #[error("git rebase --abort failed: {0}")]
    AbortFailed(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn validate_upstream(upstream: &str) -> Result<(), RebaseError> {
    let bad = upstream.is_empty()
        || upstream.contains('\0')
        || upstream.contains(' ')
        || upstream.contains("..")
        || upstream.contains('~')
        || upstream.contains('^')
        || upstream.contains(':')
        || upstream.contains('?')
        || upstream.contains('*')
        || upstream.contains('[')
        || upstream.contains('\\');
    if bad {
        return Err(RebaseError::InvalidUpstream(upstream.to_string()));
    }
    Ok(())
}

fn git_output_detail(stdout: &[u8], stderr: &[u8]) -> String {
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    );
    let t = combined.trim();
    if t.is_empty() {
        return "git exited with an error status".into();
    }
    if t.len() > 500 {
        t[..500].to_string()
    } else {
        t.to_string()
    }
}

fn is_no_remote_detail(lower: &str) -> bool {
    lower.contains("no remote")
        || lower.contains("no such remote")
        || lower.contains("does not have any remotes")
        || lower.contains("couldn't find remote")
        || lower.contains("could not find remote")
}

fn is_conflict_detail(lower: &str) -> bool {
    lower.contains("conflict")
        || lower.contains("could not apply")
        || lower.contains("needs merge")
        || lower.contains("unmerged")
        || lower.contains("resolve all conflicts")
        || lower.contains("already in progress")
        || lower.contains("rebase in progress")
}

fn is_not_a_repo_detail(lower: &str) -> bool {
    lower.contains("not a git repository")
}

fn is_unknown_revision_detail(lower: &str) -> bool {
    lower.contains("unknown revision")
        || lower.contains("invalid upstream")
        || lower.contains("no such branch")
}

pub fn rebase_worktree(worktree: &Path, upstream: &str) -> Result<RebaseOutcome, RebaseError> {
    validate_upstream(upstream)?;
    if !worktree.is_dir() {
        return Err(RebaseError::NotAWorktree(worktree.display().to_string()));
    }
    if !WorktreeManager::git_available() {
        return Err(RebaseError::GitMissing);
    }
    let fetch = crate::gitcmd::git()
        .arg("-C")
        .arg(worktree)
        .arg("fetch")
        .output()?;
    if !fetch.status.success() {
        let detail = git_output_detail(&fetch.stdout, &fetch.stderr);
        if is_not_a_repo_detail(&detail.to_lowercase()) {
            return Err(RebaseError::NotARepo(worktree.display().to_string()));
        }
        if !is_no_remote_detail(&detail.to_lowercase()) {
            return Err(RebaseError::FetchFailed(detail));
        }
    }
    let out = crate::gitcmd::git()
        .arg("-C")
        .arg(worktree)
        .arg("rebase")
        .arg(upstream)
        .output()?;
    let detail = git_output_detail(&out.stdout, &out.stderr);
    let lower = detail.to_lowercase();
    if out.status.success() {
        let status = if lower.contains("up to date") || lower.contains("up-to-date") {
            RebaseStatus::UpToDate
        } else {
            RebaseStatus::Rebased
        };
        return Ok(RebaseOutcome {
            path: worktree.to_path_buf(),
            upstream: upstream.to_string(),
            status,
        });
    }
    if is_not_a_repo_detail(&lower) {
        return Err(RebaseError::NotARepo(worktree.display().to_string()));
    }
    if is_conflict_detail(&lower) {
        return Err(RebaseError::Conflict(
            worktree.display().to_string(),
            detail,
        ));
    }
    if is_unknown_revision_detail(&lower) {
        return Err(RebaseError::InvalidUpstream(upstream.to_string()));
    }
    Err(RebaseError::RebaseFailed(detail))
}

pub fn abort_rebase(worktree: &Path) -> Result<(), RebaseError> {
    if !worktree.is_dir() {
        return Err(RebaseError::NotAWorktree(worktree.display().to_string()));
    }
    if !WorktreeManager::git_available() {
        return Err(RebaseError::GitMissing);
    }
    let out = crate::gitcmd::git()
        .arg("-C")
        .arg(worktree)
        .arg("rebase")
        .arg("--abort")
        .output()?;
    if !out.status.success() {
        return Err(RebaseError::AbortFailed(git_output_detail(
            &out.stdout,
            &out.stderr,
        )));
    }
    Ok(())
}

pub fn rebase_many(paths: &[PathBuf], upstream: &str) -> Vec<RebaseResult> {
    paths
        .iter()
        .map(|p| {
            let result = rebase_worktree(p, upstream);
            RebaseResult {
                path: p.clone(),
                result,
            }
        })
        .collect()
}

pub fn rebase_after_merge(
    manager: &WorktreeManager,
    upstream: &str,
) -> Result<Vec<RebaseResult>, RebaseError> {
    validate_upstream(upstream)?;
    if !WorktreeManager::git_available() {
        return Err(RebaseError::GitMissing);
    }
    let entries = manager.list().map_err(|e| match e {
        crate::swarm::worktrees::WorktreeError::GitMissing => RebaseError::GitMissing,
        crate::swarm::worktrees::WorktreeError::NotARepo(p) => RebaseError::NotARepo(p),
        crate::swarm::worktrees::WorktreeError::ListFailed(d) => RebaseError::RebaseFailed(d),
        other => RebaseError::RebaseFailed(other.to_string()),
    })?;
    Ok(entries
        .into_iter()
        .map(|e| {
            let result = rebase_worktree(&e.path, upstream);
            RebaseResult {
                path: e.path,
                result,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    // (git spawns go through `crate::gitcmd::git()`, which pins an absolute path)

    fn git(args: &[&str], dir: &Path) {
        let out = crate::gitcmd::git()
            .args(args)
            .current_dir(dir)
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn git_branch(dir: &Path) -> String {
        let out = crate::gitcmd::git()
            .args(["branch", "--show-current"])
            .current_dir(dir)
            .output()
            .expect("spawn git");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    fn init_repo(dir: &Path) -> String {
        git(&["init"], dir);
        git(&["config", "user.email", "atlas@test.invalid"], dir);
        git(&["config", "user.name", "atlas-test"], dir);
        write(dir, "file.txt", "base\n");
        git(&["add", "."], dir);
        git(&["commit", "-m", "init"], dir);
        let branch = git_branch(dir);
        if branch != "main" {
            git(&["branch", "-M", "main"], dir);
        }
        "main".to_string()
    }

    #[test]
    fn invalid_upstream_rejected_without_touching_git() {
        for bad in [
            "",
            " ",
            "main..other",
            "a b",
            "a~1",
            "a^",
            "a:b",
            "a?b",
            "a*b",
            "a[b",
        ] {
            let err = rebase_worktree(Path::new("/tmp"), bad).unwrap_err();
            assert!(
                matches!(err, RebaseError::InvalidUpstream(_)),
                "upstream {bad:?} gave {err:?}"
            );
        }
    }

    #[test]
    fn missing_worktree_dir_is_not_a_worktree() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ghost = tmp.path().join("ghost-wt");
        let err = rebase_worktree(&ghost, "main").unwrap_err();
        assert!(matches!(err, RebaseError::NotAWorktree(_)));
        let err = abort_rebase(&ghost).unwrap_err();
        assert!(matches!(err, RebaseError::NotAWorktree(_)));
    }

    #[test]
    fn rebase_clean_divergence_rebases_then_reports_up_to_date() {
        if !WorktreeManager::git_available() {
            eprintln!("skipping — git not on PATH");
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        let main = init_repo(tmp.path());
        git(&["checkout", "-b", "feature"], tmp.path());
        write(tmp.path(), "feature.txt", "work\n");
        git(&["add", "."], tmp.path());
        git(&["commit", "-m", "feature work"], tmp.path());
        git(&["checkout", &main], tmp.path());
        write(tmp.path(), "main.txt", "other\n");
        git(&["add", "."], tmp.path());
        git(&["commit", "-m", "main moves"], tmp.path());
        git(&["checkout", "feature"], tmp.path());

        let outcome = rebase_worktree(tmp.path(), &main).expect("clean rebase");
        assert_eq!(outcome.upstream, main);
        assert_eq!(outcome.status, RebaseStatus::Rebased);

        let again = rebase_worktree(tmp.path(), &main).expect("second rebase");
        assert_eq!(again.status, RebaseStatus::UpToDate);
    }

    #[test]
    fn rebase_conflict_fails_safe_to_manual_and_abort_recovers() {
        if !WorktreeManager::git_available() {
            eprintln!("skipping — git not on PATH");
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        let main = init_repo(tmp.path());
        git(&["checkout", "-b", "feature"], tmp.path());
        write(tmp.path(), "file.txt", "feature side\n");
        git(&["add", "."], tmp.path());
        git(&["commit", "-m", "feature edit"], tmp.path());
        git(&["checkout", &main], tmp.path());
        write(tmp.path(), "file.txt", "main side\n");
        git(&["add", "."], tmp.path());
        git(&["commit", "-m", "main edit"], tmp.path());
        git(&["checkout", "feature"], tmp.path());

        let err = rebase_worktree(tmp.path(), &main).unwrap_err();
        assert!(
            matches!(err, RebaseError::Conflict(_, _)),
            "expected Conflict, got {err:?}"
        );
        abort_rebase(tmp.path()).expect("abort restores pre-rebase state");
    }

    #[test]
    fn rebase_many_collects_per_path_without_short_circuit() {
        if !WorktreeManager::git_available() {
            eprintln!("skipping — git not on PATH");
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        let main = init_repo(tmp.path());
        let ghost = tmp.path().join("ghost-wt");
        let results = rebase_many(&[tmp.path().to_path_buf(), ghost.clone()], &main);
        assert_eq!(results.len(), 2);
        assert!(results[0].result.is_ok());
        assert!(matches!(
            results[1].result,
            Err(RebaseError::NotAWorktree(_))
        ));
        assert_eq!(results[1].path, ghost);
    }

    #[test]
    fn rebase_after_merge_discovers_live_worktrees_via_manager() {
        if !WorktreeManager::git_available() {
            eprintln!("skipping — git not on PATH");
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        init_repo(tmp.path());
        let manager = WorktreeManager::new(tmp.path().to_path_buf(), tmp.path().join("wts"));
        let wt = manager.add("m1", "backend", None).expect("add worktree");
        let results = rebase_after_merge(&manager, "main").expect("rebase-after-merge");
        assert!(results.iter().any(|r| r.path == wt));
        assert!(results.iter().all(|r| r.result.is_ok()));
        manager.remove("m1", "backend").expect("cleanup");
    }

    #[test]
    fn rebase_after_merge_rejects_bad_upstream_before_listing() {
        let manager = WorktreeManager::new(
            PathBuf::from("/repo"),
            PathBuf::from("/home/u/.opencode/worktrees"),
        );
        let err = rebase_after_merge(&manager, "").unwrap_err();
        assert!(matches!(err, RebaseError::InvalidUpstream(_)));
    }
}
