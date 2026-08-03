// OpenCode OS — Calendar BusyWindow SQLite CRUD (RFC 28 Section G).
//
// Platform-agnostic CRUD over the `calendar_busy_windows` table (M18).
// Two callers populate it:
//
// * the MS Graph poller (`calendar-graph` feature, `graph_reader.rs`)
//   UPSERTs events polled from `/me/calendarView` with `source=graph`.
// * the local ICS-subscription poller (post-MVP) UPSERTs events with
//   `source=ics_local`.
// * the CLI `opencode calendar busy add` (post-MVP) INSERTs with
//   `source=manual`.
//
// The Planning engine consults `overlapping(start, end)` to know if a
// turn's `[start, start + eta)` overlaps any busy window above the
// configured weight threshold.

use std::str::FromStr;

use rusqlite::Connection;

use crate::calendar::error::{CalendarError, Result};
use crate::calendar::payload::{BusySource, BusyWindow};

/// Bundled input for `BusyWindowQueue::upsert` / `insert_manual`.
/// Keeps the call sites under clippy's `too_many_arguments` threshold
/// (7) and gives the Graph poller a typed shape to construct per
/// `/me/calendarView` event.
#[derive(Clone, Debug)]
pub struct BusyWindowInput<'a> {
    pub source: BusySource,
    pub external_id: &'a str,
    pub subject: &'a str,
    pub body: Option<&'a str>,
    pub starts_at: i64,
    pub ends_at: i64,
    pub weight: f64,
}

impl<'a> BusyWindowInput<'a> {
    fn validate(&self) -> Result<()> {
        if self.starts_at >= self.ends_at {
            return Err(CalendarError::InvalidRange {
                starts: self.starts_at,
                ends: self.ends_at,
            });
        }
        Ok(())
    }
}

/// SQL row projection of `calendar_busy_windows`.
#[derive(Clone, Debug, PartialEq)]
pub struct BusyWindowRow {
    pub id: i64,
    pub source: BusySource,
    pub external_id: String,
    pub subject: String,
    pub body: Option<String>,
    pub starts_at: i64,
    pub ends_at: i64,
    pub weight: f64,
    pub recorded_at: i64,
}

impl From<BusyWindowRow> for BusyWindow {
    fn from(r: BusyWindowRow) -> Self {
        Self {
            id: r.id,
            source: r.source,
            external_id: r.external_id,
            subject: r.subject,
            body: r.body,
            starts_at: r.starts_at,
            ends_at: r.ends_at,
            weight: r.weight,
        }
    }
}

/// Window onto the `calendar_busy_windows` table. The underlying
/// `Connection` is owned by `Journal`; we accept a `&Connection` (the
/// SQLite write lock is taken transiently by `execute`/`prepare`).
pub struct BusyWindowQueue<'a> {
    conn: &'a Connection,
}

impl<'a> BusyWindowQueue<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// UPSERT a busy window. The `(source, external_id)` UNIQUE
    /// constraint dedupes the same external event across poller
    /// iterations. `weight` should already be in `[0.0, 1.0]`. Returns
    /// the row id (existing on update, new on insert).
    pub fn upsert(&self, input: &BusyWindowInput<'_>) -> Result<i64> {
        input.validate()?;
        self.conn.execute(
            "INSERT INTO calendar_busy_windows
                (source, external_id, subject, body, starts_at, ends_at, weight)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(source, external_id) DO UPDATE SET
                subject     = excluded.subject,
                body        = excluded.body,
                starts_at   = excluded.starts_at,
                ends_at     = excluded.ends_at,
                weight      = excluded.weight,
                recorded_at = unixepoch() * 1000",
            rusqlite::params![
                input.source.as_str(),
                input.external_id,
                input.subject,
                input.body,
                input.starts_at,
                input.ends_at,
                input.weight,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Insert a manual busy window. Manual rows are never auto-deduped
    /// (caller-supplied external_id) so the operator can stack multiple
    /// reminders on the same logical external event. Returns the new
    /// row id. The `source` field of `input` is overridden to `manual`.
    pub fn insert_manual(&self, input: &BusyWindowInput<'_>) -> Result<i64> {
        input.validate()?;
        self.conn.execute(
            "INSERT INTO calendar_busy_windows
                (source, external_id, subject, body, starts_at, ends_at, weight)
             VALUES ('manual', ?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                input.external_id,
                input.subject,
                input.body,
                input.starts_at,
                input.ends_at,
                input.weight,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Delete a row by id. Returns `true` if a row was removed.
    pub fn delete(&self, id: i64) -> Result<bool> {
        let n = self
            .conn
            .execute("DELETE FROM calendar_busy_windows WHERE id = ?1", [id])?;
        Ok(n == 1)
    }

    /// Delete all rows from a given source. Used by the Graph poller
    /// to evict stale `graph` rows that no longer appear in the
    /// `/me/calendarView` response.
    pub fn delete_by_source(&self, source: BusySource) -> Result<i64> {
        let n = self.conn.execute(
            "DELETE FROM calendar_busy_windows WHERE source = ?1",
            [source.as_str()],
        )?;
        Ok(n as i64)
    }

    /// Look up the row by id.
    pub fn get(&self, id: i64) -> Result<Option<BusyWindowRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, external_id, subject, body, starts_at, ends_at, weight, recorded_at
             FROM calendar_busy_windows WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        match rows.next()? {
            Some(r) => Ok(Some(row_to_busywindowrow(r)?)),
            None => Ok(None),
        }
    }

    /// Return all busy windows whose `[starts_at, ends_at)` overlaps
    /// `[start_ms, end_ms)` — the Planning engine's primary query.
    /// Half-open intervals: an event ending at `t` does NOT block a
    /// turn starting at `t`. Ordered by `starts_at ASC` so callers can
    /// iterate deterministically.
    pub fn overlapping(&self, start_ms: i64, end_ms: i64) -> Result<Vec<BusyWindowRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, external_id, subject, body, starts_at, ends_at, weight, recorded_at
             FROM calendar_busy_windows
             WHERE starts_at < ?2 AND ?1 < ends_at
             ORDER BY starts_at ASC",
        )?;
        let rows = stmt.query_map([start_ms, end_ms], row_to_busywindowrow)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Return all rows from a given source — used by the Graph poller
    /// to diff against a fresh `/me/calendarView` response and decide
    /// which external_ids to delete (no longer present in the calendar)
    /// vs. update (subject/body shifted).
    pub fn list_by_source(&self, source: BusySource) -> Result<Vec<BusyWindowRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, external_id, subject, body, starts_at, ends_at, weight, recorded_at
             FROM calendar_busy_windows
             WHERE source = ?1
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([source.as_str()], row_to_busywindowrow)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// List recent rows (newest first), capped at `limit`. Used by the
    /// `opencode calendar busy list` CLI debugging surface.
    pub fn list(&self, limit: i64) -> Result<Vec<BusyWindowRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, external_id, subject, body, starts_at, ends_at, weight, recorded_at
             FROM calendar_busy_windows
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], row_to_busywindowrow)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Total row count — mostly for tests.
    pub fn count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT COUNT(*) FROM calendar_busy_windows", [], |r| {
                r.get(0)
            })?)
    }
}

fn row_to_busywindowrow(r: &rusqlite::Row<'_>) -> rusqlite::Result<BusyWindowRow> {
    let source_str: String = r.get(1)?;
    let source = BusySource::from_str(&source_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(BusyWindowRow {
        id: r.get(0)?,
        source,
        external_id: r.get(2)?,
        subject: r.get(3)?,
        body: r.get(4)?,
        starts_at: r.get(5)?,
        ends_at: r.get(6)?,
        weight: r.get(7)?,
        recorded_at: r.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::payload::BusySource;

    fn fresh_queue() -> BusyWindowQueue<'static> {
        // SAFETY: same pattern as `toast::queue::fresh_queue`. The
        // leaked in-memory conn lives for the test thread duration.
        let conn = Box::leak(Box::new(rusqlite::Connection::open_in_memory().unwrap()));
        crate::journal::schema::migrate(conn).unwrap();
        BusyWindowQueue::new(conn)
    }

    fn input<'a>(
        source: BusySource,
        external_id: &'a str,
        subject: &'a str,
        starts_at: i64,
        ends_at: i64,
        weight: f64,
    ) -> BusyWindowInput<'a> {
        BusyWindowInput {
            source,
            external_id,
            subject,
            body: None,
            starts_at,
            ends_at,
            weight,
        }
    }

    #[test]
    fn upsert_inserts_and_returns_id() {
        let q = fresh_queue();
        let id = q
            .upsert(&input(
                BusySource::Graph,
                "ev-1",
                "Sprint planning",
                1_000,
                2_000,
                1.0,
            ))
            .unwrap();
        assert!(id > 0);
        assert_eq!(q.count().unwrap(), 1);
    }

    #[test]
    fn upsert_replaces_on_conflict() {
        let q = fresh_queue();
        q.upsert(&input(
            BusySource::Graph,
            "ev-1",
            "first",
            1_000,
            2_000,
            1.0,
        ))
        .unwrap();
        let mut second = input(BusySource::Graph, "ev-1", "second", 3_000, 4_000, 0.5);
        second.body = Some("body");
        q.upsert(&second).unwrap();
        assert_eq!(q.count().unwrap(), 1, "UNIQUE constraint should dedupe");
        let row = q.list(10).unwrap().pop().unwrap();
        assert_eq!(row.subject, "second");
        assert_eq!(row.starts_at, 3_000);
        assert!((row.weight - 0.5).abs() < 1e-6);
    }

    #[test]
    fn insert_manual_works() {
        let q = fresh_queue();
        let id = q
            .insert_manual(&input(
                BusySource::Manual,
                "ev-2",
                "lunch",
                1_000,
                2_000,
                0.5,
            ))
            .unwrap();
        let row = q.get(id).unwrap().unwrap();
        assert_eq!(row.source, BusySource::Manual);
        assert!((row.weight - 0.5).abs() < 1e-6);
    }

    #[test]
    fn upsert_rejects_invalid_range() {
        let q = fresh_queue();
        let err = q
            .upsert(&input(BusySource::Graph, "ev", "x", 2_000, 1_000, 1.0))
            .unwrap_err();
        assert!(
            matches!(err, CalendarError::InvalidRange { .. }),
            "got: {err:?}"
        );
    }

    #[test]
    fn upsert_rejects_zero_duration() {
        let q = fresh_queue();
        let err = q
            .upsert(&input(BusySource::Graph, "ev", "x", 1_000, 1_000, 1.0))
            .unwrap_err();
        assert!(
            matches!(err, CalendarError::InvalidRange { .. }),
            "got: {err:?}"
        );
    }

    #[test]
    fn delete_by_id_removes_row() {
        let q = fresh_queue();
        let id = q
            .upsert(&input(BusySource::Graph, "ev", "x", 1_000, 2_000, 1.0))
            .unwrap();
        assert!(q.delete(id).unwrap());
        assert!(q.get(id).unwrap().is_none());
    }

    #[test]
    fn delete_returns_false_for_missing_id() {
        let q = fresh_queue();
        assert!(!q.delete(42).unwrap());
    }

    #[test]
    fn delete_by_source_only_affects_that_source() {
        let q = fresh_queue();
        q.upsert(&input(BusySource::Graph, "g-1", "g1", 1_000, 2_000, 1.0))
            .unwrap();
        q.upsert(&input(BusySource::Graph, "g-2", "g2", 1_000, 2_000, 1.0))
            .unwrap();
        q.insert_manual(&input(BusySource::Manual, "m-1", "m1", 1_000, 2_000, 0.5))
            .unwrap();
        let n = q.delete_by_source(BusySource::Graph).unwrap();
        assert_eq!(n, 2);
        assert_eq!(q.count().unwrap(), 1, "manual row should survive");
    }

    #[test]
    fn overlapping_returns_intersecting_events_only() {
        let q = fresh_queue();
        q.upsert(&input(BusySource::Graph, "a", "A", 1_000, 2_000, 1.0))
            .unwrap();
        q.upsert(&input(BusySource::Graph, "b", "B", 3_000, 4_000, 1.0))
            .unwrap();
        q.upsert(&input(BusySource::Graph, "c", "C", 5_000, 6_000, 1.0))
            .unwrap();
        // query [2500, 3500) → should hit B (3000-4000) only.
        let hits = q.overlapping(2_500, 3_500).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].external_id, "b");
    }

    #[test]
    fn overlapping_half_open_boundaries() {
        let q = fresh_queue();
        q.upsert(&input(BusySource::Graph, "x", "X", 1_000, 2_000, 1.0))
            .unwrap();
        // query [2000, 3000) — starts_at = ends_at; no overlap (half-open).
        assert!(q.overlapping(2_000, 3_000).unwrap().is_empty());
        // query [500, 1000) — ends_at = starts_at; no overlap.
        assert!(q.overlapping(500, 1_000).unwrap().is_empty());
        // query [1500, 2500) — proper overlap.
        assert_eq!(q.overlapping(1_500, 2_500).unwrap().len(), 1);
    }

    #[test]
    fn list_by_source_orders_by_id_asc() {
        let q = fresh_queue();
        let a = q
            .upsert(&input(BusySource::Graph, "a", "a", 1_000, 2_000, 1.0))
            .unwrap();
        let b = q
            .upsert(&input(BusySource::Graph, "b", "b", 1_000, 2_000, 1.0))
            .unwrap();
        let c = q
            .insert_manual(&input(BusySource::Manual, "c", "c", 1_000, 2_000, 0.5))
            .unwrap();
        let _ = (a, b, c);
        let graph_rows = q.list_by_source(BusySource::Graph).unwrap();
        assert_eq!(graph_rows.len(), 2);
        assert!(graph_rows[0].id < graph_rows[1].id);
    }

    #[test]
    fn list_orders_by_id_desc_with_limit() {
        let q = fresh_queue();
        for i in 0..5 {
            q.upsert(&input(
                BusySource::Graph,
                &format!("e{i}"),
                "x",
                1_000,
                2_000,
                1.0,
            ))
            .unwrap();
        }
        let rows = q.list(3).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows[0].id > rows[1].id);
        assert!(rows[1].id > rows[2].id);
    }

    #[test]
    fn get_returns_none_for_missing() {
        let q = fresh_queue();
        assert!(q.get(999).unwrap().is_none());
    }
}
