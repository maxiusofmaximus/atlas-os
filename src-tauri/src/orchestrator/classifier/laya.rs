// Atlas OS — Laya task-type classifier stub (RFC 04 §7, Phase 9 sub-fase 9.1, M33).
//
// Fourth backend behind the `TaskTypeClassifier` trait: `laya = "0.1.1"`
// (`aovestdipaperino/laya-rust`, crates.io, Sep 2026) — "Rust inference
// for the Laya non-autoregressive typed-decision model (ModernBERT-large
// + RL decision head)". This is the "System One" judgments backend the
// Round 7 audit (RFC 35 §7.1) decided: a small typed-decision model for
// fast per-turn routing/eval/judging/compaction instead of a frontier
// model on every step.
//
// RFC 25 §11 audit outcome: DIFERIDO (see RFC 22 §7 AN-9.1). The candle
// stack (`candle-core`/`candle-nn`/`candle-transformers` 0.9 +
// `tokenizers` 0.21) fails the single-binary gate: `rand 0.9` conflicts
// with the pinned `rand 0.8`, `tokenizers` is not in deps (contrary to
// RFC 35 §7.1's assumption) and pulls a C++ build (`esaxx-rs`) plus a
// precompiled lib (`spm_precompiled`), the `serve` feature wants
// `axum 0.8` while the HUD pins `axum 0.7`, the 178 KB crate ships code
// only (ModernBERT-large weights download at runtime, hundreds of MB),
// and the crate is 5 days old with 61 downloads and a yanked 0.1.0.
// Bundling it now would double the inference runtimes (`ort-sys` via
// `fastembed` + `candle`) against the 30-45 MB budget.
//
// This module therefore ships the Phase 8 precedent (8.2/8.3/8.4
// std-only MVP): the owned `LayaClassifier` shape with zero new deps,
// deterministic lexical-delegated inference tagged `ClassifierKind::Laya`,
// and the `load` failure path (`modelo ausente → fallback al backend
// activo`). The real ModernBERT + candle inference swaps in behind the
// empty `laya` cargo feature (default off) without changing callers —
// the same gate pattern as `fastembed`'s feature-off stub in
// `embedding.rs`. Follow-ups of the same front stay annotated:
// compaction wiring (5.3, `weak_model` → Laya) + tool-result judging
// (winnow).

use async_trait::async_trait;

use super::lexical::LexicalClassifier;
use super::{ClassifierContext, ClassifierError, ClassifierKind, TaskTypeClassifier, TaskVerdict};

/// Laya classifier (std-only MVP). Holds the lexical delegate that
/// produces the deterministic verdict plus the optional model dir the
/// future candle backend will load safetensors from.
pub struct LayaClassifier {
    inner: LexicalClassifier,
    model_dir: Option<std::path::PathBuf>,
}

impl std::fmt::Debug for LayaClassifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayaClassifier")
            .field("model_dir", &self.model_dir)
            .finish_non_exhaustive()
    }
}

impl LayaClassifier {
    /// Stub constructor: deterministic lexical-delegated inference,
    /// no model files required. Used when `AutoRouterConfig` selects
    /// `ClassifierKind::Laya` without a `weights_path`.
    pub fn new(max_prompt_chars: usize) -> Self {
        Self {
            inner: LexicalClassifier::new(max_prompt_chars),
            model_dir: None,
        }
    }

    /// Future candle-backend entry point. Today it only validates the
    /// path exists and records it — real safetensors + tokenizers
    /// inference lands behind `#[cfg(feature = "laya")]` (empty gate,
    /// default off; see Cargo.toml). Missing paths error so the
    /// `classifier_for` dispatcher falls back to `LexicalClassifier`
    /// (same contract as `LogisticRegressionClassifier::load`).
    pub fn load(path: &str, max_prompt_chars: usize) -> Result<Self, ClassifierError> {
        let dir = std::path::PathBuf::from(path);
        if !dir.exists() {
            return Err(ClassifierError::WeightsFile {
                path: path.into(),
                reason: "laya model dir absent — falling back to lexical".into(),
            });
        }
        Ok(Self {
            inner: LexicalClassifier::new(max_prompt_chars),
            model_dir: Some(dir),
        })
    }
}

#[async_trait]
impl TaskTypeClassifier for LayaClassifier {
    async fn classify(&self, ctx: &ClassifierContext<'_>) -> Result<TaskVerdict, ClassifierError> {
        let mut verdict = self.inner.classify(ctx).await?;
        verdict.backend = ClassifierKind::Laya;
        Ok(verdict)
    }

    fn kind(&self) -> ClassifierKind {
        ClassifierKind::Laya
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::classifier::TaskType;
    use crate::orchestrator::registry::Registry;

    fn ctx<'a>(prompt: &'a str, registry: &'a Registry) -> ClassifierContext<'a> {
        ClassifierContext {
            prompt,
            mission_id: None,
            registry,
        }
    }

    #[tokio::test]
    async fn laya_stub_emits_coding_with_laya_backend() {
        let reg = Registry::default();
        let c = LayaClassifier::new(4096);
        assert_eq!(c.kind(), ClassifierKind::Laya);
        let v = c
            .classify(&ctx("please write a function that adds two numbers", &reg))
            .await
            .expect("classify");
        assert_eq!(v.task_type, TaskType::Coding);
        assert_eq!(v.backend, ClassifierKind::Laya);
        assert!(v.confidence > 0.0);
    }

    #[test]
    fn laya_load_missing_dir_errors_for_dispatcher_fallback() {
        let res = LayaClassifier::load("/nonexistent/laya-model", 4096);
        assert!(
            res.is_err(),
            "missing model dir must error so classifier_for falls back"
        );
    }
}
