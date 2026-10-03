// Atlas OS — Calendar ICS subscription reader (RFC 28 §G READ, v3.1.A).
// Fetches an operator-provided `.ics` URL, parses RFC 5545 VEVENTs through the
// `icalendar` crate, and replaces that feed's `ics_local` busy windows.
//
// Each feed is namespaced by its subscription name (`{name}:{uid}`) so several
// `.ics` subscriptions can coexist in `source = ics_local` without colliding,
// and one feed can be re-synced or dropped without evicting the others. A
// conditional GET (`If-None-Match` / `If-Modified-Since`) short-circuits an
// unchanged feed to a `304` so `calendar sync-all` and the poller stay cheap.
//
// Network I/O (`fetch_ics`) is kept apart from the journal writes (`apply_ics`)
// so the apply path is unit-testable without a server, and so the CLI and the
// background poller can share one loop (`sync_all_ics`) that never holds the
// journal lock across an `.await`.

use std::sync::Arc;

use chrono::{NaiveDate, NaiveTime};
use icalendar::{Calendar, CalendarDateTime, Component, DatePerhapsTime};
use parking_lot::Mutex;
use reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use reqwest::{Client, StatusCode};

use crate::calendar::error::{CalendarError, Result};
use crate::calendar::payload::BusySource;
use crate::calendar::queue::BusyWindowInput;
use crate::journal::Journal;

/// One VEVENT normalised into the busy-window shape (owned strings so it
/// outlives the parsed calendar).
#[derive(Clone, Debug, PartialEq)]
pub struct IcsBusy {
    pub external_id: String,
    pub subject: String,
    pub body: Option<String>,
    pub starts_at: i64,
    pub ends_at: i64,
    pub weight: f64,
}

/// HTTP validators from a previous fetch, replayed as conditional-GET
/// headers so an unchanged feed answers `304 Not Modified`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Validators<'a> {
    pub etag: Option<&'a str>,
    pub last_modified: Option<&'a str>,
}

/// A fetched (or `304`-short-circuited) feed, before it touches the journal.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FetchedIcs {
    pub events: Vec<IcsBusy>,
    pub not_modified: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// Outcome of a single-feed sync.
#[derive(Clone, Debug, PartialEq)]
pub struct SyncOutcome {
    pub written: usize,
    pub not_modified: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// Outcome of one subscription within a `sync-all` run.
#[derive(Clone, Debug, PartialEq)]
pub struct SubSyncResult {
    pub name: String,
    pub written: usize,
    pub not_modified: bool,
    pub error: Option<String>,
}

/// Namespace an event UID under its subscription so several `.ics` feeds
/// can coexist in `source = ics_local` without id collisions.
fn namespaced(name: &str, uid: &str) -> String {
    format!("{name}:{uid}")
}

/// Parse an iCalendar document into busy windows. All-day events (`VALUE=DATE`)
/// get weight 0.0; timed events get weight 1.0. Events without a UID/DTSTART or
/// with an inverted range are skipped rather than failing the whole parse.
pub fn parse_ics(text: &str) -> Result<Vec<IcsBusy>> {
    let calendar: Calendar = text
        .parse::<Calendar>()
        .map_err(|e| CalendarError::Ics(format!("{e:?}")))?;

    let mut out = Vec::new();
    for event in calendar.events() {
        let Some(uid) = event.get_uid() else {
            continue;
        };
        let Some(start) = event.get_start() else {
            continue;
        };
        let end = event.get_end();

        let (starts_at, ends_at, weight) = match (start, end) {
            (DatePerhapsTime::Date(start_date), Some(DatePerhapsTime::Date(end_date))) => (
                date_midnight_ms(start_date),
                date_midnight_ms(end_date),
                0.0,
            ),
            (DatePerhapsTime::Date(start_date), _) => {
                let start_ms = date_midnight_ms(start_date);
                (start_ms, start_ms + 86_400_000, 0.0)
            }
            (DatePerhapsTime::DateTime(start_dt), Some(DatePerhapsTime::DateTime(end_dt))) => {
                (to_epoch_ms(&start_dt), to_epoch_ms(&end_dt), 1.0)
            }
            (DatePerhapsTime::DateTime(start_dt), _) => {
                let start_ms = to_epoch_ms(&start_dt);
                (start_ms, start_ms + 3_600_000, 1.0)
            }
        };
        if starts_at >= ends_at {
            continue;
        }

        out.push(IcsBusy {
            external_id: uid.to_string(),
            subject: event
                .get_summary()
                .map(str::to_string)
                .unwrap_or_else(|| "(busy)".to_string()),
            body: None,
            starts_at,
            ends_at,
            weight,
        });
    }
    Ok(out)
}

fn date_midnight_ms(date: NaiveDate) -> i64 {
    date.and_time(NaiveTime::MIN).and_utc().timestamp_millis()
}

fn to_epoch_ms(dt: &CalendarDateTime) -> i64 {
    match dt {
        CalendarDateTime::Floating(naive) => naive.and_utc().timestamp_millis(),
        CalendarDateTime::Utc(dt) => dt.timestamp_millis(),
        CalendarDateTime::WithTimezone { date_time, .. } => date_time.and_utc().timestamp_millis(),
    }
}

fn header_string(
    response: &reqwest::Response,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    response
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

/// Fetch `url` conditionally and parse it. A `304` yields `not_modified = true`
/// and no events; it is not an error.
pub async fn fetch_ics(url: &str, validators: Validators<'_>) -> Result<FetchedIcs> {
    let client = Client::new();
    let mut req = client.get(url);
    if let Some(etag) = validators.etag {
        req = req.header(IF_NONE_MATCH, etag);
    }
    if let Some(lm) = validators.last_modified {
        req = req.header(IF_MODIFIED_SINCE, lm);
    }
    let response = req
        .send()
        .await
        .map_err(|e| CalendarError::Backend(anyhow::Error::from(e)))?;

    let etag = header_string(&response, ETAG);
    let last_modified = header_string(&response, LAST_MODIFIED);

    if response.status() == StatusCode::NOT_MODIFIED {
        return Ok(FetchedIcs {
            events: Vec::new(),
            not_modified: true,
            etag,
            last_modified,
        });
    }
    if !response.status().is_success() {
        return Err(CalendarError::Ics(format!(
            "ics HTTP {}",
            response.status().as_u16()
        )));
    }
    let body = response
        .text()
        .await
        .map_err(|e| CalendarError::Backend(anyhow::Error::from(e)))?;

    Ok(FetchedIcs {
        events: parse_ics(&body)?,
        not_modified: false,
        etag,
        last_modified,
    })
}

/// Replace subscription `name`'s `ics_local` busy windows with `fetched`
/// (evict by `{name}:` prefix, then UPSERT the namespaced events). Returns the
/// number of windows written. Purely synchronous — no lock is held across an
/// await by callers.
pub fn apply_ics(journal: &Journal, name: &str, fetched: &FetchedIcs) -> anyhow::Result<usize> {
    let prefix = format!("{name}:");
    journal.busy_window_delete_by_source_prefix(BusySource::IcsLocal, &prefix)?;
    for row in &fetched.events {
        let external_id = namespaced(name, &row.external_id);
        let input = BusyWindowInput {
            source: BusySource::IcsLocal,
            external_id: &external_id,
            subject: &row.subject,
            body: row.body.as_deref(),
            starts_at: row.starts_at,
            ends_at: row.ends_at,
            weight: row.weight,
        };
        journal.busy_window_upsert(&input)?;
    }
    Ok(fetched.events.len())
}

/// Fetch and apply a single feed. A `304` is not an error: it yields
/// `written = 0` and `not_modified = true`.
pub async fn sync_named(
    journal: &Journal,
    name: &str,
    url: &str,
    validators: Validators<'_>,
) -> Result<SyncOutcome> {
    let fetched = fetch_ics(url, validators).await?;
    if fetched.not_modified {
        return Ok(SyncOutcome {
            written: 0,
            not_modified: true,
            etag: fetched.etag,
            last_modified: fetched.last_modified,
        });
    }
    let written = apply_ics(journal, name, &fetched).map_err(CalendarError::Backend)?;
    Ok(SyncOutcome {
        written,
        not_modified: false,
        etag: fetched.etag,
        last_modified: fetched.last_modified,
    })
}

/// Ad-hoc entry point used by `atlas calendar sync-ics <url>`: sync a bare URL
/// under the fixed `adhoc` namespace with no conditional state.
pub async fn sync_ics(journal: &Journal, url: &str) -> Result<usize> {
    Ok(sync_named(journal, "adhoc", url, Validators::default())
        .await?
        .written)
}

/// Sync every enabled ICS subscription once, recording each outcome in
/// `calendar_subscriptions`. Shared by the `calendar sync-all` CLI and the
/// background `CalendarPoller`. The journal lock is taken only for the short
/// synchronous reads/writes around each network fetch.
pub async fn sync_all_ics(journal: &Arc<Mutex<Journal>>) -> Result<Vec<SubSyncResult>> {
    let subs = {
        let guard = journal.lock();
        guard.subscription_list().map_err(CalendarError::Backend)?
    };
    let mut out = Vec::new();
    for sub in subs.into_iter().filter(|s| s.enabled && s.kind == "ics") {
        let validators = Validators {
            etag: sub.etag.as_deref(),
            last_modified: sub.last_modified.as_deref(),
        };
        match fetch_ics(&sub.url, validators).await {
            Ok(fetched) => {
                let written = if fetched.not_modified {
                    0
                } else {
                    apply_ics(&journal.lock(), &sub.name, &fetched)
                        .map_err(CalendarError::Backend)?
                };
                let status = if fetched.not_modified {
                    "ok (304 not modified)".to_string()
                } else {
                    format!("ok ({written} events)")
                };
                journal
                    .lock()
                    .subscription_record_sync(
                        &sub.name,
                        fetched.etag.as_deref(),
                        fetched.last_modified.as_deref(),
                        &status,
                    )
                    .map_err(CalendarError::Backend)?;
                out.push(SubSyncResult {
                    name: sub.name,
                    written,
                    not_modified: fetched.not_modified,
                    error: None,
                });
            }
            Err(e) => {
                let msg = e.to_string();
                journal
                    .lock()
                    .subscription_record_sync(&sub.name, None, None, &format!("error: {msg}"))
                    .map_err(CalendarError::Backend)?;
                out.push(SubSyncResult {
                    name: sub.name,
                    written: 0,
                    not_modified: false,
                    error: Some(msg),
                });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    fn epoch(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        DateTime::<Utc>::from_naive_utc_and_offset(
            NaiveDate::from_ymd_opt(y, mo, d)
                .expect("valid date")
                .and_hms_opt(h, mi, 0)
                .expect("valid time"),
            Utc,
        )
        .timestamp_millis()
    }

    fn fresh_journal() -> (tempfile::TempDir, Journal) {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        (tmp, journal)
    }

    const FIXTURE: &str = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
PRODID:-//Atlas//Test//EN\r\n\
BEGIN:VEVENT\r\n\
UID:timed-event@example.com\r\n\
DTSTART:20261003T090000Z\r\n\
DTEND:20261003T100000Z\r\n\
SUMMARY:Timed Event\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
UID:all-day-event@example.com\r\n\
DTSTART;VALUE=DATE:20261004\r\n\
DTEND;VALUE=DATE:20261005\r\n\
SUMMARY:All Day Event\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    #[test]
    fn parse_timed_and_all_day_events() {
        let rows = parse_ics(FIXTURE).expect("parse");
        assert_eq!(rows.len(), 2);

        let timed = rows
            .iter()
            .find(|r| r.external_id == "timed-event@example.com")
            .expect("timed event");
        assert_eq!(timed.subject, "Timed Event");
        assert_eq!(timed.weight, 1.0);
        assert_eq!(timed.starts_at, epoch(2026, 10, 3, 9, 0));
        assert_eq!(timed.ends_at, epoch(2026, 10, 3, 10, 0));

        let all_day = rows
            .iter()
            .find(|r| r.external_id == "all-day-event@example.com")
            .expect("all-day event");
        assert_eq!(all_day.weight, 0.0);
        assert_eq!(all_day.starts_at, epoch(2026, 10, 4, 0, 0));
        assert_eq!(all_day.ends_at, epoch(2026, 10, 5, 0, 0));
    }

    #[test]
    fn skips_inverted_range_and_missing_uid() {
        let ics = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Atlas//EN\r\n\
BEGIN:VEVENT\r\nUID:bad@example.com\r\nDTSTART:20261003T100000Z\r\nDTEND:20261003T090000Z\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\nDTSTART:20261003T100000Z\r\nDTEND:20261003T110000Z\r\nEND:VEVENT\r\n\
END:VCALENDAR\r\n";
        let rows = parse_ics(ics).expect("parse");
        assert!(rows.is_empty());
    }

    #[test]
    fn malformed_document_errors() {
        assert!(matches!(
            parse_ics("{ not ical"),
            Err(CalendarError::Ics(_))
        ));
    }

    #[test]
    fn namespaced_ids_are_scoped_to_the_feed() {
        assert_eq!(namespaced("work", "e1"), "work:e1");
        assert_ne!(namespaced("work", "e1"), namespaced("home", "e1"));
    }

    #[test]
    fn apply_namespaces_feed_and_evicts_only_its_own_rows() {
        let (_dir, journal) = fresh_journal();
        let fetched = |events| FetchedIcs {
            events,
            ..Default::default()
        };

        apply_ics(&journal, "a", &fetched(parse_ics(FIXTURE).unwrap())).unwrap();
        apply_ics(&journal, "b", &fetched(parse_ics(FIXTURE).unwrap())).unwrap();
        assert_eq!(journal.busy_window_count().unwrap(), 4);

        // Re-apply feed `a` with a single event: only `a:` rows are replaced.
        let one = fetched(parse_ics(FIXTURE).unwrap().into_iter().take(1).collect());
        let written = apply_ics(&journal, "a", &one).unwrap();
        assert_eq!(written, 1);

        let rows = journal.busy_window_list(50).unwrap();
        assert_eq!(
            rows.iter()
                .filter(|r| r.external_id.starts_with("a:"))
                .count(),
            1
        );
        assert_eq!(
            rows.iter()
                .filter(|r| r.external_id.starts_with("b:"))
                .count(),
            2
        );
    }

    #[test]
    fn apply_empty_feed_evicts_only_that_feed() {
        let (_dir, journal) = fresh_journal();
        apply_ics(
            &journal,
            "a",
            &FetchedIcs {
                events: parse_ics(FIXTURE).unwrap(),
                ..Default::default()
            },
        )
        .unwrap();
        apply_ics(
            &journal,
            "b",
            &FetchedIcs {
                events: parse_ics(FIXTURE).unwrap(),
                ..Default::default()
            },
        )
        .unwrap();

        assert_eq!(apply_ics(&journal, "a", &FetchedIcs::default()).unwrap(), 0);
        let rows = journal.busy_window_list(50).unwrap();
        assert!(rows.iter().all(|r| r.external_id.starts_with("b:")));
        assert_eq!(rows.len(), 2);
    }
}
