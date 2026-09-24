// Atlas OS — Step 3: Detect Ambiguities (RFC 23 §1, §2.3).
//
// Heuristic detectors for the 10 anti-patterns `C1..C10`. Each detector
// is keyword/length/regex-driven; together they emit the `gaps[]` array of
// the `PublicUnderstandingVerdict`. The downstream Step 5 (Clarification)
// consumes the gap list to generate K=3 questions, and Step 7 (Consolidate)
// uses it to compute the confidence rubric (`feasibility_clarity`, etc.).

use crate::prompt::steps::parse::ParsedIntent;
use crate::prompt::types::{Gap, GapType, Severity};

pub fn run(parsed: &ParsedIntent, raw: &str) -> Vec<Gap> {
    let mut gaps: Vec<Gap> = Vec::new();
    let lower = raw.to_lowercase();
    let token_count = raw.split_whitespace().count();
    let char_count = raw.chars().count();

    // C7 — low_information_prompt — interjections / non-actionable single
    // verbs → Blocker. These are conversational continuations ("go", "ok",
    // "yes", "next") or empty prompts — none carries an actionable verb
    // the pipeline can build intent from.
    if matches!(
        lower.as_str(),
        "" | "go" | "ok" | "okay" | "yes" | "no" | "y" | "n" | "continue" | "next"
    ) {
        gaps.push(Gap {
            kind: GapType::LowInformationPrompt,
            evidence: raw.trim().to_string(),
            severity: Severity::Blocker,
            auto_resolvable: false,
        });
    } else if token_count <= 3 {
        gaps.push(Gap {
            kind: GapType::LowInformationPrompt,
            evidence: raw.to_string(),
            severity: Severity::High,
            auto_resolvable: false,
        });
    }

    // C3 — underspecified — generic verbs without object: "make it better",
    // "fix it", "improve", "fix the bug" (no concrete thing referenced).
    let underspecified_triggers = [
        "make it better",
        "improve",
        "fix it",
        "fix the bug",
        "fix this",
        "do it",
        "make it work",
        "make it faster",
        "make it cleaner",
    ];
    if underspecified_triggers.iter().any(|t| lower.contains(t)) && parsed.keys.is_empty() {
        gaps.push(Gap {
            kind: GapType::Underspecified,
            evidence: raw.to_string(),
            severity: Severity::High,
            auto_resolvable: false,
        });
    }

    // C1 — capacity_hallucination — science-fiction phrasing.
    const SCI_FI: &[&str] = &[
        "fly through the galaxy",
        "travel through space",
        "build a spaceship",
        "from scratch build a continent",
        "infinite amount",
        "magic",
        "infinite capacity",
        "omniscient",
        "through the galaxy",
        "fly through space",
        "into the galaxy",
    ];
    for needle in SCI_FI {
        if lower.contains(needle) {
            gaps.push(Gap {
                kind: GapType::CapacityHallucination,
                evidence: needle.to_string(),
                severity: Severity::Blocker,
                auto_resolvable: false,
            });
            break;
        }
    }

    // C2 — unknown_tool_dependency — modifiers around "tool nobody knows":
    // "X that almost nobody knows", "obscure X", "edge-case X".
    const UNKNOWN_TOOL: &[&str] = &[
        "almost nobody knows",
        "almost noone knows",
        "almost no one knows",
        "obscure",
        "very rare",
        "extremely rare",
        "edge-case tool",
        "edge case tool",
    ];
    for needle in UNKNOWN_TOOL {
        if lower.contains(needle) {
            gaps.push(Gap {
                kind: GapType::UnknownToolDependency,
                evidence: needle.to_string(),
                severity: Severity::High,
                auto_resolvable: false,
            });
            break;
        }
    }

    // C4 — implicit_assumption — "you should", "obviously", "of course",
    // "as usual". The user presupposes a context they did not state.
    const IMPLICIT: &[&str] = &[
        "obviously",
        "of course",
        "as usual",
        "as always",
        "you should",
    ];
    for needle in IMPLICIT {
        if lower.contains(needle) {
            gaps.push(Gap {
                kind: GapType::ImplicitAssumption,
                evidence: needle.to_string(),
                severity: Severity::Med,
                auto_resolvable: true, // step 6 can resolve via Architecture Memory
            });
            break;
        }
    }

    // C5 — capacity_overreach — "build something that escapes my hands",
    // "do everything", "automate all the things".
    const OVERREACH: &[&str] = &[
        "out of my hands",
        "escapes my hands",
        "bigger than me",
        "automate everything",
        "automate all the things",
        "i have no idea how",
        "i don't know how",
    ];
    for needle in OVERREACH {
        if lower.contains(needle) {
            gaps.push(Gap {
                kind: GapType::CapacityOverreach,
                evidence: needle.to_string(),
                severity: Severity::High,
                auto_resolvable: false,
            });
            break;
        }
    }

    // C6 — user_knowledge_gap — "how do i", "i'm not sure how", "tutorial".
    const KNOWLEDGE_GAP: &[&str] = &[
        "how do i even",
        "i have no way",
        "i don't have the way",
        "i do not have the way",
        "i'm new to this",
        "i am new to this",
        "i'm a beginner",
        "i am a beginner",
    ];
    for needle in KNOWLEDGE_GAP {
        if lower.contains(needle) {
            gaps.push(Gap {
                kind: GapType::UserKnowledgeGap,
                evidence: needle.to_string(),
                severity: Severity::Med,
                auto_resolvable: false,
            });
            break;
        }
    }

    // C8 — lost_in_the_middle — long prompts with the operative clause
    // in the middle. Heuristic: > 400 chars and no clear verb in the first
    // 50 / last 50 chars. RFC 23 §1 cites Liu et al. 2023.
    if char_count > 400 {
        let head: String = raw.chars().take(50).collect();
        let tail: String = raw
            .chars()
            .rev()
            .take(50)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        let operative_verbs = [
            "fix", "create", "add", "test", "deploy", "update", "remove", "refactor",
        ];
        let head_has = operative_verbs
            .iter()
            .any(|v| head.to_lowercase().contains(v));
        let tail_has = operative_verbs
            .iter()
            .any(|v| tail.to_lowercase().contains(v));
        if !head_has && !tail_has {
            gaps.push(Gap {
                kind: GapType::LostInTheMiddle,
                evidence: "...".to_string(),
                severity: Severity::Med,
                auto_resolvable: true,
            });
        }
    }

    // C9 — role_overload — "act as", "pretend you" + N roles listed.
    let role_count = lower.matches("act as").count()
        + lower.matches("pretend you").count()
        + lower.matches("as an ").count();
    if role_count >= 2 {
        let snippet = if lower.len() > 80 {
            lower.chars().take(80).collect()
        } else {
            lower.clone()
        };
        gaps.push(Gap {
            kind: GapType::RoleOverload,
            evidence: snippet,
            severity: Severity::Med,
            auto_resolvable: false,
        });
    }

    // C10 — model_omniscience_assumption — handled by step 2 implicit
    // signals; here we only flag it as a gap if the explicit phrase is
    // present.
    if lower.contains("you already know") || lower.contains("as an ai") {
        gaps.push(Gap {
            kind: GapType::ModelOmniscienceAssumption,
            evidence: "you already know / as an ai".to_string(),
            severity: Severity::Low,
            auto_resolvable: true,
        });
    }

    gaps.sort_by_key(|g| g.kind as i32);
    gaps
}

/// RFC 29 §3.C — resolve `user_knowledge_gap` (C6) against the stored
/// `user_profile.knowledge_state.gaps_identified`. When the prompt
/// mentions a registered gap, push a `UserKnowledgeGap` (Med) so the
/// downstream steps offer mentor mode + shortcuts automatically.
/// Returns `true` when a gap was added. No-op when C6 is already
/// present or when no registered gap matches.
pub fn apply_user_profile(gaps: &mut Vec<Gap>, gaps_identified: &[String], raw: &str) -> bool {
    if gaps.iter().any(|g| g.kind == GapType::UserKnowledgeGap) {
        return false;
    }
    let lower = raw.to_ascii_lowercase();
    let hit = gaps_identified.iter().find(|gap| {
        let g = gap.to_ascii_lowercase();
        let trimmed = g.trim();
        if trimmed.is_empty() {
            return false;
        }
        if lower.contains(trimmed) {
            return true;
        }
        trimmed.len() > 3
            && trimmed
                .split_whitespace()
                .any(|w| w.len() > 3 && lower.contains(w))
    });
    if let Some(evidence) = hit {
        gaps.push(Gap {
            kind: GapType::UserKnowledgeGap,
            evidence: evidence.clone(),
            severity: Severity::Med,
            auto_resolvable: false,
        });
        gaps.sort_by_key(|g| g.kind as i32);
        return true;
    }
    false
}

/// RFC 32 Phase 5 sub-fase 5.4 — one consultable learned rule projected
/// as a prompt-time hint. Built by the caller from
/// `Journal::list_consultable_rules` + `learned_rule_when_then`
/// (`candidate` / `active` only — deprecated rules never reach here).
/// `trigger` is the `RuleWhen.pattern` tag; `hint` the `RuleThen.diff_hint`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LearnedHint {
    pub rule_id: String,
    pub trigger: String,
    pub hint: String,
}

/// Match consultable hints against the raw prompt (same
/// case-insensitive substring rule as `apply_user_profile`) and render
/// each hit as a `[rule <id>] <hint>` observation line. Returns the
/// rendered lines in input order. Empty triggers never match.
pub fn match_learned_hints(hints: &[LearnedHint], raw: &str) -> Vec<String> {
    let lower = raw.to_ascii_lowercase();
    let mut out = Vec::new();
    for h in hints {
        let trigger = h.trigger.trim().to_ascii_lowercase();
        if trigger.is_empty() || !lower.contains(&trigger) {
            continue;
        }
        let line = format!("[rule {}] {}", h.rule_id, h.hint);
        if !out.contains(&line) {
            out.push(line);
        }
    }
    out
}

/// Append `match_learned_hints` output to `observations`, skipping lines
/// already present. Returns the number of lines added. Pure: the
/// pipeline verdict gains context hints without touching gaps,
/// confidence or clarification flow.
pub fn apply_learned_hints(
    observations: &mut Vec<String>,
    hints: &[LearnedHint],
    raw: &str,
) -> usize {
    let mut added = 0;
    for line in match_learned_hints(hints, raw) {
        if !observations.contains(&line) {
            observations.push(line);
            added += 1;
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::steps::parse;

    fn parse_prompt(raw: &str) -> (ParsedIntent, Vec<Gap>) {
        let p = parse::run(&crate::prompt::steps::capture::normalise(raw));
        let g = run(&p, raw);
        (p, g)
    }

    fn has_gap(gaps: &[Gap], t: GapType) -> bool {
        gaps.iter().any(|g| g.kind == t)
    }

    #[test]
    fn empty_or_single_word_prompt_is_low_information_blocker() {
        let (_, g) = parse_prompt("go");
        assert!(has_gap(&g, GapType::LowInformationPrompt));
        let blocker = g
            .iter()
            .find(|x| x.kind == GapType::LowInformationPrompt)
            .unwrap();
        assert_eq!(blocker.severity, Severity::Blocker);
    }

    #[test]
    fn very_short_prompts_get_high_severity_low_information() {
        let (_, g) = parse_prompt("fix the bug");
        assert_eq!(g[0].severity, Severity::High);
    }

    #[test]
    fn make_it_better_is_underspecified_with_no_keys() {
        let (_, g) = parse_prompt("make it better please");
        assert!(has_gap(&g, GapType::Underspecified));
    }

    #[test]
    fn galaxy_phrasing_is_capacity_hallucination_blocker() {
        let (_, g) = parse_prompt("I want to fly through the galaxy and return tomorrow");
        let cg = g.iter().find(|x| x.kind == GapType::CapacityHallucination);
        assert!(cg.is_some());
        assert_eq!(cg.unwrap().severity, Severity::Blocker);
    }

    #[test]
    fn obscure_tool_is_unknown_tool_dependency() {
        let (_, g) = parse_prompt("use a 3d printer that almost nobody knows");
        assert!(has_gap(&g, GapType::UnknownToolDependency));
    }

    #[test]
    fn out_of_my_hands_is_capacity_overreach() {
        let (_, g) = parse_prompt("build a system that escapes my hands");
        assert!(has_gap(&g, GapType::CapacityOverreach));
    }

    #[test]
    fn role_overload_fires_when_two_role_phrases_present() {
        let (_, g) = parse_prompt("act as a lawyer and act as a chef and write the contract");
        assert!(has_gap(&g, GapType::RoleOverload));
    }

    #[test]
    fn profile_gap_match_adds_c6() {
        let (p, mut g) = parse_prompt("expose the service via kubernetes ingress");
        assert!(!has_gap(&g, GapType::UserKnowledgeGap));
        let added = apply_user_profile(
            &mut g,
            &["kubernetes ingress".to_string()],
            "expose the service via kubernetes ingress",
        );
        assert!(added);
        assert!(has_gap(&g, GapType::UserKnowledgeGap));
        let _ = p;
    }

    #[test]
    fn profile_gap_no_match_adds_nothing() {
        let (p, mut g) = parse_prompt("refactor the billing module");
        let added = apply_user_profile(
            &mut g,
            &["kubernetes ingress".to_string()],
            "refactor the billing module",
        );
        assert!(!added);
        assert!(!has_gap(&g, GapType::UserKnowledgeGap));
        let _ = p;
    }

    #[test]
    fn profile_hook_is_noop_when_c6_already_present() {
        let p = parse::run(&crate::prompt::steps::capture::normalise(
            "i'm new to this, expose via kubernetes ingress",
        ));
        let mut g = run(&p, "i'm new to this, expose via kubernetes ingress");
        assert!(has_gap(&g, GapType::UserKnowledgeGap));
        let n = g.len();
        assert!(!apply_user_profile(
            &mut g,
            &["kubernetes ingress".to_string()],
            "i'm new to this, expose via kubernetes ingress"
        ));
        assert_eq!(g.len(), n);
    }

    fn learned_hint(trigger: &str, hint: &str) -> LearnedHint {
        LearnedHint {
            rule_id: "r-2026-07-04-001".into(),
            trigger: trigger.into(),
            hint: hint.into(),
        }
    }

    #[test]
    fn learned_rule_match_injects_hint_observation() {
        let hints = vec![learned_hint("no_println", "remove println statements")];
        let mut observations = vec!["domain=backend".to_string()];
        let added = apply_learned_hints(
            &mut observations,
            &hints,
            "fix the no_println lint in the billing module",
        );
        assert_eq!(added, 1);
        assert!(observations
            .iter()
            .any(|o| o == "[rule r-2026-07-04-001] remove println statements"));
    }

    #[test]
    fn learned_rule_no_match_injects_nothing() {
        let hints = vec![learned_hint("no_println", "remove println statements")];
        let mut observations = vec!["domain=backend".to_string()];
        let added = apply_learned_hints(
            &mut observations,
            &hints,
            "refactor the billing module for clarity",
        );
        assert_eq!(added, 0);
        assert_eq!(observations.len(), 1);
    }

    #[test]
    fn learned_hint_with_empty_trigger_never_matches() {
        let hints = vec![learned_hint("   ", "remove println statements")];
        let mut observations = Vec::new();
        assert_eq!(
            apply_learned_hints(&mut observations, &hints, "anything at all"),
            0
        );
        assert!(observations.is_empty());
    }

    #[test]
    fn learned_hint_match_is_case_insensitive_and_deduped() {
        let hints = vec![
            learned_hint("NO_Println", "remove println statements"),
            learned_hint("no_println", "remove println statements"),
        ];
        let mut observations =
            vec!["[rule r-2026-07-04-001] remove println statements".to_string()];
        let added = apply_learned_hints(
            &mut observations,
            &hints,
            "fix NO_PRINTLN in the billing module",
        );
        assert_eq!(added, 0);
        assert_eq!(observations.len(), 1);
    }
}
