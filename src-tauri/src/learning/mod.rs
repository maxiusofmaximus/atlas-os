// Atlas OS — Learning Engine (RFC 16). Derives reusable
// `Pattern`s from `RepairReport`s so the kernel never repeats the same
// mistake twice (RFC 16 §2 Reflection Loop). Phase 1 is heuristic-only:
// the runner consumes a `RepairReport` and emits one `LearnOutcome`
// (which MAY carry a `Draft` pattern). Promotion to `Candidate` /
// `Active` and the Skill Compressor (RFC 16 §5) land with Phase 2
// alongside the vector store and the Model Orchestrator (RFC 04).

pub mod runner;
pub mod types;

pub use runner::run;
pub use types::{
    LearnInput, LearnOutcome, Pattern, PatternMetrics, RuleLifecycle, RuleThen, RuleWhen,
};
