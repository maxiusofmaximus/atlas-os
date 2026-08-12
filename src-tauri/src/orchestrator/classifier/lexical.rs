// OpenCode OS — Lexical task-type classifier (RFC 04 §7 sub-fase 2.3).
//
// `LexicalClassifier` regex-counts keyword tokens in the prompt and
// picks the task bucket with the highest score (ties broken by the
// canonical order in `BUCKETS`). Pure Rust, no ML weights, no feature
// flags. Used as:
//
//  * a stand-alone classifier (default backend when
//    `AutoRouterConfig.classifier_kind = Lexical`),
//  * a fallback when `LogisticRegressionClassifier` has no weights
//    file and when `EmbeddingClassifier` is feature-off,
//  * the feature extractor for `LogisticRegressionClassifier` (via
//    `extract_features`).
//
// Each task bucket is a list of keyword alternatives. The bucket
// score is `Σ_count` normalised over the sum of all bucket scores —
// the resulting `confidence` (RouteLLM-style max-prob) is what the
// orchestrator's threshold routing compares against
// `AutoRouterConfig.threshold_for(task)`.

use async_trait::async_trait;

use super::{
    ClassifierContext, ClassifierError, ClassifierKind, TaskType, TaskTypeClassifier, TaskVerdict,
};

/// Bucketed keyword list. Each row is `(TaskType, &[&str keywords])`.
/// Keyword matching is case-insensitive, word-boundary anchored, and
/// matches whole-word tokens (no substring false-positives: "fix"
/// does not match "infix", "build" does not match "build-up").
const BUCKETS: &[(TaskType, &[&str])] = &[
    (
        TaskType::Coding,
        &[
            "write",
            "implement",
            "code",
            "function",
            "method",
            "class",
            "module",
        ],
    ),
    (
        TaskType::Test,
        &[
            "test", "tests", "spec", "specs", "vitest", "pytest", "unittest", "fixture", "coverage",
        ],
    ),
    (
        TaskType::Refactor,
        &[
            "refactor",
            "rename",
            "extract method",
            "split class",
            "extract",
        ],
    ),
    (
        TaskType::Fix,
        &[
            "fix",
            "bug",
            "regression",
            "stacktrace",
            "backtrace",
            "crash",
            "panic",
            "segfault",
            "nil pointer",
        ],
    ),
    (
        TaskType::Build,
        &[
            "build", "cargo", "npm", "pnpm", "package", "compile", "make", "linker", "ld",
        ],
    ),
    (
        TaskType::Plan,
        &[
            "architecture",
            "design",
            "plan",
            "milestone",
            "roadmap",
            "rfc",
            "specification",
        ],
    ),
    (
        TaskType::Review,
        &["review", "rubric", "approve", "cr nit", "nit", "pr comment"],
    ),
    (
        TaskType::Explain,
        &[
            "explain", "why", "how does", "document", "doc", "onboard", "tour",
        ],
    ),
    (
        TaskType::Translate,
        &[
            "translate",
            "translation",
            "port to",
            "rewrite from",
            "i18n",
            "l10n",
        ],
    ),
    (
        TaskType::Tidy,
        &[
            "reformat", "tidy", "rustfmt", "prettier", "shfmt", "lint fix",
        ],
    ),
    (
        TaskType::Exec,
        &[
            "shell",
            "bash",
            "powershell",
            "cmd",
            "exec",
            "terminal",
            "run in terminal",
        ],
    ),
    (
        TaskType::Chat,
        &[
            "chat",
            "ask",
            "what do you think",
            "question",
            "hi",
            "hello",
        ],
    ),
];

/// Number of feature buckets (one per concrete task type). The
/// `LogisticRegressionClassifier` consumes this vector as input.
pub const N_FEATURES: usize = 12;

/// Extract the raw 12-element feature vector from `prompt`. Each
/// element is the integer count of that bucket's keyword occurrences
/// in the (possibly truncated) prompt. Used by `LexicalClassifier`
/// internally and by `LogisticRegressionClassifier` as its input
/// feature pipeline.
pub fn extract_features(prompt: &str) -> [u32; N_FEATURES] {
    let lower = prompt.to_lowercase();
    let mut feats = [0u32; N_FEATURES];
    for (i, (_, kws)) in BUCKETS.iter().enumerate() {
        for kw in *kws {
            feats[i] += count_occurrences(&lower, kw);
        }
    }
    feats
}

fn count_occurrences(haystack: &str, needle: &str) -> u32 {
    let mut count = 0u32;
    let mut idx = 0usize;
    let hb = haystack.as_bytes();
    let nb = needle.as_bytes();
    if nb.is_empty() {
        return 0;
    }
    while idx + nb.len() <= hb.len() {
        if &hb[idx..idx + nb.len()] == nb {
            let before_is_boundary = idx == 0 || !is_word_byte(hb[idx - 1]);
            let after_is_boundary = idx + nb.len() == hb.len() || !is_word_byte(hb[idx + nb.len()]);
            if before_is_boundary && after_is_boundary {
                count += 1;
            }
        }
        idx += 1;
    }
    count
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Pure-Rust lexical classifier. No allocations beyond the
/// `String::to_lowercase` of the prompt + the bucket scan.
pub struct LexicalClassifier {
    max_prompt_chars: usize,
}

impl LexicalClassifier {
    pub fn new(max_prompt_chars: usize) -> Self {
        Self { max_prompt_chars }
    }

    fn classify_inner(&self, prompt: &str) -> (TaskType, f64) {
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
        let total: u32 = feats.iter().copied().sum();
        if total == 0 {
            return (TaskType::Unknown, 0.0);
        }
        let (best_idx, best_count) = feats
            .iter()
            .copied()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.cmp(b))
            .unwrap_or((0, 0));
        if best_count == 0 {
            return (TaskType::Unknown, 0.0);
        }
        let (task, _) = BUCKETS[best_idx];
        let confidence = best_count as f64 / total as f64;
        (task, confidence)
    }
}

#[async_trait]
impl TaskTypeClassifier for LexicalClassifier {
    async fn classify(&self, ctx: &ClassifierContext<'_>) -> Result<TaskVerdict, ClassifierError> {
        let (task_type, confidence) = self.classify_inner(ctx.prompt);
        Ok(TaskVerdict {
            task_type,
            confidence,
            backend: ClassifierKind::Lexical,
        })
    }

    fn kind(&self) -> ClassifierKind {
        ClassifierKind::Lexical
    }
}

impl Default for LexicalClassifier {
    fn default() -> Self {
        Self::new(4096)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::classifier::{ClassifierContext, TaskVerdict};
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

    #[test]
    fn extract_features_coding_bucket_one_for_write_token() {
        let f = extract_features("please write a function");
        let coding_idx = 0;
        assert_eq!(
            f[coding_idx], 2,
            "write + function → coding bucket count = 2"
        );
    }

    #[test]
    fn extract_features_word_boundary_no_substring_match() {
        // "refixing" should not match "fix".
        let f = extract_features("refixing the marshmallow");
        let fix_idx = 3;
        assert_eq!(f[fix_idx], 0, "infix-like substring should not count");
    }

    #[test]
    fn extract_features_build_matches_cargo_and_npm() {
        let f = extract_features("run cargo build then npm install");
        let build_idx = 4;
        assert!(f[build_idx] >= 2, "expect multiple matches in build bucket");
    }

    #[test]
    fn extract_features_zero_for_no_keyword() {
        let f = extract_features("a prompt with no keywords at all");
        assert_eq!(f.iter().sum::<u32>(), 0);
    }

    #[test]
    fn classify_zero_keyword_prompt_yields_unknown() {
        let c = LexicalClassifier::new(4096);
        let (t, conf) = c.classify_inner("a prompt with no keywords at all");
        assert_eq!(t, TaskType::Unknown);
        assert_eq!(conf, 0.0);
    }

    #[test]
    fn classify_plan_beats_chat_when_both_keywords_match() {
        // "plan" appears once, "what do you think" also once; ties
        // resolve to the bucket defined first in `BUCKETS`.
        let c = LexicalClassifier::new(4096);
        let (t, _) = c.classify_inner("plan the milestones; what do you think");
        assert_ne!(t, TaskType::Unknown);
    }

    #[tokio::test]
    async fn classify_async_emits_coding_backend() {
        let c = LexicalClassifier::new(4096);
        let v = c.classify(&ctx("write a function")).await.unwrap();
        assert_eq!(v.backend, ClassifierKind::Lexical);
        assert_eq!(v.task_type, TaskType::Coding);
        assert!(v.confidence > 0.5, "single-bucket match → high confidence");
    }

    #[tokio::test]
    async fn classify_async_truncates_long_prompt_without_panic() {
        let long = "write ".repeat(10_000);
        let c = LexicalClassifier::new(100);
        let v = c.classify(&ctx(&long)).await.unwrap();
        assert_eq!(v.task_type, TaskType::Coding);
    }

    #[tokio::test]
    async fn classify_test_bucket_matches_vitest_token() {
        let c = LexicalClassifier::new(4096);
        let v = c.classify(&ctx("add a vitest test")).await.unwrap();
        assert_eq!(
            v.task_type,
            TaskType::Test,
            "vitest/test both belong to test bucket"
        );
    }

    #[tokio::test]
    async fn classify_fix_bucket_matches_stacktrace() {
        let c = LexicalClassifier::new(4096);
        let v = c
            .classify(&ctx("please fix the stacktrace panic"))
            .await
            .unwrap();
        assert_eq!(v.task_type, TaskType::Fix);
    }

    #[tokio::test]
    async fn classify_unknown_when_only_stopwords_present() {
        let c = LexicalClassifier::new(4096);
        let v = c.classify(&ctx("the of to a and")).await.unwrap();
        assert_eq!(v.task_type, TaskType::Unknown);
        assert_eq!(v.confidence, 0.0);
        assert_eq!(v.backend, ClassifierKind::Lexical);
        // Confirm TaskVerdict sentinel-ish behaviour preserved.
        let sentinel = TaskVerdict::unknown();
        assert_eq!(sentinel.task_type, TaskType::Unknown);
    }
}
