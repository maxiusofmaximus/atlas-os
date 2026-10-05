// Atlas OS — `domain.toml` contract + parser (RFC 64 §2/§4, M51).
//
// Mirrors the RFC 64 §2 snippet 1:1. `parse_manifest` is the single entry
// point; it validates `domain.id` ([a-z0-9-_], 2..=64, starts [a-z]) so a
// malformed pack fails before it can be registered or installed.

use anyhow::{bail, Context, Result};
use serde::Deserialize;

fn default_engine() -> String {
    "default".into()
}

fn default_sandbox() -> String {
    "local".into()
}

fn default_version() -> String {
    "0.1.0".into()
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DomainPack {
    pub domain: DomainMeta,
    #[serde(default)]
    pub engines: EngineRouting,
    #[serde(default)]
    pub skills: SkillsRef,
    #[serde(default)]
    pub mcp: McpRef,
    #[serde(default)]
    pub tools: ToolsRef,
    #[serde(default)]
    pub artifacts: ArtifactTypes,
    #[serde(default)]
    pub policy: DomainPolicy,
    /// RFC 64 §8 — executable check commands the `Domain` validation stage runs.
    #[serde(default)]
    pub validation: DomainValidation,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DomainMeta {
    pub id: String,
    pub title: String,
    /// Recursion signal for the Project Map (RFC 11) / Capability Resolver.
    #[serde(default)]
    pub detect: Vec<String>,
    #[serde(default = "default_version")]
    pub version: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct EngineRouting {
    #[serde(default = "default_engine")]
    pub planning: String,
    #[serde(default = "default_engine")]
    pub coding: String,
    #[serde(default)]
    pub validation: Vec<String>,
}

impl Default for EngineRouting {
    fn default() -> Self {
        Self {
            planning: default_engine(),
            coding: default_engine(),
            validation: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct SkillsRef {
    #[serde(default)]
    pub bundled: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct McpRef {
    #[serde(default)]
    pub servers: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct ToolsRef {
    #[serde(default)]
    pub lateral: LateralRef,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct LateralRef {
    /// External binaries shelled out to (RFC 28 §I, never bundled).
    #[serde(default)]
    pub open: Vec<String>,
    #[serde(default)]
    pub probe: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct ArtifactTypes {
    #[serde(default)]
    pub types: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct DomainPolicy {
    #[serde(default)]
    pub sensitive: Vec<String>,
    #[serde(default = "default_sandbox")]
    pub sandbox: String,
}

impl Default for DomainPolicy {
    fn default() -> Self {
        Self {
            sensitive: Vec::new(),
            sandbox: default_sandbox(),
        }
    }
}

/// RFC 64 §8 — the executable form of the domain's validation. `engines.
/// validation` are display labels; `commands` are what the `Domain` stage
/// actually runs (empty → the stage is `Skipped`, never a fake pass).
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct DomainValidation {
    #[serde(default)]
    pub commands: Vec<String>,
}

/// Validate a domain id: `[a-z0-9-_]`, 2..=64 chars, must start with `[a-z]`.
pub fn validate_id(id: &str) -> Result<()> {
    if id.len() < 2 || id.len() > 64 {
        bail!("domain.id must be 2..=64 chars, got {} in {id:?}", id.len());
    }
    let mut chars = id.chars();
    let first = chars.next().expect("non-empty");
    if !first.is_ascii_lowercase() {
        bail!("domain.id must start with [a-z]: {id:?}");
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        bail!("domain.id must be [a-z0-9-_]: {id:?}");
    }
    Ok(())
}

/// Parse + validate a `domain.toml`.
pub fn parse_manifest(text: &str) -> Result<DomainPack> {
    let pack: DomainPack = toml::from_str(text).context("parse domain.toml")?;
    validate_id(&pack.domain.id)?;
    if pack.domain.title.trim().is_empty() {
        bail!("domain.title must not be empty");
    }
    Ok(pack)
}

/// SHA-256 (lowercase hex) of a manifest's raw bytes — the pack signature the
/// §6 RFC 18 rule checks before installation.
pub fn pack_sha256(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAD: &str = r#"
[domain]
id = "cad"
title = "Mechanical / Parametric CAD"
detect = ["*.step", "*.stl", "cadquery", "freecad"]
version = "0.2.0"

[engines]
planning = "default"
coding = "code-as-cad"
validation = ["geometry.manifold", "units.consistent"]

[skills]
bundled = ["cad.text-to-cadquery"]

[mcp]
servers = ["freecad-mcp"]

[tools.lateral]
open = ["freecad", "openscad"]
probe = "atlas domain probe cad"

[artifacts]
types = ["step", "stl", "render.png"]

[policy]
sensitive = ["exec.run", "fs.write.*"]
sandbox = "container"

[validation]
commands = ["openscad --check model.scad"]
"#;

    #[test]
    fn parses_a_full_manifest() {
        let p = parse_manifest(CAD).expect("parse");
        assert_eq!(p.domain.id, "cad");
        assert_eq!(p.domain.version, "0.2.0");
        assert_eq!(p.domain.detect.len(), 4);
        assert_eq!(p.engines.coding, "code-as-cad");
        assert_eq!(
            p.engines.validation,
            vec!["geometry.manifold", "units.consistent"]
        );
        assert_eq!(p.skills.bundled, vec!["cad.text-to-cadquery"]);
        assert_eq!(p.mcp.servers, vec!["freecad-mcp"]);
        assert_eq!(p.tools.lateral.open, vec!["freecad", "openscad"]);
        assert_eq!(p.artifacts.types, vec!["step", "stl", "render.png"]);
        assert_eq!(p.policy.sandbox, "container");
        assert_eq!(p.validation.commands, vec!["openscad --check model.scad"]);
    }

    #[test]
    fn minimal_manifest_uses_defaults() {
        let p = parse_manifest("[domain]\nid = \"x1\"\ntitle = \"X\"\n").expect("parse");
        assert_eq!(p.engines.planning, "default");
        assert_eq!(p.policy.sandbox, "local");
        assert!(p.tools.lateral.open.is_empty());
        assert_eq!(p.domain.version, "0.1.0");
        assert!(p.validation.commands.is_empty());
    }

    #[test]
    fn rejects_malformed_and_invalid_ids() {
        assert!(parse_manifest("this is not toml = =").is_err());
        assert!(parse_manifest("[domain]\ntitle = \"no id\"").is_err());
        assert!(parse_manifest("[domain]\nid = \"CAD\"\ntitle = \"T\"").is_err()); // uppercase
        assert!(parse_manifest("[domain]\nid = \"c\"\ntitle = \"T\"").is_err()); // too short
        assert!(parse_manifest("[domain]\nid = \"cad\"\ntitle = \"  \"").is_err());
        // empty title
    }

    #[test]
    fn sha256_is_stable_hex() {
        let h = pack_sha256("hello");
        assert_eq!(h.len(), 64);
        assert_eq!(
            h,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
