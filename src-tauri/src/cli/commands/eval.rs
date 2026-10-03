// Atlas OS — `atlas eval` namespace (RFC 20 Fase 22 EVAL, EVAL.1).
//
//   atlas eval run [suite]   → execute a suite, persist to eval_runs/eval_cases
//   atlas eval list          → recent runs
//   atlas eval report [id]   → per-case detail + failure-kind breakdown
//
// Suites are deterministic and offline today (`golden`); LLM-driven and
// external Harbor suites land in EVAL.2/EVAL.3.

use std::collections::BTreeMap;

use anyhow::{bail, Result};
use clap::{Args, Subcommand};

use crate::journal::Journal;

#[derive(Args, Debug)]
pub struct EvalCmd {
    #[command(subcommand)]
    pub action: EvalAction,
}

#[derive(Subcommand, Debug)]
pub enum EvalAction {
    /// Run a suite and persist its results.
    Run {
        /// Suite to run (available: `golden`).
        #[arg(default_value = "golden")]
        suite: String,
    },
    /// List recent eval runs, newest first.
    List {
        #[arg(short = 'n', long, default_value_t = 10)]
        last: i64,
    },
    /// Report a run's cases (latest run when `id` is omitted).
    Report { id: Option<String> },
    /// Import a Harbor job result (`result.json` file, single trial, or job dir).
    Import { path: String },
    /// Show normalized metrics over recent runs.
    Metrics {
        #[arg(short = 'n', long, default_value_t = 20)]
        last: i64,
    },
}

pub async fn run(cmd: EvalCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = Journal::open(&root)?;
    match cmd.action {
        EvalAction::Run { suite } => run_suite(&journal, &suite),
        EvalAction::List { last } => list(&journal, last),
        EvalAction::Report { id } => report(&journal, id.as_deref()),
        EvalAction::Import { path } => import(&journal, &path),
        EvalAction::Metrics { last } => metrics(&journal, last),
    }
}

fn metrics(journal: &Journal, last: i64) -> Result<()> {
    use crate::eval::metrics;

    let limit = last.clamp(1, 512);
    let summary = metrics::summarize_recent(journal, limit)?;
    if summary.total == 0 {
        println!("no eval cases yet (use `atlas eval run golden`)");
        return Ok(());
    }
    println!(
        "eval metrics (last {limit} runs): pass_rate={:.1}% ({}/{})  tokens/solved={:.0}  $/solved={:.4}",
        summary.pass_rate * 100.0,
        summary.passed,
        summary.total,
        summary.tokens_per_solved,
        summary.cost_per_solved,
    );
    println!(
        "  tokens={} no_action_turns={} cost=${:.4}",
        summary.tokens_total, summary.no_action_turns, summary.cost_usd
    );
    if !summary.failure_kinds.is_empty() {
        println!("  failure kinds:");
        for (kind, n) in &summary.failure_kinds {
            println!("    {kind}: {n}");
        }
    }
    let groups = metrics::summarize_by_harness_model(journal, limit)?;
    if groups.len() > 1 {
        println!("  by harness/model:");
        for g in groups {
            println!(
                "    {key}: {rate:.0}% tokens/solved={tps:.0} $/solved={cps:.4}",
                key = g.key,
                rate = g.summary.pass_rate * 100.0,
                tps = g.summary.tokens_per_solved,
                cps = g.summary.cost_per_solved,
            );
        }
    }
    Ok(())
}

fn import(journal: &Journal, path: &str) -> Result<()> {
    let report = crate::eval::harbor::import_job(journal, std::path::Path::new(path))?;
    println!(
        "imported Harbor job → run {}: {}/{} passed ({} failed, {} errored)",
        report.run_id, report.passed, report.total, report.failed, report.errored,
    );
    Ok(())
}

fn run_suite(journal: &Journal, name: &str) -> Result<()> {
    let tasks = match name {
        "golden" => crate::eval::golden::golden_suite(),
        other => bail!("unknown suite `{other}` (available: golden)"),
    };
    let report = crate::eval::run_suite(journal, name, &tasks)?;
    println!(
        "eval run {} [{}]: {}/{} passed ({} failed, {} errored) in {} ms",
        report.run_id,
        name,
        report.passed,
        report.total,
        report.failed,
        report.errored,
        report.duration_ms,
    );
    for case in journal.eval_cases(&report.run_id)? {
        println!(
            "  {status:<5} {category:<5} {id:<32} {detail}",
            status = case.status,
            category = case.category.unwrap_or_default(),
            id = case.case_id,
            detail = case.detail.unwrap_or_default(),
        );
    }
    Ok(())
}

fn list(journal: &Journal, last: i64) -> Result<()> {
    let runs = journal.eval_run_list(last.clamp(1, 512))?;
    if runs.is_empty() {
        println!("no eval runs (use `atlas eval run golden`)");
        return Ok(());
    }
    println!("eval runs ({}):", runs.len());
    for r in runs {
        println!(
            "  {id} {suite:<12} {status:<10} {passed}/{total} passed  {model}",
            id = &r.id[..r.id.len().min(8)],
            suite = r.suite,
            status = r.status,
            passed = r.passed,
            total = r.total,
            model = r.model.unwrap_or_else(|| "-".into()),
        );
    }
    Ok(())
}

fn report(journal: &Journal, id: Option<&str>) -> Result<()> {
    let run = match id {
        Some(id) => journal
            .eval_run_get(id)?
            .ok_or_else(|| anyhow::anyhow!("run `{id}` not found"))?,
        None => journal
            .eval_run_list(1)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("no eval runs yet"))?,
    };

    println!(
        "run {} [{}] {} agent={} harness={} model={}",
        run.id,
        run.suite,
        run.status,
        run.agent,
        run.harness,
        run.model.unwrap_or_else(|| "-".into()),
    );
    println!(
        "  total={} passed={} failed={} tokens={} cost=${:.4}",
        run.total, run.passed, run.failed, run.tokens_total, run.cost_usd
    );

    let cases = journal.eval_cases(&run.id)?;
    let mut failures: BTreeMap<String, i64> = BTreeMap::new();
    for c in &cases {
        if c.status != "pass" {
            *failures
                .entry(c.failure_kind.clone().unwrap_or_else(|| "UNKNOWN".into()))
                .or_default() += 1;
        }
    }
    for c in &cases {
        println!(
            "  {status:<5} {category:<5} {id:<32} {detail}",
            status = c.status,
            category = c.category.clone().unwrap_or_default(),
            id = c.case_id,
            detail = c.detail.clone().unwrap_or_default(),
        );
    }
    if !failures.is_empty() {
        println!("failure kinds:");
        for (kind, n) in failures {
            println!("  {kind}: {n}");
        }
    }
    Ok(())
}
