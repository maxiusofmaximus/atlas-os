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

#[cfg(feature = "calendar-graph")]
use anyhow::Context;

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
    /// Show whether a proactive turn can start now (consults busy windows).
    /// RFC 20 Fase 23 v3.1.2 (research/52).
    Availability {
        /// Estimated turn duration in ms (default 15 min).
        #[arg(short = 'e', long, default_value_t = 900_000)]
        eta_ms: i64,
        /// Busy windows with `weight >= threshold` block the turn.
        #[arg(short = 'w', long, default_value_t = 1.0)]
        weight_threshold: f64,
    },
    /// Authenticate against Microsoft Graph (OAuth device-code) and store the
    /// encrypted refresh token in this profile (RFC 28 §G READ).
    #[cfg(feature = "calendar-graph")]
    Login,
    /// Pull the next week of Graph events into `graph` busy windows.
    #[cfg(feature = "calendar-graph")]
    Sync,
    /// Print the stored Graph token claims (aud/scp/tid) — diagnostics.
    #[cfg(feature = "calendar-graph")]
    Status,
    /// Fetch an ad-hoc `.ics` URL and replace its `ics_local` busy windows.
    #[cfg(feature = "calendar-ics")]
    SyncIcs {
        /// Public `.ics` URL (Outlook/Google/Apple "secret address").
        url: String,
        /// Local namespace for this feed's busy windows.
        #[arg(short = 'n', long, default_value = "adhoc")]
        name: String,
    },
    /// Register (or re-point) a durable `.ics` feed subscription.
    #[cfg(feature = "calendar-ics")]
    Subscribe {
        /// Stable local name for the feed (also the id namespace; no `:`).
        name: String,
        /// Public `.ics` URL.
        url: String,
    },
    /// Remove a subscription (its busy windows are left in place).
    #[cfg(feature = "calendar-ics")]
    Unsubscribe { name: String },
    /// List registered subscriptions with their last sync state.
    #[cfg(feature = "calendar-ics")]
    Subscriptions,
    /// Sync every enabled subscription (conditional GET; `304` = no work).
    #[cfg(feature = "calendar-ics")]
    SyncAll,
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
        CalendarAction::Availability {
            eta_ms,
            weight_threshold,
        } => {
            let journal = Journal::open(&root)?;
            availability(&journal, eta_ms, weight_threshold)
        }
        #[cfg(feature = "calendar-graph")]
        CalendarAction::Login => login(&root).await,
        #[cfg(feature = "calendar-graph")]
        CalendarAction::Sync => {
            let journal = Journal::open(&root)?;
            sync(&journal, &root).await
        }
        #[cfg(feature = "calendar-graph")]
        CalendarAction::Status => {
            let journal = Journal::open(&root)?;
            status(&journal, &root).await
        }
        #[cfg(feature = "calendar-ics")]
        CalendarAction::SyncIcs { url, name } => {
            let journal = Journal::open(&root)?;
            let outcome = crate::calendar::ics_reader::sync_named(
                &journal,
                &name,
                &url,
                crate::calendar::ics_reader::Validators::default(),
            )
            .await?;
            println!(
                "calendar sync-ics[{name}]: {} busy windows `ics_local` actualizados",
                outcome.written
            );
            Ok(())
        }
        #[cfg(feature = "calendar-ics")]
        CalendarAction::Subscribe { name, url } => {
            let journal = Journal::open(&root)?;
            subscribe(&journal, &name, &url)
        }
        #[cfg(feature = "calendar-ics")]
        CalendarAction::Unsubscribe { name } => {
            let journal = Journal::open(&root)?;
            unsubscribe(&journal, &name)
        }
        #[cfg(feature = "calendar-ics")]
        CalendarAction::Subscriptions => {
            let journal = Journal::open(&root)?;
            subscriptions(&journal)
        }
        #[cfg(feature = "calendar-ics")]
        CalendarAction::SyncAll => {
            let journal = Journal::open(&root)?;
            let journal = std::sync::Arc::new(parking_lot::Mutex::new(journal));
            sync_all(&journal).await
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

fn availability(journal: &Journal, eta_ms: i64, weight_threshold: f64) -> Result<()> {
    use crate::planning::availability::{availability_now, Availability, TurnPolicy};

    let policy = TurnPolicy {
        eta_ms,
        weight_threshold,
        ..TurnPolicy::default()
    };
    let now = chrono::Utc::now().timestamp_millis();
    match availability_now(journal, now, &policy)? {
        Availability::RunNow => println!("availability: RUN NOW (free for {eta_ms} ms)"),
        Availability::WaitUntil(ts) => {
            println!("availability: WAIT until {ts} (in {} ms)", ts - now)
        }
        Availability::Blocked => println!("availability: BLOCKED (no free slot within horizon)"),
    }
    Ok(())
}

#[cfg(feature = "calendar-ics")]
fn subscribe(journal: &Journal, name: &str, url: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() || name.contains(':') {
        anyhow::bail!("subscription name must be non-empty and must not contain ':'");
    }
    journal.subscription_upsert(name, url, "ics")?;
    println!("subscribed `{name}` -> {url}");
    Ok(())
}

#[cfg(feature = "calendar-ics")]
fn unsubscribe(journal: &Journal, name: &str) -> Result<()> {
    let removed = journal.subscription_remove(name)?;
    println!(
        "subscription `{name}` {}",
        if removed { "removed" } else { "not found" }
    );
    Ok(())
}

#[cfg(feature = "calendar-ics")]
fn subscriptions(journal: &Journal) -> Result<()> {
    let rows = journal.subscription_list()?;
    if rows.is_empty() {
        println!("no subscriptions (use `atlas calendar subscribe <name> <url>`)");
        return Ok(());
    }
    println!("subscriptions ({}):", rows.len());
    for r in rows {
        println!(
            "  {state} {name:<16} {kind:<5} {sync:<24} {url}",
            state = if r.enabled { "on " } else { "off" },
            name = r.name,
            kind = r.kind,
            sync = r.last_status.unwrap_or_else(|| "never synced".into()),
            url = r.url,
        );
    }
    Ok(())
}

#[cfg(feature = "calendar-ics")]
async fn sync_all(journal: &std::sync::Arc<parking_lot::Mutex<Journal>>) -> Result<()> {
    let results = crate::calendar::ics_reader::sync_all_ics(journal).await?;
    if results.is_empty() {
        println!("no enabled subscriptions to sync");
        return Ok(());
    }
    let (mut total, mut unchanged, mut failed) = (0usize, 0usize, 0usize);
    for r in &results {
        match &r.error {
            Some(e) => {
                failed += 1;
                println!("  `{}`: error: {e}", r.name);
            }
            None if r.not_modified => {
                unchanged += 1;
                println!("  `{}`: unchanged (304)", r.name);
            }
            None => {
                total += r.written;
                println!("  `{}`: {} events", r.name, r.written);
            }
        }
    }
    println!("sync-all: {total} events written, {unchanged} feeds unchanged, {failed} failed");
    Ok(())
}

#[cfg(feature = "calendar-graph")]
const GRAPH_ACCOUNT: &str = "graph:default";

#[cfg(feature = "calendar-graph")]
async fn login(root: &Path) -> Result<()> {
    use crate::calendar::auth;

    let client = reqwest::Client::new();
    let token = auth::login_loopback(&client).await?;
    let refresh = token
        .refresh_token
        .context("Graph no devolvió refresh_token (¿falta offline_access?)")?;

    let key = auth::load_or_create_key(root)?;
    let blob = auth::encrypt_refresh_token(&key, &refresh)?;
    let expires_at = token
        .expires_in
        .map(|s| chrono::Utc::now().timestamp_millis() + (s as i64) * 1000);

    let journal = Journal::open(root)?;
    journal.save_calendar_token(GRAPH_ACCOUNT, &blob, auth::KEY_HINT, expires_at)?;
    println!("Login OK — token cifrado guardado en el perfil.");
    Ok(())
}

#[cfg(feature = "calendar-graph")]
async fn sync(journal: &Journal, root: &Path) -> Result<()> {
    use crate::calendar::{auth, graph_reader};

    let token_row = journal
        .load_calendar_token(GRAPH_ACCOUNT)?
        .context("sin token Graph; ejecuta `atlas calendar login` primero")?;
    let key = auth::load_or_create_key(root)?;
    let refresh = auth::decrypt_refresh_token(&key, &token_row.ciphertext)?;

    let client = reqwest::Client::new();
    let token = auth::refresh_access_token(&client, &refresh).await?;
    let written = graph_reader::sync(journal, &token.access_token).await?;
    println!("calendar sync: {written} busy windows `graph` actualizados");
    Ok(())
}

#[cfg(feature = "calendar-graph")]
async fn status(journal: &Journal, root: &Path) -> Result<()> {
    use crate::calendar::auth;

    let row = journal
        .load_calendar_token(GRAPH_ACCOUNT)?
        .context("sin token Graph; ejecuta `atlas calendar login` primero")?;
    let key = auth::load_or_create_key(root)?;
    let refresh = auth::decrypt_refresh_token(&key, &row.ciphertext)?;
    let client = reqwest::Client::new();
    let token = auth::refresh_access_token(&client, &refresh).await?;
    match auth::decode_claims(&token.access_token) {
        Some(c) => {
            println!("aud: {}", c.aud.unwrap_or_default());
            println!("scp: {}", c.scp.unwrap_or_default());
            println!("tid: {}", c.tid.unwrap_or_default());
            println!("upn: {}", c.upn.unwrap_or_default());
            println!("exp: {}", c.exp.map(|e| e.to_string()).unwrap_or_default());
        }
        None => println!("token válido (formato opaco JWE — sin claims decodables)"),
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

    #[cfg(feature = "calendar-ics")]
    #[test]
    fn subscription_round_trip() {
        let (_dir, journal) = fresh_journal();
        subscribe(&journal, "work", "https://example.com/work.ics").expect("subscribe");
        let rows = journal.subscription_list().expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "work");
        assert_eq!(rows[0].kind, "ics");
        assert!(rows[0].enabled);
        assert!(rows[0].last_status.is_none());

        // re-subscribe re-points the URL, keeps a single row for `work`
        subscribe(&journal, "work", "https://example.com/other.ics").expect("re-subscribe");
        let rows = journal.subscription_list().expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].url, "https://example.com/other.ics");

        journal
            .subscription_record_sync("work", Some("\"e1\""), None, "ok (3 events)")
            .expect("record");
        let row = journal.subscription_get("work").expect("get").unwrap();
        assert_eq!(row.etag.as_deref(), Some("\"e1\""));
        assert_eq!(row.last_status.as_deref(), Some("ok (3 events)"));
        assert!(row.last_sync_ms.is_some());

        assert!(journal
            .subscription_set_enabled("work", false)
            .expect("disable"));
        assert!(!journal.subscription_get("work").unwrap().unwrap().enabled);

        unsubscribe(&journal, "work").expect("unsubscribe");
        assert!(journal.subscription_list().expect("list").is_empty());
    }

    #[cfg(feature = "calendar-ics")]
    #[test]
    fn subscribe_rejects_colon_in_name() {
        let (_dir, journal) = fresh_journal();
        let err = subscribe(&journal, "a:b", "https://x/y.ics").unwrap_err();
        assert!(format!("{err:#}").contains("':'"));
        assert!(journal.subscription_list().expect("list").is_empty());
    }
}
