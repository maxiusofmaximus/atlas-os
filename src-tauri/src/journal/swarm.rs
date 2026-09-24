// Atlas OS — Swarm registry (RFC 05, Phase 4 sub-fase 4.0, M29).
// Journal-backed `swarm_agents` rows: one per agent spawned on a mission.
// Same JSON-blob + denormalised-index pattern as the other artefacts;
// `ON CONFLICT DO NOTHING` honours RFC 02 §3.1.2 (first write wins).
// Mailbox writers/readers (`send_message` / `inbox_for` / `mark_read`)
// land in sub-fase 4.3; the `agent_mailbox` table is materialised by M29.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::swarm::roles::Role;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwarmAgentRow {
    pub id: Uuid,
    pub mission_id: String,
    pub role: String,
    pub model_id: Option<String>,
    pub state: String,
    pub personality_json: Option<String>,
    pub created_at: String,
}

impl SwarmAgentRow {
    pub fn parsed_role(&self) -> Option<Role> {
        Role::parse(&self.role)
    }
}

impl super::Journal {
    pub fn register_swarm_agent(
        &self,
        id: Uuid,
        mission_id: &str,
        role: Role,
        model_id: Option<&str>,
        personality_json: Option<&str>,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO swarm_agents
                (id, mission_id, role, model_id, state, personality_json, created_at)
              VALUES (?1, ?2, ?3, ?4, 'spawned', ?5, ?6)
              ON CONFLICT(id) DO NOTHING",
            rusqlite::params![
                id.to_string(),
                mission_id,
                role.as_str(),
                model_id,
                personality_json,
                now,
            ],
        )?;
        Ok(())
    }

    pub fn set_swarm_agent_state(&self, id: Uuid, state: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE swarm_agents SET state = ?1 WHERE id = ?2",
            rusqlite::params![state, id.to_string()],
        )?;
        Ok(())
    }

    pub fn swarm_agents_for_mission(&self, mission_id: &str) -> anyhow::Result<Vec<SwarmAgentRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, mission_id, role, model_id, state, personality_json, created_at
              FROM swarm_agents WHERE mission_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map(rusqlite::params![mission_id], |row| {
            let id_str: String = row.get(0)?;
            Ok(SwarmAgentRow {
                id: Uuid::parse_str(&id_str).unwrap_or_else(|e| {
                    tracing::warn!(error = %e, raw = %id_str, "swarm agent id parse failed; using nil");
                    Uuid::nil()
                }),
                mission_id: row.get(1)?,
                role: row.get(2)?,
                model_id: row.get(3)?,
                state: row.get(4)?,
                personality_json: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn swarm_registry_happy_path_register_and_list() {
        let tmp = TempDir::new().expect("tmp");
        let journal = super::super::Journal::open(tmp.path()).expect("open");

        let id = Uuid::new_v4();
        journal
            .register_swarm_agent(id, "m1", Role::Backend, Some("gpt-4o"), None)
            .expect("register");

        let agents = journal.swarm_agents_for_mission("m1").expect("list");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, id);
        assert_eq!(agents[0].parsed_role(), Some(Role::Backend));
        assert_eq!(agents[0].model_id.as_deref(), Some("gpt-4o"));
        assert_eq!(agents[0].state, "spawned");
    }

    #[test]
    fn swarm_registry_replay_is_idempotent_first_write_wins() {
        let tmp = TempDir::new().expect("tmp");
        let journal = super::super::Journal::open(tmp.path()).expect("open");

        let id = Uuid::new_v4();
        journal
            .register_swarm_agent(id, "m1", Role::Backend, Some("gpt-4o"), None)
            .expect("register");
        journal
            .register_swarm_agent(id, "m1", Role::Frontend, Some("other"), None)
            .expect("replay");

        let agents = journal.swarm_agents_for_mission("m1").expect("list");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].parsed_role(), Some(Role::Backend));
    }

    #[test]
    fn swarm_registry_migration_is_idempotent_on_reopen() {
        let tmp = TempDir::new().expect("tmp");
        let j1 = super::super::Journal::open(tmp.path()).expect("first open");
        let id = Uuid::new_v4();
        j1.register_swarm_agent(id, "m1", Role::Planner, None, None)
            .expect("register");
        drop(j1);

        let j2 = super::super::Journal::open(tmp.path()).expect("second open");
        let agents = j2.swarm_agents_for_mission("m1").expect("list");
        assert_eq!(agents.len(), 1);

        let version: i64 = j2
            .conn
            .lock()
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |r| r.get(0),
            )
            .expect("version");
        assert!(version >= crate::journal::schema::CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn swarm_registry_set_state_updates_live_row() {
        let tmp = TempDir::new().expect("tmp");
        let journal = super::super::Journal::open(tmp.path()).expect("open");

        let id = Uuid::new_v4();
        journal
            .register_swarm_agent(id, "m1", Role::Testing, None, None)
            .expect("register");
        journal.set_swarm_agent_state(id, "working").expect("state");

        let agents = journal.swarm_agents_for_mission("m1").expect("list");
        assert_eq!(agents[0].state, "working");
    }

    #[test]
    fn swarm_registry_unknown_mission_lists_empty() {
        let tmp = TempDir::new().expect("tmp");
        let journal = super::super::Journal::open(tmp.path()).expect("open");
        let agents = journal.swarm_agents_for_mission("ghost").expect("list");
        assert!(agents.is_empty());
    }
}
