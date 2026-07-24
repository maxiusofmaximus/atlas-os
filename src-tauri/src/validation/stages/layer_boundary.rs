// RFC 14 §2 Layer-boundary check (IaC vs runtime, server vs client).
// Phase 1 has no boundary linter; skipped. The real analyser lands
// alongside the LSP host (RFC 20) in Phase 2.

use super::{skipped, Stage, StageContext};
use crate::validation::types::{StageKind, StageSummary};

pub struct LayerBoundary;

impl Stage for LayerBoundary {
    fn kind(&self) -> StageKind {
        StageKind::LayerBoundary
    }
    fn run(&self, _ctx: &StageContext) -> StageSummary {
        skipped(
            StageKind::LayerBoundary,
            "layer-boundary linter not enabled in Phase 1",
        )
    }
}
