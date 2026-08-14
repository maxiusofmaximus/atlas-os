// Atlas OS — Repair Engine (RFC 15). Converts a failed
// `ValidationReport` into a micro-cycle of repair attempts and emits a
// structured `RepairReport`. Phase 1 is heuristic-only; the model-driven
// root-cause analyser lands Phase 2 alongside the Model Orchestrator
// (RFC 04) and the Learning Engine (RFC 16).

pub mod runner;
pub mod types;

pub use runner::{run, RepairInput};
pub use types::{
    ErrorClass, RepairAttempt, RepairOutcome, RepairReport, RepairStrategy, MAX_ATTEMPTS_PER_STAGE,
    MAX_MISSION_FAILURES,
};
