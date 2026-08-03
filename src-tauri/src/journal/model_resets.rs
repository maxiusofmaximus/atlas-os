// OpenCode OS — `model_resets` row + persistence helpers (RFC 28 §H, M19).
//
// M19 populates this table with one row per `(provider, model,
// resets_at)` triple observed from an upstream provider error (429
// rate-limit or 402/403 spend-cap). Each row records the reset
// window OpenCode OS will respect: when `resets_at` arrives, the
// scheduler in §F fires a `kind='model_ready'` Toast (and the row's
// `toast_id` records the queue row used). Persistent rows survive
// process restarts — crash recovery rehydrates the scheduler's
// pending Toast list and re-evaluates `resets_at` against `now`,
// skipping rows whose window already elapsed while the process was
// down (RFC 28 §H.4 + AGENTS.md §4 "idempotency_key collision on
// replays" rule).
//
// The table is NOT feature-gated: a default build still creates it
// (cheap schema bump), so a future operator enabling §H mid-run does
// not need a migration step. All writers / readers here are unconditional.
//
// Note we deliberately do NOT carry `mission_id` on a `model_reset`
// row: a reset is per provider-model, not per mission. Multiple
// missions can be paused because of a single reset; the linkage is
// done at query-time (HUD tail joins by `model_id`).

use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::orchestrator::error::{ResetKind, SpendLimitError};

/// In-memory projection of one `model_resets` row. 1:1 with the SQL
/// schema (M19) so callers can read rows directly without a translation
/// layer.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelResetRow {
    pub id: i64,
    pub provider: String,
    pub model: String,
    pub status_code: i64,
    /// `Some("rate_limit") | Some("spend_limit") | None` — matches the
    /// OmniRoute envelope's `error.type`; `None` when the parser saw
    /// only an HTTP status and inferring produced no kind.
    pub error_type: Option<String>,
    /// Reset window, unix-millis absolute (UTC).
    pub resets_at: i64,
    pub request_id: Option<String>,
    /// Observation wall-clock, unix-seconds absolute (UTC).
    pub observed_at: i64,
    /// `Some(toast_queue.id)` once a Toast has been enqueued for this
    /// row; `None` until then.
    pub toast_id: Option<i64>,
    /// `Some(unix-millis)` once the user dismissed the Toast; `None`
    /// while the Toast is still pending / fired / re-displayed.
    pub toast_dismissed_at: Option<i64>,
}

impl ModelResetRow {
    /// `resets_at` converted to a `DateTime<Utc>` for arithmetic with
    /// `chrono`. Returns `Utc.timestamp_millis_opt(...)` so a value
    /// past the i64 range or out-of-range is clamped rather than
    /// panicking.
    pub fn resets_at_dt(&self) -> DateTime<Utc> {
        chrono::TimeZone::timestamp_millis_opt(&Utc, self.resets_at)
            .single()
            .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap_or_default())
    }

    /// Convenience: kind inferred from `error_type` (or `None` when
    /// the stored string isn't a recognized token — defensive for
    /// future schema drift).
    pub fn kind(&self) -> Option<ResetKind> {
        self.error_type.as_deref().and_then(|s| match s {
            "rate_limit" => Some(ResetKind::RateLimit),
            "spend_limit" => Some(ResetKind::SpendLimit),
            _ => None,
        })
    }
}

/// Insert one reset observation. `resets_at` / `observed_at` are unix
/// milliseconds. The caller's `SpendLimitError` already carries
/// everything the row needs; this helper just lifts the fields and
/// handles the UNIQUE dedupe: a re-observation of the same
/// `(provider, model, resets_at)` returns the existing row id
/// (UPSERT semantics).
///
/// NB: the underlying UNIQUE index is
/// `model_resets_uniq_idx(provider, model, resets_at)`. We rely on
/// the index's `ON CONFLICT DO NOTHING` (set here via `INSERT OR
/// IGNORE`) plus a follow-up `SELECT` to retrieve either row id.
pub fn insert_model_reset(
    conn: &Connection,
    error: &SpendLimitError,
    observed_at: DateTime<Utc>,
) -> anyhow::Result<i64> {
    let resets_at_ms = error.resets_at.timestamp_millis();
    let observed_at_ms = observed_at.timestamp_millis();
    conn.execute(
        "INSERT OR IGNORE INTO model_resets
            (provider, model, status_code, error_type, resets_at, request_id, observed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            error.provider,
            error.model,
            error.status_code as i64,
            error.kind.as_str(),
            resets_at_ms,
            error.request_id,
            observed_at_ms,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Return the most recent pending reset row for a given
/// `(provider, model)` pair whose `toast_dismissed_at IS NULL` and
/// whose `resets_at` has not yet elapsed relative to `now`. Used by
/// the Planning engine / Orchestrator when deciding whether to
/// enqueue a turn proactively or wait. `None` when no active reset
/// exists for that provider-model.
pub fn pending_for(
    conn: &Connection,
    provider: &str,
    model: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<Option<ModelResetRow>> {
    let now_ms = now.timestamp_millis();
    let mut stmt = conn.prepare(
        "SELECT id, provider, model, status_code, error_type, resets_at, request_id, observed_at, toast_id, toast_dismissed_at
         FROM model_resets
         WHERE provider = ?1
           AND model = ?2
           AND toast_dismissed_at IS NULL
           AND resets_at > ?3
         ORDER BY resets_at ASC
         LIMIT 1",
    )?;
    let mut rows = stmt.query_map(
        rusqlite::params![provider, model, now_ms],
        row_to_model_reset,
    )?;
    match rows.next() {
        Some(r) => Ok(Some(r?)),
        None => Ok(None),
    }
}

/// List all pending reset rows (newest `resets_at` first) whose
/// `toast_dismissed_at IS NULL` and whose `resets_at` hasn't elapsed.
/// The scheduler uses this to seed its pending-Toast list on boot /
/// crash recovery.
pub fn pending_resets(conn: &Connection, now: DateTime<Utc>) -> anyhow::Result<Vec<ModelResetRow>> {
    let now_ms = now.timestamp_millis();
    let mut stmt = conn.prepare(
        "SELECT id, provider, model, status_code, error_type, resets_at, request_id, observed_at, toast_id, toast_dismissed_at
         FROM model_resets
         WHERE toast_dismissed_at IS NULL
           AND resets_at > ?1
         ORDER BY resets_at DESC",
    )?;
    let rows = stmt.query_map(rusqlite::params![now_ms], row_to_model_reset)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Record the Toast queue row id enqueued for a reset, linking the
/// reset row to the Toast row so the scheduler knows where the
/// dismissal signal will come from.
pub fn link_toast_id(conn: &Connection, reset_id: i64, toast_id: i64) -> anyhow::Result<bool> {
    let n = conn.execute(
        "UPDATE model_resets SET toast_id = ?1 WHERE id = ?2",
        rusqlite::params![toast_id, reset_id],
    )?;
    Ok(n == 1)
}

/// Mark a reset row's Toast as dismissed. Called by the Toast callback
/// when the user dismisses the `model_ready` card (or when the cancel
/// handler runs `toast_cancel`). Returns `true` when the row was
/// updated, `false` when no row matched the id.
pub fn mark_toast_dismissed(
    conn: &Connection,
    reset_id: i64,
    dismissed_at: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let n = conn.execute(
        "UPDATE model_resets SET toast_dismissed_at = ?1 WHERE id = ?2",
        rusqlite::params![dismissed_at.timestamp_millis(), reset_id],
    )?;
    Ok(n == 1)
}

/// Mark a reset row's Toast as dismissed, looked up by `toast_id`
/// (the inverse of `link_toast_id`). Convenience for the Toast
/// callback where the caller only carries the queue row id.
pub fn mark_toast_dismissed_by_queue_id(
    conn: &Connection,
    toast_id: i64,
    dismissed_at: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let n = conn.execute(
        "UPDATE model_resets SET toast_dismissed_at = ?1 WHERE toast_id = ?2",
        rusqlite::params![dismissed_at.timestamp_millis(), toast_id],
    )?;
    Ok(n == 1)
}

fn row_to_model_reset(r: &rusqlite::Row<'_>) -> rusqlite::Result<ModelResetRow> {
    Ok(ModelResetRow {
        id: r.get(0)?,
        provider: r.get(1)?,
        model: r.get(2)?,
        status_code: r.get(3)?,
        error_type: r.get(4)?,
        resets_at: r.get(5)?,
        request_id: r.get(6)?,
        observed_at: r.get(7)?,
        toast_id: r.get(8)?,
        toast_dismissed_at: r.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::schema::migrate;
    use chrono::TimeZone;

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

    #[test]
    fn insert_returns_new_id_and_row_is_queryable() {
        let conn = fresh_conn();
        let now = Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).unwrap();
        let id = insert_model_reset(
            &conn,
            &make_error(
                "anthropic",
                "claude-3-5-sonnet",
                "2026-08-03T12:30:00Z",
                ResetKind::RateLimit,
            ),
            now,
        )
        .unwrap();
        assert!(id > 0, "expected a row id");
        let row = pending_for(&conn, "anthropic", "claude-3-5-sonnet", now)
            .unwrap()
            .expect("row should be pending");
        assert_eq!(row.provider, "anthropic");
        assert_eq!(row.model, "claude-3-5-sonnet");
        assert_eq!(row.status_code, 429);
        assert_eq!(row.error_type.as_deref(), Some("rate_limit"));
        assert_eq!(
            row.resets_at_dt(),
            Utc.with_ymd_and_hms(2026, 8, 3, 12, 30, 0).unwrap()
        );
    }

    #[test]
    fn insert_dedupes_on_unique_provider_model_resets_at() {
        let conn = fresh_conn();
        let now = Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).unwrap();
        let err = make_error(
            "openai",
            "gpt-5",
            "2026-08-03T13:00:00Z",
            ResetKind::SpendLimit,
        );
        let first = insert_model_reset(&conn, &err, now).unwrap();
        let second = insert_model_reset(&conn, &err, now).unwrap();
        assert_eq!(
            first, second,
            "INSERT OR IGNORE should yield the same id on dup insert"
        );
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM model_resets WHERE provider = 'openai'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "deduplicated to a single row");
    }

    #[test]
    fn pending_for_filters_out_past_resets_at() {
        let conn = fresh_conn();
        let now = Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).unwrap();
        insert_model_reset(
            &conn,
            &make_error(
                "anthropic",
                "claude",
                "2026-08-03T11:00:00Z",
                ResetKind::RateLimit,
            ),
            now,
        )
        .unwrap();
        let row = pending_for(&conn, "anthropic", "claude", now).unwrap();
        assert!(row.is_none(), "resets_at in the past should not be pending");
    }

    #[test]
    fn pending_for_filters_out_dismissed_rows() {
        let conn = fresh_conn();
        let now = Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).unwrap();
        let id = insert_model_reset(
            &conn,
            &make_error(
                "anthropic",
                "claude",
                "2026-08-03T13:00:00Z",
                ResetKind::RateLimit,
            ),
            now,
        )
        .unwrap();
        mark_toast_dismissed(&conn, id, now).unwrap();
        let row = pending_for(&conn, "anthropic", "claude", now).unwrap();
        assert!(row.is_none(), "dismissed row should not be pending");
    }

    #[test]
    fn link_toast_id_and_mark_dismissed_by_queue_id_round_trip() {
        let conn = fresh_conn();
        let now = Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).unwrap();
        let id = insert_model_reset(
            &conn,
            &make_error(
                "omniroute",
                "claude-3-5-sonnet",
                "2026-08-03T13:30:00Z",
                ResetKind::RateLimit,
            ),
            now,
        )
        .unwrap();
        assert!(
            link_toast_id(&conn, id, 999).unwrap(),
            "link should update 1 row"
        );
        let row = pending_for(&conn, "omniroute", "claude-3-5-sonnet", now)
            .unwrap()
            .unwrap();
        assert_eq!(row.toast_id, Some(999));
        assert!(mark_toast_dismissed_by_queue_id(&conn, 999, now).unwrap());
        let row = pending_for(&conn, "omniroute", "claude-3-5-sonnet", now).unwrap();
        assert!(row.is_none(), "dismissed row no longer pending");
    }

    #[test]
    fn pending_resets_lists_all_undismissed_future_resets() {
        let conn = fresh_conn();
        let now = Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).unwrap();
        insert_model_reset(
            &conn,
            &make_error(
                "anthropic",
                "claude-a",
                "2026-08-03T13:00:00Z",
                ResetKind::RateLimit,
            ),
            now,
        )
        .unwrap();
        insert_model_reset(
            &conn,
            &make_error(
                "openai",
                "gpt-5",
                "2026-08-03T14:00:00Z",
                ResetKind::SpendLimit,
            ),
            now,
        )
        .unwrap();
        // Past — should not appear.
        insert_model_reset(
            &conn,
            &make_error(
                "anthropic",
                "claude-b",
                "2026-08-03T11:00:00Z",
                ResetKind::RateLimit,
            ),
            now,
        )
        .unwrap();
        let rows = pending_resets(&conn, now).unwrap();
        assert_eq!(rows.len(), 2, "two undismissed + future rows");
        // ORDER BY resets_at DESC — claude-3-5-sonnet... actually the
        // second one (gpt-5) is at 14:00 → comes first.
        assert_eq!(rows[0].model, "gpt-5");
        assert_eq!(rows[1].model, "claude-a");
    }

    #[test]
    fn mark_toast_dismissed_returns_false_for_missing_id() {
        let conn = fresh_conn();
        let now = Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).unwrap();
        assert!(!mark_toast_dismissed(&conn, 9999, now).unwrap());
    }

    #[test]
    fn row_kind_round_trips_error_type_string() {
        let conn = fresh_conn();
        let now = Utc.with_ymd_and_hms(2026, 8, 3, 12, 0, 0).unwrap();
        let _ = insert_model_reset(
            &conn,
            &make_error(
                "anthropic",
                "claude",
                "2026-08-03T13:00:00Z",
                ResetKind::SpendLimit,
            ),
            now,
        )
        .unwrap();
        let row = pending_for(&conn, "anthropic", "claude", now)
            .unwrap()
            .unwrap();
        assert_eq!(row.kind(), Some(ResetKind::SpendLimit));
    }
}
