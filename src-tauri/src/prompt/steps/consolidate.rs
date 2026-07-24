// OpenCode OS — Step 7: Consolidate verdict + MissionConsolidated
// (RFC 23 §2 step 7 and §4).
//
// Heuristic impl of Self-Refine (Madaan et al. 2023). The critic loop runs
// `REFINE_ITERS = 2` iterations over the intent statement: each iteration
// trims weak tokens and appends the strongest surviving key. The rubric
// (intent_clarity / scope_clarity / feasibility_clarity / context_clarity)
// is computed from the parsed intent, the gap list and the clarification
// outcome; the mean is bucketed onto `ConfidenceLevel` per RFC 23 §5.
//
// `recommended_mode` follows RFC 23 §8:
//   trust `context` first (lost_in_the_middle),
//   then `ask` for Low/Block,
//   `architect` for Medium,
//   `code` for High.

use std::time::Instant;
use uuid::Uuid;

use crate::prompt::runner::PipelineOptions;
use crate::prompt::steps::parse::ParsedIntent;
use crate::prompt::types::{
    AcceptedAssumption, AssumptionSource, ClarificationQuestion, ConfidenceLevel, ConfidenceRubric,
    Gap, GapType, IntentHypothesis, LockSource, MissionConsolidated, PublicUnderstandingVerdict,
    RecommendedMode, Scope, SimilarMission,
};

const REFINE_ITERS: u32 = 2;

#[allow(clippy::too_many_arguments)]
pub fn build(
    parsed: ParsedIntent,
    gaps: Vec<Gap>,
    similar_missions: Vec<SimilarMission>,
    clarification_questions: Vec<ClarificationQuestion>,
    session_id: Uuid,
    raw_prompt: &str,
    started: Instant,
    options: &PipelineOptions,
) -> anyhow::Result<(PublicUnderstandingVerdict, Option<MissionConsolidated>)> {
    // Self-Refine on the intent statement.
    let refined_intent = refine_intent(parsed.intent.clone(), &parsed.keys, REFINE_ITERS);

    // Confidence rubric.
    let rubric = compute_rubric(&parsed, &gaps, &clarification_questions, &refined_intent);
    let confidence = ConfidenceLevel::from_rubric(rubric.mean());
    let recommended_mode = pick_recommended_mode(&gaps, confidence);

    let hypotheses: Vec<IntentHypothesis> = parsed
        .intent_hypotheses
        .iter()
        .map(|h| IntentHypothesis {
            rank: h.rank,
            text: h.text.clone(),
            feasibility_score: h.feasibility_score,
            rejection_reason: h.rejection_reason.clone(),
        })
        .collect();

    let elapsed_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);

    let verdict = PublicUnderstandingVerdict {
        verdict_id: Uuid::new_v4(),
        session_id,
        raw_prompt: raw_prompt.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),

        intent: refined_intent,
        intent_hypotheses: hypotheses,
        keys: parsed.keys.clone(),
        named_entities: parsed.named_entities.clone(),
        domain: parsed.domain,
        technology: parsed.technology.clone(),
        desired_action: parsed.desired_action,
        scope: parsed.scope,
        implicit_signals: parsed.implicit_signals.clone(),

        gaps: gaps.clone(),
        similar_missions: similar_missions.clone(),
        clarification_questions: clarification_questions.clone(),

        confidence,
        confidence_rubric: rubric,
        observations: make_observations(&gaps, &parsed),
        recommended_mode,

        model_id: "heuristic-v0".into(),
        judge_model_id: None,
        elapsed_ms,
    };

    // Step 9 preview: only lock when confidence is HIGH or `--force`.
    let consolidated_opt = if confidence == ConfidenceLevel::High || options.force {
        Some(build_consolidated(&verdict, &gaps, options))
    } else {
        None
    };

    Ok((verdict, consolidated_opt))
}

fn refine_intent(initial: String, keys: &[String], iters: u32) -> String {
    let mut current = initial;
    for _ in 0..iters {
        if is_weak_statement(&current) {
            if let Some(best_key) = keys.iter().find(|k| !current.contains(k.as_str())).cloned() {
                current = format!("{current} (target: {best_key})");
            } else {
                break;
            }
        } else {
            break;
        }
    }
    current
}

fn is_weak_statement(s: &str) -> bool {
    s.contains("<missing>") || s.contains("…(truncated)") || s.split_whitespace().count() < 4
}

fn compute_rubric(
    parsed: &ParsedIntent,
    gaps: &[Gap],
    clarifications: &[ClarificationQuestion],
    refined_intent: &str,
) -> ConfidenceRubric {
    // Short-circuit: a Blocker gap blocks the entire verdict (RFC 23 §5).
    // The rubric still records per-axis numbers; we floor every axis at 0.05
    // so the mean `< 0.3`, bucketing onto `ConfidenceLevel::Block`.
    let has_blocker = gaps
        .iter()
        .any(|g| g.severity == crate::prompt::types::Severity::Blocker);

    if has_blocker {
        return ConfidenceRubric {
            intent_clarity: 0.05,
            scope_clarity: 0.05,
            feasibility_clarity: 0.05,
            context_clarity: 0.1,
        };
    }

    let intent_clarity = if refined_intent.contains("<missing>") {
        0.1
    } else if !gaps.iter().any(|g| {
        matches!(
            g.kind,
            GapType::LowInformationPrompt
                | GapType::Underspecified
                | GapType::CapacityHallucination
        )
    }) {
        0.95
    } else {
        0.5
    };

    let scope_clarity = match parsed.scope {
        Scope::Project => 0.85,
        Scope::Feature => 0.7,
        Scope::Module => 0.6,
        Scope::File => 0.75,
        Scope::Unknown => 0.2,
    };

    let feasibility_clarity = if gaps.iter().any(|g| {
        matches!(
            g.kind,
            GapType::CapacityHallucination
                | GapType::UnknownToolDependency
                | GapType::CapacityOverreach
        )
    }) {
        0.15
    } else if !parsed.named_entities.is_empty() {
        0.8
    } else {
        0.4
    };

    let context_clarity = if gaps.iter().any(|g| g.kind == GapType::LostInTheMiddle) {
        0.2
    } else if !clarifications.is_empty() {
        0.5
    } else {
        0.75
    };

    ConfidenceRubric {
        intent_clarity,
        scope_clarity,
        feasibility_clarity,
        context_clarity,
    }
}

fn pick_recommended_mode(gaps: &[Gap], confidence: ConfidenceLevel) -> RecommendedMode {
    if gaps.iter().any(|g| g.kind == GapType::LostInTheMiddle) {
        return RecommendedMode::Context;
    }
    match confidence {
        ConfidenceLevel::High => RecommendedMode::Code,
        ConfidenceLevel::Medium => RecommendedMode::Architect,
        ConfidenceLevel::Low | ConfidenceLevel::Block => RecommendedMode::Ask,
    }
}

fn make_observations(gaps: &[Gap], parsed: &ParsedIntent) -> Vec<String> {
    let mut obs: Vec<String> = Vec::new();
    if gaps.is_empty() {
        obs.push("No gaps detected; proceeding to plan.".into());
    } else {
        for g in gaps {
            obs.push(format!("gap={}, severity={:?}", g.kind.tag(), g.severity));
        }
    }
    obs.push(format!("domain={}", parsed.domain.tag()));
    obs.push(format!("scope={:?}", parsed.scope));
    obs
}

fn build_consolidated(
    verdict: &PublicUnderstandingVerdict,
    gaps: &[Gap],
    options: &PipelineOptions,
) -> MissionConsolidated {
    let mission_id = Uuid::new_v4();
    let accepted_assumptions: Vec<AcceptedAssumption> = gaps
        .iter()
        .filter(|g| g.auto_resolvable)
        .map(|g| AcceptedAssumption {
            assumption: format!(
                "{} assumption seeded by heuristics for {}",
                g.kind.tag(),
                verdict.intent
            ),
            user_confirmed: false,
            auto_resolved: true,
            source: AssumptionSource::ArchitectureMemory,
        })
        .collect();

    let success_criteria: Vec<String> = if matches!(verdict.scope, Scope::Unknown) {
        vec![
            "User picks target scope (file/module/feature/project)".into(),
            "Concrete edits or research artefacts produced".into(),
        ]
    } else {
        vec![
            format!("All tests pass after edits to {:?}", verdict.scope),
            "No diff outside the implied scope (use git worktree)".into(),
        ]
    };

    let non_goals: Vec<String> = gaps
        .iter()
        .filter(|g| {
            matches!(
                g.kind,
                GapType::CapacityHallucination | GapType::CapacityOverreach
            )
        })
        .map(|g| format!("Avoid {}", g.evidence))
        .collect();

    let forbidden_actions: Vec<String> = if verdict.domain == crate::prompt::types::Domain::Frontend
    {
        vec!["do not touch backend routes".into()]
    } else {
        vec![]
    };

    let requires_research_first = gaps.iter().any(|g| {
        matches!(
            g.kind,
            GapType::UnknownToolDependency | GapType::CapacityHallucination
        )
    });

    let (locked_at, locked_by) = if options.force {
        (Some(chrono::Utc::now().to_rfc3339()), LockSource::User)
    } else {
        (None, LockSource::AutoThreshold)
    };

    MissionConsolidated {
        mission_id,
        verdict_id: verdict.verdict_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        mission_statement: verdict.intent.clone(),
        success_criteria,
        non_goals,
        accepted_assumptions,
        suggested_mode: verdict.recommended_mode,
        suggested_tooling: verdict.technology.clone(),
        forbidden_actions,
        requires_research_first,
        research_queries: None,
        locked: options.force,
        locked_at,
        locked_by,
        planning_session_id: None,
        execution_session_id: None,
    }
}
