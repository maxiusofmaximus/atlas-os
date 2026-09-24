// Atlas OS — Compaction events persistence (RFC 32 Phase 5 sub-fase 5.3,
// M30 `compaction_events`).
//
// Same JSON-free pattern as the sibling writers: the stub summary is
// plain text (headlines + per-kind counts from
// `learning::compaction::summarize`), so the row stores it verbatim and
// denormalises `mission_id` + the before/after counters for the HUD
// tail. Mission linkage into `journal_events` is by payload substring
// (`payload LIKE '%<mission_id>%'`): `journal_events` carries no
// `mission_id` column, and the mission id (UUID hex + hyphens) needs
// no LIKE escaping.

use crate::journal::store::{CompactionEventRow, JournalEntry};
use crate::learning::compaction::CompactionSummary;

fn row_from(
    id: String,
    mission_id: String,
    entries_before: i64,
    entries_after: i64,
    summary: String,
    model_id: String,
    created_at: String,
) -> CompactionEventRow {
    CompactionEventRow {
        id,
        mission_id,
        entries_before,
        entries_after,
        summary,
        model_id,
        created_at,
    }
}

impl crate::journal::Journal {
    pub fn save_compaction_event(&self, summary: &CompactionSummary) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO compaction_events
                (id, mission_id, entries_before, entries_after, summary, model_id, created_at)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
              ON CONFLICT(id) DO NOTHING",
            rusqlite::params![
                id,
                summary.mission_id,
                summary.entries_before as i64,
                summary.entries_after as i64,
                summary.summary,
                summary.model_id,
                now,
            ],
        )?;
        Ok(id)
    }

    pub fn compacted_summary(&self, mission_id: &str) -> anyhow::Result<Option<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT summary FROM compaction_events
              WHERE mission_id = ?1 ORDER BY created_at DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![mission_id], |row| {
            let summary: String = row.get(0)?;
            Ok(summary)
        })?;
        Ok(rows.next().transpose()?)
    }

    pub fn compaction_history(
        &self,
        mission_id: &str,
        limit: i64,
    ) -> anyhow::Result<Vec<CompactionEventRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, mission_id, entries_before, entries_after, summary, model_id, created_at
              FROM compaction_events
              WHERE mission_id = ?1 ORDER BY created_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![mission_id, limit], |row| {
            Ok(row_from(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Entries whose payload mentions `mission_id`, oldest-first
    /// (mirrors `tail`'s inner-newest/outer-oldest shape).
    pub fn mission_entries(
        &self,
        mission_id: &str,
        limit: i64,
    ) -> anyhow::Result<Vec<JournalEntry>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, ts, kind, payload FROM (
                SELECT id, ts, kind, payload FROM journal_events
                WHERE payload LIKE '%' || ?1 || '%'
                ORDER BY id DESC LIMIT ?2
              ) ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![mission_id, limit], |row| {
            Ok(JournalEntry {
                id: row.get(0)?,
                ts: row.get(1)?,
                kind: row.get(2)?,
                payload: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    pub fn mission_entry_count(&self, mission_id: &str) -> anyhow::Result<usize> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM journal_events WHERE payload LIKE '%' || ?1 || '%'",
            rusqlite::params![mission_id],
            |row| row.get(0),
        )?;
        Ok(count.max(0) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::bus::{BusEvent, BusEventKind};
    use crate::journal::Journal;
    use tempfile::TempDir;

    fn fresh() -> (TempDir, Journal) {
        let dir = TempDir::new().unwrap();
        let j = Journal::open(dir.path()).unwrap();
        (dir, j)
    }

    fn publish_for(j: &Journal, mission_id: uuid::Uuid, n: usize) {
        for _ in 0..n {
            j.publish(&BusEvent::new(BusEventKind::MissionSteered {
                mission_id,
                message: "steer".into(),
            }))
            .unwrap();
        }
    }

    fn summary_fixture(mission: &str) -> CompactionSummary {
        CompactionSummary {
            mission_id: mission.into(),
            entries_before: 120,
            entries_after: 20,
            summary: "mission m: 120 entries across 1 kinds (mission_steered:120)".into(),
            model_id: crate::learning::compaction::COMPACTION_MODEL_ID.into(),
        }
    }

    #[test]
    fn save_and_compacted_summary_roundtrip() {
        let (_d, j) = fresh();
        assert!(j.compacted_summary("m-1").unwrap().is_none());
        j.save_compaction_event(&summary_fixture("m-1")).unwrap();
        let back = j.compacted_summary("m-1").unwrap().expect("row");
        assert!(back.contains("120 entries"));
        assert!(j.compacted_summary("m-other").unwrap().is_none());
    }

    #[test]
    fn history_lists_newest_first_per_mission() {
        let (_d, j) = fresh();
        j.save_compaction_event(&summary_fixture("m-a")).unwrap();
        j.save_compaction_event(&summary_fixture("m-a")).unwrap();
        j.save_compaction_event(&summary_fixture("m-b")).unwrap();
        let rows = j.compaction_history("m-a", 10).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.mission_id == "m-a"));
        assert_eq!(rows[0].entries_before, 120);
        assert_eq!(rows[0].entries_after, 20);
    }

    #[test]
    fn mission_entries_filter_by_payload_and_count() {
        let (_d, j) = fresh();
        let wanted = uuid::Uuid::new_v4();
        let other = uuid::Uuid::new_v4();
        publish_for(&j, wanted, 3);
        publish_for(&j, other, 2);
        assert_eq!(j.mission_entry_count(&wanted.to_string()).unwrap(), 3);
        assert_eq!(j.mission_entry_count(&other.to_string()).unwrap(), 2);
        let entries = j.mission_entries(&wanted.to_string(), 10).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries.windows(2).all(|w| w[0].id < w[1].id));
        assert_eq!(j.mission_entry_count("m-missing").unwrap(), 0);
        assert!(j.mission_entries("m-missing", 10).unwrap().is_empty());
    }
}
