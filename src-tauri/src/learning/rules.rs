// Atlas OS — Learning Engine YAML rules (RFC 16 §4, RFC 32 Phase 5 sub-fase 5.0).
//
// Writer + loader for `.opencode/rules/<rule_id>.yaml`. The YAML shape mirrors
// the RFC 16 §3 example: a top-level `rule:` mapping carrying the `Pattern`'s
// `RuleWhen` / `RuleThen` verbatim (same serde field names, so YAML ↔ Journal
// round-trips without a translation layer). No types are duplicated here — the
// envelope borrows `RuleWhen`, `RuleThen` and `RuleLifecycle` from `types.rs`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::types::{Pattern, RuleLifecycle, RuleThen, RuleWhen};

/// Envelope matching the RFC 16 §3 YAML example (`rule:` top-level key).
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RuleFile {
    rule: RuleDoc,
}

/// Document body. `when` / `then` reuse the canonical structs; the scalar
/// columns mirror `Pattern` 1:1 so `to_pattern` is a field move.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct RuleDoc {
    id: String,
    when: RuleWhen,
    then: RuleThen,
    confidence: f32,
    #[serde(default)]
    lifecycle: LifecycleTag,
    #[serde(default)]
    priority: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    evidence: Vec<Uuid>,
    #[serde(default)]
    model_id: String,
    #[serde(default)]
    generated_at: String,
    #[serde(default = "Uuid::new_v4")]
    pattern_id: Uuid,
}

/// `RuleLifecycle` wrapper that defaults to `Draft` for hand-written YAML
/// files that omit the key (operator-authored rules start as drafts).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LifecycleTag {
    #[default]
    Draft,
    Candidate,
    Active,
    Deprecated,
}

impl From<RuleLifecycle> for LifecycleTag {
    fn from(l: RuleLifecycle) -> Self {
        match l {
            RuleLifecycle::Draft => Self::Draft,
            RuleLifecycle::Candidate => Self::Candidate,
            RuleLifecycle::Active => Self::Active,
            RuleLifecycle::Deprecated => Self::Deprecated,
        }
    }
}

impl From<LifecycleTag> for RuleLifecycle {
    fn from(t: LifecycleTag) -> Self {
        match t {
            LifecycleTag::Draft => Self::Draft,
            LifecycleTag::Candidate => Self::Candidate,
            LifecycleTag::Active => Self::Active,
            LifecycleTag::Deprecated => Self::Deprecated,
        }
    }
}

impl RuleDoc {
    fn from_pattern(p: &Pattern) -> Self {
        Self {
            id: p.rule_id.clone(),
            when: p.when.clone(),
            then: p.then.clone(),
            confidence: p.confidence,
            lifecycle: LifecycleTag::from(p.lifecycle),
            priority: p.priority,
            evidence: p.evidence.clone(),
            model_id: p.model_id.clone(),
            generated_at: p.generated_at.clone(),
            pattern_id: p.pattern_id,
        }
    }

    fn to_pattern(&self) -> anyhow::Result<Pattern> {
        if !(0.0..=1.0).contains(&self.confidence) {
            anyhow::bail!(
                "rule `{}` has out-of-range confidence {}",
                self.id,
                self.confidence
            );
        }
        if self.id.trim().is_empty() {
            anyhow::bail!("rule id must not be empty");
        }
        Ok(Pattern {
            pattern_id: self.pattern_id,
            rule_id: self.id.clone(),
            when: self.when.clone(),
            then: self.then.clone(),
            confidence: self.confidence,
            lifecycle: RuleLifecycle::from(self.lifecycle),
            priority: self.priority,
            evidence: self.evidence.clone(),
            generated_at: self.generated_at.clone(),
            model_id: self.model_id.clone(),
            elapsed_ms: 0,
        })
    }
}

/// Serialise a `Pattern` to its RFC 16 §4 YAML document.
pub fn pattern_to_yaml(pattern: &Pattern) -> anyhow::Result<String> {
    let file = RuleFile {
        rule: RuleDoc::from_pattern(pattern),
    };
    Ok(serde_yaml::to_string(&file)?)
}

/// Parse a YAML document back into a `Pattern`. Rejects empty ids and
/// out-of-range confidence; unknown `stage` / `strategy` / `lifecycle`
/// tags surface as serde errors naming the offending value.
pub fn pattern_from_yaml(yaml: &str) -> anyhow::Result<Pattern> {
    let file: RuleFile = serde_yaml::from_str(yaml)?;
    file.rule.to_pattern()
}

/// Write `.opencode/rules/<rule_id>.yaml` under `rules_dir` (created when
/// missing). Returns the written path. File name sanitises `/` and `\`
/// so a hostile `rule_id` cannot escape the directory.
pub fn write_rule_file(rules_dir: &Path, pattern: &Pattern) -> anyhow::Result<PathBuf> {
    std::fs::create_dir_all(rules_dir)?;
    let safe_id: String = pattern
        .rule_id
        .chars()
        .map(|c| if c == '/' || c == '\\' { '_' } else { c })
        .collect();
    let path = rules_dir.join(format!("{safe_id}.yaml"));
    std::fs::write(&path, pattern_to_yaml(pattern)?)?;
    Ok(path)
}

/// Read a rule file written by `write_rule_file` back into a `Pattern`.
pub fn load_rule_file(path: &Path) -> anyhow::Result<Pattern> {
    let raw = std::fs::read_to_string(path)?;
    pattern_from_yaml(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repair::types::RepairStrategy;
    use crate::validation::types::StageKind;

    fn fixture_pattern() -> Pattern {
        Pattern {
            pattern_id: Uuid::new_v4(),
            rule_id: "r-2026-07-04-001".into(),
            when: RuleWhen {
                stage: StageKind::LintFormat,
                pattern: "no_println".into(),
                lang: "rust".into(),
            },
            then: RuleThen {
                strategy: RepairStrategy::AutoFix,
                skill: String::new(),
                diff_hint: "src/lib.rs: let _ = 42;".into(),
            },
            confidence: 0.7,
            lifecycle: RuleLifecycle::Draft,
            priority: RuleLifecycle::Draft.default_priority(),
            evidence: vec![Uuid::new_v4()],
            generated_at: "2026-07-04T12:00:00Z".into(),
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        }
    }

    #[test]
    fn yaml_roundtrip_preserves_pattern() {
        let original = fixture_pattern();
        let yaml = pattern_to_yaml(&original).unwrap();
        assert!(
            yaml.contains("rule:"),
            "RFC 16 §4 envelope must use `rule:` top key"
        );
        assert!(yaml.contains("r-2026-07-04-001"));
        assert!(yaml.contains("lint_format"));
        assert!(yaml.contains("auto_fix"));
        let back = pattern_from_yaml(&yaml).unwrap();
        assert_eq!(back.rule_id, original.rule_id);
        assert_eq!(back.when, original.when);
        assert_eq!(back.then, original.then);
        assert!((back.confidence - original.confidence).abs() < 1e-6);
        assert_eq!(back.lifecycle, original.lifecycle);
        assert_eq!(back.priority, original.priority);
        assert_eq!(back.evidence, original.evidence);
        assert_eq!(back.pattern_id, original.pattern_id);
        assert_eq!(back.model_id, original.model_id);
        assert_eq!(back.generated_at, original.generated_at);
    }

    #[test]
    fn file_write_and_load_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let rules_dir = dir.path().join(".opencode").join("rules");
        let original = fixture_pattern();
        let path = write_rule_file(&rules_dir, &original).unwrap();
        assert_eq!(path, rules_dir.join("r-2026-07-04-001.yaml"));
        let back = load_rule_file(&path).unwrap();
        assert_eq!(back.rule_id, original.rule_id);
        assert_eq!(back.when, original.when);
        assert_eq!(back.then, original.then);
    }

    #[test]
    fn rejects_out_of_range_confidence() {
        let mut p = fixture_pattern();
        p.confidence = 2.0;
        let yaml = pattern_to_yaml(&p).unwrap();
        assert!(pattern_from_yaml(&yaml).is_err());
    }

    #[test]
    fn rejects_unknown_stage_tag() {
        let yaml = "rule:\n  id: r-x\n  when:\n    stage: not_a_stage\n    pattern: p\n    lang: rust\n  then:\n    strategy: auto_fix\n  confidence: 0.5\n";
        assert!(pattern_from_yaml(yaml).is_err());
    }

    #[test]
    fn rejects_unknown_strategy_tag() {
        let yaml = "rule:\n  id: r-x\n  when:\n    stage: lint_format\n  then:\n    strategy: not_a_strategy\n  confidence: 0.5\n";
        assert!(pattern_from_yaml(yaml).is_err());
    }

    #[test]
    fn hand_written_yaml_without_lifecycle_defaults_to_draft() {
        let yaml = "rule:\n  id: r-hand-001\n  when:\n    stage: type_check\n    pattern: missing_fn\n    lang: rust\n  then:\n    strategy: coding_amendment\n    diff_hint: 'add fn'\n  confidence: 0.5\n";
        let back = pattern_from_yaml(yaml).unwrap();
        assert_eq!(back.lifecycle, RuleLifecycle::Draft);
        assert_eq!(back.when.stage, StageKind::TypeCheck);
        assert_eq!(back.then.strategy, RepairStrategy::CodingAmendment);
    }
}
