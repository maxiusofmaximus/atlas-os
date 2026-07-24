// RFC 14 §2 Security scan (Semgrep / CodeQL) — fires only when
// `security_gate=true` opt-in (Phase 1: skipped). Real integration lands
// Phase 9 per the Roadmap.

use super::{skipped, Stage, StageContext};
use crate::validation::types::{StageKind, StageSummary};

pub struct SecurityScan;

impl Stage for SecurityScan {
    fn kind(&self) -> StageKind {
        StageKind::SecurityScan
    }
    fn run(&self, _ctx: &StageContext) -> StageSummary {
        skipped(
            StageKind::SecurityScan,
            "security_gate not enabled in Phase 1",
        )
    }
}
