// Atlas OS — `GET /hud/worktrees` (RFC 65 §3, Worktrees view; RFC 05 §4).
//
// Lists the git worktrees of a repository via `WorktreeManager::list`. The repo
// defaults to the process cwd but can be pointed with `?repo=`. Read-only and
// fail-safe: when git is missing or the path is not a repo, `ok:false` plus a
// human reason is returned (never a silent empty list).

use std::path::PathBuf;

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::swarm::worktrees::WorktreeManager;

#[derive(Debug, Deserialize)]
pub struct WorktreesQuery {
    /// Repository root. Defaults to the process cwd.
    pub repo: Option<String>,
}

fn error_body(repo: &str, reason: &str) -> Value {
    json!({ "repo": repo, "ok": false, "reason": reason, "entries": [] })
}

pub async fn get_worktrees(Query(q): Query<WorktreesQuery>) -> Json<Value> {
    let repo = match q.repo {
        Some(r) => PathBuf::from(r),
        None => match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => return Json(error_body("", &format!("cwd unavailable: {e}"))),
        },
    };
    let repo_display = repo.display().to_string();
    let mgr = match WorktreeManager::with_default_base(&repo) {
        Ok(m) => m,
        Err(e) => return Json(error_body(&repo_display, &e.to_string())),
    };
    match mgr.list() {
        Ok(entries) => {
            let items: Vec<Value> = entries
                .iter()
                .map(|e| {
                    json!({
                        "path": e.path.display().to_string(),
                        "branch": e.branch,
                        "detached": e.detached,
                    })
                })
                .collect();
            Json(json!({ "repo": repo_display, "ok": true, "reason": null, "entries": items }))
        }
        Err(e) => Json(error_body(&repo_display, &e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn non_repo_path_reports_ok_false_with_reason() {
        let dir = tempfile::TempDir::new().unwrap();
        let Json(v) = get_worktrees(Query(WorktreesQuery {
            repo: Some(dir.path().display().to_string()),
        }))
        .await;
        // A fresh temp dir is never a git repo: either GitMissing or NotARepo.
        assert_eq!(v["ok"], false);
        assert!(v["reason"].as_str().is_some_and(|s| !s.is_empty()));
        assert!(v["entries"].as_array().unwrap().is_empty());
    }
}
