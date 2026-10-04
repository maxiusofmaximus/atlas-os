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
