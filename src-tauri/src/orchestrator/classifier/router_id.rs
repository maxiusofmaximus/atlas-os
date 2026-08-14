// Atlas OS — Router selector via `model` field (RFC 04 §7 sub-fase 2.3,
// research/29 line 243 "non-obvious pattern #1").
//
// The caller's `model` string can be a literal model id (`"gpt-5"`) OR
// a router pseudo-id that the orchestrator's runtime expands into a
// routing policy branch. The point (per research/29) is that the
// caller emits an OpenAI-shaped request without needing to know
// whether the model id is a router aggregate or a concrete model —
// the runtime branch is internal.
//
// Two recognised router pseudo-id shapes:
//
//   * `router-auto-<strong_pct>` — RouteLLM `auto` strategy. The
//     `<strong_pct>` is a fraction [0.0, 1.0] indicating what share of
//     prompts should route to the strong model. The orchestrator then
//     picks strong vs weak per the auto-router's confidence threshold
//     (`AutoRouterConfig.thresholds`). Example: `router-auto-0.5`.
//   * `router-mf-<threshold_pct>` — RouteLLM `mf` (matrix factorisation)
//     strategy. The `<threshold_pct>` is the calibrated threshold the
//     classifier's `confidence` is compared against. Phase-2.3 default
//     is `router-mf-0.116` (research/29 line 244 — `coding=0.116`).
//
// Any other string is treated as a literal model id and the orchestrator
// falls through to the regular `RoutingStrategy::select` path.
//
// Recognising the prefix here lets the orchestrator runtime branch
// without parsing the request envelope in the hot path — a single
// `RouterId::parse(model_str)` call at request intake decides whether
// auto-routing should engage.

use serde::{Deserialize, Serialize};

/// Router pseudo-id recognised in the caller's `model` field. Falls
/// back to `Literal` for any string that does not match the `router-*`
/// prefix shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RouterId {
    /// A literal model id (`"gpt-5"`, `"claude-opus-4"`). The
    /// orchestrator routes via the strategy-only pipeline.
    Literal(String),
    /// RouteLLM `auto` strategy. The string carries the target
    /// strong-model share as a fraction in [0.0, 1.0]. Example:
    /// `router-auto-0.5` → `Auto { strong_pct: 0.5 }`.
    Auto { strong_pct: f64 },
    /// RouteLLM `mf` (matrix factorisation) strategy. The string
    /// carries the calibrated confidence threshold the classifier's
    /// `confidence` field is compared against. Example:
    /// `router-mf-0.116` → `Mf { threshold: 0.116 }`.
    Mf { threshold: f64 },
}

/// Discriminator string consumed by `parse`.
pub const ROUTER_AUTO_PREFIX: &str = "router-auto-";
pub const ROUTER_MF_PREFIX: &str = "router-mf-";

/// Policy kind the orchestrator runtime branches on. Decoupled from
/// `RouterId` so callers can `match kind` without destructuring the
/// payload (which matters when the orchestrator is the only one that
/// needs the threshold value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouterKind {
    Literal,
    Auto,
    Mf,
}

impl RouterId {
    /// Parse a `model` string. Unknown prefixes / malformed fractions
    /// fall back to `Literal` rather than erroring — the orchestrator
    /// must never refuse a request because a model id did not parse
    /// as a router pseudo-id (the literal fallback covers every other
    /// id shape that's already valid).
    pub fn parse(s: &str) -> Self {
        if let Some(rest) = s.strip_prefix(ROUTER_AUTO_PREFIX) {
            return match rest.parse::<f64>() {
                Ok(p) if (0.0..=1.0).contains(&p) => Self::Auto { strong_pct: p },
                _ => Self::Literal(s.to_string()),
            };
        }
        if let Some(rest) = s.strip_prefix(ROUTER_MF_PREFIX) {
            return match rest.parse::<f64>() {
                Ok(t) if (0.0..=1.0).contains(&t) => Self::Mf { threshold: t },
                _ => Self::Literal(s.to_string()),
            };
        }
        Self::Literal(s.to_string())
    }

    /// Return the kind discriminator without deconstructing the enum.
    pub fn kind(&self) -> RouterKind {
        match self {
            Self::Literal(_) => RouterKind::Literal,
            Self::Auto { .. } => RouterKind::Auto,
            Self::Mf { .. } => RouterKind::Mf,
        }
    }

    /// True when the parse identified a router pseudo-id (Auto or Mf).
    /// Equivalent to `self.kind() != RouterKind::Literal` but more
    /// ergonomic at call-sites.
    pub fn is_router(&self) -> bool {
        !matches!(self, Self::Literal(_))
    }
}

impl Default for RouterId {
    fn default() -> Self {
        Self::Literal(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_literal_for_plain_model_id() {
        match RouterId::parse("gpt-5") {
            RouterId::Literal(s) => assert_eq!(s, "gpt-5"),
            other => panic!("expected Literal, got {other:?}"),
        }
    }

    #[test]
    fn parse_auto_for_router_auto_0_5_prefix() {
        match RouterId::parse("router-auto-0.5") {
            RouterId::Auto { strong_pct } => assert!((strong_pct - 0.5).abs() < 1e-9),
            other => panic!("expected Auto, got {other:?}"),
        }
    }

    #[test]
    fn parse_mf_for_router_mf_0_116_prefix() {
        match RouterId::parse("router-mf-0.116") {
            RouterId::Mf { threshold } => assert!((threshold - 0.116).abs() < 1e-9),
            other => panic!("expected Mf, got {other:?}"),
        }
    }

    #[test]
    fn parse_auto_rejects_strong_pct_above_1() {
        // Out-of-range fraction falls back to Literal rather than erroring.
        let r = RouterId::parse("router-auto-2.0");
        assert!(matches!(r, RouterId::Literal(_)));
        assert!(!r.is_router());
    }

    #[test]
    fn parse_mf_rejects_negative_threshold() {
        let r = RouterId::parse("router-mf--0.1");
        assert!(matches!(r, RouterId::Literal(_)));
    }

    #[test]
    fn parse_auto_rejects_non_numeric_strong_pct() {
        let r = RouterId::parse("router-auto-strong");
        match &r {
            RouterId::Literal(s) => assert_eq!(s, "router-auto-strong"),
            other => panic!("expected Literal, got {other:?}"),
        }
    }

    #[test]
    fn parse_mf_rejects_non_numeric_threshold() {
        let r = RouterId::parse("router-mf-default");
        match &r {
            RouterId::Literal(s) => assert_eq!(s, "router-mf-default"),
            other => panic!("expected Literal, got {other:?}"),
        }
    }

    #[test]
    fn kind_returns_correct_discriminator() {
        assert_eq!(RouterId::parse("gpt-5").kind(), RouterKind::Literal);
        assert_eq!(RouterId::parse("router-auto-0.5").kind(), RouterKind::Auto);
        assert_eq!(RouterId::parse("router-mf-0.116").kind(), RouterKind::Mf);
    }

    #[test]
    fn is_router_true_only_for_pseudo_ids() {
        assert!(!RouterId::parse("gpt-5").is_router());
        assert!(RouterId::parse("router-auto-0.5").is_router());
        assert!(RouterId::parse("router-mf-0.116").is_router());
    }

    #[test]
    fn parse_handles_empty_string_as_literal() {
        let r = RouterId::parse("");
        match &r {
            RouterId::Literal(s) => assert!(s.is_empty()),
            other => panic!("expected Literal, got {other:?}"),
        }
        assert!(!r.is_router());
    }

    #[test]
    fn parse_preserves_router_prefix_when_trailing_garbage() {
        let r = RouterId::parse("router-auto-0.5-foo");
        assert!(matches!(r, RouterId::Literal(_)));
    }

    #[test]
    fn default_is_empty_literal() {
        let r = RouterId::default();
        assert!(matches!(r, RouterId::Literal(s) if s.is_empty()));
    }
}
