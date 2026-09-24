// Atlas OS — Execution Supervisor canonical types (RFC 19).
//
// The Execution Supervisor replaces the external `supervisor.ps1`
// `while ($true)` loop (RFC 19 §7) with an in-kernel state machine that
// keeps the system alive WITHOUT entering an infinite loop. It owns:
//
//   * the mission-level state machine (idle → planning → executing →
//     verifying → recovering → halted → done) — RFC 19 §6.1,
//   * the heartbeat + stall detector (§4),
//   * the anti-infinite-loop policy + doom-loop detector (§6),
//   * the budget caps (max_iterations / max_minutes / max_cost_usd /
//     max_consecutive_failures / max_context_compactions) — §6 `caps`,
//   * the resume-point snapshots (§5).
//
// Phase 1 is a pure, synchronous state machine: `tick(state, event)`
// returns a new `SupervisorState` plus a list of `SupervisorAction`s
// the kernel should dispatch (publish on the Bus, kick an agent,
// persist a checkpoint, escalate to the human). No timers, no threads,
// no disk — the Execution Supervisor's host (the Tauri main thread or
// the headless CLI) supplies the heartbeat cadence and feeds events.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// RFC 19 §6.1 — the seven lifecycle states of a mission under
// supervision. Transitions are documented in `SupervisorState::next`
/// and asserted by the runner; each transition leaves its traza in the
/// Journal (`supervisor_events`, M8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionPhase {
    /// No mission loaded yet. The next `MissionConsolidated` event
    /// transitions to `Planning`.
    Idle,
    /// The Planning Engine (RFC 12) is producing a plan.
    Planning,
    /// The Coding Engine (RFC 13) is emitting diffs.
    Executing,
    /// The Validation Engine (RFC 14) is running its cascade.
    Verifying,
    /// The Repair Engine (RFC 15) is fixing a failed validation — the
    /// loop returns to `Verifying` once a fix is proposed/applied, or
    /// to `Planning` on EscalatedPlanning, or to `Halted` on
    /// EscalatedHuman.
    Recovering,
    /// Hard stop. The mission cannot continue until the human either
    /// `Steer`s, approves an escalation, or `Fork`s a new mission.
    /// The supervisor emits an `ApprovalRequest` on entering `Halted`.
    Halted,
    /// Mission converged: validation Pass + no pending repair. The
    /// supervisor persists a final checkpoint and returns to `Idle`.
    Done,
}

impl MissionPhase {
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Planning => "planning",
            Self::Executing => "executing",
            Self::Verifying => "verifying",
            Self::Recovering => "recovering",
            Self::Halted => "halted",
            Self::Done => "done",
        }
    }

    /// True when the supervisor is allowed to accept a new mission
    /// (only `Idle` and `Done`). Other phases need an explicit `Halt`
    /// or done-trigger first.
    pub fn accepts_new_mission(self) -> bool {
        matches!(self, Self::Idle | Self::Done)
    }
}

/// RFC 19 §6 `policies.anti_infinite_loop.caps` — the budget envelopes
/// a single mission session cannot exceed. Hitting ANY of them trips
/// the `on_budget_hit` recovery path (RFC 19 §6): halt the session,
/// produce a summary, request a human decision.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BudgetCaps {
    /// RFC 19 §6 — per-session tool-call ceiling. `0` disables.
    pub max_iterations: u32,
    /// RFC 19 §6 — per-session wall-clock ceiling in minutes.
    pub max_minutes: u32,
    /// RFC 19 §6 — per-session cost ceiling in USD.
    pub max_cost_usd: f64,
    /// RFC 19 §6 — consecutive test failures before `ask`.
    pub max_consecutive_failures: u32,
    /// RFC 19 §6 — context compactions before escalating to the human.
    pub max_context_compactions: u32,
}

impl BudgetCaps {
    /// RFC 19 §6 defaults — the values from the `caps:` block of the
    /// example YAML. The host may override them per-profile (RFC 07).
    pub const DEFAULT: Self = Self {
        max_iterations: 25,
        max_minutes: 30,
        max_cost_usd: 1.50,
        max_consecutive_failures: 5,
        max_context_compactions: 2,
    };

    /// True when the running tally crosses ANY of the caps. The runner
    /// transitions to `Halted` and emits `BudgetExceeded` when this
    /// returns `true`.
    pub fn exceeded(&self, tally: &BudgetTally) -> bool {
        (self.max_iterations > 0 && tally.iterations >= self.max_iterations)
            || (self.max_minutes > 0 && tally.elapsed_minutes >= self.max_minutes as f64)
            || (self.max_cost_usd > 0.0 && tally.cost_usd >= self.max_cost_usd)
            || (self.max_consecutive_failures > 0
                && tally.consecutive_failures >= self.max_consecutive_failures)
            || (self.max_context_compactions > 0
                && tally.context_compactions >= self.max_context_compactions)
    }
}

impl Default for BudgetCaps {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Running totals the supervisor maintains across a session. The
/// runner updates these on every `tick` and consults
/// `BudgetCaps::exceeded` to decide whether to halt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BudgetTally {
    pub iterations: u32,
    pub elapsed_minutes: f64,
    pub cost_usd: f64,
    pub consecutive_failures: u32,
    pub context_compactions: u32,
}

/// RFC 19 §6.1 — execution mode governs how the supervisor reacts to
/// the doom-loop detector. `MANUAL_CLASSIC` simply logs; the three
/// permissioned modes (`HUMAN_IN_LOOP`, `AUTOPILOT`, `AUTONOMOUS`)
/// dictate the on_match action (RFC 19 §6 `on_match`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// User invokes every tool one by one; the supervisor is passive.
    ManualClassic,
    /// Human approves each doom-loop trigger.
    #[default]
    HumanInLoop,
    /// Human approves doom-loop; soft caps downgrade to this mode.
    Autopilot,
    /// Autonomous: doom-loop hard-denies + downgrades to AUTOPILOT.
    Autonomous,
}

impl ExecutionMode {
    pub fn tag(&self) -> &'static str {
        match self {
            Self::ManualClassic => "manual_classic",
            Self::HumanInLoop => "human_in_loop",
            Self::Autopilot => "autopilot",
            Self::Autonomous => "autonomous",
        }
    }
}

/// RFC 19 §6 `recovery.on_budget_hit` — the action the supervisor
/// emits when the budget envelope is violated. Phase 1 only emits a
/// `Halt + ApprovalRequest`; `produce_summary` lands with the Model
/// Orchestrator (Phase 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetCapKind {
    Iterations,
    Minutes,
    Cost,
    ConsecutiveFailures,
    ContextCompactions,
}

impl BudgetCapKind {
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Iterations => "iterations",
            Self::Minutes => "minutes",
            Self::Cost => "cost",
            Self::ConsecutiveFailures => "consecutive_failures",
            Self::ContextCompactions => "context_compactions",
        }
    }
}

/// RFC 19 §5 — a resume-point snapshot. The supervisor persists one on
/// every phase transition; `resume(mission_id)` loads the most recent
/// one and dispatches to the right engine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionCheckpoint {
    pub checkpoint_id: Uuid,
    pub mission_id: Uuid,
    pub phase: MissionPhase,
    /// The plan the Planning Engine last emitted. `None` before the
    /// first `PlanGenerated` event.
    pub current_plan_id: Option<Uuid>,
    /// Last validation report id. `None` before the first
    /// `ValidationReport`.
    pub last_validation_report_id: Option<Uuid>,
    /// Last repair report id. `None` before the first repair.
    pub last_repair_id: Option<Uuid>,
    /// Running budget tally at snapshot time.
    pub budget_tally: BudgetTally,
    pub generated_at: String,
}

/// RFC 19 §4 — a single heartbeat observation. The host schedules the
/// heartbeats (default N=5s); the supervisor compares the snapshot
/// against the previous beat to detect stalls (RFC 19 §4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatSnapshot {
    pub agent_id: Option<Uuid>,
    /// Files changed since the previous beat. `0` => no disk delta.
    pub files_changed: u32,
    /// Bus events emitted since the previous beat.
    pub events_emitted: u32,
    /// Wallclock seconds since the previous beat.
    pub delta_seconds: u32,
}

impl HeartbeatSnapshot {
    /// RFC 19 §4 — a beat is "stalled" when both disk delta and bus
    /// delta are zero. The supervisor kicks the agent on the first
    /// stall and reinficiates the agent on the second consecutive
    /// stall (RFC 19 §4 last paragraph).
    pub fn is_stalled(&self) -> bool {
        self.files_changed == 0 && self.events_emitted == 0
    }
}

/// RFC 19 §6.1 — inputs the supervisor reacts to. The runner is a pure
/// function over this enum; the host (Tauri main thread / CLI) maps
/// real BusEvents + heartbeats into these.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SupervisorEvent {
    MissionStarted {
        mission_id: Uuid,
    },
    PlanGenerated {
        plan_id: Uuid,
    },
    PlanConfidenceLow {
        plan_id: Uuid,
        confidence: f32,
    },
    CodingStarted,
    DiffEmitted {
        diff_id: Uuid,
    },
    ValidationStarted,
    ValidationPassed {
        report_id: Uuid,
    },
    DoneClaimed {
        report_id: Uuid,
        claim: String,
        evidence: Vec<String>,
    },
    ValidationFailed {
        report_id: Uuid,
    },
    CriticalValidation {
        report_id: Uuid,
    },
    RepairAttempted {
        repair_id: Uuid,
        outcome: crate::repair::types::RepairOutcome,
    },
    Heartbeat(HeartbeatSnapshot),
    ToolCall {
        tool: String,
        input_hash: String,
        cost_usd: f64,
        ts: String,
    },
    ContextCompacted,
    UserSteered {
        message: String,
    },
    UserApproved,
    UserHalted,
}

/// RFC 19 §4/§6 — the actions the supervisor asks the host to perform.
/// The host is responsible for actually publishing the matching
/// `BusEvent` (or kernel command) — Phase 1 keeps the supervisor pure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SupervisorAction {
    /// RFC 19 §4 — send "continue according to plan" to the agent.
    Kick { agent_id: Uuid },
    /// RFC 19 §4 — restart the agent from the last checkpoint.
    Restart { agent_id: Uuid, checkpoint_id: Uuid },
    /// RFC 19 §6 — escalate to the human via an ApprovalRequest.
    EscalateHuman { reason: String },
    /// RFC 19 §6 — downgrade the execution mode (AUTONOMOUS →
    /// AUTOPILOT) after a doom-loop trigger.
    DowngradeMode { to: ExecutionMode },
    /// RFC 19 §6 — inject a prompt into the active agent.
    InjectPrompt { prompt: String },
    /// RFC 19 §6 — halt the session and request a human decision.
    HaltSession {
        reason: String,
        cap: Option<BudgetCapKind>,
    },
    /// RFC 19 §5 — persist a checkpoint snapshot.
    PersistCheckpoint(MissionCheckpoint),
    /// Forward to the Planning Engine for a replan + research cycle.
    TriggerReplan,
    /// Forward to the Repair Engine.
    TriggerRepair { report_id: Uuid },
    /// Forward to the Validation Engine for re-validation.
    TriggerRevalidation { diff_id: Uuid },
    /// Inform the HUD / Journal that the mission converged.
    MarkDone,
    /// RFC 30 §2.1 — refuse an evidence-less done claim without leaving
    /// `Verifying`. The host surfaces `reason` exactly as Canny does.
    BlockDone { reason: String },
}

/// RFC 19 §6.1 — the full supervisor state passed between `tick`
/// invocations. The host owns the canonical instance; `tick` returns a
/// new copy per call (functional state machine, no interior mutation).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SupervisorState {
    pub mission_id: Option<Uuid>,
    pub phase: MissionPhase,
    pub mode: ExecutionMode,
    pub budget_tally: BudgetTally,
    pub caps: BudgetCaps,
    /// Agent currently active on the mission (if any).
    pub active_agent_id: Option<Uuid>,
    /// Last checkpoint id (for restart). The host also persists the
    /// checkpoint to the Journal.
    pub last_checkpoint_id: Option<Uuid>,
    /// RFC 19 §4 — consecutive stall count since the last delta.
    pub consecutive_stalls: u32,
    /// RFC 19 §4 — consecutive failed kick count (after 2 → restart).
    pub consecutive_kicks: u32,
    /// RFC 19 §6 — last recovery attempts per agent (max 2).
    pub recovery_attempts: u32,
    /// RFC 19 §6.2 — restarts per mission. After 4 → escalate.
    pub mission_restarts: u32,
    /// RFC 19 §6.2 — restarts per step. After 2 → blocked.
    pub step_restarts: u32,
    /// Id of the currently active step (cleared on `ValidationPassed`
    /// / `DiffEmitted` cycle).
    pub current_step_id: Option<String>,
}

impl SupervisorState {
    /// Build a fresh supervisor for a new mission. Phase + budget are
    /// zeroed; the host calls `tick` with a `MissionStarted` event to
    /// drive the first transition.
    pub fn new(caps: BudgetCaps, mode: ExecutionMode) -> Self {
        Self {
            mission_id: None,
            phase: MissionPhase::Idle,
            mode,
            budget_tally: BudgetTally::default(),
            caps,
            active_agent_id: None,
            last_checkpoint_id: None,
            consecutive_stalls: 0,
            consecutive_kicks: 0,
            recovery_attempts: 0,
            mission_restarts: 0,
            step_restarts: 0,
            current_step_id: None,
        }
    }

    /// Build a checkpoint from the current state. The host also
    /// publishes a `JournalCheckpoint` BusEvent.
    pub fn snapshot(&self) -> MissionCheckpoint {
        MissionCheckpoint {
            checkpoint_id: Uuid::new_v4(),
            mission_id: self.mission_id.unwrap_or_else(Uuid::nil),
            phase: self.phase,
            current_plan_id: None,
            last_validation_report_id: None,
            last_repair_id: None,
            budget_tally: self.budget_tally,
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
