// Atlas OS — Evaluation harness repository (RFC 20 Fase 22, EVAL.0).
//
// Durable storage for agent/engine evaluation runs. This is the deferred
// `eval_runs` (gap G16, Phase 2.5) resurrected as a first-class phase: the
// 2026 harness-engineering research converged on evaluation/instrumentation
// as the highest-leverage missing layer (see research/51).
//
// One `eval_runs` row per suite execution — the unit of measure is the
// harness × model pair — and one `eval_cases` row per task. Cases carry the
// field's converged metrics: pass/fail, turns, no-action turns, tokens,
// cost, and a failure-kind vector (KDD "The Scaffold Effect").

use super::Journal;

/// One `eval_runs` row — a completed (or running) suite execution.
#[derive(Clone, Debug, PartialEq)]
pub struct EvalRunRow {
    pub id: String,
    pub suite: String,
    pub agent: String,
    pub harness: String,
    pub model: Option<String>,
    pub status: String,
    pub total: i64,
    pub passed: i64,
    pub failed: i64,
    pub tokens_total: i64,
    pub cost_usd: f64,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub metadata_json: Option<String>,
}

/// One `eval_cases` row — a single task result within a run.
#[derive(Clone, Debug, PartialEq)]
pub struct EvalCaseRow {
    pub id: String,
    pub run_id: String,
    pub case_id: String,
    pub category: Option<String>,
    pub status: String,
    pub duration_ms: i64,
    pub turns: i64,
    pub no_action_turns: i64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f64,
    pub failure_kind: Option<String>,
    pub detail: Option<String>,
}

/// Input to open a run. `status` starts `running`.
#[derive(Clone, Debug)]
pub struct EvalRunStart<'a> {
    pub suite: &'a str,
    pub agent: &'a str,
    pub harness: &'a str,
    pub model: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
}

/// Input for one case result.
#[derive(Clone, Debug)]
pub struct EvalCaseInput<'a> {
    pub run_id: &'a str,
    pub case_id: &'a str,
    pub category: Option<&'a str>,
    pub status: &'a str,
    pub duration_ms: i64,
    pub turns: i64,
    pub no_action_turns: i64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f64,
    pub failure_kind: Option<&'a str>,
    pub detail: Option<&'a str>,
}

/// Final totals written when a run is closed.
#[derive(Clone, Debug)]
pub struct EvalTotals {
    pub status: &'static str,
    pub total: i64,
    pub passed: i64,
    pub failed: i64,
    pub tokens_total: i64,
    pub cost_usd: f64,
}

impl Journal {
    /// Open a run and return its generated id. `status` starts `running`.
    pub fn eval_run_start(&self, run: &EvalRunStart<'_>) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO eval_runs (id, suite, agent, harness, model, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                id,
                run.suite,
                run.agent,
                run.harness,
                run.model,
                run.metadata_json,
            ],
        )?;
        Ok(id)
    }

    /// Record one case result; returns the case id.
    pub fn eval_run_record_case(&self, case: &EvalCaseInput<'_>) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO eval_cases
                (id, run_id, case_id, category, status, duration_ms, turns,
                 no_action_turns, tokens_in, tokens_out, cost_usd, failure_kind, detail)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                id,
                case.run_id,
                case.case_id,
                case.category,
                case.status,
                case.duration_ms,
                case.turns,
                case.no_action_turns,
                case.tokens_in,
                case.tokens_out,
                case.cost_usd,
                case.failure_kind,
                case.detail,
            ],
        )?;
        Ok(id)
    }

    /// Close a run with its totals; `true` when a row was updated.
    pub fn eval_run_finish(&self, id: &str, totals: &EvalTotals) -> anyhow::Result<bool> {
        let conn = self.conn.lock();
        let n = conn.execute(
            "UPDATE eval_runs
                SET status = ?2, total = ?3, passed = ?4, failed = ?5,
                    tokens_total = ?6, cost_usd = ?7,
                    finished_at = unixepoch() * 1000
              WHERE id = ?1",
            rusqlite::params![
                id,
                totals.status,
                totals.total,
                totals.passed,
                totals.failed,
                totals.tokens_total,
                totals.cost_usd,
            ],
        )?;
        Ok(n == 1)
    }

    /// Fetch one run by id.
    pub fn eval_run_get(&self, id: &str) -> anyhow::Result<Option<EvalRunRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, suite, agent, harness, model, status, total, passed, failed,
                    tokens_total, cost_usd, started_at, finished_at, metadata_json
             FROM eval_runs WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        match rows.next()? {
            Some(r) => Ok(Some(run_row(r)?)),
            None => Ok(None),
        }
    }

    /// List runs, newest first.
    pub fn eval_run_list(&self, limit: i64) -> anyhow::Result<Vec<EvalRunRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, suite, agent, harness, model, status, total, passed, failed,
                    tokens_total, cost_usd, started_at, finished_at, metadata_json
             FROM eval_runs ORDER BY started_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], run_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// All cases of a run, ordered by `case_id`.
    pub fn eval_cases(&self, run_id: &str) -> anyhow::Result<Vec<EvalCaseRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, case_id, category, status, duration_ms, turns,
                    no_action_turns, tokens_in, tokens_out, cost_usd, failure_kind, detail
             FROM eval_cases WHERE run_id = ?1 ORDER BY case_id ASC",
        )?;
        let rows = stmt.query_map([run_id], case_row)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Delete a run and its cases; `true` when the run existed.
    pub fn eval_run_delete(&self, id: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM eval_cases WHERE run_id = ?1", [id])?;
        let n = conn.execute("DELETE FROM eval_runs WHERE id = ?1", [id])?;
        Ok(n == 1)
    }
}

fn run_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<EvalRunRow> {
    Ok(EvalRunRow {
        id: r.get(0)?,
        suite: r.get(1)?,
        agent: r.get(2)?,
        harness: r.get(3)?,
        model: r.get(4)?,
        status: r.get(5)?,
        total: r.get(6)?,
        passed: r.get(7)?,
        failed: r.get(8)?,
        tokens_total: r.get(9)?,
        cost_usd: r.get(10)?,
        started_at: r.get(11)?,
        finished_at: r.get(12)?,
        metadata_json: r.get(13)?,
    })
}

fn case_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<EvalCaseRow> {
    Ok(EvalCaseRow {
        id: r.get(0)?,
        run_id: r.get(1)?,
        case_id: r.get(2)?,
        category: r.get(3)?,
        status: r.get(4)?,
        duration_ms: r.get(5)?,
        turns: r.get(6)?,
        no_action_turns: r.get(7)?,
        tokens_in: r.get(8)?,
        tokens_out: r.get(9)?,
        cost_usd: r.get(10)?,
        failure_kind: r.get(11)?,
        detail: r.get(12)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_journal() -> (tempfile::TempDir, Journal) {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        (tmp, journal)
    }

    fn start<'a>(suite: &'a str, agent: &'a str) -> EvalRunStart<'a> {
        EvalRunStart {
            suite,
            agent,
            harness: "atlas",
            model: Some("qwen3-coder-next"),
            metadata_json: None,
        }
    }

    fn case<'a>(run_id: &'a str, case_id: &'a str, status: &'a str) -> EvalCaseInput<'a> {
        EvalCaseInput {
            run_id,
            case_id,
            category: Some("BUG"),
            status,
            duration_ms: 1_200,
            turns: 7,
            no_action_turns: 1,
            tokens_in: 1_000,
            tokens_out: 250,
            cost_usd: 0.004,
            failure_kind: if status == "pass" {
                None
            } else {
                Some("VERIFY")
            },
            detail: None,
        }
    }

    #[test]
    fn start_get_finish_round_trip() {
        let (_dir, journal) = fresh_journal();
        let id = journal.eval_run_start(&start("golden@1", "atlas")).unwrap();

        let row = journal.eval_run_get(&id).unwrap().unwrap();
        assert_eq!(row.suite, "golden@1");
        assert_eq!(row.status, "running");
        assert_eq!(row.total, 0);
        assert!(row.finished_at.is_none());

        let totals = EvalTotals {
            status: "completed",
            total: 2,
            passed: 1,
            failed: 1,
            tokens_total: 2_500,
            cost_usd: 0.008,
        };
        assert!(journal.eval_run_finish(&id, &totals).unwrap());
        let row = journal.eval_run_get(&id).unwrap().unwrap();
        assert_eq!(row.status, "completed");
        assert_eq!(row.total, 2);
        assert_eq!(row.passed, 1);
        assert!(row.finished_at.is_some());

        assert!(!journal.eval_run_finish("does-not-exist", &totals).unwrap());
    }

    #[test]
    fn record_cases_and_list() {
        let (_dir, journal) = fresh_journal();
        let id = journal.eval_run_start(&start("golden@1", "atlas")).unwrap();
        journal
            .eval_run_record_case(&case(&id, "case-b", "fail"))
            .unwrap();
        journal
            .eval_run_record_case(&case(&id, "case-a", "pass"))
            .unwrap();

        let cases = journal.eval_cases(&id).unwrap();
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].case_id, "case-a", "ordered by case_id");
        assert_eq!(cases[0].failure_kind, None);
        assert_eq!(cases[1].failure_kind.as_deref(), Some("VERIFY"));
        assert_eq!(cases[0].tokens_in, 1_000);

        let runs = journal.eval_run_list(10).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, id);
    }

    #[test]
    fn status_check_rejects_bad_case_status() {
        let (_dir, journal) = fresh_journal();
        let id = journal.eval_run_start(&start("golden@1", "atlas")).unwrap();
        let err = journal
            .eval_run_record_case(&case(&id, "c", "maybe"))
            .unwrap_err();
        assert!(format!("{err:#}").to_lowercase().contains("check"));
    }

    #[test]
    fn delete_removes_run_and_cases() {
        let (_dir, journal) = fresh_journal();
        let id = journal.eval_run_start(&start("golden@1", "atlas")).unwrap();
        journal
            .eval_run_record_case(&case(&id, "case-a", "pass"))
            .unwrap();

        assert!(journal.eval_run_delete(&id).unwrap());
        assert!(journal.eval_run_get(&id).unwrap().is_none());
        assert!(journal.eval_cases(&id).unwrap().is_empty());
        assert!(!journal.eval_run_delete(&id).unwrap(), "already gone");
    }
}
