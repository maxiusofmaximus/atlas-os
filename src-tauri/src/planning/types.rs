// OpenCode OS — Planning Engine canonical types (RFC 12 §3).
//
// These structs are the cross-RFC contract between the Planning Engine and
// the rest of the kernel:
//   - `Plan` is the persisted output of `planning::run` and the formal input
//     of the Coding Engine (RFC 13). The Coding Engine refuses to consume a
//     `Plan` whose `confidence < 0.7` (RFC 12 §7).
//   - `Objective`, `Step`, `Milestone` model the roadmap the Swarm
//     Coordinator (RFC 05) parallelises by. Each milestone exposes its
//     dependencies so the Scheduler can build a DAG.
//
// Tagging follows the TypeScript-style interface in RFC 12 §3 byte-for-byte so
// a SvelteKit frontend or external MCP consumer can `JSON.parse` either side
// and get the same structure. Field order in `Plan` mirrors the RFC's
// pseudo-code layout to keep JSON diff close to the spec when debugging.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// RFC 12 §6 — planning strategy. Chosen before the steps are emitted because
/// it dictates the shape of the milestone sequence (TDD inverts test/code
/// order, strangler fig introduces the new module before removing the old,
/// big bang collapses the milestones into one, incremental is the linear
/// default, pair forces 2-agent co-execution per step).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    Tdd,
    Strangler,
    BigBang,
    Incremental,
    Pair,
}

impl Strategy {
    pub fn tag(&self) -> &'static str {
        match self {
            Strategy::Tdd => "tdd",
            Strategy::Strangler => "strangler",
            Strategy::BigBang => "big_bang",
            Strategy::Incremental => "incremental",
            Strategy::Pair => "pair",
        }
    }
}

/// RFC 12 §4 — `impact` via the approvals escalation policy. Any
/// `Breaking` plan is escalated to `confirm` even if the active Execution
/// Mode is `auto` (RFC 02 §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Impact {
    Minor,
    Major,
    Breaking,
}

impl Impact {
    pub fn tag(&self) -> &'static str {
        match self {
            Impact::Minor => "minor",
            Impact::Major => "major",
            Impact::Breaking => "breaking",
        }
    }
}

/// RFC 12 §3 — one Objective in the plan's objectives[].
///
/// An Objective is a coarse-grained, verifiable goal. Each maps to one or
/// more `Step`s and at least one `Milestone`. The `verifiable_via` list is
/// what Validation Engine (RFC 14) consumes to decide whether the
/// Objective is satisfied without asking the user.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Objective {
    pub id: String,
    pub statement: String,
    pub verifiable_via: Vec<VerificationCriterion>,
    /// IDs of other Objectives that must complete first. Cycles are
    /// rejected in the runner before persistence (the plan would deadlock
    /// the Scheduler).
    pub depends_on: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationCriterion {
    pub kind: VerificationKind,
    pub description: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationKind {
    Test,
    TypeCheck,
    Lint,
    ManualReview,
    ResearchOutcome,
}

/// RFC 12 §3 — `Step`: the smallest unit of work the Coding Engine
/// (RFC 13) can dispatch to a subagent. Steps are sequenced within a
/// milestone and across milestones via `depends_on`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub milestone_id: String,
    pub statement: String,
    pub action: StepAction,
    pub depends_on: Vec<String>,
    /// Skill ids the step expects to activate (Skill Graph from RFC 06).
    /// `None` is valid — the step runs unskilled.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<SkillRef>,
    /// Model tier hints the Model Orchestrator (RFC 04) can consume to
    /// pick a model for this step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<ModelRef>,
    /// `true` when the step must not edit code (read-only research /
    /// clarify / context). The Coding Engine refuses to dispatch a
    /// `ReadOnly` step to a `code` subagent.
    #[serde(default)]
    pub read_only: bool,
}

/// RFC 12 §3 — `Step::action`. Mirrors the `desired_action` enum from the
/// Prompt Understanding Pipeline (RFC 23 §3) but specialised to plan steps
/// (`research` becomes a first-class citizen here, where in the verdict it
/// was a suggested mode hint).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepAction {
    Research,
    Refactor,
    Create,
    Modify,
    Debug,
    Test,
    Review,
    Clarify,
    Context,
}

impl StepAction {
    pub fn tag(&self) -> &'static str {
        match self {
            StepAction::Research => "research",
            StepAction::Refactor => "refactor",
            StepAction::Create => "create",
            StepAction::Modify => "modify",
            StepAction::Debug => "debug",
            StepAction::Test => "test",
            StepAction::Review => "review",
            StepAction::Clarify => "clarify",
            StepAction::Context => "context",
        }
    }
}

/// RFC 12 §3 — `SkillRef`: a pointer to a Skill Graph node. Full Skill
/// manifest lives in the Skills module (RFC 06); the Plan only stores the
/// id + version so a replayed plan can detect a skill drift.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillRef {
    pub skill_id: String,
    pub version: String,
}

/// RFC 12 §3 — `ModelRef`: a pointer into the Model Orchestrator registry
/// (RFC 04). `tier` and `provider` give the orchestrator enough information
/// to pick a substitute if the canonical model is hard-down (RFC 04 §6
/// fail-over policy).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelRef {
    pub model_id: String,
    pub provider: String,
    pub tier: ModelTier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    Local,
    FreeCloud,
    Paid,
    Frontier,
}

/// RFC 12 §3 — `RRRef`: research-run reference. Produced only when the
/// Planning Engine consumed a Research Run Report (RFC 10) before
/// emitting the plan (e.g. capacity_hallucination / unknown_tool_dependency
/// gaps triggered `requires_research_first`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RRRef {
    pub research_run_id: Uuid,
    pub outcome_tag: String,
}

/// RFC 12 §3 — `Milestone`. A plan groups steps into milestones so the
/// Swarm Coordinator (RFC 05) can parallelise across milestones with no
/// cross-milestone file lock contention.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub label: String,
    /// Objective ids satisfied once this milestone completes.
    pub objectives: Vec<String>,
    /// Milestone ids whose completion is a precondition for starting this
    /// milestone. Cycles are rejected in the runner.
    pub depends_on: Vec<String>,
}

/// RFC 12 §3 — `Blocker`: the `blocked[]` array. Each entry records *why*
/// the plan cannot proceed to Coding Engine, so the HUD can render the
/// remediation path (more research / clarify / human override).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Blocker {
    pub kind: BlockerKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerKind {
    /// `verdict.confidence` ∈ {Low, Block} per RFC 12 §7.
    VerdictTooLow,
    /// `Plan.confidence < CONFIDENCE_PLAN_THRESHOLD` (0.7 per RFC 12 §7).
    PlanConfidenceBelowThreshold,
    /// Mission not locked (RFC 12 §2).
    MissionNotLocked,
    /// Mission requires research run first (`requires_research_first=true`
    /// from RFC 23 §4) and no ResearchRun report is attached.
    MissingResearch,
    /// User-supplied clarification answers do not cover every gap flagged
    /// as `auto_resolvable == false` in the verdict (RFC 12 §2 —
    /// `task.rejected: reason=missing_clarification`).
    MissingClarification,
    /// `impact=breaking` requires human confirmation before Coding (RFC 12
    /// §4 + RFC 02 §4.1 approvals escalation.
    BreakingNeedsHuman,
}

/// RFC 12 §3 — `Plan`. The persisted output of the Planning Engine. The
/// Coding Engine (RFC 13) consumes this entire struct via the
/// `Plan.confidence >= PLAN_CONFIDENCE_THRESHOLD` gate (RFC 12 §7).
///
/// Field order mirrors the RFC's `Plan { ... }` pseudo-code layout so the
/// serialised JSON diff stays close to the spec when debugging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub plan_id: Uuid,
    pub mission_id: Uuid,
    pub verdict_id: Uuid,
    pub generated_at: String, // RFC 3339

    pub mission: String,
    pub objectives: Vec<Objective>,
    pub steps: Vec<Step>,
    pub strategy: Strategy,
    /// 0..=1 — heuristic in Phase 1; LLM-judge mean of sub-objective
    /// confidences in Phase 2 (RFC 04 §6).
    pub risk: f32,
    pub impact: Impact,
    pub roadmap: Vec<Milestone>,
    pub skills_used: Vec<SkillRef>,
    pub models_needed: Vec<ModelRef>,
    pub research_runs: Vec<RRRef>,
    /// 0..=1. If `< PLAN_CONFIDENCE_THRESHOLD` the `blocked[]` array MUST
    /// contain a `PlanConfidenceBelowThreshold` entry (invariant asserted
    /// in the runner).
    pub confidence: f32,
    pub resume_point: String,
    pub blocked: Vec<Blocker>,
    /// Provenance: heuristic-v0 for Phase 1, model id for Phase 2.
    pub model_id: String,
    pub elapsed_ms: u64,
}

/// RFC 12 §7 — anti-inicio defectuoso threshold. Any plan with confidence
/// strictly below this value MUST NOT init Coding Engine and MUST include
/// a `PlanConfidenceBelowThreshold` blocker.
pub const PLAN_CONFIDENCE_THRESHOLD: f32 = 0.7;

/// RFC 12 §2 — verdict confidence gate. The Planning Engine refuses to
/// generate a Plan when the associated `PublicUnderstandingVerdict.confidence`
/// is `Low` or `Block` UNLESS the user forced the mission lock from the HUD
/// (`MissionConsolidated.locked_by == User`). In that case the plan is
/// generated with a `VerdictTooLow` blocker so the human override is
/// auditable but the Coding Engine still cannot proceed (the blocker stays
/// until the user re-runs `/refine` or drops the override).
pub const VERDICT_CONFIDENCE_FLOOR: crate::prompt::types::ConfidenceLevel =
    crate::prompt::types::ConfidenceLevel::Medium;
