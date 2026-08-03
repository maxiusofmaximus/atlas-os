// OpenCode OS — Calendar ICS HUD route handler (RFC 28 Section G).
//
// Single axum endpoint mounted on the HUD Mission Control server:
//
//     GET /opencode-calendar.ics
//
// Returns a `text/calendar` body (RFC 5545) synthesized by
// `CalendarWriter::render` from the mission projection
// `Journal::missions_for_ics(30)`. The feed is read-only and intended
// for `webcal://` subscription from Outlook / Apple Calendar / Google
// Calendar pointing at `http://127.0.0.1:{hud_port}/opencode-
// calendar.ics`.
//
// One subtle wrinkle: the Journal is behind a `parking_lot::Mutex` and
// acquiring a sync mutex inside an async handler is acceptable here
// because the critical section is bounded SQL work (no `.await`s held
// across the guard). The guard is dropped before `render` runs to
// minimize contention.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::IntoResponse;

use crate::core::state::AppState;

/// Default lookback window: missions updated within the last 30 days.
/// RFC 28 §G.2.1 lists 30 days as the subscription default; operators
/// can tweak this once §G ships a per-request `?days=` query param
/// (post-MVP — the writer already accepts an arbitrary slice).
const DEFAULT_LOOKBACK_DAYS: i64 = 30;

pub async fn get_calendar_ics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let missions = {
        let journal = state.journal();
        match journal.missions_for_ics(DEFAULT_LOOKBACK_DAYS) {
            Ok(rows) => rows,
            Err(err) => {
                tracing::error!(error = %err, "calendar/ics: missions_for_ics failed");
                return ics_error_response(crate::calendar::CalendarError::Backend(err));
            }
        }
    };

    let body = match crate::calendar::ics_writer::CalendarWriter::render(&missions) {
        Ok(text) => text,
        Err(err) => {
            tracing::error!(error = %err, "calendar/ics: render failed");
            return ics_error_response(crate::calendar::CalendarError::Ics(err.to_string()));
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/calendar; charset=utf-8"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, max-age=0"),
    );
    (axum::http::StatusCode::OK, headers, body)
}

fn ics_error_response(
    err: crate::calendar::CalendarError,
) -> (axum::http::StatusCode, HeaderMap, String) {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        headers,
        format!("opencode-calendar.ics: {err}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::ics_writer::CalendarWriter;
    use crate::core::state::AppState;
    use crate::journal::Journal;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use std::sync::Arc;

    fn fresh_state() -> Arc<AppState> {
        let dir = std::env::temp_dir().join(format!("opencode-ics-route-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let journal = Journal::open(&dir).unwrap();
        Arc::new(AppState::from_journal(journal))
    }

    #[tokio::test]
    async fn get_calendar_ics_returns_text_calendar_content_type() {
        let state = fresh_state();
        let resp = get_calendar_ics(State(state)).await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("Content-Type header");
        assert!(
            ct.to_str().unwrap().starts_with("text/calendar"),
            "content-type was {ct:?}",
        );
    }

    #[tokio::test]
    async fn get_calendar_ics_body_contains_vcalendar_frame() {
        let state = fresh_state();
        let resp = get_calendar_ics(State(state)).await.into_response();
        let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.contains("BEGIN:VCALENDAR"), "text was: {text}");
        assert!(text.contains("END:VCALENDAR"));
        assert_eq!(text.matches("BEGIN:VEVENT").count(), 0, "no missions yet");
    }

    #[tokio::test]
    async fn get_calendar_ics_end_to_end_with_one_mission() {
        let state = fresh_state();
        // Seed a mission so the feed lists one VEVENT.
        {
            let journal = state.journal();
            let id = uuid::Uuid::new_v4();
            journal
                .create_mission(id, "ship the calendar feature")
                .unwrap();
        }
        let resp = get_calendar_ics(State(state)).await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(text.matches("BEGIN:VEVENT").count(), 1, "text was: {text}");
        assert!(
            text.contains("ship the calendar feature"),
            "expected summary in feed: {text}",
        );
        // Drop the harness so the formatter isn't recomputed across suites.
        let _ = CalendarWriter::render(&[]);
    }
}
