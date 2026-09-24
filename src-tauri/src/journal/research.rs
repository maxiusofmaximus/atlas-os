// Atlas OS — Research runs persistence (RFC 10, Phase 3 sub-fases 3.0 + 3.4,
// M25 `research_runs` / `research_sources` / `research_consensus` + M26
// `research_notes`).
//
// Same JSON-blob-adjacent pattern as the other Journal writers: the
// canonical `ResearchRunReport` YAML (RFC 10 §7) is the source of
// truth consumed by the HUD/CLI; these tables only duplicate the
// columns the SQL engine needs for indexing and tail queries.
// `ON CONFLICT DO NOTHING` / `OR REPLACE` honour RFC 02 §3.1.2
// at-least-once idempotency (first run-row wins, re-scores replace).
// Hands-on notes (RFC 10 §3, sub-fase 3.4) persist the full
// `ResearchNote` with tags as a JSON array string; the per-run
// `journal_ref` (`jr-…`, RFC 10 §7) is a `journal_events` row
// (`kind='research_run'`) so `journal tail` audits every run.

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

    pub fn save_research_note(&self, note: &crate::research::ResearchNote) -> anyhow::Result<()> {
        note.validate()
            .map_err(|e| anyhow::anyhow!("research note invalid: {e}"))?;
        let tags_json = serde_json::to_string(&note.tags).unwrap_or_else(|_| "[]".to_string());
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO research_notes
                (id, title, project, decision, outcome, confidence,
                 tags_json, attached_at, signature, created_at)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
              ON CONFLICT(id) DO NOTHING",
            rusqlite::params![
                note.id,
                note.title,
                note.project.as_deref(),
                note.decision,
                note.outcome.as_deref(),
                note.confidence,
                tags_json,
                note.attached_at,
                note.signature,
                now,
            ],
        )?;
        Ok(())
    }

    pub fn get_research_note(
        &self,
        id: &str,
    ) -> anyhow::Result<Option<crate::research::ResearchNote>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, project, decision, outcome, confidence,
                    tags_json, attached_at, signature
             FROM research_notes WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![id], |row| {
            let tags_json: String = row.get(6)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            Ok(crate::research::ResearchNote {
                id: row.get(0)?,
                title: row.get(1)?,
                project: row.get(2)?,
                decision: row.get(3)?,
                outcome: row.get(4)?,
                confidence: row.get(5)?,
                tags,
                attached_at: row.get(7)?,
                signature: row.get(8)?,
            })
        })?;
        Ok(rows.next().transpose()?)
    }

    pub fn list_research_notes(
        &self,
        limit: i64,
    ) -> anyhow::Result<Vec<crate::research::ResearchNote>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, project, decision, outcome, confidence,
                    tags_json, attached_at, signature
             FROM research_notes ORDER BY attached_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![limit], |row| {
            let tags_json: String = row.get(6)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            Ok(crate::research::ResearchNote {
                id: row.get(0)?,
                title: row.get(1)?,
                project: row.get(2)?,
                decision: row.get(3)?,
                outcome: row.get(4)?,
                confidence: row.get(5)?,
                tags,
                attached_at: row.get(7)?,
                signature: row.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn record_research_journal_ref(
        &self,
        journal_ref: &str,
        run_id: &str,
        query: &str,
    ) -> anyhow::Result<()> {
        let payload = serde_json::json!({
            "type": "research_run",
            "journal_ref": journal_ref,
            "run_id": run_id,
            "query": query,
        });
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO journal_events (event_id, idempotency_key, ts, kind, payload)
             VALUES (?1, ?2, ?3, 'research_run', ?4)
             ON CONFLICT(event_id) DO NOTHING",
            rusqlite::params![journal_ref, journal_ref, now, payload.to_string()],
        )?;
        Ok(())
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

    #[test]
    fn research_note_roundtrips_with_tags() {
        let (_d, j) = fresh();
        let note = crate::research::ResearchNote {
            id: "rn-2026-07-04-001".into(),
            title: "Lo hice así en producción".into(),
            project: Some("fintech X".into()),
            decision: "Event Sourcing + Kafka".into(),
            outcome: Some("exitoso pero costoso en ops".into()),
            confidence: 0.81,
            tags: vec!["architecture".into(), "event-sourcing".into()],
            attached_at: "2026-07-04".into(),
            signature: "operator".into(),
        };
        j.save_research_note(&note).unwrap();
        let back = j
            .get_research_note("rn-2026-07-04-001")
            .unwrap()
            .expect("note");
        assert_eq!(back, note);
        let all = j.list_research_notes(10).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].tags, vec!["architecture", "event-sourcing"]);
    }

    #[test]
    fn research_note_save_is_idempotent_on_replay() {
        let (_d, j) = fresh();
        let note = crate::research::ResearchNote {
            id: "rn-dup".into(),
            title: "t".into(),
            project: None,
            decision: "d".into(),
            outcome: None,
            confidence: 0.7,
            tags: vec![],
            attached_at: "2026-07-04".into(),
            signature: "operator".into(),
        };
        j.save_research_note(&note).unwrap();
        j.save_research_note(&note).unwrap();
        assert_eq!(j.list_research_notes(10).unwrap().len(), 1);
    }

    #[test]
    fn research_note_rejects_invalid_confidence() {
        let (_d, j) = fresh();
        let mut note = crate::research::ResearchNote {
            id: "rn-bad".into(),
            title: "t".into(),
            project: None,
            decision: "d".into(),
            outcome: None,
            confidence: 2.0,
            tags: vec![],
            attached_at: "2026-07-04".into(),
            signature: "operator".into(),
        };
        assert!(j.save_research_note(&note).is_err());
        note.confidence = 0.5;
        note.title = "   ".into();
        assert!(j.save_research_note(&note).is_err());
        assert!(j.get_research_note("rn-bad").unwrap().is_none());
    }

    #[test]
    fn journal_ref_is_auditable_via_tail() {
        let (_d, j) = fresh();
        j.record_research_journal_ref("jr-2026-07-04-001", "rr-2026-07-04-001", "q")
            .unwrap();
        j.record_research_journal_ref("jr-2026-07-04-001", "rr-2026-07-04-001", "q")
            .unwrap();
        let entries = j.tail(10).unwrap();
        let found = entries.iter().filter(|e| e.kind == "research_run").count();
        assert_eq!(found, 1, "journal_ref must appear once in the audit tail");
    }
}
