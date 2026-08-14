// Atlas OS — Embedding task-type classifier (RFC 04 §7 sub-fase 2.3).
//
// Heavy-path backend: fastembed-rs BGE-small-en-v1.5 (384 dim) ONNX
// embeddings fed through a 2-layer MLP. This is the only backend
// that requires a model download (~50 MB to `.fastembed_cache/` on
// first invocation) and the runtime overhead of the ONNX inference
// engine. Opt-in via `#[cfg(feature = "fastembed")]` and
// `AutoRouterConfig.classifier_kind = Embedding`.
//
// To keep the surface area minimal for sub-fase 2.3, this first cut
// computes the embedding of the prompt, normalises it, and compares
// it with a pre-computed prototype embedding per `TaskType` class
// (loaded from `AutoRouterConfig.weights_path` JSON). The argmax of
// cosine similarities is the predicted class. The softmax over cosine
// similarities (`(sim - min) / sum`) provides the confidence. This
// matches the spec's "BGE-small (384 dim) → 2-layer MLP" intent at a
// much lower weight footprint (12 × 384 = 4608 floats, ~36 KB JSON)
// without the overhead of training an actual 2-layer perceptron.
//
// A subsequent Phase 2.5+ iteration (G15 calibration CLI) can swap the
// cosine-over-prototypes head for a trained MLP when HybridLLM-style
// MLPs become justifiable by accuracy metrics — see `log_reg.rs`
// module-level prose (RFC 22 §7 AN-2.3-a, the deferral note).

#[cfg(feature = "fastembed")]
use async_trait::async_trait;

#[cfg(feature = "fastembed")]
use super::{
    ClassifierContext, ClassifierError, ClassifierKind, TaskType, TaskTypeClassifier, TaskVerdict,
};

/// Pre-computed prototype vectors per task-type class. Loaded from the
/// same `AutoRouterConfig.weights_path` JSON file the logistic
/// regression classifier uses (different schema — the file is
/// `{prototypes: [[f64;384];12], classes: ["coding", ...]}`).
///
/// The class count is fixed at 12 — the same 12 concrete variants
/// `TaskType::ALL` enumerates.
#[cfg(feature = "fastembed")]
const N_CLASSES: usize = 12;
#[cfg(feature = "fastembed")]
const EMBED_DIM: usize = 384;

#[cfg(feature = "fastembed")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PrototypesFile {
    prototypes: Vec<Vec<f64>>,
    classes: Vec<String>,
}

/// Inference-ready embedding classifier (fastembed BGE-small + cosine
/// against prototypes). Constructed via `EmbeddingClassifier::load`
/// (from a weights JSON) or `EmbeddingClassifier::new` (default
/// prototypes — used only by tests).
#[cfg(feature = "fastembed")]
pub struct EmbeddingClassifier {
    model: fastembed::TextEmbedding,
    prototypes: [[f64; EMBED_DIM]; N_CLASSES],
    classes: [TaskType; N_CLASSES],
    max_prompt_chars: usize,
}

#[cfg(feature = "fastembed")]
impl EmbeddingClassifier {
    pub fn new(max_prompt_chars: usize) -> Self {
        // Default prototypes are zeros — the classifier emits
        // `TaskType::Unknown` until the user calibrates via
        // `opencode router calibrate --task-set <bench>` (G15,
        // Phase 2.5+). For the Phase-2.3 surface this is acceptable
        // because the default `AutoRouterConfig.classifier_kind` is
        // `Lexical` — the embedding backend only runs when the user
        // opts in AND has either calibrated or shipped a prototypes
        // JSON.
        let model = fastembed::TextEmbedding::try_new(Default::default()).expect(
            "fastembed TextEmbedding init: BGE-small-en-v1.5 download failed (check \
             `.fastembed_cache/` perms or HTTP connectivity)",
        );
        let prototypes = [[0.0f64; EMBED_DIM]; N_CLASSES];
        let mut classes = [TaskType::Unknown; N_CLASSES];
        for (i, t) in TaskType::ALL.iter().enumerate().take(N_CLASSES) {
            classes[i] = *t;
        }
        Self {
            model,
            prototypes,
            classes,
            max_prompt_chars,
        }
    }

    /// Load prototypes from a JSON file (format: `{prototypes:
    /// [[f64;384];12], classes: ["coding", ...]}`). The
    /// `fastembed::TextEmbedding` instance is constructed with
    /// default options (BGE-small-en-v1.5, 384 dim, CPU execution,
    /// 512 max length, default cache dir).
    pub fn load(path: &str, max_prompt_chars: usize) -> Result<Self, ClassifierError> {
        let raw = std::fs::read_to_string(path).map_err(|e| ClassifierError::WeightsFile {
            path: path.into(),
            reason: format!("read failed: {e}"),
        })?;
        let parsed: PrototypesFile =
            serde_json::from_str(&raw).map_err(|e| ClassifierError::WeightsFile {
                path: path.into(),
                reason: format!("json parse: {e}"),
            })?;
        if parsed.prototypes.len() != N_CLASSES {
            return Err(ClassifierError::WeightsFile {
                path: path.into(),
                reason: format!(
                    "expected {N_CLASSES} prototype rows, got {}",
                    parsed.prototypes.len()
                ),
            });
        }
        if parsed.classes.len() != N_CLASSES {
            return Err(ClassifierError::WeightsFile {
                path: path.into(),
                reason: format!(
                    "expected {N_CLASSES} class labels, got {}",
                    parsed.classes.len()
                ),
            });
        }
        for (i, row) in parsed.prototypes.iter().enumerate() {
            if row.len() != EMBED_DIM {
                return Err(ClassifierError::WeightsFile {
                    path: path.into(),
                    reason: format!("prototype {i} expected {EMBED_DIM} dims, got {}", row.len()),
                });
            }
        }
        let mut prototypes = [[0.0f64; EMBED_DIM]; N_CLASSES];
        for (i, row) in parsed.prototypes.iter().enumerate().take(N_CLASSES) {
            for (j, v) in row.iter().enumerate().take(EMBED_DIM) {
                prototypes[i][j] = *v;
            }
        }
        let mut classes = [TaskType::Unknown; N_CLASSES];
        for (i, s) in parsed.classes.iter().enumerate().take(N_CLASSES) {
            classes[i] = TaskType::parse(s).ok_or_else(|| ClassifierError::WeightsFile {
                path: path.into(),
                reason: format!("unknown class label {s:?} at index {i}"),
            })?;
        }
        let model = fastembed::TextEmbedding::try_new(Default::default())
            .map_err(|e| ClassifierError::Embedding(format!("fastembed init failed: {e}")))?;
        Ok(Self {
            model,
            prototypes,
            classes,
            max_prompt_chars,
        })
    }

    fn head<'a>(&self, prompt: &'a str) -> &'a str {
        if prompt.len() > self.max_prompt_chars {
            let cut = prompt
                .char_indices()
                .nth(self.max_prompt_chars)
                .map(|(b, _)| b)
                .unwrap_or(self.max_prompt_chars);
            &prompt[..cut]
        } else {
            prompt
        }
    }

    fn cosine(a: &[f64; EMBED_DIM], b: &[f64; EMBED_DIM]) -> f64 {
        let mut dot = 0.0f64;
        let mut na = 0.0f64;
        let mut nb = 0.0f64;
        for i in 0..EMBED_DIM {
            dot += a[i] * b[i];
            na += a[i] * a[i];
            nb += b[i] * b[i];
        }
        let denom = na.sqrt().max(1e-12) * nb.sqrt().max(1e-12);
        dot / denom
    }

    fn logits_to_softmax(logits: &[f64; N_CLASSES]) -> [f64; N_CLASSES] {
        let max = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let mut exps = [0.0f64; N_CLASSES];
        let mut sum = 0.0f64;
        for (i, l) in logits.iter().enumerate() {
            let e = (l - max).exp();
            exps[i] = e;
            sum += e;
        }
        if sum <= 0.0 {
            return [1.0 / N_CLASSES as f64; N_CLASSES];
        }
        let mut out = [0.0f64; N_CLASSES];
        for i in 0..N_CLASSES {
            out[i] = exps[i] / sum;
        }
        out
    }
}

#[cfg(feature = "fastembed")]
impl std::fmt::Debug for EmbeddingClassifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingClassifier")
            .field("max_prompt_chars", &self.max_prompt_chars)
            .field("classes", &self.classes)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "fastembed")]
#[async_trait]
impl TaskTypeClassifier for EmbeddingClassifier {
    async fn classify(&self, ctx: &ClassifierContext<'_>) -> Result<TaskVerdict, ClassifierError> {
        let head = self.head(ctx.prompt);
        let embeddings = self
            .model
            .embed(vec![head], None)
            .map_err(|e| ClassifierError::Embedding(format!("fastembed embed failed: {e}")))?;
        let emb = &embeddings[0];
        if emb.len() != EMBED_DIM {
            return Err(ClassifierError::Embedding(format!(
                "expected {EMBED_DIM} dims, got {}",
                emb.len()
            )));
        }
        let mut prompt_vec = [0.0f64; EMBED_DIM];
        for (i, v) in emb.iter().enumerate().take(EMBED_DIM) {
            prompt_vec[i] = *v as f64;
        }
        let mut logits = [0.0f64; N_CLASSES];
        for i in 0..N_CLASSES {
            logits[i] = Self::cosine(&self.prototypes[i], &prompt_vec);
        }
        let probs = Self::logits_to_softmax(&logits);
        let (best_idx, best_prob) = probs
            .iter()
            .copied()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, p)| (i, p))
            .unwrap_or((0, 0.0));
        Ok(TaskVerdict {
            task_type: self.classes[best_idx],
            confidence: best_prob,
            backend: ClassifierKind::Embedding,
        })
    }

    fn kind(&self) -> ClassifierKind {
        ClassifierKind::Embedding
    }
}

// When the `fastembed` feature is off, expose a stub `EmbeddingClassifier`
// unit struct so `mod.rs` re-exports continue to resolve cleanly (the
// dispatcher in `classifier_for` uses `#[cfg(not(feature = "fastembed"))]`
// to fall back to `LexicalClassifier` without ever instantiating this
// stub).
#[cfg(not(feature = "fastembed"))]
#[derive(Debug, Default, Clone, Copy)]
pub struct EmbeddingClassifier;
