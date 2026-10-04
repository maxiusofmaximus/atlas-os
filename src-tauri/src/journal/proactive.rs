// Atlas OS — Proactive-turn policy + backlog persistence (RFC 20 Fase 23
// v3.1.2.2; plan research/52).
//
// `proactive_policy` is a single-row table holding the operator's `TurnPolicy`
// for the proactive turn engine (`planning::availability`). The Journal stores
// the raw fields — it does not depend on `planning` types; the host converts.
// `next_pending_mission` is the backlog probe: the oldest mission still in the
// `received` state, i.e. a mission awaiting its first turn.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Journal;

/// The persisted proactive-turn policy (raw fields; mirrors
/// `planning::availability::TurnPolicy`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProactivePolicyRow {
    pub eta_ms: i64,
    pub weight_threshold: f64,
    pub horizon_ms: i64,
    pub enabled: bool,
}

impl ProactivePolicyRow {
    /// Defaults matching `TurnPolicy::default()`: 15 min turn, only hard-busy
    /// (weight 1.0) blocks, 24 h horizon, enabled.
    pub fn defaults() -> Self {
        Self {
            eta_ms: 15 * 60 * 1000,
            weight_threshold: 1.0,
            horizon_ms: 24 * 60 * 60 * 1000,
            enabled: true,
        }
    }
}

impl Default for ProactivePolicyRow {
    fn default() -> Self {
        Self::defaults()
    }
}

impl Journal {
    /// Load the persisted policy, or `None` when never set (caller applies
    /// `ProactivePolicyRow::defaults()`).
    pub fn load_proactive_policy(&self) -> anyhow::Result<Option<ProactivePolicyRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT eta_ms, weight_threshold, horizon_ms, enabled
             FROM proactive_policy WHERE id = 1",
        )?;
        let mut rows = stmt.query([])?;
        match rows.next()? {
            Some(r) => Ok(Some(ProactivePolicyRow {
                eta_ms: r.get(0)?,
                weight_threshold: r.get(1)?,
                horizon_ms: r.get(2)?,
                enabled: r.get::<_, i64>(3)? != 0,
            })),
            None => Ok(None),
        }
    }

    /// Persist the policy (UPSERT on the single row `id = 1`).
    pub fn save_proactive_policy(&self, policy: &ProactivePolicyRow) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO proactive_policy
                (id, eta_ms, weight_threshold, horizon_ms, enabled, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, unixepoch() * 1000)
             ON CONFLICT(id) DO UPDATE SET
                eta_ms           = excluded.eta_ms,
                weight_threshold = excluded.weight_threshold,
                horizon_ms       = excluded.horizon_ms,
                enabled          = excluded.enabled,
                updated_at       = unixepoch() * 1000",
            rusqlite::params![
                policy.eta_ms,
                policy.weight_threshold,
                policy.horizon_ms,
                policy.enabled as i64,
            ],
        )?;
        Ok(())
    }

    /// The oldest mission still awaiting its first turn (`status = 'received'`),
    /// or `None` when the backlog is empty. This is the host's backlog probe.
    pub fn next_pending_mission(&self) -> anyhow::Result<Option<Uuid>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id FROM missions WHERE status = 'received' ORDER BY created_at ASC LIMIT 1",
        )?;
        let mut rows = stmt.query([])?;
        match rows.next()? {
            Some(r) => {
                let id: String = r.get(0)?;
                Ok(Uuid::parse_str(&id).ok())
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh() -> (TempDir, Journal) {
        let dir = TempDir::new().unwrap();
        let j = Journal::open(dir.path()).unwrap();
        (dir, j)
    }

    #[test]
    fn policy_roundtrip_and_defaults_when_unset() {
        let (_d, j) = fresh();
        assert!(j.load_proactive_policy().unwrap().is_none());
        let p = ProactivePolicyRow {
            eta_ms: 600_000,
            weight_threshold: 0.5,
            horizon_ms: 3_600_000,
            enabled: false,
        };
        j.save_proactive_policy(&p).unwrap();
        assert_eq!(j.load_proactive_policy().unwrap(), Some(p));
        // Upsert replaces the single row.
        let q = ProactivePolicyRow::defaults();
        j.save_proactive_policy(&q).unwrap();
        assert_eq!(j.load_proactive_policy().unwrap(), Some(q));
    }

    #[test]
    fn next_pending_mission_returns_oldest_received() {
        let (_d, j) = fresh();
        assert!(j.next_pending_mission().unwrap().is_none());
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        j.create_mission(first, "first").unwrap();
        // Ensure a distinct created_at ordering.
        std::thread::sleep(std::time::Duration::from_millis(5));
        j.create_mission(second, "second").unwrap();
        assert_eq!(j.next_pending_mission().unwrap(), Some(first));
    }
}
