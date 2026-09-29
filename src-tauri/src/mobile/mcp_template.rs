use serde_json::{Map, Value};

pub const ARTEMIS_MCP_SERVER_ID: &str = "artemis";
pub const ARTEMIS_MCP_MODULE: &str = "mcp_server";
pub const ARTEMIS_MCP_TIMEOUT_MS: u64 = 60_000;
pub const ARTEMIS_REPO_PLACEHOLDER: &str = "<PATH-TO-ARTEMIS-CLONE>";

pub const ARTEMIS_MCP_TOOLS: [&str; 5] = [
    "mobile_run_task",
    "mobile_manage_task",
    "mobile_get_device_state",
    "mobile_inspect_trace",
    "mobile_diagnose",
];

pub fn artemis_tools() -> [&'static str; 5] {
    ARTEMIS_MCP_TOOLS
}

pub fn default_repo_dir() -> String {
    std::env::var(crate::mobile::ARTEMIS_REPO_ENV)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| ARTEMIS_REPO_PLACEHOLDER.to_string())
}

pub fn opencode_command(repo_dir: &str) -> Vec<String> {
    vec![
        "uv".to_string(),
        "--directory".to_string(),
        repo_dir.trim().to_string(),
        "run".to_string(),
        crate::mobile::ARTEMIS_BIN.to_string(),
        "mcp".to_string(),
    ]
}

pub fn render_full_file(repo_dir: &str) -> String {
    let repo = normalize_repo(repo_dir);
    let mut artemis = Map::new();
    artemis.insert("type".to_string(), Value::String("stdio".to_string()));
    artemis.insert(
        "command".to_string(),
        Value::Array(
            opencode_command(&repo)
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    artemis.insert("enabled".to_string(), Value::Bool(true));
    artemis.insert(
        "timeout".to_string(),
        Value::Number(ARTEMIS_MCP_TIMEOUT_MS.into()),
    );
    let mut mcp = Map::new();
    mcp.insert(ARTEMIS_MCP_SERVER_ID.to_string(), Value::Object(artemis));
    let mut root = Map::new();
    root.insert(
        "$schema".to_string(),
        Value::String("https://opencode.ai/mcp.json".to_string()),
    );
    root.insert("mcp".to_string(), Value::Object(mcp));
    serde_json::to_string_pretty(&Value::Object(root)).unwrap_or_default()
}

pub fn render_guide(repo_dir: &str) -> String {
    let repo = normalize_repo(repo_dir);
    format!(
        "artemis MCP wiring (lateral, RFC 38 §2.1 — 5 tools tipados):\n\
         tools: {tools}\n\
         server (stdio): `uv --directory \"{repo}\" run artemis mcp` (= `uv run artemis mcp`, mcp_server/README).\n\
         direct alt: `<venv-python> -m {module}` with cwd `{repo}` + env PYTHONUNBUFFERED=1.\n\
         1. setup: `git clone {url}` + `start.bat` + USB debugging (`atlas mobile status`).\n\
         2. one-click: `uv run artemis mcp --install all` (o `--generate-config <client>` para el JSON manual).\n\
         3. opencode: pega el snippet en `.opencode/mcp.json` o corre `atlas mobile mcp-template --write`.\n\
         4. valida: dispositivo físico + `atlas mobile run --task \"...\" --profile flash` end-to-end.\n\
         {license}",
        tools = ARTEMIS_MCP_TOOLS.join(", "),
        repo = repo,
        module = ARTEMIS_MCP_MODULE,
        url = crate::mobile::ARTEMIS_REPO_URL,
        license = crate::mobile::ARTEMIS_LICENSE_NOTE
    )
}

pub fn merge_into_opencode_config(existing: &str, repo_dir: &str) -> anyhow::Result<String> {
    let repo = normalize_repo(repo_dir);
    let mut root: Value = serde_json::from_str(existing)
        .map_err(|e| anyhow::anyhow!("existing mcp.json is not valid JSON: {e}"))?;
    let obj = root
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("existing mcp.json root must be an object"))?;
    let mcp = obj
        .entry("mcp".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let mcp_obj = mcp
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("`mcp` key must be an object"))?;
    let entry = serde_json::json!({
        "type": "stdio",
        "command": opencode_command(&repo),
        "enabled": true,
        "timeout": ARTEMIS_MCP_TIMEOUT_MS,
    });
    mcp_obj.insert(ARTEMIS_MCP_SERVER_ID.to_string(), entry);
    if !obj.contains_key("$schema") {
        obj.insert(
            "$schema".to_string(),
            Value::String("https://opencode.ai/mcp.json".to_string()),
        );
    }
    Ok(serde_json::to_string_pretty(&root)?)
}

fn normalize_repo(raw: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        ARTEMIS_REPO_PLACEHOLDER.to_string()
    } else {
        t.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_match_rfc38_five_typed_names() {
        assert_eq!(
            artemis_tools(),
            [
                "mobile_run_task",
                "mobile_manage_task",
                "mobile_get_device_state",
                "mobile_inspect_trace",
                "mobile_diagnose",
            ]
        );
    }

    #[test]
    fn full_file_parses_and_carries_stdio_shape() {
        let out = render_full_file("/tmp/artemis-clone");
        let v: Value = serde_json::from_str(&out).expect("template must be valid JSON");
        let cmd = v
            .pointer("/mcp/artemis/command")
            .and_then(|c| c.as_array())
            .expect("mcp.artemis.command array");
        let flat: Vec<&str> = cmd.iter().filter_map(|s| s.as_str()).collect();
        assert_eq!(
            flat,
            vec![
                "uv",
                "--directory",
                "/tmp/artemis-clone",
                "run",
                "artemis",
                "mcp"
            ]
        );
        assert_eq!(
            v.pointer("/mcp/artemis/type").and_then(|t| t.as_str()),
            Some("stdio")
        );
    }

    #[test]
    fn blank_repo_falls_back_to_placeholder() {
        let out = render_full_file("   ");
        assert!(out.contains(ARTEMIS_REPO_PLACEHOLDER));
    }

    #[test]
    fn merge_preserves_existing_servers_and_adds_artemis() {
        let existing =
            r#"{"mcp":{"context7":{"type":"stdio","command":["pnpm","dlx","x"],"enabled":true}}}"#;
        let merged = merge_into_opencode_config(existing, "/tmp/artemis").unwrap();
        let v: Value = serde_json::from_str(&merged).unwrap();
        assert!(v.pointer("/mcp/context7").is_some());
        assert!(v.pointer("/mcp/artemis/command").is_some());
    }

    #[test]
    fn merge_rejects_invalid_json() {
        assert!(merge_into_opencode_config("{not json", "/tmp/artemis").is_err());
    }

    #[test]
    fn merge_rejects_non_object_root() {
        assert!(merge_into_opencode_config("[1,2]", "/tmp/artemis").is_err());
    }

    #[test]
    fn guide_names_tools_and_lateral_flow() {
        let g = render_guide("/tmp/artemis");
        for t in ARTEMIS_MCP_TOOLS {
            assert!(g.contains(t));
        }
        assert!(g.contains("uv run artemis mcp --install all"));
        assert!(g.contains("atlas mobile run"));
        assert!(g.contains("Apache-2.0"));
    }
}
