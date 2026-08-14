// Atlas OS — Learning Engine canonical types (RFC 16).
//
// The Learning Engine converts each Repair outcome into a reusable
// `Pattern` (a draft rule) so the kernel never repeats the same mistake
// twice (RFC 16 §2 Reflection Loop). Phase 1 is heuristic-only: the
// runner derives patterns by inspecting the `RepairReport.attempts[]`
// chain — no model calls, no embeddings. Phase 2 will feed the same
// `Pattern` shape into a vector store and the Skill Compressor (RFC 16
// §5) once `fastembed-rs` lands.
//
// Rules are NOT free-form text: they are JSON-serialisable so the
// Validation Engine and the Reasoning Engine can parse them directly
// (RFC 16 §4 "Auto-reglas del harness"). A future Sempigra-layer will
// compile `Pattern`s into real Semgrep rules.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::coding::types::Diff;
use crate::repair::types::RepairStrategy;
use crate::validation::types::StageKind;

/// RFC 16 §2 — lifecycle of a Learning Engine rule. The runner emits
/// every new pattern as `Draft`; promotion to `Candidate` and `Active`
/// is decided by the reviewer / vote pipeline (Phase 2). `Deprecated`
/// marks rules superseded by a more specific one or by a Skill
/// compression (RFC 16 §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleLifecycle {
    /// Just emitted by the runner; priority 0; not yet verified.
    Draft,
    /// Reviewed by a reviewer subagent; priority 30; `verified=true`
    /// but no run record yet.
    Candidate,
    /// Verified AND ran N times correctly; priority 60+; actively
    /// consulted by the Validation / Coding engines.
    Active,
    /// Superseded or contradicted by reality. Kept in the store for
    /// audit (RFC 16 §6) but never consulted.
    Deprecated,
}

impl RuleLifecycle {
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Candidate => "candidate",
            Self::Active => "active",
            Self::Deprecated => "deprecated",
        }
    }

    /// RFC 16 §2 — default priority per lifecycle stage. The runner
    /// uses this when emitting a fresh `Draft`; promotions bump it
    /// through the bands.
    pub fn default_priority(self) -> u32 {
        match self {
            Self::Draft => 0,
            Self::Candidate => 30,
            Self::Active => 60,
            Self::Deprecated => 0,
        }
    }
}

/// RFC 16 §3 — the `when` predicate. Phase 1 records the minimal
/// triplet (stage + rule + lang) needed for the Validation Engine to
/// match. Phase 2 will add a `file_glob` and a `code_predicate`
/// (AST-aware) once the LSP host lands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleWhen {
    pub stage: StageKind,
    /// The `Finding.rule` string produced by the Validation stage
    /// (e.g. "no_println", "unbalanced_braces"). Empty for stages
    /// that do not emit rule-tagged findings.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pattern: String,
    /// `"rust"`, `"typescript"`, `"*"`. Heuristic default `"*"`.
    #[serde(default = "star_lang")]
    pub lang: String,
}

fn star_lang() -> String {
    "*".into()
}

/// RFC 16 §3 — the `then` action. For Phase 1 the runner records the
/// `RepairStrategy` that produced a successful `AutoFix`/`CodingAmendment`
/// plus an optional `diff_hint` (the literal hunk that converged) so
/// the Coding Engine can replay it on the next match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleThen {
    pub strategy: RepairStrategy,
    /// The skill id the runner would invoke in Phase 2 once the
    /// Skill Graph (RFC 06) is wired. Empty string means "no skill —
    /// use the heuristic AutoFix layer directly".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub skill: String,
    /// One-line human-readable hint surfaced to the Coding Engine
    /// (RFC 13 §3 "diff_hint"). Example: "remove unused export".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub diff_hint: String,
}

/// RFC 16 §3 — one persisted pattern. Corresponds 1:1 to a row in the
/// `pattern_runs` Journal table (M7). The Learning Engine runner emits
/// zero or one `Pattern` per `RepairReport` consumed; id is a stable
/// `r-YYYY-MM-DD-NNN` string so the HUD can render it verbatim.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pattern {
    pub pattern_id: Uuid,
    /// RFC 16 §3 example: `r-2026-07-04-001`. Stable, human-readable.
    /// The runner builds it from the mission date + a per-mission
    /// counter so two patterns from the same day don't collide.
    pub rule_id: String,
    pub when: RuleWhen,
    pub then: RuleThen,
    /// RFC 16 §3 — `[0.0, 1.0]`. Phase 1 starts at `0.5` for `Draft`
    /// rules (a successful AutoFix with a clean revalidation bumps it
    /// to `0.7`; a miss drops it to `0.3`).
    pub confidence: f32,
    pub lifecycle: RuleLifecycle,
    pub priority: u32,
    /// References to the Journal rows that act as evidence for this
    /// rule — the `repair_id` that produced it plus any upstream
    /// `validation_report_id` and `diff_id`. The HUD timeline (RFC 24
    /// §3) joins on these to render the "why was this rule created"
    /// panel.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Uuid>,
    /// RFC 3339 timestamp of pattern creation. Updated on promotion.
    pub generated_at: String,
    /// Provenance. `"heuristic-v0"` for Phase 1; model id for Phase 2.
    pub model_id: String,
    pub elapsed_ms: u64,
}

impl Pattern {
    /// True when the rule has been observed running correctly at least
    /// once AND is still in the consultable lifecycle states.
    pub fn is_consultable(&self) -> bool {
        matches!(
            self.lifecycle,
            RuleLifecycle::Candidate | RuleLifecycle::Active
        )
    }

    /// True when the pattern has been retired (manual or automatic).
    pub fn is_deprecated(&self) -> bool {
        matches!(self.lifecycle, RuleLifecycle::Deprecated)
    }
}

/// RFC 16 §7 — metrics bundle the Learning Engine emits alongside the
/// `Pattern`. The Command Center (RFC 24) paints these on the per-rule
/// card; Phase 1 only fills the counters it can derive heuristically.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternMetrics {
    /// Times the rule fired and the downstream re-validation passed.
    pub was_correct: u32,
    /// Times the rule fired and was vetoed by a downstream gate
    /// (Validation Failed / Reviewer rejected).
    pub was_blocked: u32,
    /// Times the rule fired and the patched diff's unit tests passed.
    pub tests_passed: u32,
    /// Times the rule fired and was explicitly approved by the user
    /// in the Agent Console (RFC 24 §3 "approve / undo").
    pub approved_by_user: u32,
}

/// RFC 16 §2 — the runner's output. One `LearnOutcome` per
/// `RepairReport` consumed. `NoPattern` is emitted when the Repair
/// run was either a no-op (no attempts) or an `EscalatedHuman` — those
/// runs are audited but do not produce a reusable rule (RFC 16 §8
/// safety policy: rules that touched the Security Layer need explicit
/// approval before becoming `Draft`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearnOutcome {
    pub learn_id: Uuid,
    pub source_repair_id: Uuid,
    pub source_mission_id: Uuid,
    pub generated_at: String,
    /// `Some(pattern)` when the runner produced a new draft rule;
    /// `None` for `NoPattern` outcomes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<Pattern>,
    /// RFC 16 §7 — metrics snapshot at emission time. Phase 1 fills
    /// only `was_correct` (1 iff the repair was `Applied`).
    pub metrics: PatternMetrics,
    /// Free-form note the runner records for the audit log (RFC 16 §6).
    /// Example: "AutoFix on `no_println` converged — drafting rule".
    pub note: String,
    /// The diff that the pattern references as evidence (the proposed
    /// diff from the successful `RepairAttempt`). The Journal stores
    /// this as an FK so the HUD can fetch the original hunk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_diff: Option<Box<Diff>>,
}

/// RFC 16 §2 — the single public entry point of the Learning Engine.
/// Construct with `LearnInput::new(&repair)` and pass to `learning::run`.
#[derive(Clone, Debug)]
pub struct LearnInput<'a> {
    pub repair: &'a crate::repair::types::RepairReport,
    /// Per-mission counter used to build the `rule_id`. The Execution
    /// Supervisor (RFC 19) tracks this and passes it in so two patterns
    /// from the same mission don't collide. Defaults to 0.
    pub mission_pattern_index: u32,
}

impl<'a> LearnInput<'a> {
    pub fn new(repair: &'a crate::repair::types::RepairReport) -> Self {
        Self {
            repair,
            mission_pattern_index: 0,
        }
    }

    pub fn with_pattern_index(mut self, n: u32) -> Self {
        self.mission_pattern_index = n;
        self
    }
}
