// RFC 14 §2 Supply-chain (Socket / Snyk CVE scan) — fires only when the
// diff touches a lockfile (`Cargo.lock` / `pnpm-lock.yaml` /
// `package-lock.json`). Phase 1 has no live Socket/Snyk caller; the
// stage returns `Skipped` when the gate does not fire.

use super::{pass, skipped, Stage, StageContext};
use crate::validation::types::{StageKind, StageSummary};

pub struct SupplyChain;

impl Stage for SupplyChain {
    fn kind(&self) -> StageKind {
        StageKind::SupplyChain
    }
    fn run(&self, ctx: &StageContext) -> StageSummary {
        if !ctx.touches_any(&[
            "Cargo.lock",
            "pnpm-lock.yaml",
            "package-lock.json",
            "yarn.lock",
        ]) {
            return skipped(StageKind::SupplyChain, "no lockfile touched");
        }
        pass(
            StageKind::SupplyChain,
            0,
            "supply-chain: heuristic pass (no live scanner yet)",
        )
    }
}
