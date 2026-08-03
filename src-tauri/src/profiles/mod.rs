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

/// RFC 06 / RFC 28 §H.10 — A persisted per-profile configuration.
/// Stored as `<profile_root>/profile.toml`. The two §H fields drive
/// the reset-window notifications subsystem:
///   * `bail_out_threshold_secs` — how many seconds a `SpendLimitError`
///     retry loop may burn before `RetryPolicy::decide` returns
///     `GiveUp` (default 60, mirroring `RetryPolicy`'s default).
///   * `backup_profile_id` — when the user clicks "Switch Provider"
///     on a `SpendLimitErrorCard`, the HUD does
///     `opencode profile switch <backup_profile_id>`. `None` disables
///     the button (one-profile setups).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Profile {
    pub id: ProfileId,
    pub bail_out_threshold_secs: u64,
    pub backup_profile_id: Option<String>,
}

impl Profile {
    /// Default profile: `bail_out_threshold_secs = 60`,
    /// `backup_profile_id = None`. Used when `<profile_root>/profile.toml`
    /// is missing (first launch, fresh install).
    pub fn default_for(id: ProfileId) -> Self {
        Self {
            id,
            bail_out_threshold_secs: 60,
            backup_profile_id: None,
        }
    }

    /// Path to the persisted config inside a profile's root directory.
    fn config_path(root: &Path) -> PathBuf {
        root.join("profile.toml")
    }

    /// Load a `Profile` from `<root>/profile.toml`. Falls back to
    /// `default_for(id)` if the file is missing or malformed (best-effort
    /// — the file is informational, not authoritative, for the §H
    /// subsystem; the `RetryPolicy` defaults are enforced in code).
    pub fn load(root: &Path, id: ProfileId) -> anyhow::Result<Self> {
        let path = Self::config_path(root);
        if !path.exists() {
            return Ok(Self::default_for(id));
        }
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let parsed: Profile =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        if parsed.id != id {
            anyhow::bail!(
                "profile.toml at {} declares id `{}` but expected `{}`",
                path.display(),
                parsed.id,
                id
            );
        }
        Ok(parsed)
    }

    /// Persist this `Profile` to `<root>/profile.toml` (atomic write).
    pub fn save(&self, root: &Path) -> anyhow::Result<()> {
        let path = Self::config_path(root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let toml = toml::to_string(self).context("serializing profile.toml")?;
        std::fs::write(&path, toml).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_save_then_load_roundtrip_preserves_h_fields() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        let id = ProfileId::new("work");
        let p = Profile {
            id: id.clone(),
            bail_out_threshold_secs: 120,
            backup_profile_id: Some("personal".into()),
        };
        p.save(root).unwrap();
        let loaded = Profile::load(root, id.clone()).unwrap();
        assert_eq!(loaded, p);
        assert_eq!(loaded.bail_out_threshold_secs, 120);
        assert_eq!(loaded.backup_profile_id.as_deref(), Some("personal"));
    }

    #[test]
    fn profile_load_falls_back_to_default_when_file_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        let id = ProfileId::new("fresh");
        let p = Profile::load(root, id.clone()).unwrap();
        assert_eq!(p.id, id);
        assert_eq!(p.bail_out_threshold_secs, 60, "RFC 28 §H.10 default");
        assert!(p.backup_profile_id.is_none(), "default has no backup");
    }
}
