// Atlas OS — Swarm worktree isolation (RFC 05 §4, Phase 4 sub-fase 4.0).
// One real git worktree per agent under `~/.opencode/worktrees/<mission>/`
// (Conductor CN-001 port, research/31 §A.3). Driven via the `git` CLI
// through `std::process::Command` — no `git2` dependency (RFC 25 §11
// single-binary audit). Every operation fail-safes to a typed
// `WorktreeError` when git is missing or the repo is unusable; callers
// fall back to in-place execution.

use std::path::{Path, PathBuf};
// (git spawns go through `crate::gitcmd::git()`, which pins an absolute path)

use anyhow::Context;

pub struct WorktreeManager {
    repo_root: PathBuf,
    base_dir: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorktreeEntry {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub detached: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum WorktreeError {
    #[error("git not found on PATH — install git to use swarm worktrees")]
    GitMissing,
    #[error("not a git repository: {0}")]
    NotARepo(String),
    #[error("worktree already exists: {0}")]
    AlreadyExists(String),
    #[error("worktree not found: {0}")]
    NotFound(String),
    #[error("invalid mission/agent name: {0}")]
    InvalidName(String),
    #[error("git worktree add failed: {0}")]
    AddFailed(String),
    #[error("git worktree remove failed: {0}")]
    RemoveFailed(String),
    #[error("git worktree list failed: {0}")]
    ListFailed(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn validate_name(what: &str, value: &str) -> Result<(), WorktreeError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains('\0')
        || value.split('/').any(|p| p == "..")
    {
        return Err(WorktreeError::InvalidName(format!("{what} {value:?}")));
    }
    Ok(())
}

fn git_stderr_detail(stderr: &[u8]) -> String {
    let s = String::from_utf8_lossy(stderr);
    let t = s.trim();
    if t.is_empty() {
        return "git exited with an error status".into();
    }
    if t.len() > 300 {
        t[..300].to_string()
    } else {
        t.to_string()
    }
}

impl WorktreeManager {
    pub fn new(repo_root: PathBuf, base_dir: PathBuf) -> Self {
        Self {
            repo_root,
            base_dir,
        }
    }

    pub fn with_default_base(repo_root: &Path) -> anyhow::Result<Self> {
        let home = dirs::home_dir().context("could not resolve user home directory")?;
        Ok(Self::new(
            repo_root.to_path_buf(),
            home.join(".opencode").join("worktrees"),
        ))
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn path_for(&self, mission_id: &str, agent_id: &str) -> Result<PathBuf, WorktreeError> {
        validate_name("mission", mission_id)?;
        validate_name("agent", agent_id)?;
        Ok(self.base_dir.join(mission_id).join(agent_id))
    }

    pub fn git_available() -> bool {
        crate::gitcmd::git()
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn ensure_git(&self) -> Result<(), WorktreeError> {
        if Self::git_available() {
            Ok(())
        } else {
            Err(WorktreeError::GitMissing)
        }
    }

    pub fn add(
        &self,
        mission_id: &str,
        agent_id: &str,
        branch: Option<&str>,
    ) -> Result<PathBuf, WorktreeError> {
        let path = self.path_for(mission_id, agent_id)?;
        if path.exists() {
            return Err(WorktreeError::AlreadyExists(path.display().to_string()));
        }
        self.ensure_git()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut cmd = crate::gitcmd::git();
        cmd.arg("-C")
            .arg(&self.repo_root)
            .arg("worktree")
            .arg("add");
        if let Some(b) = branch {
            cmd.arg("-b").arg(b);
        } else {
            cmd.arg("--detach");
        }
        let out = cmd.arg(&path).output()?;
        if !out.status.success() {
            let detail = git_stderr_detail(&out.stderr);
            if detail.contains("not a git repository") {
                return Err(WorktreeError::NotARepo(
                    self.repo_root.display().to_string(),
                ));
            }
            return Err(WorktreeError::AddFailed(detail));
        }
        Ok(path)
    }

    pub fn remove(&self, mission_id: &str, agent_id: &str) -> Result<(), WorktreeError> {
        let path = self.path_for(mission_id, agent_id)?;
        if !path.exists() {
            return Err(WorktreeError::NotFound(path.display().to_string()));
        }
        self.ensure_git()?;
        let out = crate::gitcmd::git()
            .arg("-C")
            .arg(&self.repo_root)
            .arg("worktree")
            .arg("remove")
            .arg("--force")
            .arg(&path)
            .output()?;
        if !out.status.success() {
            return Err(WorktreeError::RemoveFailed(git_stderr_detail(&out.stderr)));
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<WorktreeEntry>, WorktreeError> {
        self.ensure_git()?;
        let out = crate::gitcmd::git()
            .arg("-C")
            .arg(&self.repo_root)
            .arg("worktree")
            .arg("list")
            .arg("--porcelain")
            .output()?;
        if !out.status.success() {
            let detail = git_stderr_detail(&out.stderr);
            if detail.contains("not a git repository") {
                return Err(WorktreeError::NotARepo(
                    self.repo_root.display().to_string(),
                ));
            }
            return Err(WorktreeError::ListFailed(detail));
        }
        Ok(parse_porcelain(&String::from_utf8_lossy(&out.stdout)))
    }
}

fn parse_porcelain(stdout: &str) -> Vec<WorktreeEntry> {
    let mut out = Vec::new();
    let mut path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;
    let mut detached = false;
    let mut flush =
        |path: &mut Option<PathBuf>, branch: &mut Option<String>, detached: &mut bool| {
            if let Some(p) = path.take() {
                out.push(WorktreeEntry {
                    path: p,
                    branch: branch.take(),
                    detached: std::mem::take(detached),
                });
            }
        };
    for line in stdout.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            flush(&mut path, &mut branch, &mut detached);
            path = Some(PathBuf::from(p));
        } else if let Some(b) = line.strip_prefix("branch ") {
            branch = Some(b.strip_prefix("refs/heads/").unwrap_or(b).to_string());
        } else if line == "detached" {
            detached = true;
        } else if line.is_empty() {
            flush(&mut path, &mut branch, &mut detached);
        }
    }
    flush(&mut path, &mut branch, &mut detached);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager_in(tmp: &Path) -> WorktreeManager {
        WorktreeManager::new(tmp.to_path_buf(), tmp.join("worktrees"))
    }

    fn git(args: &[&str], dir: &Path) {
        let status = crate::gitcmd::git()
            .args(args)
            .current_dir(dir)
            .output()
            .expect("spawn git")
            .status;
        assert!(status.success(), "git {args:?} failed");
    }

    fn init_repo(dir: &Path) {
        git(&["init"], dir);
        git(&["config", "user.email", "atlas@test.invalid"], dir);
        git(&["config", "user.name", "atlas-test"], dir);
        git(&["commit", "--allow-empty", "-m", "init"], dir);
    }

    #[test]
    fn path_for_joins_base_mission_agent() {
        let m = WorktreeManager::new(
            PathBuf::from("/repo"),
            PathBuf::from("/home/u/.opencode/worktrees"),
        );
        let p = m.path_for("m1", "backend").unwrap();
        assert_eq!(p, PathBuf::from("/home/u/.opencode/worktrees/m1/backend"));
    }

    #[test]
    fn path_for_rejects_traversal_names() {
        let m = manager_in(Path::new("/tmp"));
        assert!(m.path_for("..", "backend").is_err());
        assert!(m.path_for("m1", "../evil").is_err());
        assert!(m.path_for("m/1", "backend").is_err());
        assert!(m.path_for("", "backend").is_err());
        assert!(m.path_for("m1", "").is_err());
    }

    #[test]
    fn remove_missing_worktree_returns_not_found_without_git() {
        let tmp = tempfile::TempDir::new().unwrap();
        let m = manager_in(tmp.path());
        let err = m.remove("nope", "ghost").unwrap_err();
        assert!(matches!(err, WorktreeError::NotFound(_)));
    }

    #[test]
    fn add_existing_dir_returns_already_exists_without_git() {
        let tmp = tempfile::TempDir::new().unwrap();
        let m = manager_in(tmp.path());
        let p = m.path_for("m1", "backend").unwrap();
        std::fs::create_dir_all(&p).unwrap();
        let err = m.add("m1", "backend", None).unwrap_err();
        assert!(matches!(err, WorktreeError::AlreadyExists(_)));
    }

    #[test]
    fn parse_porcelain_reads_branches_and_detached() {
        let sample = "worktree /repo\nbranch refs/heads/main\n\nworktree /wt/a\ndetached\n\nworktree /wt/b\nbranch refs/heads/feat-x\n";
        let entries = parse_porcelain(sample);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].branch.as_deref(), Some("main"));
        assert!(!entries[0].detached);
        assert!(entries[1].detached);
        assert_eq!(entries[2].branch.as_deref(), Some("feat-x"));
    }

    #[test]
    fn worktree_add_list_remove_happy_path() {
        if !WorktreeManager::git_available() {
            eprintln!("skipping — git not on PATH");
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        init_repo(tmp.path());
        let m = manager_in(tmp.path());

        let path = m.add("m1", "backend", None).expect("add");
        assert!(path.is_dir());

        let entries = m.list().expect("list");
        assert!(entries.iter().any(|e| e.path == path));

        m.remove("m1", "backend").expect("remove");
        assert!(!path.exists());
    }

    #[test]
    fn worktree_double_add_is_already_exists() {
        if !WorktreeManager::git_available() {
            eprintln!("skipping — git not on PATH");
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        init_repo(tmp.path());
        let m = manager_in(tmp.path());

        m.add("m1", "backend", None).expect("first add");
        let err = m.add("m1", "backend", None).unwrap_err();
        assert!(matches!(err, WorktreeError::AlreadyExists(_)));
        m.remove("m1", "backend").expect("cleanup");
    }

    #[test]
    fn list_outside_repo_is_not_a_repo() {
        if !WorktreeManager::git_available() {
            eprintln!("skipping — git not on PATH");
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        let m = manager_in(tmp.path());
        let err = m.list().unwrap_err();
        assert!(matches!(err, WorktreeError::NotARepo(_)));
    }
}
