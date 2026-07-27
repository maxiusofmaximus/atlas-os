// OpenCode OS — Planner DAG emitter (RFC 12 §3.1 / RFC 28 §C item 4).
//
// Gated behind `dag_mode`. The default Planning Engine output stays the
// linear `Plan { steps: Vec<Step> }` contract from RFC 12 §3; when a
// caller opts into `dag_mode=true`, this emitter translates the same
// `Plan` into the in-memory `MissionGraph` (RFC 28 §C item 3) and the
// supervisor stores it in M15 `mission_graph_nodes`/`mission_graph_edges`.
//
// Why a pure transform (not a new field on `Plan`):
//   * `Plan` is the cross-RFC contract consumed by the Coding Engine
//     (RFC 13) and persisted by the Journal. Touching its schema would
//     cascade into runner_tests + Journal round-trips.
//   * The DAG view is purely a *projection* of the same information.
//     Round-trips through this emitter are idempotent — running it
//     twice on the same `Plan` yields the same graph (modulo fresh
//     edge ids, which we derive deterministically from string stems).
//
// Node provenance is `Inferred` everywhere: Planner output is a model
// hypothesis, not raw source — that's theGraphNode tag that survives
// `EXTRACTED` is reserved for `graph::ast` ingest of actual source files.
//
// Node `kind` is `Mission` for everything emitted here. The Planner
// does not know about engine states (those come from the supervisor
// DAG in RFC 19 §6.1.1) or skills/external artifacts (those come from
// the Skills `graph.toml` loader, RFC 23 §7.3 / RFC 28 §C item 5).

#![cfg_attr(not(feature = "dag_mode"), allow(unused, dead_code, unused_imports))]

use crate::graph::{Edge, EdgeId, EdgeKind, MissionGraph, Node, NodeId, NodeKind, Provenance};
use crate::planning::{Plan, Step};

/// Build a `MissionGraph` from a `Plan`. The root node is tagged
/// `mission:{mission_id}` and every emitted node is reached from it
/// via `EdgeKind::DependsOn`. Edge ids are derived from the source
/// and destination stems so two runs over the same plan produce an
/// identical node *set* and identical edge *labels* (the audit log
/// cares about labels, not the opaque id strings).
pub fn plan_to_graph(plan: &Plan) -> MissionGraph {
    let mission_root_id = format!("mission:{}", plan.mission_id);
    let mut g = MissionGraph::new_rooted(&mission_root_id);

    for objective in &plan.objectives {
        let node_id = NodeId(format!("obj:{}", objective.id));
        g.push_node(Node {
            id: node_id.clone(),
            mission_id: plan.mission_id.to_string(),
            kind: NodeKind::Mission,
            label: objective.statement.clone(),
            provenance: Provenance::Inferred,
            attrs_json: serde_json::json!({
                "verifiable_via": objective
                    .verifiable_via
                    .iter()
                    .map(|c| c.description.clone())
                    .collect::<Vec<_>>(),
            })
            .to_string(),
        });
        g.push_edge(edge(
            &format!("e:root:obj:{}", objective.id),
            &mission_root_id,
            node_id.as_str(),
            EdgeKind::DependsOn,
        ));
        for dep in &objective.depends_on {
            g.push_edge(edge(
                &format!("e:obj:{dep}:{}", objective.id),
                &format!("obj:{dep}"),
                node_id.as_str(),
                EdgeKind::DependsOn,
            ));
        }
    }

    for milestone in &plan.roadmap {
        let node_id = NodeId(format!("ms:{}", milestone.id));
        g.push_node(Node {
            id: node_id.clone(),
            mission_id: plan.mission_id.to_string(),
            kind: NodeKind::Mission,
            label: milestone.label.clone(),
            provenance: Provenance::Inferred,
            attrs_json: serde_json::json!({
                "objectives": milestone.objectives,
            })
            .to_string(),
        });
        g.push_edge(edge(
            &format!("e:root:ms:{}", milestone.id),
            &mission_root_id,
            node_id.as_str(),
            EdgeKind::DependsOn,
        ));
        for dep in &milestone.depends_on {
            g.push_edge(edge(
                &format!("e:ms:{dep}:{}", milestone.id),
                &format!("ms:{dep}"),
                node_id.as_str(),
                EdgeKind::DependsOn,
            ));
        }
    }

    for step in &plan.steps {
        push_step_node(&mut g, plan, step);
        g.push_edge(edge(
            &format!("e:root:step:{}", step.id),
            &mission_root_id,
            &format!("step:{}", step.id),
            EdgeKind::DependsOn,
        ));
        for dep in &step.depends_on {
            g.push_edge(edge(
                &format!("e:step:{dep}:{}", step.id),
                &format!("step:{dep}"),
                &format!("step:{}", step.id),
                EdgeKind::DependsOn,
            ));
        }
    }

    g
}

fn push_step_node(g: &mut MissionGraph, plan: &Plan, step: &Step) {
    let node_id = NodeId(format!("step:{}", step.id));
    g.push_node(Node {
        id: node_id.clone(),
        mission_id: plan.mission_id.to_string(),
        kind: NodeKind::Mission,
        label: step.statement.clone(),
        provenance: Provenance::Inferred,
        attrs_json: serde_json::json!({
            "action": step.action.tag(),
            "milestone_id": step.milestone_id,
            "read_only": step.read_only,
            "skills": step
                .skills
                .iter()
                .map(|s| s.skill_id.clone())
                .collect::<Vec<_>>(),
            "models": step
                .models
                .iter()
                .map(|m| m.model_id.clone())
                .collect::<Vec<_>>(),
        })
        .to_string(),
    });
    g.push_edge(edge(
        &format!("e:ms:{}:step:{}", step.milestone_id, step.id),
        &format!("ms:{}", step.milestone_id),
        node_id.as_str(),
        EdgeKind::DependsOn,
    ));
}

fn edge(id: &str, src: &str, dst: &str, kind: EdgeKind) -> Edge {
    Edge {
        id: EdgeId(id.to_string()),
        mission_id: String::new(),
        src: NodeId(src.to_string()),
        dst: NodeId(dst.to_string()),
        kind,
        precondition: None,
        guard: None,
        visit_count: 0,
    }
}

#[cfg(all(test, feature = "dag_mode"))]
mod tests {
    use super::*;
    use crate::planning::{
        Impact, Milestone, Objective, Plan, Step, StepAction, Strategy, VerificationCriterion,
        VerificationKind,
    };
    use uuid::Uuid;

    fn sample_plan() -> Plan {
        let mission_id = Uuid::new_v4();
        Plan {
            plan_id: Uuid::new_v4(),
            mission_id,
            verdict_id: Uuid::new_v4(),
            generated_at: "2026-07-27T12:00:00Z".into(),
            mission: "Add rate-limited public API".into(),
            objectives: vec![Objective {
                id: "o1".into(),
                statement: "Expose /api/v1/rate".into(),
                verifiable_via: vec![VerificationCriterion {
                    kind: VerificationKind::Test,
                    description: "200 OK under load".into(),
                }],
                depends_on: vec![],
            }],
            steps: vec![
                Step {
                    id: "s1".into(),
                    milestone_id: "m1".into(),
                    statement: "Add route handler".into(),
                    action: StepAction::Create,
                    depends_on: vec![],
                    skills: vec![],
                    models: vec![],
                    read_only: false,
                },
                Step {
                    id: "s2".into(),
                    milestone_id: "m1".into(),
                    statement: "Add integration tests".into(),
                    action: StepAction::Test,
                    depends_on: vec!["s1".into()],
                    skills: vec![],
                    models: vec![],
                    read_only: false,
                },
            ],
            strategy: Strategy::Incremental,
            risk: 0.2,
            impact: Impact::Minor,
            roadmap: vec![Milestone {
                id: "m1".into(),
                label: "Rate limited endpoint live".into(),
                objectives: vec!["o1".into()],
                depends_on: vec![],
            }],
            skills_used: vec![],
            models_needed: vec![],
            research_runs: vec![],
            confidence: 0.8,
            resume_point: "s1".into(),
            blocked: vec![],
            model_id: "heuristic-v0".into(),
            elapsed_ms: 42,
        }
    }

    #[test]
    fn plan_to_graph_has_root_with_out_degree_4() {
        let plan = sample_plan();
        let g = plan_to_graph(&plan);
        // 1 obj + 1 milestone + 2 steps = 4 DependsOn edges out of the root.
        assert_eq!(
            g.out_degree(&NodeId(format!("mission:{}", plan.mission_id))),
            4
        );
    }

    #[test]
    fn plan_to_graph_emits_node_per_objective_milestone_step() {
        let plan = sample_plan();
        let g = plan_to_graph(&plan);
        // 1 mission + 1 obj + 1 milestone + 2 steps = 5 nodes
        assert_eq!(g.nodes.len(), 5);
        assert!(g.find_node(&NodeId("obj:o1".into())).is_some());
        assert!(g.find_node(&NodeId("ms:m1".into())).is_some());
        assert!(g.find_node(&NodeId("step:s1".into())).is_some());
        assert!(g.find_node(&NodeId("step:s2".into())).is_some());
    }

    #[test]
    fn plan_to_graph_step_dependency_edge_present() {
        let plan = sample_plan();
        let g = plan_to_graph(&plan);
        let s2_in_edges: Vec<_> = g
            .in_edges(&NodeId("step:s2".into()))
            .map(|e| e.kind)
            .collect();
        // root -> s2 (DependsOn) + s1 -> s2 (DependsOn) + m1 -> s2 (DependsOn) = 3
        assert_eq!(s2_in_edges.len(), 3);
        assert!(s2_in_edges.iter().all(|k| *k == EdgeKind::DependsOn));
    }

    #[test]
    fn plan_to_graph_milestone_to_step_edge_present() {
        let plan = sample_plan();
        let g = plan_to_graph(&plan);
        let ms_out: Vec<_> = g
            .out_edges(&NodeId("ms:m1".into()))
            .map(|e| (e.src.as_str().to_string(), e.dst.as_str().to_string()))
            .collect();
        assert!(ms_out.iter().any(|(_, dst)| dst == "step:s1"));
        assert!(ms_out.iter().any(|(_, dst)| dst == "step:s2"));
    }

    #[test]
    fn plan_to_graph_idempotent_node_set() {
        let plan = sample_plan();
        let g1 = plan_to_graph(&plan);
        let g2 = plan_to_graph(&plan);
        let ids1: Vec<String> = g1.nodes.iter().map(|n| n.id.as_str().to_string()).collect();
        let ids2: Vec<String> = g2.nodes.iter().map(|n| n.id.as_str().to_string()).collect();
        assert_eq!(ids1, ids2);
        let labels1: Vec<String> = g1.edges.iter().map(|e| e.id.as_str().to_string()).collect();
        let labels2: Vec<String> = g2.edges.iter().map(|e| e.id.as_str().to_string()).collect();
        assert_eq!(labels1, labels2);
    }

    #[test]
    fn plan_to_graph_empty_plan_yields_only_root() {
        let mut plan = sample_plan();
        plan.objectives.clear();
        plan.roadmap.clear();
        plan.steps.clear();
        let g = plan_to_graph(&plan);
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.edges.len(), 0);
    }

    #[test]
    fn plan_to_graph_objective_dependency_chain() {
        let mut plan = sample_plan();
        plan.objectives.push(Objective {
            id: "o2".into(),
            statement: "Document /api/v1/rate".into(),
            verifiable_via: vec![],
            depends_on: vec!["o1".into()],
        });
        let g = plan_to_graph(&plan);
        let edges_from_o1: Vec<_> = g
            .out_edges(&NodeId("obj:o1".into()))
            .map(|e| e.dst.as_str().to_string())
            .collect();
        assert!(edges_from_o1.iter().any(|d| d == "obj:o2"));
    }

    #[test]
    fn plan_to_graph_node_attrs_carry_action_tag() {
        let plan = sample_plan();
        let g = plan_to_graph(&plan);
        let step_node = g
            .find_node(&NodeId("step:s1".into()))
            .expect("step present");
        let attrs: serde_json::Value =
            serde_json::from_str(&step_node.attrs_json).expect("attrs valid json");
        assert_eq!(attrs["action"], "create");
        assert_eq!(attrs["milestone_id"], "m1");
    }

    #[test]
    fn plan_to_graph_provenance_inferred_for_emitted_nodes() {
        let plan = sample_plan();
        let g = plan_to_graph(&plan);
        // Root mission node is inherited from `MissionGraph::new_rooted`
        // (Extracted — the mission itself is a fact, not a hypothesis).
        // Every node we add via the emitter is Inferred.
        for n in &g.nodes {
            if n.id.as_str().starts_with("mission:") {
                assert_eq!(n.provenance, Provenance::Extracted);
            } else {
                assert_eq!(n.provenance, Provenance::Inferred);
            }
        }
    }
}
