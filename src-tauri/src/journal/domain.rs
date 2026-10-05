// Atlas OS — Domain Pack persistence (RFC 64 §7, M51 / schema v40).
//
// `domain_packs` records installed/replaced packs (manifest + signature);
// `domain_runs` records a domain-scoped mission execution. Raw fields only —
// the Journal does not depend on `domain` types; the CLI converts.

use serde::{Deserialize, Serialize};

use super::Journal;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomainPackRow {
    pub id: String,
    pub title: String,
    pub version: String,
    pub manifest_toml: String,
    pub sha256: Option<String>,
    pub signed: bool,
    pub installed: bool,
    pub ts: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomainRunRow {
    pub id: String,
    pub mission_id: String,
    pub domain_id: String,
    pub pack_version: String,
    pub status: String,
    pub ts_started: i64,
    pub ts_ended: Option<i64>,
}

impl Journal {
    /// Insert/replace a domain pack by id (idempotent install/override).
    pub fn upsert_domain_pack(&self, row: &DomainPackRow) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO domain_packs
                (id, title, version, manifest_toml, sha256, signed, installed, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                row.id,
                row.title,
                row.version,
                row.manifest_toml,
                row.sha256,
                i64::from(row.signed),
                i64::from(row.installed),
                row.ts,
            ],
        )?;
        Ok(())
    }

    pub fn domain_pack(&self, id: &str) -> anyhow::Result<Option<DomainPackRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, version, manifest_toml, sha256, signed, installed, ts
             FROM domain_packs WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map([id], Self::map_domain_pack)?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    }

    pub fn list_domain_packs(&self) -> anyhow::Result<Vec<DomainPackRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, title, version, manifest_toml, sha256, signed, installed, ts
             FROM domain_packs ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], Self::map_domain_pack)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn map_domain_pack(r: &rusqlite::Row<'_>) -> rusqlite::Result<DomainPackRow> {
        Ok(DomainPackRow {
            id: r.get(0)?,
            title: r.get(1)?,
            version: r.get(2)?,
            manifest_toml: r.get(3)?,
            sha256: r.get(4)?,
            signed: r.get::<_, i64>(5)? != 0,
            installed: r.get::<_, i64>(6)? != 0,
            ts: r.get(7)?,
        })
    }

    pub fn create_domain_run(&self, row: &DomainRunRow) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR IGNORE INTO domain_runs
                (id, mission_id, domain_id, pack_version, status, ts_started, ts_ended)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                row.id,
                row.mission_id,
                row.domain_id,
                row.pack_version,
                row.status,
                row.ts_started,
                row.ts_ended,
            ],
        )?;
        Ok(())
    }

    pub fn finish_domain_run(&self, id: &str, status: &str, ts_ended: i64) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE domain_runs SET status = ?2, ts_ended = ?3 WHERE id = ?1",
            rusqlite::params![id, status, ts_ended],
        )?;
        Ok(())
    }

    pub fn domain_runs_for_mission(&self, mission_id: &str) -> anyhow::Result<Vec<DomainRunRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, mission_id, domain_id, pack_version, status, ts_started, ts_ended
             FROM domain_runs WHERE mission_id = ?1 ORDER BY ts_started ASC",
        )?;
        let rows = stmt.query_map([mission_id], |r| {
            Ok(DomainRunRow {
                id: r.get(0)?,
                mission_id: r.get(1)?,
                domain_id: r.get(2)?,
                pack_version: r.get(3)?,
                status: r.get(4)?,
                ts_started: r.get(5)?,
                ts_ended: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn pack(id: &str) -> DomainPackRow {
        DomainPackRow {
            id: id.into(),
            title: format!("{id} title"),
            version: "0.1.0".into(),
            manifest_toml: format!("[domain]\nid=\"{id}\"\ntitle=\"{id}\"\n"),
            sha256: Some("abc".into()),
            signed: true,
            installed: true,
            ts: 1,
        }
    }

    #[test]
    fn upsert_is_idempotent_and_replaces() {
        let dir = TempDir::new().unwrap();
        let j = Journal::open(dir.path()).unwrap();
        j.upsert_domain_pack(&pack("cad")).unwrap();
        j.upsert_domain_pack(&pack("cad")).unwrap();
        assert_eq!(j.list_domain_packs().unwrap().len(), 1);
        let mut updated = pack("cad");
        updated.version = "2.0.0".into();
        j.upsert_domain_pack(&updated).unwrap();
        assert_eq!(j.domain_pack("cad").unwrap().unwrap().version, "2.0.0");
    }

    #[test]
    fn domain_runs_round_trip_and_finish() {
        let dir = TempDir::new().unwrap();
        let j = Journal::open(dir.path()).unwrap();
        let mission = Uuid::new_v4();
        j.create_mission(mission, "cad mission").unwrap();
        j.create_domain_run(&DomainRunRow {
            id: "run1".into(),
            mission_id: mission.to_string(),
            domain_id: "cad".into(),
            pack_version: "0.1.0".into(),
            status: "running".into(),
            ts_started: 10,
            ts_ended: None,
        })
        .unwrap();
        j.finish_domain_run("run1", "done", 20).unwrap();
        let runs = j.domain_runs_for_mission(&mission.to_string()).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, "done");
        assert_eq!(runs[0].ts_ended, Some(20));
    }
}
