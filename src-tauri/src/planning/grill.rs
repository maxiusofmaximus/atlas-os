// Atlas OS — grill gate (plan 30 §A.3 / §B 3.5, RFC 12 anti-gate).
//
// Relentless interview pass over a `Plan` before `Plan.lock`: walks the
// decision tree branch-by-branch (objectives → steps → blockers → risk →
// research coverage) until every unresolved decision has a question facing
// the operator. Deterministic and LLM-free — the strong model (or the
// bundled `grill-me` skill) owns the follow-up prose; this module owns the
// question list so the gate is mechanical: `can_lock` is true only when no
// blocking question remains. Coding still refuses plans under
// `PLAN_CONFIDENCE_THRESHOLD` (RFC 12 §7); the grill is the confidence
// ladder that gets the plan there.

use serde::{Deserialize, Serialize};

use super::types::{Plan, PLAN_CONFIDENCE_THRESHOLD};

/// One grill question. `blocks_lock` marks the decisions that must resolve
/// before `Plan.lock` (missing verification, live blockers, breaking impact);
/// advisory questions (`blocks_lock == false`) raise confidence without
/// gating the lock.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrillQuestion {
    pub id: String,
    pub focus: String,
    pub question: String,
    pub blocks_lock: bool,
}

/// Outcome of the grill pass over one plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrillReport {
    pub plan_id: String,
    pub question_count: usize,
    pub blocking_count: usize,
    #[serde(default)]
    pub questions: Vec<GrillQuestion>,
    pub can_lock: bool,
}

/// Walk the plan decision tree and emit one question per unresolved
/// decision. Question order is stable (confidence → blockers → impact →
/// objectives → steps → risk → research) with `G<n>` ids so snapshots and
/// fixture tests never flake.
pub fn grill_plan(plan: &Plan) -> GrillReport {
    let mut questions: Vec<GrillQuestion> = Vec::new();
    let mut push = |focus: &str, question: String, blocks_lock: bool| {
        let id = format!("G{}", questions.len() + 1);
        questions.push(GrillQuestion {
            id,
            focus: focus.to_string(),
            question,
            blocks_lock,
        });
    };

    if plan.confidence < PLAN_CONFIDENCE_THRESHOLD {
        push(
            "confidence",
            format!(
                "Plan.confidence = {:.3} < {PLAN_CONFIDENCE_THRESHOLD}: \
                 ¿qué decisión es la más débil y qué evidencia la subiría \
                 por encima del umbral antes del lock?",
                plan.confidence
            ),
            true,
        );
    }
    for b in &plan.blocked {
        push(
            "blocker",
            format!(
                "El plan trae blocker {:?}: {}. ¿Cómo se resuelve \
                 (más research / clarify / override humano) antes del lock?",
                b.kind, b.message
            ),
            true,
        );
    }
    if matches!(plan.impact, super::types::Impact::Breaking) {
        push(
            "impact",
            "impact=breaking: ¿confirma el operador la escalada a `confirm` \
             (RFC 02 §4.1) y cuál es el plan de rollback si la ruptura muerde?"
                .to_string(),
            true,
        );
    }
    for o in &plan.objectives {
        if o.verifiable_via.is_empty() {
            push(
                "objective",
                format!(
                    "El objetivo {} (\"{}\") no tiene criterio verificable: \
                     ¿cómo sabremos que está cumplido sin preguntar al usuario?",
                    o.id, o.statement
                ),
                true,
            );
        }
    }
    if plan.steps.is_empty() {
        push(
            "steps",
            "El plan no trae steps: ¿qué primer paso concreto ejecuta la \
             misión y quién lo verifica?"
                .to_string(),
            true,
        );
    }
    for s in &plan.steps {
        if s.statement.trim().is_empty() {
            push(
                "steps",
                format!(
                    "El step {} no describe su trabajo: ¿qué hace exactamente \
                     y cuál es su done-criteria?",
                    s.id
                ),
                true,
            );
        }
    }
    if plan.risk >= 0.7 {
        push(
            "risk",
            format!(
                "risk = {:.2} (alto): ¿qué rama del árbol de decisiones se cae \
                 primero y qué spike la de-riesga antes del lock?",
                plan.risk
            ),
            false,
        );
    }
    let needs_research = plan
        .steps
        .iter()
        .any(|s| matches!(s.action, super::types::StepAction::Research));
    if needs_research && plan.research_runs.is_empty() {
        push(
            "research",
            "El plan pide steps de research sin ResearchRun adjunto: \
             ¿qué pregunta corre `atlas research query` primero y con qué \
             fuentes mínimas?"
                .to_string(),
            false,
        );
    }

    let blocking_count = questions.iter().filter(|q| q.blocks_lock).count();
    GrillReport {
        plan_id: plan.plan_id.to_string(),
        question_count: questions.len(),
        blocking_count,
        questions,
        can_lock: blocking_count == 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning::types::{
        Impact, Milestone, Objective, Plan, Step, StepAction, Strategy, VerificationCriterion,
        VerificationKind,
    };
    use uuid::Uuid;

    fn strong_plan() -> Plan {
        Plan {
            plan_id: Uuid::new_v4(),
            mission_id: Uuid::new_v4(),
            verdict_id: Uuid::new_v4(),
            generated_at: "2026-07-13T14:02:00+00:00".into(),
            mission: "Ship the feature".into(),
            objectives: vec![Objective {
                id: "O0".into(),
                statement: "Ship the feature".into(),
                verifiable_via: vec![VerificationCriterion {
                    kind: VerificationKind::Test,
                    description: "tests pass".into(),
                }],
                depends_on: Vec::new(),
            }],
            steps: vec![Step {
                id: "SO0".into(),
                milestone_id: "M1".into(),
                statement: "Implement the change".into(),
                action: StepAction::Create,
                depends_on: Vec::new(),
                skills: Vec::new(),
                models: Vec::new(),
                read_only: false,
            }],
            strategy: Strategy::Incremental,
            risk: 0.2,
            impact: Impact::Minor,
            roadmap: vec![Milestone {
                id: "M1".into(),
                label: "M1".into(),
                objectives: vec!["O0".into()],
                depends_on: Vec::new(),
            }],
            skills_used: Vec::new(),
            models_needed: Vec::new(),
            research_runs: Vec::new(),
            confidence: 0.85,
            resume_point: "SO0".into(),
            blocked: Vec::new(),
            model_id: "heuristic-v0".into(),
            elapsed_ms: 1,
        }
    }

    #[test]
    fn strong_plan_passes_grill_with_no_questions() {
        let report = grill_plan(&strong_plan());
        assert!(report.can_lock);
        assert_eq!(report.blocking_count, 0);
        assert_eq!(report.question_count, 0);
        assert!(report.questions.is_empty());
    }

    #[test]
    fn weak_plan_collects_blocking_questions_deterministically() {
        let mut plan = strong_plan();
        plan.confidence = 0.4;
        plan.impact = Impact::Breaking;
        plan.objectives[0].verifiable_via.clear();
        plan.blocked.push(crate::planning::types::Blocker {
            kind: crate::planning::types::BlockerKind::MissingResearch,
            message: "no ResearchRun attached".into(),
        });
        let once = grill_plan(&plan);
        let twice = grill_plan(&plan);
        assert_eq!(once, twice);
        assert!(!once.can_lock);
        assert!(once.blocking_count >= 4);
        assert_eq!(once.questions[0].id, "G1");
        assert_eq!(once.questions[0].focus, "confidence");
        assert!(once.questions.iter().any(|q| q.focus == "blocker"));
        assert!(once.questions.iter().any(|q| q.focus == "impact"));
        assert!(once.questions.iter().any(|q| q.focus == "objective"));
        let json = serde_json::to_string(&once).unwrap();
        let back: GrillReport = serde_json::from_str(&json).unwrap();
        assert_eq!(once, back);
    }

    #[test]
    fn empty_steps_and_high_risk_ask_before_lock() {
        let mut plan = strong_plan();
        plan.steps.clear();
        plan.risk = 0.9;
        let report = grill_plan(&plan);
        assert!(!report.can_lock);
        assert!(report.questions.iter().any(|q| q.focus == "steps"));
        let risk_q = report
            .questions
            .iter()
            .find(|q| q.focus == "risk")
            .expect("risk question");
        assert!(!risk_q.blocks_lock);
    }

    #[test]
    fn research_steps_without_runs_ask_advisory_question() {
        let mut plan = strong_plan();
        plan.steps[0].action = StepAction::Research;
        let report = grill_plan(&plan);
        let q = report
            .questions
            .iter()
            .find(|q| q.focus == "research")
            .expect("research question");
        assert!(!q.blocks_lock);
        assert!(report.can_lock);
    }
}
