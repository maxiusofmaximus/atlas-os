// Atlas OS — Journal wrappers for the calendar busy-window queue (RFC 28 §G).
//
// The CLI (`atlas calendar busy …`) and, later, the Planning engine call
// these instead of reaching into the SQLite connection directly. The
// underlying CRUD lives in `crate::calendar::queue::BusyWindowQueue`, which
// is compiled unconditionally so a non-`calendar` build still exposes the
// operator surface.

use super::Journal;
use crate::calendar::payload::BusySource;
use crate::calendar::queue::{BusyWindowInput, BusyWindowQueue, BusyWindowRow};

/// One decrypted-at-call-site row from `calendar_auth`.
#[derive(Clone, Debug, PartialEq)]
pub struct CalendarTokenRow {
    pub ciphertext: Vec<u8>,
    pub key_hint: String,
    pub expires_at: Option<i64>,
}

impl Journal {
    /// List busy windows, newest first (bounded by `limit`).
    pub fn busy_window_list(&self, limit: i64) -> anyhow::Result<Vec<BusyWindowRow>> {
        let conn = self.conn.lock();
        Ok(BusyWindowQueue::new(&conn).list(limit)?)
    }

    /// Count all busy windows regardless of `source`.
    pub fn busy_window_count(&self) -> anyhow::Result<i64> {
        let conn = self.conn.lock();
        Ok(BusyWindowQueue::new(&conn).count()?)
    }

    /// Insert an operator-authored (`source = manual`) busy window.
    pub fn busy_window_insert_manual(&self, input: &BusyWindowInput<'_>) -> anyhow::Result<i64> {
        let conn = self.conn.lock();
        Ok(BusyWindowQueue::new(&conn).insert_manual(input)?)
    }

    /// UPSERT a busy window (deduped on `(source, external_id)`) — the poller path.
    pub fn busy_window_upsert(&self, input: &BusyWindowInput<'_>) -> anyhow::Result<i64> {
        let conn = self.conn.lock();
        Ok(BusyWindowQueue::new(&conn).upsert(input)?)
    }

    /// Delete every busy window from a given `source` (poller eviction).
    pub fn busy_window_delete_by_source(&self, source: BusySource) -> anyhow::Result<i64> {
        let conn = self.conn.lock();
        Ok(BusyWindowQueue::new(&conn).delete_by_source(source)?)
    }

    /// Delete a busy window by id; `true` when a row was removed.
    pub fn busy_window_delete(&self, id: i64) -> anyhow::Result<bool> {
        let conn = self.conn.lock();
        Ok(BusyWindowQueue::new(&conn).delete(id)?)
    }

    /// Busy windows overlapping `[start_ms, end_ms)` — the Planning query.
    pub fn busy_windows_overlapping(
        &self,
        start_ms: i64,
        end_ms: i64,
    ) -> anyhow::Result<Vec<BusyWindowRow>> {
        let conn = self.conn.lock();
        Ok(BusyWindowQueue::new(&conn).overlapping(start_ms, end_ms)?)
    }

    /// Persist (UPSERT) the encrypted refresh token for `account`.
    pub fn save_calendar_token(
        &self,
        account: &str,
        token_ciphertext: &[u8],
        key_hint: &str,
        expires_at: Option<i64>,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO calendar_auth (account, token_ciphertext, key_hint, expires_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, unixepoch() * 1000)
             ON CONFLICT(account) DO UPDATE SET
                token_ciphertext = excluded.token_ciphertext,
                key_hint        = excluded.key_hint,
                expires_at      = excluded.expires_at,
                updated_at      = unixepoch() * 1000",
            rusqlite::params![account, token_ciphertext, key_hint, expires_at],
        )?;
        Ok(())
    }

    /// Read the encrypted refresh token row for `account`.
    pub fn load_calendar_token(&self, account: &str) -> anyhow::Result<Option<CalendarTokenRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT token_ciphertext, key_hint, expires_at FROM calendar_auth WHERE account = ?1",
        )?;
        let mut rows = stmt.query([account])?;
        match rows.next()? {
            Some(r) => Ok(Some(CalendarTokenRow {
                ciphertext: r.get(0)?,
                key_hint: r.get(1)?,
                expires_at: r.get(2)?,
            })),
            None => Ok(None),
        }
    }
}
