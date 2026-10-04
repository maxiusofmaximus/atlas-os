// Atlas OS — Coding Engine (RFC 13).
//
// The Coding Engine emits structured `Diff`s from a `Plan` + `Step`
// (RFC 13 §2). It REJECTS rather than panics when a guard fires so the
// HUD + Journal can render the rejection path the same way the Planning
// Engine does (RFC 12 §7).
//
// Phase 1 is heuristic — no LLM call. The runner signature matches the
// LLM-driven coder coming with RFC 04 so the rest of the kernel stays
// forward-compatible.

pub mod llm;
pub mod runner;
pub mod types;

pub use llm::{parse_diff_json, DiffMeta, DiffParseError, DIFF_CONTRACT_PROMPT};
pub use runner::{run, CodingInput, WorkspaceFile};
pub use types::*;

#[cfg(test)]
mod tests;
