// Atlas OS — Coding Engine canonical types (RFC 13 §2, §8).
//
// The Coding Engine emits `Diff`s, never full-file rewrites (RFC 13 §8):
//   - The Reviewer subagent validates diffs.
//   - The Merger subagent fuses diffs in the swarm.
//   - The Journal persists diffs as the audit unit.
//   - The Learning Engine extracts patterns from diffs.
//
// A `Diff` is structured (not a textual unified-diff blob) so the kernel
// can apply it without shelling out to `patch`, and so a subagent can
// stream hunks individually to the HUD via the `agent.diff` Kernel Bus
// event (RFC 02 §3.1 / RFC 24 §4.1).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// RFC 13 §8 — one hunk inside a file edit. `old_start..old_end` is the
/// half-open range of lines being replaced (0..=0 means "pure insertion
/// at the start of the file"). `new_lines` is the replacement; an empty
/// `Vec` denotes a pure deletion. Lines are stored WITHOUT trailing
/// newlines so callers can join with `\n` deterministically.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hunk {
    pub old_start: u32,
    pub old_end: u32,
    pub new_lines: Vec<String>,
    /// Free-form justification surfaced in the Agent Console (RFC 13 §6).
    /// "what it did / what it does now / why" — keeps the human approver
    /// in the loop without re-reading the diff.
    pub rationale: String,
}

/// RFC 13 §8 — one file's worth of hunks. Multiple `Hunk`s per file are
/// allowed (multi-hunk diffs); the kernel applies them in increasing
/// `old_start` order without overlap (an invariant the runner asserts
/// before persisting).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEdit {
    pub path: String,
    pub hunks: Vec<Hunk>,
    /// `true` when the file is being created (path does not exist yet).
    /// RFC 13 §3 MSW-first and Prisma-touch rules use this flag.
    #[serde(default)]
    pub is_new_file: bool,
    /// `true` when the file is being deleted (all hunks collapse to a
    /// single delete with `new_lines == []` and `old_start == 1,
    /// old_end = line_count + 1`).
    #[serde(default)]
    pub is_delete: bool,
}

impl FileEdit {
    /// Total lines added across all hunks. Drives the `agent.diff`
    /// Kernel Bus event payload (RFC 02 §3.1).
    pub fn lines_added(&self) -> u32 {
        self.hunks.iter().map(|h| h.new_lines.len() as u32).sum()
    }

    /// Total lines removed across all hunks. Same consumer as `lines_added`.
    pub fn lines_removed(&self) -> u32 {
        self.hunks
            .iter()
            .map(|h| h.old_end.saturating_sub(h.old_start))
            .sum()
    }

    /// Assert that hunks do not overlap and are sorted by `old_start`.
    /// Called by the runner before persisting — the kernel refuses to
    /// emit a `Diff` whose hunks would race when applied.
    pub fn hunks_are_disjoint_and_sorted(&self) -> bool {
        let mut cursor: u32 = 0;
        for h in &self.hunks {
            if h.old_start < cursor {
                return false;
            }
            if h.old_end < h.old_start {
                return false;
            }
            cursor = h.old_end;
        }
        true
    }
}

/// RFC 13 §2 — the output of the Coding Engine for a single `Step`. One
/// `Step` may produce multiple `FileEdit`s (RFC 13 §8 multi-file diff).
/// The `Diff` is the unit the Reviewer subagent (RFC 05) consumes and the
/// Journal persists (RFC 02 §3.4). The application of a `Diff` to disk
/// only happens after the Validation Engine (RFC 14) confirms the
/// pre-application snapshot — the Coding Engine never writes files
/// directly.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diff {
    pub diff_id: Uuid,
    pub plan_id: Uuid,
    pub mission_id: Uuid,
    pub step_id: String,
    pub agent_id: Uuid,
    pub generated_at: String, // RFC 3339

    pub files: Vec<FileEdit>,

    /// Tail of the "what it did / what it does now / why" narrative the
    /// HUD Agent Console renders verbatim (RFC 13 §6).
    pub narrative: String,

    /// Research refs cited in the narrative (RFC 13 §6 "evidence").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub research_refs: Vec<Uuid>,

    /// Risk decision surfaced to the human reviewer for `impact=breaking`
    /// plans (RFC 13 §6 "Decisión de riesgo"). `None` for non-breaking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_decision: Option<String>,

    /// Provenance: "heuristic-v0" for Phase 1; model id for Phase 2.
    pub model_id: String,
    pub elapsed_ms: u64,
}

/// RFC 13 §3 / Capacity Resolver (RFC 02 §4) — the per-step skills the
/// Coding Engine is allowed to invoke. The runner cross-checks this list
/// against the `Plan.skills_used` so a step cannot silently pull in a
/// skill the planner did not approve.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedSkill {
    pub skill_id: String,
    pub version: String,
}

/// RFC 13 — outcome of `coding::run`. The engine NEVER panics on a guard
/// violation; it returns a `CodingOutcome` with the rejection recorded so
/// the HUD/Journal can render the rejection path (RFC 12 §7-style).
///
/// `Emitted` boxes the `Diff` so the `CodingOutcome` enum stays small
/// (clippy::large_enum_variant) — `Rejected` is the common case when
/// guards fire and should not pay the size tax of the full `Diff`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CodingOutcome {
    Emitted { diff: Box<Diff> },
    Rejected { reason: CodingRejection },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodingRejection {
    /// RFC 12 §7 — `Plan.confidence < 0.7` or `Plan.blocked` is non-empty.
    PlanBlocked,
    /// RFC 13 §2 — `Step.read_only == true` cannot be dispatched to a
    /// `code` subagent.
    ReadOnlyStep,
    /// RFC 13 §3 — a skill in "approved" is not present in `Plan.skills_used`.
    SkillNotApproved,
    /// RFC 13 §8 — emitted hunks are not disjoint / sorted (defensive —
    /// heuristic planner should never produce overlapping hunks).
    OverlappingHunks,
}
