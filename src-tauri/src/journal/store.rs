// OpenCode OS — Journal store: typed row representations exposed via IPC.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: i64,
    pub ts: String,
    pub kind: String,
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mission {
    pub id: Uuid,
    pub label: String,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub seq: i64,
    pub ts: String,
    pub actor: String,
    pub action: String,
    pub inputs: serde_json::Value,
    pub outputs: serde_json::Value,
}

/// Row projection of `prompt_verdicts` for the `journal tail` / HUD live stream.
/// The full payload is available on demand via `Journal::verdict_payload`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerdictRow {
    pub verdict_id: Uuid,
    pub session_id: Uuid,
    pub mission_id: Option<Uuid>,
    pub raw_prompt: String,
    pub ts: String,
    pub confidence: String,
    pub rubric_mean: f32,
    pub recommended_mode: String,
    pub gap_count: i64,
}

/// Row projection of `mission_consolidated` for the same consumers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsolidatedRow {
    pub mission_id: Uuid,
    pub verdict_id: Uuid,
    pub generated_at: String,
    pub mission_statement: String,
    pub suggested_mode: String,
    pub locked: bool,
    pub locked_at: Option<String>,
    pub locked_by: Option<String>,
    pub requires_research: bool,
}

/// Row projection of `plans` for the `journal tail` / HUD live stream.
/// The full payload is available on demand via `Journal::plan_payload`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanRow {
    pub plan_id: Uuid,
    pub mission_id: Uuid,
    pub verdict_id: Uuid,
    pub generated_at: String,
    pub strategy: String,
    pub risk: f32,
    pub impact: String,
    pub confidence: f32,
    pub resume_point: String,
    pub blocker_count: i64,
}

/// Row projection of `diffs` for the Journal tail / HUD live stream.
/// The full payload (hunks, narrative, research_refs) is available on
/// demand via `Journal::diff_payload`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiffRow {
    pub diff_id: Uuid,
    pub plan_id: Uuid,
    pub mission_id: Uuid,
    pub step_id: String,
    pub agent_id: Uuid,
    pub generated_at: String,
    pub lines_added: i64,
    pub lines_removed: i64,
    pub file_count: i64,
}

/// Row projection of `validation_reports` for the Journal tail / HUD
/// live stream (RFC 24 §3) and the Repair Engine (RFC 15). The full
/// payload (`ValidationReport` with the per-stage `StageSummary` lists)
/// is available on demand via `Journal::report_payload`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationReportRow {
    pub report_id: Uuid,
    pub diff_id: Uuid,
    pub plan_id: Uuid,
    pub mission_id: Uuid,
    pub generated_at: String,
    pub mode: String,
    pub outcome: String,
    /// `None` when `outcome == "pass"`; the StageKind tag of the first
    /// failing stage otherwise.
    pub failed_stage: Option<String>,
    pub stage_count: i64,
}

/// Row projection of `repair_runs` for the Journal tail / HUD live
/// stream (RFC 24 §3) and the Learning Engine (RFC 16). The full payload
/// (`RepairReport` with the per-attempt chain) is available on demand
/// via `Journal::repair_payload`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepairRunRow {
    pub repair_id: Uuid,
    pub triggered_by_report_id: Uuid,
    pub source_diff_id: Uuid,
    pub plan_id: Uuid,
    pub mission_id: Uuid,
    pub generated_at: String,
    pub outcome: String,
    pub triggering_stage: String,
    pub attempt_count: i64,
    /// `None` when no attempt succeeded (outcome != Applied/Proposed).
    pub successful_attempt: Option<i64>,
}

/// Row projection of `pattern_runs` for the Journal tail / HUD live
/// stream (RFC 24 §3) and the Skill Compressor's duplicate-detection
/// queries (RFC 16 §5). The full payload (`LearnOutcome` with the
/// `Pattern` + metrics + evidence diff) is available on demand via
/// `Journal::pattern_payload`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternRow {
    pub pattern_id: Uuid,
    pub learn_id: Uuid,
    pub source_repair_id: Uuid,
    pub source_mission_id: Uuid,
    pub rule_id: String,
    pub stage: String,
    pub strategy: String,
    pub lifecycle: String,
    pub priority: i64,
    pub confidence: f64,
    pub was_correct: i64,
    pub tests_passed: i64,
    pub generated_at: String,
}

/// Row projection of `mission_checkpoints` for the Journal tail / HUD
/// recent-checkpoints panel (RFC 24 §3) and the supervisor's own
/// `resume(mission_id)` lookup (RFC 19 §5).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckpointRow {
    pub checkpoint_id: Uuid,
    pub mission_id: Uuid,
    pub phase: String,
    pub generated_at: String,
}

/// Row projection of `skill_manifests` for the Journal tail / HUD
/// Skill Graph panel (RFC 24 §3) and the §3 candidate pipeline's
/// "ORDER BY priority" path. The full payload (`SkillManifest` with
/// dependencies/conflicts/compatible_models/vector_embedding) is
/// available on demand via `Journal::skill_payload`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillRow {
    pub skill_id: String,
    pub version: String,
    pub engine: String,
    pub priority: i64,
    pub domain: Option<String>,
    pub language: Option<String>,
    pub framework: Option<String>,
    pub confidence: f64,
    pub auto_generated: bool,
    pub verified: bool,
    pub requires_sandbox: bool,
    pub generated_at: String,
}

/// Row projection of `model_swaps` (M10) for the HUD tail (RFC 24 §3)
/// and the operator "show me every swap on mission X" CLI query. The
/// full `ModelSwapped` bus event is preserved in `payload` for replay.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelSwapRow {
    pub swap_id: Uuid,
    pub mission_id: Uuid,
    pub prev_model_id: String,
    pub new_model_id: String,
    pub initiator: String,
    pub occurred_at: String,
}

/// Row projection of `step_states` (M11) for the HUD step pills (RFC 27
/// §G). One row per `(mission_id, plan_id, step_id)` — the latest phase
/// the supervisor observed for that step.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StepStateRow {
    pub mission_id: Uuid,
    pub plan_id: Uuid,
    pub step_id: String,
    pub phase: String,
    pub updated_at: String,
}
