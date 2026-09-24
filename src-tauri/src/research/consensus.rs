// Atlas OS — Consensus scorer trait (RFC 10 §5, Phase 3 sub-fase 3.0).
//
// Collective Engineering Intelligence formalised: one scorer per
// dimension (Community, Enterprise, Academic, Official). Sub-fase 3.3
// provides the production impls (GitHub Issues/Discussions via `gh`,
// arXiv via webfetch, docs via the 3.1 gateway); this module only
// fixes the trait shape plus the shared invariants so 3.3 can land
// without touching callers:
//
// * scores are 0..100 with cited references (RFC 10 §5);
// * fewer than `MIN_SOURCES` inputs is "no data" (`None`) — same
//   floor as `AffinityRow::MIN_SAMPLES` and the A.4 weak-model rule
//   (`n_samples < 3` → no signal, strong model never synthesises
//   from noise).

use serde::{Deserialize, Serialize};

/// RFC 10 §5 — the four consensus dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusDimension {
    Community,
    Enterprise,
    Academic,
    Official,
}

impl ConsensusDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConsensusDimension::Community => "community",
            ConsensusDimension::Enterprise => "enterprise",
            ConsensusDimension::Academic => "academic",
            ConsensusDimension::Official => "official",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "community" => Some(ConsensusDimension::Community),
            "enterprise" => Some(ConsensusDimension::Enterprise),
            "academic" => Some(ConsensusDimension::Academic),
            "official" => Some(ConsensusDimension::Official),
            _ => None,
        }
    }

    pub const ALL: [ConsensusDimension; 4] = [
        ConsensusDimension::Community,
        ConsensusDimension::Enterprise,
        ConsensusDimension::Academic,
        ConsensusDimension::Official,
    ];
}

/// Minimum evidence floor before a scorer may emit a signal.
pub const MIN_SOURCES: usize = 3;

/// One pre-filtered evidence item handed to a scorer. The weak model
/// (A.4) classifies and de-noises raw hits before this point; the
/// scorer only sees items worth scoring.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceInput {
    pub url: String,
    pub kind: String,
    pub weight: f64,
}

impl SourceInput {
    pub fn new(url: &str, kind: &str, weight: f64) -> Self {
        Self {
            url: url.into(),
            kind: kind.into(),
            weight,
        }
    }
}

/// Scored outcome of one dimension: 0..100 plus the cited references
/// that back it (RFC 10 §5 "score (0–100) y referencias citadas").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConsensusScore {
    pub dimension: ConsensusDimension,
    pub score: f64,
    pub references: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
}

impl ConsensusScore {
    pub fn validate(&self) -> Result<(), ConsensusError> {
        if !self.score.is_finite() || self.score < 0.0 || self.score > 100.0 {
            return Err(ConsensusError::ScoreOutOfRange { value: self.score });
        }
        if self.references.is_empty() {
            return Err(ConsensusError::NoReferences);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum ConsensusError {
    #[error("consensus score {value} out of range (expected 0.0..=100.0)")]
    ScoreOutOfRange { value: f64 },
    #[error("consensus score carries no cited references")]
    NoReferences,
}

/// Trait every dimension scorer implements. Synchronous and
/// deterministic: given the same inputs the scorer returns the same
/// score, which is what makes the sub-fase 3.3 fixture tests stable.
/// Returns `None` when `sources.len() < MIN_SOURCES` ("no data").
pub trait ConsensusScorer: Send + Sync {
    fn dimension(&self) -> ConsensusDimension;
    fn score_sources(&self, sources: &[SourceInput]) -> Option<ConsensusScore>;
}

/// Weighted mean of per-dimension scores into a 0..1 confidence.
/// Hands-on expert notes weigh ×1.5 (RFC 10 §3 evidence hierarchy).
/// Returns `None` when `scores` is empty.
pub fn combined_confidence(scores: &[(ConsensusDimension, f64, f64)]) -> Option<f64> {
    if scores.is_empty() {
        return None;
    }
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;
    for (_, score, weight) in scores {
        if !score.is_finite() || !weight.is_finite() || *weight <= 0.0 {
            continue;
        }
        weighted_sum += score.clamp(0.0, 100.0) * weight;
        weight_total += weight;
    }
    if weight_total <= 0.0 {
        return None;
    }
    Some((weighted_sum / weight_total) / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedScorer {
        dimension: ConsensusDimension,
        score: f64,
    }

    impl ConsensusScorer for FixedScorer {
        fn dimension(&self) -> ConsensusDimension {
            self.dimension
        }

        fn score_sources(&self, sources: &[SourceInput]) -> Option<ConsensusScore> {
            if sources.len() < MIN_SOURCES {
                return None;
            }
            Some(ConsensusScore {
                dimension: self.dimension,
                score: self.score,
                references: sources.iter().map(|s| s.url.clone()).collect(),
                note: None,
            })
        }
    }

    fn three_sources() -> Vec<SourceInput> {
        vec![
            SourceInput::new("https://example.com/a", "blog", 1.0),
            SourceInput::new("https://example.com/b", "blog", 1.0),
            SourceInput::new("https://example.com/c", "docs", 1.5),
        ]
    }

    #[test]
    fn dimension_roundtrips_through_as_str_parse() {
        for d in ConsensusDimension::ALL {
            assert_eq!(ConsensusDimension::parse(d.as_str()), Some(d));
        }
        let json = serde_json::to_string(&ConsensusDimension::Official).unwrap();
        assert_eq!(json, "\"official\"");
    }

    #[test]
    fn dimension_rejects_unknown_tag() {
        assert_eq!(ConsensusDimension::parse("press"), None);
        assert_eq!(ConsensusDimension::parse(""), None);
    }

    #[test]
    fn scorer_emits_deterministic_score() {
        let scorer = FixedScorer {
            dimension: ConsensusDimension::Community,
            score: 71.0,
        };
        let out = scorer.score_sources(&three_sources()).expect("score");
        assert_eq!(out.dimension, ConsensusDimension::Community);
        assert_eq!(out.score, 71.0);
        assert_eq!(out.references.len(), 3);
        out.validate().unwrap();
    }

    #[test]
    fn scorer_returns_no_data_below_min_sources() {
        let scorer = FixedScorer {
            dimension: ConsensusDimension::Academic,
            score: 55.0,
        };
        let few = &three_sources()[..MIN_SOURCES - 1];
        assert!(scorer.score_sources(few).is_none());
        assert!(scorer.score_sources(&[]).is_none());
    }

    #[test]
    fn score_validation_rejects_out_of_range() {
        let bad = ConsensusScore {
            dimension: ConsensusDimension::Enterprise,
            score: 120.0,
            references: vec!["https://example.com".into()],
            note: None,
        };
        assert_eq!(
            bad.validate().unwrap_err(),
            ConsensusError::ScoreOutOfRange { value: 120.0 }
        );
    }

    #[test]
    fn score_validation_rejects_missing_references() {
        let bad = ConsensusScore {
            dimension: ConsensusDimension::Official,
            score: 78.0,
            references: vec![],
            note: None,
        };
        assert_eq!(bad.validate().unwrap_err(), ConsensusError::NoReferences);
    }

    #[test]
    fn combined_confidence_weights_hands_on_notes() {
        let conf = combined_confidence(&[
            (ConsensusDimension::Community, 70.0, 1.0),
            (ConsensusDimension::Official, 80.0, 1.5),
        ])
        .unwrap();
        let expected = (70.0 * 1.0 + 80.0 * 1.5) / 2.5 / 100.0;
        assert!((conf - expected).abs() < 1e-9);
    }

    #[test]
    fn combined_confidence_empty_is_no_data() {
        assert_eq!(combined_confidence(&[]), None);
    }
}
