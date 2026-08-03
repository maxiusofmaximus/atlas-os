// OpenCode OS — HUD §H card pipeline (RFC 28 §H.3 / §H.4 / §H.7 item 9).
// Maps the persistent `ModelResetRow` (M19) onto the JSON payloads
// the Svelte `<SpendLimitErrorCard>` / `<ModelReadyCard>` components
// subscribe to via the kernel-bus WS stream.
//
// The HUD server does NOT store its own state — these helpers are
// pure serialisers so the kernel bus can call them from the WS
// `handle_spend_limit_error` code path. Two shapes:
//
//   SpendLimitErrorCardPayload   — "limit hit right now" card.
//   ModelReadyCardPayload        — "limit window elapsed, model
//                                  ready again" card.
//
// Both shapes mirror the Svelte store types in
// `src/lib/stores/hud.ts` — keep field names in lock-step.

use serde::Serialize;

use crate::journal::model_resets::ModelResetRow;
use crate::orchestrator::error::ResetKind;

/// "Spend Limit Reached" / "Rate Limited" card payload. Emitted by
/// `orchestrator::handle_spend_limit_error` immediately after
/// persisting a fresh `model_resets` row. Drives the Svelte card at
/// `src/lib/components/SpendLimitErrorCard.svelte`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SpendLimitErrorCardPayload {
    pub provider: String,
    pub model: String,
    pub status_code: i64,
    /// `"rate_limit"` or `"spend_limit"` — drives the card title and
    /// the colour token (`#f85149` red vs `#f0883e` orange).
    pub error_type: String,
    /// RFC-3339 UTC string of the reset window `(2026-08-…Z)`.
    /// Formatted off `ModelResetRow::resets_at_dt()`.
    pub resets_at: String,
    /// Optional upstream provider request id (OmniRoute `request_id`).
    pub request_id: Option<String>,
    /// Toast queue row id when a Toast was enqueued for this reset;
    /// `None` when the `toast` feature is off or the row was already
    /// linked to an earlier Toast (idempotent replay).
    pub toast_enqueued_id: Option<i64>,
}

/// "Model Ready" card payload. Emitted by the scheduler (§F) when
/// the Toast queue row `kind='model_ready'` is fired. Drives the
/// Svelte card at `src/lib/components/ModelReadyCard.svelte`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModelReadyCardPayload {
    pub provider: String,
    pub model: String,
    /// RFC-3339 UTC string of the reset window (matches the original
    /// SpendLimit observation).
    pub resets_at: String,
    /// Optional — the mission that was paused due to the limit. The
    /// scheduler joins paused missions by `model_id` before emitting
    /// the Toast; `None` when no mission was paused.
    pub mission_id: Option<String>,
    /// The id of the `toast_queue` row carrying this card so the HUD
    /// can correlate dismissals back to the scheduler.
    pub toast_queue_id: i64,
}

/// Build a `SpendLimitErrorCardPayload` from a freshly-persisted
/// `ModelResetRow`. The row carries everything needed; we just
/// serialise the timestamps to RFC-3339 and lift the kind string.
pub fn spend_limit_error_card(row: &ModelResetRow) -> SpendLimitErrorCardPayload {
    let error_type = row
        .error_type
        .as_deref()
        .map(str::to_string)
        .unwrap_or_else(|| {
            ResetKind::from_status_code(row.status_code as u16)
                .map(|k| k.as_str().to_string())
                .unwrap_or_else(|| "rate_limit".into())
        });
    SpendLimitErrorCardPayload {
        provider: row.provider.clone(),
        model: row.model.clone(),
        status_code: row.status_code,
        error_type,
        resets_at: row.resets_at_dt().to_rfc3339(),
        request_id: row.request_id.clone(),
        toast_enqueued_id: row.toast_id,
    }
}

/// Build a `ModelReadyCardPayload` from a `ModelResetRow` whose
/// `resets_at` has just elapsed. `mission_id` is optional — the
/// HUD joins the SlowMissions list with `model_id` to populate it.
pub fn model_ready_card(
    row: &ModelResetRow,
    mission_id: Option<String>,
    toast_queue_id: i64,
) -> ModelReadyCardPayload {
    ModelReadyCardPayload {
        provider: row.provider.clone(),
        model: row.model.clone(),
        resets_at: row.resets_at_dt().to_rfc3339(),
        mission_id,
        toast_queue_id,
    }
}

/// Convenience: returns `Utc::now()` for the common "right now"
/// observations coming from `handle_spend_limit_error`. Kept here so
/// tests can pin `observed_at` without dragging in `Utc::now()` mocks
/// at the call site.
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn now_for_test() -> chrono::DateTime<chrono::Utc> {
    chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 8, 3, 12, 0, 0)
        .single()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::schema::migrate;
    use crate::orchestrator::error::{ResetKind, SpendLimitError};
    use chrono::{DateTime, TimeZone, Utc};
    use rusqlite::Connection;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn make_error(
        provider: &str,
        model: &str,
        resets_at: &str,
        kind: ResetKind,
    ) -> SpendLimitError {
        let (status, kind) = match kind {
            ResetKind::RateLimit => (429u16, ResetKind::RateLimit),
            ResetKind::SpendLimit => (402, ResetKind::SpendLimit),
        };
        SpendLimitError {
            provider: provider.into(),
            model: model.into(),
            status_code: status,
            kind,
            resets_at: DateTime::parse_from_rfc3339(resets_at)
                .unwrap()
                .with_timezone(&Utc),
            request_id: Some("req_1".into()),
            message: Some("test".into()),
        }
    }

    fn insert_row(conn: &Connection) -> ModelResetRow {
        let now = Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).unwrap();
        let err = make_error(
            "anthropic",
            "claude-3-5-sonnet",
            "2026-08-03T13:00:00Z",
            ResetKind::RateLimit,
        );
        let _id = crate::journal::model_resets::insert_model_reset(conn, &err, now).unwrap();
        crate::journal::model_resets::pending_for(conn, "anthropic", "claude-3-5-sonnet", now)
            .unwrap()
            .expect("row must exist")
    }

    #[test]
    fn spend_limit_error_card_lif_all_fields_from_row() {
        let conn = fresh_conn();
        let row = insert_row(&conn);
        let p = spend_limit_error_card(&row);
        assert_eq!(p.provider, "anthropic");
        assert_eq!(p.model, "claude-3-5-sonnet");
        assert_eq!(p.status_code, 429);
        assert_eq!(p.error_type, "rate_limit");
        assert!(p.resets_at.starts_with("2026-08-03T13:00:00"));
        assert_eq!(p.request_id.as_deref(), Some("req_1"));
        assert_eq!(p.toast_enqueued_id, None, "row not linked yet");
    }

    #[test]
    fn spend_limit_error_card_falls_back_to_status_inference_when_kind_missing() {
        let conn = fresh_conn();
        let row = insert_row(&conn);
        // Force null error_type to exercise the fallback path.
        conn.execute(
            "UPDATE model_resets SET error_type = NULL WHERE id = ?1",
            rusqlite::params![row.id],
        )
        .unwrap();
        let now = Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).unwrap();
        let refreshed =
            crate::journal::model_resets::pending_for(&conn, "anthropic", "claude-3-5-sonnet", now)
                .unwrap()
                .unwrap();
        let p = spend_limit_error_card(&refreshed);
        assert_eq!(p.error_type, "rate_limit", "429 → rate_limit fallback");
    }

    #[test]
    fn model_ready_card_lif_from_row_with_mission_id() {
        let conn = fresh_conn();
        let row = insert_row(&conn);
        let p = model_ready_card(&row, Some("00000000-0000-0000-0000-00000000abc".into()), 42);
        assert_eq!(p.provider, "anthropic");
        assert_eq!(p.model, "claude-3-5-sonnet");
        assert_eq!(p.toast_queue_id, 42);
        assert_eq!(
            p.mission_id.as_deref(),
            Some("00000000-0000-0000-0000-00000000abc")
        );
    }

    #[test]
    fn model_ready_card_allows_null_mission_id() {
        let conn = fresh_conn();
        let row = insert_row(&conn);
        let p = model_ready_card(&row, None, 100);
        assert!(p.mission_id.is_none());
        assert_eq!(p.toast_queue_id, 100);
    }

    #[test]
    fn card_payload_serializes_to_json() {
        let conn = fresh_conn();
        let row = insert_row(&conn);
        let p = spend_limit_error_card(&row);
        let j = serde_json::to_value(&p).unwrap();
        assert_eq!(j["provider"], "anthropic");
        assert_eq!(j["model"], "claude-3-5-sonnet");
        assert_eq!(j["status_code"], 429);
    }
}
