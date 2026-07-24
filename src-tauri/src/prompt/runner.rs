// OpenCode OS — Pipeline runner for RFC 23 §2.
//
// The runner is intentionally a free function over `&mut VerdictBuilder`,
// not a trait object. Each step is a pure transformation: mutate the
// builder in place; the persistence side-effect (Journal write) happens
// only at the end of step 7 and at the end of step 9.
//
// Why not trait objects for the steps? The 9 steps are a closed catalogue
// pinned by the RFC; we want exhaustive `match` warnings if any new step
// is added. Plus each step has a wildly different signature (capture takes
// `raw_prompt: String`, detect takes the parsed intent, etc.) so a uniform
// trait would force us to thread unused parameters through every step.
//
// The LLM-driven slots are traits (`IntentParser`, `ClarificationGenerator`,
// `MissionConsolidator`) implemented twice: `HeuristicXxx` (Phase 1) and,
// later, `LlmXxx` (after RFC 04).

use std::time::Instant;
use uuid::Uuid;

use crate::prompt::steps::{capture, clarify, consolidate, detect, parse, similar};
use crate::prompt::types::{MissionConsolidated, PublicUnderstandingVerdict};

/// Knob flags for the runner (mirror of RFC 23 §7.1 `/refine` flags).
/// Default = no shortcuts (i.e. behave like raw `mission new`).
#[derive(Clone, Debug)]
pub struct PipelineOptions {
    /// `--force` — skip the confidence gate and lock the mission anyway.
    pub force: bool,
    /// `--dry-run` — produce a verdict but never persist or publish.
    pub dry_run: bool,
    /// `--max-questions K` — clamp K=3 clarification questions down to K
    /// (default 3 per RFC 23 §7.1).
    pub max_questions: u32,
    /// `--perspective p1,p2,...` — override the default STORM perspective set
    /// (default: developer, qa, ops per RFC 23 §7.1).
    pub perspectives: Vec<String>,
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self {
            force: false,
            dry_run: false,
            max_questions: 3,
            perspectives: vec!["developer".into(), "qa".into(), "ops".into()],
        }
    }
}

impl PipelineOptions {
    /// Same as `Default` but with `force = true` — used by the CLI `--force`
    /// flag and by tests that want to skip clarification/research gates.
    pub fn default_force() -> Self {
        Self {
            force: true,
            ..Self::default()
        }
    }
}

/// In-flight builder used by the pipeline. The intermediate state lives in
/// `PipelineState` (parsed intent, gaps, etc.) — it disappears once the
/// builder is consumed by `step_7_consolidate` into a verdict.
#[allow(dead_code)]
pub(crate) struct PipelineState {
    pub session_id: Uuid,
    pub raw_prompt: String,
    pub started: Instant,
    pub parsed: crate::prompt::steps::parse::ParsedIntent,
    pub gaps: Vec<crate::prompt::types::Gap>,
    pub similar_missions: Vec<crate::prompt::types::SimilarMission>,
    pub clarification_questions: Vec<crate::prompt::types::ClarificationQuestion>,
}

/// Run the 9-step pipeline end to end. `HeuristicPipeline::run` is the
/// single entry point for IPC `mission_new` (Fase 1.8) and `/refine` (Fase 1.10).
/// It returns a `(verdict, Option<MissionConsolidated>)` — the consolidation
/// is `Some` only when step 9 succeeds (i.e. confidence >= threshold, or
/// `options.force == true`).
pub fn run(
    raw_prompt: &str,
    options: &PipelineOptions,
) -> anyhow::Result<(PublicUnderstandingVerdict, Option<MissionConsolidated>)> {
    let session_id = Uuid::new_v4();
    let started = Instant::now();

    // Step 1 — Capture.
    let parsed = parse::run(&capture::normalise(raw_prompt));
    // Step 3 — Detect Ambiguities.
    let gaps = detect::run(&parsed, raw_prompt);
    // Step 4 — Similar Missions. Phase 1 always returns an empty list here;
    // the sqlite-vec lookup lives in `similar::lookup` (Fase 1.5) and is
    // invoked from `Journal`-backed wiring, not from the pure runner.
    let similar_missions = similar::lookup(&parsed);
    // Step 5 + 6 — Clarification K=3 + Auto-Resolve.
    let clarification_questions = clarify::generate(&parsed, &gaps, options)?;
    // Step 7 — Consolidated Mission + Verdict (Self-Refine heuristic: 2 iters
    // of inner critic re-write — for now both iterations are deterministic
    // rewrites of the intent statement).
    let (verdict, consolidated_opt) = consolidate::build(
        parsed.clone(),
        gaps,
        similar_missions,
        clarification_questions,
        session_id,
        raw_prompt,
        started,
        options,
    )?;

    Ok((verdict, consolidated_opt))
}
