use crate::journal::*;

impl Journal {
    /// Thin wrapper over [`learning_graphs::persist_graph`]. Acquires
    /// the connection lock once and delegates.
    #[cfg(feature = "dag_mode")]
    pub fn persist_learning_graph(
        &self,
        req: &learning_graphs::PersistGraphRequest<'_>,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        learning_graphs::persist_graph(&conn, req)
    }

    /// Thin wrapper over [`learning_graphs::retrieve_similar_graphs`].
    #[cfg(feature = "dag_mode")]
    pub fn retrieve_similar_learning_graphs(
        &self,
        query_embedding: Option<&[f32]>,
        intent_signature: Option<&str>,
        top_k: usize,
    ) -> anyhow::Result<Vec<ScoredGraph>> {
        let conn = self.conn.lock();
        learning_graphs::retrieve_similar_graphs(&conn, query_embedding, intent_signature, top_k)
    }

    /// Thin wrapper over [`mission_graph::read_graph`]. Returns the
    /// full persisted graph (nodes+edges) for `mission_id`, or `None`
    /// when no M15 rows exist for that mission. The M15 tables are
    /// created unconditionally (schema.rs), so the read side is always
    /// available; only the write side is gated behind `dag_mode`.
    pub fn read_mission_graph(
        &self,
        mission_id: &str,
    ) -> anyhow::Result<Option<crate::graph::MissionGraph>> {
        let conn = self.conn.lock();
        mission_graph::read_graph(&conn, mission_id)
    }

    /// Test-only helper: bulk-insert raw graph rows for HUD/router
    /// integration tests. NOT for production use; production callers
    /// go through `skills::graph_loader::instantiate` or
    /// `planning::graph_emitter::plan_to_graph` (caller owns the
    /// write side).
    #[cfg(test)]
    pub fn seed_test_graph_node(
        &self,
        mission_id: &str,
        node_id: &str,
        kind: &str,
        label: &str,
        provenance: &str,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![node_id, mission_id, kind, label, provenance],
        )?;
        Ok(())
    }
}
