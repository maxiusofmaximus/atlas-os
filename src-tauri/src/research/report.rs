// Atlas OS — Research Engine report types (RFC 10 §7, Phase 3 sub-fase 3.0).
//
// Canonical `ResearchRunReport` mirrors the YAML shape in RFC 10 §7
// byte-for-byte so a SvelteKit HUD, an external MCP consumer, or the
// `atlas research query` CLI (sub-fase 3.3) can `YAML.parse` either side
// and get the same structure. The JSON blob is the cross-RFC contract;
// the M25 SQLite tables (`research_runs` / `research_sources` /
// `research_consensus`) only duplicate the columns the SQL engine needs
// for indexing (`status`, `confidence`, `recommended`).

use serde::{Deserialize, Serialize};

use super::consensus::ConsensusDimension;

/// RFC 10 §1 — run kind. The Planning / Prompt Understanding engines pick
/// the kind; the operator forces one with `/research`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchRunKind {
    Full,
    Targeted,
    Mega,
}

impl ResearchRunKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResearchRunKind::Full => "full",
            ResearchRunKind::Targeted => "targeted",
            ResearchRunKind::Mega => "mega",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "full" => Some(ResearchRunKind::Full),
            "targeted" => Some(ResearchRunKind::Targeted),
            "mega" => Some(ResearchRunKind::Mega),
            _ => None,
        }
    }

    pub const ALL: [ResearchRunKind; 3] = [
        ResearchRunKind::Full,
        ResearchRunKind::Targeted,
        ResearchRunKind::Mega,
    ];
}

/// RFC 10 §10 — fail-safe run status. `NeedingHuman` is the hard
/// anti-hallucination escape: confidence below threshold produces no
/// recommendation, only a replan request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchRunStatus {
    Running,
    Completed,
    NeedingHuman,
}

impl ResearchRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResearchRunStatus::Running => "running",
            ResearchRunStatus::Completed => "completed",
            ResearchRunStatus::NeedingHuman => "needing_human",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "running" => Some(ResearchRunStatus::Running),
            "completed" => Some(ResearchRunStatus::Completed),
            "needing_human" => Some(ResearchRunStatus::NeedingHuman),
            _ => None,
        }
    }
}

/// One consensus line of the RFC 10 §7 `consensus:` map: which strategy
/// the dimension converges on plus its 0..1 score.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConsensusEntry {
    pub recommendation: String,
    pub score: f64,
}

impl ConsensusEntry {
    pub fn validate(&self) -> Result<(), ReportError> {
        if !self.score.is_finite() || self.score < 0.0 || self.score > 1.0 {
            return Err(ReportError::ScoreOutOfRange { value: self.score });
        }
        if self.recommendation.trim().is_empty() {
            return Err(ReportError::EmptyRecommendation);
        }
        Ok(())
    }
}

/// RFC 10 §5/§7 — the four consensus dimensions. Field names match the
/// YAML keys (`community`, `enterprise`, `academic`, `official`) so
/// `serde_yaml` round-trips without renames.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResearchConsensus {
    pub community: ConsensusEntry,
    pub enterprise: ConsensusEntry,
    pub academic: ConsensusEntry,
    pub official: ConsensusEntry,
}

impl ResearchConsensus {
    pub fn entry(&self, dimension: ConsensusDimension) -> &ConsensusEntry {
        match dimension {
            ConsensusDimension::Community => &self.community,
            ConsensusDimension::Enterprise => &self.enterprise,
            ConsensusDimension::Academic => &self.academic,
            ConsensusDimension::Official => &self.official,
        }
    }

    pub fn validate(&self) -> Result<(), ReportError> {
        self.community.validate()?;
        self.enterprise.validate()?;
        self.academic.validate()?;
        self.official.validate()?;
        Ok(())
    }
}

/// RFC 10 §7 — canonical research-run report.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResearchRunReport {
    pub id: String,
    pub query: String,
    #[serde(default = "default_source_count")]
    pub sources: u32,
    pub consensus: ResearchConsensus,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub enterprise: Vec<String>,
    #[serde(default)]
    pub bugs: Vec<String>,
    #[serde(default)]
    pub proposal: Vec<String>,
    pub confidence: f64,
    pub recommended: String,
    #[serde(default)]
    pub journal_ref: Option<String>,
}

fn default_source_count() -> u32 {
    0
}

impl ResearchRunReport {
    pub fn validate(&self) -> Result<(), ReportError> {
        if self.id.trim().is_empty() {
            return Err(ReportError::EmptyId);
        }
        if self.query.trim().is_empty() {
            return Err(ReportError::EmptyQuery);
        }
        if !self.confidence.is_finite() || self.confidence < 0.0 || self.confidence > 1.0 {
            return Err(ReportError::ScoreOutOfRange {
                value: self.confidence,
            });
        }
        if self.recommended.trim().is_empty() {
            return Err(ReportError::EmptyRecommendation);
        }
        self.consensus.validate()?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum ReportError {
    #[error("score {value} out of range (expected 0.0..=1.0)")]
    ScoreOutOfRange { value: f64 },
    #[error("recommendation must not be empty")]
    EmptyRecommendation,
    #[error("report id must not be empty")]
    EmptyId,
    #[error("report query must not be empty")]
    EmptyQuery,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(rec: &str, score: f64) -> ConsensusEntry {
        ConsensusEntry {
            recommendation: rec.into(),
            score,
        }
    }

    fn sample() -> ResearchRunReport {
        ResearchRunReport {
            id: "rr-2026-07-04-001".into(),
            query: "Event sourcing vs CRDT para nuestro ledger?".into(),
            sources: 87,
            consensus: ResearchConsensus {
                community: entry("strategy_pattern", 0.71),
                enterprise: entry("event_sourcing", 0.66),
                academic: entry("crdt", 0.55),
                official: entry("event_sourcing", 0.78),
            },
            authors: vec!["Martin Kleppmann -> crdt".into()],
            enterprise: vec!["Microsoft -> event_sourcing".into()],
            bugs: vec!["GitHub issue #1234: crash bajo X carga en Kafka 3.7".into()],
            proposal: vec!["A: Event Sourcing + Kafka, fits actual stack".into()],
            confidence: 0.81,
            recommended: "A".into(),
            journal_ref: Some("jr-2026-07-04-001".into()),
        }
    }

    #[test]
    fn report_roundtrips_through_json() {
        let r = sample();
        let json = serde_json::to_string(&r).unwrap();
        let back: ResearchRunReport = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
        back.validate().unwrap();
    }

    #[test]
    fn report_roundtrips_through_yaml() {
        let r = sample();
        let yaml = serde_yaml::to_string(&r).unwrap();
        assert!(yaml.contains("community:"));
        assert!(yaml.contains("confidence: 0.81"));
        let back: ResearchRunReport = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn kind_roundtrips_through_as_str_parse() {
        for k in ResearchRunKind::ALL {
            assert_eq!(ResearchRunKind::parse(k.as_str()), Some(k));
        }
        let json = serde_json::to_string(&ResearchRunKind::Mega).unwrap();
        assert_eq!(json, "\"mega\"");
        let back: ResearchRunKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ResearchRunKind::Mega);
    }

    #[test]
    fn status_roundtrips_through_as_str_parse() {
        assert_eq!(
            ResearchRunStatus::parse("needing_human"),
            Some(ResearchRunStatus::NeedingHuman)
        );
        assert_eq!(ResearchRunStatus::Completed.as_str(), "completed");
    }

    #[test]
    fn validate_rejects_confidence_above_one() {
        let mut r = sample();
        r.confidence = 1.5;
        let err = r.validate().unwrap_err();
        assert_eq!(err, ReportError::ScoreOutOfRange { value: 1.5 });
    }

    #[test]
    fn validate_rejects_empty_recommended() {
        let mut r = sample();
        r.recommended = "   ".into();
        assert_eq!(r.validate().unwrap_err(), ReportError::EmptyRecommendation);
    }

    #[test]
    fn validate_rejects_unknown_kind_string() {
        assert_eq!(ResearchRunKind::parse("ultra"), None);
        assert_eq!(ResearchRunStatus::parse("done"), None);
    }
}
