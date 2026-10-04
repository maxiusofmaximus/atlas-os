// Atlas OS — Model reliability gate policy persistence (RFC 20 Fase 24 v24.2;
// plan research/53).
//
// `reliability_gate` is a single-row table holding the operator's
// `ReliabilityGate` for EVAL-informed routing. The Journal stores the raw
// fields — it does not depend on `orchestrator` types; the host converts.

use serde::{Deserialize, Serialize};

use super::Journal;

/// The persisted reliability-gate policy (raw fields; mirrors
/// `orchestrator::reliability_gate::ReliabilityGate`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReliabilityGateRow {
    pub min_samples: i64,
    pub min_pass_rate: f64,
    pub allow_unknown: bool,
    pub enabled: bool,
}

impl ReliabilityGateRow {
    /// Defaults matching `ReliabilityGate::default()` but **disabled** — the
    /// gate is opt-in and must not change routing until the operator enables it.
    pub fn defaults() -> Self {
        Self {
            min_samples: 20,
            min_pass_rate: 0.5,
            allow_unknown: true,
            enabled: false,
        }
    }
}

impl Default for ReliabilityGateRow {
    fn default() -> Self {
        Self::defaults()
    }
}

impl Journal {
    /// Load the persisted policy, or `None` when never set.
    pub fn load_reliability_gate(&self) -> anyhow::Result<Option<ReliabilityGateRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT min_samples, min_pass_rate, allow_unknown, enabled
             FROM reliability_gate WHERE id = 1",
        )?;
        let mut rows = stmt.query([])?;
        match rows.next()? {
            Some(r) => Ok(Some(ReliabilityGateRow {
                min_samples: r.get(0)?,
                min_pass_rate: r.get(1)?,
                allow_unknown: r.get::<_, i64>(2)? != 0,
                enabled: r.get::<_, i64>(3)? != 0,
            })),
            None => Ok(None),
        }
    }

    /// Persist the policy (UPSERT on the single row `id = 1`).
    pub fn save_reliability_gate(&self, policy: &ReliabilityGateRow) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO reliability_gate
                (id, min_samples, min_pass_rate, allow_unknown, enabled, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, unixepoch() * 1000)
             ON CONFLICT(id) DO UPDATE SET
                min_samples   = excluded.min_samples,
                min_pass_rate = excluded.min_pass_rate,
                allow_unknown = excluded.allow_unknown,
                enabled       = excluded.enabled,
                updated_at    = unixepoch() * 1000",
            rusqlite::params![
                policy.min_samples,
                policy.min_pass_rate,
                policy.allow_unknown as i64,
                policy.enabled as i64,
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn gate_policy_roundtrip_and_defaults_when_unset() {
        let dir = TempDir::new().unwrap();
        let j = Journal::open(dir.path()).unwrap();
        assert!(j.load_reliability_gate().unwrap().is_none());
        let p = ReliabilityGateRow {
            min_samples: 5,
            min_pass_rate: 0.8,
            allow_unknown: false,
            enabled: true,
        };
        j.save_reliability_gate(&p).unwrap();
        assert_eq!(j.load_reliability_gate().unwrap(), Some(p));
        let q = ReliabilityGateRow::defaults();
        j.save_reliability_gate(&q).unwrap();
        assert_eq!(j.load_reliability_gate().unwrap(), Some(q));
    }
}
