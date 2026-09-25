// Atlas OS — Sister IDE-in-a-terminal (RFC 20 Phase 8 sub-fase 8.4,
// research 36 SECTOR B 8.4 + A.3, research 28 SECTOR B).
//
// Audit decision (RFC 25 §11, AGENTS.md §4): `ratatui` is DEFERRED — no
// new crates in this sub-fase. It drags `crossterm` + `unicode-width` +
// layout widgets onto the single-binary budget, and research 28 already
// ranks the Tier-1 sister surface as the `terminal-kit` Node sub-package
// (`src/cli-tui/`), not a Rust crate. This module is the owned fallback
// the plan names: a dependency-free Document Model over the SAME Kernel
// Bus data the HUD WebSocket serves (research 28 §B.4: no client state —
// the kernel is source of truth). `SisterSnapshot` + `render_frame` fix
// the shape, so a future `tui` feature (ratatui alternate-screen or the
// `atlas-tui` binary) swaps the renderer without changing callers.
// Poll cadence reuses `MONITOR_POLL_SECS` (research 36 SECTOR C: 5s MVP).

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::core::bus::{BusEvent, BusEventKind};
use crate::journal::Journal;
use crate::monitor::{snapshot_now, HardwareSnapshot};

/// Live bus events retained in the frame footer.
pub const SISTER_FEED_CAP: usize = 20;
/// Mission rows shown before the `… +N more` elision line.
pub const SISTER_MISSIONS_SHOWN: usize = 8;
/// Frame width the renderer pads/truncates rows to (deterministic).
pub const SISTER_FRAME_WIDTH: usize = 72;

/// Kernel Bus WebSocket URL for a HUD port — the same endpoint the
/// SvelteKit HUD dials (research 28 §B.4 bus reuse). The sister frame
/// renders journal state, but a live TUI tails this URL instead.
pub fn ws_url(hud_port: u16) -> String {
    format!("ws://localhost:{hud_port}/ws")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SisterMissionLine {
    pub label: String,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SisterSnapshot {
    pub profile: String,
    pub hud_port: Option<u16>,
    pub missions_total: usize,
    pub recent_missions: Vec<SisterMissionLine>,
    pub plans_total: usize,
    pub cost_usd: f64,
    pub monitor: HardwareSnapshot,
}

pub fn build_snapshot(
    journal: &Journal,
    profile: &str,
    hud_port: Option<u16>,
) -> anyhow::Result<SisterSnapshot> {
    let missions = journal.list_missions().unwrap_or_default();
    let recent_missions = missions
        .iter()
        .rev()
        .take(SISTER_MISSIONS_SHOWN)
        .map(|m| SisterMissionLine {
            label: truncate(&m.label, SISTER_FRAME_WIDTH - 22),
            status: m.status.clone(),
        })
        .collect();
    let plans_total = journal.plan_tail(200).map(|rows| rows.len()).unwrap_or(0);
    let cost_usd = journal.total_model_cost_usd().unwrap_or(0.0);
    let monitor = snapshot_now(cost_usd);
    Ok(SisterSnapshot {
        profile: profile.to_string(),
        hud_port: hud_port.filter(|p| *p != 0),
        missions_total: missions.len(),
        recent_missions,
        plans_total,
        cost_usd,
        monitor,
    })
}

#[derive(Clone, Debug, Default)]
pub struct SisterFeed {
    events: VecDeque<String>,
}

impl SisterFeed {
    pub fn push_event(&mut self, event: &BusEvent) {
        self.events.push_back(describe_event(&event.kind));
        while self.events.len() > SISTER_FEED_CAP {
            self.events.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

fn describe_event(kind: &BusEventKind) -> String {
    match kind {
        BusEventKind::HudServed { hud_port } => format!("hud served :{hud_port}"),
        BusEventKind::HardwareSnapshot { cost_usd, .. } => {
            format!("hardware snapshot ${cost_usd:.4}")
        }
        other => other.tag().to_string(),
    }
}

/// Deterministic pure-text frame: same snapshot + feed always renders the
/// same string (the property the future alternate-screen renderer keeps).
pub fn render_frame(snap: &SisterSnapshot, feed: &SisterFeed) -> String {
    let mut out = String::new();
    out.push_str(&rule('='));
    out.push_str(&pad(&format!(
        "ATLAS SISTER [{}]",
        truncate(&snap.profile, 32)
    )));
    out.push_str(&match snap.hud_port {
        Some(p) => pad(&format!("bus {}  (same WS as HUD)", ws_url(p))),
        None => pad("bus <hud not running>  (start atlas-os-desktop)"),
    });
    out.push_str(&rule('-'));
    out.push_str(&pad(&format!(
        "missions {}  plans {}  cost ${:.4}",
        snap.missions_total, snap.plans_total, snap.cost_usd
    )));
    out.push_str(&pad(&format!("hw {}", snap.monitor.summary_line())));
    out.push_str(&rule('-'));
    if snap.recent_missions.is_empty() {
        out.push_str(&pad("(no missions yet — `atlas mission new \"...\"`)"));
    } else {
        for m in &snap.recent_missions {
            out.push_str(&pad(&format!("[{}] {}", m.status, m.label)));
        }
        if snap.missions_total > snap.recent_missions.len() {
            out.push_str(&pad(&format!(
                "… +{} more",
                snap.missions_total - snap.recent_missions.len()
            )));
        }
    }
    out.push_str(&rule('-'));
    if feed.is_empty() {
        out.push_str(&pad("(bus feed idle — events appear here live)"));
    } else {
        for e in feed.events.iter().rev().take(5) {
            out.push_str(&pad(&format!("› {e}")));
        }
    }
    out.push_str(&rule('='));
    out
}

fn rule(ch: char) -> String {
    format!("+{0}+\n", ch.to_string().repeat(SISTER_FRAME_WIDTH - 2))
}

fn pad(line: &str) -> String {
    let clipped: String = line.chars().take(SISTER_FRAME_WIDTH - 4).collect();
    let fill = SISTER_FRAME_WIDTH - 4 - clipped.chars().count();
    format!("| {clipped}{} |\n", " ".repeat(fill))
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    if max <= 1 {
        return chars.into_iter().take(max).collect();
    }
    chars.into_iter().take(max - 1).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bus::BusEventKind;

    fn sample_snapshot() -> SisterSnapshot {
        SisterSnapshot {
            profile: "default".to_string(),
            hud_port: Some(1420),
            missions_total: 10,
            recent_missions: vec![SisterMissionLine {
                label: "ship it".to_string(),
                status: "open".to_string(),
            }],
            plans_total: 3,
            cost_usd: 1.5,
            monitor: HardwareSnapshot {
                ram_total_mb: 16384,
                ram_used_mb: 8192,
                vram_total_mb: None,
                vram_used_mb: None,
                cpu_pressure: None,
                cumulative_cost_usd: 1.5,
            },
        }
    }

    #[test]
    fn frame_is_deterministic_and_names_bus_and_sections() {
        let snap = sample_snapshot();
        let feed = SisterFeed::default();
        let a = render_frame(&snap, &feed);
        let b = render_frame(&snap, &feed);
        assert_eq!(a, b);
        assert!(a.contains("ATLAS SISTER [default]"));
        assert!(a.contains("ws://localhost:1420/ws"));
        assert!(a.contains("missions 10"));
        assert!(a.contains("[open] ship it"));
        assert!(a.contains("… +9 more"));
        assert!(a.contains("$1.5000"));
        assert!(a.contains("bus feed idle"));
        for line in a.lines() {
            assert_eq!(line.chars().count(), SISTER_FRAME_WIDTH);
        }
    }

    #[test]
    fn frame_without_hud_points_at_desktop_binary() {
        let mut snap = sample_snapshot();
        snap.hud_port = Some(0).filter(|p| *p != 0);
        snap.recent_missions = Vec::new();
        let frame = render_frame(&snap, &SisterFeed::default());
        assert!(frame.contains("hud not running"));
        assert!(frame.contains("atlas mission new"));
    }

    #[test]
    fn feed_caps_oldest_and_renders_newest_first() {
        let mut feed = SisterFeed::default();
        for _ in 0..SISTER_FEED_CAP + 5 {
            feed.push_event(&BusEvent::new(BusEventKind::HudServed { hud_port: 1 }));
        }
        assert_eq!(feed.len(), SISTER_FEED_CAP);
        let frame = render_frame(&sample_snapshot(), &feed);
        assert!(frame.contains("› hud served :1"));
        assert!(!frame.contains("bus feed idle"));
        feed.push_event(&BusEvent::new(BusEventKind::HardwareSnapshot {
            ram_total_mb: 8,
            ram_used_mb: 4,
            vram_total_mb: None,
            vram_used_mb: None,
            cost_usd: 2.0,
        }));
        let frame = render_frame(&sample_snapshot(), &feed);
        assert!(frame.lines().any(|l| l.contains("$2.0000")));
    }

    #[test]
    fn snapshot_on_empty_journal_is_zeroed_but_valid() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = Journal::open(dir.path()).unwrap();
        let snap = build_snapshot(&journal, "thin", None).unwrap();
        assert_eq!(snap.missions_total, 0);
        assert!(snap.recent_missions.is_empty());
        assert_eq!(snap.cost_usd, 0.0);
        assert_eq!(snap.hud_port, None);
        let frame = render_frame(&snap, &SisterFeed::default());
        assert!(frame.contains("ATLAS SISTER [thin]"));
    }

    #[test]
    fn ws_url_matches_hud_convention() {
        assert_eq!(ws_url(1420), "ws://localhost:1420/ws");
    }
}
