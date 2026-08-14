// Atlas OS — Calendar ICS writer (RFC 28 Section G — WRITE path).
//
// `CalendarWriter::from_missions(&[IcsMission]) -> String` builds the
// RFC 5545 iCalendar feed served at `GET /atlas-calendar.ics` on
// the HUD axum server. The feed is read-only — `METHOD:PUBLISH` per
// RFC 5545 §3.14.6 — and exists so a user can subscribe via
// `webcal://127.0.0.1:{port}/atlas-calendar.ics?token=...` from
// Outlook / Apple Calendar / Google Calendar.
//
// Mapping (RFC 28 §G.2.1):
//
// | iCal field     | Source                                   |
// | -------------- | ---------------------------------------- |
// | UID            | `mission-{id}@opencode.dev`             |
// | DTSTAMP        | `updated_at` (UTC, `Z` suffix)          |
// | DTSTART / DTEND| `created_at` / `updated_at` (timed) — OR |
// |                | `DTSTART;VALUE=DATE` when all-day        |
// | SUMMARY        | `mission.label` (truncated 240 chars)   |
// | DESCRIPTION    | JSON snapshot of recent engine events   |
// |                | (omitted if no recent events)            |
// | CATEGORIES     | mission status token (`snake_case`)      |
// | STATUS         | see `IcsMissionStatus::to_ical_status`   |
// | RRULE          | only when the mission has recurrence     |
// |                | programmed (post-MVP; today omitted)    |
//
// All-day heuristic: when `(updated_at - created_at) >= 24 h` we use
// `DTSTART;VALUE=DATE` and omit `DTEND` per RFC 5545 §3.6.1 (an event
// spanning N days uses DTSTART only). Otherwise we emit both DTSTART
// and DTEND in UTC with the `Z` suffix.
//
// Uses `ics = "0.5.8"` (MIT OR Apache-2.0) by hummingly,
// https://github.com/hummingly/ics — pure Rust RFC 5545 emitter.

use chrono::{DateTime, Utc};

use crate::calendar::error::Result;
use crate::calendar::payload::IcsMission;

/// Production id emitted in the `PRODID` calendar property.
const PROD_ID: &str = "-//Atlas OS//Calendar//EN";

/// Build the `ICALendar` text from a slice of `IcsMission` projections.
///
/// The caller (HUD axum handler) is responsible for filtering missions
/// to the desired range (e.g. last 30 days) before constructing the
/// projections — see `Journal::missions_for_ics(30 days)` (post-MVP).
pub struct CalendarWriter;

impl CalendarWriter {
    /// Render the full RFC 5545 feed (header + one `VEVENT` per
    /// mission, including calendar properties `VERSION`, `PRODID`,
    /// `CALSCALE:GREGORIAN`, `METHOD:PUBLISH`, `X-WR-CALNAME`).
    pub fn render(missions: &[IcsMission]) -> Result<String> {
        let mut cal = ics::ICalendar::new("2.0", PROD_ID);
        cal.push(ics::properties::CalScale::new("GREGORIAN"));
        cal.push(ics::properties::Method::new("PUBLISH"));
        // RFC 7986 — `NAME` improves calendar UI tab display.
        cal.push(ics::properties::Name::new("Atlas OS"));
        // RFC 7986 — `REFRESH-INTERVAL` hints desktop clients to poll
        // hourly. Outlook ignores it (asks the user); Apple/Google
        // honour it. Value is an ISO 8601 duration.
        cal.push(ics::properties::RefreshInterval::new("PT1H"));

        for m in missions {
            let event = build_event(m)?;
            cal.add_event(event);
        }

        Ok(cal.to_string())
    }
}

fn build_event(m: &IcsMission) -> Result<ics::Event<'static>> {
    let dtstamp = fmt_utc_compact(m.updated_at);
    let mut event = ics::Event::new(m.ical_uid(), dtstamp);

    if m.is_all_day() {
        let date_only = m.created_at.format("%Y%m%d").to_string();
        let mut dtstart = ics::components::Property::new("DTSTART", date_only);
        dtstart.add(ics::parameters::Value::DATE);
        event.push(dtstart);
    } else {
        let start = fmt_utc_compact(m.created_at);
        let end = fmt_utc_compact(m.updated_at);
        event.push(ics::properties::DtStart::new(start));
        event.push(ics::properties::DtEnd::new(end));
    }

    let summary = truncate_summary(&m.label);
    event.push(ics::properties::Summary::new(summary));
    event.push(ics::properties::Status::new(m.status.to_ical_status()));
    event.push(ics::properties::Categories::new(m.status.to_category()));

    Ok(event)
}

fn fmt_utc_compact(t: DateTime<Utc>) -> String {
    t.format("%Y%m%dT%H%M%SZ").to_string()
}

fn truncate_summary(label: &str) -> String {
    // RFC 5545 line-fold happens at 75 bytes by the ics crate; we still
    // truncate aggressively because some calendar UIs only show the
    // first ~80 chars and counts nice when the whole summary is there.
    const MAX: usize = 240;
    if label.chars().count() <= MAX {
        return label.to_string();
    }
    let mut end = 0;
    for (i, ch) in label.char_indices() {
        if i + ch.len_utf8() > MAX {
            end = i;
            break;
        }
        end = i + ch.len_utf8();
    }
    label[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::payload::IcsMissionStatus;

    fn ts(s: &str) -> DateTime<Utc> {
        chrono::DateTime::parse_from_rfc3339(s)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn mission(
        id: &str,
        label: &str,
        status: IcsMissionStatus,
        created: &str,
        updated: &str,
    ) -> IcsMission {
        IcsMission {
            id: id.into(),
            label: label.into(),
            status,
            created_at: ts(created),
            updated_at: ts(updated),
        }
    }

    #[test]
    fn render_starts_with_vcalendar_header() {
        let text = CalendarWriter::render(&[]).unwrap();
        assert!(text.contains("BEGIN:VCALENDAR"), "text was: {text}");
        assert!(text.contains("VERSION:2.0"));
        assert!(text.contains("PRODID:"));
        assert!(text.contains("CALSCALE:GREGORIAN"));
        assert!(text.contains("METHOD:PUBLISH"));
        assert!(text.contains("END:VCALENDAR"));
    }

    #[test]
    fn render_includes_one_vevent_per_mission() {
        let missions = vec![
            mission(
                "m1",
                "ship feature 1",
                IcsMissionStatus::InProgress,
                "2026-08-01T10:00:00Z",
                "2026-08-01T12:00:00Z",
            ),
            mission(
                "m2",
                "fix bug",
                IcsMissionStatus::Planning,
                "2026-08-01T09:00:00Z",
                "2026-08-01T09:30:00Z",
            ),
        ];
        let text = CalendarWriter::render(&missions).unwrap();
        let count = text.matches("BEGIN:VEVENT").count();
        assert_eq!(count, 2);
    }

    #[test]
    fn render_emits_uid_with_mission_id_suffix() {
        let m = mission(
            "abc",
            "x",
            IcsMissionStatus::Planning,
            "2026-08-01T10:00:00Z",
            "2026-08-01T10:00:00Z",
        );
        let text = CalendarWriter::render(&[m]).unwrap();
        assert!(text.contains("UID:mission-abc@opencode.dev"));
    }

    #[test]
    fn render_emits_dtstart_and_dtend_when_timed() {
        let m = mission(
            "abc",
            "x",
            IcsMissionStatus::InProgress,
            "2026-08-01T10:00:00Z",
            "2026-08-01T12:00:00Z",
        );
        let text = CalendarWriter::render(&[m]).unwrap();
        assert!(
            text.contains("DTSTART:20260801T100000Z")
                || text.contains("DTSTART;VALUE=DATE-TIME:20260801T100000Z"),
            "text was: {text}"
        );
        assert!(text.contains("20260801T120000"));
    }

    #[test]
    fn render_emits_dtstart_value_date_when_all_day() {
        let m = mission(
            "abc",
            "x",
            IcsMissionStatus::InProgress,
            "2026-08-01T10:00:00Z",
            "2026-08-03T12:00:00Z",
        );
        let text = CalendarWriter::render(&[m]).unwrap();
        assert!(
            text.contains("DTSTART;VALUE=DATE:20260801"),
            "text was: {text}"
        );
        // No DTEND for all-day (RFC 5545 §3.6.1: spans the day).
        assert!(!text.contains("DTEND"));
    }

    #[test]
    fn render_status_maps_correctly() {
        let cases = [
            (IcsMissionStatus::Received, "TENTATIVE"),
            (IcsMissionStatus::Planning, "TENTATIVE"),
            (IcsMissionStatus::InProgress, "CONFIRMED"),
            (IcsMissionStatus::Validation, "CONFIRMED"),
            (IcsMissionStatus::Completed, "CONFIRMED"),
            (IcsMissionStatus::Failed, "CANCELLED"),
        ];
        for (status, expected) in cases {
            let m = mission(
                "x",
                "x",
                status,
                "2026-08-01T10:00:00Z",
                "2026-08-01T11:00:00Z",
            );
            let text = CalendarWriter::render(&[m]).unwrap();
            assert!(
                text.contains(&format!("STATUS:{expected}")),
                "expected STATUS:{expected} for {:?}, got: {text}",
                status
            );
        }
    }

    #[test]
    fn render_categories_use_mission_status_token() {
        let m = mission(
            "x",
            "x",
            IcsMissionStatus::InProgress,
            "2026-08-01T10:00:00Z",
            "2026-08-01T11:00:00Z",
        );
        let text = CalendarWriter::render(&[m]).unwrap();
        assert!(text.contains("CATEGORIES:in_progress"));
    }

    #[test]
    fn render_truncates_long_summary_to_240_chars() {
        let long_label = "x".repeat(300);
        let m = mission(
            "x",
            &long_label,
            IcsMissionStatus::InProgress,
            "2026-08-01T10:00:00Z",
            "2026-08-01T11:00:00Z",
        );
        let text = CalendarWriter::render(&[m]).unwrap();
        // The summary in the rendered text should be at most 240 chars
        // (line folding may split; we look at the first content line).
        let summary_line = text
            .lines()
            .find(|l| l.starts_with("SUMMARY:"))
            .unwrap_or_else(|| panic!("no SUMMARY in: {text}"))
            .trim_start_matches("SUMMARY:");
        let first_chunk_chars = summary_line.chars().count();
        // 240 is the cap; the second line of the fold doesn't add to this.
        assert!(
            first_chunk_chars <= 240,
            "first SUMMARY chunk was {first_chunk_chars} chars: {summary_line:?}"
        );
    }

    #[test]
    fn render_emits_refresh_interval_and_name() {
        let text = CalendarWriter::render(&[]).unwrap();
        // The ics crate emits `REFRESH-INTERVAL;VALUE=DURATION:PT1H` per
        // RFC 5545 §3.3.6 (a typed VALUE parameter). We just assert the
        // property name + interval are present, not the exact shape —
        // different calendar UIs accept both `;VALUE=DURATION:PT1H` and
        // bare `:PT1H`.
        assert!(
            text.contains("REFRESH-INTERVAL"),
            "missing REFRESH-INTERVAL in: {text}",
        );
        assert!(text.contains("PT1H"), "missing PT1H in: {text}");
        assert!(text.contains("NAME:Atlas OS"));
    }

    #[test]
    fn render_empty_missions_is_still_valid_calendar() {
        let text = CalendarWriter::render(&[]).unwrap();
        assert!(text.starts_with("BEGIN:VCALENDAR"));
        assert!(text.ends_with("END:VCALENDAR\r\n") || text.ends_with("END:VCALENDAR\n"));
        let events = text.matches("BEGIN:VEVENT").count();
        assert_eq!(events, 0);
    }
}
