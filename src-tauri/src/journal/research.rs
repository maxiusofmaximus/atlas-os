// Atlas OS — Research runs persistence (RFC 10, Phase 3 sub-fase 3.0,
// M25 `research_runs` / `research_sources` / `research_consensus`).
//
// Same JSON-blob-adjacent pattern as the other Journal writers: the
// canonical `ResearchRunReport` YAML (RFC 10 §7) is the source of
// truth consumed by the HUD/CLI; these tables only duplicate the
// columns the SQL engine needs for indexing and tail queries.
// `ON CONFLICT DO NOTHING` / `OR REPLACE` honour RFC 02 §3.1.2
// at-least-once idempotency (first run-row wins, re-scores replace).

use serde::{Deserialize, Serialize};

/// Row projection of `research_runs` for tails and HUD lists.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResearchRunRow {
    pub id: String,
    pub query: String,
    pub status: Option<String>,
    pub confidence: Option<f64>,
    pub recommended: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// Row projection of `research_sources`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResearchSourceRow {
    pub id: String,
    pub run_id: String,
    pub kind: String,
    pub url: String,
    pub score: Option<f64>,
    pub fetched_at: String,
}

/// Row projection of `research_consensus`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResearchConsensusRow {
    pub run_id: String,
    pub dimension: String,
    pub score: f64,
    pub note: Option<String>,
}

impl crate::journal::Journal {
    pub fn create_research_run(
        &self,
        id: &str,
        query: &str,
        status: &crate::research::ResearchRunStatus,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO research_runs (id, query, status, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO NOTHING",
            rusqlite::params![id, query, status.as_str(), now],
        )?;
        Ok(())
    }

    pub fn complete_research_run(
        &self,
        id: &str,
        status: &crate::research::ResearchRunStatus,
        confidence: f64,
        recommended: &str,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE research_runs
             SET status = ?1, confidence = ?2, recommended = ?3, completed_at = ?4
             WHERE id = ?5",
            rusqlite::params![status.as_str(), confidence, recommended, now, id],
        )?;
        Ok(())
    }

    pub fn get_research_run(&self, id: &str) -> anyhow::Result<Option<ResearchRunRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, query, status, confidence, recommended, created_at, completed_at
             FROM research_runs WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![id], |row| {
            Ok(ResearchRunRow {
                id: row.get(0)?,
                query: row.get(1)?,
                status: row.get(2)?,
                confidence: row.get(3)?,
                recommended: row.get(4)?,
                created_at: row.get(5)?,
                completed_at: row.get(6)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    pub fn add_research_source(
        &self,
        id: &str,
        run_id: &str,
        kind: &str,
        url: &str,
        score: Option<f64>,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO research_sources (id, run_id, kind, url, score, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO NOTHING",
            rusqlite::params![id, run_id, kind, url, score, now],
        )?;
        Ok(())
    }

    pub fn list_research_sources(&self, run_id: &str) -> anyhow::Result<Vec<ResearchSourceRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, kind, url, score, fetched_at
             FROM research_sources WHERE run_id = ?1 ORDER BY fetched_at ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![run_id], |row| {
            Ok(ResearchSourceRow {
                id: row.get(0)?,
                run_id: row.get(1)?,
                kind: row.get(2)?,
                url: row.get(3)?,
                score: row.get(4)?,
                fetched_at: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn save_research_consensus(
        &self,
        run_id: &str,
        dimension: &crate::research::ConsensusDimension,
        score: f64,
        note: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO research_consensus (run_id, dimension, score, note)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![run_id, dimension.as_str(), score, note],
        )?;
        Ok(())
    }

    pub fn list_research_consensus(
        &self,
        run_id: &str,
    ) -> anyhow::Result<Vec<ResearchConsensusRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT run_id, dimension, score, note
             FROM research_consensus WHERE run_id = ?1 ORDER BY dimension ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![run_id], |row| {
            Ok(ResearchConsensusRow {
                run_id: row.get(0)?,
                dimension: row.get(1)?,
                score: row.get(2)?,
                note: row.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use crate::journal::Journal;
    use tempfile::TempDir;

    fn fresh() -> (TempDir, Journal) {
        let dir = TempDir::new().unwrap();
        let j = Journal::open(dir.path()).unwrap();
        (dir, j)
    }

    #[test]
    fn create_and_complete_run_roundtrip() {
        let (_d, j) = fresh();
        j.create_research_run(
            "rr-001",
            "Event sourcing vs CRDT?",
            &crate::research::ResearchRunStatus::Running,
        )
        .unwrap();
        let row = j.get_research_run("rr-001").unwrap().expect("row");
        assert_eq!(row.status.as_deref(), Some("running"));

        j.complete_research_run(
            "rr-001",
            &crate::research::ResearchRunStatus::Completed,
            0.81,
            "A",
        )
        .unwrap();
        let done = j.get_research_run("rr-001").unwrap().expect("row");
        assert_eq!(done.status.as_deref(), Some("completed"));
        assert_eq!(done.recommended.as_deref(), Some("A"));
        assert!(done.completed_at.is_some());
    }

    #[test]
    fn create_run_is_idempotent_on_replay() {
        let (_d, j) = fresh();
        j.create_research_run("rr-dup", "q1", &crate::research::ResearchRunStatus::Running)
            .unwrap();
        j.create_research_run("rr-dup", "q1", &crate::research::ResearchRunStatus::Running)
            .unwrap();
        assert!(j.get_research_run("rr-dup").unwrap().is_some());
    }

    #[test]
    fn sources_and_consensus_persist() {
        let (_d, j) = fresh();
        j.create_research_run("rr-002", "q", &crate::research::ResearchRunStatus::Running)
            .unwrap();
        j.add_research_source(
            "rs-1",
            "rr-002",
            "documentation",
            "https://example.com",
            Some(0.9),
        )
        .unwrap();
        j.save_research_consensus(
            "rr-002",
            &crate::research::ConsensusDimension::Official,
            78.0,
            Some("docs recommend X"),
        )
        .unwrap();
        let sources = j.list_research_sources("rr-002").unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].url, "https://example.com");
        let consensus = j.list_research_consensus("rr-002").unwrap();
        assert_eq!(consensus.len(), 1);
        assert_eq!(consensus[0].dimension, "official");
    }

    #[test]
    fn consensus_rescore_replaces_same_dimension() {
        let (_d, j) = fresh();
        j.create_research_run("rr-003", "q", &crate::research::ResearchRunStatus::Running)
            .unwrap();
        j.save_research_consensus(
            "rr-003",
            &crate::research::ConsensusDimension::Community,
            60.0,
            None,
        )
        .unwrap();
        j.save_research_consensus(
            "rr-003",
            &crate::research::ConsensusDimension::Community,
            71.0,
            None,
        )
        .unwrap();
        let rows = j.list_research_consensus("rr-003").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].score, 71.0);
    }

    #[test]
    fn unknown_run_returns_none() {
        let (_d, j) = fresh();
        assert!(j.get_research_run("rr-missing").unwrap().is_none());
        assert!(j.list_research_sources("rr-missing").unwrap().is_empty());
    }
}
