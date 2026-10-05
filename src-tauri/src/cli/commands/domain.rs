// Atlas OS — `atlas domain` verbs (RFC 64 §5/§8, Fase 28).
//
// list/use/probe/open/guide over the seed + installed domain packs; install a
// pack from a local `domain.toml` with a mandatory SHA-256 signature (RFC 18
// §6). Lateral tools are always external processes (never bundled).

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};

use crate::domain::{lateral, pack_sha256, registry::DomainRegistry};
use crate::journal::DomainPackRow;

#[derive(Args, Debug)]
pub struct DomainCmd {
    #[command(subcommand)]
    pub action: DomainAction,
}

#[derive(Subcommand, Debug)]
pub enum DomainAction {
    /// List domain packs (seed + installed).
    List,
    /// Show a pack's engine routing / policy (Project Map auto-activates it).
    Use { id: String },
    /// Probe a pack's lateral tools (`installed?`).
    Probe {
        id: String,
        /// Restrict to one tool id.
        tool: Option<String>,
    },
    /// Shell out to a pack's lateral tool.
    Open {
        id: String,
        tool: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Print the install guide for a lateral tool.
    Guide {
        id: String,
        /// Restrict to one tool id.
        tool: Option<String>,
    },
    /// Install a pack from a local `domain.toml` (signature required, RFC 18 §6).
    Install {
        /// Path to the `domain.toml` (or its directory).
        path: String,
        /// Expected SHA-256 of the manifest (mandatory; unsigned packs are rejected).
        #[arg(long, value_name = "SHA256")]
        sha256: String,
    },
}

fn join_or_dash(items: &[String]) -> String {
    if items.is_empty() {
        "—".to_string()
    } else {
        items.join(", ")
    }
}

fn require<'a>(reg: &'a DomainRegistry, id: &str) -> Result<&'a crate::domain::DomainPack> {
    reg.get(id)
        .with_context(|| format!("domain pack `{id}` not found (try `atlas domain list`)"))
}

pub async fn run(cmd: DomainCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let reg = DomainRegistry::load(&root);

    match cmd.action {
        DomainAction::List => {
            for p in reg.list() {
                println!(
                    "{:<16} {:<40} v{} ({} lateral, {} skills)",
                    p.domain.id,
                    p.domain.title,
                    p.domain.version,
                    p.tools.lateral.open.len(),
                    p.skills.bundled.len()
                );
            }
        }
        DomainAction::Use { id } => {
            let p = require(&reg, &id)?;
            println!("domain: {} — {}", p.domain.id, p.domain.title);
            println!("  engines.planning = {}", p.engines.planning);
            println!("  engines.coding   = {}", p.engines.coding);
            println!(
                "  validation       = {}",
                join_or_dash(&p.engines.validation)
            );
            println!("  artifacts        = {}", join_or_dash(&p.artifacts.types));
            println!("  sandbox          = {}", p.policy.sandbox);
            println!(
                "  detect (Project Map RFC 11) = {}",
                join_or_dash(&p.domain.detect)
            );
        }
        DomainAction::Probe { id, tool } => {
            let p = require(&reg, &id)?;
            for t in lateral::tools_for(p) {
                if let Some(filter) = &tool {
                    if &t.id != filter {
                        continue;
                    }
                }
                if t.installed() {
                    println!("[ok] {:<12}", t.id);
                } else {
                    println!("[--] {:<12} {}", t.id, t.guide);
                }
            }
        }
        DomainAction::Open { id, tool, args } => {
            let p = require(&reg, &id)?;
            let t = lateral::tools_for(p)
                .into_iter()
                .find(|t| t.id == tool)
                .with_context(|| format!("tool `{tool}` is not declared by pack `{id}`"))?;
            let status = lateral::open(&t, &args)?;
            if !status.success() {
                bail!("`{}` exited with {status}", t.bin);
            }
        }
        DomainAction::Guide { id, tool } => {
            let p = require(&reg, &id)?;
            match tool {
                Some(t) => println!("{}", lateral::guide_for(&t)),
                None => {
                    for t in lateral::tools_for(p) {
                        println!("{}: {}", t.id, t.guide);
                    }
                }
            }
        }
        DomainAction::Install { path, sha256 } => {
            let p = PathBuf::from(&path);
            let manifest_path = if p.is_dir() {
                p.join("domain.toml")
            } else {
                p.clone()
            };
            let text = std::fs::read_to_string(&manifest_path)
                .with_context(|| format!("read {}", manifest_path.display()))?;
            let pack = crate::domain::parse_manifest(&text)?;
            let actual = pack_sha256(&text);
            if !actual.eq_ignore_ascii_case(sha256.trim()) {
                bail!(
                    "signature mismatch — manifest sha256 = {actual}, expected {}. \
                     RFC 18 §6: unsigned/tampered packs are rejected.",
                    sha256.trim()
                );
            }
            let dest_dir = root.join("domains").join(&pack.domain.id);
            std::fs::create_dir_all(&dest_dir)
                .with_context(|| format!("create {}", dest_dir.display()))?;
            std::fs::write(dest_dir.join("domain.toml"), &text)?;
            let journal = crate::journal::Journal::open(&root)?;
            journal.upsert_domain_pack(&DomainPackRow {
                id: pack.domain.id.clone(),
                title: pack.domain.title.clone(),
                version: pack.domain.version.clone(),
                manifest_toml: text,
                sha256: Some(actual),
                signed: true,
                installed: true,
                ts: chrono::Utc::now().timestamp_millis(),
            })?;
            println!(
                "installed domain pack `{}` v{} -> {}",
                pack.domain.id,
                pack.domain.version,
                dest_dir.display()
            );
        }
    }
    Ok(())
}
