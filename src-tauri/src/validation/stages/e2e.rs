// RFC 14 §2 E2E (Playwright) — runs only when UI is touched OR when
// `ValidationMode::Pr` is active. Phase 1 has no real Playwright harness;
// the heuristic returns `Skipped` when the conditions don't apply and
// `Pass` (with a recording marker) when they do. The Repair Engine's
// real integration arrives with RFC 15 + Phase 2.

use super::{pass, skipped, Stage, StageContext};
use crate::validation::types::{StageKind, StageSummary};

pub struct E2e;

impl Stage for E2e {
    fn kind(&self) -> StageKind {
        StageKind::E2e
    }
    fn run(&self, ctx: &StageContext) -> StageSummary {
        let ui_touched = ctx.touches_any(&[".svelte", ".tsx", ".vue", "playwright.config"]);
        let pr_mode = matches!(ctx.mode, crate::validation::types::ValidationMode::Pr);
        if !ui_touched && !pr_mode {
            return skipped(StageKind::E2e, "no UI file touched and not PR mode");
        }
        pass(
            StageKind::E2e,
            0,
            "e2e: heuristic pass (no real harness yet)",
        )
    }
}
