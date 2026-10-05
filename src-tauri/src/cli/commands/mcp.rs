// Atlas OS — `atlas mcp` runtime CLI (RFC 07, Fase 29.0).
//
// Replaces the Fase-0 stub. Reads the same registry the HUD does
// (`.opencode/mcp.json`) plus the RFC 07 `mcp.json`, and speaks the real
// MCP stdio protocol for `probe`/`call`. `call` enforces the per-server
// `allowed_tools` allowlist (RFC 07 §4) before any byte reaches the
// server, so a mis-declared tool can never be reached by accident.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use serde_json::Value;

use crate::mcp::{McpClient, McpRegistry, McpSandbox, McpServerConfig};
use crate::profiles::{resolve_root, ProfileId};

#[derive(Args, Debug)]
pub struct McpCmd {
    #[command(subcommand)]
    pub action: McpAction,
}

#[derive(Subcommand, Debug)]
pub enum McpAction {
    /// List configured MCP servers (registry read-only, never spawns).
    List,
    /// Add or replace a server in `<profile>/mcp.json`.
    Add {
        /// Registry key, e.g. `context7`.
        name: String,
        /// Executable to spawn, e.g. `pnpm`.
        #[arg(long)]
        command: String,
        /// One argv element (repeatable), e.g. --arg dlx --arg @upstash/context7-mcp.
        #[arg(long = "arg", allow_hyphen_values = true)]
        args: Vec<String>,
        /// One `KEY=VALUE` environment entry (repeatable).
        #[arg(long = "env", allow_hyphen_values = true)]
        env: Vec<String>,
        /// Mark the server's package signature as verified (RFC 07 §1).
        #[arg(long, default_value_t = false)]
        trusted: bool,
        /// Isolation model (RFC 07 §2).
        #[arg(long, value_parser = ["none", "vuOnly", "container"], default_value = "container")]
        sandbox: String,
        /// One allowlisted tool (repeatable). Empty = the agent may call nothing.
        #[arg(long = "allow")]
        allow: Vec<String>,
    },
    /// Remove a server from the registry.
    Remove { name: String },
    /// Connect, handshake and list the server's tools (read-only).
    Probe {
        name: String,
        /// Refuse to spawn when the server's required sandbox is not enforceable (RFC 07 §2).
        #[arg(long, default_value_t = false)]
        strict: bool,
    },
    /// Call a tool (enforced against the server's `allowed_tools`).
    Call {
        name: String,
        tool: String,
        /// JSON arguments object, e.g. --args '{"libraryId":"/x/y"}'.
        #[arg(long, default_value = "{}")]
        args: String,
        /// Refuse to spawn when the server's required sandbox is not enforceable (RFC 07 §2).
        #[arg(long, default_value_t = false)]
        strict: bool,
    },
}

pub async fn run(cmd: McpCmd, profile: &str) -> Result<()> {
    let root = resolve_root(&ProfileId::new(profile))?;
    match cmd.action {
        McpAction::List => list(&root),
        McpAction::Add {
            name,
            command,
            args,
            env,
            trusted,
            sandbox,
            allow,
        } => add(&root, name, command, args, env, trusted, &sandbox, allow),
        McpAction::Remove { name } => remove(&root, &name),
        McpAction::Probe { name, strict } => probe(&root, &name, strict).await,
        McpAction::Call {
            name,
            tool,
            args,
            strict,
        } => call(&root, &name, &tool, &args, strict).await,
    }
}

/// Candidate registry files, in precedence order: the profile's RFC 07
/// file, its opencode file, then the process cwd (the repo checkout, where
/// `.opencode/mcp.json` lives).
fn registry_paths(root: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = vec![
        McpRegistry::config_path(root),
        root.join(".opencode").join("mcp.json"),
    ];
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join(".opencode").join("mcp.json"));
        paths.push(cwd.join("mcp.json"));
    }
    paths
}

fn load_registry(root: &std::path::Path) -> Result<McpRegistry> {
    McpRegistry::load_from(&registry_paths(root))
}

fn list(root: &std::path::Path) -> Result<()> {
    let registry = load_registry(root)?;
    if registry.is_empty() {
        println!(
            "mcp: no servers configured.\n  Add one with `atlas mcp add <name> --command <exe>`,\n  or drop a `.opencode/mcp.json` (opencode shape) at the repo root."
        );
        return Ok(());
    }
    println!("mcp: {} server(s) configured", registry.len());
    for name in registry.names() {
        let cfg = registry.get(name).expect("name from keys");
        let trust = if cfg.trusted { "trusted" } else { "untrusted" };
        let enabled = if cfg.enabled { "enabled" } else { "disabled" };
        let tools = if cfg.exposes_any() {
            format!("{} allowed", cfg.allowed_tools.len())
        } else {
            "no tools exposed".to_string()
        };
        let effective = cfg.effective_sandbox();
        let sandbox = if effective == cfg.sandbox {
            format!("{}", cfg.sandbox)
        } else {
            format!("{}→{effective}", cfg.sandbox)
        };
        println!(
            "  {name}  [{sandbox}] {trust} {enabled} — {tools}\n    {}",
            cfg.argv().join(" ")
        );
        if let Some(finding) = cfg.sandbox_finding() {
            println!("    ! {finding}");
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add(
    root: &std::path::Path,
    name: String,
    command: String,
    args: Vec<String>,
    env: Vec<String>,
    trusted: bool,
    sandbox: &str,
    allow: Vec<String>,
) -> Result<()> {
    let mut registry = load_registry(root)?;
    let mut cfg = McpServerConfig::new(command);
    cfg.args = args;
    cfg.env = parse_env(&env)?;
    cfg.trusted = trusted;
    cfg.sandbox = sandbox
        .parse::<McpSandbox>()
        .map_err(|e| anyhow::anyhow!(e))?;
    cfg.allowed_tools = allow;
    cfg.validate().map_err(|e| anyhow::anyhow!(e))?;
    let finding = cfg.sandbox_finding();
    let replaced = registry.insert(name.clone(), cfg).is_some();
    let path = McpRegistry::config_path(root);
    registry.save(&path)?;
    println!(
        "mcp: {} `{name}` in {}",
        if replaced { "replaced" } else { "added" },
        path.display()
    );
    if let Some(f) = finding {
        println!("mcp: note — {f}");
    }
    Ok(())
}

fn remove(root: &std::path::Path, name: &str) -> Result<()> {
    let mut registry = load_registry(root)?;
    if registry.remove(name).is_none() {
        bail!("no MCP server named `{name}` in the registry");
    }
    let path = McpRegistry::config_path(root);
    registry.save(&path)?;
    println!("mcp: removed `{name}` from {}", path.display());
    Ok(())
}

fn parse_env(entries: &[String]) -> Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    for entry in entries {
        let (k, v) = entry
            .split_once('=')
            .with_context(|| format!("env entry `{entry}` must be KEY=VALUE"))?;
        if k.trim().is_empty() {
            bail!("env entry `{entry}` has an empty key");
        }
        env.insert(k.to_string(), v.to_string());
    }
    Ok(env)
}

/// RFC 07 §2 — warn (default) or refuse (`--strict`) before spawning a server
/// whose required isolation the runtime cannot yet enforce. Fail-safe: never
/// silently run an unsandboxed server when policy demands isolation.
fn enforce_sandbox(name: &str, cfg: &McpServerConfig, strict: bool) -> Result<()> {
    if let Some(finding) = cfg.sandbox_finding() {
        if strict {
            bail!("mcp: refusing `{name}` under --strict — {finding}");
        }
        eprintln!("mcp: WARNING — {finding}");
    }
    Ok(())
}

async fn probe(root: &std::path::Path, name: &str, strict: bool) -> Result<()> {
    let registry = load_registry(root)?;
    let cfg = registry
        .get(name)
        .with_context(|| format!("no MCP server named `{name}` in the registry"))?;
    cfg.validate().map_err(|e| anyhow::anyhow!(e))?;
    enforce_sandbox(name, cfg, strict)?;

    let mut client = McpClient::connect(name, cfg)
        .await
        .map_err(anyhow::Error::from)?;
    let tools = client.tools_list().await.map_err(anyhow::Error::from)?;

    println!("mcp: `{name}` ready — {} tool(s)", tools.len());
    for tool in &tools {
        let mark = if cfg.is_tool_allowed(&tool.name) {
            "allowed"
        } else {
            "NOT allowed"
        };
        let desc = tool
            .description
            .as_deref()
            .map(|d| format!(" — {d}"))
            .unwrap_or_default();
        println!("  {} [{mark}]{desc}", tool.name);
    }
    if !cfg.exposes_any() {
        println!(
            "  (no tools allowlisted — add them with `atlas mcp add {name} ... --allow <tool>`)"
        );
    }
    Ok(())
}

async fn call(
    root: &std::path::Path,
    name: &str,
    tool: &str,
    args: &str,
    strict: bool,
) -> Result<()> {
    let registry = load_registry(root)?;
    let cfg = registry
        .get(name)
        .with_context(|| format!("no MCP server named `{name}` in the registry"))?;
    cfg.validate().map_err(|e| anyhow::anyhow!(e))?;
    enforce_sandbox(name, cfg, strict)?;
    let arguments: Value = serde_json::from_str(args).context("`--args` must be a JSON value")?;

    let mut client = McpClient::connect(name, cfg)
        .await
        .map_err(anyhow::Error::from)?;
    let outcome = client
        .tools_call_checked(cfg, tool, arguments)
        .await
        .map_err(anyhow::Error::from)?;

    if !outcome.text.is_empty() {
        println!("{}", outcome.text);
    }
    if outcome.is_error {
        bail!("MCP tool `{tool}` reported an error");
    }
    Ok(())
}
