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

use crate::mcp::config::{McpRegistry, McpServerConfig};

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

    // Prefer the opencode file the operator already uses; fall back to the RFC 07
    // `mcp.json`. Missing = honest `ok:false`, never a fabricated list.
    let candidates = [
        repo.join(".opencode").join("mcp.json"),
        repo.join("mcp.json"),
    ];
    let existing: Vec<PathBuf> = candidates.iter().filter(|p| p.is_file()).cloned().collect();
    if existing.is_empty() {
        return Json(missing(
            &repo_display,
            &format!("{}: not found", candidates[0].display()),
        ));
    }
    let path = existing[0].clone();
    let registry = match McpRegistry::load_from(&existing) {
        Ok(r) => r,
        Err(e) => return Json(missing(&repo_display, &format!("invalid registry: {e}"))),
    };

    let servers: Vec<Value> = registry
        .servers
        .iter()
        .map(|(name, cfg)| server_json(name, cfg))
        .collect();

    Json(json!({
        "repo": repo_display,
        "ok": true,
        "reason": Value::Null,
        "path": path.display().to_string(),
        "servers": servers,
    }))
}

/// One server, enriched with the RFC 07 §2/§3/§4 policy the runtime enforces
/// (sandbox, supply chain, allowlist), so the HUD is the honest read side.
fn server_json(name: &str, cfg: &McpServerConfig) -> Value {
    let sandbox = cfg.effective_sandbox();
    json!({
        "name": name,
        "type": cfg.transport,
        "transport": cfg.transport,
        "enabled": cfg.enabled,
        "command": cfg.argv(),
        "allowed_tools": cfg.allowed_tools,
        "exposes_tools": cfg.exposes_any(),
        "timeout_ms": cfg.timeout_ms,
        "sandbox": {
            "declared": cfg.sandbox.as_str(),
            "effective": sandbox.as_str(),
            "enforced": cfg.is_sandbox_enforced(),
            "finding": cfg.sandbox_finding(),
        },
        "package": cfg.package_spec(),
        "supply": cfg.supply_report().map(|r| json!({
            "package": r.package,
            "verdict": r.verdict.as_str(),
            "reasons": r.reasons,
        })),
    })
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

    #[tokio::test]
    async fn enriches_servers_with_policy_and_supply() {
        let dir = tempfile::TempDir::new().unwrap();
        let opencode = dir.path().join(".opencode");
        std::fs::create_dir_all(&opencode).unwrap();
        std::fs::write(
            opencode.join("mcp.json"),
            r#"{"mcp":{"context7":{"type":"stdio","command":["pnpm","dlx","@upstash/context7-mcp@3.2.4"],"enabled":true,"allowed_tools":["resolve-library-id"]}}}"#,
        )
        .unwrap();
        let Json(v) = get_mcp(Query(McpQuery {
            repo: Some(dir.path().display().to_string()),
        }))
        .await;
        let s = &v["servers"][0];
        assert_eq!(s["allowed_tools"][0], "resolve-library-id");
        assert_eq!(s["exposes_tools"], true);
        // RFC 07 §2: unsigned → container, not enforced yet (Fase 29.1).
        assert_eq!(s["sandbox"]["effective"], "container");
        assert_eq!(s["sandbox"]["enforced"], false);
        assert!(s["sandbox"]["finding"].as_str().is_some());
        // RFC 07 §3: package extracted + a supply verdict.
        assert_eq!(s["package"], "@upstash/context7-mcp");
        assert_eq!(s["supply"]["package"], "@upstash/context7-mcp");
        assert!(s["supply"]["verdict"].as_str().is_some());
    }
}
