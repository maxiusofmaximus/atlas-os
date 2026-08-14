// Atlas OS — Execution Supervisor runner (RFC 19 §6.1 state machine).
//
// The runner is a pure function: `tick(state, event) -> (state, actions)`.
// It owns no timers, no threads, no disk; the host (Tauri main thread
// or the headless CLI) supplies the heartbeat cadence, persists
// checkpoints, and dispatches the returned `SupervisorAction`s on the
// Kernel Bus.
//
// State transitions (RFC 19 §6.1):
//
//   idle      --MissionStarted-->              planning
//   planning  --PlanGenerated-->               executing
//   planning  --PlanConfidenceLow-->           planning   (TriggerReplan)
//   executing --CodingStarted/DiffEmitted-->   executing
//   executing --ValidationStarted-->           verifying
//   verifying --ValidationPassed-->            done
//   verifying --ValidationFailed-->            recovering (TriggerRepair)
//   verifying --CriticalValidation-->          planning   (TriggerReplan, §8)
//   recovering --RepairAttempted[Applied]-->   verifying  (TriggerRevalidation)
//   recovering --RepairAttempted[Proposed]-->  verifying  (TriggerRevalidation)
//   recovering --RepairAttempted[EscalPlanning]--> planning (§4 budget)
//   recovering --RepairAttempted[EscalHuman]--> halted   (EscalateHuman)
//   halted    --UserApproved-->                executing
//   halted    --UserSteered-->                 planning
//   halted    --UserHalted-->                  idle
//   done      --(implicit)-->                  idle
//
// Caps (RFC 19 §6 `caps`): hitting ANY cap triggers `HaltSession` +
// `on_budget_hit` recovery regardless of phase. Doom-loop: the detector
// is stashed OUTSIDE the runner (host-owned); the runner consumes the
// `ToolCall` event, feeds it to the detector, and reacts to the
// detector's own firing.

use std::time::Duration;

use uuid::Uuid;

use super::doom_loop::{DoomLoopConfig, DoomLoopDetector};
use super::types::*;

/// Phase-1 host-owned context. Keeps the runner pure by holding the
/// mutable bits the runner needs (clock + doom-loop detector). The
/// host calls `tick(&mut ctx, state, event)`.
pub struct TickContext {
    pub session_started_at: chrono::DateTime<chrono::Utc>,
    pub doom: DoomLoopDetector,
    pub doom_cfg: DoomLoopConfig,
    pub max_recovery_attempts: u32,
}

impl TickContext {
    pub fn new(_mode: ExecutionMode) -> Self {
        let doom_cfg = DoomLoopConfig::DEFAULT;
        Self {
            session_started_at: chrono::Utc::now(),
            doom: DoomLoopDetector::new(doom_cfg),
            doom_cfg,
            max_recovery_attempts: 2,
        }
    }

    fn elapsed_minutes(&self) -> f64 {
        let dur = chrono::Utc::now().signed_duration_since(self.session_started_at);
        // Phase 1 — `chrono::Duration::to_std` is fallible (negative);
        // we clamp to zero for the (impossible) backwards-clock case.
        dur.to_std().unwrap_or(Duration::ZERO).as_secs_f64() / 60.0
    }
}

/// Pure output of `tick`. Carries the new state + the actions the host
/// should dispatch (publish on the Bus, persist a checkpoint, etc).
#[derive(Clone, Debug)]
pub struct TickOutput {
    pub state: SupervisorState,
    pub actions: Vec<SupervisorAction>,
}

impl TickOutput {
    fn pass_through(state: SupervisorState) -> Self {
        Self {
            state,
            actions: vec![],
        }
    }

    fn with_actions(state: SupervisorState, actions: Vec<SupervisorAction>) -> Self {
        Self { state, actions }
    }
}

/// RFC 19 §6.1 entry point. Pure: takes the current state by value and
/// returns the next state plus zero or more actions.
pub fn tick(ctx: &mut TickContext, state: SupervisorState, event: SupervisorEvent) -> TickOutput {
    // Keep the elapsed-minutes tally fresh before evaluating anything.
    let mut next = state.clone();
    next.budget_tally.elapsed_minutes = ctx.elapsed_minutes();

    // ── Budget cap evaluation (RFC 19 §6 `caps`) ──
    // Tool calls also bump the iteration tally.
    if let SupervisorEvent::ToolCall { cost_usd, .. } = &event {
        next.budget_tally.iterations = next.budget_tally.iterations.saturating_add(1);
        next.budget_tally.cost_usd += *cost_usd;
    }
    if let SupervisorEvent::ContextCompacted = &event {
        next.budget_tally.context_compactions =
            next.budget_tally.context_compactions.saturating_add(1);
    }
    if let Some(cap) = cap_hit(&next) {
        return halt_on_budget(&next, cap);
    }

    // ── Doom-loop detector (RFC 19 §6) ──
    if let SupervisorEvent::ToolCall {
        tool, input_hash, ..
    } = &event
    {
        let key = DoomLoopDetector::build_key(tool, input_hash);
        let now_secs = (chrono::Utc::now()
            .signed_duration_since(ctx.session_started_at)
            .num_seconds()
            .max(0)) as u64;
        if let Some(fired_key) = ctx.doom.observe(&key, now_secs) {
            let actions = doom_loop_actions(&next, ctx, &fired_key);
            if ctx.doom.fire_count > ctx.max_recovery_attempts {
                // Recovery budget exhausted — hard halt.
                let mut actions = actions;
                actions.push(SupervisorAction::HaltSession {
                    reason: "doom-loop recovery exhausted (RFC 19 §6)".into(),
                    cap: None,
                });
                next.phase = MissionPhase::Halted;
                return TickOutput::with_actions(next, actions);
            }
            return TickOutput::with_actions(next, actions);
        }
    }

    // ── Phase transition (RFC 19 §6.1) ──
    match event {
        SupervisorEvent::MissionStarted { mission_id } => {
            if !state.phase.accepts_new_mission() {
                return TickOutput::pass_through(state);
            }
            next.mission_id = Some(mission_id);
            next.phase = MissionPhase::Planning;
            let checkpoint = snapshot_with(&next);
            next.last_checkpoint_id = Some(checkpoint.checkpoint_id);
            TickOutput::with_actions(next, vec![SupervisorAction::PersistCheckpoint(checkpoint)])
        }
        SupervisorEvent::PlanGenerated { .. } => {
            if matches!(state.phase, MissionPhase::Planning) {
                next.phase = MissionPhase::Executing;
                let cp = snapshot_with(&next);
                next.last_checkpoint_id = Some(cp.checkpoint_id);
                TickOutput::with_actions(next, vec![SupervisorAction::PersistCheckpoint(cp)])
            } else {
                TickOutput::pass_through(state)
            }
        }
        SupervisorEvent::PlanConfidenceLow { .. } => {
            if matches!(state.phase, MissionPhase::Planning) {
                let cp = snapshot_with(&next);
                next.last_checkpoint_id = Some(cp.checkpoint_id);
                TickOutput::with_actions(
                    next,
                    vec![
                        SupervisorAction::TriggerReplan,
                        SupervisorAction::PersistCheckpoint(cp),
                    ],
                )
            } else {
                TickOutput::pass_through(state)
            }
        }
        SupervisorEvent::CodingStarted | SupervisorEvent::DiffEmitted { .. } => {
            if matches!(state.phase, MissionPhase::Executing) {
                TickOutput::pass_through(next)
            } else {
                TickOutput::pass_through(state)
            }
        }
        SupervisorEvent::ValidationStarted => {
            if matches!(state.phase, MissionPhase::Executing) {
                next.phase = MissionPhase::Verifying;
                let cp = snapshot_with(&next);
                next.last_checkpoint_id = Some(cp.checkpoint_id);
                TickOutput::with_actions(next, vec![SupervisorAction::PersistCheckpoint(cp)])
            } else {
                TickOutput::pass_through(state)
            }
        }
        SupervisorEvent::ValidationPassed { .. } => {
            if matches!(state.phase, MissionPhase::Verifying) {
                next.phase = MissionPhase::Done;
                next.budget_tally.consecutive_failures = 0;
                let cp = snapshot_with(&next);
                next.last_checkpoint_id = Some(cp.checkpoint_id);
                TickOutput::with_actions(
                    next,
                    vec![
                        SupervisorAction::MarkDone,
                        SupervisorAction::PersistCheckpoint(cp),
                    ],
                )
            } else {
                TickOutput::pass_through(state)
            }
        }
        SupervisorEvent::ValidationFailed { report_id } => {
            if matches!(state.phase, MissionPhase::Verifying) {
                next.phase = MissionPhase::Recovering;
                next.budget_tally.consecutive_failures =
                    next.budget_tally.consecutive_failures.saturating_add(1);
                let cp = snapshot_with(&next);
                next.last_checkpoint_id = Some(cp.checkpoint_id);
                TickOutput::with_actions(
                    next,
                    vec![
                        SupervisorAction::TriggerRepair { report_id },
                        SupervisorAction::PersistCheckpoint(cp),
                    ],
                )
            } else {
                TickOutput::pass_through(state)
            }
        }
        SupervisorEvent::CriticalValidation { .. } => {
            // RFC 14 §8 same-stage-twice → Critical → escalate to Planning.
            if matches!(
                state.phase,
                MissionPhase::Verifying | MissionPhase::Recovering
            ) {
                next.phase = MissionPhase::Planning;
                let cp = snapshot_with(&next);
                next.last_checkpoint_id = Some(cp.checkpoint_id);
                TickOutput::with_actions(
                    next,
                    vec![
                        SupervisorAction::TriggerReplan,
                        SupervisorAction::PersistCheckpoint(cp),
                    ],
                )
            } else {
                TickOutput::pass_through(state)
            }
        }
        SupervisorEvent::RepairAttempted {
            repair_id: _,
            outcome,
        } => {
            if !matches!(state.phase, MissionPhase::Recovering) {
                return TickOutput::pass_through(state);
            }
            match outcome {
                crate::repair::types::RepairOutcome::Applied => {
                    next.phase = MissionPhase::Verifying;
                    let cp = snapshot_with(&next);
                    next.last_checkpoint_id = Some(cp.checkpoint_id);
                    TickOutput::with_actions(next, vec![SupervisorAction::PersistCheckpoint(cp)])
                }
                crate::repair::types::RepairOutcome::Proposed => {
                    next.phase = MissionPhase::Verifying;
                    let cp = snapshot_with(&next);
                    next.last_checkpoint_id = Some(cp.checkpoint_id);
                    TickOutput::with_actions(next, vec![SupervisorAction::PersistCheckpoint(cp)])
                }
                crate::repair::types::RepairOutcome::EscalatedPlanning => {
                    next.phase = MissionPhase::Planning;
                    let cp = snapshot_with(&next);
                    next.last_checkpoint_id = Some(cp.checkpoint_id);
                    TickOutput::with_actions(
                        next,
                        vec![
                            SupervisorAction::TriggerReplan,
                            SupervisorAction::PersistCheckpoint(cp),
                        ],
                    )
                }
                crate::repair::types::RepairOutcome::EscalatedHuman => {
                    next.phase = MissionPhase::Halted;
                    let cp = snapshot_with(&next);
                    next.last_checkpoint_id = Some(cp.checkpoint_id);
                    TickOutput::with_actions(
                        next,
                        vec![
                            SupervisorAction::EscalateHuman {
                                reason: "Repair escalated to human (RFC 15 §4)".into(),
                            },
                            SupervisorAction::PersistCheckpoint(cp),
                        ],
                    )
                }
            }
        }
        SupervisorEvent::Heartbeat(snap) => {
            // RFC 19 §4 — stall detection. Two consecutive stalls →
            // kick; a third consecutive stall after a failed kick →
            // restart.
            if snap.is_stalled() {
                next.consecutive_stalls = next.consecutive_stalls.saturating_add(1);
                if next.consecutive_stalls >= 2 {
                    let agent = snap.agent_id.unwrap_or_else(Uuid::nil);
                    if next.consecutive_kicks == 0 {
                        next.consecutive_kicks = 1;
                        TickOutput::with_actions(
                            next,
                            vec![SupervisorAction::Kick { agent_id: agent }],
                        )
                    } else {
                        next.consecutive_kicks = 0;
                        next.consecutive_stalls = 0;
                        next.mission_restarts = next.mission_restarts.saturating_add(1);
                        if next.mission_restarts >= 4 {
                            next.phase = MissionPhase::Halted;
                            TickOutput::with_actions(
                                next,
                                vec![SupervisorAction::EscalateHuman {
                                    reason: "4 mission restarts (RFC 19 §6.2) — structural blocker"
                                        .into(),
                                }],
                            )
                        } else if let Some(cp) = state.last_checkpoint_id {
                            TickOutput::with_actions(
                                next,
                                vec![SupervisorAction::Restart {
                                    agent_id: agent,
                                    checkpoint_id: cp,
                                }],
                            )
                        } else {
                            TickOutput::pass_through(next)
                        }
                    }
                } else {
                    TickOutput::pass_through(next)
                }
            } else {
                next.consecutive_stalls = 0;
                next.consecutive_kicks = 0;
                TickOutput::pass_through(next)
            }
        }
        SupervisorEvent::ContextCompacted => {
            // The tally bump already happened above; cap-evaluation
            // would have halted. Just pass through.
            TickOutput::pass_through(next)
        }
        SupervisorEvent::UserSteered { message } => {
            // RFC 19 §6 `on_goal_drift` — re-articulate the goal.
            // The user steer resets the doom-loop detector (§6
            // `recovery.on_doom_loop`).
            ctx.doom.reset();
            if matches!(state.phase, MissionPhase::Halted) {
                next.phase = MissionPhase::Planning;
                TickOutput::with_actions(
                    next,
                    vec![
                        SupervisorAction::InjectPrompt {
                            prompt: format!(
                                "Re-articulate the original goal. User steer: {message}"
                            ),
                        },
                        SupervisorAction::TriggerReplan,
                    ],
                )
            } else {
                TickOutput::with_actions(
                    next,
                    vec![SupervisorAction::InjectPrompt {
                        prompt: format!("Re-articulate the original goal. User steer: {message}"),
                    }],
                )
            }
        }
        SupervisorEvent::UserApproved => {
            if matches!(state.phase, MissionPhase::Halted) {
                next.phase = MissionPhase::Executing;
                TickOutput::with_actions(next, vec![])
            } else {
                TickOutput::pass_through(state)
            }
        }
        SupervisorEvent::UserHalted => {
            next.phase = MissionPhase::Idle;
            next.mission_id = None;
            TickOutput::with_actions(next, vec![])
        }
        SupervisorEvent::ToolCall { .. } => {
            // Iteration/cost tally already handled above.
            TickOutput::pass_through(next)
        }
    }
}

/// Check whether ANY budget cap has been exceeded. Returns the first
/// one in the order iterations → minutes → cost → failures →
/// compactions.
fn cap_hit(state: &SupervisorState) -> Option<BudgetCapKind> {
    let caps = state.caps;
    let t = state.budget_tally;
    if caps.max_iterations > 0 && t.iterations >= caps.max_iterations {
        return Some(BudgetCapKind::Iterations);
    }
    if caps.max_minutes > 0 && t.elapsed_minutes >= caps.max_minutes as f64 {
        return Some(BudgetCapKind::Minutes);
    }
    if caps.max_cost_usd > 0.0 && t.cost_usd >= caps.max_cost_usd {
        return Some(BudgetCapKind::Cost);
    }
    if caps.max_consecutive_failures > 0 && t.consecutive_failures >= caps.max_consecutive_failures
    {
        return Some(BudgetCapKind::ConsecutiveFailures);
    }
    if caps.max_context_compactions > 0 && t.context_compactions >= caps.max_context_compactions {
        return Some(BudgetCapKind::ContextCompactions);
    }
    None
}

fn halt_on_budget(state: &SupervisorState, cap: BudgetCapKind) -> TickOutput {
    let mut next = state.clone();
    next.phase = MissionPhase::Halted;
    let cp = snapshot_with(&next);
    next.last_checkpoint_id = Some(cp.checkpoint_id);
    TickOutput::with_actions(
        next,
        vec![
            SupervisorAction::HaltSession {
                reason: format!("budget cap `{:?}` exceeded (RFC 19 §6)", cap),
                cap: Some(cap),
            },
            SupervisorAction::PersistCheckpoint(cp),
        ],
    )
}

fn doom_loop_actions(
    state: &SupervisorState,
    _ctx: &TickContext,
    _fired_key: &str,
) -> Vec<SupervisorAction> {
    let mut out = vec![SupervisorAction::InjectPrompt {
        prompt:
            "You're repeating the same tool call. Summarize the state, list current blockers, propose 3 alternative approaches."
                .into(),
    }];
    // AUTONOMOUS downgrades to AUTOPILOT; the other modes just log.
    match state.mode {
        ExecutionMode::Autonomous | ExecutionMode::Autopilot => {
            out.push(SupervisorAction::DowngradeMode {
                to: ExecutionMode::Autopilot,
            });
        }
        _ => {}
    }
    out
}

fn snapshot_with(state: &SupervisorState) -> MissionCheckpoint {
    state.snapshot()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repair::types::RepairOutcome;

    fn fresh_state() -> SupervisorState {
        SupervisorState::new(BudgetCaps::DEFAULT, ExecutionMode::HumanInLoop)
    }

    fn mission_id() -> Uuid {
        Uuid::new_v4()
    }

    fn advance_planning(state: SupervisorState, ctx: &mut TickContext) -> SupervisorState {
        let out = tick(
            ctx,
            state,
            SupervisorEvent::MissionStarted {
                mission_id: mission_id(),
            },
        );
        assert_eq!(out.state.phase, MissionPhase::Planning);
        let out = tick(
            ctx,
            out.state,
            SupervisorEvent::PlanGenerated {
                plan_id: Uuid::new_v4(),
            },
        );
        assert_eq!(out.state.phase, MissionPhase::Executing);
        out.state
    }

    #[test]
    fn idle_state_accepts_mission() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let state = fresh_state();
        let out = tick(
            &mut ctx,
            state,
            SupervisorEvent::MissionStarted {
                mission_id: mission_id(),
            },
        );
        assert_eq!(out.state.phase, MissionPhase::Planning);
        assert_eq!(out.actions.len(), 1);
        assert!(matches!(
            out.actions[0],
            SupervisorAction::PersistCheckpoint(_)
        ));
    }

    #[test]
    fn planning_state_rejects_mission() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let mut state = fresh_state();
        state = tick(
            &mut ctx,
            state,
            SupervisorEvent::MissionStarted {
                mission_id: mission_id(),
            },
        )
        .state;
        let out = tick(
            &mut ctx,
            state,
            SupervisorEvent::MissionStarted {
                mission_id: mission_id(),
            },
        );
        assert_eq!(
            out.state.phase,
            MissionPhase::Planning,
            "MissionStarted is a no-op in Planning"
        );
        assert!(out.actions.is_empty());
    }

    #[test]
    fn full_happy_path_idle_to_done() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let state = fresh_state();
        let state = advance_planning(state, &mut ctx);
        let out = tick(&mut ctx, state, SupervisorEvent::ValidationStarted);
        assert_eq!(out.state.phase, MissionPhase::Verifying);
        let out = tick(
            &mut ctx,
            out.state,
            SupervisorEvent::ValidationPassed {
                report_id: Uuid::new_v4(),
            },
        );
        assert_eq!(out.state.phase, MissionPhase::Done);
        assert!(out
            .actions
            .iter()
            .any(|a| matches!(a, SupervisorAction::MarkDone)));
    }

    #[test]
    fn validation_failed_triggers_repair() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let state = advance_planning(fresh_state(), &mut ctx);
        let state = tick(&mut ctx, state, SupervisorEvent::ValidationStarted).state;
        let out = tick(
            &mut ctx,
            state,
            SupervisorEvent::ValidationFailed {
                report_id: Uuid::new_v4(),
            },
        );
        assert_eq!(out.state.phase, MissionPhase::Recovering);
        assert!(out
            .actions
            .iter()
            .any(|a| matches!(a, SupervisorAction::TriggerRepair { .. })));
        assert_eq!(out.state.budget_tally.consecutive_failures, 1);
    }

    #[test]
    fn repair_applied_returns_to_verifying() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let state = advance_planning(fresh_state(), &mut ctx);
        let state = tick(&mut ctx, state, SupervisorEvent::ValidationStarted).state;
        let state = tick(
            &mut ctx,
            state,
            SupervisorEvent::ValidationFailed {
                report_id: Uuid::new_v4(),
            },
        )
        .state;
        let out = tick(
            &mut ctx,
            state,
            SupervisorEvent::RepairAttempted {
                repair_id: Uuid::new_v4(),
                outcome: RepairOutcome::Applied,
            },
        );
        assert_eq!(out.state.phase, MissionPhase::Verifying);
    }

    #[test]
    fn repair_escalated_human_halts() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let state = advance_planning(fresh_state(), &mut ctx);
        let state = tick(&mut ctx, state, SupervisorEvent::ValidationStarted).state;
        let state = tick(
            &mut ctx,
            state,
            SupervisorEvent::ValidationFailed {
                report_id: Uuid::new_v4(),
            },
        )
        .state;
        let out = tick(
            &mut ctx,
            state,
            SupervisorEvent::RepairAttempted {
                repair_id: Uuid::new_v4(),
                outcome: RepairOutcome::EscalatedHuman,
            },
        );
        assert_eq!(out.state.phase, MissionPhase::Halted);
        assert!(out
            .actions
            .iter()
            .any(|a| matches!(a, SupervisorAction::EscalateHuman { .. })));
    }

    #[test]
    fn critical_validation_escalates_to_planning() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let state = advance_planning(fresh_state(), &mut ctx);
        let state = tick(&mut ctx, state, SupervisorEvent::ValidationStarted).state;
        let out = tick(
            &mut ctx,
            state,
            SupervisorEvent::CriticalValidation {
                report_id: Uuid::new_v4(),
            },
        );
        assert_eq!(out.state.phase, MissionPhase::Planning);
        assert!(out
            .actions
            .iter()
            .any(|a| matches!(a, SupervisorAction::TriggerReplan)));
    }

    #[test]
    fn budget_cap_iterations_halts() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let caps = BudgetCaps {
            max_iterations: 3,
            max_minutes: 0,
            max_cost_usd: 0.0,
            max_consecutive_failures: 0,
            max_context_compactions: 0,
        };
        let mut state = SupervisorState::new(caps, ExecutionMode::HumanInLoop);
        state = tick(
            &mut ctx,
            state,
            SupervisorEvent::MissionStarted {
                mission_id: mission_id(),
            },
        )
        .state;
        for _ in 0..3 {
            state = tick(
                &mut ctx,
                state,
                SupervisorEvent::ToolCall {
                    tool: "bash".into(),
                    input_hash: "ls".into(),
                    cost_usd: 0.0,
                    ts: chrono::Utc::now().to_rfc3339(),
                },
            )
            .state;
        }
        assert_eq!(state.phase, MissionPhase::Halted);
        assert_eq!(state.budget_tally.iterations, 3);
    }

    #[test]
    fn budget_cap_consecutive_failures_halts_after_five_failures() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let mut state = advance_planning(fresh_state(), &mut ctx);
        for _ in 0..5 {
            state = tick(&mut ctx, state, SupervisorEvent::ValidationStarted).state;
            state = tick(
                &mut ctx,
                state,
                SupervisorEvent::ValidationFailed {
                    report_id: Uuid::new_v4(),
                },
            )
            .state;
            // Apply a "fail" repair so we cycle back to Verifying.
            // First 4 attempts return to Recovering, and we feed
            // Proposed to keep cycling.
            state = tick(
                &mut ctx,
                state,
                SupervisorEvent::RepairAttempted {
                    repair_id: Uuid::new_v4(),
                    outcome: RepairOutcome::Proposed,
                },
            )
            .state;
        }
        assert_eq!(
            state.phase,
            MissionPhase::Halted,
            "5 consecutive failures halt"
        );
        assert!(state.budget_tally.consecutive_failures >= 5);
    }

    #[test]
    fn user_approved_unhalts() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let mut state = fresh_state();
        state.phase = MissionPhase::Halted;
        let out = tick(&mut ctx, state, SupervisorEvent::UserApproved);
        assert_eq!(out.state.phase, MissionPhase::Executing);
    }

    #[test]
    fn user_halted_resets_to_idle() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let mut state = fresh_state();
        state.phase = MissionPhase::Halted;
        state.mission_id = Some(Uuid::new_v4());
        let out = tick(&mut ctx, state, SupervisorEvent::UserHalted);
        assert_eq!(out.state.phase, MissionPhase::Idle);
        assert!(out.state.mission_id.is_none());
    }

    #[test]
    fn user_steered_resets_doom_loop_detector() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        // Fire the detector once.
        for _ in 0..3 {
            ctx.doom.observe("k", 0);
        }
        assert_eq!(ctx.doom.fire_count, 1);
        let state = fresh_state();
        let out = tick(
            &mut ctx,
            state,
            SupervisorEvent::UserSteered {
                message: "stop".into(),
            },
        );
        assert_eq!(ctx.doom.fire_count, 0, "UserSteered reset");
        assert!(out
            .actions
            .iter()
            .any(|a| matches!(a, SupervisorAction::InjectPrompt { .. })));
    }

    #[test]
    fn heartbeat_stall_kicks_then_restarts() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let mut state = advance_planning(fresh_state(), &mut ctx);
        state.active_agent_id = Some(Uuid::new_v4());
        // Two stalled heartbeats → kick on second.
        let s = HeartbeatSnapshot {
            agent_id: state.active_agent_id,
            files_changed: 0,
            events_emitted: 0,
            delta_seconds: 5,
        };
        let out = tick(&mut ctx, state.clone(), SupervisorEvent::Heartbeat(s));
        assert_eq!(out.state.consecutive_stalls, 1);
        let out = tick(&mut ctx, out.state, SupervisorEvent::Heartbeat(s));
        assert!(out
            .actions
            .iter()
            .any(|a| matches!(a, SupervisorAction::Kick { .. })));
        // Third consecutive stall after kick → restart.
        let state = out.state;
        let out = tick(&mut ctx, state, SupervisorEvent::Heartbeat(s));
        assert!(out
            .actions
            .iter()
            .any(|a| matches!(a, SupervisorAction::Restart { .. })));
    }

    #[test]
    fn heartbeat_recovers_on_delta() {
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let mut state = advance_planning(fresh_state(), &mut ctx);
        state.active_agent_id = Some(Uuid::new_v4());
        let stalled = HeartbeatSnapshot {
            agent_id: state.active_agent_id,
            files_changed: 0,
            events_emitted: 0,
            delta_seconds: 5,
        };
        let live = HeartbeatSnapshot {
            agent_id: state.active_agent_id,
            files_changed: 1,
            events_emitted: 2,
            delta_seconds: 5,
        };
        let out = tick(&mut ctx, state.clone(), SupervisorEvent::Heartbeat(stalled));
        assert_eq!(out.state.consecutive_stalls, 1);
        let out = tick(&mut ctx, out.state, SupervisorEvent::Heartbeat(live));
        assert_eq!(out.state.consecutive_stalls, 0);
        assert_eq!(out.state.consecutive_kicks, 0);
    }
}
