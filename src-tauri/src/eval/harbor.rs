// Atlas OS — Harbor result importer (RFC 20 Fase 22, EVAL.3; plan research/51).
//
// Ingests a Harbor job result into `eval_runs` / `eval_cases` so an external,
// reproducible baseline (Terminal-Bench 2.0 / SWE-bench Verified) lands in the
// same store as the local golden suite — the "harness × model" unit of measure
// the field converges on.
//
// Grounded in Harbor's `JobResult` / `TrialResult` pydantic schema:
//   results/<job>/result.json          (JobResult: id, trial_results[], stats)
//   results/<job>/<task>__<trial>/result.json  (TrialResult)
// Fields read (tolerantly, via serde_json::Value so schema drift does not
// break ingestion): task_name, trial_name, source, agent_info.name,
// agent_info.model_info.name | model_name, agent_result.{n_input_tokens,
// n_output_tokens,cost_usd}, verifier_result.rewards{}, exception_info.{type,
// message}, started_at/finished_at.
//
// Parsing is offline (no Harbor install, no Docker): `atlas eval import <path>`
// where `<path>` is a JobResult `result.json` file, a single TrialResult file,
// or a job directory.

use std::path::Path;

use serde_json::Value;

use crate::journal::{EvalCaseInput, EvalRunStart, EvalTotals, Journal};

use super::{SuiteReport, TaskStatus};

const HARNESS: &str = "harbor";

/// Import a Harbor job (or a single trial) into `eval_runs`/`eval_cases`.
pub fn import_job(journal: &Journal, path: &Path) -> anyhow::Result<SuiteReport> {
    let job = load_json(path)?;
    let trials = load_trials(&job);
    if trials.is_empty() {
        anyhow::bail!(
            "no `trial_results` (and not a TrialResult) in {}",
            path.display()
        );
    }

    let first = &trials[0];
    let agent = str_at(first, &["agent_info", "name"]).unwrap_or_else(|| "unknown".into());
    let model = str_at(first, &["agent_info", "model_info", "name"])
        .or_else(|| str_at(first, &["agent_info", "model_name"]));
    let suite = str_at(first, &["source"]).unwrap_or_else(|| "harbor".into());
    let metadata = serde_json::json!({
        "harbor_job_id": str_at(&job, &["id"]).unwrap_or_default(),
        "n_total_trials": job.get("n_total_trials").and_then(Value::as_i64),
    })
    .to_string();

    let run_id = journal.eval_run_start(&EvalRunStart {
        suite: &suite,
        agent: &agent,
        harness: HARNESS,
        model: model.as_deref(),
        metadata_json: Some(&metadata),
    })?;

    let (mut passed, mut failed, mut errored) = (0i64, 0i64, 0i64);
    let (mut tokens_total, mut cost_total) = (0i64, 0.0f64);

    for trial in &trials {
        let task_name = str_at(trial, &["task_name"]).unwrap_or_else(|| "unknown".into());
        let case_id = str_at(trial, &["trial_name"]).unwrap_or_else(|| task_name.clone());
        let category = str_at(trial, &["source"]).unwrap_or_else(|| "harbor".into());

        let exception = trial.get("exception_info").filter(|v| !v.is_null());
        let rewards = trial.pointer("/verifier_result/rewards");
        let solved = rewards.and_then(max_reward).map(|r| r >= 1.0);

        let status = if exception.is_some() {
            TaskStatus::Error
        } else if solved.unwrap_or(false) {
            TaskStatus::Pass
        } else {
            TaskStatus::Fail
        };
        let failure_kind: Option<&'static str> = match status {
            TaskStatus::Error => Some("REASON"),
            TaskStatus::Fail => Some("VERIFY"),
            TaskStatus::Pass => None,
        };
        let detail = exception
            .and_then(|e| str_at(e, &["type"]).or_else(|| str_at(e, &["message"])))
            .or_else(|| rewards.map(|r| r.to_string()));

        let tokens_in = int_at(trial, &["agent_result", "n_input_tokens"]).unwrap_or(0);
        let tokens_out = int_at(trial, &["agent_result", "n_output_tokens"]).unwrap_or(0);
        let cost_usd = float_at(trial, &["agent_result", "cost_usd"]).unwrap_or(0.0);
        let duration_ms = duration_ms_between(trial);

        match status {
            TaskStatus::Pass => passed += 1,
            TaskStatus::Fail => failed += 1,
            TaskStatus::Error => errored += 1,
        }
        tokens_total += tokens_in + tokens_out;
        cost_total += cost_usd;

        journal.eval_run_record_case(&EvalCaseInput {
            run_id: &run_id,
            case_id: &case_id,
            category: Some(&category),
            status: status.as_str(),
            duration_ms,
            turns: 0,
            no_action_turns: 0,
            tokens_in,
            tokens_out,
            cost_usd,
            failure_kind,
            detail: detail.as_deref(),
        })?;
    }

    let total = trials.len() as i64;
    journal.eval_run_finish(
        &run_id,
        &EvalTotals {
            status: "completed",
            total,
            passed,
            failed: failed + errored,
            tokens_total,
            cost_usd: cost_total,
        },
    )?;

    Ok(SuiteReport {
        run_id,
        total,
        passed,
        failed,
        errored,
        duration_ms: 0,
    })
}

fn load_json(path: &Path) -> anyhow::Result<Value> {
    let file = if path.is_dir() {
        path.join("result.json")
    } else {
        path.to_path_buf()
    };
    let text = std::fs::read_to_string(&file)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", file.display()))?;
    serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("parse {}: {e}", file.display()))
}

fn load_trials(job: &Value) -> Vec<Value> {
    if let Some(arr) = job.get("trial_results").and_then(Value::as_array) {
        return arr.clone();
    }
    if job.get("task_name").is_some() {
        return vec![job.clone()];
    }
    Vec::new()
}

fn str_at(v: &Value, path: &[&str]) -> Option<String> {
    let mut cur = v;
    for key in path {
        cur = cur.get(key)?;
    }
    cur.as_str().map(str::to_string)
}

fn int_at(v: &Value, path: &[&str]) -> Option<i64> {
    let mut cur = v;
    for key in path {
        cur = cur.get(key)?;
    }
    cur.as_i64()
}

fn float_at(v: &Value, path: &[&str]) -> Option<f64> {
    let mut cur = v;
    for key in path {
        cur = cur.get(key)?;
    }
    cur.as_f64()
}

fn max_reward(rewards: &Value) -> Option<f64> {
    rewards
        .as_object()?
        .values()
        .filter_map(Value::as_f64)
        .reduce(f64::max)
}

fn duration_ms_between(trial: &Value) -> i64 {
    let started =
        str_at(trial, &["started_at"]).and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok());
    let finished =
        str_at(trial, &["finished_at"]).and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok());
    match (started, finished) {
        (Some(a), Some(b)) => (b - a).num_milliseconds().max(0),
        _ => 0,
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

    const JOB: &str = r#"{
      "id": "job-abc",
      "started_at": "2026-10-03T10:00:00Z",
      "finished_at": "2026-10-03T10:10:00Z",
      "n_total_trials": 3,
      "trial_results": [
        {
          "task_name": "task-a", "trial_name": "task-a__1", "trial_uri": "",
          "task_id": "task-a", "source": "terminal-bench", "task_checksum": "x", "config": {},
          "agent_info": {"name": "atlas", "version": "0.1.1", "model_info": {"name": "qwen3-coder-next"}},
          "agent_result": {"n_input_tokens": 1000, "n_output_tokens": 200, "cost_usd": 0.010},
          "verifier_result": {"rewards": {"reward": 1.0}},
          "started_at": "2026-10-03T10:00:00Z", "finished_at": "2026-10-03T10:01:00Z"
        },
        {
          "task_name": "task-b", "trial_name": "task-b__1", "trial_uri": "",
          "task_id": "task-b", "source": "terminal-bench", "task_checksum": "x", "config": {},
          "agent_info": {"name": "atlas", "version": "0.1.1", "model_info": {"name": "qwen3-coder-next"}},
          "agent_result": {"n_input_tokens": 500, "n_output_tokens": 100, "cost_usd": 0.005},
          "verifier_result": {"rewards": {"reward": 0.0}},
          "started_at": "2026-10-03T10:01:00Z", "finished_at": "2026-10-03T10:02:00Z"
        },
        {
          "task_name": "task-c", "trial_name": "task-c__1", "trial_uri": "",
          "task_id": "task-c", "source": "terminal-bench", "task_checksum": "x", "config": {},
          "agent_info": {"name": "atlas", "version": "0.1.1"},
          "exception_info": {"type": "AgentTimeoutError", "message": "timed out"},
          "started_at": "2026-10-03T10:02:00Z", "finished_at": "2026-10-03T10:03:00Z"
        }
      ]
    }"#;

    #[test]
    fn imports_job_result_with_pass_fail_and_error() {
        let (dir, journal) = fresh_journal();
        let file = dir.path().join("result.json");
        std::fs::write(&file, JOB).unwrap();

        let report = import_job(&journal, &file).expect("import");
        assert_eq!(report.total, 3);
        assert_eq!(report.passed, 1);
        assert_eq!(report.failed, 1);
        assert_eq!(report.errored, 1);

        let run = journal.eval_run_get(&report.run_id).unwrap().unwrap();
        assert_eq!(run.harness, "harbor");
        assert_eq!(run.agent, "atlas");
        assert_eq!(run.suite, "terminal-bench");
        assert_eq!(run.model.as_deref(), Some("qwen3-coder-next"));
        assert_eq!(run.tokens_total, 1800);
        assert!((run.cost_usd - 0.015).abs() < 1e-9);
        assert!(run.metadata_json.unwrap().contains("job-abc"));

        let cases = journal.eval_cases(&report.run_id).unwrap();
        assert_eq!(cases.len(), 3);
        let a = cases.iter().find(|c| c.case_id == "task-a__1").unwrap();
        assert_eq!(a.status, "pass");
        assert_eq!(a.category.as_deref(), Some("terminal-bench"));
        assert_eq!(a.tokens_in, 1000);
        assert_eq!(a.duration_ms, 60_000);
        let b = cases.iter().find(|c| c.case_id == "task-b__1").unwrap();
        assert_eq!(b.status, "fail");
        assert_eq!(b.failure_kind.as_deref(), Some("VERIFY"));
        let c = cases.iter().find(|c| c.case_id == "task-c__1").unwrap();
        assert_eq!(c.status, "error");
        assert_eq!(c.failure_kind.as_deref(), Some("REASON"));
        assert_eq!(c.detail.as_deref(), Some("AgentTimeoutError"));
    }

    #[test]
    fn imports_from_job_directory() {
        let (dir, journal) = fresh_journal();
        let job_dir = dir.path().join("jobs").join("run-1");
        std::fs::create_dir_all(&job_dir).unwrap();
        std::fs::write(job_dir.join("result.json"), JOB).unwrap();

        let report = import_job(&journal, &job_dir).expect("import dir");
        assert_eq!(report.total, 3);
    }

    #[test]
    fn imports_single_trial_result() {
        let (_dir, journal) = fresh_journal();
        let single = r#"{
          "task_name": "solo", "trial_name": "solo__1", "trial_uri": "",
          "task_id": "solo", "source": "swe-bench", "task_checksum": "x", "config": {},
          "agent_info": {"name": "atlas", "version": "0.1.1", "model_name": "local"},
          "verifier_result": {"rewards": {"reward": 1}}
        }"#;
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("result.json");
        std::fs::write(&file, single).unwrap();

        let report = import_job(&journal, &file).expect("import single");
        assert_eq!(report.total, 1);
        assert_eq!(report.passed, 1);
        let run = journal.eval_run_get(&report.run_id).unwrap().unwrap();
        assert_eq!(run.suite, "swe-bench");
        assert_eq!(run.model.as_deref(), Some("local"));
    }

    #[test]
    fn missing_file_errors() {
        let (_dir, journal) = fresh_journal();
        let err = import_job(&journal, Path::new("does-not-exist.json")).unwrap_err();
        assert!(format!("{err}").contains("read"));
    }
}
