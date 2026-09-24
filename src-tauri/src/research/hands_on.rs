// Atlas OS — Hands-on expert notes + application branches (RFC 10 §3–§4, Phase 3 sub-fase 3.4).
//
// Two halves of the same sub-fase share one module because they meet in the
// report: operator-attached closed cases (`ResearchNote`, "esto yo lo hice
// así (profesionalmente)") weigh as expert evidence, and `ApplicationBranch`
// ("cómo podrías hacerlo tú", Opción A/B/C) folds consensus + docs + notes +
// project fit into one citable proposal per branch. Both are deterministic
// and dependency-free (serde only, RFC 25 §11): the same inputs always yield
// the same branches, which is what keeps the fixture tests stable. The
// strong model still owns the final prose; this module owns the shape.

use serde::{Deserialize, Serialize};

use super::collective::DimensionOutcome;
use super::consensus::ConsensusDimension;

/// RFC 10 §3 — one closed real-world case attached by the operator (or the
/// team). Treated as expert evidence: it weighs more than any blog
/// (×1.5 via `HANDS_ON_WEIGHT` in the confidence fold) and every branch it
/// backs is marked `backed by hands-on note` so the audit trail shows why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResearchNote {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub decision: String,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default = "default_note_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub attached_at: String,
    #[serde(default)]
    pub signature: String,
}

fn default_note_confidence() -> f64 {
    0.8
}

impl ResearchNote {
    pub fn validate(&self) -> Result<(), HandsOnError> {
        if self.id.trim().is_empty() {
            return Err(HandsOnError::EmptyId);
        }
        if self.title.trim().is_empty() {
            return Err(HandsOnError::EmptyTitle);
        }
        if self.decision.trim().is_empty() {
            return Err(HandsOnError::EmptyDecision);
        }
        if !self.confidence.is_finite() || self.confidence < 0.0 || self.confidence > 1.0 {
            return Err(HandsOnError::ConfidenceOutOfRange {
                value: self.confidence,
            });
        }
        if self.signature.trim().is_empty() {
            return Err(HandsOnError::EmptySignature);
        }
        if self.attached_at.trim().is_empty() {
            return Err(HandsOnError::EmptyAttachedAt);
        }
        Ok(())
    }

    pub fn mentions(&self, needle: &str) -> bool {
        let n = needle.trim().to_ascii_lowercase();
        if n.is_empty() {
            return false;
        }
        let hay = format!(
            "{} {} {} {}",
            self.title,
            self.decision,
            self.outcome.as_deref().unwrap_or(""),
            self.tags.join(" ")
        );
        let hay = hay.to_ascii_lowercase();
        hay.contains(&n) || n.split_whitespace().any(|w| w.len() > 3 && hay.contains(w))
    }
}

/// RFC 10 §4 — one "cómo podrías hacerlo tú" branch. Each branch joins what
/// the community says, what the official docs recommend, and how it fits the
/// current architecture (Context Engine Project Map hint), plus a cost
/// estimate in this codebase. `sources` cites the audit trail
/// (`consensus:<dimension>:<recommendation>` plus `note:<id>` for every
/// backing expert note) so the YAML stays citable offline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApplicationBranch {
    pub id: String,
    pub summary: String,
    #[serde(default)]
    pub pros: Vec<String>,
    #[serde(default)]
    pub cons: Vec<String>,
    pub confidence: f64,
    #[serde(default)]
    pub cost_estimate: String,
    #[serde(default)]
    pub fits_stack: String,
    #[serde(default)]
    pub sources: Vec<String>,
}

impl ApplicationBranch {
    pub fn validate(&self) -> Result<(), HandsOnError> {
        if self.id.trim().is_empty() {
            return Err(HandsOnError::EmptyBranchId);
        }
        if self.summary.trim().is_empty() {
            return Err(HandsOnError::EmptyBranchSummary);
        }
        if self.pros.is_empty() {
            return Err(HandsOnError::EmptyPros);
        }
        if self.cons.is_empty() {
            return Err(HandsOnError::EmptyCons);
        }
        if !self.confidence.is_finite() || self.confidence < 0.0 || self.confidence > 1.0 {
            return Err(HandsOnError::ConfidenceOutOfRange {
                value: self.confidence,
            });
        }
        if self.cost_estimate.trim().is_empty() {
            return Err(HandsOnError::EmptyCostEstimate);
        }
        if self.sources.is_empty() {
            return Err(HandsOnError::EmptySources);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum HandsOnError {
    #[error("research note id must not be empty")]
    EmptyId,
    #[error("research note title must not be empty")]
    EmptyTitle,
    #[error("research note decision must not be empty")]
    EmptyDecision,
    #[error("research note signature must not be empty")]
    EmptySignature,
    #[error("research note attached_at must not be empty")]
    EmptyAttachedAt,
    #[error("confidence {value} out of range (expected 0.0..=1.0)")]
    ConfidenceOutOfRange { value: f64 },
    #[error("branch id must not be empty")]
    EmptyBranchId,
    #[error("branch summary must not be empty")]
    EmptyBranchSummary,
    #[error("branch pros must not be empty")]
    EmptyPros,
    #[error("branch cons must not be empty")]
    EmptyCons,
    #[error("branch cost_estimate must not be empty")]
    EmptyCostEstimate,
    #[error("branch sources must not be empty")]
    EmptySources,
}

/// Split a CLI `--tags "a, b, c"` value into trimmed non-empty tags.
pub fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

/// Derive the auditable journal reference for a run: the run id with its
/// leading `rr-` swapped for `jr-` (RFC 10 §7 `journal_ref`), or `jr-`
/// prefixed when the run id carries no such prefix.
pub fn journal_ref_for_run(run_id: &str) -> String {
    let run_id = run_id.trim();
    if let Some(rest) = run_id.strip_prefix("rr-") {
        format!("jr-{rest}")
    } else if let Some(rest) = run_id.strip_prefix("jr-") {
        format!("jr-{rest}")
    } else {
        format!("jr-{run_id}")
    }
}

fn cost_estimate_for(score_100: f64) -> &'static str {
    if score_100 >= 75.0 {
        "low adaptation cost — fits actual stack"
    } else if score_100 >= 60.0 {
        "moderate cost — verify integration points before lock"
    } else {
        "high uncertainty — spike first, do not lock the plan"
    }
}

/// Build the Opción A/B/C branches from ranked dimension outcomes plus the
/// persisted expert notes. Deterministic: outcomes sort by score desc
/// (ties break in `ConsensusDimension::ALL` order), the top three become
/// A/B/C, and a note backs a branch when the branch recommendation (or its
/// dimension) appears in the note text. Expert backing lifts confidence by
/// +0.05 (capped at 1.0) — the visible half of the ×1.5 fold weight. Empty
/// (all `no-data`) outcomes yield no branches instead of invented options.
pub fn build_branches(
    outcomes: &[DimensionOutcome],
    notes: &[ResearchNote],
    project_hint: Option<&str>,
) -> Vec<ApplicationBranch> {
    let mut ranked: Vec<&DimensionOutcome> = outcomes
        .iter()
        .filter(|o| o.recommendation.trim() != "no-data" && o.score_100.is_finite())
        .collect();
    ranked.sort_by(|a, b| {
        b.score_100
            .partial_cmp(&a.score_100)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| order_of(a.dimension).cmp(&order_of(b.dimension)))
    });
    let lowest = ranked.last().map(|o| o.recommendation.clone());
    let hint = project_hint
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .unwrap_or("unknown stack — confirm via Context Engine Project Map");
    ranked
        .into_iter()
        .take(3)
        .enumerate()
        .map(|(i, o)| {
            let tag = if i < 26 {
                ((b'A' + i as u8) as char).to_string()
            } else {
                format!("P{}", i + 1)
            };
            let backing: Vec<&ResearchNote> = notes
                .iter()
                .filter(|n| n.mentions(&o.recommendation) || n.mentions(o.dimension.as_str()))
                .collect();
            let mut pros = vec![format!(
                "{} consensus {:.2} ({})",
                o.dimension.as_str(),
                o.score_100 / 100.0,
                o.recommendation
            )];
            for n in &backing {
                pros.push(format!("backed by hands-on note '{}' ({})", n.title, n.id));
            }
            let mut cons = Vec::new();
            if let Some(low) = &lowest {
                if *low != o.recommendation {
                    cons.push(format!("rival consensus converges on '{low}' — check fit"));
                }
            }
            if o.score_100 < 60.0 {
                cons.push("low consensus — needs human review before lock".to_string());
            }
            if backing.is_empty() {
                cons.push(
                    "no hands-on case attached — first adoption carries ops risk".to_string(),
                );
            }
            let confidence = if backing.is_empty() {
                o.score_100 / 100.0
            } else {
                ((o.score_100 / 100.0) + 0.05).min(1.0)
            };
            let mut sources = vec![format!(
                "consensus:{}:{}",
                o.dimension.as_str(),
                o.recommendation
            )];
            for n in &backing {
                sources.push(format!("note:{}:{}", n.id, n.title));
            }
            ApplicationBranch {
                id: tag,
                summary: format!(
                    "{} via {} ({:.2})",
                    o.recommendation,
                    o.dimension.as_str(),
                    o.score_100 / 100.0
                ),
                pros,
                cons,
                confidence,
                cost_estimate: cost_estimate_for(o.score_100).to_string(),
                fits_stack: format!("{} — {} branch", hint, o.dimension.as_str()),
                sources,
            }
        })
        .collect()
}

fn order_of(dimension: ConsensusDimension) -> usize {
    ConsensusDimension::ALL
        .iter()
        .position(|d| *d == dimension)
        .unwrap_or(usize::MAX)
}

/// Render branches as the `proposal:` lines of the canonical RFC 10 §7
/// report, keeping the YAML shape backward compatible with pre-3.4 runs.
pub fn branch_proposal_lines(branches: &[ApplicationBranch]) -> Vec<String> {
    branches
        .iter()
        .map(|b| {
            format!(
                "{}: {} [conf {:.2}, cost: {}]",
                b.id, b.summary, b.confidence, b.cost_estimate
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(dimension: ConsensusDimension, rec: &str, score: f64) -> DimensionOutcome {
        DimensionOutcome {
            dimension,
            recommendation: rec.into(),
            score_100: score,
            weight: 1.0,
        }
    }

    fn note(id: &str, title: &str, decision: &str) -> ResearchNote {
        ResearchNote {
            id: id.into(),
            title: title.into(),
            project: Some("fintech X".into()),
            decision: decision.into(),
            outcome: Some("exitoso pero costoso en ops".into()),
            confidence: 0.81,
            tags: vec!["architecture".into(), "event-sourcing".into()],
            attached_at: "2026-07-04".into(),
            signature: "operator".into(),
        }
    }

    #[test]
    fn note_roundtrips_through_json_and_yaml() {
        let n = note(
            "rn-2026-07-04-001",
            "Lo hice así en producción",
            "Event Sourcing + Kafka",
        );
        n.validate().unwrap();
        let json = serde_json::to_string(&n).unwrap();
        let back: ResearchNote = serde_json::from_str(&json).unwrap();
        assert_eq!(n, back);
        let yaml = serde_yaml::to_string(&n).unwrap();
        assert!(yaml.contains("Lo hice"));
        let back_yaml: ResearchNote = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(n, back_yaml);
    }

    #[test]
    fn note_validation_rejects_empty_fields_and_bad_confidence() {
        let mut n = note("rn-1", "t", "d");
        n.title = "   ".into();
        assert_eq!(n.validate().unwrap_err(), HandsOnError::EmptyTitle);
        let mut n = note("rn-1", "t", "d");
        n.decision = String::new();
        assert_eq!(n.validate().unwrap_err(), HandsOnError::EmptyDecision);
        let mut n = note("rn-1", "t", "d");
        n.confidence = 1.5;
        assert_eq!(
            n.validate().unwrap_err(),
            HandsOnError::ConfidenceOutOfRange { value: 1.5 }
        );
        let mut n = note("rn-1", "t", "d");
        n.signature = String::new();
        assert_eq!(n.validate().unwrap_err(), HandsOnError::EmptySignature);
    }

    #[test]
    fn branches_rank_top_three_deterministically() {
        let outcomes = vec![
            outcome(ConsensusDimension::Community, "strategy-pattern", 71.0),
            outcome(ConsensusDimension::Enterprise, "event-sourcing", 66.0),
            outcome(ConsensusDimension::Academic, "crdt", 55.0),
            outcome(ConsensusDimension::Official, "event-sourcing-docs", 78.0),
        ];
        let once = build_branches(&outcomes, &[], None);
        let twice = build_branches(&outcomes, &[], None);
        assert_eq!(once, twice);
        assert_eq!(once.len(), 3);
        assert_eq!(once[0].id, "A");
        assert_eq!(once[1].id, "B");
        assert_eq!(once[2].id, "C");
        assert!(once[0].summary.contains("event-sourcing-docs"));
        for b in &once {
            b.validate().unwrap();
        }
    }

    #[test]
    fn branches_cite_backing_notes_and_lift_confidence() {
        let outcomes = vec![
            outcome(ConsensusDimension::Enterprise, "event-sourcing", 66.0),
            outcome(ConsensusDimension::Official, "event-sourcing", 78.0),
            outcome(ConsensusDimension::Community, "strategy-pattern", 71.0),
            outcome(ConsensusDimension::Academic, "crdt", 55.0),
        ];
        let notes = vec![note(
            "rn-1",
            "Lo hice así en producción",
            "Event Sourcing + Kafka",
        )];
        let branches = build_branches(&outcomes, &notes, Some("atlas-os"));
        let backed: Vec<&ApplicationBranch> = branches
            .iter()
            .filter(|b| b.sources.iter().any(|s| s.contains("rn-1")))
            .collect();
        assert!(
            !backed.is_empty(),
            "event-sourcing branches must cite the note"
        );
        for b in backed {
            assert!(b.pros.iter().any(|p| p.contains("hands-on note")));
            assert!(b.fits_stack.contains("atlas-os"));
        }
        let plain = build_branches(&outcomes, &[], None);
        let backed_conf: f64 = branches
            .iter()
            .find(|b| b.sources.iter().any(|s| s.contains("rn-1")))
            .map(|b| b.confidence)
            .unwrap();
        let plain_conf: f64 = plain
            .iter()
            .find(|b| {
                b.summary
                    == branches
                        .iter()
                        .find(|x| x.sources.iter().any(|s| s.contains("rn-1")))
                        .unwrap()
                        .summary
            })
            .map(|b| b.confidence)
            .unwrap_or(0.0);
        assert!(backed_conf > plain_conf);
    }

    #[test]
    fn branches_empty_on_no_data_instead_of_inventing() {
        let outcomes = vec![
            DimensionOutcome::no_data(ConsensusDimension::Community),
            DimensionOutcome::no_data(ConsensusDimension::Enterprise),
            DimensionOutcome::no_data(ConsensusDimension::Academic),
            DimensionOutcome::no_data(ConsensusDimension::Official),
        ];
        assert!(build_branches(&outcomes, &[], None).is_empty());
        assert!(build_branches(&[], &[], None).is_empty());
    }

    #[test]
    fn proposal_lines_keep_report_shape() {
        let outcomes = vec![
            outcome(ConsensusDimension::Official, "event-sourcing", 78.0),
            outcome(ConsensusDimension::Community, "strategy-pattern", 71.0),
            outcome(ConsensusDimension::Enterprise, "event-sourcing", 66.0),
            outcome(ConsensusDimension::Academic, "crdt", 55.0),
        ];
        let branches = build_branches(&outcomes, &[], None);
        let lines = branch_proposal_lines(&branches);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("A:"));
        assert!(lines[0].contains("cost:"));
    }

    #[test]
    fn journal_ref_derives_from_run_id() {
        assert_eq!(
            journal_ref_for_run("rr-2026-07-04-001"),
            "jr-2026-07-04-001"
        );
        assert_eq!(journal_ref_for_run("rr-x"), "jr-x");
        assert_eq!(journal_ref_for_run("custom"), "jr-custom");
    }

    #[test]
    fn parse_tags_splits_and_trims() {
        assert_eq!(
            parse_tags("architecture, event-sourcing, k"),
            vec!["architecture", "event-sourcing", "k"]
        );
        assert!(parse_tags("  , ,").is_empty());
    }
}
