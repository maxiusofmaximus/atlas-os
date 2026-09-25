// Atlas OS — VRAM/RAM/cost monitor (RFC 20 Phase 8 sub-fase 8.2,
// research 36 SECTOR B 8.2 + A.3).
//
// Audit decision (RFC 25 §11, AGENTS.md §4): `sysinfo` + `nvml-wrapper`
// are DEFERRED — no new crates in this sub-fase. Both are MIT and would
// give exact RAM/CPU (sysinfo) and NVML VRAM (nvml-wrapper), but each
// drags transitive deps + binary weight onto the single-binary budget,
// so they need the full §11 audit (transitive tree, MSRV vs 1.84,
// Windows NVML fail-safe) before adoption. This module is the fallback
// branch research 36 already names: an owned std-only monitor
// (`std::process` wmic/nvidia-smi best-effort, fail-safe `None`) with
// the `HardwareSnapshot` shape fixed, so a future `hardware-monitor`
// feature (default off) can swap the readers without changing callers.
// Poll cadence 5s is enough for the MVP (research 36 SECTOR C).

use serde::{Deserialize, Serialize};

/// RAM pressure above which the snapshot is `Warn`.
pub const MONITOR_RAM_WARN_PRESSURE: f64 = 0.85;
/// RAM pressure above which the snapshot is `Critical`.
pub const MONITOR_RAM_CRIT_PRESSURE: f64 = 0.95;
/// VRAM pressure above which the snapshot is `Warn` (`None` → `Ok`).
pub const MONITOR_VRAM_WARN_PRESSURE: f64 = 0.85;
/// VRAM pressure above which the snapshot is `Critical`.
pub const MONITOR_VRAM_CRIT_PRESSURE: f64 = 0.95;
/// Cumulative `model_invocations.cost_usd` above which cost is `Warn`.
pub const MONITOR_COST_WARN_USD: f64 = 5.0;
/// Cumulative cost above which cost is `Critical`.
pub const MONITOR_COST_CRIT_USD: f64 = 20.0;
/// HUD poll cadence for the monitor card (research 36 SECTOR C: 5s MVP).
pub const MONITOR_POLL_SECS: u64 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pressure {
    Ok,
    Warn,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CostSummary {
    pub total_cost_usd: f64,
    pub invocations: usize,
    pub mean_cost_usd: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HardwareSnapshot {
    pub ram_total_mb: u64,
    pub ram_used_mb: u64,
    pub vram_total_mb: Option<u64>,
    pub vram_used_mb: Option<u64>,
    pub cpu_pressure: Option<f64>,
    pub cumulative_cost_usd: f64,
}

pub fn pressure_ratio(used_mb: u64, total_mb: u64) -> f64 {
    if total_mb == 0 {
        return 0.0;
    }
    (used_mb as f64 / total_mb as f64).clamp(0.0, 1.0)
}

pub fn classify_pressure(ratio: f64, warn: f64, crit: f64) -> Pressure {
    if ratio >= crit {
        Pressure::Critical
    } else if ratio >= warn {
        Pressure::Warn
    } else {
        Pressure::Ok
    }
}

pub fn classify_cost(total_usd: f64) -> Pressure {
    classify_pressure(total_usd, MONITOR_COST_WARN_USD, MONITOR_COST_CRIT_USD)
}

pub fn summarize_cost(costs: &[f64]) -> CostSummary {
    let total: f64 = costs.iter().copied().sum();
    let n = costs.len();
    CostSummary {
        total_cost_usd: total,
        invocations: n,
        mean_cost_usd: if n == 0 { 0.0 } else { total / n as f64 },
    }
}

impl HardwareSnapshot {
    pub fn ram_pressure(&self) -> f64 {
        pressure_ratio(self.ram_used_mb, self.ram_total_mb)
    }

    pub fn vram_pressure(&self) -> Option<f64> {
        match (self.vram_used_mb, self.vram_total_mb) {
            (Some(used), Some(total)) if total > 0 => Some(pressure_ratio(used, total)),
            _ => None,
        }
    }

    pub fn ram_level(&self) -> Pressure {
        self.ram_level_with_thresholds(MONITOR_RAM_WARN_PRESSURE, MONITOR_RAM_CRIT_PRESSURE)
    }

    pub fn ram_level_with_thresholds(&self, warn: f64, crit: f64) -> Pressure {
        classify_pressure(self.ram_pressure(), warn, crit)
    }

    pub fn vram_level(&self) -> Pressure {
        match self.vram_pressure() {
            Some(p) => classify_pressure(p, MONITOR_VRAM_WARN_PRESSURE, MONITOR_VRAM_CRIT_PRESSURE),
            None => Pressure::Ok,
        }
    }

    pub fn cost_level(&self) -> Pressure {
        self.cost_level_with_thresholds(MONITOR_COST_WARN_USD, MONITOR_COST_CRIT_USD)
    }

    pub fn cost_level_with_thresholds(&self, warn_usd: f64, crit_usd: f64) -> Pressure {
        classify_pressure(self.cumulative_cost_usd, warn_usd, crit_usd)
    }

    pub fn needs_attention(&self) -> bool {
        self.ram_level() != Pressure::Ok
            || self.vram_level() != Pressure::Ok
            || self.cost_level() != Pressure::Ok
    }

    pub fn summary_line(&self) -> String {
        let vram = match (self.vram_used_mb, self.vram_total_mb) {
            (Some(used), Some(total)) => format!("{used}/{total}MB"),
            _ => "n/a".to_string(),
        };
        format!(
            "ram {}/{}MB ({:.0}%) vram {} cost ${:.4}",
            self.ram_used_mb,
            self.ram_total_mb,
            self.ram_pressure() * 100.0,
            vram,
            self.cumulative_cost_usd,
        )
    }
}

pub fn read_ram_mb() -> Option<(u64, u64)> {
    #[cfg(target_os = "windows")]
    {
        read_ram_mb_windows().or(read_ram_mb_fallback_env())
    }
    #[cfg(target_os = "linux")]
    {
        read_ram_mb_linux().or(read_ram_mb_fallback_env())
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        read_ram_mb_fallback_env()
    }
}

#[cfg(target_os = "windows")]
fn read_ram_mb_windows() -> Option<(u64, u64)> {
    read_ram_mb_wmic().or_else(read_ram_mb_powershell)
}

#[cfg(target_os = "windows")]
fn read_ram_mb_wmic() -> Option<(u64, u64)> {
    let out = std::process::Command::new("wmic")
        .args([
            "OS",
            "get",
            "TotalVisibleMemorySize,FreePhysicalMemory",
            "/Value",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_wmic_mem(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(target_os = "windows")]
fn read_ram_mb_powershell() -> Option<(u64, u64)> {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-CimInstance Win32_OperatingSystem).TotalVisibleMemorySize; (Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_cim_mem_lines(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(any(target_os = "windows", test))]
fn parse_cim_mem_lines(text: &str) -> Option<(u64, u64)> {
    let mut nums = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter_map(|l| l.parse::<u64>().ok());
    let total_kb = nums.next()?;
    let free_kb = nums.next()?;
    if total_kb == 0 {
        return None;
    }
    let total_mb = total_kb / 1024;
    let used_mb = total_kb.saturating_sub(free_kb) / 1024;
    Some((total_mb, used_mb.min(total_mb)))
}

#[cfg(target_os = "linux")]
fn read_ram_mb_linux() -> Option<(u64, u64)> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    parse_proc_meminfo(&text)
}

fn read_ram_mb_fallback_env() -> Option<(u64, u64)> {
    let total = std::env::var("ATLAS_MONITOR_RAM_TOTAL_MB")
        .ok()?
        .parse::<u64>()
        .ok()?;
    let used = std::env::var("ATLAS_MONITOR_RAM_USED_MB")
        .ok()?
        .parse::<u64>()
        .ok()?;
    if total == 0 {
        return None;
    }
    Some((total, used.min(total)))
}

#[cfg(any(target_os = "windows", test))]
fn parse_wmic_mem(text: &str) -> Option<(u64, u64)> {
    let mut total_kb: Option<u64> = None;
    let mut free_kb: Option<u64> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("TotalVisibleMemorySize=") {
            total_kb = v.trim().parse().ok();
        } else if let Some(v) = line.strip_prefix("FreePhysicalMemory=") {
            free_kb = v.trim().parse().ok();
        }
    }
    let total_kb = total_kb?;
    let free_kb = free_kb?;
    if total_kb == 0 {
        return None;
    }
    let total_mb = total_kb / 1024;
    let used_mb = total_kb.saturating_sub(free_kb) / 1024;
    Some((total_mb, used_mb.min(total_mb)))
}

#[cfg(any(target_os = "linux", test))]
fn parse_proc_meminfo(text: &str) -> Option<(u64, u64)> {
    let mut total_kb: Option<u64> = None;
    let mut avail_kb: Option<u64> = None;
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("MemTotal:") => total_kb = parts.next()?.parse().ok(),
            Some("MemAvailable:") => avail_kb = parts.next()?.parse().ok(),
            _ => {}
        }
    }
    let total_kb = total_kb?;
    let avail_kb = avail_kb.unwrap_or(0);
    if total_kb == 0 {
        return None;
    }
    let total_mb = total_kb / 1024;
    let used_mb = total_kb.saturating_sub(avail_kb) / 1024;
    Some((total_mb, used_mb.min(total_mb)))
}

pub fn read_vram_mb() -> Option<(u64, u64)> {
    let out = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.total,memory.used",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_nvidia_smi(&String::from_utf8_lossy(&out.stdout))
}

fn parse_nvidia_smi(text: &str) -> Option<(u64, u64)> {
    let first = text.lines().map(str::trim).find(|l| !l.is_empty())?;
    let mut parts = first.split(',');
    let total: u64 = parts.next()?.trim().parse().ok()?;
    let used: u64 = parts.next()?.trim().parse().ok()?;
    if total == 0 {
        return None;
    }
    Some((total, used.min(total)))
}

pub fn snapshot_now(cumulative_cost_usd: f64) -> HardwareSnapshot {
    let (ram_total_mb, ram_used_mb) = read_ram_mb().unwrap_or((0, 0));
    let (vram_total_mb, vram_used_mb) = match read_vram_mb() {
        Some((total, used)) => (Some(total), Some(used)),
        None => (None, None),
    };
    HardwareSnapshot {
        ram_total_mb,
        ram_used_mb,
        vram_total_mb,
        vram_used_mb,
        cpu_pressure: None,
        cumulative_cost_usd: cumulative_cost_usd.max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(ram_used: u64, ram_total: u64, cost: f64) -> HardwareSnapshot {
        HardwareSnapshot {
            ram_total_mb: ram_total,
            ram_used_mb: ram_used,
            vram_total_mb: None,
            vram_used_mb: None,
            cpu_pressure: None,
            cumulative_cost_usd: cost,
        }
    }

    #[test]
    fn ram_threshold_fires_only_above_warn() {
        assert_eq!(
            snap(84, 100, 0.0).ram_level_with_thresholds(0.85, 0.95),
            Pressure::Ok
        );
        assert_eq!(
            snap(85, 100, 0.0).ram_level_with_thresholds(0.85, 0.95),
            Pressure::Warn
        );
        assert_eq!(
            snap(95, 100, 0.0).ram_level_with_thresholds(0.85, 0.95),
            Pressure::Critical
        );
    }

    #[test]
    fn cost_threshold_threads_through_cli_flags() {
        assert_eq!(
            snap(0, 0, 4.99).cost_level_with_thresholds(5.0, 20.0),
            Pressure::Ok
        );
        assert_eq!(
            snap(0, 0, 5.0).cost_level_with_thresholds(5.0, 20.0),
            Pressure::Warn
        );
        assert_eq!(
            snap(0, 0, 20.0).cost_level_with_thresholds(5.0, 20.0),
            Pressure::Critical
        );
    }

    #[test]
    fn zero_total_means_zero_pressure_without_panic() {
        assert_eq!(pressure_ratio(10, 0), 0.0);
        assert_eq!(snap(0, 0, 0.0).ram_pressure(), 0.0);
        assert_eq!(snap(0, 0, 0.0).ram_level(), Pressure::Ok);
        assert_eq!(snap(0, 0, 0.0).vram_level(), Pressure::Ok);
        assert!(!snap(0, 0, 0.0).needs_attention());
    }

    #[test]
    fn vram_none_is_ok_but_over_threshold_warns() {
        let mut s = snap(10, 100, 0.0);
        assert_eq!(s.vram_pressure(), None);
        assert_eq!(s.vram_level(), Pressure::Ok);
        s.vram_total_mb = Some(100);
        s.vram_used_mb = Some(90);
        assert_eq!(s.vram_level(), Pressure::Warn);
        s.vram_used_mb = Some(96);
        assert_eq!(s.vram_level(), Pressure::Critical);
        assert!(s.needs_attention());
    }

    #[test]
    fn summarize_cost_sums_and_means() {
        let empty = summarize_cost(&[]);
        assert_eq!(empty.total_cost_usd, 0.0);
        assert_eq!(empty.invocations, 0);
        assert_eq!(empty.mean_cost_usd, 0.0);
        let s = summarize_cost(&[0.01, 0.03]);
        assert!((s.total_cost_usd - 0.04).abs() < 1e-12);
        assert_eq!(s.invocations, 2);
        assert!((s.mean_cost_usd - 0.02).abs() < 1e-12);
    }

    #[test]
    fn wmic_and_meminfo_parsers_agree_on_shape() {
        let wmic = "FreePhysicalMemory=8388608\r\nTotalVisibleMemorySize=16777216\r\n";
        assert_eq!(parse_wmic_mem(wmic), Some((16384, 8192)));
        assert_eq!(parse_wmic_mem("garbage"), None);
        assert_eq!(
            parse_wmic_mem("TotalVisibleMemorySize=0\nFreePhysicalMemory=0\n"),
            None
        );
        let meminfo = "MemTotal:        16384000 kB\nMemAvailable:     8192000 kB\n";
        assert_eq!(parse_proc_meminfo(meminfo), Some((16000, 8000)));
        assert_eq!(parse_proc_meminfo("MemTotal: 0 kB\n"), None);
    }

    #[test]
    fn nvidia_smi_takes_first_gpu_and_rejects_zero() {
        assert_eq!(parse_nvidia_smi("8192, 4096\n"), Some((8192, 4096)));
        assert_eq!(parse_nvidia_smi(""), None);
        assert_eq!(parse_nvidia_smi("0, 0\n"), None);
        assert_eq!(parse_nvidia_smi("not-a-gpu\n"), None);
    }

    #[test]
    fn cim_powershell_fallback_parses_two_kb_lines() {
        assert_eq!(
            parse_cim_mem_lines("16777216\r\n8388608\r\n"),
            Some((16384, 8192))
        );
        assert_eq!(parse_cim_mem_lines("garbage\n"), None);
        assert_eq!(parse_cim_mem_lines("0\n0\n"), None);
        assert_eq!(parse_cim_mem_lines("16777216\n"), None);
    }

    #[test]
    fn snapshot_summary_line_is_stable() {
        let s = snap(8192, 16384, 1.5);
        let line = s.summary_line();
        assert!(line.contains("8192/16384MB"));
        assert!(line.contains("50%"));
        assert!(line.contains("vram n/a"));
        assert!(line.contains("$1.5000"));
    }

    #[test]
    fn total_model_cost_is_zero_on_empty_journal() {
        let dir = tempfile::TempDir::new().unwrap();
        let j = crate::journal::Journal::open(dir.path()).unwrap();
        assert_eq!(j.total_model_cost_usd().unwrap(), 0.0);
    }
}
