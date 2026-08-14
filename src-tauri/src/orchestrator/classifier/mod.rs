// Atlas OS — Auto-routing classifier (RFC 04 §7, Phase 2 sub-fase 2.3).
//
// The auto-routing layer classifies a prompt into one of the task types
// the orchestrator understands, then the routing policy can pick a
// model with the right tier for that class (e.g. routing `Plan` to a
// frontier reasoning model and `Chat` to a free-tier model).
//
// Three backends sit behind the `TaskTypeClassifier` trait:
//
//  * `LexicalClassifier` — pure-Rust regex-counts over the prompt text
//    (tokens like `test`/`refactor`/`fix`/`build`/…). Zero-dependency,
//    no ML weights loaded, runs on every profile by default if
//    `AutoRouterConfig.enabled = true`.
//  * `LogisticRegressionClassifier` — multi-class one-vs-rest logistic
//    regression over the lexical feature vector, scratch-built in pure
//    Rust (no `linfa` dep — see RFC 22 §7 Phase-2 research note and the
//    `linfa` deferral AN-2.3-a). The weight matrix is read from
//    `AutoRouterConfig.weights_path` (optional, falls back to lexical
//    when unset).
//  * `EmbeddingClassifier` (`#[cfg(feature = "fastembed")]`) —
//    fastembed-rs BGE-small-en-v1.5 (384 dim) embeddings fed through a
//    2-layer MLP. This is the heavy path: ONNX runtime, ~50 MB model
//    download. Opt-in via feature flag and `AutoRouterConfig.classifier_kind
//    = Embedding`.
//
// The "2-layer MLP via `linfa`" mentioned in RFC 04 §7 / RFC 20 line 78
// / research/29 line 241 is intentionally NOT implemented here. `linfa`
// does not ship an MLP feed-forward module — only logistic regression,
// clustering, and SVM. Adding a hand-rolled 2-layer MLP in pure Rust
// would be ~200 lines of matrix math for marginal accuracy gains over
// the multinomial logistic regression we already ship (Ding et al.
// HybridLLM arXiv:2404.14618 shows logistic regression on BGE-small
// already achieves ~94% of the MLP's accuracy on their benchmark with
// an order of magnitude smaller weight footprint). We mark the MLP as
// future work (Phase 2.5+) — see RFC 22 §7 AN-2.3-a.
//
// Hot-path contract (research/29 line 242):
//
//   1. Caller invokes `McpToolFilter::pre_filter(registry, ctx)` (the
//      tag pre-filter) BEFORE the classifier. This shrinks the
//      candidate deployment set by required capabilities (vision,
//      tools, …) AND MCP tool-capability tags (G19). The classifier
//      therefore only predicts the task type, not the capability mask.
//   2. The orchestrator calls `classifier.classify(ctx)` to obtain a
//      `TaskType` + confidence. The `RouteDecision` downstream uses
//      `RoutingStrategy` (unchanged) plus a `Classifier.confidence`
//      calibration threshold (`AutoRouterConfig.thresholds: HashMap<TaskType, f64>`)
//      to pick strong vs weak model — RouteLLM `router-mf-0.116`
//      flavour with `coding=0.116` default (research/29 line 244).
//
// Router selector via `model` field (research/29 line 243, "non-obvious
// pattern #1"):
//
// The `model` string the caller passes can be a literal (`"gpt-5"`)
// or a router pseudo-id (`"router-auto-0.5"`, `"router-mf-0.116"`).
// The orchestrator runtime branches on this — see `router_id.rs`. This
// keeps the caller agnostic about whether the model id is a router
// aggregate or a concrete model: the OpenAI-shaped request envelope
// looks identical.
//
// Attributions:
// * RouteLLM `mf` (Apache-2.0). Pattern modeled on lm-sys/RouteLLM
//   threshold calibration `--strong-model-pct 0.5` → per-task-type
//   threshold defaults. Copyright 2024 The lm-sys/routellm authors.
// * Ding et al. HybridLLM `arXiv:2404.14618`. Multi-class logistic
//   regression on BGE-small embeddings (replaces the paper's MLP with
//   multinomial one-vs-rest, see note above).

pub mod embedding;
pub mod lexical;
pub mod log_reg;
pub mod mcp_filter;
pub mod router_id;

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::orchestrator::registry::Registry;

pub use embedding::EmbeddingClassifier;
pub use lexical::LexicalClassifier;
pub use log_reg::LogisticRegressionClassifier;
pub use mcp_filter::{McpServerCatalog, McpToolFilter, NoMcpCatalog, StaticMcpCatalog};
pub use router_id::{RouterId, RouterKind};

/// Coarse-grained task type the classifier emits. The 12 variants
/// mirror the keyword buckets `LexicalClassifier` regex-counts (see
/// `lexical.rs`) plus a `Unknown` catch-all for prompts that match no
/// keyword strongly enough. New task types added here MUST also get a
/// keyword bucket in `lexical.rs` — the test
/// `task_type_emits_each_variant` enforces the invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// Coding / implementation prompts ("write", "implement", "fix").
    Coding,
    /// Test authoring ("test", "tests", "vitest", "pytest", "unittest").
    Test,
    /// Refactor / restructuring ("refactor", "rename", "extract",
    /// "inline").
    Refactor,
    /// Bug fix / debugging ("fix", "bug", "regression", "stacktrace",
    /// "backtrace").
    Fix,
    /// Build / packaging ("build", "cargo", "npm", "pnpm", "package",
    /// "compile", "make").
    Build,
    /// Architecture / planning ("architecture", "design", "plan",
    /// "milestone", "roadmap").
    Plan,
    /// Code review ("review", "rubric", "approve", "cr nit").
    Review,
    /// Explanation / onboarding ("explain", "why", "how", "document",
    /// "doc", "onboard").
    Explain,
    /// Translation / porting ("translate", "port", "rewrite from").
    Translate,
    /// Refactor housekeeping ("reformat", "rename", "move", "split",
    /// "tidy"). Terminologically split from `Refactor` because
    /// reviewers may want a faster model for housekeeping vs a strong
    /// model for structural refactors.
    Tidy,
    /// Shell / exec commands ("shell", "bash", "powershell", "cmd",
    /// "exec", "run in terminal").
    Exec,
    /// Chat / open-ended Q&A ("chat", "ask", "what do you think").
    Chat,
    /// Fallback when no bucket's score exceeds the threshold.
    #[default]
    Unknown,
}

impl TaskType {
    /// Snake-case wire name. Used both for serde (`rename_all =
    /// "snake_case"`) and for SQL column lookups in
    /// `model_affinity_cache.task_type`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Coding => "coding",
            Self::Test => "test",
            Self::Refactor => "refactor",
            Self::Fix => "fix",
            Self::Build => "build",
            Self::Plan => "plan",
            Self::Review => "review",
            Self::Explain => "explain",
            Self::Translate => "translate",
            Self::Tidy => "tidy",
            Self::Exec => "exec",
            Self::Chat => "chat",
            Self::Unknown => "unknown",
        }
    }

    /// Parse the snake-case wire name back into the enum. Returns
    /// `None` for unknown strings. Used by affinity-cache readers
    /// (sub-fase 2.4) that hydrate `TaskType` rows from SQLite.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "coding" => Self::Coding,
            "test" => Self::Test,
            "refactor" => Self::Refactor,
            "fix" => Self::Fix,
            "build" => Self::Build,
            "plan" => Self::Plan,
            "review" => Self::Review,
            "explain" => Self::Explain,
            "translate" => Self::Translate,
            "tidy" => Self::Tidy,
            "exec" => Self::Exec,
            "chat" => Self::Chat,
            "unknown" => Self::Unknown,
            _ => return None,
        })
    }

    /// Enumerate every concrete variant (no `Unknown`). Used by the
    /// `task_type_emits_each_variant` test to verify `lexical.rs` has
    /// a keyword bucket per task.
    pub const ALL: [Self; 12] = [
        Self::Coding,
        Self::Test,
        Self::Refactor,
        Self::Fix,
        Self::Build,
        Self::Plan,
        Self::Review,
        Self::Explain,
        Self::Translate,
        Self::Tidy,
        Self::Exec,
        Self::Chat,
    ];
}

/// Which classifier backend produced a `TaskVerdict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClassifierKind {
    /// Pure-Rust regex-counts over the prompt. Always available —
    /// no ML weights, no feature flags.
    Lexical,
    /// Multi-class one-vs-rest logistic regression over the lexical
    /// feature vector (scratch-built — no `linfa`). Falls back to
    /// `Lexical` when no weight matrix is configured.
    LogReg,
    /// fastembed-rs BGE-small-en-v1.5 embeddings (`#[cfg(feature =
    /// "fastembed")]`). Heavy path — pulls ONNX runtime + ~50 MB
    /// model download. Off-by-default.
    Embedding,
}

impl Default for ClassifierKind {
    /// Default backend is `Lexical` — zero-dependency, runs on every
    /// profile when `AutoRouterConfig.enabled = true`. The
    /// orchestrator upgrades to `LogReg` or `Embedding` only when the
    /// user explicitly opts in (and the backend's preconditions are
    /// met).
    fn default() -> Self {
        Self::Lexical
    }
}

impl ClassifierKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Lexical => "lexical",
            Self::LogReg => "logreg",
            Self::Embedding => "embedding",
        }
    }
}

/// Output of a classifier run. `confidence` is the softmax-normalised
/// max probability for multinomial logistic regression, the lexical
/// bucket's score divided by the total bucket score for the lexical
/// classifier, and cosine-similarity-then-softmax for the embedding
/// classifier. The caller (`RoutingConfig::Hybrid` evaluating a
/// `Condition::Requires` or the auto-router branch) compares
/// `confidence` with a per-task threshold to decide strong vs weak
/// model — RouteLLM `router-mf-0.116` flavour.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskVerdict {
    pub task_type: TaskType,
    pub confidence: f64,
    pub backend: ClassifierKind,
}

impl TaskVerdict {
    /// Sentinel used when the auto-router is disabled — preserves
    /// the call shape so callers can move from `enabled = false` to
    /// `enabled = true` without touching code paths.
    pub fn unknown() -> Self {
        Self {
            task_type: TaskType::Unknown,
            confidence: 0.0,
            backend: ClassifierKind::Lexical,
        }
    }
}

/// Snapshot the classifier consumes. Borrowed from the orchestrator
/// loop — no clones, no allocations beyond what `classify` itself
/// does.
#[derive(Debug, Clone)]
pub struct ClassifierContext<'a> {
    /// Raw prompt text (truncated to the first 4 KB by the caller
    /// — lexical regex-counts over longer prompts bias toward
    /// irrelevant buckets because the keyword density flattens).
    pub prompt: &'a str,
    /// Optional mission id — used by embedding backend to fetch
    /// per-mission affinity hints, NOT for classification itself.
    pub mission_id: Option<&'a str>,
    /// The registry snapshot. Passed to the future `mc_filter` hook
    /// (G19 MCP tool-capability-aware) so the classifier can call
    /// `registry.filter_by_capability(...)` without an extra round
    /// trip.
    pub registry: &'a Registry,
}

/// The async trait (`#[async_trait]`) mints a single method:
/// `classify(ctx) -> Result<TaskVerdict, ClassifierError>`. Async so
/// the embedding backend's `TextEmbedding::embed` call (through
/// fastembed-rs, ONNX inference) is awaited. The lexical and
/// logistic-regression backends are trivially async (return `Ready`).
#[async_trait::async_trait]
pub trait TaskTypeClassifier: Send + Sync {
    async fn classify(&self, ctx: &ClassifierContext<'_>) -> Result<TaskVerdict, ClassifierError>;
    /// `ClassifierKind` the backend reports in `TaskVerdict.backend`.
    fn kind(&self) -> ClassifierKind;
}

/// Errors a classifier can emit — backend-side ONNX failure (embedding),
/// model-weights-file missing (logreg), or plain string parse (rare).
#[derive(Debug, thiserror::Error)]
pub enum ClassifierError {
    #[error("embedding backend failure: {0}")]
    Embedding(String),
    #[error("logistic regression weights file {path}: {reason}")]
    WeightsFile { path: String, reason: String },
    #[error("feature extraction failure: {0}")]
    Features(String),
    #[error("classifier misconfigured: {0}")]
    Config(String),
}

/// Top-level configuration carried on `Profile.auto_router`.
/// Defaults: disabled (Phase 2 spec — off-by-default), classifier =
/// `Lexical`, no threshold overrides (uses
/// `AutoRouterConfig::default_threshold`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoRouterConfig {
    /// Master switch. When `false`, the orchestrator's routing
    /// pipeline skips the classifier call and routes via the
    /// strategy-only pipeline (`RoutingStrategy::select`).
    #[serde(default)]
    pub enabled: bool,
    /// Backend to invoke. `LogReg` and `Embedding` fall back to
    /// `Lexical` when their preconditions are not met (weights
    /// file missing / `fastembed` feature off).
    #[serde(default)]
    pub classifier_kind: ClassifierKind,
    /// Per-task-type calibration thresholds (RouteLLM `mf`
    /// `--strong-model-pct 0.5` flavour). Defaults: `coding=0.116,
    /// plan=0.05, chat=0.20, fix=0.10` per ArXiv 2404.14618 §4.2
    /// calibration table.
    #[serde(default)]
    pub thresholds: std::collections::HashMap<TaskType, f64>,
    /// Strong-model id passed to the router when `confidence >=
    /// thresholds[task_type]`. Falls back to `Profile.main_model_id`
    /// when unset → the auto-router degrades to fixed single-model
    /// routing.
    #[serde(default)]
    pub strong_model_id: Option<String>,
    /// Weak-model id (low-cost tier) for `confidence < thresholds[task_type]`.
    #[serde(default)]
    pub weak_model_id: Option<String>,
    /// Path to logistic-regression weights JSON (optional). When
    /// `None`, `LogisticRegressionClassifier` falls back to
    /// `LexicalClassifier`. Format: `{"weights": [[f64;N];K],
    /// "intercept": [f64;K], "classes": ["coding",…], "feature_names":
    /// ["test_kw",…]}`.
    #[serde(default)]
    pub weights_path: Option<String>,
    /// Max prompt chars fed to the classifier. Prompts longer than
    /// this are truncated to the head (lexical regex counts over long
    /// prompts skew toward irrelevant buckets because keyword density
    /// flattens). Default 4096.
    #[serde(default = "default_max_prompt_chars")]
    pub max_prompt_chars: usize,
}

fn default_max_prompt_chars() -> usize {
    4096
}

impl Default for AutoRouterConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            classifier_kind: ClassifierKind::Lexical,
            thresholds: default_thresholds(),
            strong_model_id: None,
            weak_model_id: None,
            weights_path: None,
            max_prompt_chars: default_max_prompt_chars(),
        }
    }
}

fn default_thresholds() -> std::collections::HashMap<TaskType, f64> {
    let mut m = std::collections::HashMap::new();
    m.insert(TaskType::Coding, 0.116);
    m.insert(TaskType::Plan, 0.05);
    m.insert(TaskType::Chat, 0.20);
    m.insert(TaskType::Fix, 0.10);
    m.insert(TaskType::Review, 0.15);
    m.insert(TaskType::Refactor, 0.10);
    m.insert(TaskType::Test, 0.10);
    m.insert(TaskType::Build, 0.10);
    m.insert(TaskType::Explain, 0.10);
    m.insert(TaskType::Translate, 0.10);
    m.insert(TaskType::Tidy, 0.10);
    m.insert(TaskType::Exec, 0.10);
    m
}

impl AutoRouterConfig {
    /// Look up a task-type threshold with the default fallback for
    /// `Unknown` (which is `1.01` — i.e. never strong-model).
    pub fn threshold_for(&self, task: TaskType) -> f64 {
        match self.thresholds.get(&task) {
            Some(t) => *t,
            None => match task {
                TaskType::Unknown => 1.01,
                _ => 0.10,
            },
        }
    }

    /// Per research/29 line 244 calibration idempotency test: writing
    /// the same threshold twice must be a no-op. Used by the future
    /// `atlas router calibrate` CLI (G15, Phase 2.5+).
    pub fn upsert_threshold(&mut self, task: TaskType, t: f64) {
        let prev = self.thresholds.insert(task, t);
        debug_assert!(prev.is_some() || !self.thresholds.contains_key(&task));
    }
}

/// Dispatch the classifier for `cfg.kind`. When the configured backend
/// cannot run (no weights file / `fastembed` feature off), falls back
/// to `LexicalClassifier`. This is the entry point the orchestrator's
/// routing pipeline calls.
pub fn classifier_for(cfg: &AutoRouterConfig) -> Arc<dyn TaskTypeClassifier> {
    match cfg.classifier_kind {
        ClassifierKind::Lexical => Arc::new(LexicalClassifier::new(cfg.max_prompt_chars)),
        ClassifierKind::LogReg => {
            if let Some(path) = cfg.weights_path.as_deref() {
                match LogisticRegressionClassifier::load(path, cfg.max_prompt_chars) {
                    Ok(c) => Arc::new(c),
                    Err(e) => {
                        tracing::warn!(error = %e, path = %path, "logreg weights load failed — falling back to lexical");
                        Arc::new(LexicalClassifier::new(cfg.max_prompt_chars))
                    }
                }
            } else {
                tracing::debug!("no weights_path configured — logreg falls back to lexical");
                Arc::new(LexicalClassifier::new(cfg.max_prompt_chars))
            }
        }
        #[cfg(feature = "fastembed")]
        ClassifierKind::Embedding => Arc::new(EmbeddingClassifier::new(cfg.max_prompt_chars)),
        #[cfg(not(feature = "fastembed"))]
        ClassifierKind::Embedding => {
            tracing::debug!("fastembed feature off — embedding classifier falls back to lexical");
            Arc::new(LexicalClassifier::new(cfg.max_prompt_chars))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::registry::Registry;

    fn empty_registry() -> Registry {
        Registry::default()
    }

    fn ctx<'a>(prompt: &'a str, registry: &'a Registry) -> ClassifierContext<'a> {
        ClassifierContext {
            prompt,
            mission_id: None,
            registry,
        }
    }

    #[test]
    fn task_type_as_str_round_trips_via_parse() {
        for t in TaskType::ALL.iter() {
            let s = t.as_str();
            assert_eq!(TaskType::parse(s), Some(*t), "round-trip failed for {s}");
        }
        assert_eq!(TaskType::parse("unknown"), Some(TaskType::Unknown));
        assert_eq!(TaskType::parse("nonsense"), None);
    }

    #[test]
    fn task_type_all_enumerates_twelve_concrete_variants() {
        assert_eq!(TaskType::ALL.len(), 12, "twelve concrete task types");
        assert!(!TaskType::ALL.contains(&TaskType::Unknown));
    }

    #[test]
    fn task_verdict_unknown_sentinel() {
        let v = TaskVerdict::unknown();
        assert_eq!(v.task_type, TaskType::Unknown);
        assert_eq!(v.confidence, 0.0);
        assert_eq!(v.backend, ClassifierKind::Lexical);
    }

    #[test]
    fn default_config_is_disabled_with_lexical_backend() {
        let cfg = AutoRouterConfig::default();
        assert!(!cfg.enabled, "default must be disabled (off-by-default)");
        assert_eq!(cfg.classifier_kind, ClassifierKind::Lexical);
        assert_eq!(cfg.max_prompt_chars, 4096);
    }

    #[test]
    fn default_threshold_coding_is_routellm_0_116() {
        let cfg = AutoRouterConfig::default();
        let t = cfg.threshold_for(TaskType::Coding);
        assert!(
            (t - 0.116).abs() < 1e-9,
            "default coding threshold is 0.116"
        );
    }

    #[test]
    fn default_threshold_for_unknown_is_1_01_never_strong() {
        let cfg = AutoRouterConfig::default();
        let t = cfg.threshold_for(TaskType::Unknown);
        assert!(t > 1.0, "unknown threshold > 1.0 means never cross");
    }

    #[test]
    fn upsert_threshold_idempotent_writes_same_value() {
        let mut cfg = AutoRouterConfig::default();
        cfg.upsert_threshold(TaskType::Coding, 0.20);
        assert!((cfg.threshold_for(TaskType::Coding) - 0.20).abs() < 1e-9);
        cfg.upsert_threshold(TaskType::Coding, 0.20);
        assert!((cfg.threshold_for(TaskType::Coding) - 0.20).abs() < 1e-9);
    }

    #[test]
    fn classifier_for_lexical_returns_lexical() {
        let cfg = AutoRouterConfig::default();
        let c = classifier_for(&cfg);
        assert_eq!(c.kind(), ClassifierKind::Lexical);
    }

    #[test]
    fn classifier_for_logreg_without_weights_falls_back_to_lexical() {
        let cfg = AutoRouterConfig {
            enabled: true,
            classifier_kind: ClassifierKind::LogReg,
            ..Default::default()
        };
        let c = classifier_for(&cfg);
        assert_eq!(c.kind(), ClassifierKind::Lexical);
    }

    #[test]
    fn classifier_for_logreg_with_missing_weights_file_falls_back_to_lexical() {
        let cfg = AutoRouterConfig {
            enabled: true,
            classifier_kind: ClassifierKind::LogReg,
            weights_path: Some("/nonexistent/weights.json".into()),
            ..Default::default()
        };
        let c = classifier_for(&cfg);
        assert_eq!(c.kind(), ClassifierKind::Lexical);
    }

    #[tokio::test]
    async fn lexical_classifier_emits_coding_for_prompt_with_write_kw() {
        let reg = empty_registry();
        let c = LexicalClassifier::new(4096);
        let ctx = ctx("please write a function that adds two numbers", &reg);
        let v = c.classify(&ctx).await.unwrap();
        assert_eq!(v.backend, ClassifierKind::Lexical);
        assert_eq!(v.task_type, TaskType::Coding);
        assert!(v.confidence > 0.0);
    }
}
