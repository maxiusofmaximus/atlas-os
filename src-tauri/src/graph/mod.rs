// OpenCode OS — mission graph (RFC 28 §C).
//
// Pattern derived from `safishamsi/graphify` (Apache-2.0, Copyright
// Graphify Labs): nodes + edges tagged with EXTRACTED | INFERRED |
// AMBIGUOUS provenance, persisted in SQLite, traversable via petgraph
// when the `dag_mode` feature is on. NO direct dependency on graphify
// (Python-only, violates RFC 25 §11); we port the *pattern* to Rust.
//
// This module declares the canonical types that everyone else reads
// from. It compiles unconditionally so callers (HUD tail, Journal
// rows, Planning types) can hold `MissionGraph` values without pulling
// petgraph. The traversal math lives in `traverse` (gated `dag_mode`)
// and the AST extractor lives in `ast` (gated `codebase-graph`).
//
// Schema lives in `journal::schema` M15 — see `mission_graph_nodes`
// and `mission_graph_edges`. The SQLite columns line up with the
// fields below by name; the Journal layer round-trips via
// `serde_json` so callers never construct raw SQL.

pub mod ast;
pub mod traverse;

use serde::{Deserialize, Serialize};

/// Where a graph node's existence came from. Mirrors the graphify
/// `provenance` tag, verbatim (study cited in RFC 28 apéndice
/// "Research sources"). The order also doubles as a rough confidence
/// ladder (EXTRACTED > INFERRED > AMBIGUOUS); the HUD renders
/// AMBIGUOUS with a contrasting badge so the operator can steer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Provenance {
    /// Found in source: parsed from tree-sitter AST, observed in the
    /// journal stream, written by the kernel. Cannot be false.
    Extracted,
    /// Deduced by the model / Planner / Learning Engine. May be wrong.
    /// The supervisor logs the deduction rationale in `attrs_json` so a
    /// `was_correct=false` later shifts the edge to AMBIGUOUS.
    Inferred,
    /// Low confidence. The Planner produced this node when its
    /// confidence was below the auto-lock threshold and the operator
    /// hasn't reviewed it yet. The HUD shows a "steer" affordance.
    Ambiguous,
}

impl Provenance {
    /// String used by M15 `mission_graph_nodes.provenance` CHECK
    /// constraint — must match the literals declared in schema.rs.
    pub fn tag(self) -> &'static str {
        match self {
            Provenance::Extracted => "EXTRACTED",
            Provenance::Inferred => "INFERRED",
            Provenance::Ambiguous => "AMBIGUOUS",
        }
    }

    /// Parse the literal emitted by [`Provenance::tag`]; returns
    /// `None` for any other string so callers can treat unknown
    /// provenance as data corruption rather than a panic.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "EXTRACTED" => Some(Provenance::Extracted),
            "INFERRED" => Some(Provenance::Inferred),
            "AMBIGUOUS" => Some(Provenance::Ambiguous),
            _ => None,
        }
    }
}

/// What is this node? Mirrors the M15
/// `mission_graph_nodes.kind` CHECK enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// RFC 19 supervisor state-machine vertex (`idle`, `planning`,
    /// `executing`, `verifying`, `recovering`, `halted`, `done`).
    EngineState,
    /// The mission root node — every graph has exactly one of these
    /// at id = `format!("mission:{mission_id}")`.
    Mission,
    /// RFC 06 skill node — references a `SkillManifest` and is the
    /// entry point a `SkillGraph` template contributes when loaded.
    Skill,
    /// External dependency: library, file, remote MCP — anything the
    /// kernel depends on but doesn't own. RFC 10 Research Engine
    /// populates these from `probe_feasibility` outputs.
    External,
}

impl NodeKind {
    pub fn tag(self) -> &'static str {
        match self {
            NodeKind::EngineState => "engine_state",
            NodeKind::Mission => "mission",
            NodeKind::Skill => "skill",
            NodeKind::External => "external",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "engine_state" => Some(NodeKind::EngineState),
            "mission" => Some(NodeKind::Mission),
            "skill" => Some(NodeKind::Skill),
            "external" => Some(NodeKind::External),
            _ => None,
        }
    }
}

/// Directed edge kind. Mirrors the M15
/// `mission_graph_edges.kind` CHECK enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// `f` calls `g` — direct function-call relationship, typically
    /// `EXTRACTED` from the AST.
    Calls,
    /// `f` imports `g` — module-level dependency.
    Imports,
    /// State machine transition. `precondition` carries the event
    /// that triggers the transition; `guard` carries the RFC 19
    /// guard expression that must evaluate true.
    TransitionsTo,
    /// `a` depends on `b` (template-level — not a direct call but
    /// the b-branch must complete first). Produced by the Planner
    /// when emitting a DAG.
    DependsOn,
    /// Soft reference — doc cross-link, comment mention. Usually
    /// `INFERRED`. The HUD draws these dashed.
    References,
}

impl EdgeKind {
    pub fn tag(self) -> &'static str {
        match self {
            EdgeKind::Calls => "calls",
            EdgeKind::Imports => "imports",
            EdgeKind::TransitionsTo => "transitions_to",
            EdgeKind::DependsOn => "depends_on",
            EdgeKind::References => "references",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "calls" => Some(EdgeKind::Calls),
            "imports" => Some(EdgeKind::Imports),
            "transitions_to" => Some(EdgeKind::TransitionsTo),
            "depends_on" => Some(EdgeKind::DependsOn),
            "references" => Some(EdgeKind::References),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn mission(mission_id: &str) -> Self {
        NodeId(format!("mission:{mission_id}"))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(pub String);

impl EdgeId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A single graph node. The on-disk row carries an `attrs_json`
/// column for ad-hoc decoration (RFC 19 phase, skill version, AST
/// location, etc.); here we expose it as a raw string so callers
/// don't pay for a serde roundtrip they may not need.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub mission_id: String,
    pub kind: NodeKind,
    pub label: String,
    pub provenance: Provenance,
    /// Raw JSON string (often `{}` for engine_state vertices). Use
    /// `Node::attrs` to parse on demand.
    pub attrs_json: String,
}

impl Node {
    pub fn attrs(&self) -> serde_json::Value {
        serde_json::from_str(&self.attrs_json).unwrap_or(serde_json::Value::Null)
    }
}

/// A single directed edge. `precondition`/`guard` are RFC 19 DFA
/// decorations — the supervisor evaluates them before traversing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub mission_id: String,
    pub src: NodeId,
    pub dst: NodeId,
    pub kind: EdgeKind,
    pub precondition: Option<String>,
    pub guard: Option<String>,
    pub visit_count: u32,
}

/// A whole mission graph. The HUD `GET /hud/graph/:mission_id`
/// endpoint serialises this directly. Callers in Rust should prefer
/// the traverse helpers over poking `nodes`/`edges` by hand when
/// the `dag_mode` feature is enabled.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MissionGraph {
    pub mission_id: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl MissionGraph {
    /// Empty graph containing only the mission root node.
    pub fn new_rooted(mission_id: &str) -> Self {
        let root = Node {
            id: NodeId::mission(mission_id),
            mission_id: mission_id.to_string(),
            kind: NodeKind::Mission,
            label: mission_id.to_string(),
            provenance: Provenance::Extracted,
            attrs_json: "{}".to_string(),
        };
        MissionGraph {
            mission_id: mission_id.to_string(),
            nodes: vec![root],
            edges: vec![],
        }
    }

    pub fn find_node(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| &n.id == id)
    }

    pub fn find_node_mut(&mut self, id: &NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| &n.id == id)
    }

    pub fn out_edges<'a>(&'a self, src: &'a NodeId) -> impl Iterator<Item = &'a Edge> + 'a {
        self.edges.iter().filter(move |e| &e.src == src)
    }

    pub fn in_edges<'a>(&'a self, dst: &'a NodeId) -> impl Iterator<Item = &'a Edge> + 'a {
        self.edges.iter().filter(move |e| &e.dst == dst)
    }

    pub fn out_degree(&self, src: &NodeId) -> usize {
        self.out_edges(src).count()
    }

    pub fn in_degree(&self, dst: &NodeId) -> usize {
        self.in_edges(dst).count()
    }

    /// Degree sum (`out + in`). A "god node" is one whose total
    /// degree exceeds a percentile threshold — see
    /// `traverse::god_nodes`.
    pub fn degree(&self, id: &NodeId) -> usize {
        self.out_degree(id) + self.in_degree(id)
    }

    pub fn push_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    pub fn push_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }

    /// Bump `visit_count` on the given edge. Used by the
    /// supervisor before transitioning across an edge so the
    /// doom-loop watcher can spot hot loops by degree over time.
    pub fn bump_edge(&mut self, edge_id: &EdgeId) -> Option<u32> {
        self.edges.iter_mut().find(|e| &e.id == edge_id).map(|e| {
            e.visit_count = e.visit_count.saturating_add(1);
            e.visit_count
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_node(id: &str, kind: NodeKind) -> Node {
        Node {
            id: NodeId(id.to_string()),
            mission_id: "m1".to_string(),
            kind,
            label: id.to_string(),
            provenance: Provenance::Extracted,
            attrs_json: "{}".to_string(),
        }
    }

    fn sample_edge(id: &str, src: &str, dst: &str, kind: EdgeKind) -> Edge {
        Edge {
            id: EdgeId(id.to_string()),
            mission_id: "m1".to_string(),
            src: NodeId(src.to_string()),
            dst: NodeId(dst.to_string()),
            kind,
            precondition: None,
            guard: None,
            visit_count: 0,
        }
    }

    #[test]
    fn provenance_tag_round_trip() {
        for v in [
            Provenance::Extracted,
            Provenance::Inferred,
            Provenance::Ambiguous,
        ] {
            assert_eq!(Provenance::parse(v.tag()), Some(v));
        }
        assert!(Provenance::parse("garbage").is_none());
    }

    #[test]
    fn node_kind_tag_round_trip() {
        for v in [
            NodeKind::EngineState,
            NodeKind::Mission,
            NodeKind::Skill,
            NodeKind::External,
        ] {
            assert_eq!(NodeKind::parse(v.tag()), Some(v));
        }
        assert!(NodeKind::parse("garbage").is_none());
    }

    #[test]
    fn edge_kind_tag_round_trip() {
        for v in [
            EdgeKind::Calls,
            EdgeKind::Imports,
            EdgeKind::TransitionsTo,
            EdgeKind::DependsOn,
            EdgeKind::References,
        ] {
            assert_eq!(EdgeKind::parse(v.tag()), Some(v));
        }
        assert!(EdgeKind::parse("garbage").is_none());
    }

    #[test]
    fn mission_id_rooted_has_single_mission_node() {
        let g = MissionGraph::new_rooted("M-1");
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.nodes[0].kind, NodeKind::Mission);
        assert_eq!(g.nodes[0].id.as_str(), "mission:M-1");
        assert!(g.edges.is_empty());
    }

    #[test]
    fn out_in_degree_and_iteration() {
        let mut g = MissionGraph::new_rooted("m1");
        g.push_node(sample_node("a", NodeKind::Skill));
        g.push_node(sample_node("b", NodeKind::Skill));
        g.push_node(sample_node("c", NodeKind::External));
        g.push_edge(sample_edge("e1", "mission:m1", "a", EdgeKind::DependsOn));
        g.push_edge(sample_edge("e2", "a", "b", EdgeKind::Calls));
        g.push_edge(sample_edge("e3", "a", "b", EdgeKind::Imports));
        g.push_edge(sample_edge("e4", "b", "c", EdgeKind::References));

        assert_eq!(g.out_degree(&NodeId("mission:m1".into())), 1);
        assert_eq!(g.out_degree(&NodeId("a".into())), 2);
        assert_eq!(g.in_degree(&NodeId("b".into())), 2);
        assert_eq!(g.degree(&NodeId("b".into())), 3);
    }

    #[test]
    fn bump_edge_increments_and_saturates() {
        let mut g = MissionGraph::new_rooted("m1");
        g.push_node(sample_node("a", NodeKind::Skill));
        g.push_node(sample_node("b", NodeKind::Skill));
        g.push_edge(sample_edge("e1", "a", "b", EdgeKind::Calls));
        assert_eq!(g.bump_edge(&EdgeId("e1".into())), Some(1));
        assert_eq!(g.bump_edge(&EdgeId("e1".into())), Some(2));
        assert_eq!(g.bump_edge(&EdgeId("ghost".into())), None);
    }

    #[test]
    fn attrs_returns_null_on_invalid_json() {
        let n = Node {
            id: NodeId("x".into()),
            mission_id: "m1".into(),
            kind: NodeKind::EngineState,
            label: "x".into(),
            provenance: Provenance::Inferred,
            attrs_json: "not json".to_string(),
        };
        assert!(n.attrs().is_null());
    }

    #[test]
    fn node_id_mission_canonical_format() {
        let id = NodeId::mission("00000000-0000-0000-0000-000000000001");
        assert_eq!(id.as_str(), "mission:00000000-0000-0000-0000-000000000001");
        assert_eq!(
            format!("{id}"),
            "mission:00000000-0000-0000-0000-000000000001"
        );
    }

    #[test]
    fn mission_graph_serialises_to_json() {
        let g = MissionGraph::new_rooted("m1");
        let s = serde_json::to_string(&g).expect("serialise");
        assert!(s.contains("\"mission_id\":\"m1\""));
        assert!(s.contains("\"kind\":\"mission\""));
        assert!(s.contains("\"provenance\":\"EXTRACTED\""));
        let back: MissionGraph = serde_json::from_str(&s).expect("deserialise");
        assert_eq!(g, back);
    }
}
