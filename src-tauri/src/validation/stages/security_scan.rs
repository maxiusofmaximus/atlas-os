// RFC 14 §2 Security scan (Semgrep / CodeQL) — external lateral-only
// (RFC 20 Fase 12 sub-fase 12.0). The pipeline stage itself stays
// fail-safe `Skipped` (never blocks on a missing tool, no IO in the
// runner); the real launch + parse lives in
// `validation::stages::static_analysis` behind `atlas validate
// --semgrep/--codeql`, which emits an M32 `AuditReport`.

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
            "security_scan is external-only (RFC 20 Fase 12.0): run `atlas validate --semgrep/--codeql`; pipeline never blocks on a missing tool",
        )
    }
}
