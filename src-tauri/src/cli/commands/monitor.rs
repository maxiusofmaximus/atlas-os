// Atlas OS — `atlas monitor` VRAM/RAM/cost snapshot (RFC 20 Phase 8
// sub-fase 8.2, research 36 SECTOR B 8.2).
//
// Thin verb over `crate::monitor`: reads the best-effort snapshot
// (std-only, fail-safe `None` when `nvidia-smi`/wmic is missing),
// joins the cumulative `model_invocations.cost_usd` tail, prints one
// HUD-card line, and publishes `BusEventKind::HardwareSnapshot` so the
// WS tail can project it without a new tail route.

use anyhow::Result;
use clap::Args;

use crate::monitor::{
    snapshot_now, MONITOR_COST_CRIT_USD, MONITOR_COST_WARN_USD, MONITOR_RAM_CRIT_PRESSURE,
    MONITOR_RAM_WARN_PRESSURE,
};

#[derive(Args, Debug)]
pub struct MonitorCmd {
    /// RAM pressure in [0.0, 1.0] above which the line reports WARN.
    #[arg(long, default_value_t = MONITOR_RAM_WARN_PRESSURE)]
    pub ram_warn: f64,
    /// RAM pressure in [0.0, 1.0] above which the line reports CRITICAL.
    #[arg(long, default_value_t = MONITOR_RAM_CRIT_PRESSURE)]
    pub ram_crit: f64,
    /// Cumulative cost USD above which the line reports WARN.
    #[arg(long, default_value_t = MONITOR_COST_WARN_USD)]
    pub cost_warn: f64,
    /// Cumulative cost USD above which the line reports CRITICAL.
    #[arg(long, default_value_t = MONITOR_COST_CRIT_USD)]
    pub cost_crit: f64,
    /// Skip publishing the snapshot onto the Kernel Bus journal tail.
    #[arg(long, default_value_t = false)]
    pub no_publish: bool,
}

pub async fn run(cmd: MonitorCmd, profile: &str) -> Result<()> {
    if !(0.0..=1.0).contains(&cmd.ram_warn) || !(0.0..=1.0).contains(&cmd.ram_crit) {
        anyhow::bail!(
            "--ram-warn/--ram-crit must be within [0.0, 1.0], got {:.3}/{:.3}",
            cmd.ram_warn,
            cmd.ram_crit
        );
    }
    if cmd.ram_warn > cmd.ram_crit {
        anyhow::bail!(
            "--ram-warn ({:.3}) must be <= --ram-crit ({:.3})",
            cmd.ram_warn,
            cmd.ram_crit
        );
    }
    snapshot(cmd, profile)
}

fn snapshot(cmd: MonitorCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    let cumulative = journal.total_model_cost_usd().unwrap_or(0.0);
    let snap = snapshot_now(cumulative);
    let ram_level = snap.ram_level_with_thresholds(cmd.ram_warn, cmd.ram_crit);
    let cost_level = snap.cost_level_with_thresholds(cmd.cost_warn, cmd.cost_crit);
    let vram_level = snap.vram_level();
    println!(
        "monitor [{pid}] {} ram={:?} vram={:?} cost={:?}",
        snap.summary_line(),
        ram_level,
        vram_level,
        cost_level,
    );
    if cmd.no_publish {
        return Ok(());
    }
    journal.publish(&crate::core::bus::BusEvent::new(
        crate::core::bus::BusEventKind::HardwareSnapshot {
            ram_total_mb: snap.ram_total_mb,
            ram_used_mb: snap.ram_used_mb,
            vram_total_mb: snap.vram_total_mb,
            vram_used_mb: snap.vram_used_mb,
            cost_usd: snap.cumulative_cost_usd,
        },
    ))?;
    Ok(())
}
