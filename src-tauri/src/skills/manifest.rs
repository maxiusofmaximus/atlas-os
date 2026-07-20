// OpenCode OS — Skill manifest parser (RFC 06).
// Skills live in `~/.opencode/profiles/<id>/skills/<name>/skill.toml`
// or `skills/<name>/skill.toml` bundled with the desktop installer.
// Phase 0 minimal struct; Phase 5 adds compression pipeline (RFC 16).

use anyhow::Context;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct SkillManifest {
    pub id: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub requires_sandbox: bool,
    #[serde(default)]
    pub verified: bool,
}

pub fn load_skill(dir: &Path) -> anyhow::Result<SkillManifest> {
    let manifest_path = dir.join("skill.toml");
    let contents = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading skill manifest {}", manifest_path.display()))?;
    let manifest: SkillManifest = toml::from_str(&contents).context("parsing skill.toml")?;
    Ok(manifest)
}
