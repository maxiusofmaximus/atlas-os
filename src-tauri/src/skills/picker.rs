use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::skills::{Engine, SkillManifest};

pub const DEFAULT_PICK_THRESHOLD: f64 = 0.5;
pub const PRIORITY_WEIGHT: f64 = 0.4;
pub const MATCH_WEIGHT: f64 = 0.4;
pub const LIFECYCLE_WEIGHT: f64 = 0.2;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoredSkill {
    pub manifest: SkillManifest,
    pub relevance: f64,
    pub suggested: bool,
}

pub fn tokenize(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn skill_signal_tokens(skill: &SkillManifest) -> HashSet<String> {
    let mut out = HashSet::new();
    for token in skill
        .id
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
    {
        out.insert(token.to_string());
    }
    for token in tokenize(&skill.description) {
        out.insert(token);
    }
    if let Some(summary) = &skill.summary {
        for token in tokenize(summary) {
            out.insert(token);
        }
    }
    for tag in [
        Some(skill.engine.tag()),
        skill.domain.as_deref(),
        skill.language.as_deref(),
        skill.framework.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        for token in tokenize(tag) {
            out.insert(token);
        }
    }
    for cap in &skill.capabilities {
        for token in tokenize(cap) {
            out.insert(token);
        }
    }
    out
}

pub fn priority_score(skill: &SkillManifest) -> f64 {
    (skill.priority.min(100) as f64) / 100.0
}

pub fn match_score(prompt: &str, skill: &SkillManifest) -> f64 {
    let prompt_tokens = tokenize(prompt);
    if prompt_tokens.is_empty() {
        return 0.0;
    }
    let signals = skill_signal_tokens(skill);
    if signals.is_empty() {
        return 0.0;
    }
    let hits = prompt_tokens.intersection(&signals).count() as f64;
    (hits / prompt_tokens.len() as f64).clamp(0.0, 1.0)
}

pub fn lifecycle_score(skill: &SkillManifest) -> f64 {
    if skill.verified {
        1.0
    } else {
        0.0
    }
}

pub fn relevance(prompt: &str, skill: &SkillManifest) -> f64 {
    let raw = PRIORITY_WEIGHT * priority_score(skill)
        + MATCH_WEIGHT * match_score(prompt, skill)
        + LIFECYCLE_WEIGHT * lifecycle_score(skill);
    raw.clamp(0.0, 1.0)
}

pub fn pick_skills(prompt: &str, skills: &[SkillManifest]) -> Vec<ScoredSkill> {
    pick_skills_with_threshold(prompt, skills, DEFAULT_PICK_THRESHOLD)
}

pub fn pick_skills_with_threshold(
    prompt: &str,
    skills: &[SkillManifest],
    threshold: f64,
) -> Vec<ScoredSkill> {
    let mut out: Vec<ScoredSkill> = skills
        .iter()
        .map(|m| {
            let rel = relevance(prompt, m);
            ScoredSkill {
                manifest: m.clone(),
                relevance: rel,
                suggested: rel >= threshold,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.relevance
            .partial_cmp(&a.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.manifest.priority.cmp(&a.manifest.priority))
            .then(a.manifest.id.cmp(&b.manifest.id))
    });
    out
}

pub fn engine_hint(prompt: &str) -> Option<Engine> {
    let tokens = tokenize(prompt);
    if tokens.contains("plan") || tokens.contains("planning") || tokens.contains("mission") {
        Some(Engine::Planning)
    } else if tokens.contains("research") || tokens.contains("arxiv") || tokens.contains("search") {
        Some(Engine::Research)
    } else if tokens.contains("test") || tokens.contains("validate") || tokens.contains("audit") {
        Some(Engine::Validation)
    } else if tokens.contains("security") || tokens.contains("owasp") || tokens.contains("gdpr") {
        Some(Engine::Security)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(id: &str, priority: u32, description: &str) -> SkillManifest {
        SkillManifest {
            id: id.into(),
            version: "1.0.0".into(),
            description: description.into(),
            engine: Engine::Coding,
            priority,
            verified: true,
            ..Default::default()
        }
    }

    #[test]
    fn relevance_is_deterministic_and_bounded() {
        let s = skill(
            "react-ui-expert",
            90,
            "Generacion de componentes React accesibles",
        );
        let a = relevance("componentes react accesibles", &s);
        let b = relevance("componentes react accesibles", &s);
        assert!((a - b).abs() < 1e-12);
        assert!((0.0..=1.0).contains(&a));
    }

    #[test]
    fn keyword_match_drives_suggested_split() {
        let react = skill(
            "react-ui-expert",
            90,
            "Generacion de componentes React accesibles",
        );
        let vue = SkillManifest {
            id: "vue-ui-expert".into(),
            version: "1.0.0".into(),
            description: "Componentes Vue con composition api".into(),
            engine: Engine::Coding,
            priority: 90,
            framework: Some("vue".into()),
            verified: true,
            ..Default::default()
        };
        let out = pick_skills("necesito componentes react accesibles", &[react, vue]);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].manifest.id, "react-ui-expert");
        assert!(out[0].suggested);
        assert!(out[0].relevance > out[1].relevance);
    }

    #[test]
    fn threshold_filters_suggested_vs_dimmed() {
        let high = skill("react-ui-expert", 90, "react componentes frontend");
        let low = SkillManifest {
            id: "vue-ui-expert".into(),
            version: "0.1.0".into(),
            description: "otra cosa distinta backend go".into(),
            engine: Engine::Unspecified,
            priority: 5,
            verified: false,
            ..Default::default()
        };
        let strict = pick_skills_with_threshold(
            "react componentes frontend",
            &[high.clone(), low.clone()],
            0.9,
        );
        let loose = pick_skills_with_threshold("react componentes frontend", &[high, low], 0.0);
        assert!(strict.iter().any(|s| !s.suggested));
        assert!(loose.iter().all(|s| s.suggested));
    }

    #[test]
    fn priority_weight_orders_ties() {
        let high = skill("a-skill", 90, "tarea comun");
        let low = skill("b-skill", 10, "tarea comun");
        let out = pick_skills("prompt sin solape xyz", &[low, high]);
        assert_eq!(out[0].manifest.id, "a-skill");
        assert!(out[0].relevance > out[1].relevance);
    }

    #[test]
    fn verified_lifecycle_bonus_applies() {
        let mut verified = skill("v-skill", 50, "tarea comun");
        verified.verified = true;
        let mut unverified = skill("u-skill", 50, "tarea comun");
        unverified.verified = false;
        let rv = relevance("prompt sin solape xyz", &verified);
        let ru = relevance("prompt sin solape xyz", &unverified);
        assert!((rv - ru - LIFECYCLE_WEIGHT).abs() < 1e-9);
    }

    #[test]
    fn empty_prompt_scores_only_priority_plus_lifecycle() {
        let s = skill("a-skill", 80, "cualquier descripcion");
        let rel = relevance("", &s);
        let expected = PRIORITY_WEIGHT * 0.8 + LIFECYCLE_WEIGHT * 1.0;
        assert!((rel - expected).abs() < 1e-9);
        let out = pick_skills("", &[s]);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn sort_is_deterministic_on_full_tie() {
        let mut a = skill("b-id", 50, "misma descripcion");
        a.verified = true;
        let mut b = skill("a-id", 50, "misma descripcion");
        b.verified = true;
        let out = pick_skills("misma descripcion", &[a, b]);
        assert_eq!(out[0].manifest.id, "a-id");
        assert_eq!(out[1].manifest.id, "b-id");
    }

    #[test]
    fn engine_tag_participates_in_match() {
        let mut coding = skill("fix", 50, "repara errores");
        coding.engine = Engine::Coding;
        assert!(skill_signal_tokens(&coding).contains("coding"));
        assert_eq!(match_score("coding", &coding), 1.0);
    }
}
