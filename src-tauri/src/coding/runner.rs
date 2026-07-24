// OpenCode OS — Coding Engine runner (RFC 13).
//
// Phase 1: heuristic-only. The runner produces a `Diff` from a `Plan` +
// `Step` using deterministic rules (file creation, file deletion, simple
// append). No LLM call. The signature of `run` is stable and matches the
// LLM-driven coder coming with RFC 04 so the rest of the kernel stays
// forward-compatible.
//
// The runner never writes to disk. It returns a `Diff` (or a
// `CodingOutcome::Rejected`) and the caller (the Execution Supervisor,
// RFC 19) decides when to apply it after Validation (RFC 14) signs off.

use std::time::Instant;
use uuid::Uuid;

use crate::coding::types::{ApprovedSkill, CodingOutcome, CodingRejection, Diff, FileEdit, Hunk};
use crate::planning::types::{Plan, Step, StepAction, PLAN_CONFIDENCE_THRESHOLD};

/// Inputs the runner needs to fence RFC 12 §7 (Plan confidence gate) and
/// RFC 13 §3 (approved-skills gate). The `Plan` is referenced by slice so
/// the caller (Execution Supervisor) keeps ownership — the runner never
/// mutates the plan; the timeline invariant (RFC 02 §3.1) requires this.
#[derive(Clone, Debug)]
pub struct CodingInput<'a> {
    pub plan: &'a Plan,
    pub step: &'a Step,
    pub agent_id: Uuid,
    /// Skills approved by the Skill Graph (RFC 06) for this step. The
    /// runner cross-checks each id against `Plan.skills_used` so a step
    /// cannot silently pull in a skill the planner did not bless
    /// (RFC 13 §3 "reglas, no consejos").
    pub approved_skills: Vec<ApprovedSkill>,
    /// Existing file contents the Coding Engine can read to compute
    /// hunks. The runner does NOT touch the filesystem — Phase 1 only,
    /// the caller supplies the directories / file bodies via this map.
    /// `path -> contents` (Unix `\n` line endings, no trailing newline).
    pub workspace_files: Vec<WorkspaceFile>,
    /// Free-form rationale the human reviewer reads in the Agent Console
    /// (RFC 13 §6). The runner surface this verbatim into the `Diff`.
    pub narrative: String,
    /// Research refs cited in the narrative. Empty in Phase 1.
    pub research_refs: Vec<Uuid>,
}

#[derive(Clone, Debug)]
pub struct WorkspaceFile {
    pub path: String,
    pub lines: Vec<String>,
}

/// Run the Coding Engine for a single step. Returns `CodingOutcome` so
/// the caller can either persist the `Diff` (`Emitted`) or surface the
/// rejection (`Rejected`) to the HUD without a separate error channel.
pub fn run(input: &CodingInput) -> anyhow::Result<CodingOutcome> {
    let started = Instant::now();

    // RFC 12 §7 — anti-inicio defectuoso gate.
    if input.plan.confidence < PLAN_CONFIDENCE_THRESHOLD || !input.plan.blocked.is_empty() {
        return Ok(CodingOutcome::Rejected {
            reason: CodingRejection::PlanBlocked,
        });
    }

    // RFC 13 §2 — read-only steps cannot be dispatched to a coder.
    if input.step.read_only {
        return Ok(CodingOutcome::Rejected {
            reason: CodingRejection::ReadOnlyStep,
        });
    }

    // RFC 13 §3 — every approved skill must appear in Plan.skills_used.
    let skill_ok = input.approved_skills.iter().all(|s| {
        input
            .plan
            .skills_used
            .iter()
            .any(|p| p.skill_id == s.skill_id)
    });
    if !skill_ok {
        return Ok(CodingOutcome::Rejected {
            reason: CodingRejection::SkillNotApproved,
        });
    }

    let files = produce_files(input);
    for f in &files {
        if !f.hunks_are_disjoint_and_sorted() {
            return Ok(CodingOutcome::Rejected {
                reason: CodingRejection::OverlappingHunks,
            });
        }
    }

    let elapsed_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);

    let diff = Diff {
        diff_id: Uuid::new_v4(),
        plan_id: input.plan.plan_id,
        mission_id: input.plan.mission_id,
        step_id: input.step.id.clone(),
        agent_id: input.agent_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        files,
        narrative: input.narrative.clone(),
        research_refs: input.research_refs.clone(),
        risk_decision: if matches!(input.plan.impact, crate::planning::types::Impact::Breaking) {
            Some(format!(
                "impact=breaking; risk={:.2}; step={}",
                input.plan.risk, input.step.id
            ))
        } else {
            None
        },
        model_id: "heuristic-v0".into(),
        elapsed_ms,
    };

    Ok(CodingOutcome::Emitted {
        diff: Box::new(diff),
    })
}

/// Heuristic producer. Maps the `Step::action` to a deterministic
/// `FileEdit` against the workspace snapshot.
///
/// The Phase 1 implementation models the three actions the Planner emits
/// for `code` mode (Create / Modify / Debug) plus Delete for Strangler's
/// "legacy module removal" milestone. Research / Clarify / Context are
/// read-only and never reach this branch (the runner short-circuits
/// above).
fn produce_files(input: &CodingInput) -> Vec<FileEdit> {
    match input.step.action {
        StepAction::Create => produce_create(input),
        StepAction::Modify | StepAction::Debug => produce_modify(input),
        StepAction::Refactor => produce_modify(input),
        StepAction::Test => produce_test(input),
        _ => Vec::new(),
    }
}

fn produce_create(input: &CodingInput) -> Vec<FileEdit> {
    // RFC 13 §3 — new files require doc. The heuristic emits a stub file
    // whose first lines are the doc block; the Validation Engine (RFC 14)
    // later asserts the module-level doc is not empty.
    let path = guess_target_path(input);
    let module_doc = format!(
        "//! OpenCode OS — {step_label}.\n//!\n//! Generated by the Coding Engine for step `{step_id}` of plan `{plan_id}`.\n//! See RFC 13 §5 (auto-documentation).",
        step_label = input.step.statement,
        step_id = input.step.id,
        plan_id = input.plan.plan_id,
    );
    let lines: Vec<String> = module_doc.lines().map(String::from).collect();
    vec![FileEdit {
        path,
        hunks: vec![Hunk {
            old_start: 0,
            old_end: 0,
            new_lines: lines,
            rationale: "Append module-level doc for new file (RFC 13 §5).".to_string(),
        }],
        is_new_file: true,
        is_delete: false,
    }]
}

fn produce_modify(input: &CodingInput) -> Vec<FileEdit> {
    // Append a single-line TODO marker inline — the heuristic proves the
    // round-trip `Hunk` + snapshot apply works end-to-end without
    // relying on an LLM. Phase 2 will replace this with the model-driven
    // edit producer.
    let target = input.workspace_files.first();
    let path = target
        .map(|wf| wf.path.clone())
        .unwrap_or_else(|| guess_target_path(input));
    let old_end = target.map(|wf| wf.lines.len() as u32).unwrap_or(0);
    vec![FileEdit {
        path,
        hunks: vec![Hunk {
            old_start: old_end,
            old_end,
            new_lines: vec![format!(
                "// todo({}): {}",
                input.step.id, input.step.statement
            )],
            rationale: "Append sentinel edit so the Reviewer + Validation round-trip exercise a non-empty multi-line hunk.".into(),
        }],
        is_new_file: false,
        is_delete: false,
    }]
}

fn produce_test(input: &CodingInput) -> Vec<FileEdit> {
    // TDD asks for the failing test first (RFC 12 §6). The heuristic
    // emits a `*_heuristic_v0.rs` test file whose marked `#[test]`
    // forces a Red state until the impl step lands.
    let path = format!("tests/{}_heuristic_v0.rs", sanitize(input.step.id.as_str()));
    let lines: Vec<String> = vec![format!(
        "#[test] fn {}_stub() {{ assert!(false, \"red until impl lands\"); }}",
        sanitize(input.step.id.as_str())
    )];
    vec![FileEdit {
        path,
        hunks: vec![Hunk {
            old_start: 0,
            old_end: 0,
            new_lines: lines,
            rationale: "Failing test stub for TDD first step (RFC 12 §6).".into(),
        }],
        is_new_file: true,
        is_delete: false,
    }]
}

fn guess_target_path(input: &CodingInput) -> String {
    // Heuristic: if the plan recognised keys (from the verdict) name a
    // candidate file, use the first; otherwise default to a stable
    // `src/generated/<step_id>.rs`.
    format!("src/generated/{}.rs", sanitize(input.step.id.as_str()))
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
