// Atlas OS — `GET /hud/mcp` (RFC 65 §10, Skill + MCP rail).
//
// Reads the local MCP registry the operator already uses (`.opencode/mcp.json`,
// the opencode-compatible format documented by `mobile::mcp_template`:
// `{ "$schema", "mcp": { <name>: { type, command, enabled, timeout } } }`) and
// returns the declared servers. Read-only and fail-safe: a missing or malformed
// file yields `ok:false` + reason, never a fabricated list.
//
// Atlas's own hot-swap MCP runtime (RFC 07 / RFC 24 §8) is not implemented; this
// endpoint is the honest read side of the catalog.

use std::path::PathBuf;

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct McpQuery {
    /// Repository root holding `.opencode/mcp.json`. Defaults to the process cwd.
    pub repo: Option<String>,
}

fn missing(repo: &str, reason: &str) -> Value {
    json!({ "repo": repo, "ok": false, "reason": reason, "servers": [] })
}

pub async fn get_mcp(Query(q): Query<McpQuery>) -> Json<Value> {
    let repo = match q.repo {
        Some(r) => PathBuf::from(r),
        None => match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => return Json(missing("", &format!("cwd unavailable: {e}"))),
        },
    };
    let repo_display = repo.display().to_string();
    let path = repo.join(".opencode").join("mcp.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => return Json(missing(&repo_display, &format!("{}: {e}", path.display()))),
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return Json(missing(&repo_display, &format!("invalid JSON: {e}"))),
    };
    let servers: Vec<Value> = value
        .get("mcp")
        .and_then(Value::as_object)
        .map(|obj| {
            obj.iter()
                .map(|(name, cfg)| {
                    json!({
                        "name": name,
                        "type": cfg.get("type").and_then(Value::as_str),
                        "enabled": cfg.get("enabled").and_then(Value::as_bool).unwrap_or(false),
                        "command": cfg.get("command").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Json(json!({
        "repo": repo_display,
        "ok": true,
        "reason": Value::Null,
        "path": path.display().to_string(),
        "servers": servers,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_config_reports_ok_false() {
        let dir = tempfile::TempDir::new().unwrap();
        let Json(v) = get_mcp(Query(McpQuery {
            repo: Some(dir.path().display().to_string()),
        }))
        .await;
        assert_eq!(v["ok"], false);
        assert!(v["reason"].as_str().is_some_and(|s| !s.is_empty()));
    }

    #[tokio::test]
    async fn parses_declared_servers() {
        let dir = tempfile::TempDir::new().unwrap();
        let opencode = dir.path().join(".opencode");
        std::fs::create_dir_all(&opencode).unwrap();
        std::fs::write(
            opencode.join("mcp.json"),
            r#"{"mcp":{"context7":{"type":"stdio","command":["npx","ctx7"],"enabled":true}}}"#,
        )
        .unwrap();
        let Json(v) = get_mcp(Query(McpQuery {
            repo: Some(dir.path().display().to_string()),
        }))
        .await;
        assert_eq!(v["ok"], true);
        assert_eq!(v["servers"][0]["name"], "context7");
        assert_eq!(v["servers"][0]["enabled"], true);
    }
}
