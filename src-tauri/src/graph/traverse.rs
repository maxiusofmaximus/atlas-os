// Atlas OS — graph traversal helpers (RFC 28 §C).
//
// Gated behind `dag_mode` because petgraph is the only cargo dep it
// pulls. The HUD endpoint `GET /hud/graph/:id` does NOT need this
// module (it serves the raw node/edge lists); the helpers below are
// for the supervisor's path queries (`shortest_path` to a healthy
// state when the doom-loop fires), the Planner's "god node" warning,
// and the Learning Engine's community partition for skill affinity.
//
// The Leiden community-detection algorithm is intentionally NOT
// ported. RFC 28 §C Riesgos says "communities via union-find simple,
// NO Leiden — no mature Rust Leiden crate (would require FFI to
// leidenalg C); defer Leiden a v2." `community_partition` here uses
// connected components via union-find, which is fully covered by
// petgraph's `UnionFind` and is good enough to bucket subgraphs by
// connectivity for the Learning Engine's top-k retrieval.

#![cfg_attr(not(feature = "dag_mode"), allow(unused))]

use super::{Edge, EdgeKind, MissionGraph, NodeId};

/// Path returned by `shortest_path`. Empty when no route exists.
/// Always starts with `start` and ends with `goal` on a successful
/// route. Duplicates a target when the `goal` is the `start`.
pub type Path = Vec<NodeId>;

/// Breadth-first shortest path following directed edges. Edge kind
/// filter accepts everything when `filter` is `None`; otherwise the
/// BFS only crosses an edge whose `kind` is in the slice. This
/// matches the supervisor's "transitions only" rule when computing
/// paths to a healthy state.
///
/// Cost model: unit weight per edge, ignores `visit_count` and
/// `attrs_json` — the supervisor uses this for topology, not for
/// biasing toward recently-warm edges.
pub fn shortest_path(
    g: &MissionGraph,
    start: &NodeId,
    goal: &NodeId,
    filter: Option<&[EdgeKind]>,
) -> Path {
    use std::collections::{HashMap, HashSet, VecDeque};
    if start == goal {
        return vec![start.clone()];
    }
    let edge_ok = |e: &Edge| match filter {
        Some(kinds) => kinds.contains(&e.kind),
        None => true,
    };

    let mut prev: HashMap<NodeId, NodeId> = HashMap::new();
    let mut queue: VecDeque<NodeId> = VecDeque::new();
    let mut visited: HashSet<NodeId> = HashSet::new();
    queue.push_back(start.clone());
    visited.insert(start.clone());

    while let Some(node) = queue.pop_front() {
        for e in g.out_edges(&node) {
            if !edge_ok(e) {
                continue;
            }
            if visited.contains(&e.dst) {
                continue;
            }
            visited.insert(e.dst.clone());
            prev.insert(e.dst.clone(), node.clone());
            if &e.dst == goal {
                let mut path = vec![goal.clone()];
                let mut cur = goal.clone();
                while let Some(p) = prev.get(&cur) {
                    path.push(p.clone());
                    cur = p.clone();
                }
                path.reverse();
                return path;
            }
            queue.push_back(e.dst.clone());
        }
    }
    Path::new()
}

/// Direct neighbours reachable in one hop from `id`, filtered by
/// the same edge-kind predicate as `shortest_path`. The HUD uses
/// this to highlight the cascade when the operator hovers a card.
pub fn get_neighbors(g: &MissionGraph, id: &NodeId, filter: Option<&[EdgeKind]>) -> Vec<NodeId> {
    let edge_ok = |e: &Edge| match filter {
        Some(kinds) => kinds.contains(&e.kind),
        None => true,
    };
    g.out_edges(id)
        .filter(|e| edge_ok(e))
        .map(|e| e.dst.clone())
        .collect()
}

/// Nodes whose total degree (`out + in`) exceeds the given
/// `percentile` cutoff within the graph. Returns a vec of
/// `(node_id, degree)` tuples sorted by descending degree.
///
/// RFC 28 §C Riesgos borrows the graphify "hub exclusion percentile"
/// idea: god nodes get treated as bridge edges for the Learning
/// Engine's similarity search to avoid flooding matches with the
/// always-referenced mission root.
pub fn god_nodes(g: &MissionGraph, percentile: f64) -> Vec<(NodeId, usize)> {
    debug_assert!((0.0..=100.0).contains(&percentile));
    let mut degrees: Vec<(NodeId, usize)> = g
        .nodes
        .iter()
        .map(|n| (n.id.clone(), g.degree(&n.id)))
        .collect();
    degrees.sort_by_key(|&(_, d)| std::cmp::Reverse(d));

    if degrees.is_empty() {
        return degrees;
    }
    let threshold_idx = ((percentile / 100.0) * (degrees.len() as f64 - 1.0)).round() as usize;
    let threshold = degrees[threshold_idx].1;
    degrees
        .into_iter()
        .filter(|(_, d)| *d >= threshold && *d > 0)
        .collect()
}

/// Connected components via union-find. Returns groups of node ids
/// that share an undirected path through any edge. RFC 28 §C Riesgos
/// explicitly says Leiden is deferred to v2; this is the v1
/// approximation — it splits artifacts across skill/module boundaries
/// only when there's no path at all between them.
#[cfg(feature = "dag_mode")]
pub fn community_partition(g: &MissionGraph) -> Vec<Vec<NodeId>> {
    use petgraph::unionfind::UnionFind;
    use std::collections::HashMap;

    let index: HashMap<NodeId, usize> = g
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.clone(), i))
        .collect();
    if index.is_empty() {
        return Vec::new();
    }
    let n = g.nodes.len();
    let mut uf = UnionFind::new(n);
    for e in &g.edges {
        debug_assert!(!e.src.0.is_empty(), "edge src must be set");
        debug_assert!(!e.dst.0.is_empty(), "edge dst must be set");
        if let (Some(&a), Some(&b)) = (index.get(&e.src), index.get(&e.dst)) {
            uf.union(a, b);
        }
    }
    let mut groups: HashMap<usize, Vec<NodeId>> = HashMap::new();
    for (id, i) in &index {
        let label = uf.find(*i);
        groups.entry(label).or_default().push(id.clone());
    }
    let mut out: Vec<Vec<NodeId>> = groups.into_values().collect();
    out.sort_by_key(|v| std::cmp::Reverse(v.len()));
    out
}

#[cfg(all(test, feature = "dag_mode"))]
mod tests {
    use super::*;
    use crate::graph::{EdgeId, EdgeKind, MissionGraph, Node, NodeId, NodeKind, Provenance};

    fn n(id: &str, kind: NodeKind) -> Node {
        Node {
            id: NodeId(id.to_string()),
            mission_id: "m1".to_string(),
            kind,
            label: id.to_string(),
            provenance: Provenance::Extracted,
            attrs_json: "{}".to_string(),
        }
    }

    fn e(id: &str, src: &str, dst: &str, kind: EdgeKind) -> Edge {
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

    fn small_graph() -> MissionGraph {
        let mut g = MissionGraph::new_rooted("m1");
        g.push_node(n("a", NodeKind::Skill));
        g.push_node(n("b", NodeKind::Skill));
        g.push_node(n("c", NodeKind::EngineState));
        g.push_node(n("d", NodeKind::External));
        g.push_edge(e("e1", "mission:m1", "a", EdgeKind::DependsOn));
        g.push_edge(e("e2", "a", "b", EdgeKind::Calls));
        g.push_edge(e("e3", "b", "c", EdgeKind::TransitionsTo));
        g.push_edge(e("e4", "mission:m1", "d", EdgeKind::References));
        g
    }

    #[test]
    fn shortest_path_follows_directed_edges() {
        let g = small_graph();
        let path = shortest_path(&g, &NodeId("mission:m1".into()), &NodeId("c".into()), None);
        assert_eq!(
            path,
            vec![
                NodeId("mission:m1".into()),
                NodeId("a".into()),
                NodeId("b".into()),
                NodeId("c".into()),
            ]
        );
    }

    #[test]
    fn shortest_path_returns_single_node_for_start_eq_goal() {
        let g = small_graph();
        let path = shortest_path(&g, &NodeId("c".into()), &NodeId("c".into()), None);
        assert_eq!(path, vec![NodeId("c".into())]);
    }

    #[test]
    fn shortest_path_returns_empty_when_unreachable() {
        let g = small_graph();
        let path = shortest_path(&g, &NodeId("d".into()), &NodeId("a".into()), None);
        assert!(path.is_empty());
    }

    #[test]
    fn shortest_path_respects_edge_kind_filter() {
        let g = small_graph();
        let only_trans = shortest_path(
            &g,
            &NodeId("mission:m1".into()),
            &NodeId("c".into()),
            Some(&[EdgeKind::TransitionsTo]),
        );
        assert!(
            only_trans.is_empty(),
            "no transition edge leaves the mission root"
        );
    }

    #[test]
    fn get_neighbors_filters_by_kind() {
        let g = small_graph();
        let neighbors = get_neighbors(
            &g,
            &NodeId("mission:m1".into()),
            Some(&[EdgeKind::DependsOn]),
        );
        assert_eq!(neighbors, vec![NodeId("a".into())]);
    }

    #[test]
    fn god_nodes_finds_hub_by_degree() {
        let g = small_graph();
        let gods = god_nodes(&g, 50.0);
        assert!(
            gods.iter().any(|(id, _)| id.as_str() == "mission:m1"),
            "mission root should be god, got: {gods:?}"
        );
    }

    #[test]
    fn god_nodes_empty_graph_returns_empty() {
        let g = MissionGraph::new_rooted("m");
        let gods = god_nodes(&g, 90.0);
        assert!(gods.is_empty(), "isolated root has degree 0");
    }

    #[test]
    fn community_partition_groups_connected_nodes() {
        let mut g = MissionGraph::new_rooted("m1");
        g.push_node(n("a", NodeKind::Skill));
        g.push_node(n("b", NodeKind::Skill));
        g.push_node(n("iso", NodeKind::External));
        g.push_edge(e("e1", "mission:m1", "a", EdgeKind::DependsOn));
        g.push_edge(e("e2", "a", "b", EdgeKind::Calls));

        let mut groups = community_partition(&g);
        groups.sort_by_key(|v| std::cmp::Reverse(v.len()));
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].len(), 3);
        assert_eq!(groups[1].len(), 1);
        assert_eq!(groups[1][0].as_str(), "iso");
    }

    #[test]
    fn community_partition_handles_empty_graph() {
        let g = MissionGraph::new_rooted("m");
        let groups = community_partition(&g);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 1);
    }
}
