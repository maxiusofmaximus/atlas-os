// Atlas OS — Calendar ICS HUD route handler (RFC 28 Section G).
//
// Single axum endpoint mounted on the HUD Mission Control server:
//
//     GET /atlas-calendar.ics?token={base64url(16 bytes)}
//
// Returns a `text/calendar` body (RFC 5545) synthesized by
// `CalendarWriter::render` from the mission projection
// `Journal::missions_for_ics(30)`. The feed is read-only and intended
// for `webcal://` subscription from Outlook / Apple Calendar / Google
// Calendar. The opaque `token` (§G.2) gates access — see
// `crate::calendar::token`.
//
// One subtle wrinkle: the Journal is behind a `parking_lot::Mutex` and
// acquiring a sync mutex inside an async handler is acceptable here
// because the critical section is bounded SQL work (no `.await`s held
// across the guard). The guard is dropped before `render` runs to
// minimize contention.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::core::state::AppState;

/// Default lookback window: missions updated within the last 30 days.
/// RFC 28 §G.2.1 lists 30 days as the subscription default; operators
/// can tweak this once §G ships a per-request `?days=` query param
/// (post-MVP — the writer already accepts an arbitrary slice).
const DEFAULT_LOOKBACK_DAYS: i64 = 30;

#[derive(Debug, Deserialize)]
pub struct FeedQuery {
    pub token: Option<String>,
}

pub async fn get_calendar_ics(
    State(state): State<Arc<AppState>>,
    Query(q): Query<FeedQuery>,
) -> Response {
    let expected = match crate::calendar::token::ensure_token(&state.profile_root()) {
        Ok(t) => t,
        Err(err) => {
            tracing::error!(error = %err, "calendar/ics: token unavailable");
            return plain(StatusCode::INTERNAL_SERVER_ERROR, "token unavailable");
        }
    };
    if !crate::calendar::token::verify(&expected, q.token.as_deref()) {
        return plain(StatusCode::UNAUTHORIZED, "missing or invalid token");
    }

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
    (StatusCode::OK, headers, body).into_response()
}

fn plain(status: StatusCode, message: &str) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    (status, headers, format!("atlas-calendar.ics: {message}")).into_response()
}

fn ics_error_response(err: crate::calendar::CalendarError) -> Response {
    plain(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
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
        Arc::new(AppState::from_journal_in(journal, dir))
    }

    async fn call(state: Arc<AppState>, token: Option<String>) -> Response {
        get_calendar_ics(State(state), Query(FeedQuery { token })).await
    }

    fn good_token(state: &AppState) -> String {
        crate::calendar::token::ensure_token(&state.profile_root()).unwrap()
    }

    #[tokio::test]
    async fn returns_text_calendar_with_valid_token() {
        let state = fresh_state();
        let token = good_token(&state);
        let resp = call(state, Some(token)).await;
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
    async fn missing_or_wrong_token_is_unauthorized() {
        let state = fresh_state();
        let _ = good_token(&state);
        assert_eq!(
            call(state.clone(), None).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            call(state.clone(), Some("wrong".into())).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            call(state, Some("a".repeat(22))).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn body_contains_vcalendar_frame_with_one_mission() {
        let state = fresh_state();
        {
            let journal = state.journal();
            journal
                .create_mission(uuid::Uuid::new_v4(), "ship the calendar feature")
                .unwrap();
        }
        let token = good_token(&state);
        let resp = call(state, Some(token)).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.contains("BEGIN:VCALENDAR"), "text was: {text}");
        assert!(text.contains("END:VCALENDAR"));
        assert_eq!(text.matches("BEGIN:VEVENT").count(), 1, "text was: {text}");
        assert!(text.contains("ship the calendar feature"));
        let _ = CalendarWriter::render(&[]);
    }
}
