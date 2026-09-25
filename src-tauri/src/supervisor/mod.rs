// Atlas OS — Execution Supervisor (RFC 19). Replaces the external
// `supervisor.ps1` `while ($true) { ... }` loop (RFC 19 §7) with an
// in-kernel state machine that keeps the system alive WITHOUT entering
// an infinite loop. The supervisor owns:
//
//   * the mission state machine (RFC 19 §6.1) — `MissionPhase`,
//   * the heartbeat + stall detector (§4),
//   * the doom-loop detector (§6),
//   * the budget caps (§6 `caps`),
//   * the resume-point snapshots (§5).
//
// Phase 1 keeps the supervisor pure: `tick(ctx, state, event)` returns
// the next state plus a list of `SupervisorAction`s for the host to
// dispatch on the Kernel Bus. Phase 2 will wire the host side (timers,
// threads, Journal) onto this pure core.

pub mod doom_loop;
pub mod resume;
pub mod runner;
pub mod types;

pub use doom_loop::{DoomLoopConfig, DoomLoopDetector};
pub use resume::{parse_phase_tag, resume_state};
pub use runner::{tick, TickContext, TickOutput};
pub use types::{
    BudgetCapKind, BudgetCaps, BudgetTally, ExecutionMode, HeartbeatSnapshot, MissionCheckpoint,
    MissionPhase, SupervisorAction, SupervisorEvent, SupervisorState,
};
