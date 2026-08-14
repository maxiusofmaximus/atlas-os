// Atlas OS — Step 5 + Step 6: Clarification generation + Auto-Resolve
// (RFC 23 §2 steps 5 and 6).
//
// Phase 1: STORM K=3 perspectives, deterministic. Each detected gap with
// `auto_resolvable == false` produces one question per perspective, up to
// `PipelineOptions::max_questions`. Auto-resolvable gaps (C4 implicit
// assumption, C10 model omniscience) are turned into a default_answer
// seeded by heuristics so the HUD can render "Accept all auto-resolutions".

use crate::prompt::runner::PipelineOptions;
use crate::prompt::steps::parse::ParsedIntent;
use crate::prompt::types::{
    ClarificationQuestion, Gap, GapType, Perspective, QuestionFormat, QuestionSource,
};
use uuid::Uuid;

pub fn generate(
    parsed: &ParsedIntent,
    gaps: &[Gap],
    options: &PipelineOptions,
) -> anyhow::Result<Vec<ClarificationQuestion>> {
    let mut out: Vec<ClarificationQuestion> = Vec::new();

    let perspectives: Vec<Perspective> = options
        .perspectives
        .iter()
        .filter_map(|s| parse_perspective(s))
        .take(options.max_questions as usize)
        .collect();

    let needs_clarification = gaps
        .iter()
        .any(|g| !g.auto_resolvable && needs_question(g.kind));

    if !needs_clarification {
        return Ok(out);
    }

    let gap_budget = options.max_questions as usize;

    for (i, perspective) in perspectives.into_iter().enumerate() {
        if out.len() >= gap_budget {
            break;
        }
        let question_idx = i as u32;
        let first_gap = gaps
            .iter()
            .find(|g| !g.auto_resolvable && needs_question(g.kind))
            .cloned()
            .unwrap_or_else(|| Gap {
                kind: GapType::Underspecified,
                evidence: parsed.intent.clone(),
                severity: crate::prompt::types::Severity::Med,
                auto_resolvable: false,
            });

        let question = craft_question(&first_gap, perspective, question_idx, parsed);
        out.push(question);
    }

    // Step 6 — auto-resolve: for any auto_resolvable gap (C4 implicit
    // assumption / C10 model omniscience), seed a default answer.
    for q in out.iter_mut() {
        if let Some(auto) = gaps
            .iter()
            .find(|g| g.auto_resolvable && maps(g.kind, q.perspective))
        {
            q.auto_resolved = true;
            q.default_answer = Some(seed_default(auto, parsed));
            q.source = Some(QuestionSource {
                memory_id: Some(Uuid::nil()),
                journal_id: None,
            });
        }
    }

    // Dedup by `question` text (deterministic STORM can produce near-
    // duplicates when the gap set is small).
    out.sort_by(|a, b| a.question.cmp(&b.question));
    out.dedup_by(|a, b| a.question == b.question);

    Ok(out)
}

fn parse_perspective(s: &str) -> Option<Perspective> {
    match s.to_lowercase().as_str() {
        "developer" => Some(Perspective::Developer),
        "qa" => Some(Perspective::Qa),
        "ops" => Some(Perspective::Ops),
        "security" => Some(Perspective::Security),
        "user" => Some(Perspective::User),
        _ => None,
    }
}

fn needs_question(kind: GapType) -> bool {
    matches!(
        kind,
        GapType::Underspecified
            | GapType::LowInformationPrompt
            | GapType::CapacityHallucination
            | GapType::CapacityOverreach
            | GapType::RoleOverload
            | GapType::UnknownToolDependency
            | GapType::UserKnowledgeGap
    )
}

fn maps(kind: GapType, _p: Perspective) -> bool {
    matches!(
        kind,
        GapType::ImplicitAssumption | GapType::ModelOmniscienceAssumption
    )
}

fn craft_question(
    gap: &Gap,
    perspective: Perspective,
    idx: u32,
    parsed: &ParsedIntent,
) -> ClarificationQuestion {
    let perspective_str = match perspective {
        Perspective::Developer => "developer",
        Perspective::Qa => "qa",
        Perspective::Ops => "ops",
        Perspective::Security => "security",
        Perspective::User => "user",
    };
    let (question, format, options) = match gap.kind {
        GapType::LowInformationPrompt | GapType::Underspecified => {
            let q = format!(
                "{perspective_str} view: you said \"{}\" — which of these is closest?",
                parsed.intent
            );
            (
                q,
                QuestionFormat::MultipleChoice,
                Some(vec![
                    "A) Fix an existing bug".into(),
                    "B) Add a new feature".into(),
                    "C) Refactor / migrate".into(),
                    "D) Research / explain only".into(),
                ]),
            )
        }
        GapType::CapacityHallucination => (
            "Goal sounds beyond our current tooling — can you confirm scope?".into(),
            QuestionFormat::MultipleChoice,
            Some(vec![
                "A) Drop the impossible part and ship a smaller win".into(),
                "B) Treat as research (no code edits)".into(),
                "C) Insist; escalate to human".into(),
            ]),
        ),
        GapType::CapacityOverreach => (
            "End-to-end automation may be too large — pick scope:".into(),
            QuestionFormat::MultipleChoice,
            Some(vec![
                "A) Single file".into(),
                "B) Single module".into(),
                "C) Whole project (long-running)".into(),
            ]),
        ),
        GapType::RoleOverload => (
            "Multiple roles requested — pick one primary:".into(),
            QuestionFormat::MultipleChoice,
            Some(vec![
                "A) developer".into(),
                "B) qa".into(),
                "C) ops".into(),
                "D) security".into(),
            ]),
        ),
        GapType::UnknownToolDependency => (
            "Tool may not be available — which fallback?".into(),
            QuestionFormat::MultipleChoice,
            Some(vec![
                "A) Try the tool anyway (probe via research)".into(),
                "B) Use a well-known alternative".into(),
                "C) Abort — tool is required".into(),
            ]),
        ),
        GapType::UserKnowledgeGap => (
            "I can teach this step-by-step; do you want mentor mode?".into(),
            QuestionFormat::MultipleChoice,
            Some(vec!["A) yes (mentor)".into(), "B) no (just do it)".into()]),
        ),
        GapType::ImplicitAssumption => (
            "I assumed the default project tech stack — correct?".into(),
            QuestionFormat::MultipleChoice,
            Some(vec!["A) yes".into(), "B) no (please specify)".into()]),
        ),
        GapType::ModelOmniscienceAssumption => (
            "I may not actually know your repo — should I scan it?".into(),
            QuestionFormat::MultipleChoice,
            Some(vec![
                "A) yes (scan repo)".into(),
                "B) no (I'll tell you)".into(),
            ]),
        ),
        GapType::LostInTheMiddle => (
            "Long prompt; key verb may be buried — can you restate the action?".into(),
            QuestionFormat::Text,
            None,
        ),
    };

    ClarificationQuestion {
        id: format!("q-{idx}-{perspective_str}"),
        perspective,
        question,
        justification: format!("gap={}, evidence={:?}", gap.kind.tag(), gap.evidence),
        format,
        options,
        default_answer: None,
        auto_resolved: false,
        source: None,
    }
}

fn seed_default(gap: &Gap, parsed: &ParsedIntent) -> String {
    match gap.kind {
        GapType::ImplicitAssumption => format!("Assume default stack for {}", parsed.domain.tag()),
        GapType::ModelOmniscienceAssumption => "Scan repo first, then proceed".to_string(),
        _ => String::new(),
    }
}
