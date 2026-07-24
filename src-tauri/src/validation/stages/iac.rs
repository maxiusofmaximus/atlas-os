// RFC 14 §2 IaC check (terraform validate / docker build) — fires only
// when the diff touches `*.tf` / `Dockerfile` / `docker-compose.yml`.
// Phase 1 has no live IaC toolchain; heuristic returns `Skipped` when
// the gate does not fire.

use super::{pass, skipped, Stage, StageContext};
use crate::validation::types::{StageKind, StageSummary};

pub struct Iac;

impl Stage for Iac {
    fn kind(&self) -> StageKind {
        StageKind::Iac
    }
    fn run(&self, ctx: &StageContext) -> StageSummary {
        if !ctx.touches_any(&[
            ".tf",
            "Dockerfile",
            "docker-compose.yml",
            "docker-compose.yaml",
        ]) {
            return skipped(StageKind::Iac, "no IaC file touched");
        }
        pass(
            StageKind::Iac,
            0,
            "iac: heuristic pass (no live toolchain yet)",
        )
    }
}
