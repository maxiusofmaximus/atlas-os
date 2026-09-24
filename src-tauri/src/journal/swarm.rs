// Atlas OS — Swarm registry + agent mailbox (RFC 05, Phase 4 sub-fases 4.0/4.3, M29).
// Journal-backed `swarm_agents` rows: one per agent spawned on a mission.
// Same JSON-blob + denormalised-index pattern as the other artefacts;
// `ON CONFLICT DO NOTHING` honours RFC 02 §3.1.2 (first write wins).
// Sub-fase 4.3 (munder-difflin, research/31 §A.2): `send_message` /
// `inbox_for` / `mark_read` over the M29 `agent_mailbox` table, plus
// per-agent memory (`swarm_agent` identity + `agent_resume` joining the
// mission's latest RFC 19 checkpoint for exact resumption).

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MailboxMessage {
    pub id: Uuid,
    pub from_agent: Uuid,
    pub to_agent: Uuid,
    pub body_json: String,
    pub read_at: Option<String>,
    pub created_at: String,
}

impl MailboxMessage {
    pub fn is_read(&self) -> bool {
        self.read_at.is_some()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentResume {
    pub agent: SwarmAgentRow,
    pub checkpoint: Option<super::CheckpointRow>,
}

fn parse_agent_id(raw: &str) -> Uuid {
    Uuid::parse_str(raw).unwrap_or_else(|e| {
        tracing::warn!(error = %e, raw = %raw, "swarm agent id parse failed; using nil");
        Uuid::nil()
    })
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

    pub fn swarm_agent(&self, id: Uuid) -> anyhow::Result<Option<SwarmAgentRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, mission_id, role, model_id, state, personality_json, created_at
              FROM swarm_agents WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            Ok(SwarmAgentRow {
                id: parse_agent_id(&id_str),
                mission_id: row.get(1)?,
                role: row.get(2)?,
                model_id: row.get(3)?,
                state: row.get(4)?,
                personality_json: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        if let Some(row) = rows.next().transpose()? {
            return Ok(Some(row));
        }
        Ok(None)
    }

    pub fn agent_resume(&self, id: Uuid) -> anyhow::Result<Option<AgentResume>> {
        let Some(agent) = self.swarm_agent(id)? else {
            return Ok(None);
        };
        let checkpoint = match Uuid::parse_str(&agent.mission_id) {
            Ok(mission_id) => self.latest_checkpoint(mission_id)?,
            Err(e) => {
                tracing::warn!(error = %e, raw = %agent.mission_id, "mission_id is not a UUID; no checkpoint for agent resume");
                None
            }
        };
        Ok(Some(AgentResume { agent, checkpoint }))
    }

    pub fn send_message(
        &self,
        id: Uuid,
        from_agent: Uuid,
        to_agent: Uuid,
        body_json: &str,
    ) -> anyhow::Result<()> {
        if body_json.trim().is_empty() {
            anyhow::bail!("mailbox body must not be empty");
        }
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO agent_mailbox
                (id, from_agent, to_agent, body_json, read_at, created_at)
              VALUES (?1, ?2, ?3, ?4, NULL, ?5)
              ON CONFLICT(id) DO NOTHING",
            rusqlite::params![
                id.to_string(),
                from_agent.to_string(),
                to_agent.to_string(),
                body_json,
                now,
            ],
        )?;
        Ok(())
    }

    fn inbox_query(
        &self,
        to_agent: Uuid,
        unread_only: bool,
    ) -> anyhow::Result<Vec<MailboxMessage>> {
        let conn = self.conn.lock();
        let sql = if unread_only {
            "SELECT id, from_agent, to_agent, body_json, read_at, created_at
              FROM agent_mailbox WHERE to_agent = ?1 AND read_at IS NULL
              ORDER BY created_at ASC"
        } else {
            "SELECT id, from_agent, to_agent, body_json, read_at, created_at
              FROM agent_mailbox WHERE to_agent = ?1
              ORDER BY created_at ASC"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params![to_agent.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let from_str: String = row.get(1)?;
            let to_str: String = row.get(2)?;
            Ok(MailboxMessage {
                id: parse_agent_id(&id_str),
                from_agent: parse_agent_id(&from_str),
                to_agent: parse_agent_id(&to_str),
                body_json: row.get(3)?,
                read_at: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for item in rows.flatten() {
            out.push(item);
        }
        Ok(out)
    }

    pub fn inbox_for(&self, to_agent: Uuid) -> anyhow::Result<Vec<MailboxMessage>> {
        self.inbox_query(to_agent, false)
    }

    pub fn unread_inbox_for(&self, to_agent: Uuid) -> anyhow::Result<Vec<MailboxMessage>> {
        self.inbox_query(to_agent, true)
    }

    pub fn unread_count(&self, to_agent: Uuid) -> anyhow::Result<i64> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM agent_mailbox WHERE to_agent = ?1 AND read_at IS NULL",
            rusqlite::params![to_agent.to_string()],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn mark_read(&self, id: Uuid) -> anyhow::Result<bool> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        let changed = conn.execute(
            "UPDATE agent_mailbox SET read_at = ?1 WHERE id = ?2 AND read_at IS NULL",
            rusqlite::params![now, id.to_string()],
        )?;
        Ok(changed > 0)
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

    #[test]
    fn mailbox_send_inbox_round_trip_in_order() {
        let tmp = TempDir::new().expect("tmp");
        let journal = super::super::Journal::open(tmp.path()).expect("open");

        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        journal
            .send_message(first, from, to, r#"{"text":"hello"}"#)
            .expect("send first");
        journal
            .send_message(second, from, to, r#"{"text":"follow-up"}"#)
            .expect("send second");

        let inbox = journal.inbox_for(to).expect("inbox");
        assert_eq!(inbox.len(), 2);
        assert_eq!(inbox[0].id, first);
        assert_eq!(inbox[0].from_agent, from);
        assert_eq!(inbox[0].to_agent, to);
        assert!(!inbox[0].is_read());
        assert_eq!(inbox[1].id, second);

        assert_eq!(journal.unread_count(to).expect("count"), 2);
        assert!(journal.inbox_for(from).expect("sender inbox").is_empty());
    }

    #[test]
    fn mailbox_replay_is_idempotent_first_write_wins() {
        let tmp = TempDir::new().expect("tmp");
        let journal = super::super::Journal::open(tmp.path()).expect("open");

        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let id = Uuid::new_v4();
        journal
            .send_message(id, from, to, r#"{"text":"original"}"#)
            .expect("send");
        journal
            .send_message(id, from, to, r#"{"text":"replay"}"#)
            .expect("replay");

        let inbox = journal.inbox_for(to).expect("inbox");
        assert_eq!(inbox.len(), 1);
        assert!(inbox[0].body_json.contains("original"));
    }

    #[test]
    fn mailbox_send_rejects_empty_body() {
        let tmp = TempDir::new().expect("tmp");
        let journal = super::super::Journal::open(tmp.path()).expect("open");

        let err = journal
            .send_message(Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), "   ")
            .unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn mailbox_mark_read_is_idempotent() {
        let tmp = TempDir::new().expect("tmp");
        let journal = super::super::Journal::open(tmp.path()).expect("open");

        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let id = Uuid::new_v4();
        journal
            .send_message(id, from, to, r#"{"text":"ping"}"#)
            .expect("send");

        assert!(journal.mark_read(id).expect("first mark"));
        assert!(!journal.mark_read(id).expect("second mark"));
        assert_eq!(journal.unread_count(to).expect("count"), 0);
        assert_eq!(journal.unread_inbox_for(to).expect("unread").len(), 0);
        assert_eq!(journal.inbox_for(to).expect("all").len(), 1);
        assert!(journal.inbox_for(to).expect("all")[0].is_read());
    }

    #[test]
    fn mailbox_mark_read_unknown_id_reports_false() {
        let tmp = TempDir::new().expect("tmp");
        let journal = super::super::Journal::open(tmp.path()).expect("open");
        assert!(!journal.mark_read(Uuid::new_v4()).expect("mark unknown"));
    }

    #[test]
    fn swarm_agent_lookup_returns_none_for_unknown() {
        let tmp = TempDir::new().expect("tmp");
        let journal = super::super::Journal::open(tmp.path()).expect("open");
        assert!(journal
            .swarm_agent(Uuid::new_v4())
            .expect("lookup")
            .is_none());
        assert!(journal
            .agent_resume(Uuid::new_v4())
            .expect("resume")
            .is_none());
    }

    #[test]
    fn agent_resume_joins_agent_row_with_latest_checkpoint() {
        use crate::supervisor::types::{BudgetTally, MissionCheckpoint, MissionPhase};

        let tmp = TempDir::new().expect("tmp");
        let journal = super::super::Journal::open(tmp.path()).expect("open");

        let mission_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        journal
            .register_swarm_agent(agent_id, &mission_id.to_string(), Role::Backend, None, None)
            .expect("register");
        journal
            .save_checkpoint(&MissionCheckpoint {
                checkpoint_id: Uuid::new_v4(),
                mission_id,
                phase: MissionPhase::Executing,
                current_plan_id: None,
                last_validation_report_id: None,
                last_repair_id: None,
                budget_tally: BudgetTally::default(),
                generated_at: chrono::Utc::now().to_rfc3339(),
            })
            .expect("checkpoint");

        let resume = journal
            .agent_resume(agent_id)
            .expect("resume")
            .expect("some");
        assert_eq!(resume.agent.id, agent_id);
        assert_eq!(resume.agent.parsed_role(), Some(Role::Backend));
        let checkpoint = resume.checkpoint.expect("checkpoint joined");
        assert_eq!(checkpoint.mission_id, mission_id);
    }
}
