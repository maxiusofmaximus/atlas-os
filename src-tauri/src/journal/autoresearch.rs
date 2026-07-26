// OpenCode OS — Autoresearch loop kernel (RFC 28 �A, derived from
//   karpathy/autoresearch program.md, MIT, Copyright (c) 2025-2026
//   Andrej Karpathy).
//
// Greedy hill-climbing on git commits, scoped to the four ecstaticpirate
// preconditions: limited scope + deterministic metric + time-box +
// git checkpoint. The supervisor owns the verdict (keep vs reset --hard)
// ─ NOT the LLM ─ by comparing the numeric metric emitted by the
// `metric_command`. The LLM only edits the file (analogous to Karpathy's
// `train.py`) and emits the diff hunk; the supervisor commits, runs the
// metric, and decides.
//
// This module is pure: it owns no git subprocess, no training spawn, no
// disk. The host (Tauri main thread / headless CLI / supervisor tick)
// drives the state machine by feeding `AutoresearchEvent`s and dispatching
// the returned `AutoresearchAction`s. The host owns the wall-clock and
// is responsible for aborting on timebox expiry (emits `Timeout`).
//
// Schema persistence is in M13 (see `journal/schema.rs`). The rows
// produced here are persisted by the journal layer; the supervisor pipes
// `RunSnapshot` / `CandidateRow` into the matching INSERT on every
// transition.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Per RFC 28 �A, the supervisor caps a single run at 200 candidates to
/// avoid inflating git history beyond recovery. The host may pass a
/// smaller `max_steps` per mission; this is the absolute ceiling.
pub const HARD_MAX_STEPS: u32 = 200;

/// RFC 28 �A — final disposition of an autoresearch run. Persisted in
/// `autoresearch_runs.outcome`. The transition into any non-`Running`
/// outcome terminates the loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Loop in progress; candidates being tried.
    Running,
    /// Run finished before `max_steps` with a strictly-improving last
    /// candidate AND the LLM emitted `Done`.
    Improved,
    /// Three consecutive candidates that were reset (`kept=false`) ─
    /// supervisor aborts to avoid wasting budget.
    Plateau,
    /// Wall-clock budget exhausted OR `max_steps` reached before
    /// convergence.
    Timeout,
    /// Human / external signal cancelled the run.
    Aborted,
}

impl Outcome {
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Improved => "improved",
            Self::Plateau => "plateau",
            Self::Timeout => "timeout",
            Self::Aborted => "aborted",
        }
    }
}

/// RFC 28 �A — one experiment trial. Maps 1:1 to a row in
/// `autoresearch_candidates`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: Uuid,
    pub step: u32,
    /// Git SHA AFTER the experimental commit (7+ char short hash). The
    /// supervisor knows the SHA BEFORE the experiment because it's the
    /// previous candidate's `git_sha` (or `git_sha_start` for step 0).
    pub git_sha: String,
    pub diff_hunk: String,
    /// Metric recorded on the baseline state BEFORE this candidate ran.
    /// Lets the audit trail regress against the previous step too.
    pub metric_baseline_at_step: f64,
    /// Metric recorded AFTER this candidate was committed and the
    /// metric_command ran. Lower is better (mirrors Karpathy `val_bpb`).
    pub metric_after: f64,
    pub kept: bool,
    pub rationale: String,
}

impl Candidate {
    /// Per RFC 28 �A a candidate is kept iff its `metric_after` is
    /// STRICTLY lower than `metric_baseline_at_step`. Ties trigger a
    /// reset (Karpathy program.md step 9: "If val_bpb is equal or
    /// worse, you git reset back").
    pub fn is_improvement(&self) -> bool {
        self.metric_after < self.metric_baseline_at_step
    }
}

/// RFC 28 �A — run-level snapshot. Maps 1:1 to a row in
/// `autoresearch_runs`. The host constructs one at `Start` and patches
/// it on every `CandidateRecorded` / `Outcome` transition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunSnapshot {
    pub id: Uuid,
    pub mission_id: Uuid,
    pub baseline_metric: f64,
    /// Best (lowest) metric observed across all candidates so far.
    /// `None` until the first candidate is recorded.
    pub best_metric: Option<f64>,
    pub git_sha_start: String,
    pub git_sha_end: Option<String>,
    pub metric_command: String,
    pub max_steps: u32,
    pub timebox_seconds: u32,
    pub step_count: u32,
    pub outcome: Outcome,
    pub ts_started: i64,
    pub ts_ended: Option<i64>,
}

impl RunSnapshot {
    /// RFC 28 �A — `max_steps` never exceeds `HARD_MAX_STEPS`, even
    /// when the caller asks for more. The host should display the
    /// clamped value so the user isn't surprised when a 1000-step run
    /// stops at 200.
    pub fn clamp_max_steps(requested: u32) -> u32 {
        requested.min(HARD_MAX_STEPS)
    }
}

/// Pure inputs the supervisor reacts to. The host maps real BusEvents ─
/// git commits finishing, metric_command stdout parsing, timebox timers
/// firing ─ into these.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutoresearchEvent {
    /// Kick off a new run. Carries the initial git SHA and the baseline
    /// metric (the host runs `metric_command` once BEFORE the loop).
    Start {
        run_id: Uuid,
        mission_id: Uuid,
        git_sha_start: String,
        baseline_metric: f64,
        metric_command: String,
        max_steps: u32,
        timebox_seconds: u32,
    },
    /// LLM emitted a diff hunk and the host committed it. Carries the
    /// new git SHA and the parsed metric AFTER the run. The supervisor
    /// decides `kept` by comparing `metric_after` to
    /// `metric_baseline_at_step`.
    CandidateRecorded {
        run_id: Uuid,
        git_sha: String,
        diff_hunk: String,
        metric_baseline_at_step: f64,
        metric_after: f64,
        rationale: String,
    },
    /// Wall-clock budget exhausted OR `max_steps` reached.
    Timeout { run_id: Uuid },
    /// Human pressed Stop (HUD) or sent `session/cancel`.
    Abort { run_id: Uuid },
    /// LLM declared the search converged (no more ideas).
    Done { run_id: Uuid },
}

/// Pure outputs the supervisor asks the host to dispatch. The host
/// performs the actual git operations (commit / reset --hard) and the
/// Journal INSERTs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutoresearchAction {
    /// Persist a new `autoresearch_runs` row.
    PersistRun(RunSnapshot),
    /// Persist a new `autoresearch_candidates` row. `Candidate` carries
    /// the `kept` verdict already computed.
    PersistCandidate { run_id: Uuid, candidate: Candidate },
    /// `git reset --hard <sha>` ─ the experimental commit is discarded
    /// (metric did not improve, or it tied). The host MUST run git with
    /// the same working-dir as the experiment.
    ResetHard { run_id: Uuid, sha: String },
    /// Tell the LLM "next idea please". The host forwards this to the
    /// Coding Engine as an InjectPrompt.
    SolicitNextCandidate { run_id: Uuid },
    /// Final transition. The host updates the `autoresearch_runs` row
    /// with `git_sha_end`, `best_metric`, `outcome`, `ts_ended`.
    FinalizeRun {
        run_id: Uuid,
        outcome: Outcome,
        git_sha_end: String,
        best_metric: Option<f64>,
    },
    /// Forward telemetry to the HUD `autoresearch:<run_id>` channel.
    /// The HUD `AutoresearchCard.svelte` renders baseline, best metric,
    /// step count, mini-sparkline.
    PublishTelemetry { run_id: Uuid, snapshot: RunSnapshot },
}

/// RFC 28 �A — supervisor-owned loop state. Pure: `tick(state, event)`
/// returns a new state + a vec of `AutoresearchAction`s for the host.
/// The host owns the canonical instance and the wall-clock; the journal
/// persists rows on every PersistRun / PersistCandidate / FinalizeRun
/// output.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutoresearchState {
    pub snapshot: RunSnapshot,
    /// Most recent `kept=true` candidate. The host resets to this SHA
    /// when a candidate is discarded.
    pub last_kept_sha: Option<String>,
    /// Consecutive `kept=false` count since the last kept candidate.
    /// Three triggers `Outcome::Plateau`.
    pub consecutive_resets: u32,
}

impl AutoresearchState {
    /// Build a fresh state from a `Start` event. The host invokes this
    /// when the supervisor transitions into MissionPhase::Autoresearch.
    pub fn from_start(evt: &AutoresearchEvent) -> Self {
        let AutoresearchEvent::Start {
            run_id,
            mission_id,
            git_sha_start,
            baseline_metric,
            metric_command,
            max_steps,
            timebox_seconds,
        } = evt
        else {
            panic!("AutoresearchState::from_start expects a Start event");
        };
        let snapshot = RunSnapshot {
            id: *run_id,
            mission_id: *mission_id,
            baseline_metric: *baseline_metric,
            best_metric: None,
            git_sha_start: git_sha_start.clone(),
            git_sha_end: None,
            metric_command: metric_command.clone(),
            max_steps: Self::clamp(*max_steps),
            timebox_seconds: *timebox_seconds,
            step_count: 0,
            outcome: Outcome::Running,
            ts_started: chrono::Utc::now().timestamp(),
            ts_ended: None,
        };
        Self {
            snapshot,
            last_kept_sha: None,
            consecutive_resets: 0,
        }
    }

    fn clamp(max_steps: u32) -> u32 {
        RunSnapshot::clamp_max_steps(max_steps)
    }

    /// True when `step_count` reached `max_steps` (or the hard ceiling).
    pub fn step_budget_exhausted(&self) -> bool {
        self.snapshot.step_count >= self.snapshot.max_steps
    }

    /// True when three consecutive candidates were reset. Per RFC 28
    /// �A "Doom-loop autoresearch" risk + Karpathy "rewind sparingly".
    pub fn plateau_detected(&self) -> bool {
        self.consecutive_resets >= 3
    }
}

/// Pure output of `tick`. Mirrors `supervisor::runner::TickOutput`.
#[derive(Clone, Debug, PartialEq)]
pub struct AutoresearchOutput {
    pub state: AutoresearchState,
    pub actions: Vec<AutoresearchAction>,
}

impl AutoresearchOutput {
    fn pass(state: AutoresearchState) -> Self {
        Self {
            state,
            actions: Vec::new(),
        }
    }

    fn emit(state: AutoresearchState, actions: Vec<AutoresearchAction>) -> Self {
        Self { state, actions }
    }
}

/// RFC 28 �A — the pure state machine. Host calls
/// `tick(&mut state, event)`; the host then dispatches the returned
/// actions in order. No-ops (event run_id mismatch, terminal state)
/// produce an empty action list and copy the state forward unchanged.
pub fn tick(state: &AutoresearchState, event: &AutoresearchEvent) -> AutoresearchOutput {
    // Terminal states are sticky; ignore everything but `Abort`.
    if state.snapshot.outcome != Outcome::Running
        && !matches!(event, AutoresearchEvent::Abort { .. })
    {
        return AutoresearchOutput::pass(state.clone());
    }

    match event {
        AutoresearchEvent::Start { .. } => {
            // Re-Start on a running state is a host bug ─ ignore.
            AutoresearchOutput::pass(state.clone())
        }
        AutoresearchEvent::CandidateRecorded {
            run_id,
            git_sha,
            diff_hunk,
            metric_baseline_at_step,
            metric_after,
            rationale,
        } => {
            if *run_id != state.snapshot.id {
                return AutoresearchOutput::pass(state.clone());
            }
            let kept = *metric_after < *metric_baseline_at_step;
            let step = state.snapshot.step_count + 1;
            let candidate = Candidate {
                id: Uuid::new_v4(),
                step,
                git_sha: git_sha.clone(),
                diff_hunk: diff_hunk.clone(),
                metric_baseline_at_step: *metric_baseline_at_step,
                metric_after: *metric_after,
                kept,
                rationale: rationale.clone(),
            };
            let mut new_state = state.clone();
            new_state.snapshot.step_count = step;
            let prev_best = new_state
                .snapshot
                .best_metric
                .unwrap_or(state.snapshot.baseline_metric);
            if kept {
                new_state.snapshot.best_metric = Some(prev_best.min(*metric_after));
                new_state.last_kept_sha = Some(git_sha.clone());
                new_state.consecutive_resets = 0;
            } else {
                new_state.consecutive_resets += 1;
            }

            let mut actions = vec![AutoresearchAction::PersistCandidate {
                run_id: *run_id,
                candidate: candidate.clone(),
            }];
            if kept {
                // Keep the commit; ask the LLM for the next idea.
                actions.push(AutoresearchAction::SolicitNextCandidate { run_id: *run_id });
            } else {
                // Discard the commit; reset --hard to last kept SHA (or
                // the run start SHA) before asking for the next idea.
                let reset_target = new_state
                    .last_kept_sha
                    .clone()
                    .unwrap_or_else(|| state.snapshot.git_sha_start.clone());
                actions.push(AutoresearchAction::ResetHard {
                    run_id: *run_id,
                    sha: reset_target,
                });
                // Plateau detection: 3 consecutive resets abort.
                if new_state.plateau_detected() {
                    actions.push(finalize(
                        *run_id,
                        Outcome::Plateau,
                        new_state.last_kept_sha.clone(),
                        new_state.snapshot.best_metric,
                    ));
                    new_state.snapshot.outcome = Outcome::Plateau;
                    new_state.snapshot.ts_ended = Some(chrono::Utc::now().timestamp());
                    new_state.snapshot.git_sha_end = new_state.last_kept_sha.clone();
                    actions.push(AutoresearchAction::PublishTelemetry {
                        run_id: *run_id,
                        snapshot: new_state.snapshot.clone(),
                    });
                    return AutoresearchOutput::emit(new_state, actions);
                }
                actions.push(AutoresearchAction::SolicitNextCandidate { run_id: *run_id });
            }
            // Step budget check.
            if new_state.step_budget_exhausted() {
                let outcome = Outcome::Timeout;
                actions.push(finalize(
                    *run_id,
                    outcome,
                    new_state.last_kept_sha.clone(),
                    new_state.snapshot.best_metric,
                ));
                new_state.snapshot.outcome = outcome;
                new_state.snapshot.ts_ended = Some(chrono::Utc::now().timestamp());
                new_state.snapshot.git_sha_end = new_state.last_kept_sha.clone();
                actions.push(AutoresearchAction::PublishTelemetry {
                    run_id: *run_id,
                    snapshot: new_state.snapshot.clone(),
                });
            } else {
                actions.push(AutoresearchAction::PublishTelemetry {
                    run_id: *run_id,
                    snapshot: new_state.snapshot.clone(),
                });
            }
            AutoresearchOutput::emit(new_state, actions)
        }
        AutoresearchEvent::Timeout { run_id } => {
            if *run_id != state.snapshot.id {
                return AutoresearchOutput::pass(state.clone());
            }
            let mut new_state = state.clone();
            new_state.snapshot.outcome = Outcome::Timeout;
            new_state.snapshot.ts_ended = Some(chrono::Utc::now().timestamp());
            new_state.snapshot.git_sha_end = new_state.last_kept_sha.clone();
            let mut actions = vec![finalize(
                *run_id,
                Outcome::Timeout,
                new_state.last_kept_sha.clone(),
                new_state.snapshot.best_metric,
            )];
            actions.push(AutoresearchAction::PublishTelemetry {
                run_id: *run_id,
                snapshot: new_state.snapshot.clone(),
            });
            AutoresearchOutput::emit(new_state, actions)
        }
        AutoresearchEvent::Abort { run_id } => {
            if *run_id != state.snapshot.id {
                return AutoresearchOutput::pass(state.clone());
            }
            let mut new_state = state.clone();
            new_state.snapshot.outcome = Outcome::Aborted;
            new_state.snapshot.ts_ended = Some(chrono::Utc::now().timestamp());
            new_state.snapshot.git_sha_end = new_state.last_kept_sha.clone();
            let mut actions = vec![finalize(
                *run_id,
                Outcome::Aborted,
                new_state.last_kept_sha.clone(),
                new_state.snapshot.best_metric,
            )];
            actions.push(AutoresearchAction::PublishTelemetry {
                run_id: *run_id,
                snapshot: new_state.snapshot.clone(),
            });
            AutoresearchOutput::emit(new_state, actions)
        }
        AutoresearchEvent::Done { run_id } => {
            if *run_id != state.snapshot.id {
                return AutoresearchOutput::pass(state.clone());
            }
            let mut new_state = state.clone();
            new_state.snapshot.outcome = Outcome::Improved;
            new_state.snapshot.ts_ended = Some(chrono::Utc::now().timestamp());
            new_state.snapshot.git_sha_end = new_state.last_kept_sha.clone();
            let mut actions = vec![finalize(
                *run_id,
                Outcome::Improved,
                new_state.last_kept_sha.clone(),
                new_state.snapshot.best_metric,
            )];
            actions.push(AutoresearchAction::PublishTelemetry {
                run_id: *run_id,
                snapshot: new_state.snapshot.clone(),
            });
            AutoresearchOutput::emit(new_state, actions)
        }
    }
}

fn finalize(
    run_id: Uuid,
    outcome: Outcome,
    git_sha_end: Option<String>,
    best_metric: Option<f64>,
) -> AutoresearchAction {
    AutoresearchAction::FinalizeRun {
        run_id,
        outcome,
        git_sha_end: git_sha_end.unwrap_or_default(),
        best_metric,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start_event(max_steps: u32) -> AutoresearchEvent {
        AutoresearchEvent::Start {
            run_id: Uuid::new_v4(),
            mission_id: Uuid::new_v4(),
            git_sha_start: "abc1234".into(),
            baseline_metric: 1.0,
            metric_command: "rg -c 'error' src".into(),
            max_steps,
            timebox_seconds: 600,
        }
    }

    fn candidate_event(run_id: Uuid, baseline: f64, after: f64, sha: &str) -> AutoresearchEvent {
        AutoresearchEvent::CandidateRecorded {
            run_id,
            git_sha: sha.into(),
            diff_hunk: "+impl Foo {}".into(),
            metric_baseline_at_step: baseline,
            metric_after: after,
            rationale: "bump lr".into(),
        }
    }

    #[test]
    fn start_creates_running_state() {
        let evt = start_event(50);
        let state = AutoresearchState::from_start(&evt);
        assert_eq!(state.snapshot.outcome, Outcome::Running);
        assert_eq!(state.snapshot.step_count, 0);
        assert_eq!(state.snapshot.best_metric, None);
        assert_eq!(state.consecutive_resets, 0);
        assert_eq!(state.last_kept_sha, None);
    }

    #[test]
    fn clamps_max_steps_to_hard_ceiling() {
        let evt = start_event(10_000);
        let state = AutoresearchState::from_start(&evt);
        assert_eq!(state.snapshot.max_steps, HARD_MAX_STEPS);
    }

    #[test]
    fn happy_path_two_kept_one_reset_then_done() {
        let start = start_event(50);
        let run_id = match &start {
            AutoresearchEvent::Start { run_id, .. } => *run_id,
            _ => unreachable!(),
        };
        let mut state = AutoresearchState::from_start(&start);
        assert!(!state.plateau_detected());

        // step 1: keep (after < baseline)
        let out = tick(&state, &candidate_event(run_id, 1.0, 0.95, "sha0001"));
        assert_eq!(out.actions.len(), 3); // PersistCandidate + SolicitNext + PublishTelemetry
        assert!(matches!(
            out.actions[0],
            AutoresearchAction::PersistCandidate { ref candidate, .. } if candidate.kept
        ));
        assert!(matches!(
            out.actions[1],
            AutoresearchAction::SolicitNextCandidate { .. }
        ));
        state = out.state;
        assert_eq!(state.snapshot.step_count, 1);
        assert_eq!(state.snapshot.best_metric, Some(0.95));
        assert_eq!(state.last_kept_sha.as_deref(), Some("sha0001"));
        assert_eq!(state.consecutive_resets, 0);

        // step 2: keep again (after < prev_baseline, which is new best)
        let out = tick(&state, &candidate_event(run_id, 0.95, 0.91, "sha0002"));
        assert_eq!(out.actions.len(), 3);
        assert!(matches!(
            out.actions[0],
            AutoresearchAction::PersistCandidate { ref candidate, .. } if candidate.kept
        ));
        state = out.state;
        assert_eq!(state.snapshot.best_metric, Some(0.91));
        assert_eq!(state.last_kept_sha.as_deref(), Some("sha0002"));
        assert_eq!(state.consecutive_resets, 0);

        // step 3: discard (after >= baseline)
        let out = tick(&state, &candidate_event(run_id, 0.91, 0.92, "sha0003"));
        // PersistCandidate + ResetHard + SolicitNext + PublishTelemetry = 4
        assert_eq!(out.actions.len(), 4);
        assert!(matches!(
            out.actions[0],
            AutoresearchAction::PersistCandidate { ref candidate, .. } if !candidate.kept
        ));
        assert!(matches!(
            out.actions[1],
            AutoresearchAction::ResetHard { ref sha, .. } if sha == "sha0002"
        ));
        assert!(matches!(
            out.actions[2],
            AutoresearchAction::SolicitNextCandidate { .. }
        ));
        state = out.state;
        assert_eq!(state.consecutive_resets, 1);
        // best stays at 0.91 (the discarded one didn't improve)
        assert_eq!(state.snapshot.best_metric, Some(0.91));
        assert_eq!(state.last_kept_sha.as_deref(), Some("sha0002"));

        // LLM says "I'm out of ideas" -> Done
        let out = tick(&state, &AutoresearchEvent::Done { run_id });
        // FinalizeRun(improved) + PublishTelemetry = 2
        assert_eq!(out.actions.len(), 2);
        assert!(matches!(
            out.actions[0],
            AutoresearchAction::FinalizeRun {
                outcome: Outcome::Improved,
                ..
            }
        ));
        assert_eq!(out.state.snapshot.outcome, Outcome::Improved);
        assert!(out.state.snapshot.ts_ended.is_some());
    }

    #[test]
    fn three_consecutive_resets_trigger_plateau() {
        let start = start_event(50);
        let run_id = match &start {
            AutoresearchEvent::Start { run_id, .. } => *run_id,
            _ => unreachable!(),
        };
        let mut state = AutoresearchState::from_start(&start);

        // step 1: keep to set a last_kept_sha
        let out = tick(&state, &candidate_event(run_id, 1.0, 0.95, "sha1"));
        state = out.state;
        assert_eq!(state.consecutive_resets, 0);

        // 3 discards in a row
        let mut last_actions: Vec<AutoresearchAction> = out.actions;
        for i in 1..=3u32 {
            let sha = format!("bad{i}");
            // baseline at each step stays at 0.95 since we reset back
            let out = tick(&state, &candidate_event(run_id, 0.95, 0.96, &sha));
            assert_eq!(
                out.state.consecutive_resets, i,
                "consecutive_resets should be {i} after {i} discards"
            );
            state = out.state;
            last_actions = out.actions;
        }
        assert_eq!(state.snapshot.outcome, Outcome::Plateau);
        assert!(state.snapshot.ts_ended.is_some());
        // Final tick should contain FinalizeRun(Plateau)
        assert!(last_actions.iter().any(|a| matches!(
            a,
            AutoresearchAction::FinalizeRun {
                outcome: Outcome::Plateau,
                ..
            }
        )));
    }

    #[test]
    fn timebox_timeout_aborts() {
        let start = start_event(50);
        let run_id = match &start {
            AutoresearchEvent::Start { run_id, .. } => *run_id,
            _ => unreachable!(),
        };
        let mut state = AutoresearchState::from_start(&start);
        // one kept
        let out = tick(&state, &candidate_event(run_id, 1.0, 0.95, "sha1"));
        state = out.state;
        // Time runs out before next candidate
        let out = tick(&state, &AutoresearchEvent::Timeout { run_id });
        assert_eq!(out.actions.len(), 2);
        assert!(matches!(
            out.actions[0],
            AutoresearchAction::FinalizeRun {
                outcome: Outcome::Timeout,
                ..
            }
        ));
        assert_eq!(out.state.snapshot.outcome, Outcome::Timeout);
        assert_eq!(out.state.snapshot.git_sha_end.as_deref(), Some("sha1"));
    }

    #[test]
    fn step_budget_exhaustion_triggers_timeout_finalize() {
        let start = start_event(2);
        let run_id = match &start {
            AutoresearchEvent::Start { run_id, .. } => *run_id,
            _ => unreachable!(),
        };
        let mut state = AutoresearchState::from_start(&start);
        // step 1: keep
        let out = tick(&state, &candidate_event(run_id, 1.0, 0.95, "sha1"));
        assert!(out.state.snapshot.outcome == Outcome::Running);
        state = out.state;
        // step 2: budget exhausted (max_steps=2)
        let out = tick(&state, &candidate_event(run_id, 0.95, 0.94, "sha2"));
        assert_eq!(out.state.snapshot.outcome, Outcome::Timeout);
        assert!(out.actions.iter().any(|a| matches!(
            a,
            AutoresearchAction::FinalizeRun {
                outcome: Outcome::Timeout,
                ..
            }
        )));
    }

    #[test]
    fn abort_terminates_even_mid_run() {
        let start = start_event(50);
        let run_id = match &start {
            AutoresearchEvent::Start { run_id, .. } => *run_id,
            _ => unreachable!(),
        };
        let state = AutoresearchState::from_start(&start);
        let _ = tick(&state, &candidate_event(run_id, 1.0, 0.95, "sha1"));
        let out = tick(&state, &AutoresearchEvent::Abort { run_id });
        assert_eq!(out.state.snapshot.outcome, Outcome::Aborted);
        assert!(out.actions.iter().any(|a| matches!(
            a,
            AutoresearchAction::FinalizeRun {
                outcome: Outcome::Aborted,
                ..
            }
        )));
    }

    #[test]
    fn terminal_state_ignores_late_candidates() {
        let start = start_event(50);
        let run_id = match &start {
            AutoresearchEvent::Start { run_id, .. } => *run_id,
            _ => unreachable!(),
        };
        let mut state = AutoresearchState::from_start(&start);
        let _ = tick(&state, &candidate_event(run_id, 1.0, 0.95, "sha1"));
        // abort
        let out = tick(&state, &AutoresearchEvent::Abort { run_id });
        state = out.state;
        // late candidate ignored
        let out = tick(&state, &candidate_event(run_id, 0.95, 0.90, "sha2_late"));
        assert!(out.actions.is_empty());
        assert_eq!(out.state.snapshot.outcome, Outcome::Aborted);
    }

    #[test]
    fn mismatched_run_id_is_ignored() {
        let start = start_event(50);
        let state = AutoresearchState::from_start(&start);
        let other_run = Uuid::new_v4();
        let out = tick(&state, &candidate_event(other_run, 1.0, 0.5, "shaX"));
        assert!(out.actions.is_empty());
        assert_eq!(out.state.snapshot.step_count, 0);
    }

    #[test]
    fn tie_does_not_keep() {
        // Karpathy program.md step 9: "If val_bpb is equal or worse, you
        // git reset back". Strict-less-than only.
        let start = start_event(50);
        let run_id = match &start {
            AutoresearchEvent::Start { run_id, .. } => *run_id,
            _ => unreachable!(),
        };
        let state = AutoresearchState::from_start(&start);
        let out = tick(&state, &candidate_event(run_id, 1.0, 1.0, "sha1"));
        assert!(matches!(
            out.actions[0],
            AutoresearchAction::PersistCandidate { ref candidate, .. } if !candidate.kept
        ));
        assert!(matches!(
            out.actions[1],
            AutoresearchAction::ResetHard { .. }
        ));
    }

    #[test]
    fn publish_telemetry_emitted_on_every_transition() {
        let start = start_event(50);
        let run_id = match &start {
            AutoresearchEvent::Start { run_id, .. } => *run_id,
            _ => unreachable!(),
        };
        let state = AutoresearchState::from_start(&start);
        let out = tick(&state, &candidate_event(run_id, 1.0, 0.95, "sha1"));
        assert!(out
            .actions
            .iter()
            .any(|a| matches!(a, AutoresearchAction::PublishTelemetry { .. })));

        let out = tick(&state, &AutoresearchEvent::Timeout { run_id });
        assert!(out
            .actions
            .iter()
            .any(|a| matches!(a, AutoresearchAction::PublishTelemetry { .. })));
    }
}
