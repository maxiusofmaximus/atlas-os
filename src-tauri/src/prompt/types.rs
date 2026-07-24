// OpenCode OS — Prompt Understanding Pipeline canonical types (RFC 23 §3-§4).
//
// These structs are the cross-RFC contract between the Prompt Understanding
// Pipeline and the rest of the kernel:
//   - `PublicUnderstandingVerdict` is the output of step 7 and the persistable
//     artefact stored in the Journal (`prompt_verdicts` table, Fase 1.1).
//   - `MissionConsolidated` is the formal input to the Planning Engine (RFC 12).
//     The Planning Engine refuses to consume a `MissionConsolidated` whose
//     `locked == false`.
//
// Tagging follows the TypeScript interfaces in RFC 23 §3/§4 byte-for-byte so a
// SvelteKit frontend or external consumer can `JSON.parse` either side and
// get the same structure.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Closed catalogue of user anti-patterns from RFC 23 §1 (`C1..C10`).
/// Kept exhaustive by hand because every detector and every resolution path
/// is keyed off this enum; an `Other` variant would let a gap slip past the
/// resolution router and crash the pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapType {
    /// C1 — grandiosity impossible with available tools ("fly through the galaxy").
    CapacityHallucination,
    /// C2 — edge-case tool / dependency the kernel has never seen.
    UnknownToolDependency,
    /// C3 — instruction is underspecified ("make it better").
    Underspecified,
    /// C4 — implicit assumption the user did not state.
    ImplicitAssumption,
    /// C5 — capacity overreach the user cannot back up.
    CapacityOverreach,
    /// C6 — user lacks the domain knowledge to specify the request.
    UserKnowledgeGap,
    /// C7 — low-information prompt ("go", "fix", "ok").
    LowInformationPrompt,
    /// C8 — long prompt with the operative clause buried mid-text
    /// (Liu et al. 2023, "Lost in the Middle").
    LostInTheMiddle,
    /// C9 — too many roles requested simultaneously.
    RoleOverload,
    /// C10 — assumes the model is omniscient ("you already know...").
    ModelOmniscienceAssumption,
}

impl GapType {
    /// Tag string persisted in SQL and surfaced over IPC. Mirrors the serde
    /// `rename_all = "snake_case"` discriminator so the on-wire and on-disk
    /// forms always agree.
    pub fn tag(&self) -> &'static str {
        match self {
            GapType::CapacityHallucination => "capacity_hallucination",
            GapType::UnknownToolDependency => "unknown_tool_dependency",
            GapType::Underspecified => "underspecified",
            GapType::ImplicitAssumption => "implicit_assumption",
            GapType::CapacityOverreach => "capacity_overreach",
            GapType::UserKnowledgeGap => "user_knowledge_gap",
            GapType::LowInformationPrompt => "low_information_prompt",
            GapType::LostInTheMiddle => "lost_in_the_middle",
            GapType::RoleOverload => "role_overload",
            GapType::ModelOmniscienceAssumption => "model_omniscience_assumption",
        }
    }
}

/// Same 4-level confidence ladder as RFC 23 §5 and the
/// already-existing `crate::core::bus::Confidence`. We re-declare here as a
/// struct-local copy with richer semantics so the Prompt Understanding
/// Pipeline can carry the rubric score that produced it; the Kernel Bus
/// variant remains a 1-byte tag for wire efficiency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
    Block,
}

impl ConfidenceLevel {
    /// Numeric floor of the bucket, copied from RFC 23 §5. The rubric score
    /// (mean of `intent_clarity`, `scope_clarity`, `feasibility_clarity`,
    /// `context_clarity`) is bucketed onto these thresholds.
    pub fn threshold(self) -> f32 {
        match self {
            ConfidenceLevel::High => 0.85,
            ConfidenceLevel::Medium => 0.6,
            ConfidenceLevel::Low => 0.3,
            ConfidenceLevel::Block => 0.0,
        }
    }

    /// Inverse of `threshold`: pick the bucket a rubric mean falls into.
    /// Ties go to the higher bucket so exactly `0.85` is `High`.
    pub fn from_rubric(score: f32) -> Self {
        if score >= 0.85 {
            Self::High
        } else if score >= 0.6 {
            Self::Medium
        } else if score >= 0.3 {
            Self::Low
        } else {
            Self::Block
        }
    }

    /// String persisted to the `prompt_verdicts.confidence` column. Mirrors
    /// the serde discriminator so `.from_str` on the JSON form and the SQL
    /// column always agree.
    pub fn tag(&self) -> &'static str {
        match self {
            ConfidenceLevel::High => "HIGH",
            ConfidenceLevel::Medium => "MEDIUM",
            ConfidenceLevel::Low => "LOW",
            ConfidenceLevel::Block => "BLOCK",
        }
    }
}

impl From<ConfidenceLevel> for crate::core::bus::Confidence {
    fn from(level: ConfidenceLevel) -> Self {
        match level {
            ConfidenceLevel::High => crate::core::bus::Confidence::High,
            ConfidenceLevel::Medium => crate::core::bus::Confidence::Medium,
            ConfidenceLevel::Low => crate::core::bus::Confidence::Low,
            ConfidenceLevel::Block => crate::core::bus::Confidence::Block,
        }
    }
}

/// RFC 23 §3 — `ClarificationQuestion`.
///
/// One question generated by step 5 (STORM K=3 perspectives) and optionally
/// pre-filled by step 6 (Architecture Memory auto-resolve). The HUD renders
/// these as selectable cards; the user's answer is stored beside the verdict.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClarificationQuestion {
    pub id: String,
    pub perspective: Perspective,
    pub question: String,
    /// Cites the gap (e.g. `GapType::Underspecified` + the literal substring)
    /// so the HUD can show *why* the question was asked.
    pub justification: String,
    pub format: QuestionFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_answer: Option<String>,
    /// `true` when step 6 filled `default_answer` from Architecture Memory.
    #[serde(default)]
    pub auto_resolved: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<QuestionSource>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Perspective {
    Developer,
    Qa,
    Ops,
    Security,
    User,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionFormat {
    MultipleChoice,
    Text,
    FileUrl,
    ToolInstallUrl,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuestionSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal_id: Option<i64>,
}

/// RFC 23 §3 — one detected gap from step 3.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gap {
    #[serde(rename = "type")]
    pub kind: GapType,
    /// Literal snippet of the user's raw prompt that triggered the detector.
    pub evidence: String,
    pub severity: Severity,
    /// `true` when step 6 (Architecture Memory lookup) can resolve this gap
    /// without asking the user.
    pub auto_resolvable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Med,
    High,
    Blocker,
}

/// RFC 23 §3 — `PublicUnderstandingVerdict`. The full output of step 7.
///
/// Field order mirrors the RFC's interface so serialised JSON diff stays
/// close to the spec when debugging.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicUnderstandingVerdict {
    pub verdict_id: Uuid,
    pub session_id: Uuid,
    pub raw_prompt: String,
    pub timestamp: String, // RFC 3339

    // step 2 — parse
    pub intent: String,
    pub intent_hypotheses: Vec<IntentHypothesis>,
    pub keys: Vec<String>,
    pub named_entities: Vec<NamedEntity>,
    pub domain: Domain,
    pub technology: Vec<String>,
    pub desired_action: DesiredAction,
    pub scope: Scope,
    pub implicit_signals: Vec<String>,

    // step 3 — gaps
    pub gaps: Vec<Gap>,

    // step 4 — similar missions
    pub similar_missions: Vec<SimilarMission>,

    // step 5 + 6 — clarification
    pub clarification_questions: Vec<ClarificationQuestion>,

    // step 7 — verdict
    pub confidence: ConfidenceLevel,
    pub confidence_rubric: ConfidenceRubric,
    pub observations: Vec<String>,
    pub recommended_mode: RecommendedMode,

    // provenance
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judge_model_id: Option<String>,
    pub elapsed_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentHypothesis {
    pub rank: u32,
    pub text: String,
    /// 0..1, scored by LLM-as-judge in the future; heuristic baseline in Fase 1.
    pub feasibility_score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamedEntity {
    pub text: String,
    pub r#type: String,
    pub in_kb: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Frontend,
    Backend,
    Devops,
    Data,
    Ml,
    Docs,
    Unknown,
}

impl Domain {
    pub fn tag(&self) -> &'static str {
        match self {
            Domain::Frontend => "frontend",
            Domain::Backend => "backend",
            Domain::Devops => "devops",
            Domain::Data => "data",
            Domain::Ml => "ml",
            Domain::Docs => "docs",
            Domain::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesiredAction {
    Create,
    Modify,
    Debug,
    Explain,
    Test,
    Research,
    Refactor,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    File,
    Module,
    Feature,
    Project,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimilarMission {
    pub journal_id: i64,
    /// Cosine similarity 0..1 (sqlite-vec). 0 when the feature is disabled
    /// or no similar mission exists yet.
    pub similarity: f32,
    pub outcome: SimilarMissionOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflexion: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimilarMissionOutcome {
    Success,
    Failed,
    Abandoned,
    Refined,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ConfidenceRubric {
    pub intent_clarity: f32,
    pub scope_clarity: f32,
    pub feasibility_clarity: f32,
    pub context_clarity: f32,
}

impl ConfidenceRubric {
    /// Simple arithmetic mean, used by `from_rubric` bucketing. RFC 23 §5
    /// mentions LLM-as-judge with randomised order to defeat position bias;
    /// the heuristic impl in Fase 1 just averages the four sub-scores.
    pub fn mean(&self) -> f32 {
        (self.intent_clarity + self.scope_clarity + self.feasibility_clarity + self.context_clarity)
            / 4.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendedMode {
    Ask,
    Architect,
    Code,
    Context,
}

impl RecommendedMode {
    pub fn tag(&self) -> &'static str {
        match self {
            RecommendedMode::Ask => "ask",
            RecommendedMode::Architect => "architect",
            RecommendedMode::Code => "code",
            RecommendedMode::Context => "context",
        }
    }
}

/// RFC 23 §4 — `MissionConsolidated`. The locked input the Planning Engine
/// (RFC 12) consumes. `locked == false` missions are never forwarded to
/// Planning; step 9 flips this flag and emits `BusEventKind::MissionConsolidated`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MissionConsolidated {
    pub mission_id: Uuid,
    pub verdict_id: Uuid,
    pub generated_at: String, // RFC 3339

    pub mission_statement: String,
    pub success_criteria: Vec<String>,
    pub non_goals: Vec<String>,

    pub accepted_assumptions: Vec<AcceptedAssumption>,

    pub suggested_mode: RecommendedMode,
    pub suggested_tooling: Vec<String>,
    pub forbidden_actions: Vec<String>,
    pub requires_research_first: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub research_queries: Option<Vec<String>>,

    pub locked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked_at: Option<String>,
    pub locked_by: LockSource,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planning_session_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_session_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AcceptedAssumption {
    pub assumption: String,
    pub user_confirmed: bool,
    pub auto_resolved: bool,
    pub source: AssumptionSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssumptionSource {
    ArchitectureMemory,
    Journal,
    UserAnswer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockSource {
    User,
    AutoThreshold,
}

impl LockSource {
    pub fn tag(&self) -> &'static str {
        match self {
            LockSource::User => "user",
            LockSource::AutoThreshold => "auto_threshold",
        }
    }
}
