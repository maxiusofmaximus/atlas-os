// Atlas OS — cumulative model cost helper (RFC 20 Phase 8 sub-fase 8.2).
//
// `total_model_cost_usd` sums `model_invocations.cost_usd` so the
// VRAM/RAM/cost monitor (`crate::monitor::snapshot_now` + `atlas
// monitor`) can join hardware pressure with spend on one HUD line.
// Empty journal → 0.0 (fail-safe, never NULL).

impl crate::journal::Journal {
    pub fn total_model_cost_usd(&self) -> anyhow::Result<f64> {
        let conn = self.conn.lock();
        let total: Option<f64> = conn
            .query_row(
                "SELECT COALESCE(SUM(cost_usd), 0) FROM model_invocations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(Some(0.0));
        Ok(total.unwrap_or(0.0).max(0.0))
    }
}

#[cfg(test)]
mod tests {
    use crate::journal::Journal;
    use tempfile::TempDir;

    #[test]
    fn total_cost_is_zero_on_empty_journal() {
        let dir = TempDir::new().unwrap();
        let j = Journal::open(dir.path()).unwrap();
        assert_eq!(j.total_model_cost_usd().unwrap(), 0.0);
    }
}
