// Atlas OS — User modeling persistence (RFC 29 §3.C, M28 `user_profile`).
//
// One row per operator (`user_id`). `preferences` carries
// `{favorite_models, coding_style, …}` as JSON text; `knowledge_state`
// carries `{domains_known, gaps_identified}` as JSON text.
// `interaction_history_summary` is a free-form summary of past
// interactions; `updated_at` is unix-millis. Writes are upserts — the
// latest snapshot wins (live state, like `step_states` M11). Reads feed
// the Prompt Understanding Pipeline (RFC 23 §2 paso 2) to resolve
// `user_knowledge_gap` (C6): when the prompt matches a `gaps_identified`
// entry, the detector offers mentor mode + shortcuts automatically.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    #[serde(default = "default_json_object")]
    pub preferences: serde_json::Value,
    #[serde(default = "default_json_object")]
    pub knowledge_state: serde_json::Value,
    #[serde(default)]
    pub interaction_history_summary: Option<String>,
    #[serde(default)]
    pub updated_at: i64,
}

fn default_json_object() -> serde_json::Value {
    serde_json::Value::Object(Default::default())
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum UserProfileError {
    #[error("user_id must not be empty")]
    EmptyUserId,
    #[error("preferences must be a JSON object")]
    PreferencesNotObject,
    #[error("knowledge_state must be a JSON object")]
    KnowledgeStateNotObject,
    #[error("updated_at must be a positive unix-millis timestamp")]
    BadUpdatedAt,
}

impl UserProfile {
    pub fn new(
        user_id: &str,
        preferences: serde_json::Value,
        knowledge_state: serde_json::Value,
    ) -> Self {
        Self {
            user_id: user_id.to_string(),
            preferences,
            knowledge_state,
            interaction_history_summary: None,
            updated_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn validate(&self) -> Result<(), UserProfileError> {
        if self.user_id.trim().is_empty() {
            return Err(UserProfileError::EmptyUserId);
        }
        if !self.preferences.is_object() {
            return Err(UserProfileError::PreferencesNotObject);
        }
        if !self.knowledge_state.is_object() {
            return Err(UserProfileError::KnowledgeStateNotObject);
        }
        if self.updated_at <= 0 {
            return Err(UserProfileError::BadUpdatedAt);
        }
        Ok(())
    }

    pub fn gaps_identified(&self) -> Vec<String> {
        self.knowledge_state
            .get("gaps_identified")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .filter(|s| !s.trim().is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn matches_known_gap(&self, raw: &str) -> Option<String> {
        let lower = raw.to_ascii_lowercase();
        for gap in self.gaps_identified() {
            let g = gap.to_ascii_lowercase();
            let trimmed = g.trim();
            if trimmed.is_empty() {
                continue;
            }
            if lower.contains(trimmed) {
                return Some(gap);
            }
            if trimmed.len() > 3
                && trimmed
                    .split_whitespace()
                    .any(|w| w.len() > 3 && lower.contains(w))
            {
                return Some(gap);
            }
        }
        None
    }
}

impl crate::journal::Journal {
    pub fn save_user_profile(&self, profile: &UserProfile) -> anyhow::Result<()> {
        profile
            .validate()
            .map_err(|e| anyhow::anyhow!("user profile invalid: {e}"))?;
        let preferences = serde_json::to_string(&profile.preferences)?;
        let knowledge_state = serde_json::to_string(&profile.knowledge_state)?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO user_profile
                (user_id, preferences, knowledge_state, interaction_history_summary, updated_at)
              VALUES (?1, ?2, ?3, ?4, ?5)
              ON CONFLICT(user_id) DO UPDATE SET
                preferences = excluded.preferences,
                knowledge_state = excluded.knowledge_state,
                interaction_history_summary = excluded.interaction_history_summary,
                updated_at = excluded.updated_at",
            rusqlite::params![
                profile.user_id,
                preferences,
                knowledge_state,
                profile.interaction_history_summary,
                profile.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn get_user_profile(&self, user_id: &str) -> anyhow::Result<Option<UserProfile>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT user_id, preferences, knowledge_state, interaction_history_summary, updated_at
              FROM user_profile WHERE user_id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![user_id], |row| {
            let preferences_raw: String = row.get(1)?;
            let knowledge_raw: String = row.get(2)?;
            Ok((
                row.get::<_, String>(0)?,
                preferences_raw,
                knowledge_raw,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?;
        match rows.next().transpose()? {
            Some((user_id, preferences_raw, knowledge_raw, summary, updated_at)) => {
                let preferences: serde_json::Value = serde_json::from_str(&preferences_raw)?;
                let knowledge_state: serde_json::Value = serde_json::from_str(&knowledge_raw)?;
                Ok(Some(UserProfile {
                    user_id,
                    preferences,
                    knowledge_state,
                    interaction_history_summary: summary,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::Journal;
    use tempfile::TempDir;

    fn fresh() -> (TempDir, Journal) {
        let dir = TempDir::new().unwrap();
        let j = Journal::open(dir.path()).unwrap();
        (dir, j)
    }

    fn profile(user_id: &str) -> UserProfile {
        UserProfile {
            user_id: user_id.into(),
            preferences: serde_json::json!({"favorite_models": ["claude"], "coding_style": "tabs"}),
            knowledge_state: serde_json::json!({"domains_known": ["rust"], "gaps_identified": ["kubernetes ingress"]}),
            interaction_history_summary: Some("prefers terse diffs".into()),
            updated_at: chrono::Utc::now().timestamp_millis(),
        }
    }

    #[test]
    fn save_and_get_roundtrip() {
        let (_d, j) = fresh();
        assert!(j.get_user_profile("op").unwrap().is_none());
        let p = profile("op");
        j.save_user_profile(&p).unwrap();
        let back = j.get_user_profile("op").unwrap().expect("row");
        assert_eq!(back, p);
    }

    #[test]
    fn upsert_replaces_latest_snapshot() {
        let (_d, j) = fresh();
        let mut p = profile("op");
        j.save_user_profile(&p).unwrap();
        p.preferences = serde_json::json!({"coding_style": "spaces"});
        p.updated_at = chrono::Utc::now().timestamp_millis();
        j.save_user_profile(&p).unwrap();
        let back = j.get_user_profile("op").unwrap().expect("row");
        assert_eq!(back.preferences, p.preferences);
    }

    #[test]
    fn rejects_invalid_profile() {
        let (_d, j) = fresh();
        let mut p = profile("op");
        p.user_id = "   ".into();
        assert!(j.save_user_profile(&p).is_err());
        let mut bad = profile("op2");
        bad.preferences = serde_json::json!(["not", "an", "object"]);
        assert!(j.save_user_profile(&bad).is_err());
        let mut bad2 = profile("op3");
        bad2.knowledge_state = serde_json::json!("scalar");
        assert!(j.save_user_profile(&bad2).is_err());
        assert!(j.get_user_profile("op2").unwrap().is_none());
    }

    #[test]
    fn gaps_identified_and_match() {
        let p = profile("op");
        assert_eq!(p.gaps_identified(), vec!["kubernetes ingress"]);
        assert_eq!(
            p.matches_known_gap("how do I expose this via kubernetes ingress?"),
            Some("kubernetes ingress".into())
        );
        assert!(p.matches_known_gap("refactor the billing module").is_none());
    }

    #[test]
    fn m28_creates_user_profile_table_and_bumps_version() {
        let (_d, j) = fresh();
        let _ = j;
        let dir = TempDir::new().unwrap();
        let journal = Journal::open(dir.path()).unwrap();
        let _ = journal;
        let conn = rusqlite::Connection::open(dir.path().join("journal.db")).unwrap();
        let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='user_profile'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1);
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert!(v >= 28, "expected schema version >= 28, got {v}");
    }
}
