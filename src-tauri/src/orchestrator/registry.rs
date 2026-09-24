// Atlas OS — Model registry (RFC 04 §1, Phase 2 sub-fase 2.0).
//
// The registry holds the in-memory index of every `ModelDescriptor`
// and `Deployment` Atlas OS knows how to invoke. The schema-of-
// record is the JSON seed `assets/model_prices_and_context_window.json`
// (subset of BerriAI/litellm MIT-licensed `model_prices_and_context_window.json`)
// bundled into the binary via `include_str!` — single-binary safe per
// RFC 25 §11 and updateable without a rebuild via `atlas models refresh`
// (which writes the new JSON to `<profile_root>/model_prices.json` and
// hot-swaps the in-memory `Arc<Registry>` via `ArcSwap::store`).
//
// Per-deployment overrides (api_base, api_key env, cooldown, weights)
// live in SQLite tables `models` / `deployments` / `model_aliases` /
// `model_groups` (M20 migration). The SQL rows take precedence over
// the JSON seed for hot-swappable values; the JSON is authoritative
// for `tier`, `context_window`, `capabilities`, and cost figures
// (those change rarely and a binary bump is acceptable).
//
// Phase 2 sub-fase 2.0 only materialises the in-memory index +
// JSON-seed loader + the SQLite table shapes. The ArcSwap wiring +
// persistence reader/writer arrives in sub-fase 2.1 (routing policy)
// once the `model_invocations` migration (M21) lands alongside. For
// now the `Registry::from_seed_json` function is exercised by tests;
// the SQLite reader is stubbed with happy-path row counts.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::orchestrator::affinity::AffinityIndex;
use crate::orchestrator::provider::{
    Capability, Deployment, ModelDescriptor, Provider, ProviderWire, Tier,
};

/// Seed JSON root object — parsed once at boot via `serde_json::from_str`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistrySeed {
    #[serde(rename = "_meta")]
    pub meta: RegistrySeedMeta,
    pub models: Vec<ModelDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrySeedMeta {
    #[serde(default)]
    pub _comment: String,
    #[serde(default)]
    pub _license: String,
    #[serde(default)]
    pub _version: String,
    #[serde(default)]
    pub _schema: String,
}

impl RegistrySeedMeta {
    /// Sentinel for tests when a field is unimportant. Real seeds from
    /// `include_str!` are always populated.
    pub fn test_default() -> Self {
        Self {
            _comment: String::new(),
            _license: String::new(),
            _version: "test".into(),
            _schema: "atlas-os-registry-v1".into(),
        }
    }
}

/// In-memory index. Cheaply clonable because the inner maps are
/// `Arc`-shared — clones share the underlying allocations with
/// atomic refcounts, no deep copy.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    /// Hash by model id. Hashbrown would be faster but we already pull
    /// `indexmap` for ordered profiles, so `HashMap` from std is fine.
    pub by_id: HashMap<String, Arc<ModelDescriptor>>,
    /// Reverse index: alias → model_id. `~claude-opus-latest`,
    /// `opus`, `sonnet`, etc. Multiple aliases resolve to the same
    /// model — first one wins on duplicate keys.
    pub by_alias: HashMap<String, String>,
    /// Secondary index: provider wire-tag → list of model ids.
    /// Used by `local-only` / `free-only` mode filters (RFC 04 §5)
    /// to narrow the candidate pool before the routing strategy
    /// even starts its score function.
    pub by_provider: HashMap<ProviderWire, Vec<String>>,
    /// Per-deployment rows. `model_id → Vec<Deployment>`.
    /// Empty in a pure Json-seed Registry; populated by SQLite reader
    /// (sub-fase 2.1) and explicit `atlas models add-deployment`.
    pub deployments: HashMap<String, Vec<Arc<Deployment>>>,
    /// User-supplied custom backends. `id → Arc<dyn Config>` live trait
    /// objects, owned only here so that `Provider::Custom` variants can
    /// call `custom_config()` and reach the runtime object.
    pub custom_backends: HashMap<String, Arc<dyn crate::orchestrator::provider::Config>>,
    /// RFC 04 §8 sub-fase 2.4 — affinity cache. Cloning a `Registry`
    /// yields a handle sharing the underlying `ArcSwap`; the refresh
    /// task (orchestrator loop) stores a fresh map; route readers
    /// `load()` cheaply and deref to `&Arc<HashMap<...>>`.
    /// Empty by default; populate via `refresh_affinity_from_journal`.
    pub affinity: AffinityIndex,
}

impl Registry {
    /// Build the registry from a parsed `RegistrySeed`. The custom
    /// backend table is empty by default — backends are registered via
    /// `Registry::with_custom_backend` (in sub-fase 2.1).
    pub fn from_seed(seed: RegistrySeed) -> anyhow::Result<Self> {
        let mut reg = Self::default();
        for m in seed.models {
            let id = m.id.clone();
            for a in &m.aliases {
                reg.by_alias.entry(a.clone()).or_insert_with(|| id.clone());
            }
            reg.by_provider
                .entry(m.provider.clone())
                .or_default()
                .push(id.clone());
            reg.by_id.insert(id, Arc::new(m));
        }
        Ok(reg)
    }

    /// Parse the bundled seed JSON. Called at boot via
    /// `Registry::bundled_seed()` — the JSON is `include_str!`'d at
    /// compile time so there's zero filesystem I/O on the happy path;
    /// `atlas models refresh` may swap it at runtime (sub-fase 2.1).
    pub fn bundled_seed() -> &'static str {
        include_str!("assets/model_prices_and_context_window.json")
    }

    /// Build the registry from the bundled seed. Convenience entry
    /// point used by tests + future `AppState::bootstrap()` wiring
    /// (sub-fase 2.1).
    pub fn from_bundled_seed() -> anyhow::Result<Self> {
        let raw = Self::bundled_seed();
        let seed: RegistrySeed = serde_json::from_str(raw)?;
        Self::from_seed(seed)
    }

    /// Resolve a possibly-alias lookup to a concrete model id. Returns
    /// `None` when neither the alias nor the literal id is known. The
    /// returned `String` decouples the result from `&self` so callers
    /// can drop the registry borrow before looking up `get()`.
    pub fn resolve(&self, alias_or_id: &str) -> Option<String> {
        if self.by_id.contains_key(alias_or_id) {
            return Some(alias_or_id.to_string());
        }
        self.by_alias.get(alias_or_id).cloned()
    }

    pub fn get(&self, id: &str) -> Option<&Arc<ModelDescriptor>> {
        self.by_id.get(id)
    }

    /// All descriptors for a given `Provider` (filters by the runtime
    /// enum, not by `ProviderWire` — for custom backends the wire id
    /// is matched against the backends table).
    pub fn descriptors_for_provider(&self, p: &Provider) -> Vec<&Arc<ModelDescriptor>> {
        let wire = ProviderWire::from_runtime(p);
        self.by_provider
            .get(&wire)
            .map(|ids| ids.iter().filter_map(|id| self.by_id.get(id)).collect())
            .unwrap_or_default()
    }

    /// Mode filter (RFC 04 §5). `local-only` returns only `Tier::Local`
    /// models; `free-only` returns `Tier::Free` + `Tier::FreeTier`;
    /// `mixto` returns everything.
    pub fn filter_by_resource_mode(&self, mode: ResourceMode) -> Vec<&Arc<ModelDescriptor>> {
        self.by_id
            .values()
            .filter(|d| match mode {
                ResourceMode::Local => matches!(d.tier, Tier::Local),
                ResourceMode::Free => matches!(d.tier, Tier::Free | Tier::FreeTier),
                ResourceMode::Mixed => true,
            })
            .collect()
    }

    /// Capability filter — `vision`-only prompts route only to models
    /// carrying `Capability::Vision`. Used by sub-fase 2.3 auto-routing
    /// classifier (the tag pre-filter hot-path reduces the search space
    /// before the classifier even runs).
    pub fn filter_by_capability(&self, cap: Capability) -> Vec<&Arc<ModelDescriptor>> {
        self.by_id
            .values()
            .filter(|d| d.capabilities.contains(&cap))
            .collect()
    }

    /// RFC 04 §8 sub-fase 2.4 — re-crunch the affinity cache from
    /// journal telemetry and atomically swap it into `self.affinity`.
    /// The orchestrator refresh loop (or `atlas models refresh`)
    /// invokes this after a sampling window elapses.
    ///
    /// Reads `window` most-recent `model_invocations` rows, joins
    /// `task_classifier_decisions` for the predicted task type, groups
    /// by `(task_type, model_id)`, persists the fresh snapshot back to
    /// `model_affinity_cache` (M23 mirror), and stores an in-memory
    /// snapshot for hot-path reads.
    pub fn refresh_affinity_from_journal(
        &self,
        journal: &crate::journal::Journal,
        window: u32,
    ) -> anyhow::Result<()> {
        use crate::orchestrator::classifier::TaskType;
        let rows = journal.read_affinity(window)?;
        journal.upsert_affinity_rows(&rows)?;
        let mut map: HashMap<(TaskType, String), crate::orchestrator::affinity::AffinityRow> =
            HashMap::with_capacity(rows.len());
        for row in rows {
            map.insert((row.task_type, row.model_id.clone()), row);
        }
        self.affinity.store_all(map);
        Ok(())
    }
}

/// RFC 04 §5 mode flag — what subset of tiers the orchestrator is
/// allowed to invoke. The `Profile` carries this; the `RoutingStrategy`
/// (sub-fase 2.1) reads it as part of its `RoutingConfig`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceMode {
    /// Only `Tier::Local` — Ollama / LmStudio / llama.cpp-server.
    Local,
    /// `Tier::Free` + `Tier::FreeTier` — Groq / Cerebras / SambaNova /
    /// Nvidia NIM free tier / Cloudflare Workers AI / HF Inference /
    /// GitHub Models free / Gemini Flash.
    Free,
    /// All tiers including `Paid` / `Frontier` / `Experimental`.
    Mixed,
}

impl Default for ResourceMode {
    /// Defaults to `Mixed` per RFC 04 §5 ("mixto" is the documented
    /// default). Callers wanting a stricter mode set it via `Profile`.
    fn default() -> Self {
        Self::Mixed
    }
}

impl ResourceMode {
    /// Display name shown in the HUD profile card.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local-only",
            Self::Free => "free-only",
            Self::Mixed => "mixed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "local-only" | "local_only" | "local" => Self::Local,
            "free-only" | "free_only" | "free" => Self::Free,
            "mixed" | "mixto" => Self::Mixed,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::provider::{Capability, ModelDescriptor, ProviderWire, Tier};

    fn test_descriptor(id: &str, provider: ProviderWire, tier: Tier) -> ModelDescriptor {
        ModelDescriptor {
            id: id.into(),
            provider,
            display_name: id.into(),
            tier,
            context_window: 200_000,
            max_output_tokens: 8_000,
            capabilities: vec![Capability::Text, Capability::FunctionCalling],
            input_cost_per_1m_tokens: 1.0,
            output_cost_per_1m_tokens: 2.0,
            cache_read_cost_per_1m_tokens: 0.1,
            latency_ms_p50: 500,
            aliases: vec![],
        }
    }

    fn test_seed() -> RegistrySeed {
        let models = vec![
            {
                let mut d =
                    test_descriptor("claude-opus-4", ProviderWire::Anthropic, Tier::Frontier);
                d.aliases = vec!["opus".into(), "~claude-opus-latest".into()];
                d
            },
            {
                let mut d = test_descriptor("gemini-flash", ProviderWire::Gemini, Tier::FreeTier);
                d.aliases = vec!["flash".into()];
                d
            },
            test_descriptor("ollama-llama", ProviderWire::Ollama, Tier::Local),
            test_descriptor("groq-llama", ProviderWire::Groq, Tier::FreeTier),
        ];
        RegistrySeed {
            meta: RegistrySeedMeta::test_default(),
            models,
        }
    }

    #[test]
    fn registry_from_seed_builds_indexes() {
        let reg = Registry::from_seed(test_seed()).unwrap();
        assert!(reg.by_id.contains_key("claude-opus-4"));
        assert!(reg.by_id.contains_key("gemini-flash"));
        assert_eq!(
            reg.by_alias.get("opus").map(|s| s.as_str()),
            Some("claude-opus-4")
        );
        assert_eq!(
            reg.by_alias.get("~claude-opus-latest").map(|s| s.as_str()),
            Some("claude-opus-4")
        );
        assert_eq!(
            reg.by_alias.get("flash").map(|s| s.as_str()),
            Some("gemini-flash")
        );
        let anthropic_ids = reg.by_provider.get(&ProviderWire::Anthropic).unwrap();
        assert_eq!(anthropic_ids, &vec!["claude-opus-4".to_string()]);
        let ollama_ids = reg.by_provider.get(&ProviderWire::Ollama).unwrap();
        assert_eq!(ollama_ids.len(), 1);
    }

    #[test]
    fn registry_resolve_alias_returns_model_id() {
        let reg = Registry::from_seed(test_seed()).unwrap();
        assert_eq!(reg.resolve("opus").as_deref(), Some("claude-opus-4"));
        assert_eq!(
            reg.resolve("~claude-opus-latest").as_deref(),
            Some("claude-opus-4")
        );
        assert_eq!(reg.resolve("flash").as_deref(), Some("gemini-flash"));
        assert_eq!(
            reg.resolve("claude-opus-4").as_deref(),
            Some("claude-opus-4")
        );
        assert_eq!(reg.resolve("nonexistent"), None);
    }

    #[test]
    fn registry_first_alias_wins_on_duplicate() {
        let mut models = test_seed().models;
        let mut d = test_descriptor("dup-target", ProviderWire::OpenAI, Tier::Paid);
        d.aliases = vec!["dup".into()];
        models.push(d);
        let mut d2 = test_descriptor("dup-others", ProviderWire::OpenAI, Tier::Paid);
        d2.aliases = vec!["dup".into()];
        models.push(d2);
        let seed = RegistrySeed {
            meta: RegistrySeedMeta::test_default(),
            models,
        };
        let reg = Registry::from_seed(seed).unwrap();
        let winner = reg.by_alias.get("dup").unwrap();
        assert_eq!(*winner, "dup-target");
        let loser_models = ["dup-others"];
        for _s in &loser_models {
            assert_ne!(
                reg.by_alias.get("dup").map(|s| s.as_str()),
                Some("dup-others")
            );
        }
    }

    #[test]
    fn registry_from_bundled_seed_loads_18_models() {
        let reg = Registry::from_bundled_seed().unwrap();
        assert!(reg.by_id.len() >= 18, "bundled seed has 18+ models");
        assert!(reg.by_alias.contains_key("opus"));
        assert!(reg.by_alias.contains_key("sonnet"));
        assert!(reg.by_alias.contains_key("gpt-flag"));
        assert!(reg.by_alias.contains_key("gemini-pro"));
        assert!(reg.by_alias.contains_key("local-llama"));
        let anthropic = reg.by_provider.get(&ProviderWire::Anthropic).unwrap();
        assert!(anthropic.len() >= 3, "anthropic has 3+ models bundled");
        let locals = reg.by_provider.get(&ProviderWire::Ollama).unwrap();
        assert!(locals.len() >= 2, "ollama has 2+ models bundled");
    }

    #[test]
    fn registry_filter_by_resource_mode_local() {
        let reg = Registry::from_seed(test_seed()).unwrap();
        let locals = reg.filter_by_resource_mode(ResourceMode::Local);
        let local_ids: Vec<&str> = locals.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(local_ids, vec!["ollama-llama"]);
    }

    #[test]
    fn registry_filter_by_resource_mode_free() {
        let reg = Registry::from_seed(test_seed()).unwrap();
        let frees = reg.filter_by_resource_mode(ResourceMode::Free);
        let free_ids: Vec<&str> = frees.iter().map(|d| d.id.as_str()).collect();
        let mut expected = vec!["gemini-flash", "groq-llama"];
        expected.sort();
        let mut got = free_ids.clone();
        got.sort();
        assert_eq!(got, expected);
    }

    #[test]
    fn registry_filter_by_resource_mode_mixed_returns_all() {
        let reg = Registry::from_seed(test_seed()).unwrap();
        let all = reg.filter_by_resource_mode(ResourceMode::Mixed);
        assert_eq!(all.len(), reg.by_id.len());
    }

    #[test]
    fn registry_filter_by_capability_vision() {
        let reg = Registry::from_bundled_seed().unwrap();
        let _ = reg.filter_by_capability(Capability::Vision);
        let vision = reg.filter_by_capability(Capability::Vision);
        assert!(vision.iter().any(|d| d.id == "claude-opus-4"));
        assert!(vision.iter().any(|d| d.id == "gpt-5"));
        assert!(vision.iter().any(|d| d.id == "gemini-2.5-pro"));
        let ollama_llama = reg.get("ollama-llama-3.3-70b").unwrap();
        assert!(!ollama_llama.capabilities.contains(&Capability::Vision));
    }

    #[test]
    fn resource_mode_default_is_mixed() {
        assert_eq!(ResourceMode::default(), ResourceMode::Mixed);
    }

    #[test]
    fn resource_mode_as_str_roundtrips_from_str() {
        for m in [ResourceMode::Local, ResourceMode::Free, ResourceMode::Mixed] {
            let s = m.as_str();
            assert_eq!(ResourceMode::parse(s), Some(m));
        }
        assert_eq!(ResourceMode::parse("local_only"), Some(ResourceMode::Local));
        assert_eq!(ResourceMode::parse("mixto"), Some(ResourceMode::Mixed));
        assert_eq!(ResourceMode::parse("unknown"), None);
    }

    #[test]
    fn registry_seed_meta_serde_preserves_underscore_prefixed_fields() {
        let meta = RegistrySeedMeta::test_default();
        let json = serde_json::to_string(&meta).unwrap();
        let back: RegistrySeedMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back, meta);
    }
}
