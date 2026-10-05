// Atlas OS — Agent capability persistence (RFC 63 §6, M49/M50; schema v38/v39).
//
// The capability layer's spine: one `agent_runs` row per agent invocation, one
// `agent_steps` row per loop turn, one `tool_invocations` row per tool call, and
// one `artifacts` row per verified artifact. Raw fields only — the Journal does
// not depend on `orchestrator` types; the host converts. All writers are
// idempotent (INSERT OR IGNORE on a caller-supplied id).

use serde::{Deserialize, Serialize};

use super::Journal;

/// A single agent run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentRunRow {
    pub id: String,
    pub mission_id: Option<String>,
    pub goal: String,
    pub success_predicate: String,
    pub sandbox: String,
    pub status: String,
    pub steps: i64,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f64,
    pub ts_started: i64,
    pub ts_ended: Option<i64>,
}

/// One loop turn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentStepRow {
    pub id: String,
    pub run_id: String,
    pub step: i64,
    pub thought: Option<String>,
    pub action: Option<String>,
    pub observation: Option<String>,
    pub evidence_json: Option<String>,
    pub verdict: Option<String>,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f64,
    pub ts: i64,
}

/// One tool call inside a step.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolInvocationRow {
    pub id: String,
    pub run_id: String,
    pub step: i64,
    pub tool: String,
    pub args_json: String,
    pub result_json: Option<String>,
    pub exit_code: Option<i64>,
    pub duration_ms: Option<i64>,
    pub tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub ts: i64,
}

/// One artifact produced/verified by the run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRow {
    pub id: String,
    pub run_id: String,
    pub kind: String,
    pub path: Option<String>,
    pub sha256: Option<String>,
    pub verified: bool,
    pub evidence_json: Option<String>,
}

impl Journal {
    // ---- agent_runs (M49 / v38) -------------------------------------------

    pub fn create_agent_run(&self, run: &AgentRunRow) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR IGNORE INTO agent_runs
                (id, mission_id, goal, success_predicate, sandbox, status, steps,
                 tokens_in, tokens_out, cost_usd, ts_started, ts_ended)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                run.id,
                run.mission_id,
                run.goal,
                run.success_predicate,
                run.sandbox,
                run.status,
                run.steps,
                run.tokens_in,
                run.tokens_out,
                run.cost_usd,
                run.ts_started,
                run.ts_ended,
            ],
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish_agent_run(
        &self,
        id: &str,
        status: &str,
        steps: i64,
        tokens_in: i64,
        tokens_out: i64,
        cost_usd: f64,
        ts_ended: i64,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE agent_runs
             SET status = ?2, steps = ?3, tokens_in = ?4, tokens_out = ?5,
                 cost_usd = ?6, ts_ended = ?7
             WHERE id = ?1",
            rusqlite::params![id, status, steps, tokens_in, tokens_out, cost_usd, ts_ended],
        )?;
        Ok(())
    }

    pub fn get_agent_run(&self, id: &str) -> anyhow::Result<Option<AgentRunRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, mission_id, goal, success_predicate, sandbox, status, steps,
                    tokens_in, tokens_out, cost_usd, ts_started, ts_ended
             FROM agent_runs WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        match rows.next()? {
            Some(r) => Ok(Some(AgentRunRow {
                id: r.get(0)?,
                mission_id: r.get(1)?,
                goal: r.get(2)?,
                success_predicate: r.get(3)?,
                sandbox: r.get(4)?,
                status: r.get(5)?,
                steps: r.get(6)?,
                tokens_in: r.get(7)?,
                tokens_out: r.get(8)?,
                cost_usd: r.get(9)?,
                ts_started: r.get(10)?,
                ts_ended: r.get(11)?,
            })),
            None => Ok(None),
        }
    }

    pub fn list_agent_runs(&self, limit: i64) -> anyhow::Result<Vec<AgentRunRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, mission_id, goal, success_predicate, sandbox, status, steps,
                    tokens_in, tokens_out, cost_usd, ts_started, ts_ended
             FROM agent_runs ORDER BY ts_started DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |r| {
            Ok(AgentRunRow {
                id: r.get(0)?,
                mission_id: r.get(1)?,
                goal: r.get(2)?,
                success_predicate: r.get(3)?,
                sandbox: r.get(4)?,
                status: r.get(5)?,
                steps: r.get(6)?,
                tokens_in: r.get(7)?,
                tokens_out: r.get(8)?,
                cost_usd: r.get(9)?,
                ts_started: r.get(10)?,
                ts_ended: r.get(11)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// RFC 65 §3 — `(status, count)` buckets over `agent_runs` for the Health
    /// KPIs card (running / done / failed / other). Read-only.
    pub fn agent_run_status_counts(&self) -> anyhow::Result<Vec<(String, i64)>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT status, COUNT(*) AS n
             FROM agent_runs
             GROUP BY status
             ORDER BY n DESC, status ASC",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // ---- agent_steps (M49 / v38) ------------------------------------------
    pub fn record_agent_step(&self, s: &AgentStepRow) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR IGNORE INTO agent_steps
                (id, run_id, step, thought, action, observation, evidence_json, verdict,
                 tokens_in, tokens_out, cost_usd, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                s.id,
                s.run_id,
                s.step,
                s.thought,
                s.action,
                s.observation,
                s.evidence_json,
                s.verdict,
                s.tokens_in,
                s.tokens_out,
                s.cost_usd,
                s.ts,
            ],
        )?;
        Ok(())
    }

    pub fn agent_steps_for_run(&self, run_id: &str) -> anyhow::Result<Vec<AgentStepRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, step, thought, action, observation, evidence_json, verdict,
                    tokens_in, tokens_out, cost_usd, ts
             FROM agent_steps WHERE run_id = ?1 ORDER BY step ASC",
        )?;
        let rows = stmt.query_map([run_id], Self::map_agent_step)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    fn map_agent_step(r: &rusqlite::Row<'_>) -> rusqlite::Result<AgentStepRow> {
        Ok(AgentStepRow {
            id: r.get(0)?,
            run_id: r.get(1)?,
            step: r.get(2)?,
            thought: r.get(3)?,
            action: r.get(4)?,
            observation: r.get(5)?,
            evidence_json: r.get(6)?,
            verdict: r.get(7)?,
            tokens_in: r.get(8)?,
            tokens_out: r.get(9)?,
            cost_usd: r.get(10)?,
            ts: r.get(11)?,
        })
    }

    /// Newest-N steps across ALL runs (RFC 63 §9 / HUD `/tail/agent_steps`).
    pub fn agent_step_tail(&self, last: i64) -> anyhow::Result<Vec<AgentStepRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, step, thought, action, observation, evidence_json, verdict,
                    tokens_in, tokens_out, cost_usd, ts
             FROM agent_steps ORDER BY ts DESC, step DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([last], Self::map_agent_step)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // ---- tool_invocations (M50 / v39) -------------------------------------

    pub fn record_tool_invocation(&self, t: &ToolInvocationRow) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR IGNORE INTO tool_invocations
                (id, run_id, step, tool, args_json, result_json, exit_code,
                 duration_ms, tokens, cost_usd, ts)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                t.id,
                t.run_id,
                t.step,
                t.tool,
                t.args_json,
                t.result_json,
                t.exit_code,
                t.duration_ms,
                t.tokens,
                t.cost_usd,
                t.ts,
            ],
        )?;
        Ok(())
    }

    pub fn tool_invocations_for_run(&self, run_id: &str) -> anyhow::Result<Vec<ToolInvocationRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, step, tool, args_json, result_json, exit_code,
                    duration_ms, tokens, cost_usd, ts
             FROM tool_invocations WHERE run_id = ?1 ORDER BY step ASC, ts ASC",
        )?;
        let rows = stmt.query_map([run_id], |r| {
            Ok(ToolInvocationRow {
                id: r.get(0)?,
                run_id: r.get(1)?,
                step: r.get(2)?,
                tool: r.get(3)?,
                args_json: r.get(4)?,
                result_json: r.get(5)?,
                exit_code: r.get(6)?,
                duration_ms: r.get(7)?,
                tokens: r.get(8)?,
                cost_usd: r.get(9)?,
                ts: r.get(10)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // ---- artifacts (M50 / v39) --------------------------------------------

    pub fn record_artifact(&self, a: &ArtifactRow) -> anyhow::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR IGNORE INTO artifacts
                (id, run_id, kind, path, sha256, verified, evidence_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                a.id,
                a.run_id,
                a.kind,
                a.path,
                a.sha256,
                i64::from(a.verified),
                a.evidence_json,
            ],
        )?;
        Ok(())
    }

    pub fn artifacts_for_run(&self, run_id: &str) -> anyhow::Result<Vec<ArtifactRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, run_id, kind, path, sha256, verified, evidence_json
             FROM artifacts WHERE run_id = ?1 ORDER BY kind, path",
        )?;
        let rows = stmt.query_map([run_id], |r| {
            Ok(ArtifactRow {
                id: r.get(0)?,
                run_id: r.get(1)?,
                kind: r.get(2)?,
                path: r.get(3)?,
                sha256: r.get(4)?,
                verified: r.get::<_, i64>(5)? != 0,
                evidence_json: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open() -> Journal {
        let tmp = TempDir::new().expect("tmp");
        Journal::open(tmp.path()).expect("open")
    }

    fn run(id: &str) -> AgentRunRow {
        AgentRunRow {
            id: id.into(),
            mission_id: None,
            goal: "make out.txt".into(),
            success_predicate: "[]".into(),
            sandbox: "local".into(),
            status: "running".into(),
            steps: 0,
            tokens_in: 0,
            tokens_out: 0,
            cost_usd: 0.0,
            ts_started: 1,
            ts_ended: None,
        }
    }

    #[test]
    fn run_round_trips_and_finishes() {
        let j = open();
        j.create_agent_run(&run("r1")).unwrap();
        assert_eq!(j.get_agent_run("r1").unwrap().unwrap().status, "running");
        j.finish_agent_run("r1", "done", 3, 100, 50, 0.02, 9)
            .unwrap();
        let r = j.get_agent_run("r1").unwrap().unwrap();
        assert_eq!(r.status, "done");
        assert_eq!(r.steps, 3);
        assert_eq!(r.tokens_in, 100);
        assert_eq!(r.ts_ended, Some(9));
    }

    #[test]
    fn steps_are_ordered() {
        let j = open();
        j.create_agent_run(&run("r1")).unwrap();
        for s in [2, 0, 1] {
            j.record_agent_step(&AgentStepRow {
                id: format!("s{s}"),
                run_id: "r1".into(),
                step: s,
                thought: None,
                action: Some(format!("cmd{s}")),
                observation: None,
                evidence_json: None,
                verdict: Some("pass".into()),
                tokens_in: 0,
                tokens_out: 0,
                cost_usd: 0.0,
                ts: 10 + s,
            })
            .unwrap();
        }
        let steps = j.agent_steps_for_run("r1").unwrap();
        assert_eq!(
            steps.iter().map(|s| s.step).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn tool_invocations_and_artifacts_round_trip() {
        let j = open();
        j.create_agent_run(&run("r1")).unwrap();
        j.record_tool_invocation(&ToolInvocationRow {
            id: "t1".into(),
            run_id: "r1".into(),
            step: 0,
            tool: "exec.run".into(),
            args_json: "{\"command\":\"ls\"}".into(),
            result_json: Some("{\"ok\":true}".into()),
            exit_code: Some(0),
            duration_ms: Some(12),
            tokens: None,
            cost_usd: None,
            ts: 5,
        })
        .unwrap();
        j.record_artifact(&ArtifactRow {
            id: "a1".into(),
            run_id: "r1".into(),
            kind: "file".into(),
            path: Some("out.txt".into()),
            sha256: Some("abc".into()),
            verified: true,
            evidence_json: None,
        })
        .unwrap();
        assert_eq!(j.tool_invocations_for_run("r1").unwrap().len(), 1);
        let arts = j.artifacts_for_run("r1").unwrap();
        assert_eq!(arts.len(), 1);
        assert!(arts[0].verified);
    }

    #[test]
    fn list_agent_runs_returns_newest_first() {
        let j = open();
        for (id, ts) in [("a", 1), ("b", 3), ("c", 2)] {
            let mut r = run(id);
            r.ts_started = ts;
            j.create_agent_run(&r).unwrap();
        }
        let ids: Vec<String> = j
            .list_agent_runs(10)
            .unwrap()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(ids, vec!["b", "c", "a"]);
    }

    #[test]
    fn run_status_counts_bucket_by_status() {
        let j = open();
        for (id, status) in [("a", "done"), ("b", "done"), ("c", "failed")] {
            let mut r = run(id);
            r.status = status.into();
            j.create_agent_run(&r).unwrap();
        }
        let counts = j.agent_run_status_counts().unwrap();
        assert_eq!(counts[0], ("done".to_string(), 2));
        assert!(counts.iter().any(|(s, n)| s == "failed" && *n == 1));
    }

    #[test]
    fn agent_step_tail_spans_runs_newest_first() {
        let j = open();
        j.create_agent_run(&run("r1")).unwrap();
        j.create_agent_run(&run("r2")).unwrap();
        for (id, run_id, step, ts) in [("a", "r1", 0, 1), ("b", "r2", 0, 5), ("c", "r2", 1, 6)] {
            j.record_agent_step(&AgentStepRow {
                id: id.into(),
                run_id: run_id.into(),
                step,
                thought: None,
                action: Some("run_command".into()),
                observation: None,
                evidence_json: None,
                verdict: None,
                tokens_in: 0,
                tokens_out: 0,
                cost_usd: 0.0,
                ts,
            })
            .unwrap();
        }
        let tail = j.agent_step_tail(10).unwrap();
        // newest ts first (c=6), and it spans both runs.
        assert_eq!(tail[0].id, "c");
        assert_eq!(tail.iter().filter(|s| s.run_id == "r2").count(), 2);
    }
}
