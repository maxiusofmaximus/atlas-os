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

/// One row from `calendar_subscriptions` (M35) — a durable external
/// calendar feed registered by the operator (RFC 28 §G READ, v3.1.A.2).
/// `name` is both the local handle and the id namespace used in
/// `calendar_busy_windows.external_id` (`{name}:{uid}`); `etag` /
/// `last_modified` are the HTTP validators for the next conditional GET.
#[derive(Clone, Debug, PartialEq)]
pub struct CalendarSubscriptionRow {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub kind: String,
    pub enabled: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub last_sync_ms: Option<i64>,
    pub last_status: Option<String>,
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

    /// Delete every `ics_local` busy window belonging to subscription
    /// `name` (ids are namespaced `{name}:{uid}`).
    pub fn busy_window_delete_by_source_prefix(
        &self,
        source: BusySource,
        prefix: &str,
    ) -> anyhow::Result<i64> {
        let conn = self.conn.lock();
        Ok(BusyWindowQueue::new(&conn).delete_by_source_prefix(source, prefix)?)
    }

    /// Register or re-point an ICS feed subscription (UPSERT on `name`).
    /// Conditional-GET state (`etag` / `last_modified`) is preserved so a
    /// URL change does not force a full re-download from scratch.
    pub fn subscription_upsert(&self, name: &str, url: &str, kind: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO calendar_subscriptions (name, url, kind, enabled, updated_at)
             VALUES (?1, ?2, ?3, 1, unixepoch() * 1000)
             ON CONFLICT(name) DO UPDATE SET
                url        = excluded.url,
                kind       = excluded.kind,
                updated_at = unixepoch() * 1000",
            rusqlite::params![name, url, kind],
        )?;
        Ok(())
    }

    /// List subscriptions, enabled first, then by `name`.
    pub fn subscription_list(&self) -> anyhow::Result<Vec<CalendarSubscriptionRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, url, kind, enabled, etag, last_modified, last_sync_ms, last_status
             FROM calendar_subscriptions
             ORDER BY enabled DESC, name ASC",
        )?;
        let rows = stmt.query_map([], subscription_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Fetch one subscription by `name`.
    pub fn subscription_get(&self, name: &str) -> anyhow::Result<Option<CalendarSubscriptionRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, url, kind, enabled, etag, last_modified, last_sync_ms, last_status
             FROM calendar_subscriptions WHERE name = ?1",
        )?;
        let mut rows = stmt.query([name])?;
        match rows.next()? {
            Some(r) => Ok(Some(subscription_row(r)?)),
            None => Ok(None),
        }
    }

    /// Remove a subscription by `name`; `true` when a row was removed.
    /// Its busy windows are intentionally left behind so the operator can
    /// inspect them (`calendar busy list`) until the next ICS sync.
    pub fn subscription_remove(&self, name: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock();
        let n = conn.execute("DELETE FROM calendar_subscriptions WHERE name = ?1", [name])?;
        Ok(n == 1)
    }

    /// Enable/disable a subscription (disabled feeds are skipped by
    /// `calendar sync-all` and the poller but keep their rows).
    pub fn subscription_set_enabled(&self, name: &str, enabled: bool) -> anyhow::Result<bool> {
        let conn = self.conn.lock();
        let n = conn.execute(
            "UPDATE calendar_subscriptions
                SET enabled = ?2, updated_at = unixepoch() * 1000
              WHERE name = ?1",
            rusqlite::params![name, enabled as i64],
        )?;
        Ok(n == 1)
    }

    /// Record the outcome of a sync: HTTP validators for the next
    /// conditional GET plus a human-readable status. Called by both
    /// `calendar sync-all` and the background poller.
    pub fn subscription_record_sync(
        &self,
        name: &str,
        etag: Option<&str>,
        last_modified: Option<&str>,
        status: &str,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE calendar_subscriptions
                SET etag = ?2, last_modified = ?3,
                    last_sync_ms = unixepoch() * 1000,
                    last_status = ?4,
                    updated_at = unixepoch() * 1000
              WHERE name = ?1",
            rusqlite::params![name, etag, last_modified, status],
        )?;
        Ok(())
    }
}

fn subscription_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<CalendarSubscriptionRow> {
    Ok(CalendarSubscriptionRow {
        id: r.get(0)?,
        name: r.get(1)?,
        url: r.get(2)?,
        kind: r.get(3)?,
        enabled: r.get::<_, i64>(4)? != 0,
        etag: r.get(5)?,
        last_modified: r.get(6)?,
        last_sync_ms: r.get(7)?,
        last_status: r.get(8)?,
    })
}
