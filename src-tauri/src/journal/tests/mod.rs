// Atlas OS — Rust tests for the Journal store.
// Phase 20.3: former inline `*_tests` modules split one-per-file.

#[cfg(test)]
mod journal_store_tests;

#[cfg(test)]
mod journal_verdict_tests;

#[cfg(test)]
mod journal_plan_tests;

#[cfg(test)]
mod journal_diff_tests;

#[cfg(test)]
mod journal_validation_tests;

#[cfg(test)]
mod journal_repair_tests;

#[cfg(test)]
mod journal_pattern_tests;

#[cfg(test)]
mod journal_checkpoint_tests;

#[cfg(test)]
mod journal_skill_tests;

#[cfg(test)]
mod mission_graph_schema_tests;

mod model_resets_schema_tests;

#[cfg(test)]
mod research_m25_schema_tests;

#[cfg(test)]
mod research_m26_schema_tests;

mod research_m27_schema_tests;

#[cfg(test)]
mod journal_phase80_tests;
