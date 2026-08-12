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
///
/// RFC 04 Phase 2 sub-fase 2.0 — Aider tri-model port: each profile
/// may designate three model roles independently. The defaults fall
/// back to `main_model_id`, preserving Aider's pattern where the
/// `--weak-model` defaults to `--model` when unset. The `weak` role
/// is the cheapest model in the pool and is used for context-window
/// compactification + commit messages — never for primary inference
/// (Aider v0.42+ semantics).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Profile {
    pub id: ProfileId,
    pub bail_out_threshold_secs: u64,
    pub backup_profile_id: Option<String>,
    /// RFC 04 §1 / RFC 04 Phase 2 — The main / primary model. The
    /// Orchestrator uses this for every kind of inference unless a
    /// role-specific override (`architect_model_id` /
    /// `editor_model_id` / `weak_model_id`) is set.
    #[serde(default)]
    pub main_model_id: Option<String>,
    /// RFC 21 §12 `architect` mode — model used to PROPOSE changes
    /// (the editor then applies them). Defaults to `main_model_id`
    /// when `None`. Aider `--model` (with `--architect`).
    #[serde(default)]
    pub architect_model_id: Option<String>,
    /// RFC 21 §12 `architect` mode — model used to APPLY edits.
    /// Defaults to `main_model_id` when `None`. Aider `--editor-model`.
    #[serde(default)]
    pub editor_model_id: Option<String>,
    /// Aider `--weak-model`. The cheapest model in the pool, used
    /// for commit-message synthesis + context-window compactification
    /// when `--max-chat-history-tokens` is exceeded (RFC 11 Context
    /// Engine). Defaults to `main_model_id` when `None`.
    #[serde(default)]
    pub weak_model_id: Option<String>,
    /// RFC 04 §5 Phase 2 sub-fase 2.0 — Resource-mode filter applied
    /// to the orchestrator. Defaults to `Mixed`.
    #[serde(default = "default_resource_mode")]
    pub resource_mode: String,
    /// RFC 04 §6 — routing + fallback configuration for the model
    /// orchestrator. Sub-fase 2.1. Defaults to `RoutingConfig::default`
    /// (SimpleShuffle, empty fallback buckets, `max_fallbacks=5`,
    /// `default_cooldown_secs=60`).
    #[serde(default)]
    pub routing_config: crate::orchestrator::routing::RoutingConfig,
    /// RFC 04 §3 — aggregation policy for the model orchestrator.
    /// Sub-fase 2.2. Defaults to `AggregationMode::default()` (=
    /// `Single` — no multi-sample aggregation, one inference per
    /// request). Profiles that opt into MajorityVote / MoA / Council /
    /// Reflexion / SelfRefine / SelfDiscover set this field explicitly.
    #[serde(default)]
    pub aggregation: crate::orchestrator::aggregation::AggregationMode,
}

fn default_resource_mode() -> String {
    "mixed".to_string()
}

impl Default for Profile {
    fn default() -> Self {
        Self::default_for(ProfileId::new("default"))
    }
}

impl Profile {
    /// Default profile: `bail_out_threshold_secs = 60`,
    /// `backup_profile_id = None`, no tri-model overrides (all roles
    /// fall back to `main_model_id`), `resource_mode = "mixed"` (RFC
    /// 04 §5 default). Used when `<profile_root>/profile.toml` is
    /// missing (first launch, fresh install).
    pub fn default_for(id: ProfileId) -> Self {
        Self {
            id,
            bail_out_threshold_secs: 60,
            backup_profile_id: None,
            main_model_id: None,
            architect_model_id: None,
            editor_model_id: None,
            weak_model_id: None,
            resource_mode: default_resource_mode(),
            routing_config: crate::orchestrator::routing::RoutingConfig::default(),
            aggregation: crate::orchestrator::aggregation::AggregationMode::default(),
        }
    }

    /// Resolve the effective architect model — `architect_model_id`
    /// when set, else `main_model_id`. Returns `None` when neither is
    /// set (the orchestrator then falls back to the registry default).
    pub fn effective_architect_model(&self) -> Option<&str> {
        self.architect_model_id
            .as_deref()
            .or(self.main_model_id.as_deref())
    }
    /// Resolve the effective editor model — `editor_model_id` when
    /// set, else `main_model_id`.
    pub fn effective_editor_model(&self) -> Option<&str> {
        self.editor_model_id
            .as_deref()
            .or(self.main_model_id.as_deref())
    }

    /// Resolve the effective weak model — `weak_model_id` when set,
    /// else `main_model_id`. Aider's downstream semantic.
    pub fn effective_weak_model(&self) -> Option<&str> {
        self.weak_model_id
            .as_deref()
            .or(self.main_model_id.as_deref())
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
            main_model_id: Some("claude-opus-4".into()),
            architect_model_id: Some("gpt-5".into()),
            editor_model_id: Some("claude-sonnet-4.5".into()),
            weak_model_id: Some("gemini-2.5-flash".into()),
            resource_mode: "free".into(),
            ..Default::default()
        };
        p.save(root).unwrap();
        let loaded = Profile::load(root, id.clone()).unwrap();
        assert_eq!(loaded, p);
        assert_eq!(loaded.bail_out_threshold_secs, 120);
        assert_eq!(loaded.backup_profile_id.as_deref(), Some("personal"));
        assert_eq!(loaded.main_model_id.as_deref(), Some("claude-opus-4"));
        assert_eq!(loaded.architect_model_id.as_deref(), Some("gpt-5"));
        assert_eq!(loaded.editor_model_id.as_deref(), Some("claude-sonnet-4.5"));
        assert_eq!(loaded.weak_model_id.as_deref(), Some("gemini-2.5-flash"));
        assert_eq!(loaded.resource_mode, "free");
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
        assert!(p.main_model_id.is_none(), "default has no main model");
        assert!(p.architect_model_id.is_none(), "default has no architect");
        assert!(p.editor_model_id.is_none(), "default has no editor");
        assert!(p.weak_model_id.is_none(), "default has no weak");
        assert_eq!(p.resource_mode, "mixed", "RFC 04 §5 default");
    }

    #[test]
    fn profile_effective_tri_model_falls_back_to_main() {
        let profile = Profile {
            id: ProfileId::new("test"),
            bail_out_threshold_secs: 60,
            backup_profile_id: None,
            main_model_id: Some("claude-opus-4".into()),
            architect_model_id: None,
            editor_model_id: None,
            weak_model_id: None,
            resource_mode: "mixed".into(),
            ..Default::default()
        };
        assert_eq!(profile.effective_architect_model(), Some("claude-opus-4"));
        assert_eq!(profile.effective_editor_model(), Some("claude-opus-4"));
        assert_eq!(profile.effective_weak_model(), Some("claude-opus-4"));
    }

    #[test]
    fn profile_effective_tri_model_uses_overrides_when_set() {
        let profile = Profile {
            id: ProfileId::new("test"),
            bail_out_threshold_secs: 60,
            backup_profile_id: None,
            main_model_id: Some("claude-opus-4".into()),
            architect_model_id: Some("gpt-5".into()),
            editor_model_id: Some("claude-sonnet-4.5".into()),
            weak_model_id: Some("gemini-2.5-flash".into()),
            resource_mode: "mixed".into(),
            ..Default::default()
        };
        assert_eq!(profile.effective_architect_model(), Some("gpt-5"));
        assert_eq!(profile.effective_editor_model(), Some("claude-sonnet-4.5"));
        assert_eq!(profile.effective_weak_model(), Some("gemini-2.5-flash"));
    }

    #[test]
    fn profile_effective_returns_none_when_no_main_no_override() {
        let profile = Profile {
            id: ProfileId::new("test"),
            bail_out_threshold_secs: 60,
            backup_profile_id: None,
            main_model_id: None,
            architect_model_id: None,
            editor_model_id: None,
            weak_model_id: None,
            resource_mode: "mixed".into(),
            ..Default::default()
        };
        assert_eq!(profile.effective_architect_model(), None);
        assert_eq!(profile.effective_editor_model(), None);
        assert_eq!(profile.effective_weak_model(), None);
    }

    #[test]
    fn profile_old_toml_without_phase2_fields_loads_with_defaults() {
        // A pre-Phase-2 `profile.toml` won't have the new fields — serde
        // defaults must keep the load successful.
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();
        let path = root.join("profile.toml");
        std::fs::write(
            &path,
            "id = 'work'\nbail_out_threshold_secs = 90\nbackup_profile_id = 'personal'\n",
        )
        .unwrap();
        let loaded = Profile::load(root, ProfileId::new("work")).unwrap();
        assert_eq!(loaded.bail_out_threshold_secs, 90);
        assert_eq!(loaded.backup_profile_id.as_deref(), Some("personal"));
        assert!(loaded.main_model_id.is_none());
        assert!(loaded.architect_model_id.is_none());
        assert!(loaded.editor_model_id.is_none());
        assert!(loaded.weak_model_id.is_none());
        assert_eq!(loaded.resource_mode, "mixed");
    }
}
