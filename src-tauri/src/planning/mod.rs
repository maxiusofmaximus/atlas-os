// Atlas OS — Planning Engine (RFC 12).
//
// The Planning Engine is the motor that *only thinks*. It consumes a
// locked `MissionConsolidated` (RFC 23 §4) plus its associated
// `PublicUnderstandingVerdict` (RFC 23 §3) and produces a `Plan` with:
//   - objectives and verifiable success criteria,
//   - milestones exposing inter-dependencies for the Swarm Coordinator,
//   - steps with skills/models/strategy hints,
//   - risk + impact (drives approvals escalation per RFC 02 §4.1),
//   - a confidence score gated at `PLAN_CONFIDENCE_THRESHOLD = 0.7` so
//     the Coding Engine cannot init on a weak plan (RFC 12 §7).
//
// Phase 1 is heuristic — no LLM call. The signature of `runner::run` is
// stable and matches the LLM-driven planner coming with RFC 04 so the
// rest of the kernel stays forward-compatible.

pub mod availability;

#[cfg(feature = "dag_mode")]
pub mod graph_emitter;
pub mod grill;
pub mod runner;
pub mod types;

pub use availability::{availability_now, next_free_slot, Availability, TurnPolicy};

pub use grill::{grill_plan, GrillQuestion, GrillReport};

pub use runner::{run, AttachedResearchRun, ClarificationAnswer, PlanningInput};
pub use types::*;

#[cfg(test)]
mod tests;
