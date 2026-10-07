// Atlas OS — Mission graph store (RFC 28 §C item 7, RFC 24 §3.3
// graph cards). Read side of the M15 `mission_graph_nodes` /
// `mission_graph_edges` tables. The write side (Planner DAG emitter,
// Skills `graph.toml` loader, AST extractor) lives in the
// `planning::graph_emitter`, `skills::graph_loader`, and `graph::ast`
// modules; this module only projects rows back into the canonical
// `MissionGraph` shape for the HUD `GET /graph/:mission_id` route.
//
// The M15 `mission_graph_nodes` / `mission_graph_edges` tables are
// created unconditionally (schema.rs), so this read side is always
// compiled; only the write side (Planner DAG emitter, Skills loader,
// AST extractor) is gated behind `dag_mode`.

use rusqlite::Connection;

use crate::graph::{Edge, EdgeId, EdgeKind, MissionGraph, Node, NodeId, NodeKind, Provenance};

/// A single row emitted from `mission_graph_nodes` joined with its
/// edge count (per node, for the HUD badge). The HUD only needs the
/// canonical `MissionGraph` payload, so projection helpers below fold
/// the rows back into the public `Node`/`Edge` types.
#[derive(Clone, Debug, PartialEq)]
pub struct MissionGraphRow {
    pub mission_id: String,
    pub graph: MissionGraph,
}

/// Read the full graph (nodes + edges) attached to `mission_id`.
/// Returns `Ok(None)` when the mission exists but no graph has been
/// persisted yet, and `Err` only for SQL failures (i.e. an unparseable
/// row, never for a missing mission — that's a 404 concern for the
/// HUD router, not a store concern).
pub fn read_graph(conn: &Connection, mission_id: &str) -> anyhow::Result<Option<MissionGraph>> {
    let node_count: i64 = conn.query_row(
        "SELECT count(*) FROM mission_graph_nodes WHERE mission_id = ?1",
        rusqlite::params![mission_id],
        |r| r.get(0),
    )?;
    if node_count == 0 {
        return Ok(None);
    }

    let mut graph = MissionGraph::new_rooted(mission_id);
    // `new_rooted` inserts the root mission node already; the read side
    // below re-seeds from the persisted rows so the canonical id label
    // matches exactly what was stored. Start from an empty graph so we
    // don't double-count.
    graph.nodes.clear();

    let mut node_stmt = conn.prepare(
        "SELECT id, kind, label, provenance, attrs_json FROM mission_graph_nodes
         WHERE mission_id = ?1",
    )?;
    let node_rows = node_stmt.query_map(rusqlite::params![mission_id], |row| {
        let id: String = row.get(0)?;
        let kind: String = row.get(1)?;
        let label: String = row.get(2)?;
        let provenance: String = row.get(3)?;
        let attrs: Option<String> = row.get(4)?;
        Ok((id, kind, label, provenance, attrs))
    })?;
    for row in node_rows {
        let (id, kind, label, provenance, attrs) = row?;
        let node = Node {
            id: NodeId(id),
            mission_id: mission_id.to_string(),
            kind: parse_node_kind(&kind),
            label,
            provenance: parse_provenance(&provenance),
            attrs_json: attrs.unwrap_or_else(|| serde_json::Value::Null.to_string()),
        };
        graph.push_node(node);
    }

    let mut edge_stmt = conn.prepare(
        "SELECT id, src, dst, kind, precondition, guard, visit_count
         FROM mission_graph_edges WHERE mission_id = ?1",
    )?;
    let edge_rows = edge_stmt.query_map(rusqlite::params![mission_id], |row| {
        let id: String = row.get(0)?;
        let src: String = row.get(1)?;
        let dst: String = row.get(2)?;
        let kind: String = row.get(3)?;
        let precondition: Option<String> = row.get(4)?;
        let guard: Option<String> = row.get(5)?;
        let visit_count: i64 = row.get(6)?;
        Ok((id, src, dst, kind, precondition, guard, visit_count))
    })?;
    for row in edge_rows {
        let (id, src, dst, kind, precondition, guard, visit_count) = row?;
        let edge = Edge {
            id: EdgeId(id),
            mission_id: mission_id.to_string(),
            src: NodeId(src),
            dst: NodeId(dst),
            kind: parse_edge_kind(&kind),
            precondition,
            guard,
            visit_count: visit_count as u32,
        };
        graph.push_edge(edge);
    }

    Ok(Some(graph))
}

fn parse_node_kind(s: &str) -> NodeKind {
    match s {
        "engine_state" => NodeKind::EngineState,
        "mission" => NodeKind::Mission,
        "skill" => NodeKind::Skill,
        "external" => NodeKind::External,
        _ => NodeKind::EngineState,
    }
}

fn parse_edge_kind(s: &str) -> EdgeKind {
    match s {
        "calls" => EdgeKind::Calls,
        "imports" => EdgeKind::Imports,
        "transitions_to" => EdgeKind::TransitionsTo,
        "depends_on" => EdgeKind::DependsOn,
        "references" => EdgeKind::References,
        _ => EdgeKind::DependsOn,
    }
}

fn parse_provenance(s: &str) -> Provenance {
    match s {
        "EXTRACTED" => Provenance::Extracted,
        "AMBIGUOUS" => Provenance::Ambiguous,
        _ => Provenance::Inferred,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::schema::migrate;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn seed_mission(conn: &Connection) {
        conn.execute(
            "INSERT INTO missions (id, label, status, created_at, updated_at)
             VALUES ('00000000-0000-0000-0000-000000000001', 'm', 'received', '2026-07-28T00:00:00Z', '2026-07-28T00:00:00Z')",
            [],
        )
        .unwrap();
    }

    fn seed_node(conn: &Connection, id: &str, kind: &str, label: &str, proof: &str) {
        conn.execute(
            "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance)
             VALUES (?1, '00000000-0000-0000-0000-000000000001', ?2, ?3, ?4)",
            rusqlite::params![id, kind, label, proof],
        )
        .unwrap();
    }

    fn seed_edge(conn: &Connection, id: &str, src: &str, dst: &str, kind: &str, visit: i64) {
        conn.execute(
            "INSERT INTO mission_graph_edges (id, mission_id, src, dst, kind, visit_count)
             VALUES (?1, '00000000-0000-0000-0000-000000000001', ?2, ?3, ?4, ?5)",
            rusqlite::params![id, src, dst, kind, visit],
        )
        .unwrap();
    }

    #[test]
    fn read_graph_returns_none_when_no_rows() {
        let conn = fresh_conn();
        seed_mission(&conn);
        let g = read_graph(&conn, "00000000-0000-0000-0000-000000000001").unwrap();
        assert!(g.is_none());
    }

    #[test]
    fn read_graph_returns_some_when_nodes_exist() {
        let conn = fresh_conn();
        seed_mission(&conn);
        seed_node(&conn, "n1", "mission", "root", "EXTRACTED");
        let g = read_graph(&conn, "00000000-0000-0000-0000-000000000001").unwrap();
        assert!(g.is_some());
        assert_eq!(g.unwrap().nodes.len(), 1);
    }

    #[test]
    fn read_graph_roundtrip_single_node() {
        let conn = fresh_conn();
        seed_mission(&conn);
        seed_node(&conn, "n1", "skill", "Skill", "INFERRED");
        let g = read_graph(&conn, "00000000-0000-0000-0000-000000000001")
            .unwrap()
            .unwrap();
        let n = &g.nodes[0];
        assert_eq!(n.kind, NodeKind::Skill);
        assert_eq!(n.label, "Skill");
        assert_eq!(n.provenance, Provenance::Inferred);
    }

    #[test]
    fn read_graph_roundtrip_edge_with_visit_count() {
        let conn = fresh_conn();
        seed_mission(&conn);
        seed_node(&conn, "n1", "engine_state", "a", "EXTRACTED");
        seed_node(&conn, "n2", "engine_state", "b", "EXTRACTED");
        seed_edge(&conn, "e1", "n1", "n2", "transitions_to", 7);
        let g = read_graph(&conn, "00000000-0000-0000-0000-000000000001")
            .unwrap()
            .unwrap();
        assert_eq!(g.edges.len(), 1);
        let e = &g.edges[0];
        assert_eq!(e.kind, EdgeKind::TransitionsTo);
        assert_eq!(e.visit_count, 7);
    }

    #[test]
    fn read_graph_parser_routes_unknown_kinds_to_fallbacks() {
        // The DB CHECK constraints reject unknown `kind` strings, so
        // the parser functions must be exercised directly: any value
        // that slips past the CHECK (future schema widening, raw
        // fixture import) must map to a safe fallback rather than
        // panic.
        assert_eq!(parse_node_kind("bogus_kind"), NodeKind::EngineState);
        assert_eq!(parse_edge_kind("bogus_kind"), EdgeKind::DependsOn);
        assert_eq!(parse_provenance("WAT"), Provenance::Inferred);
        // Sanity-check the happy paths so we don't silently break
        // the table mapping later.
        assert_eq!(parse_node_kind("skill"), NodeKind::Skill);
        assert_eq!(parse_edge_kind("calls"), EdgeKind::Calls);
        assert_eq!(parse_provenance("EXTRACTED"), Provenance::Extracted);
    }

    #[test]
    fn read_graph_attrs_null_when_column_absent_or_unparseable() {
        let conn = fresh_conn();
        seed_mission(&conn);
        conn.execute(
            "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance, attrs_json)
             VALUES ('n1', '00000000-0000-0000-0000-000000000001', 'mission', 'x', 'EXTRACTED', 'not-json')",
            [],
        )
        .unwrap();
        let g = read_graph(&conn, "00000000-0000-0000-0000-000000000001")
            .unwrap()
            .unwrap();
        let n = &g.nodes[0];
        assert!(n.attrs().is_null());
    }

    #[test]
    fn read_graph_attrs_roundtrips_valid_json() {
        let conn = fresh_conn();
        seed_mission(&conn);
        conn.execute(
            "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance, attrs_json)
             VALUES ('n1', '00000000-0000-0000-0000-000000000001', 'mission', 'x', 'EXTRACTED', ?1)",
            rusqlite::params![serde_json::json!({"action":"fork"}).to_string()],
        )
        .unwrap();
        let g = read_graph(&conn, "00000000-0000-0000-0000-000000000001")
            .unwrap()
            .unwrap();
        let n = &g.nodes[0];
        assert_eq!(n.attrs()["action"], "fork");
    }

    #[test]
    fn read_graph_for_nonexistent_mission_returns_none() {
        let conn = fresh_conn();
        let g = read_graph(&conn, "never-existed").unwrap();
        assert!(g.is_none());
    }

    #[test]
    fn read_graph_emits_precondition_and_guard_strings() {
        let conn = fresh_conn();
        seed_mission(&conn);
        seed_node(&conn, "n1", "engine_state", "a", "EXTRACTED");
        seed_node(&conn, "n2", "engine_state", "b", "EXTRACTED");
        conn.execute(
            "INSERT INTO mission_graph_edges (id, mission_id, src, dst, kind, precondition, guard)
             VALUES ('e1', '00000000-0000-0000-0000-000000000001', 'n1', 'n2', 'transitions_to', 'P1', 'G1')",
            [],
        )
        .unwrap();
        let g = read_graph(&conn, "00000000-0000-0000-0000-000000000001")
            .unwrap()
            .unwrap();
        let e = &g.edges[0];
        assert_eq!(e.precondition.as_deref(), Some("P1"));
        assert_eq!(e.guard.as_deref(), Some("G1"));
    }
}
