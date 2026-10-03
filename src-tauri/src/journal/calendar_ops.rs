// Atlas OS — Journal wrappers for the calendar busy-window queue (RFC 28 §G).
//
// The CLI (`atlas calendar busy …`) and, later, the Planning engine call
// these instead of reaching into the SQLite connection directly. The
// underlying CRUD lives in `crate::calendar::queue::BusyWindowQueue`, which
// is compiled unconditionally so a non-`calendar` build still exposes the
// operator surface.

use super::Journal;
use crate::calendar::queue::{BusyWindowInput, BusyWindowQueue, BusyWindowRow};

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
}
