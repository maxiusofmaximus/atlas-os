// Atlas OS — Golden eval suite (RFC 20 Fase 22, EVAL.1).
//
// Deterministic, offline invariants over Atlas's own engines. Each task is a
// pure function so the suite runs in CI on every engine change and produces a
// stable pass/fail signal (the "you cannot improve what you cannot measure"
// baseline from research/51). LLM-driven and external (Harbor) suites come in
// EVAL.2/EVAL.3.

use super::{EvalTask, TaskResult};

/// The golden suite: a small, deterministic, offline set of invariants.
pub fn golden_suite() -> Vec<EvalTask> {
    vec![
        EvalTask {
            id: "schema.migrate_idempotent",
            category: "SYS",
            run: schema_migrate_idempotent,
        },
        EvalTask {
            id: "supply.exact_known_passes",
            category: "SEC",
            run: supply_exact_known_passes,
        },
        EvalTask {
            id: "supply.typosquat_blocks",
            category: "SEC",
            run: supply_typosquat_blocks,
        },
        EvalTask {
            id: "supply.install_script_blocks",
            category: "SEC",
            run: supply_install_script_blocks,
        },
        EvalTask {
            id: "supply.env_access_warns",
            category: "SEC",
            run: supply_env_access_warns,
        },
        EvalTask {
            id: "calendar.overlap_half_open",
            category: "DATA",
            run: calendar_overlap_half_open,
        },
        EvalTask {
            id: "planning.availability",
            category: "SYS",
            run: planning_availability_respects_busy,
        },
        EvalTask {
            id: "planning.proactive_trigger",
            category: "SYS",
            run: planning_proactive_trigger,
        },
        EvalTask {
            id: "orchestrator.reliability_gate",
            category: "SYS",
            run: orchestrator_reliability_gate,
        },
        EvalTask {
            id: "orchestrator.request_codec",
            category: "SYS",
            run: orchestrator_request_codec,
        },
        EvalTask {
            id: "coding.llm_diff_codec",
            category: "SYS",
            run: coding_llm_diff_codec,
        },
        EvalTask {
            id: "agent.loop_closes",
            category: "SYS",
            run: agent_loop_closes,
        },
        EvalTask {
            id: "agent.evidence_blocks_done",
            category: "SYS",
            run: agent_evidence_blocks_done,
        },
    ]
}

fn in_memory() -> Result<rusqlite::Connection, TaskResult> {
    let conn = rusqlite::Connection::open_in_memory()
        .map_err(|e| TaskResult::error(format!("open in-memory db: {e}")))?;
    crate::journal::schema::migrate(&conn)
        .map_err(|e| TaskResult::error(format!("migrate: {e}")))?;
    Ok(conn)
}

fn schema_migrate_idempotent() -> TaskResult {
    let conn = match in_memory() {
        Ok(c) => c,
        Err(r) => return r,
    };
    // A second migrate over the same connection must be a no-op, not an error.
    if let Err(e) = crate::journal::schema::migrate(&conn) {
        return TaskResult::error(format!("re-migrate not idempotent: {e}"));
    }
    let version: i64 = match conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |r| r.get(0),
    ) {
        Ok(v) => v,
        Err(e) => return TaskResult::error(format!("read schema version: {e}")),
    };
    TaskResult::assert(
        version == crate::journal::schema::CURRENT_SCHEMA_VERSION,
        format!(
            "schema version {version} != CURRENT_SCHEMA_VERSION {}",
            crate::journal::schema::CURRENT_SCHEMA_VERSION
        ),
    )
}

fn supply_exact_known_passes() -> TaskResult {
    use crate::security::supply_gate::{evaluate_package, SupplyVerdict};
    let report = evaluate_package("react", None);
    TaskResult::assert(
        report.verdict == SupplyVerdict::Pass,
        format!(
            "expected Pass for `react`, got {:?} ({:?})",
            report.verdict, report.reasons
        ),
    )
}

fn supply_typosquat_blocks() -> TaskResult {
    use crate::security::supply_gate::evaluate_package;
    // distance-1 from `react`.
    let report = evaluate_package("reacct", None);
    TaskResult::assert(
        report.verdict.is_blocking(),
        format!(
            "expected Block for typosquat `reacct`, got {:?}",
            report.verdict
        ),
    )
}

fn supply_install_script_blocks() -> TaskResult {
    use crate::security::supply_gate::evaluate_package;
    let manifest = r#"{"scripts":{"postinstall":"echo hi"}}"#;
    let report = evaluate_package("innocent-pkg", Some(manifest));
    TaskResult::assert(
        report.verdict.is_blocking(),
        format!(
            "expected Block for postinstall script, got {:?}",
            report.verdict
        ),
    )
}

fn supply_env_access_warns() -> TaskResult {
    use crate::security::supply_gate::{evaluate_package, SupplyVerdict};
    // A non-install script that reads env → Warn (not Block).
    let manifest = r#"{"scripts":{"test":"node process.env.CI"}}"#;
    let report = evaluate_package("innocent-pkg", Some(manifest));
    TaskResult::assert(
        report.verdict == SupplyVerdict::Warn,
        format!(
            "expected Warn for env read, got {:?} ({:?})",
            report.verdict, report.reasons
        ),
    )
}

fn calendar_overlap_half_open() -> TaskResult {
    use crate::calendar::payload::BusySource;
    use crate::calendar::queue::{BusyWindowInput, BusyWindowQueue};

    let conn = match in_memory() {
        Ok(c) => c,
        Err(r) => return r,
    };
    let queue = BusyWindowQueue::new(&conn);
    let input = BusyWindowInput {
        source: BusySource::Manual,
        external_id: "golden-e1",
        subject: "golden",
        body: None,
        starts_at: 1_000,
        ends_at: 2_000,
        weight: 1.0,
    };
    if let Err(e) = queue.upsert(&input) {
        return TaskResult::error(format!("upsert: {e}"));
    }
    let hit = match queue.overlapping(1_500, 1_600) {
        Ok(v) => v,
        Err(e) => return TaskResult::error(format!("overlap hit: {e}")),
    };
    let touching = match queue.overlapping(2_000, 3_000) {
        Ok(v) => v,
        Err(e) => return TaskResult::error(format!("overlap boundary: {e}")),
    };
    TaskResult::assert(
        hit.len() == 1 && touching.is_empty(),
        format!(
            "half-open overlap broken: hit={} touching={}",
            hit.len(),
            touching.len()
        ),
    )
}

fn planning_availability_respects_busy() -> TaskResult {
    use crate::calendar::payload::BusySource;
    use crate::calendar::queue::BusyWindowRow;
    use crate::planning::availability::{next_free_slot, Availability, TurnPolicy};

    let policy = TurnPolicy::default();
    let free = next_free_slot(&[], 1_000_000, &policy);
    let busy = [BusyWindowRow {
        id: 0,
        source: BusySource::Manual,
        external_id: "e".into(),
        subject: "busy".into(),
        body: None,
        starts_at: 999_000,
        ends_at: 1_500_000,
        weight: 1.0,
        recorded_at: 0,
    }];
    let waiting = next_free_slot(&busy, 1_000_000, &policy);
    TaskResult::assert(
        free == Availability::RunNow && waiting == Availability::WaitUntil(1_500_000),
        format!("free={free:?} waiting={waiting:?}"),
    )
}

fn planning_proactive_trigger() -> TaskResult {
    use crate::planning::availability::Availability;
    use crate::supervisor::runner::{tick, TickContext};
    use crate::supervisor::types::{
        BudgetCaps, ExecutionMode, SupervisorAction, SupervisorEvent, SupervisorState,
    };

    let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
    let state = SupervisorState::new(BudgetCaps::DEFAULT, ExecutionMode::HumanInLoop);
    let mission = uuid::Uuid::new_v4();

    let run = tick(
        &mut ctx,
        state.clone(),
        SupervisorEvent::ProactiveCheck {
            availability: Availability::RunNow,
            pending_mission: Some(mission),
        },
    );
    let wait = tick(
        &mut ctx,
        state,
        SupervisorEvent::ProactiveCheck {
            availability: Availability::WaitUntil(1),
            pending_mission: Some(mission),
        },
    );

    let started = matches!(
        run.actions.as_slice(),
        [SupervisorAction::EnqueueProactiveTurn { mission_id }] if *mission_id == mission
    );
    TaskResult::assert(
        started && wait.actions.is_empty(),
        format!("run={:?} wait={:?}", run.actions, wait.actions),
    )
}

fn orchestrator_reliability_gate() -> TaskResult {
    use std::collections::HashMap;

    use crate::eval::metrics::Reliability;
    use crate::orchestrator::reliability_gate::{gate_model, GateDecision, ReliabilityGate};

    let rel = |model: &str, n: i64, pass_rate: f64| Reliability {
        model: model.to_string(),
        n,
        pass_rate,
        tokens_per_solved: 0.0,
        cost_per_solved: 0.0,
    };
    let mut reliabilities: HashMap<String, Reliability> = HashMap::new();
    reliabilities.insert("bad".into(), rel("bad", 50, 0.1));
    reliabilities.insert("thin".into(), rel("thin", 3, 0.0));

    let gate = ReliabilityGate::default();
    let bad = gate_model("bad", &reliabilities, &gate);
    let thin = gate_model("thin", &reliabilities, &gate);
    let unknown = gate_model("mystery", &reliabilities, &gate);

    TaskResult::assert(
        bad == GateDecision::Deny("pass_rate below threshold")
            && thin == GateDecision::Allow
            && unknown == GateDecision::Allow,
        format!("bad={bad:?} thin={thin:?} unknown={unknown:?}"),
    )
}

fn orchestrator_request_codec() -> TaskResult {
    use crate::orchestrator::client::{
        build_chat_body, parse_chat_response, ChatMessage, ChatRequest,
    };

    let request = ChatRequest {
        model: "m".into(),
        messages: vec![ChatMessage::user("hi")],
        temperature: Some(0.2),
        max_tokens: Some(64),
    };
    let body = build_chat_body(&request);
    let body_ok =
        body.get("model").and_then(|m| m.as_str()) == Some("m") && body.get("messages").is_some();

    let canned = serde_json::json!({
        "model": "m",
        "choices": [{ "message": { "content": "hello" } }],
        "usage": { "prompt_tokens": 3, "completion_tokens": 1 }
    });
    let parsed = parse_chat_response(&canned);
    let parse_ok = matches!(
        &parsed,
        Ok(r) if r.content == "hello" && r.usage.map(|u| u.prompt_tokens) == Some(3)
    );

    TaskResult::assert(
        body_ok && parse_ok,
        format!("body_ok={body_ok} parsed={parsed:?}"),
    )
}

fn coding_llm_diff_codec() -> TaskResult {
    use crate::coding::llm::{parse_diff_json, DiffMeta};

    let meta = DiffMeta {
        mission_id: uuid::Uuid::new_v4(),
        plan_id: uuid::Uuid::new_v4(),
        step_id: "S1".into(),
        agent_id: uuid::Uuid::new_v4(),
        model_id: "gpt-x".into(),
    };
    let fenced = "here:\n```json\n{\"narrative\":\"n\",\"files\":[{\"path\":\"src\\\\lib.rs\",\"hunks\":[{\"old_start\":0,\"old_end\":0,\"new_lines\":[\"pub fn hi() {}\"],\"rationale\":\"r\"}]}]}\n```";
    let parsed = parse_diff_json(fenced, meta.clone());
    let ok = matches!(
        &parsed,
        Ok(d) if d.files[0].path == "src/lib.rs"
            && d.files[0].hunks[0].new_lines == vec!["pub fn hi() {}".to_string()]
            && d.model_id == "gpt-x"
    );
    let rejected = parse_diff_json("no json at all", meta).is_err();
    TaskResult::assert(
        ok && rejected,
        format!("ok={ok} rejected={rejected} parsed={parsed:?}"),
    )
}

/// RFC 63 §10 — `agent.loop_closes`: the agent runs a tool, verifies the
/// artifact, and closes the loop. Offline (scripted client), deterministic.
fn agent_loop_closes() -> TaskResult {
    run_agent_case(
        vec![
            r#"{"tool":"fs.write","args":{"path":"out.txt","content":"hello world"}}"#.into(),
            r#"{"done":true,"summary":"wrote out.txt"}"#.into(),
        ],
        &[
            crate::orchestrator::artifacts::ArtifactCheck::FileContains {
                path: "out.txt".into(),
                needle: "hello world".into(),
            },
        ],
        true,
    )
}

/// RFC 63 §10 — `agent.evidence_blocks_done`: a `done` with no evidence is
/// rejected, forcing another turn until the predicate holds.
fn agent_evidence_blocks_done() -> TaskResult {
    run_agent_case(
        vec![
            r#"{"done":true,"summary":"claim"}"#.into(),
            r#"{"tool":"fs.write","args":{"path":"out.txt","content":"ok"}}"#.into(),
            r#"{"done":true,"summary":"really done"}"#.into(),
        ],
        &[crate::orchestrator::artifacts::ArtifactCheck::FileExists {
            path: "out.txt".into(),
        }],
        true,
    )
}

/// Shared driver: run the agent loop offline with a scripted client and the
/// core `ToolRegistry`, against a temp root with `checks` as success predicate.
fn run_agent_case(
    replies: Vec<String>,
    checks: &[crate::orchestrator::artifacts::ArtifactCheck],
    expect_done: bool,
) -> TaskResult {
    use crate::orchestrator::agent::{run_agent, AgentConfig};
    use crate::orchestrator::client::{ChatRequest, ChatResponse, ClientResult, ProviderClient};
    use crate::orchestrator::provider::Deployment;
    use crate::orchestrator::routing::RoutingConfig;
    use std::sync::Mutex;

    struct Scripted {
        replies: Mutex<Vec<String>>,
    }
    impl ProviderClient for Scripted {
        async fn chat(&self, _d: &Deployment, _r: &ChatRequest) -> ClientResult<ChatResponse> {
            let c = self.replies.lock().unwrap().remove(0);
            Ok(ChatResponse {
                model: "m".into(),
                content: c,
                usage: None,
            })
        }
    }

    let tmp = match tempfile::TempDir::new() {
        Ok(d) => d,
        Err(e) => return TaskResult::error(format!("tmp: {e}")),
    };
    let root = tmp.path().to_path_buf();
    let client = Scripted {
        replies: Mutex::new(replies),
    };
    let cfg = AgentConfig {
        success_predicate: checks.to_vec(),
        tools: vec![("fs.write".into(), "write a file".into())],
        ..AgentConfig::default()
    };
    let reg = crate::orchestrator::tools::ToolRegistry::with_core_tools();
    let exec_root = root.clone();
    let vr = root.clone();
    let checks = checks.to_vec();
    let dep = {
        let mut d = Deployment::new("m", "http://x");
        d.id = "d1".into();
        d
    };

    // `futures::executor::block_on` works both standalone (cargo test) and
    // inside a tokio runtime (the CLI's async main) — unlike `Runtime::block_on`,
    // which panics with "cannot start a runtime from within a runtime". The
    // scripted client does no I/O, so no reactor is needed.
    let outcome = futures::executor::block_on(async {
        run_agent(
            &client,
            RoutingConfig::default(),
            "m",
            &[dep],
            "do the task",
            &root,
            &cfg,
            &[],
            move |tool, args| {
                let ctx = crate::orchestrator::tools::ToolContext::new(exec_root.clone());
                reg.call(tool, &ctx, args).unwrap_or_else(|e| {
                    crate::orchestrator::tools::ToolResult::err(tool, e.to_string())
                })
            },
            move || {
                crate::orchestrator::artifacts::verify_artifacts(
                    &vr,
                    &checks,
                    &crate::orchestrator::sandbox::LocalSandbox,
                )
            },
        )
        .await
    });

    match outcome {
        Ok(o) => {
            let ok = o.done == expect_done;
            TaskResult::assert(
                ok,
                format!("done={} expected={} turns={}", o.done, expect_done, o.turns),
            )
        }
        Err(e) => TaskResult::fail("REASON", format!("agent loop errored: {e:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::TaskStatus;

    #[test]
    fn every_golden_task_passes_standalone() {
        let suite = golden_suite();
        assert!(suite.len() >= 5);
        for task in &suite {
            let result = (task.run)();
            assert_eq!(
                result.status,
                TaskStatus::Pass,
                "golden task `{}` did not pass: {:?}",
                task.id,
                result.detail
            );
        }
    }
}
