// OpenCode OS — Profiles (RFC 25 §4, RFC 22 §1 Hermes pattern).
// A profile == a worktree-bound workspace with its own Journal, models,
// skills cache, embeddings. Switching profile rewrites `~/.opencode/current`.

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileId(pub String);

impl std::fmt::Display for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl ProfileId {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self(name.into())
    }
}

/// Resolve the filesystem root for a given profile.
/// Layout:
///   Linux  : $HOME/.opencode/profiles/<id>/
///   macOS  : $HOME/.opencode/profiles/<id>/
///   Windows: %USERPROFILE%\.opencode\profiles\<id>\
pub fn resolve_root(profile: &ProfileId) -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().context("could not resolve user home directory")?;
    let root = home.join(".opencode").join("profiles").join(&profile.0);
    std::fs::create_dir_all(&root)
        .with_context(|| format!("creating profile dir {}", root.display()))?;
    Ok(root)
}

/// List all profiles found on disk.
pub fn list_all() -> anyhow::Result<Vec<ProfileId>> {
    let home = dirs::home_dir().context("could not resolve user home directory")?;
    let profiles_dir = home.join(".opencode/profiles");
    if !profiles_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&profiles_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                out.push(ProfileId(name.to_owned()));
            }
        }
    }
    Ok(out)
}

pub fn default_profile_dir() -> anyhow::Result<PathBuf> {
    resolve_root(&ProfileId::default())
}

/// Path to `~/.opencode/current` — the persistent pointer used by both the
/// CLI (`opencode` binary uses the active profile on every invocation) and
/// the desktop shell (boot ups select the same profile the CLI last set).
fn current_file() -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().context("could not resolve user home directory")?;
    Ok(home.join(".opencode/current"))
}

/// Return the persisted current profile id, or `default` if the pointer has
/// not been set yet (first launch, fresh install).
pub fn current() -> ProfileId {
    match current_file() {
        Ok(path) => std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(ProfileId(trimmed.to_owned()))
                }
            })
            .unwrap_or_default(),
        Err(_) => ProfileId::default(),
    }
}

/// Persist `id` as the active profile for the next `opencode` invocation
/// and the next desktop launch. Best-effort: logs a warning if the write
/// fails so the rest of the switch flow can still proceed.
pub fn set_current(id: &ProfileId) -> anyhow::Result<()> {
    let path = current_file()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&path, &id.0).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

pub fn extract<P: AsRef<Path>>(p: P) -> Option<ProfileId> {
    let comps = p.as_ref().components().collect::<Vec<_>>();
    comps
        .iter()
        .rev()
        .nth(1)
        .and_then(|c| c.as_os_str().to_str())
        .map(ProfileId::new)
}
