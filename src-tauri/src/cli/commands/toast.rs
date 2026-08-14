// Atlas OS — `atlas toast` subcommand (RFC 28 Section F).
//
// Sub-actions gated behind the `toast` Cargo feature:
//
//   * `opencode toast queue   <kind> --title T [--body B] [--deep-link L]
//                                       [--in <OFFSET_SECS>]`
//   * `opencode toast list    [--limit N]`
//   * `opencode toast cancel  <id>`
//
// The CLI only ever reads/writes the SQLite queue — it never dispatches
// the WinRT Toast itself. Dispatch is owned by the `ToastDriver` task
// spawned from `AppState::bootstrap()` (RFC 28 §F.3) which lives in the
// same process as the HUD. The CLI commands are for manual queue edits
// and for tests that introspect `toast_history`.
//
// Output is JSON line-delimited to stdout so downstream pipes / skills
// can consume it.

use std::str::FromStr;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::toast::payload::ToastKind;

#[derive(Subcommand, Debug)]
pub enum ToastSub {
    /// Enqueue a scheduled toast.
    Queue(QueueArgs),
    /// List recent queued toasts (newest first).
    List(ListArgs),
    /// Cancel a pending toast by id.
    Cancel(CancelArgs),
}

#[derive(Args, Debug)]
pub struct QueueArgs {
    /// Toast kind: `model_ready` | `turn_end` | `validation_failed`
    /// | `calendar_reminder` | `critical` | `info`.
    pub kind: String,
    /// Title (short, single-line).
    #[arg(long)]
    pub title: String,
    /// Body (multi-line) — optional.
    #[arg(long)]
    pub body: Option<String>,
    /// Deep-link opencode:// URL — optional. The driver hands this
    /// to WinRT `Action::new("Open", "open", link)` so clicking the
    /// toast deep-links back into the HUD.
    #[arg(long)]
    pub deep_link: Option<String>,
    /// Delay in seconds from now. `0` fires immediately
    /// (the driver takes 5s to notice unless already overdue).
    #[arg(short = 'i', long = "in", default_value_t = 0)]
    pub in_secs: u64,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Max number of rows to print.
    #[arg(short = 'n', long, default_value_t = 20)]
    pub limit: i64,
}

#[derive(Args, Debug)]
pub struct CancelArgs {
    /// Toast queue row id to cancel.
    pub id: i64,
}

pub async fn run(cmd: ToastCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let profile_root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&profile_root)?;
    match cmd.action {
        ToastSub::Queue(a) => queue(&journal, a).await?,
        ToastSub::List(a) => list(&journal, a).await?,
        ToastSub::Cancel(a) => cancel(&journal, a).await?,
    }
    Ok(())
}

async fn queue(journal: &crate::journal::Journal, a: QueueArgs) -> Result<()> {
    let kind = ToastKind::from_str(&a.kind).context("parsing --kind")?;
    let now = chrono::Utc::now().timestamp_millis();
    let fire_at = now + (a.in_secs as i64 * 1000);
    let id = journal.toast_enqueue(
        kind,
        &a.title,
        a.body.as_deref(),
        a.deep_link.as_deref(),
        fire_at,
    )?;
    let out = serde_json::json!({
        "queued": true,
        "id": id,
        "kind": kind.to_string(),
        "fire_at": fire_at,
        "deep_link": a.deep_link,
    });
    println!("{out}");
    Ok(())
}

async fn list(journal: &crate::journal::Journal, a: ListArgs) -> Result<()> {
    let rows = journal.toast_list(a.limit)?;
    if rows.is_empty() {
        println!("[]");
        return Ok(());
    }
    for r in rows {
        let json = serde_json::json!({
            "id": r.id,
            "kind": r.kind.to_string(),
            "title": r.title,
            "body": r.body,
            "deep_link": r.deep_link,
            "fire_at": r.fire_at,
            "status": r.status.to_string(),
            "attempts": r.attempts,
        });
        println!("{json}");
    }
    Ok(())
}

async fn cancel(journal: &crate::journal::Journal, a: CancelArgs) -> Result<()> {
    let removed = journal.toast_cancel(a.id)?;
    let out = serde_json::json!({
        "cancelled": removed,
        "id": a.id,
    });
    println!("{out}");
    Ok(())
}

#[derive(Args, Debug)]
pub struct ToastCmd {
    #[command(subcommand)]
    pub action: ToastSub,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct TestCli {
        #[command(subcommand)]
        action: ToastSub,
    }

    #[test]
    fn parse_queue_minimal() {
        let c = TestCli::parse_from(["test", "queue", "info", "--title", "hello"]);
        match c.action {
            ToastSub::Queue(a) => {
                assert_eq!(a.kind, "info");
                assert_eq!(a.title, "hello");
                assert_eq!(a.body, None);
                assert_eq!(a.deep_link, None);
                assert_eq!(a.in_secs, 0);
            }
            other => panic!("expected Queue, got {other:?}"),
        }
    }

    #[test]
    fn parse_queue_with_offset_and_body() {
        let c = TestCli::parse_from([
            "test",
            "queue",
            "model_ready",
            "--title",
            "T",
            "--body",
            "B",
            "--deep-link",
            "opencode://mission/x",
            "--in",
            "60",
        ]);
        match c.action {
            ToastSub::Queue(a) => {
                assert_eq!(a.kind, "model_ready");
                assert_eq!(a.body.as_deref(), Some("B"));
                assert_eq!(a.deep_link.as_deref(), Some("opencode://mission/x"));
                assert_eq!(a.in_secs, 60);
            }
            other => panic!("expected Queue, got {other:?}"),
        }
    }

    #[test]
    fn parse_list_with_limit() {
        let c = TestCli::parse_from(["test", "list", "--limit", "5"]);
        match c.action {
            ToastSub::List(a) => assert_eq!(a.limit, 5),
            other => panic!("expected List, got {other:?}"),
        }
    }

    #[test]
    fn parse_cancel_by_id() {
        let c = TestCli::parse_from(["test", "cancel", "42"]);
        match c.action {
            ToastSub::Cancel(a) => assert_eq!(a.id, 42),
            other => panic!("expected Cancel, got {other:?}"),
        }
    }

    #[test]
    fn queue_requires_title() {
        let res = TestCli::try_parse_from(["test", "queue", "info"]);
        assert!(res.is_err(), "missing --title should be a clap error");
    }

    #[test]
    fn queue_requires_kind_positional() {
        let res = TestCli::try_parse_from(["test", "queue", "--title", "x"]);
        assert!(res.is_err(), "missing positional `kind` should error");
    }

    #[test]
    fn cancel_requires_id_positional() {
        let res = TestCli::try_parse_from(["test", "cancel"]);
        assert!(res.is_err(), "missing positional `id` should error");
    }
}
