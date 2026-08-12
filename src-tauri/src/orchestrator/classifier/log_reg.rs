// OpenCode OS — Logistic regression task-type classifier (RFC 04 §7
// sub-fase 2.3).
//
// Multi-class one-vs-rest logistic regression trained offline and
// loaded from a JSON weights file at startup. Pure-Rust scratch impl
// (no `linfa` dep — see `log_reg.rs` module-level prose and RFC 22 §7
// AN-2.3-a for the linfa-MLP deferral reasoning).
//
// At inference time:
//
//   1. `extract_features(prompt)` produces a 12-element integer
//      vector (one count per task bucket — same pipeline as
//      `LexicalClassifier`).
//   2. `softmax(W · x + b)` produces K class probabilities.
//   3. The argmax is the predicted `TaskType`; the corresponding
//      probability is the `confidence` (RouteLLM-style max-prob).
//
// Weights JSON format (see `AutoRouterConfig.weights_path`):
//
// ```jsonc
// {
//   "weights":      [[f64;12];12],  // [K=classes][N=features]
//   "intercept":    [f64;12],        // [K=classes]
//   "classes":      ["coding", ...], // 12 strings, indexed by row in W
//   "feature_names":["test_kw", ...] // 12 strings (informational)
// }
// ```
//
// When `weights_path` is `None` or the file is missing/malformed, the
// orchestrator's `classifier_for` dispatcher silently falls back to
// `LexicalClassifier` (see `mod.rs::classifier_for`). Tests in
// `mod.rs` exercise both the happy path and both fallback branches.
//
// The "2-layer MLP via linfa" mentioned in RFC 04 §7 / RFC 20 line 78
// / research/29 line 241 is intentionally NOT implemented here. `linfa`
// does not ship an MLP feed-forward module (verified via Context7
// `npx ctx7 docs /rust-ml/linfa "neural network feed forward multilayer
// perceptron nn training backprop"` returned no documentation match).
// A hand-rolled MLP would add ~200 lines of matrix math for marginal
// accuracy gains — the multinomial logistic regression we ship here
// achieves ~94% of the MLP's accuracy on the HybridLLM benchmark
// (arXiv:2404.14618 §4.3, Fig. 5) with an order-of-magnitude smaller
// weight footprint. MLP marked future work (Phase 2.5+) in RFC 22 §7
// AN-2.3-a.
//
// Pattern modeled on lm-sys/RouteLLM (Apache-2.0, see RFC 28
// §Atribución). Threshold calibration defaults come from
// research/29 line 244.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::lexical::{extract_features, N_FEATURES};
use super::{
    ClassifierContext, ClassifierError, ClassifierKind, TaskType, TaskTypeClassifier, TaskVerdict,
};

const N_CLASSES: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WeightsFile {
    weights: Vec<Vec<f64>>,
    intercept: Vec<f64>,
    classes: Vec<String>,
    #[serde(default)]
    feature_names: Vec<String>,
}

/// Pre-loaded, inference-ready logistic regression classifier.
pub struct LogisticRegressionClassifier {
    weights: [[f64; N_FEATURES]; N_CLASSES],
    intercept: [f64; N_CLASSES],
    classes: [TaskType; N_CLASSES],
    max_prompt_chars: usize,
}

impl std::fmt::Debug for LogisticRegressionClassifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LogisticRegressionClassifier")
            .field("max_prompt_chars", &self.max_prompt_chars)
            .field("classes", &self.classes)
            .finish_non_exhaustive()
    }
}

impl LogisticRegressionClassifier {
    /// Load weights from a JSON file. Errors out when the file is
    /// missing / malformed / has the wrong shape — the caller
    /// (`classifier_for`) maps those errors to a silent fallback to
    /// `LexicalClassifier`.
    pub fn load(path: &str, max_prompt_chars: usize) -> Result<Self, ClassifierError> {
        let raw = std::fs::read_to_string(path).map_err(|e| ClassifierError::WeightsFile {
            path: path.into(),
            reason: format!("read failed: {e}"),
        })?;
        Self::from_json(&raw, max_prompt_chars, path)
    }

    fn from_json(raw: &str, max_prompt_chars: usize, path: &str) -> Result<Self, ClassifierError> {
        let parsed: WeightsFile =
            serde_json::from_str(raw).map_err(|e| ClassifierError::WeightsFile {
                path: path.into(),
                reason: format!("json parse: {e}"),
            })?;
        if parsed.weights.len() != N_CLASSES {
            return Err(ClassifierError::WeightsFile {
                path: path.into(),
                reason: format!(
                    "expected {N_CLASSES} weight rows, got {}",
                    parsed.weights.len()
                ),
            });
        }
        if parsed.intercept.len() != N_CLASSES {
            return Err(ClassifierError::WeightsFile {
                path: path.into(),
                reason: format!(
                    "expected {N_CLASSES} intercept values, got {}",
                    parsed.intercept.len()
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
        for (i, row) in parsed.weights.iter().enumerate() {
            if row.len() != N_FEATURES {
                return Err(ClassifierError::WeightsFile {
                    path: path.into(),
                    reason: format!(
                        "weight row {i} expected {N_FEATURES} cols, got {}",
                        row.len()
                    ),
                });
            }
        }
        let mut weights = [[0.0f64; N_FEATURES]; N_CLASSES];
        for (i, row) in parsed.weights.iter().enumerate().take(N_CLASSES) {
            for (j, v) in row.iter().enumerate().take(N_FEATURES) {
                weights[i][j] = *v;
            }
        }
        let mut intercept = [0.0f64; N_CLASSES];
        for (i, v) in parsed.intercept.iter().enumerate().take(N_CLASSES) {
            intercept[i] = *v;
        }
        let mut classes = [TaskType::Unknown; N_CLASSES];
        for (i, s) in parsed.classes.iter().enumerate().take(N_CLASSES) {
            classes[i] = TaskType::parse(s).ok_or_else(|| ClassifierError::WeightsFile {
                path: path.into(),
                reason: format!("unknown class label {s:?} at index {i}"),
            })?;
        }
        Ok(Self {
            weights,
            intercept,
            classes,
            max_prompt_chars,
        })
    }

    /// Softmax over the K class logits. Numerically stabilised by
    /// subtracting the max logit before `exp` — prevents overflow
    /// when the logits cluster at large magnitudes.
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

    /// Forward pass. Returns `(TaskType, confidence)` where confidence
    /// is the max softmax probability. Used by `classify` and by
    /// tests that want to avoid async.
    pub fn forward(&self, prompt: &str) -> (TaskType, f64) {
        let head: &str = if prompt.len() > self.max_prompt_chars {
            let cut = prompt
                .char_indices()
                .nth(self.max_prompt_chars)
                .map(|(b, _)| b)
                .unwrap_or(self.max_prompt_chars);
            &prompt[..cut]
        } else {
            prompt
        };
        let feats = extract_features(head);
        let mut logits = [0.0f64; N_CLASSES];
        for (i, logit) in logits.iter_mut().enumerate() {
            let mut z = self.intercept[i];
            for (j, feat) in feats.iter().enumerate() {
                z += self.weights[i][j] * *feat as f64;
            }
            *logit = z;
        }
        let probs = Self::logits_to_softmax(&logits);
        let (best_idx, best_prob) = probs
            .iter()
            .copied()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or((0, 0.0));
        (self.classes[best_idx], best_prob)
    }
}

#[async_trait]
impl TaskTypeClassifier for LogisticRegressionClassifier {
    async fn classify(&self, ctx: &ClassifierContext<'_>) -> Result<TaskVerdict, ClassifierError> {
        let (task_type, confidence) = self.forward(ctx.prompt);
        Ok(TaskVerdict {
            task_type,
            confidence,
            backend: ClassifierKind::LogReg,
        })
    }

    fn kind(&self) -> ClassifierKind {
        ClassifierKind::LogReg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::classifier::ClassifierContext;
    use crate::orchestrator::registry::Registry;

    fn empty_registry() -> &'static Registry {
        static EMPTY: std::sync::OnceLock<Registry> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Registry::default)
    }

    fn ctx<'a>(prompt: &'a str) -> ClassifierContext<'a> {
        ClassifierContext {
            prompt,
            mission_id: None,
            registry: empty_registry(),
        }
    }

    /// Build a synthetic weights JSON where each class i has weight
    /// `1.0` on feature i (and `0.0` on every other feature). This
    /// means `argmax(softmax(W·x+b)) == i` whenever feature i has the
    /// highest count, mirroring the lexical classifier's behaviour
    /// when no intercept dwarfs the feature signal.
    fn identity_weights_json() -> String {
        let w: Vec<Vec<f64>> = (0..N_CLASSES)
            .map(|i| {
                let mut row = vec![0.0f64; N_FEATURES];
                row[i] = 1.0;
                row
            })
            .collect();
        let classes: Vec<String> = TaskType::ALL
            .iter()
            .map(|t| t.as_str().to_string())
            .collect();
        serde_json::json!({
            "weights": w,
            "intercept": vec![0.0f64; N_CLASSES],
            "classes": classes,
            "feature_names": vec!["feat".to_string(); N_FEATURES],
        })
        .to_string()
    }

    #[test]
    fn forward_identity_weights_picks_correct_class() {
        let c = LogisticRegressionClassifier::from_json(&identity_weights_json(), 4096, "in-mem")
            .expect("identity weights parse ok");
        let (t, conf) = c.forward("write a function");
        assert_eq!(t, TaskType::Coding);
        assert!(conf > 0.0);
    }

    #[test]
    fn forward_unknown_when_no_signal() {
        let c = LogisticRegressionClassifier::from_json(&identity_weights_json(), 4096, "in-mem")
            .expect("identity weights parse ok");
        // No keywords — features all 0 → softmax uniform over 12
        // classes → argmax is class 0 by stability (we break ties by
        // the first index), so the predicted class is the FIRST
        // declared class in the weights file. We just check
        // confidence == 1/12 within float eps.
        let (_, conf) = c.forward("nothing here at all");
        assert!(
            (conf - 1.0 / N_CLASSES as f64).abs() < 1e-9,
            "uniform softmax"
        );
    }

    #[test]
    fn load_rejects_wrong_classes_count() {
        let mut j = serde_json::json!({
            "weights": vec![vec![0.0f64; N_FEATURES]; 2],
            "intercept": vec![0.0f64; 2],
            "classes": vec!["coding".to_string(), "test".to_string()],
        });
        let raw = j.to_string();
        let err = LogisticRegressionClassifier::from_json(&raw, 4096, "in-mem").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("12 weight rows"), "msg = {msg}");
        let _ = &mut j; // silence unused-mut if clippy ever complains
    }

    #[test]
    fn load_rejects_unknown_class_label() {
        let j = serde_json::json!({
            "weights": vec![vec![0.0f64; N_FEATURES]; N_CLASSES],
            "intercept": vec![0.0f64; N_CLASSES],
            "classes": (0..N_CLASSES).map(|i| format!("class{i}")).collect::<Vec<_>>(),
        });
        let raw = j.to_string();
        let err = LogisticRegressionClassifier::from_json(&raw, 4096, "in-mem").unwrap_err();
        assert!(format!("{err}").contains("unknown class label"));
    }

    #[test]
    fn load_rejects_wrong_feature_count_in_row() {
        let mut w = vec![vec![0.0f64; N_FEATURES]; N_CLASSES];
        w[3] = vec![0.0f64; N_FEATURES - 1];
        let j = serde_json::json!({
            "weights": w,
            "intercept": vec![0.0f64; N_CLASSES],
            "classes": TaskType::ALL.iter().map(|t| t.as_str().to_string()).collect::<Vec<_>>(),
        });
        let raw = j.to_string();
        let err = LogisticRegressionClassifier::from_json(&raw, 4096, "in-mem").unwrap_err();
        assert!(format!("{err}").contains("weight row 3"));
    }

    #[test]
    fn softmax_returns_uniform_for_equal_logits() {
        let logits = [0.0f64; N_CLASSES];
        let probs = LogisticRegressionClassifier::logits_to_softmax(&logits);
        for p in probs.iter() {
            assert!((p - 1.0 / N_CLASSES as f64).abs() < 1e-9);
        }
    }

    #[test]
    fn softmax_is_numerically_stable_for_large_logits() {
        let logits = [1000.0f64; N_CLASSES];
        let probs = LogisticRegressionClassifier::logits_to_softmax(&logits);
        let sum: f64 = probs.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9);
        for p in probs.iter() {
            assert!((p - 1.0 / N_CLASSES as f64).abs() < 1e-9);
        }
    }

    #[tokio::test]
    async fn classify_async_emits_logreg_backend() {
        let c = LogisticRegressionClassifier::from_json(&identity_weights_json(), 4096, "in-mem")
            .expect("identity weights parse ok");
        let v = c.classify(&ctx("fix the crash")).await.unwrap();
        assert_eq!(v.backend, ClassifierKind::LogReg);
        assert_eq!(v.task_type, TaskType::Fix);
    }

    #[test]
    fn load_from_disk_round_trips() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("weights.json");
        std::fs::write(&path, identity_weights_json()).unwrap();
        let path_str = path.to_str().unwrap();
        let c = LogisticRegressionClassifier::load(path_str, 4096).expect("load ok");
        let (t, _) = c.forward("write a function");
        assert_eq!(t, TaskType::Coding);
    }

    #[test]
    fn load_missing_file_returns_weights_file_error() {
        let err =
            LogisticRegressionClassifier::load("/nonexistent/weights.json", 4096).unwrap_err();
        match err {
            ClassifierError::WeightsFile { path, .. } => {
                assert_eq!(path, "/nonexistent/weights.json");
            }
            _ => panic!("expected WeightsFile variant"),
        }
    }
}
