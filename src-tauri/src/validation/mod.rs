// Atlas OS — Validation Engine (RFC 14). Validates a `Diff` produced
// by the Coding Engine (RFC 13) through a stage cascade and emits a
// structured `ValidationReport`. Phase 1 is heuristic-only; stage
// signatures are stable and the real toolchain integrations arrive in
// Phase 2 once the Model Orchestrator (RFC 04) is wired up.

pub mod runner;
pub mod stages;
pub mod types;

pub use runner::{run, ValidationInput};
pub use types::{
    Finding, StageKind, StageStatus, StageSummary, ValidationMode, ValidationOutcome,
    ValidationReport,
};
