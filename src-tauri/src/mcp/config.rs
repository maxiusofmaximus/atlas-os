// Atlas OS — MCP server registry (RFC 07 §1/§2/§4).
//
// Two on-disk shapes are accepted and normalized into one
// `McpServerConfig`:
//
//   * opencode-compatible (`.opencode/mcp.json`, the live file the HUD's
//     `GET /hud/mcp` already reads): `{ "mcp": { name: { type, command:
//     [argv], enabled, timeout } } }`.
//   * RFC 07 (`mcp.json`): `{ "mcpServers": { name: { command, args,
//     env, trusted, sandbox, allowed_tools } } }`.
//
// `command` may be a single string (RFC 07) or an argv array (opencode);
// both collapse to `command` + `args`. A missing file is not an error —
// it means "no servers configured", never a fabricated list.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Live opencode-compatible registry, relative to a repo root.
pub const OPENCODE_CONFIG: &str = ".opencode/mcp.json";
/// RFC 07 registry, relative to a root.
pub const RFC07_CONFIG: &str = "mcp.json";
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
pub const DEFAULT_TRANSPORT: &str = "stdio";

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_MS
}

/// Isolation model for a server (RFC 07 §2). `camelCase` keeps the RFC's
/// own spelling (`vuOnly`), which a plain `lowercase` rename would break.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum McpSandbox {
    /// Kernel-owned servers only. Never for third-party code.
    None,
    /// Same host, virtualized filesystem, network allowlist.
    VuOnly,
    /// Docker/Podman container, restricted network, readonly mounts.
    #[default]
    Container,
}

impl McpSandbox {
    pub fn as_str(self) -> &'static str {
        match self {
            McpSandbox::None => "none",
            McpSandbox::VuOnly => "vuOnly",
            McpSandbox::Container => "container",
        }
    }
}

impl std::fmt::Display for McpSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for McpSandbox {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(McpSandbox::None),
            "vuOnly" | "vuonly" => Ok(McpSandbox::VuOnly),
            "container" => Ok(McpSandbox::Container),
            other => Err(format!(
                "unknown sandbox `{other}` (expected none|vuOnly|container)"
            )),
        }
    }
}

/// One normalized MCP server. Every field has a safe default so a sparse
/// opencode entry still yields a usable config.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub trusted: bool,
    #[serde(default)]
    pub sandbox: McpSandbox,
    /// RFC 07 §4: empty means the agent may call *nothing* on this server.
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_transport")]
    pub transport: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_transport() -> String {
    DEFAULT_TRANSPORT.to_string()
}

impl McpServerConfig {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: BTreeMap::new(),
            trusted: false,
            sandbox: McpSandbox::default(),
            allowed_tools: Vec::new(),
            enabled: true,
            transport: default_transport(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }

    pub fn is_stdio(&self) -> bool {
        self.transport.eq_ignore_ascii_case(DEFAULT_TRANSPORT)
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_ms.max(1))
    }

    /// RFC 07 §4 — a tool is reachable by the agent only if listed.
    pub fn is_tool_allowed(&self, tool: &str) -> bool {
        self.allowed_tools.iter().any(|t| t == tool)
    }

    /// Whether this server exposes any tool to the agent at all.
    pub fn exposes_any(&self) -> bool {
        !self.allowed_tools.is_empty()
    }

    /// The full argv to spawn: `command` followed by `args`.
    pub fn argv(&self) -> Vec<String> {
        let mut v = Vec::with_capacity(1 + self.args.len());
        v.push(self.command.clone());
        v.extend(self.args.iter().cloned());
        v
    }

    /// Reject configs the client cannot honour before we try to spawn.
    pub fn validate(&self) -> Result<(), String> {
        if self.command.trim().is_empty() {
            return Err("server is missing a `command`".to_string());
        }
        if !self.is_stdio() {
            return Err(format!(
                "transport `{}` is not supported yet (only `stdio`)",
                self.transport
            ));
        }
        Ok(())
    }
}

/// The registry: name → server. Serializes to the RFC 07 `mcpServers`
/// shape so `atlas mcp add` writes a file both shapes can read.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpRegistry {
    #[serde(rename = "mcpServers", default)]
    pub servers: BTreeMap<String, McpServerConfig>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.servers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&McpServerConfig> {
        self.servers.get(name)
    }

    pub fn insert(
        &mut self,
        name: impl Into<String>,
        cfg: McpServerConfig,
    ) -> Option<McpServerConfig> {
        self.servers.insert(name.into(), cfg)
    }

    pub fn remove(&mut self, name: &str) -> Option<McpServerConfig> {
        self.servers.remove(name)
    }

    pub fn names(&self) -> Vec<&str> {
        self.servers.keys().map(String::as_str).collect()
    }

    /// Path of the RFC 07 registry file for a root.
    pub fn config_path(root: &Path) -> PathBuf {
        root.join(RFC07_CONFIG)
    }

    /// Parse either on-disk shape. `mcpServers` (RFC 07) is applied
    /// first; `mcp` (opencode) fills names not already present.
    pub fn from_json(text: &str) -> anyhow::Result<Self> {
        let value: Value = serde_json::from_str(text).context("invalid JSON")?;
        Self::from_value(&value)
    }

    pub fn from_value(value: &Value) -> anyhow::Result<Self> {
        let mut out = McpRegistry::default();
        if let Some(obj) = value.get("mcpServers").and_then(Value::as_object) {
            for (name, cfg) in obj {
                out.servers.insert(
                    name.clone(),
                    normalize_rfc07(cfg).with_context(|| format!("server `{name}`"))?,
                );
            }
        }
        if let Some(obj) = value.get("mcp").and_then(Value::as_object) {
            for (name, cfg) in obj {
                let cfg = normalize_opencode(cfg).with_context(|| format!("server `{name}`"))?;
                out.servers.entry(name.clone()).or_insert(cfg);
            }
        }
        Ok(out)
    }

    /// Merge registries from several paths, first path winning on a name
    /// collision. Missing files are skipped.
    pub fn load_from(paths: &[PathBuf]) -> anyhow::Result<Self> {
        let mut out = McpRegistry::default();
        for path in paths {
            if !path.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?;
            let reg =
                Self::from_json(&text).with_context(|| format!("parsing {}", path.display()))?;
            for (name, cfg) in reg.servers {
                out.servers.entry(name).or_insert(cfg);
            }
        }
        Ok(out)
    }

    /// Convenience: the RFC 07 file for `root` plus the opencode file
    /// under the same root.
    pub fn discover(root: &Path) -> anyhow::Result<Self> {
        Self::load_from(&[Self::config_path(root), root.join(OPENCODE_CONFIG)])
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self).context("serializing registry")?;
        std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }
}

/// Extract `(command, args)` from a server value. `command` accepts a
/// single string (RFC 07) or an argv array (opencode); an extra `args`
/// array is appended after the argv tail.
fn command_and_args(cfg: &Value) -> anyhow::Result<(String, Vec<String>)> {
    let command = cfg.get("command").context("server is missing `command`")?;
    let (base, mut args) = match command {
        Value::String(s) => (s.clone(), Vec::new()),
        Value::Array(items) => {
            let mut it = items.iter();
            let first = it
                .next()
                .and_then(Value::as_str)
                .context("`command` array must start with a non-empty string")?;
            if first.trim().is_empty() {
                bail!("`command` array must start with a non-empty string");
            }
            let rest = it
                .map(|v| v.as_str().unwrap_or_default().to_string())
                .collect::<Vec<_>>();
            (first.to_string(), rest)
        }
        _ => bail!("`command` must be a string or an array of strings"),
    };
    if let Some(extra) = cfg.get("args").and_then(Value::as_array) {
        args.extend(
            extra
                .iter()
                .map(|v| v.as_str().unwrap_or_default().to_string()),
        );
    }
    Ok((base, args))
}

fn env_map(cfg: &Value) -> BTreeMap<String, String> {
    cfg.get("env")
        .and_then(Value::as_object)
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn string_list(cfg: &Value, key: &str) -> Vec<String> {
    cfg.get(key)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn normalize_rfc07(cfg: &Value) -> anyhow::Result<McpServerConfig> {
    let (command, args) = command_and_args(cfg)?;
    let sandbox = cfg
        .get("sandbox")
        .and_then(Value::as_str)
        .map(|s| s.parse().unwrap_or_default())
        .unwrap_or_default();
    Ok(McpServerConfig {
        command,
        args,
        env: env_map(cfg),
        trusted: cfg.get("trusted").and_then(Value::as_bool).unwrap_or(false),
        sandbox,
        allowed_tools: string_list(cfg, "allowed_tools"),
        enabled: cfg.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        transport: cfg
            .get("transport")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_TRANSPORT)
            .to_string(),
        timeout_ms: cfg
            .get("timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TIMEOUT_MS),
    })
}

fn normalize_opencode(cfg: &Value) -> anyhow::Result<McpServerConfig> {
    let (command, args) = command_and_args(cfg)?;
    Ok(McpServerConfig {
        command,
        args,
        env: env_map(cfg),
        trusted: false,
        sandbox: McpSandbox::default(),
        allowed_tools: string_list(cfg, "allowed_tools"),
        enabled: cfg.get("enabled").and_then(Value::as_bool).unwrap_or(true),
        transport: cfg
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_TRANSPORT)
            .to_string(),
        timeout_ms: cfg
            .get("timeout")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TIMEOUT_MS),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_live_opencode_registry() {
        let text = r#"{
            "$schema": "https://opencode.ai/mcp.json",
            "mcp": {
                "context7": {
                    "type": "stdio",
                    "command": ["pnpm", "dlx", "@upstash/context7-mcp@3.2.4"],
                    "enabled": true,
                    "timeout": 60000
                }
            }
        }"#;
        let reg = McpRegistry::from_json(text).unwrap();
        let c7 = reg.get("context7").expect("context7 present");
        assert_eq!(c7.command, "pnpm");
        assert_eq!(c7.args, vec!["dlx", "@upstash/context7-mcp@3.2.4"]);
        assert!(c7.enabled);
        assert_eq!(c7.timeout_ms, 60000);
        assert!(c7.is_stdio());
        assert_eq!(c7.argv()[0], "pnpm");
    }

    #[test]
    fn parses_rfc07_registry_with_string_command() {
        let text = r#"{
            "mcpServers": {
                "playwright": {
                    "command": "npx",
                    "args": ["-y", "@playwright/mcp"],
                    "sandbox": "container",
                    "allowed_tools": ["navigate", "click"]
                }
            }
        }"#;
        let reg = McpRegistry::from_json(text).unwrap();
        let pw = reg.get("playwright").expect("playwright present");
        assert_eq!(pw.command, "npx");
        assert_eq!(pw.args, vec!["-y", "@playwright/mcp"]);
        assert_eq!(pw.sandbox, McpSandbox::Container);
        assert!(pw.is_tool_allowed("navigate"));
        assert!(!pw.is_tool_allowed("shell.exec"));
    }

    #[test]
    fn vu_only_spelling_roundtrips() {
        let reg =
            McpRegistry::from_json(r#"{"mcpServers":{"x":{"command":"c","sandbox":"vuOnly"}}}"#)
                .unwrap();
        assert_eq!(reg.get("x").unwrap().sandbox, McpSandbox::VuOnly);
        let json = serde_json::to_value(reg).unwrap();
        assert_eq!(json["mcpServers"]["x"]["sandbox"], "vuOnly");
    }

    #[test]
    fn missing_command_is_rejected() {
        let err = McpRegistry::from_json(r#"{"mcpServers":{"x":{"args":[]}}}"#).unwrap_err();
        // anyhow's Display shows only the outermost context; `{:#}` walks
        // the chain down to the actual cause.
        assert!(format!("{err:#}").contains("command"));
    }

    #[test]
    fn empty_allowlist_exposes_nothing() {
        let cfg = McpServerConfig::new("cmd");
        assert!(!cfg.exposes_any());
        assert!(!cfg.is_tool_allowed("anything"));
    }

    #[test]
    fn validate_rejects_non_stdio_transport() {
        let mut cfg = McpServerConfig::new("cmd");
        cfg.transport = "sse".into();
        assert!(cfg.validate().unwrap_err().contains("not supported"));
        cfg.transport = "stdio".into();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = McpRegistry::config_path(dir.path());
        let mut reg = McpRegistry::new();
        let mut cfg = McpServerConfig::new("npx");
        cfg.args = vec!["-y".into(), "@playwright/mcp".into()];
        cfg.allowed_tools = vec!["navigate".into()];
        cfg.trusted = true;
        reg.insert("playwright", cfg);
        reg.save(&path).unwrap();

        let loaded = McpRegistry::discover(dir.path()).unwrap();
        let pw = loaded.get("playwright").unwrap();
        assert_eq!(pw.command, "npx");
        assert!(pw.trusted);
        assert!(pw.is_tool_allowed("navigate"));
    }

    #[test]
    fn load_missing_file_is_empty_not_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let reg = McpRegistry::discover(dir.path()).unwrap();
        assert!(reg.is_empty());
    }

    #[test]
    fn load_from_first_path_wins() {
        let dir = tempfile::TempDir::new().unwrap();
        let a = dir.path().join("a.json");
        let b = dir.path().join("b.json");
        std::fs::write(&a, r#"{"mcpServers":{"s":{"command":"from-a"}}}"#).unwrap();
        std::fs::write(
            &b,
            r#"{"mcpServers":{"s":{"command":"from-b"},"t":{"command":"only-b"}}}"#,
        )
        .unwrap();
        let reg = McpRegistry::load_from(&[a, b]).unwrap();
        assert_eq!(reg.get("s").unwrap().command, "from-a");
        assert_eq!(reg.get("t").unwrap().command, "only-b");
    }
}
