// Atlas OS — `atlas calendar` namespace (RFC 28 §G).
//
// Operator surface over the already-built calendar module:
//
//   opencode calendar feed             → webcal:// URL for the ICS WRITE feed
//   opencode calendar busy list/count  → inspect the busy-window queue
//   opencode calendar busy add/rm      → hand-author `manual` busy windows
//
// The Graph READ path (`calendar-graph`) is still a stub, so this surface
// intentionally needs no Microsoft credentials — busy windows here are the
// operator-authored ones that the Planning engine will consult.

use std::path::Path;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::calendar::payload::BusySource;
use crate::calendar::queue::BusyWindowInput;
use crate::journal::Journal;

#[derive(Args, Debug)]
pub struct CalendarCmd {
    #[command(subcommand)]
    pub action: CalendarAction,
}

#[derive(Subcommand, Debug)]
pub enum CalendarAction {
    /// Print the read-only ICS feed URL to subscribe to (`webcal://…`).
    Feed,
    /// Busy-window queue operations (READ-path storage, no Graph needed).
    Busy {
        #[command(subcommand)]
        action: BusyAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum BusyAction {
    /// List busy windows, newest first.
    List {
        #[arg(short = 'n', long, default_value_t = 20)]
        last: i64,
    },
    /// Count busy windows.
    Count,
    /// Insert a manual busy window (epoch ms, half-open `[start, end)`).
    Add {
        starts_at: i64,
        ends_at: i64,
        #[arg(short = 'l', long, default_value = "manual")]
        label: String,
        #[arg(short = 'w', long, default_value_t = 1.0)]
        weight: f64,
    },
    /// Delete a busy window by id.
    Rm { id: i64 },
}

pub async fn run(cmd: CalendarCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    match cmd.action {
        CalendarAction::Feed => {
            println!("{}", feed_url(read_hud_port(&root)));
            Ok(())
        }
        CalendarAction::Busy { action } => {
            let journal = Journal::open(&root)?;
            busy(&journal, action)
        }
    }
}

/// Read the HUD port persisted by `atlas hud` (`<root>/hud_port.txt`).
fn read_hud_port(root: &Path) -> Option<u16> {
    std::fs::read_to_string(root.join("hud_port.txt"))
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// Pure URL builder so the shape is testable without a running HUD.
pub fn feed_url(port: Option<u16>) -> String {
    match port {
        Some(p) => format!("webcal://127.0.0.1:{p}/atlas-calendar.ics"),
        None => "start the HUD (`atlas hud`) to serve GET /atlas-calendar.ics; then \
                 subscribe to webcal://127.0.0.1:<hud_port>/atlas-calendar.ics"
            .to_string(),
    }
}

fn busy(journal: &Journal, action: BusyAction) -> Result<()> {
    match action {
        BusyAction::List { last } => {
            let n = last.clamp(1, 2048);
            let rows = journal.busy_window_list(n)?;
            println!("busy windows ({}) last {n}:", rows.len());
            for r in rows {
                println!(
                    "  #{id} {start}..{end} w={w:.2} {src:<9} {subj}",
                    id = r.id,
                    start = r.starts_at,
                    end = r.ends_at,
                    w = r.weight,
                    src = r.source.as_str(),
                    subj = r.subject,
                );
            }
        }
        BusyAction::Count => {
            println!("busy windows: {}", journal.busy_window_count()?);
        }
        BusyAction::Add {
            starts_at,
            ends_at,
            label,
            weight,
        } => {
            let external_id = format!("manual-{}", uuid::Uuid::new_v4());
            let input = BusyWindowInput {
                source: BusySource::Manual,
                external_id: external_id.as_str(),
                subject: label.as_str(),
                body: None,
                starts_at,
                ends_at,
                weight: weight.clamp(0.0, 1.0),
            };
            let id = journal.busy_window_insert_manual(&input)?;
            println!("inserted busy window #{id} [{starts_at}, {ends_at}) label={label}");
        }
        BusyAction::Rm { id } => {
            let removed = journal.busy_window_delete(id)?;
            println!(
                "busy window #{id} {}",
                if removed { "removed" } else { "not found" }
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh_journal() -> (TempDir, Journal) {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        (tmp, journal)
    }

    #[test]
    fn feed_url_uses_port_when_present() {
        assert_eq!(
            feed_url(Some(8090)),
            "webcal://127.0.0.1:8090/atlas-calendar.ics"
        );
        assert!(feed_url(None).contains("atlas hud"));
    }

    #[test]
    fn busy_add_list_count_rm_round_trip() {
        let (_dir, journal) = fresh_journal();
        busy(
            &journal,
            BusyAction::Add {
                starts_at: 1_000,
                ends_at: 2_000,
                label: "focus".into(),
                weight: 0.5,
            },
        )
        .expect("add #1");
        busy(
            &journal,
            BusyAction::Add {
                starts_at: 3_000,
                ends_at: 4_000,
                label: "standup".into(),
                weight: 1.0,
            },
        )
        .expect("add #2");

        assert_eq!(journal.busy_window_count().expect("count"), 2);
        let rows = journal.busy_window_list(10).expect("list");
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.source == BusySource::Manual));

        let id = rows[0].id;
        busy(&journal, BusyAction::Rm { id }).expect("rm");
        assert_eq!(journal.busy_window_count().expect("count after rm"), 1);
    }

    #[test]
    fn busy_add_rejects_inverted_range() {
        let (_dir, journal) = fresh_journal();
        let err = busy(
            &journal,
            BusyAction::Add {
                starts_at: 5_000,
                ends_at: 4_000,
                label: "bad".into(),
                weight: 1.0,
            },
        )
        .unwrap_err();
        assert!(format!("{err:#}").to_lowercase().contains("range"));
        assert_eq!(journal.busy_window_count().expect("count"), 0);
    }

    #[test]
    fn busy_windows_overlapping_is_half_open() {
        let (_dir, journal) = fresh_journal();
        busy(
            &journal,
            BusyAction::Add {
                starts_at: 1_000,
                ends_at: 2_000,
                label: "a".into(),
                weight: 1.0,
            },
        )
        .expect("add");

        let hit = journal
            .busy_windows_overlapping(1_500, 1_600)
            .expect("overlap");
        assert_eq!(hit.len(), 1);
        let touching = journal
            .busy_windows_overlapping(2_000, 3_000)
            .expect("half-open");
        assert!(touching.is_empty(), "event ending at t must not block t");
    }
}
