// Atlas OS — Calendar ICS subscription reader (RFC 28 §G READ, v3.1.A).
// Fetches an operator-provided `.ics` URL, parses RFC 5545 VEVENTs through the
// `icalendar` crate, and replaces the `ics_local` busy windows in the journal.

use chrono::{NaiveDate, NaiveTime};
use icalendar::{Calendar, CalendarDateTime, Component, DatePerhapsTime};
use reqwest::Client;

use crate::calendar::error::{CalendarError, Result};
use crate::calendar::payload::BusySource;
use crate::calendar::queue::BusyWindowInput;

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

impl IcsBusy {
    fn as_input(&self) -> BusyWindowInput<'_> {
        BusyWindowInput {
            source: BusySource::IcsLocal,
            external_id: &self.external_id,
            subject: &self.subject,
            body: self.body.as_deref(),
            starts_at: self.starts_at,
            ends_at: self.ends_at,
            weight: self.weight,
        }
    }
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

/// Fetch `url` and replace the `ics_local` busy windows with the events it
/// declares. Returns the number of windows written.
pub async fn sync_ics(journal: &crate::journal::Journal, url: &str) -> Result<usize> {
    let client = Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| CalendarError::Backend(anyhow::Error::from(e)))?;
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

    let rows = parse_ics(&body)?;
    journal
        .busy_window_delete_by_source(BusySource::IcsLocal)
        .map_err(CalendarError::Backend)?;
    for row in &rows {
        journal
            .busy_window_upsert(&row.as_input())
            .map_err(CalendarError::Backend)?;
    }
    Ok(rows.len())
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
        assert_eq!(timed.as_input().source, BusySource::IcsLocal);

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
}
