// Atlas OS — Evaluation harness (RFC 20 Fase 22 EVAL; plan research/51).
//
// A deterministic, offline suite runner: each `EvalTask` is a pure function
// returning pass/fail + the field's converged metrics, executed in-process and
// persisted into `eval_runs` / `eval_cases` (EVAL.0). This is the local
// "golden suite" half of EVAL.1; the external Harbor adapter (Terminal-Bench /
// SWE-bench) is EVAL.3.
//
// Tasks never touch the network — deterministic invariants over Atlas's own
// engines — so the suite is safe to run in CI on every engine change.

use crate::journal::{EvalCaseInput, EvalRunStart, EvalTotals, Journal};

pub mod golden;
pub mod harbor;
pub mod metrics;

/// Outcome of one task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Pass,
    Fail,
    Error,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TaskStatus::Pass => "pass",
            TaskStatus::Fail => "fail",
            TaskStatus::Error => "error",
        }
    }
}

/// A task's result. Deterministic local tasks carry `turns = 1`, zero tokens
/// and zero cost; the fields exist so the same shape serves the LLM-driven
/// suites (EVAL.3) without a schema change.
#[derive(Clone, Debug)]
pub struct TaskResult {
    pub status: TaskStatus,
    pub failure_kind: Option<&'static str>,
    pub detail: Option<String>,
    pub turns: i64,
    pub no_action_turns: i64,
}

impl TaskResult {
    pub fn pass() -> Self {
        Self {
            status: TaskStatus::Pass,
            failure_kind: None,
            detail: None,
            turns: 1,
            no_action_turns: 0,
        }
    }

    pub fn fail(kind: &'static str, detail: impl Into<String>) -> Self {
        Self {
            status: TaskStatus::Fail,
            failure_kind: Some(kind),
            detail: Some(detail.into()),
            turns: 1,
            no_action_turns: 0,
        }
    }

    pub fn error(detail: impl Into<String>) -> Self {
        Self {
            status: TaskStatus::Error,
            failure_kind: Some("REASON"),
            detail: Some(detail.into()),
            turns: 1,
            no_action_turns: 0,
        }
    }

    /// Assert a boolean; `Pass` when true, else `Fail` with kind `VERIFY`.
    pub fn assert(cond: bool, fail_detail: impl Into<String>) -> Self {
        if cond {
            Self::pass()
        } else {
            Self::fail("VERIFY", fail_detail)
        }
    }
}

/// One named, deterministic check.
pub struct EvalTask {
    pub id: &'static str,
    /// Task archetype (KDD buckets: SYS, SEC, DATA, BUILD, IMPL, BUG, …).
    pub category: &'static str,
    pub run: fn() -> TaskResult,
}

/// Summary of one suite execution.
#[derive(Clone, Debug, PartialEq)]
pub struct SuiteReport {
    pub run_id: String,
    pub total: i64,
    pub passed: i64,
    pub failed: i64,
    pub errored: i64,
    pub duration_ms: i64,
}

/// Execute `tasks` as suite `suite`, persisting each case and closing the run.
/// A panicking task is recorded as `error` (kind `REASON`) rather than
/// aborting the suite.
pub fn run_suite(
    journal: &Journal,
    suite: &str,
    tasks: &[EvalTask],
) -> anyhow::Result<SuiteReport> {
    let run_id = journal.eval_run_start(&EvalRunStart {
        suite,
        agent: "atlas",
        harness: "atlas-local",
        model: None,
        metadata_json: None,
    })?;

    let started = std::time::Instant::now();
    let (mut passed, mut failed, mut errored) = (0i64, 0i64, 0i64);

    for task in tasks {
        let t0 = std::time::Instant::now();
        let result = std::panic::catch_unwind(task.run)
            .unwrap_or_else(|payload| TaskResult::error(panic_message(payload)));
        let duration_ms = t0.elapsed().as_millis() as i64;

        match result.status {
            TaskStatus::Pass => passed += 1,
            TaskStatus::Fail => failed += 1,
            TaskStatus::Error => errored += 1,
        }

        journal.eval_run_record_case(&EvalCaseInput {
            run_id: &run_id,
            case_id: task.id,
            category: Some(task.category),
            status: result.status.as_str(),
            duration_ms,
            turns: result.turns,
            no_action_turns: result.no_action_turns,
            tokens_in: 0,
            tokens_out: 0,
            cost_usd: 0.0,
            failure_kind: result.failure_kind,
            detail: result.detail.as_deref(),
        })?;
    }

    let total = tasks.len() as i64;
    let duration_ms = started.elapsed().as_millis() as i64;
    journal.eval_run_finish(
        &run_id,
        &EvalTotals {
            status: "completed",
            total,
            passed,
            failed: failed + errored,
            tokens_total: 0,
            cost_usd: 0.0,
        },
    )?;

    Ok(SuiteReport {
        run_id,
        total,
        passed,
        failed,
        errored,
        duration_ms,
    })
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "task panicked".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_journal() -> (tempfile::TempDir, Journal) {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        (tmp, journal)
    }

    #[test]
    fn golden_suite_passes_fully_and_persists() {
        let (_dir, journal) = fresh_journal();
        let tasks = golden::golden_suite();
        let report = run_suite(&journal, "golden", &tasks).expect("run");

        assert!(report.total >= 5, "expected a non-trivial suite");
        assert_eq!(report.passed, report.total, "all golden tasks must pass");
        assert_eq!(report.failed, 0);
        assert_eq!(report.errored, 0);

        let run = journal.eval_run_get(&report.run_id).unwrap().unwrap();
        assert_eq!(run.status, "completed");
        assert_eq!(run.suite, "golden");
        assert_eq!(run.harness, "atlas-local");
        assert_eq!(run.total, report.total);

        let cases = journal.eval_cases(&report.run_id).unwrap();
        assert_eq!(cases.len() as i64, report.total);
        assert!(cases.iter().all(|c| c.status == "pass"));
    }

    fn always_fail() -> TaskResult {
        TaskResult::fail("VERIFY", "intentional failure")
    }

    #[test]
    fn failing_task_is_recorded_with_failure_kind() {
        let (_dir, journal) = fresh_journal();
        let tasks = [
            EvalTask {
                id: "t.pass",
                category: "SYS",
                run: TaskResult::pass,
            },
            EvalTask {
                id: "t.fail",
                category: "SYS",
                run: always_fail,
            },
        ];
        let report = run_suite(&journal, "synthetic", &tasks).expect("run");
        assert_eq!(report.passed, 1);
        assert_eq!(report.failed, 1);
        assert_eq!(report.errored, 0);

        let cases = journal.eval_cases(&report.run_id).unwrap();
        assert_eq!(cases.len(), 2);
        let bad = cases.iter().find(|c| c.case_id == "t.fail").unwrap();
        assert_eq!(bad.status, "fail");
        assert_eq!(bad.failure_kind.as_deref(), Some("VERIFY"));
    }
}
