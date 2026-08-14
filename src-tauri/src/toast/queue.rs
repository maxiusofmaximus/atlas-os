// Atlas OS — Toast SQLite queue (RFC 28 Section F).
//
// Platform-agnostic CRUD over the `toast_queue` + `toast_history`
// tables. The dispatcher (`scheduler::ToastDriver`) pulls rows via
// `next_pending`, hands them to the manager for WinRT dispatch, then
// mutates the row with the outcome (`mark_fired` / `mark_dismissed`
// / `mark_failed`). On boot the driver also walks `toast_history`
// to skip rows whose outcome was already recorded (idempotent
// backfill).

use std::str::FromStr;

use rusqlite::Connection;

use crate::toast::error::{Result as ToastResult, ToastFacadeError};
use crate::toast::payload::{ToastKind, ToastPayload, ToastStatus};

/// SQL row projection of `toast_queue`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueRow {
    pub id: i64,
    pub kind: ToastKind,
    pub title: String,
    pub body: Option<String>,
    pub deep_link: Option<String>,
    pub fire_at: i64,
    pub status: ToastStatus,
    pub attempts: i64,
}

impl QueueRow {
    pub fn to_payload(&self) -> ToastPayload {
        ToastPayload {
            id: self.id,
            kind: self.kind,
            title: self.title.clone(),
            body: self.body.clone(),
            deep_link: self.deep_link.clone(),
            fire_at: self.fire_at,
        }
    }
}

/// Window onto the `toast_queue` and `toast_history` tables. The
/// underlying `Connection` is owned by `Journal`; we accept a
/// `&Connection` (immutable, per `migrate` in `journal::schema`)
/// and use `execute`/`prepare` (which acquire a write lock
/// transiently on `SharedConnection`).
pub struct ToastQueue<'a> {
    conn: &'a Connection,
}

impl<'a> ToastQueue<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Insert a queued toast and return the new row id.
    ///
    /// `fire_at` is unix millis. `kind` and `status` are stored as
    /// their lowercase TEXT token. Documentation of the TEXT format
    /// is in `payload.rs`; the round-trip equality is asserted by
    /// the `migrate` test in `journal::tests`.
    pub fn enqueue(
        &self,
        kind: ToastKind,
        title: &str,
        body: Option<&str>,
        deep_link: Option<&str>,
        fire_at_ms: i64,
    ) -> ToastResult<i64> {
        self.conn.execute(
            "INSERT INTO toast_queue
                (kind, title, body, deep_link, fire_at, status, attempts)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending', 0)",
            rusqlite::params![kind.to_string(), title, body, deep_link, fire_at_ms,],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Return the next `pending` row whose `fire_at <= now_ms`.
    /// `LIMIT 1` ordered by `fire_at ASC` so the earliest firing
    /// toast wins. Returns `Ok(None)` when there are no overdue
    /// pendents (the driver sleeps 5 s after this).
    pub fn next_pending(&self, now_ms: i64) -> ToastResult<Option<QueueRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, title, body, deep_link, fire_at, status, attempts
             FROM toast_queue
             WHERE fire_at <= ?1 AND status = 'pending'
             ORDER BY fire_at ASC
             LIMIT 1",
        )?;
        let mut rows = stmt.query([now_ms])?;
        match rows.next()? {
            Some(r) => Ok(Some(row_to_queuerow(r)?)),
            None => Ok(None),
        }
    }

    /// Action-dequeue when status was already `fired`/`dismissed`/
    /// `failed` (e.g., the boot phase walks the historical queue
    /// to confirm no overdue `pending` survives a crash).
    pub fn count_pending(&self) -> ToastResult<i64> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) FROM toast_queue WHERE status = 'pending'",
            [],
            |r| r.get(0),
        )?)
    }

    /// Update the row to `fired` and append an audit entry to
    /// `toast_history` so a later crash-recovery pass can skip it.
    pub fn mark_fired(&self, id: i64, fired_at_ms: i64) -> ToastResult<()> {
        self.conn.execute(
            "UPDATE toast_queue
             SET status = 'fired',
                 fired_at = ?2,
                 attempts = attempts + 1
             WHERE id = ?1",
            rusqlite::params![id, fired_at_ms],
        )?;
        self.append_history(id, "fired", None)?;
        Ok(())
    }

    pub fn mark_dismissed(&self, id: i64, dismissed_at_ms: i64, reason: &str) -> ToastResult<()> {
        self.conn.execute(
            "UPDATE toast_queue
             SET status = 'dismissed',
                 dismissed_at = ?2,
                 dismiss_reason = ?3,
                 attempts = attempts + 1
             WHERE id = ?1",
            rusqlite::params![id, dismissed_at_ms, reason],
        )?;
        self.append_history(id, "dismissed", Some(reason))?;
        Ok(())
    }

    pub fn mark_failed(&self, id: i64, attempts: i64) -> ToastResult<()> {
        self.conn.execute(
            "UPDATE toast_queue
             SET status = 'failed',
                 attempts = attempts
             WHERE id = ?1",
            rusqlite::params![id, attempts],
        )?;
        self.append_history(id, "failed", None)?;
        Ok(())
    }

    /// Look up the row by id.
    pub fn get(&self, id: i64) -> ToastResult<Option<QueueRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, title, body, deep_link, fire_at, status, attempts
             FROM toast_queue WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        match rows.next()? {
            Some(r) => Ok(Some(row_to_queuerow(r)?)),
            None => Ok(None),
        }
    }

    /// Cancel (delete) a pending toast. Returns `true` if a row was
    /// removed. Used by the `opencode toast cancel <id>` CLI when
    /// the user manually defers a reminder.
    pub fn cancel(&self, id: i64) -> ToastResult<bool> {
        let n = self.conn.execute(
            "DELETE FROM toast_queue WHERE id = ?1 AND status = 'pending'",
            [id],
        )?;
        Ok(n == 1)
    }

    /// List recent rows (newest first), capped at `limit`.
    pub fn list(&self, limit: i64) -> ToastResult<Vec<QueueRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, title, body, deep_link, fire_at, status, attempts
             FROM toast_queue
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], row_to_queuerow)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    fn append_history(
        &self,
        queue_id: i64,
        outcome: &str,
        reason: Option<&str>,
    ) -> ToastResult<()> {
        let row = self.get(queue_id)?.ok_or(ToastFacadeError::Journal(
            rusqlite::Error::QueryReturnedNoRows,
        ))?;
        self.conn.execute(
            "INSERT INTO toast_history
                (queue_id, kind, title, body, deep_link, fire_at, fired_at, dismiss_reason, outcome)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                queue_id,
                row.kind.to_string(),
                row.title,
                row.body,
                row.deep_link,
                row.fire_at,
                chrono::Utc::now().timestamp_millis(),
                reason,
                outcome,
            ],
        )?;
        Ok(())
    }
}

fn row_to_queuerow(r: &rusqlite::Row<'_>) -> rusqlite::Result<QueueRow> {
    let kind_str: String = r.get(1)?;
    let status_str: String = r.get(6)?;
    let kind = ToastKind::from_str(&kind_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let status = ToastStatus::from_str(&status_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(QueueRow {
        id: r.get(0)?,
        kind,
        title: r.get(2)?,
        body: r.get(3)?,
        deep_link: r.get(4)?,
        fire_at: r.get(5)?,
        status,
        attempts: r.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_queue() -> ToastQueue<'static> {
        // SAFETY: We hand-maintain the conn leak so the queue lives as
        // long as the test thread. Tests are single-threaded and each
        // test creates its own fresh in-memory conn + migration.
        let conn = Box::leak(Box::new(rusqlite::Connection::open_in_memory().unwrap()));
        crate::journal::schema::migrate(conn).unwrap();
        ToastQueue::new(conn)
    }

    #[test]
    fn enqueue_then_next_pending_returns_in_order() {
        let q = fresh_queue();
        let id_a = q
            .enqueue(ToastKind::Info, "first", None, None, 1_000)
            .unwrap();
        let id_b = q
            .enqueue(ToastKind::Info, "second", None, None, 2_000)
            .unwrap();
        // Both overdue at now=5_000 -> first(1_000) wins.
        let r = q.next_pending(5_000).unwrap().expect("pending");
        assert_eq!(r.id, id_a);
        // After marking fired, next call should pick second.
        q.mark_fired(id_a, 5_000).unwrap();
        let r = q.next_pending(5_000).unwrap().expect("pending b");
        assert_eq!(r.id, id_b);
    }

    #[test]
    fn next_pending_returns_none_when_no_overdue() {
        let q = fresh_queue();
        q.enqueue(ToastKind::Info, "later", None, None, 10_000)
            .unwrap();
        assert!(q.next_pending(1_000).unwrap().is_none());
    }

    #[test]
    fn mark_fired_advances_status_and_attempts() {
        let q = fresh_queue();
        let id = q.enqueue(ToastKind::Info, "x", None, None, 0).unwrap();
        q.mark_fired(id, 42).unwrap();
        let r = q.get(id).unwrap().unwrap();
        assert_eq!(r.status, ToastStatus::Fired);
        assert_eq!(r.attempts, 1);
    }

    #[test]
    fn mark_dismissed_records_reason() {
        let q = fresh_queue();
        let id = q.enqueue(ToastKind::Info, "x", None, None, 0).unwrap();
        q.mark_dismissed(id, 84, "userCanceled").unwrap();
        let r = q.get(id).unwrap().unwrap();
        assert_eq!(r.status, ToastStatus::Dismissed);
    }

    #[test]
    fn cancel_pending_removes_row() {
        let q = fresh_queue();
        let id = q.enqueue(ToastKind::Info, "x", None, None, 0).unwrap();
        assert!(q.cancel(id).unwrap());
        assert!(q.get(id).unwrap().is_none());
    }

    #[test]
    fn cancel_only_pending_rows() {
        let q = fresh_queue();
        let id = q.enqueue(ToastKind::Info, "x", None, None, 0).unwrap();
        q.mark_fired(id, 0).unwrap();
        // Already fired -> cancel should return false.
        assert!(!q.cancel(id).unwrap());
    }

    #[test]
    fn count_pending_lives_up_to_status_changes() {
        let q = fresh_queue();
        q.enqueue(ToastKind::Info, "x", None, None, 0).unwrap();
        q.enqueue(ToastKind::Info, "y", None, None, 0).unwrap();
        assert_eq!(q.count_pending().unwrap(), 2);
        let r = q.next_pending(0).unwrap().unwrap();
        q.mark_fired(r.id, 0).unwrap();
        assert_eq!(q.count_pending().unwrap(), 1);
    }

    #[test]
    fn history_is_appended_per_outcome() {
        let q = fresh_queue();
        let id = q.enqueue(ToastKind::Info, "x", None, None, 0).unwrap();
        q.mark_fired(id, 0).unwrap();
        q.mark_dismissed(id, 0, "timedOut").unwrap();
        let conn = q.conn;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM toast_history WHERE queue_id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn payload_projection_round_trips_text_fields() {
        let q = fresh_queue();
        let id = q
            .enqueue(
                ToastKind::CalendarReminder,
                "standup",
                Some("in 2 minutes"),
                Some("opencode://mission/m1"),
                100,
            )
            .unwrap();
        let r = q.get(id).unwrap().unwrap();
        let p = r.to_payload();
        assert_eq!(p.id, id);
        assert_eq!(p.kind, ToastKind::CalendarReminder);
        assert_eq!(p.body.as_deref(), Some("in 2 minutes"));
        assert_eq!(p.deep_link.as_deref(), Some("opencode://mission/m1"));
        assert_eq!(p.fire_at, 100);
    }

    #[test]
    fn list_returns_in_descending_id_order() {
        let q = fresh_queue();
        q.enqueue(ToastKind::Info, "a", None, None, 0).unwrap();
        q.enqueue(ToastKind::Info, "b", None, None, 0).unwrap();
        q.enqueue(ToastKind::Info, "c", None, None, 0).unwrap();
        let rows = q.list(2).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].title, "c");
        assert_eq!(rows[1].title, "b");
    }
}
