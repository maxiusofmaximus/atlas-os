// Atlas OS — Repair Engine canonical types (RFC 15).
//
// The Repair Engine converts a failed `ValidationReport` into a micro
// cycle: classify → analyse → propose → apply (via Coding) → re-validate.
// Phase 1 is heuristic-only and stops one step before re-validation —
// the runner returns a `RepairReport` that the Execution Supervisor
// (RFC 19) feeds back into the Coding Engine + Validation Engine loop.
//
// The runner never escalates via `Result::Err`; everything flows back as
// a structured `RepairOutcome` so the Kernel Bus / HUD can render the
// escalation path without a separate error channel (mirrors the
// Validation Engine contract from RFC 14 §3).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::coding::types::Diff;
use crate::validation::types::StageKind;

/// RFC 15 §2 — the eight error categories. Each maps to a Repair
/// strategy (column 2 of the RFC §2 table). Phase 1 only emits the
/// subset reachable from the heuristic Validation stages; the others are
/// carried through for forward compatibility with Phase 2/9 stages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClass {
    /// Lint / format smell (Biome auto-fix, clippy --fix, etc.).
    SyntaxFormat,
    /// Type-check hole (cargo check, tsc, mypy…).
    TypeCheck,
    /// Unit test failure (red assertion, missing coverage).
    TestFailure,
    /// Playwright / E2E snapshot mismatch.
    E2e,
    /// Knip-equivalent dead code.
    DeadCode,
    /// Snyk / Socket CVE or supply-chain alert.
    Vulnerability,
    /// Pulumi / OpenTofu / Docker config drift.
    Config,
    /// `Plan.confidence < 0.7` (RFC 12 §7) — not a code error; escalated
    /// to the Planning Engine for replan + research.
    RatingConfidenceLow,
}

impl ErrorClass {
    /// The Validation stage tag that produced the first failing finding.
    /// The runner uses this as the primary classifier; some stages (e.g.
    /// `LintFormat`) split findings into multiple `ErrorClass` subtypes
    /// heuristically (`no_println` → `SyntaxFormat`).
    pub fn from_stage(stage: StageKind) -> Option<Self> {
        match stage {
            StageKind::LintFormat => Some(Self::SyntaxFormat),
            StageKind::TypeCheck => Some(Self::TypeCheck),
            StageKind::UnitTests => Some(Self::TestFailure),
            StageKind::E2e => Some(Self::E2e),
            StageKind::DeadCode => Some(Self::DeadCode),
            StageKind::SupplyChain => Some(Self::Vulnerability),
            StageKind::SecurityScan => Some(Self::Vulnerability),
            StageKind::Iac => Some(Self::Config),
            StageKind::LayerBoundary => None,
            StageKind::EvidenceGate => Some(Self::TestFailure),
        }
    }

    pub fn tag(&self) -> &'static str {
        match self {
            Self::SyntaxFormat => "syntax_format",
            Self::TypeCheck => "type_check",
            Self::TestFailure => "test_failure",
            Self::E2e => "e2e",
            Self::DeadCode => "dead_code",
            Self::Vulnerability => "vulnerability",
            Self::Config => "config",
            Self::RatingConfidenceLow => "rating_confidence_low",
        }
    }
}

/// RFC 15 §2 — strategy the Repair Engine picks per `ErrorClass`.
/// Phase 1 dispatches the first five directly (heuristic fixers); the
/// remaining three are escalated immediately. `RatingConfidenceLow`
/// ALWAYS routes back to Planning (RFC 15 §2 last row).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairStrategy {
    /// Apply an auto-fix patch directly (Biome `--write`, heuristic
    /// rewrite). No Coding Engine round-trip needed.
    AutoFix,
    /// Ask the Coding Engine for an amendment with a structural
    /// constraint (RFC 13 §3 "request amendment").
    CodingAmendment,
    /// Re-run the Coding Engine with the same brief (used for flaky
    /// tests / transient assertion failures).
    CodingRetry,
    /// Escalate to Planning for replan + research (RFC 12).
    PlanningReplan,
    /// RFC 15 §4 — after 3 + 3 failures, escalate to the human via the
    /// Agent Console (RFC 24).
    HumanEscalate,
}

impl RepairStrategy {
    pub fn tag(&self) -> &'static str {
        match self {
            Self::AutoFix => "auto_fix",
            Self::CodingAmendment => "coding_amendment",
            Self::CodingRetry => "coding_retry",
            Self::PlanningReplan => "planning_replan",
            Self::HumanEscalate => "human_escalate",
        }
    }
}

/// RFC 15 §1 — one attempt logged inside `RepairReport.attempts[]`.
/// The runner records every pass — successful or not — so the Learning
/// Engine (RFC 16) can extract patterns from misses as well as hits.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepairAttempt {
    pub error_class: ErrorClass,
    pub strategy: RepairStrategy,
    /// `true` when this attempt produced a `Diff` that re-validates
    /// clean. `false` means the strategy did not converge — the runner
    /// either tries the next strategy or escalates.
    pub success: bool,
    /// Free-form root-cause note the runner records for the Learning
    /// Engine. May be empty for `AutoFix` clear-cut cases.
    pub root_cause: String,
    /// `Some(diff)` when a fix was proposed regardless of success — the
    /// Learning Engine extracts patterns from the proposed diff even
    /// when it later failed re-validation. Stored as `Box<Diff>` to
    /// keep `RepairAttempt` below the clippy large-struct threshold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposed_diff: Option<Box<Diff>>,
}

/// RFC 15 §6 — the final outcome of a Repair run. The Execution
/// Supervisor branches on this to either re-enter the Coding/Validation
/// loop or surface the issue to the human reviewer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairOutcome {
    /// A fix was applied and re-validated (RFC 15 §6 `repair.applied`).
    /// Phase 1 marks `Applied` when the heuristic produced a `Diff`
    /// whose `Self::would_revalidate_clean` predicate fires.
    Applied,
    /// A fix was proposed but the runner cannot guarantee re-validation
    /// passes without a real Validation round-trip (Phase 2 path). The
    /// Execution Supervisor re-enters the Validation Engine.
    Proposed,
    /// All strategies exhausted without convergence (RFC 15 §4).
    /// The Planning Engine is asked to replan + research.
    EscalatedPlanning,
    /// Surpassed the 3+3 attempt budget (RFC 15 §4). Surfaced to the
    /// human in the Agent Console (RFC 24).
    EscalatedHuman,
}

impl RepairOutcome {
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Proposed => "proposed",
            Self::EscalatedPlanning => "escalated_planning",
            Self::EscalatedHuman => "escalated_human",
        }
    }
}

/// RFC 15 §6 — the persisted artefact. One `RepairReport` per Repair
/// run. The Learning Engine (RFC 16 + RFC 15 §6 "ambos insertan en el
/// repositorio de patterns") consumes the full `attempts[]` chain
/// whether the run converged or not — misses are as valuable as hits
/// for pattern extraction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepairReport {
    pub repair_id: Uuid,
    /// The validation report that triggered the repair. The Journal
    /// stores both side-by-side so the HUD timeline can correlate them.
    pub triggered_by_report_id: Uuid,
    /// The diff that was being validated when Repair was invoked. The
    /// proposed fix is built ON TOP of this diff (hunk-merge semantics
    /// in the Coding Engine, RFC 13 §8).
    pub source_diff_id: Uuid,
    pub plan_id: Uuid,
    pub mission_id: Uuid,
    pub generated_at: String, // RFC 3339
    pub attempts: Vec<RepairAttempt>,
    pub outcome: RepairOutcome,
    /// The `StageKind` whose failure triggered the run. Used by the
    /// Learning Engine to bucket patterns by stage.
    pub triggering_stage: StageKind,
    /// Attempt index (0-based) of the strategy that finally converged.
    /// `None` if no attempt succeeded (outcome != Applied/Proposed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub successful_attempt: Option<u32>,
    /// `Some(diff)` iff `outcome == Applied | Proposed` and a fix was
    /// produced. The Execution Supervisor feeds this back to the
    /// Validation Engine for re-validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_diff: Option<Box<Diff>>,
    /// Free-form provenance. "heuristic-v0" for Phase 1; model id for
    /// the model-driven analyser coming with Phase 2.
    pub model_id: String,
    pub elapsed_ms: u64,
}

impl RepairReport {
    pub fn is_applied(&self) -> bool {
        matches!(self.outcome, RepairOutcome::Applied)
    }
    pub fn is_escalated(&self) -> bool {
        matches!(
            self.outcome,
            RepairOutcome::EscalatedPlanning | RepairOutcome::EscalatedHuman
        )
    }
}

/// RFC 15 §4 — attempt budget. The runner escalates to Planning after
/// `MAX_ATTEMPTS_PER_STAGE` consecutive failures for the same stage,
/// and to a human after `MAX_MISSION_FAILURES` total failures across
/// the whole mission.
pub const MAX_ATTEMPTS_PER_STAGE: u32 = 3;
pub const MAX_MISSION_FAILURES: u32 = 6;
