// OpenCode OS — Step 3: Detect Ambiguities (RFC 23 §1, §2.3).
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
}
