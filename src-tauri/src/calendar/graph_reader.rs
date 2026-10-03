// Atlas OS — Calendar MS Graph reader (RFC 28 Section G — READ path).
//
// Fetches `/me/calendarView` for a forward window, maps each event to a
// `graph`-sourced busy window, and replaces the previous poll's rows. The
// pure parsing layer (`parse_calendar_view`) is unit-tested against a
// fixture; only `sync` touches the network.

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

use crate::calendar::error::{CalendarError, Result};
use crate::calendar::payload::BusySource;
use crate::calendar::queue::BusyWindowInput;
use crate::journal::Journal;

/// How far ahead of "now" a sync pulls events.
const LOOKAHEAD_DAYS: i64 = 7;

/// One event already normalised into the busy-window shape (owned strings so
/// it survives past the response buffer).
#[derive(Clone, Debug, PartialEq)]
pub struct GraphBusy {
    pub external_id: String,
    pub subject: String,
    pub body: Option<String>,
    pub starts_at: i64,
    pub ends_at: i64,
    pub weight: f64,
}

impl GraphBusy {
    fn as_input(&self) -> BusyWindowInput<'_> {
        BusyWindowInput {
            source: BusySource::Graph,
            external_id: &self.external_id,
            subject: &self.subject,
            body: self.body.as_deref(),
            starts_at: self.starts_at,
            ends_at: self.ends_at,
            weight: self.weight,
        }
    }
}

#[derive(Deserialize)]
struct CalendarViewResponse {
    value: Vec<GraphEvent>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphEvent {
    id: String,
    #[serde(default)]
    subject: Option<String>,
    start: GraphDateTime,
    end: GraphDateTime,
    #[serde(default)]
    show_as: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphDateTime {
    date_time: String,
}

/// Parse a `/me/calendarView` JSON body into normalised busy windows.
/// Events with an unparseable timestamp or an inverted range are skipped
/// rather than failing the whole sync.
pub fn parse_calendar_view(json: &str) -> Result<Vec<GraphBusy>> {
    let response: CalendarViewResponse = serde_json::from_str(json)
        .map_err(|e| CalendarError::Graph(format!("calendarView parse: {e}")))?;
    let mut out = Vec::with_capacity(response.value.len());
    for event in response.value {
        let (Some(starts_at), Some(ends_at)) = (
            parse_graph_datetime(&event.start.date_time),
            parse_graph_datetime(&event.end.date_time),
        ) else {
            continue;
        };
        if starts_at >= ends_at {
            continue;
        }
        out.push(GraphBusy {
            external_id: event.id,
            subject: event.subject.unwrap_or_else(|| "(busy)".to_string()),
            body: None,
            starts_at,
            ends_at,
            weight: weight_for(event.show_as.as_deref()),
        });
    }
    Ok(out)
}

/// Graph returns `2026-10-03T09:00:00.0000000` when the `Prefer:
/// outlook.timezone="UTC"` header is set. `%.f` absorbs the fractional part.
fn parse_graph_datetime(value: &str) -> Option<i64> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.timestamp_millis());
    }
    chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S"))
        .ok()
        .map(|naive| naive.and_utc().timestamp_millis())
}

/// Map the Graph `showAs` free/busy status to a `[0, 1]` weight.
fn weight_for(show_as: Option<&str>) -> f64 {
    match show_as.unwrap_or("busy").to_ascii_lowercase().as_str() {
        "free" => 0.0,
        "tentative" => 0.5,
        _ => 1.0,
    }
}

/// Fetch the next [`LOOKAHEAD_DAYS`] of events and replace the `graph` busy
/// windows in the journal. Returns the number of windows written.
pub async fn sync(journal: &Journal, access_token: &str) -> Result<usize> {
    let now = Utc::now();
    let json = fetch_calendar_view(access_token, now, now + Duration::days(LOOKAHEAD_DAYS)).await?;
    let rows = parse_calendar_view(&json)?;
    journal.busy_window_delete_by_source(BusySource::Graph)?;
    for row in &rows {
        journal.busy_window_upsert(&row.as_input())?;
    }
    Ok(rows.len())
}

async fn fetch_calendar_view(
    access_token: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<String> {
    let start = start.format("%Y-%m-%dT%H:%M:%SZ");
    let end = end.format("%Y-%m-%dT%H:%M:%SZ");
    let url = format!(
        "https://graph.microsoft.com/v1.0/me/calendarView?startDateTime={start}&endDateTime={end}&\
         $select=id,subject,start,end,isAllDay,showAs"
    );
    let resp = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Prefer", "outlook.timezone=\"UTC\"")
        .send()
        .await
        .map_err(|e| CalendarError::Graph(e.to_string()))?;
    let status = resp.status();
    let www = resp
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = resp
        .text()
        .await
        .map_err(|e| CalendarError::Graph(e.to_string()))?;
    if !status.is_success() {
        return Err(CalendarError::Graph(format!(
            "calendarView HTTP {} [{www}]: {body}",
            status.as_u16()
        )));
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_utc_event_into_a_graph_busy() {
        let json = r#"{
            "value": [
                {
                    "id": "AAMkAD-1",
                    "subject": "Design review",
                    "start": { "dateTime": "2026-10-03T09:00:00.0000000", "timeZone": "UTC" },
                    "end":   { "dateTime": "2026-10-03T10:00:00.0000000", "timeZone": "UTC" },
                    "isAllDay": false,
                    "showAs": "busy"
                }
            ]
        }"#;
        let rows = parse_calendar_view(json).expect("parse");
        assert_eq!(rows.len(), 1);
        let e = &rows[0];
        assert_eq!(e.external_id, "AAMkAD-1");
        assert_eq!(e.subject, "Design review");
        let expected = chrono::NaiveDate::from_ymd_opt(2026, 10, 3)
            .unwrap()
            .and_hms_opt(9, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        assert_eq!(e.starts_at, expected);
        assert_eq!(e.ends_at - e.starts_at, 3_600_000);
        assert_eq!(e.weight, 1.0);
        assert_eq!(e.as_input().source, BusySource::Graph);
    }

    #[test]
    fn show_as_drives_weight() {
        assert_eq!(weight_for(Some("free")), 0.0);
        assert_eq!(weight_for(Some("tentative")), 0.5);
        assert_eq!(weight_for(Some("oof")), 1.0);
        assert_eq!(weight_for(None), 1.0);
    }

    #[test]
    fn skips_inverted_ranges_and_missing_ids_do_not_panic() {
        let json = r#"{
            "value": [
                {
                    "id": "bad-range",
                    "start": { "dateTime": "2026-10-03T10:00:00.0000000" },
                    "end":   { "dateTime": "2026-10-03T09:00:00.0000000" }
                },
                {
                    "id": "bad-date",
                    "start": { "dateTime": "not-a-date" },
                    "end":   { "dateTime": "2026-10-03T09:00:00.0000000" }
                },
                {
                    "id": "good",
                    "start": { "dateTime": "2026-10-03T11:00:00Z" },
                    "end":   { "dateTime": "2026-10-03T11:30:00Z" }
                }
            ]
        }"#;
        let rows = parse_calendar_view(json).expect("parse");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].external_id, "good");
        assert_eq!(rows[0].subject, "(busy)");
    }

    #[test]
    fn rejects_malformed_envelope() {
        assert!(parse_calendar_view("{ not json").is_err());
    }
}
