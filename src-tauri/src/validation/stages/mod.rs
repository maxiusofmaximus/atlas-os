// Atlas OS — Validation stage modules (RFC 14 §1, §2). Each stage is
// a pure transformation that consumes the workspace snapshot + the `Diff`
// being validated and returns a `StageSummary`. The runner threads them
// together; each stage is independently unit-tested.

pub mod dead_code;
pub mod e2e;
pub mod iac;
pub mod layer_boundary;
pub mod lint_format;
pub mod security_scan;
pub mod supply_chain;
pub mod type_check;
pub mod unit_tests;

use crate::validation::types::{StageKind, StageStatus, StageSummary};

/// Common interface every stage implements. The runner only needs the
/// kind tag and a `run` callable the stage advertises.
pub trait Stage {
    fn kind(&self) -> StageKind;
    fn run(&self, ctx: &StageContext) -> StageSummary;
}

/// Inputs shared by every stage. `Diff`-aware so a stage can decide
/// whether the touched files fall in its scope (e.g. `SupplyChain` only
/// fires when the diff touches `Cargo.lock` / `pnpm-lock.yaml`).
pub struct StageContext<'a> {
    pub diff: &'a crate::coding::types::Diff,
    /// Paths the diff touched, normalised to forward slashes. Pre-computed
    /// so the stages do not have to walk `Diff.files` themselves.
    pub touched_paths: Vec<String>,
    /// RFC 14 §7 — `Loose` skips `UnitTests` / downstream; `Pr` forces
    /// E2E; `Strict` is the default.
    pub mode: crate::validation::types::ValidationMode,
}

impl<'a> StageContext<'a> {
    pub fn touches(&self, needle: &str) -> bool {
        self.touched_paths
            .iter()
            .any(|p| p == needle || p.ends_with(needle))
    }
    pub fn touches_any(&self, needles: &[&str]) -> bool {
        self.touched_paths
            .iter()
            .any(|p| needles.iter().any(|n| p == n || p.ends_with(n)))
    }
}

/// Helper for the simple Pass case (no findings).
pub fn pass(kind: StageKind, elapsed_ms: u64, summary: &str) -> StageSummary {
    StageSummary {
        stage: kind,
        status: StageStatus::Pass,
        elapsed_ms,
        findings: vec![],
        summary: summary.into(),
    }
}

/// Helper for the simple Skipped case (no findings, no elapsed time).
pub fn skipped(kind: StageKind, summary: &str) -> StageSummary {
    StageSummary {
        stage: kind,
        status: StageStatus::Skipped,
        elapsed_ms: 0,
        findings: vec![],
        summary: summary.into(),
    }
}
