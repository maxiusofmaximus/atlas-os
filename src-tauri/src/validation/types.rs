// OpenCode OS — Validation Engine canonical types (RFC 14 §3).
//
// The Validation Engine emits structured `StageSummary`s (RFC 14 §3 "stage
// summaries canónicos"), never raw log strings. The Repair Engine (RFC 15)
// and the HUD (RFC 24) consume these structures directly.
//
// Pipeline contract (RFC 14 §1, §4):
//   1. each stage runs in the order declared by `StageKind` (RFC 14 §1);
//   2. if a stage returns `StageStatus::Fail`, subsequent stages are
//      skipped — "el Repair Engine recibe el summary, decide y reprocesa";
//   3. the runner returns a `ValidationReport` regardless of outcome
//      (no Result::Err channel — the Coding Engine remains free to
//      record a rejection as a structured event).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// RFC 14 §1 — the stage catalogue. Order matters: it matches the RFC §1
/// pipeline layout. `LintFormat`, `TypeCheck`, `UnitTests` are always
/// on; the rest are conditionally on (RFC 14 §2: e.g. `E2E` runs when UI
/// is touched, `SupplyChain` when a lockfile is touched, `IaC` when the
/// `Cargo.toml` / `Dockerfile` / `*.tf` is touched).
///
/// The `as u32` cast in the runner relies on the order declared here
/// (LintFormat=0 .. IaC=n) — guarded by a unit test.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageKind {
    LintFormat,
    TypeCheck,
    UnitTests,
    E2e,
    DeadCode,
    SupplyChain,
    SecurityScan,
    LayerBoundary,
    Iac,
}

impl StageKind {
    pub fn tag(&self) -> &'static str {
        match self {
            StageKind::LintFormat => "lint_format",
            StageKind::TypeCheck => "type_check",
            StageKind::UnitTests => "unit_tests",
            StageKind::E2e => "e2e",
            StageKind::DeadCode => "dead_code",
            StageKind::SupplyChain => "supply_chain",
            StageKind::SecurityScan => "security_scan",
            StageKind::LayerBoundary => "layer_boundary",
            StageKind::Iac => "iac",
        }
    }

    /// RFC 14 §1 — canonical ordering of the pipeline.
    pub fn pipeline_order() -> &'static [StageKind] {
        &[
            StageKind::LintFormat,
            StageKind::TypeCheck,
            StageKind::UnitTests,
            StageKind::E2e,
            StageKind::DeadCode,
            StageKind::SupplyChain,
            StageKind::SecurityScan,
            StageKind::LayerBoundary,
            StageKind::Iac,
        ]
    }
}

/// RFC 14 §3 — `status` field of a stage summary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    /// All findings resolved (zero or all autoFixed).
    Pass,
    /// Stage produced findings but did not block.
    Warn,
    /// Stage produced blocking findings; downstream stages are skipped.
    Fail,
    /// Stage was skipped per `ValidationMode` (RFC 14 §7) or per
    /// throttling (RFC 14 §4).
    Skipped,
}

impl StageStatus {
    pub fn tag(&self) -> &'static str {
        match self {
            StageStatus::Pass => "pass",
            StageStatus::Warn => "warn",
            StageStatus::Fail => "fail",
            StageStatus::Skipped => "skipped",
        }
    }
}

/// RFC 14 §3 — one finding inside `StageSummary.findings[]`.
/// `file` follows the `path:line` convention so the Reviewer / Repair
/// Engine can jump straight to the offending region.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub file: String,
    pub rule: String,
    /// `None` when the finding has no suggestion (e.g. type error Biome
    /// cannot fix).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// `true` when the stage can fix the finding on a second pass
    /// (Biome `--write`, clippy `--fix`, etc.). The Repair Engine
    /// consumes this flag to pick autofix vs. human review (RFC 15 §2).
    #[serde(default)]
    pub auto_fixable: bool,
}

/// RFC 14 §3 — the structured summary a stage emits. Maps one-to-one to
/// the YAML snippet in the RFC ("stage / status / elapsed / findings /
/// summary"). Persisted verbatim inside the `ValidationReport` payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageSummary {
    pub stage: StageKind,
    pub status: StageStatus,
    pub elapsed_ms: u64,
    pub findings: Vec<Finding>,
    /// One-line human-readable summary the HUD Agent Console renders
    /// verbatim ("2 issues, 1 auto-fixed").
    pub summary: String,
}

/// RFC 14 §7 — `ValidationMode`. Strict is the default; Loose skips Tests;
/// PR mode forces E2E + full unit test coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationMode {
    /// RFC 14 §7 strict — 100% green or no commit.
    #[default]
    Strict,
    /// RFC 14 §7 loose — Lint + TypeCheck only (no Tests, no downstream).
    Loose,
    /// RFC 14 §7 PR — full E2E + branch tests before open.
    Pr,
}

impl ValidationMode {
    pub fn tag(&self) -> &'static str {
        match self {
            ValidationMode::Strict => "strict",
            ValidationMode::Loose => "loose",
            ValidationMode::Pr => "pr",
        }
    }
}

/// RFC 14 — the accumulated report the Validation Engine returns after
/// running (or skipping) every applicable stage. Persisted in Journal;
/// the Repair Engine (RFC 15) consumes the latest `fail`/`critical` row.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationReport {
    pub report_id: Uuid,
    pub diff_id: Uuid,
    pub plan_id: Uuid,
    pub mission_id: Uuid,
    pub generated_at: String, // RFC 3339
    pub mode: ValidationMode,
    /// Stages in `pipeline_order()`, including `Skipped` ones. The HUD
    /// renders this array as the timeline — gaps would be misleading.
    pub stages: Vec<StageSummary>,
    /// Aggregated outcome. `Critical` is raised when the same stage fails
    /// twice in a row (RFC 14 §8); the runner sets this when the
    /// `previous_critical_failure` field of the input matches the
    /// current failing stage.
    pub outcome: ValidationOutcome,
    pub model_id: String,
    pub elapsed_ms: u64,
}

impl ValidationReport {
    pub fn is_pass(&self) -> bool {
        matches!(self.outcome, ValidationOutcome::Pass)
    }

    pub fn failed_stage(&self) -> Option<StageKind> {
        self.stages
            .iter()
            .find(|s| matches!(s.status, StageStatus::Fail))
            .map(|s| s.stage)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationOutcome {
    /// RFC 14 §5 `validation.pass` — every stage Pass or Warn.
    Pass,
    /// RFC 14 §5 `validation.fail` — one stage produced blocking findings.
    Fail,
    /// RFC 14 §8 `validation.fail.critical` — same stage failed twice.
    Critical,
}

impl ValidationOutcome {
    pub fn tag(&self) -> &'static str {
        match self {
            ValidationOutcome::Pass => "pass",
            ValidationOutcome::Fail => "fail",
            ValidationOutcome::Critical => "critical",
        }
    }
}
