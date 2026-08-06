// OpenCode OS — Orchestrator providers (RFC 04 §1, Phase 2 sub-fase 2.0).
//
// The `Provider` enum and `Config` trait are pattern-ported from
// `64bit/async-openai` (MIT, Copyright 64bit) — the canonical Rust
// pattern for `Box<dyn Config>` polymorphism over OpenAI-compatible
// providers. We don't depend on `async-openai` directly because this
// crate needs to also speak Anthropic / Gemini NATIVE wire formats
// (OpenAI-compat proxies lose `prompt_caching`, `tool_use` shape,
// `thinking` reasoning, and other provider-specific affordances); the
// pattern's shape however is faithfully preserved (trait Config +
// enum with Custom(Arc<dyn Config>) variant for user-supplied
// backends) so the box-dispatch call-site is ergonomic.
//
// Phase 2 sub-fase 2.0 ONLY materialises the enum, the trait surface,
// and the data descriptors (`ModelDescriptor`, `Deployment`,
// `Capability`, `Tier`). The actual HTTP request plumbing arrives in
// sub-fase 2.0.5 (tool-call normalization) + 2.1 (routing policy) —
// callers there match exhaustively on `Provider` for variant-specific
// message shape and reach the wire via the shared `reqwest::Client`
// already present in the workspace.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// All first-class providers OpenCode OS knows how to reach natively.
/// Variants mirror RFC 04 §1 plus RFC 25 §3.8's 14-provider list; the
/// `Custom` variant holds any user-defined backend registered via
/// `opencode models register-custom`.
///
/// `Provider` is `Clone + PartialEq` but NOT `Eq + Hash` because the
/// `Custom(Arc<dyn Config>)` variant holds a trait object that can't
/// be hashed or equated beyond identity. Callers that need a stable
/// hashable key should use `ProviderWire` (the serialisable form) —
/// that's exactly why the wire form exists separately from the
/// runtime form. Cooldown maps (sub-fase 2.0.5) key on `ProviderWire`
/// rather than `Provider`.
#[derive(Debug, Clone)]
pub enum Provider {
    OpenAI,
    Anthropic,
    Gemini,
    VertexAI,
    Bedrock,
    Azure,
    DeepSeek,
    Mistral,
    Groq,
    Cerebras,
    Sambanova,
    NvidiaNim,
    CloudflareWorkersAi,
    HuggingFaceInference,
    GitHubModels,
    OpenRouter,
    LiteLLM,
    Ollama,
    LmStudio,
    LlamaCppServer,
    /// User-defined backend. The `Arc<dyn Config>` holds a trait
    /// object supplied at runtime by `opencode models register-custom`
    /// — the orchestrator dispatches via `Provider::custom_config()`
    /// rather than `match` arms (no static dispatch available).
    Custom(Arc<dyn Config>),
}

impl PartialEq for Provider {
    /// Compares builtin variants by name; `Custom` variants compare
    /// equal iff their `Config::id()`s match (the trait object pointer
    /// can change across registry reloads while the id remains stable).
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::OpenAI, Self::OpenAI)
            | (Self::Anthropic, Self::Anthropic)
            | (Self::Gemini, Self::Gemini)
            | (Self::VertexAI, Self::VertexAI)
            | (Self::Bedrock, Self::Bedrock)
            | (Self::Azure, Self::Azure)
            | (Self::DeepSeek, Self::DeepSeek)
            | (Self::Mistral, Self::Mistral)
            | (Self::Groq, Self::Groq)
            | (Self::Cerebras, Self::Cerebras)
            | (Self::Sambanova, Self::Sambanova)
            | (Self::NvidiaNim, Self::NvidiaNim)
            | (Self::CloudflareWorkersAi, Self::CloudflareWorkersAi)
            | (Self::HuggingFaceInference, Self::HuggingFaceInference)
            | (Self::GitHubModels, Self::GitHubModels)
            | (Self::OpenRouter, Self::OpenRouter)
            | (Self::LiteLLM, Self::LiteLLM)
            | (Self::Ollama, Self::Ollama)
            | (Self::LmStudio, Self::LmStudio)
            | (Self::LlamaCppServer, Self::LlamaCppServer) => true,
            (Self::Custom(a), Self::Custom(b)) => a.id() == b.id(),
            _ => false,
        }
    }
}

impl Provider {
    /// Wire-protocol id used in the `models.provider` SQLite column and
    /// the JSON seed (`assets/model_prices_and_context_window.json`).
    /// Stable string — never reused across variants.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::VertexAI => "vertex_ai",
            Self::Bedrock => "bedrock",
            Self::Azure => "azure",
            Self::DeepSeek => "deepseek",
            Self::Mistral => "mistral",
            Self::Groq => "groq",
            Self::Cerebras => "cerebras",
            Self::Sambanova => "sambanova",
            Self::NvidiaNim => "nvidia_nim",
            Self::CloudflareWorkersAi => "cloudflare_workers_ai",
            Self::HuggingFaceInference => "hf_inference",
            Self::GitHubModels => "github_models",
            Self::OpenRouter => "openrouter",
            Self::LiteLLM => "litellm",
            Self::Ollama => "ollama",
            Self::LmStudio => "lm_studio",
            Self::LlamaCppServer => "llama_cpp_server",
            Self::Custom(_) => "custom",
        }
    }

    /// Parse the wire id back into a `Provider`. `Custom` variants
    /// cannot be round-tripped through this function (the `Arc<dyn
    /// Config>` is runtime-only) — `"custom"` returns `None` and
    /// callers should consult the `ModelRegistry.custom_backends()`
    /// table for the per-id trait object.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "openai" => Self::OpenAI,
            "anthropic" => Self::Anthropic,
            "gemini" => Self::Gemini,
            "vertex_ai" => Self::VertexAI,
            "bedrock" => Self::Bedrock,
            "azure" => Self::Azure,
            "deepseek" => Self::DeepSeek,
            "mistral" => Self::Mistral,
            "groq" => Self::Groq,
            "cerebras" => Self::Cerebras,
            "sambanova" => Self::Sambanova,
            "nvidia_nim" => Self::NvidiaNim,
            "cloudflare_workers_ai" => Self::CloudflareWorkersAi,
            "hf_inference" => Self::HuggingFaceInference,
            "github_models" => Self::GitHubModels,
            "openrouter" => Self::OpenRouter,
            "litellm" => Self::LiteLLM,
            "ollama" => Self::Ollama,
            "lm_studio" => Self::LmStudio,
            "llama_cpp_server" => Self::LlamaCppServer,
            _ => return None,
        })
    }

    /// `true` for variants whose traffic terminates on a model
    /// running in the user's own machine. Used by RFC 04 §5's
    /// `local-only` mode filter. `Custom` defaults to `false` — the
    /// operator declares `kind=local` at registration time and the
    /// `ModelDescriptor` carries the truth for routing decisions.
    pub const fn is_local_builtin(&self) -> bool {
        matches!(self, Self::Ollama | Self::LmStudio | Self::LlamaCppServer)
    }

    /// `true` for variants that have a free tier by default (RFC 04 §5
    /// `free-only` mode filter). This is the static, build-time truth;
    /// per-deployment overrides (e.g. paid Cerebras account) live in
    /// the `Deployment` row.
    pub const fn is_free_tier_builtin(&self) -> bool {
        matches!(
            self,
            Self::Groq
                | Self::Cerebras
                | Self::Sambanova
                | Self::NvidiaNim
                | Self::CloudflareWorkersAi
                | Self::HuggingFaceInference
                | Self::GitHubModels
                | Self::Ollama
                | Self::LmStudio
                | Self::LlamaCppServer
        )
    }

    /// Access the user-supplied `Config` for `Custom` variants. Other
    /// variants return `None` — they have hardcoded transport paths
    /// that don't need a trait object.
    pub fn custom_config(&self) -> Option<&dyn Config> {
        match self {
            Self::Custom(c) => Some(c.as_ref()),
            _ => None,
        }
    }
}

/// User-supplied backend contract — modelled on `async-openai::Config`.
/// The `id()` is stable and unique within a `ModelRegistry`; it backs
/// both the `models.id` SQLite primary key and `Provider::Custom`'s
/// `as_str()` output (`"custom:" + id()`).
///
/// The trait is intentionally narrow in sub-fase 2.0: only identity +
/// base URL + auth header. Sub-fase 2.0.5 (tool-call normalization)
/// extends it with `ToolCall` shape mapping; sub-fase 2.1 (routing
/// policy) adds `TokenSampler` hooks; sub-fase 2.4 plumbs `cooldown`
/// overrides.
pub trait Config: Send + Sync + std::fmt::Debug {
    /// Stable unique id, e.g. `"my-internal-vllm"`.
    fn id(&self) -> &str;

    /// Base URL, e.g. `https://internal-vllm.corp/v1`. The orchestrator
    /// appends provider-specific suffixes (e.g. `/chat/completions`
    /// for OpenAI-compat custom backends).
    fn base_url(&self) -> &str;

    /// Optional auth header tuple. `None` means anonymous (local
    /// servers, IP-allowlisted endpoints).
    fn auth_header(&self) -> Option<(&'static str, String)> {
        None
    }

    /// `true` if this backend is reachable from the user's machine
    /// (i.e. not behind a VPN the orchestrator can't detect). The
    /// registry uses this to filter unreachable custom backends from
    /// the routing pool.
    fn is_reachable(&self) -> bool {
        true
    }
}

/// Capability flags a model/deployment can carry. Mirrors RFC 04 §1
/// `capabilities` array + `capability_tags` field. Bitflags would be
/// marginally faster but `enumset`/`bitflags` add dep surface; the
/// registry stays in serde-land for JSON seed round-trip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Text,
    Vision,
    Audio,
    ImageGeneration,
    FunctionCalling,
    /// Anthropic-style `tool_use` (also Gemini `function_call`).
    /// Distinct from `FunctionCalling` (OpenAI `function_calling`)
    /// because the wire shapes differ — sub-fase 2.0.5 normalizes
    /// between them at the boundary.
    ToolUse,
    /// OpenAI `reasoning_effort` / Anthropic `thinking` / Gemini
    /// `thinkingConfig` — sub-fase 2.0.5 trait-shapes the per-provider
    /// knobs.
    Reasoning,
    /// Anthropic prompt-cache (`cache_control: ephemeral`). OpenAI's
    /// auto prompt-cache is provider-side implicit; we mark OpenAI
    /// models separately if/when they expose explicit markers.
    PromptCaching,
    /// JSON mode (`response_format: { "type": "json_object" }`).
    JsonMode,
    /// Parallel tool calls (multiple `tool_calls` in one turn).
    ParallelToolCalls,
}

/// Pricing tier from RFC 04 §1. The orchestrator's `free-only` mode
/// filter accepts `Free` and `FreeTier`; `mixto` accepts any.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Free,
    FreeTier,
    Paid,
    Frontier,
    /// Local models hosted on user hardware — no per-token cost.
    Local,
    Experimental,
}

/// Logical model descriptor (RFC 04 §1). One provider can host many
/// `ModelDescriptor`s (e.g. Anthropic hosts `claude-opus-4` and
/// `claude-sonnet-4` — two descriptors, one provider).
///
/// This is the *logical* model — what the LLM can do, at what cost.
/// A `Deployment` is *where that model is reached* (API key, region,
/// endpoint) and is tracked separately because LiteLLM's split
/// (`model_name` aggregates N `litellm_params`) is the right one:
/// cooldown is per-deployment, not per-model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub id: String,
    pub provider: ProviderWire,
    pub display_name: String,
    pub tier: Tier,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub capabilities: Vec<Capability>,
    pub input_cost_per_1m_tokens: f64,
    pub output_cost_per_1m_tokens: f64,
    pub cache_read_cost_per_1m_tokens: f64,
    pub latency_ms_p50: u32,
    /// Aliases that should resolve to this descriptor (RFC 04 §10
    /// "Latest-aliases port"). E.g. `~claude-opus-latest` →
    /// `claude-opus-4`. The registry maintains the reverse map in
    /// `model_aliases`.
    pub aliases: Vec<String>,
}

/// Wire-format for `Provider` in JSON seed / SQLite storage. Custom
/// variants can't round-trip through serde (Arc<dyn Config> isn't
/// Serialize), so we store the builtins cleanly and use a
/// `Custom(String)` tag for the registry to bind back to the in-memory
/// `Arc<dyn Config>` table.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderWire {
    #[serde(rename = "openai")]
    OpenAI,
    Anthropic,
    Gemini,
    #[serde(rename = "vertex_ai")]
    VertexAI,
    Bedrock,
    Azure,
    #[serde(rename = "deepseek")]
    DeepSeek,
    Mistral,
    Groq,
    Cerebras,
    Sambanova,
    #[serde(rename = "nvidia_nim")]
    NvidiaNim,
    CloudflareWorkersAi,
    HuggingFaceInference,
    #[serde(rename = "github_models")]
    GitHubModels,
    #[serde(rename = "openrouter")]
    OpenRouter,
    #[serde(rename = "litellm")]
    LiteLLM,
    Ollama,
    #[serde(rename = "lm_studio")]
    LmStudio,
    LlamaCppServer,
    /// `id` is the `Config::id()` of a user-registered backend. The
    /// `ModelRegistry.custom_backends()` map holds the live trait
    /// object; serialisation persists only the id string so future
    /// boots can reattach.
    Custom(String),
}

impl ProviderWire {
    /// Convert a runtime `Provider` to its wire form. The `Arc<dyn
    /// Config>` is dropped to its `id()` — the live object is owned by
    /// the `ModelRegistry` at runtime, not by the wire form.
    pub fn from_runtime(p: &Provider) -> Self {
        match p {
            Provider::OpenAI => Self::OpenAI,
            Provider::Anthropic => Self::Anthropic,
            Provider::Gemini => Self::Gemini,
            Provider::VertexAI => Self::VertexAI,
            Provider::Bedrock => Self::Bedrock,
            Provider::Azure => Self::Azure,
            Provider::DeepSeek => Self::DeepSeek,
            Provider::Mistral => Self::Mistral,
            Provider::Groq => Self::Groq,
            Provider::Cerebras => Self::Cerebras,
            Provider::Sambanova => Self::Sambanova,
            Provider::NvidiaNim => Self::NvidiaNim,
            Provider::CloudflareWorkersAi => Self::CloudflareWorkersAi,
            Provider::HuggingFaceInference => Self::HuggingFaceInference,
            Provider::GitHubModels => Self::GitHubModels,
            Provider::OpenRouter => Self::OpenRouter,
            Provider::LiteLLM => Self::LiteLLM,
            Provider::Ollama => Self::Ollama,
            Provider::LmStudio => Self::LmStudio,
            Provider::LlamaCppServer => Self::LlamaCppServer,
            Provider::Custom(c) => Self::Custom(c.id().to_owned()),
        }
    }

    /// Stable JSON tag (mirrors `Provider::as_str` but for serde).
    pub fn as_str(&self) -> &str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::VertexAI => "vertex_ai",
            Self::Bedrock => "bedrock",
            Self::Azure => "azure",
            Self::DeepSeek => "deepseek",
            Self::Mistral => "mistral",
            Self::Groq => "groq",
            Self::Cerebras => "cerebras",
            Self::Sambanova => "sambanova",
            Self::NvidiaNim => "nvidia_nim",
            Self::CloudflareWorkersAi => "cloudflare_workers_ai",
            Self::HuggingFaceInference => "hf_inference",
            Self::GitHubModels => "github_models",
            Self::OpenRouter => "openrouter",
            Self::LiteLLM => "litellm",
            Self::Ollama => "ollama",
            Self::LmStudio => "lm_studio",
            Self::LlamaCppServer => "llama_cpp_server",
            Self::Custom(id) => id.as_str(),
        }
    }
}

/// One concrete deployment of a `ModelDescriptor` — i.e. a specific
/// `api_key` + `api_base` + region that serves the model. RFC 04 §1's
/// `api_keys` multi-key pooling maps to N `Deployment`s sharing one
/// `model_id`. Mirrors LiteLLM's `model_name → N litellm_params` split.
///
/// Cooldown (sub-fase 2.0.5) is per-`Deployment`, not per-model — when
/// one key 429s we route to other deployments before falling back to
/// another model group (LiteLLM `enable_weighted_failover`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deployment {
    pub id: String,
    pub model_id: String,
    /// Display hint for HUD: "Anthropic #2" / "Ollama local" / "vLLM corp".
    pub label: String,
    pub api_base: String,
    pub region: Option<String>,
    /// Per-deployment priority within the same `model_id`. Lower wins
    /// when weights tie in `RoutingStrategy::SimpleShuffle`.
    pub priority: u32,
    /// Sampling weight for `SimpleShuffle` / `LatencyBased` (LiteLLM).
    /// 0 means "drain" — registry keeps the row for audit but routing
    /// skips it. Defaults to 1.
    pub weight: f32,
    pub tokens_per_minute: Option<u32>,
    pub requests_per_minute: Option<u32>,
    pub max_parallel: Option<u32>,
    /// `Some(env_var_name)` — the orchestrator reads the API key from
    /// `std::env::var(env_var_name)` at first use and caches. Encrypting
    /// at-rest in SQLite is a Phase 3 concern (RFC 25 §6 OS keychain).
    pub api_key_env: Option<String>,
}

impl Deployment {
    /// Default `weight=1.0`, no region, no rate caps. Useful in tests
    /// and as a starting point for `opencode models add`.
    pub fn new(model_id: impl Into<String>, api_base: impl Into<String>) -> Self {
        let model_id = model_id.into();
        let api_base = api_base.into();
        let hash = {
            use sha2::Digest;
            let digest = sha2::Sha256::digest(api_base.as_bytes());
            hex::encode(digest)
        };
        let id = format!("{}-{}", model_id, sha2_short(&hash));
        Self {
            id,
            model_id,
            label: String::new(),
            api_base,
            region: None,
            priority: 0,
            weight: 1.0,
            tokens_per_minute: None,
            requests_per_minute: None,
            max_parallel: None,
            api_key_env: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_api_key_env(mut self, env: impl Into<String>) -> Self {
        self.api_key_env = Some(env.into());
        self
    }
}

fn sha2_short(s: &str) -> String {
    s.chars().take(8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct DummyConfig {
        id: String,
        base: String,
    }

    impl Config for DummyConfig {
        fn id(&self) -> &str {
            &self.id
        }
        fn base_url(&self) -> &str {
            &self.base
        }
    }

    #[test]
    fn provider_as_str_roundtrips_for_builtins() {
        for p in [
            Provider::OpenAI,
            Provider::Anthropic,
            Provider::Gemini,
            Provider::VertexAI,
            Provider::Bedrock,
            Provider::Azure,
            Provider::DeepSeek,
            Provider::Mistral,
            Provider::Groq,
            Provider::Cerebras,
            Provider::Sambanova,
            Provider::NvidiaNim,
            Provider::CloudflareWorkersAi,
            Provider::HuggingFaceInference,
            Provider::GitHubModels,
            Provider::OpenRouter,
            Provider::LiteLLM,
            Provider::Ollama,
            Provider::LmStudio,
            Provider::LlamaCppServer,
        ] {
            let s = p.as_str();
            assert_eq!(Provider::parse(s), Some(p.clone()), "roundtrip {s}");
        }
    }

    #[test]
    fn provider_custom_returns_none_from_from_str() {
        assert_eq!(Provider::parse("custom"), None);
    }

    #[test]
    fn provider_custom_config_yields_trait_object() {
        let cfg = Arc::new(DummyConfig {
            id: "vllm".into(),
            base: "http://localhost:8001/v1".into(),
        }) as Arc<dyn Config>;
        let p = Provider::Custom(cfg.clone());
        assert_eq!(p.custom_config().map(|c| c.id()), Some("vllm"));
        assert_eq!(
            p.custom_config().map(|c| c.base_url()),
            Some("http://localhost:8001/v1")
        );
    }

    #[test]
    fn provider_is_local_builtin_truth_table() {
        assert!(Provider::Ollama.is_local_builtin());
        assert!(Provider::LmStudio.is_local_builtin());
        assert!(Provider::LlamaCppServer.is_local_builtin());
        assert!(!Provider::OpenAI.is_local_builtin());
        assert!(!Provider::Custom(Arc::new(DummyConfig {
            id: "x".into(),
            base: "x".into(),
        }) as Arc<dyn Config>)
        .is_local_builtin());
    }

    #[test]
    fn provider_is_free_tier_builtin_truth_table() {
        assert!(Provider::Groq.is_free_tier_builtin());
        assert!(Provider::Cerebras.is_free_tier_builtin());
        assert!(Provider::Ollama.is_free_tier_builtin());
        assert!(!Provider::OpenAI.is_free_tier_builtin());
        assert!(!Provider::Anthropic.is_free_tier_builtin());
        assert!(!Provider::Bedrock.is_free_tier_builtin());
    }

    #[test]
    fn provider_wire_roundtrips_via_serde() {
        let wire = ProviderWire::OpenAI;
        let json = serde_json::to_string(&wire).unwrap();
        assert_eq!(json, "\"openai\"");
        let back: ProviderWire = serde_json::from_str("\"openai\"").unwrap();
        assert_eq!(back, wire);
    }

    #[test]
    fn provider_wire_from_runtime_strips_custom_to_id() {
        let cfg = Arc::new(DummyConfig {
            id: "vllm".into(),
            base: "http://x".into(),
        }) as Arc<dyn Config>;
        let p = Provider::Custom(cfg);
        let wire = ProviderWire::from_runtime(&p);
        assert_eq!(wire, ProviderWire::Custom("vllm".to_string()));
        assert_eq!(wire.as_str(), "vllm");
    }

    #[test]
    fn capability_serde_snake_case() {
        let caps = vec![
            Capability::Text,
            Capability::Vision,
            Capability::FunctionCalling,
            Capability::ToolUse,
            Capability::Reasoning,
            Capability::PromptCaching,
            Capability::JsonMode,
            Capability::ParallelToolCalls,
            Capability::ImageGeneration,
            Capability::Audio,
        ];
        let json = serde_json::to_string(&caps).unwrap();
        assert!(json.contains("\"function_calling\""));
        assert!(json.contains("\"tool_use\""));
        assert!(json.contains("\"parallel_tool_calls\""));
        assert!(json.contains("\"prompt_caching\""));
        let back: Vec<Capability> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, caps);
    }

    #[test]
    fn tier_serde_snake_case() {
        for t in [
            Tier::Free,
            Tier::FreeTier,
            Tier::Paid,
            Tier::Frontier,
            Tier::Local,
            Tier::Experimental,
        ] {
            let json = serde_json::to_string(&t).unwrap();
            let back: Tier = serde_json::from_str(&json).unwrap();
            assert_eq!(back, t);
        }
    }

    #[test]
    fn deployment_new_assigns_deterministic_id() {
        let d = Deployment::new("claude-opus-4", "https://api.anthropic.com");
        assert!(d.id.starts_with("claude-opus-4-"));
        assert_eq!(d.weight, 1.0);
        assert_eq!(d.priority, 0);
        assert!(d.api_key_env.is_none());
        let d2 = Deployment::new("claude-opus-4", "https://api.anthropic.com");
        assert_eq!(d.id, d2.id, "same api_base → same id");
        let d3 = Deployment::new("claude-opus-4", "https://backup.anthropic.com");
        assert_ne!(d.id, d3.id, "different api_base → different id");
    }

    #[test]
    fn deployment_builder_chains() {
        let d = Deployment::new("m", "https://x")
            .with_label("My backend")
            .with_api_key_env("MY_KEY");
        assert_eq!(d.label, "My backend");
        assert_eq!(d.api_key_env.as_deref(), Some("MY_KEY"));
    }

    #[test]
    fn model_descriptor_serde_roundtrip() {
        let desc = ModelDescriptor {
            id: "claude-opus-4".into(),
            provider: ProviderWire::Anthropic,
            display_name: "Claude Opus 4".into(),
            tier: Tier::Frontier,
            context_window: 200_000,
            max_output_tokens: 32_000,
            capabilities: vec![
                Capability::Text,
                Capability::ToolUse,
                Capability::PromptCaching,
            ],
            input_cost_per_1m_tokens: 15.0,
            output_cost_per_1m_tokens: 75.0,
            cache_read_cost_per_1m_tokens: 1.5,
            latency_ms_p50: 900,
            aliases: vec!["~claude-opus-latest".into()],
        };
        let json = serde_json::to_string(&desc).unwrap();
        let back: ModelDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(back, desc);
    }
}
