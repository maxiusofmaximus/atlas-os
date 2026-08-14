// Atlas OS — Skill graph template loader (RFC 23 §7.3 / RFC 28 §C item 5).
//
// Gated behind `dag_mode`. A skill optionally ships a 4th file,
// `graph.toml`, declaring a reusable sub-graph template (nodes + edges
// + preconditions). Loading a skill with a template inserts its
// node/edge set into the mission `MissionGraph` (M15) with fresh IDs
// prefixed by `{skill_id}:{mission_id}:` so two instances of the same
// skill on different missions never collide. Tags provenance =
// `Inferred` (the template is a skill spec, not raw source ingest —
// that's `EXTRACTED` reserved for `graph::ast`).
//
// Scope of v1:
//   * Parse the TOML via the existing `toml` crate (already in
//     `Cargo.toml` — no new dep).
//   * Return an in-memory `SkillGraphTemplate`; the supervisor /
//     Skills loader (`manifest::load_skill`) is responsible for calling
//     `instantiate` and persisting via the Journal layer (Phase 2).
//   * No precondition/guard evaluation engine here — Phase 2 wires
//     those into the Execution Supervisor hooks (RFC 19 §6.1.1).
//
// The TOML schema mirrors RFC 23 §7.3 verbatim:
//
//   nodes = [{ id = "probe", kind = "engine_state", label = "Probe" }, ...]
//   edges = [{ from = "probe", to = "gap", kind = "transitions_to",
//              precondition = "verdict.confidence < HIGH" }, ...]

#![cfg_attr(not(feature = "dag_mode"), allow(unused, dead_code, unused_imports))]

use crate::graph::{Edge, EdgeId, EdgeKind, MissionGraph, Node, NodeId, NodeKind, Provenance};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// RFC 23 §7.3 — a node in the skill's reusable sub-graph template.
/// `id` is local to the template (e.g. `"probe"`); `instantiate`
/// rewrites it to `{skill_id}:{mission_id}:{id}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateNode {
    pub id: String,
    pub kind: String,
    pub label: String,
}

/// RFC 23 §7.3 — an edge between two template nodes. `from` / `to`
/// reference local `TemplateNode::id` values; the rewriter resolves
/// them to the instantiated ids.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    #[serde(default)]
    pub precondition: Option<String>,
    #[serde(default)]
    pub guard: Option<String>,
}

/// The full skill graph template parsed from `graph.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGraphTemplate {
    pub nodes: Vec<TemplateNode>,
    #[serde(default)]
    pub edges: Vec<TemplateEdge>,
}

/// Parse a skill's `graph.toml`. Returns `Ok(None)` if the file is
/// missing — most skills don't carry a template. Returns an error if
/// the file exists but doesn't parse, so a typo in the spec is loud.
pub fn load_graph_template(dir: &Path) -> anyhow::Result<Option<SkillGraphTemplate>> {
    let path = dir.join("graph.toml");
    if !path.exists() {
        return Ok(None);
    }
    let contents =
        std::fs::read_to_string(&path).map_err(|e| anyhow::anyhow!("reading graph.toml: {e}"))?;
    let template: SkillGraphTemplate =
        toml::from_str(&contents).map_err(|e| anyhow::anyhow!("parsing graph.toml: {e}"))?;
    validate(&template)?;
    Ok(Some(template))
}

/// Reject templates that violate the contract — duplicate node ids,
/// edges referencing unknown nodes, unsupported `kind` strings. We
/// catch these at load-time so a malformed skill fails loudly before
/// the supervisor tries to instantiate it mid-mission.
fn validate(template: &SkillGraphTemplate) -> anyhow::Result<()> {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for n in &template.nodes {
        if !seen.insert(n.id.as_str()) {
            anyhow::bail!("duplicate node id in graph.toml: {}", n.id);
        }
    }
    for e in &template.edges {
        if !seen.contains(e.from.as_str()) {
            anyhow::bail!("edge references unknown `from` node: {}", e.from);
        }
        if !seen.contains(e.to.as_str()) {
            anyhow::bail!("edge references unknown `to` node: {}", e.to);
        }
        parse_edge_kind(&e.kind)
            .map_err(|err| anyhow::anyhow!("bad edge kind {:?}: {err}", e.kind))?;
    }
    Ok(())
}

/// Resolve a template into a `MissionGraph` rooted at
/// `{skill_id}:{mission_id}:root`. Each template node + edge is
/// re-homed under the prefix `{skill_id}:{mission_id}:` so the
/// resulting graph can be merged into a running mission's graph
/// without id collisions.
pub fn instantiate(
    template: &SkillGraphTemplate,
    skill_id: &str,
    mission_id: &str,
) -> MissionGraph {
    let prefix = format!("{skill_id}:{mission_id}:");
    let root_id = format!("{prefix}root");
    let mut g = MissionGraph::new_rooted(&root_id);
    g.nodes[0].label = format!("skill {skill_id} entry for {mission_id}");

    for tn in &template.nodes {
        let node_kind = parse_node_kind(&tn.kind).unwrap_or(NodeKind::EngineState);
        let fresh_id = format!("{prefix}{}", tn.id);
        g.push_node(Node {
            id: NodeId(fresh_id.clone()),
            mission_id: mission_id.to_string(),
            kind: node_kind,
            label: tn.label.clone(),
            provenance: Provenance::Inferred,
            attrs_json: serde_json::json!({
                "skill_id": skill_id,
                "template_node_id": tn.id,
                "template_kind": tn.kind,
            })
            .to_string(),
        });
        g.push_edge(Edge {
            id: EdgeId(format!("{prefix}e-root-{}", tn.id)),
            mission_id: mission_id.to_string(),
            src: NodeId(root_id.clone()),
            dst: NodeId(fresh_id),
            kind: EdgeKind::DependsOn,
            precondition: None,
            guard: None,
            visit_count: 0,
        });
    }

    for te in &template.edges {
        let kind = parse_edge_kind(&te.kind).unwrap_or(EdgeKind::TransitionsTo);
        g.push_edge(Edge {
            id: EdgeId(format!("{prefix}e-{}-{}", te.from, te.to)),
            mission_id: mission_id.to_string(),
            src: NodeId(format!("{prefix}{}", te.from)),
            dst: NodeId(format!("{prefix}{}", te.to)),
            kind,
            precondition: te.precondition.clone(),
            guard: te.guard.clone(),
            visit_count: 0,
        });
    }

    g
}

/// Map the string `kind` in a template to our canonical `NodeKind`.
/// Unknown strings fall back to `EngineState` (templates often declare
/// prose kinds that we normalise to the engine-state superset).
fn parse_node_kind(s: &str) -> Option<NodeKind> {
    match s {
        "engine_state" => Some(NodeKind::EngineState),
        "mission" => Some(NodeKind::Mission),
        "skill" => Some(NodeKind::Skill),
        "external" => Some(NodeKind::External),
        _ => Some(NodeKind::EngineState),
    }
}

/// Map the string edge `kind` in a template to the canonical
/// `EdgeKind`. Unknown strings return an error.
fn parse_edge_kind(s: &str) -> anyhow::Result<EdgeKind> {
    match s {
        "calls" => Ok(EdgeKind::Calls),
        "imports" => Ok(EdgeKind::Imports),
        "transitions_to" => Ok(EdgeKind::TransitionsTo),
        "depends_on" => Ok(EdgeKind::DependsOn),
        "references" => Ok(EdgeKind::References),
        other => anyhow::bail!("unsupported edge kind: {other}"),
    }
}

#[cfg(all(test, feature = "dag_mode"))]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_template(content: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tmp dir");
        let path = dir.path().join("graph.toml");
        let mut f = std::fs::File::create(&path).expect("create file");
        f.write_all(content.as_bytes()).expect("write");
        dir
    }

    #[test]
    fn load_graph_template_returns_none_when_missing() {
        let dir = tempfile::tempdir().expect("tmp");
        let loaded = load_graph_template(dir.path()).expect("ok");
        assert!(loaded.is_none());
    }

    #[test]
    fn load_graph_template_parses_well_formed_toml() {
        let dir = write_template(
            r#"
nodes = [
  { id = "probe",  kind = "engine_state", label = "Probe gaps" },
  { id = "refine", kind = "engine_state", label = "Refine prompt" },
]
edges = [
  { from = "probe", to = "refine", kind = "transitions_to", precondition = "verdict.confidence < HIGH" },
]
"#,
        );
        let template = load_graph_template(dir.path())
            .expect("ok")
            .expect("present");
        assert_eq!(template.nodes.len(), 2);
        assert_eq!(template.edges.len(), 1);
        assert_eq!(
            template.edges[0].precondition.as_deref(),
            Some("verdict.confidence < HIGH")
        );
    }

    #[test]
    fn load_graph_template_rejects_duplicate_node_ids() {
        let dir = write_template(
            r#"
nodes = [
  { id = "x", kind = "engine_state", label = "first" },
  { id = "x", kind = "engine_state", label = "second" },
]
edges = []
"#,
        );
        let err = load_graph_template(dir.path()).expect_err("rejected");
        let msg = format!("{err}");
        assert!(msg.contains("duplicate node id"));
    }

    #[test]
    fn load_graph_template_rejects_edge_to_unknown_node() {
        let dir = write_template(
            r#"
nodes = [{ id = "a", kind = "engine_state", label = "A" }]
edges = [{ from = "a", to = "missing", kind = "transitions_to" }]
"#,
        );
        let err = load_graph_template(dir.path()).expect_err("rejected");
        assert!(format!("{err}").contains("unknown `to` node"));
    }

    #[test]
    fn load_graph_template_rejects_bad_edge_kind() {
        let dir = write_template(
            r#"
nodes = [
  { id = "a", kind = "engine_state", label = "A" },
  { id = "b", kind = "engine_state", label = "B" },
]
edges = [{ from = "a", to = "b", kind = "frobnicate" }]
"#,
        );
        let err = load_graph_template(dir.path()).expect_err("rejected");
        assert!(format!("{err}").contains("unsupported edge kind"));
    }

    #[test]
    fn load_graph_template_rejects_bad_edge_kind_referencing_unknown_from() {
        let dir = write_template(
            r#"
nodes = [{ id = "a", kind = "engine_state", label = "A" }]
edges = [{ from = "ghost", to = "a", kind = "transitions_to" }]
"#,
        );
        let err = load_graph_template(dir.path()).expect_err("rejected");
        assert!(format!("{err}").contains("unknown `from` node"));
    }

    #[test]
    fn instantiate_produces_root_plus_template_nodes() {
        let template = SkillGraphTemplate {
            nodes: vec![
                TemplateNode {
                    id: "probe".into(),
                    kind: "engine_state".into(),
                    label: "Probe".into(),
                },
                TemplateNode {
                    id: "refine".into(),
                    kind: "engine_state".into(),
                    label: "Refine".into(),
                },
            ],
            edges: vec![TemplateEdge {
                from: "probe".into(),
                to: "refine".into(),
                kind: "transitions_to".into(),
                precondition: Some("conf < HIGH".into()),
                guard: None,
            }],
        };
        let g = instantiate(&template, "prompt-clarify", "mission-abc");
        // 1 root + 2 template nodes = 3
        assert_eq!(g.nodes.len(), 3);
        // 2 root → template + 1 template edge = 3
        assert_eq!(g.edges.len(), 3);
        assert!(g
            .find_node(&NodeId("prompt-clarify:mission-abc:probe".into()))
            .is_some());
        assert!(g
            .find_node(&NodeId("prompt-clarify:mission-abc:refine".into()))
            .is_some());
    }

    #[test]
    fn instantiate_two_skills_on_same_mission_do_not_collide() {
        let template = SkillGraphTemplate {
            nodes: vec![TemplateNode {
                id: "probe".into(),
                kind: "engine_state".into(),
                label: "Probe".into(),
            }],
            edges: vec![],
        };
        let g1 = instantiate(&template, "skill-a", "m1");
        let g2 = instantiate(&template, "skill-b", "m1");
        assert!(g1.find_node(&NodeId("skill-a:m1:probe".into())).is_some());
        assert!(g2.find_node(&NodeId("skill-b:m1:probe".into())).is_some());
    }

    #[test]
    fn instantiate_carries_precondition_and_guard() {
        let template = SkillGraphTemplate {
            nodes: vec![
                TemplateNode {
                    id: "a".into(),
                    kind: "engine_state".into(),
                    label: "A".into(),
                },
                TemplateNode {
                    id: "b".into(),
                    kind: "engine_state".into(),
                    label: "B".into(),
                },
            ],
            edges: vec![TemplateEdge {
                from: "a".into(),
                to: "b".into(),
                kind: "transitions_to".into(),
                precondition: Some("cond".into()),
                guard: Some("guard".into()),
            }],
        };
        let g = instantiate(&template, "s", "m");
        let edge = g
            .edges
            .iter()
            .find(|e| e.kind == EdgeKind::TransitionsTo)
            .expect("transitions_to edge present");
        assert_eq!(edge.precondition.as_deref(), Some("cond"));
        assert_eq!(edge.guard.as_deref(), Some("guard"));
    }

    #[test]
    fn instantiate_empty_template_yields_root_only() {
        let template = SkillGraphTemplate::default();
        let g = instantiate(&template, "noop", "m");
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.edges.len(), 0);
    }

    #[test]
    fn instantiate_node_provenance_is_inferred() {
        let template = SkillGraphTemplate {
            nodes: vec![TemplateNode {
                id: "x".into(),
                kind: "engine_state".into(),
                label: "X".into(),
            }],
            edges: vec![],
        };
        let g = instantiate(&template, "s", "m");
        let node = g.find_node(&NodeId("s:m:x".into())).expect("node present");
        assert_eq!(node.provenance, Provenance::Inferred);
        let attrs: serde_json::Value = serde_json::from_str(&node.attrs_json).expect("attrs valid");
        assert_eq!(attrs["skill_id"], "s");
        assert_eq!(attrs["template_node_id"], "x");
    }

    #[test]
    fn instantiate_unknown_node_kind_falls_back_to_engine_state() {
        let template = SkillGraphTemplate {
            nodes: vec![TemplateNode {
                id: "x".into(),
                kind: "totally-unknown-spec-tag".into(),
                label: "X".into(),
            }],
            edges: vec![],
        };
        let g = instantiate(&template, "s", "m");
        let node = g.find_node(&NodeId("s:m:x".into())).expect("node present");
        assert_eq!(node.kind, NodeKind::EngineState);
    }
}
