// Atlas OS — Prompt Understanding Pipeline (RFC 23).
//
// Phase 1 wires the 9-step pipeline described in RFC 23 §2. Design choices
// for Phase 1 are documented in `runner.rs`. The public API of this module
// is `run(raw_prompt, &options)` from `runner`.

pub mod runner;
pub mod steps;
pub mod types;

pub use runner::{
    run, run_with_learned_rules, run_with_profile, run_with_profile_and_rules, PipelineOptions,
};
pub use types::*;

#[cfg(test)]
mod tests;
