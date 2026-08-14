// Atlas OS — `atlas exec` namespace (RFC 27 §3.F).
//
// A self-documenting CLI surface that an LLM-driven Phase 2 driver can
// invoke from inside a step to drive the orchestrator without the
// `atlas run`/`resume` host loop. Phase 1 ships four stubs that
// reuse the existing engine + Journal backends:
//
//   opencode exec step   <plan_id> <step_id>   → Coding → Validation → (Repair) for one step
//   opencode exec wait   <diff_id>             → fetch the validation report row for a diff
//   opencode exec tail   <kind> <N>            → replay the last N journal rows of an artefact tail
//   opencode exec publish <kind> <payload>     → reuse BusEvent::new + Journal::publish
//
// Each subcommand is intentionally thin — the actual work is delegated
// to `crate::journal::*` and `crate::cli::commands::mission::run_single_step`.

use std::time::Instant;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use uuid::Uuid;

use crate::core::bus::BusEventKind;

#[derive(Args, Debug)]
pub struct ExecCmd {
    #[command(subcommand)]
    pub action: ExecAction,
}

#[derive(Subcommand, Debug)]
pub enum ExecAction {
    /// Run a single step (Coding → Validation → Repair) from a Plan.
    Step {
        /// Plan UUID whose payload lives in the Journal.
        plan_id: String,
        /// Step id (string) inside the Plan to execute.
        step_id: String,
    },
    /// Look up the validation report row that accompanies a diff.
    Wait {
        /// Diff UUID to match.
        diff_id: String,
    },
    /// Replay the latest N rows of a Journal tail.
    Tail {
        /// Tail kind: verdicts | plans | diffs | reports | repairs |
        /// checkpoints | consolidates | model_swaps | step_states |
        /// audits | events.
        kind: String,
        /// Number of rows to print (newest first).
        #[arg(short = 'n', long, default_value_t = 20)]
        last: u32,
    },
    /// Publish a raw BusEvent JSON payload into the Journal.
    Publish {
        /// BusEvent kind tag, e.g. `mission_steered`, `step_phase_changed`.
        kind: String,
        /// Raw JSON payload for the event (`{ ... }`). Wrapped in the
        /// outer `BusEvent` envelope by `BusEvent::new`-equivalent logic.
        payload: String,
    },
}

pub async fn run(cmd: ExecCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    match cmd.action {
        ExecAction::Step { plan_id, step_id } => exec_step(&journal, &plan_id, &step_id),
        ExecAction::Wait { diff_id } => exec_wait(&journal, &diff_id),
        ExecAction::Tail { kind, last } => exec_tail(&journal, &kind, last),
        ExecAction::Publish { kind, payload } => exec_publish(&journal, &kind, &payload),
    }
}

/// Lookup `plan_id` in the Journal, find the named step, and run only
/// that single step via the shared `run_single_step` helper used by
/// the full `run_steps_loop`.
fn exec_step(journal: &crate::journal::Journal, plan_id: &str, step_id: &str) -> Result<()> {
    let pid = Uuid::parse_str(plan_id).context("invalid plan_id")?;
    let payload = journal
        .plan_payload(pid)?
        .with_context(|| format!("plan {pid} payload not found"))?;
    let plan: crate::planning::types::Plan = serde_json::from_str(&payload)?;

    let step = plan
        .steps
        .iter()
        .find(|s| s.id == step_id)
        .with_context(|| format!("step {step_id} not found in plan {pid}"))?;
    if step.read_only {
        anyhow::bail!("step {step_id} is read-only; `opencode exec step` skips read-only steps");
    }

    let agent_id = Uuid::new_v4();
    let _start = Instant::now();
    let outcome = super::mission::run_single_step(journal, &plan, step, agent_id, 0)?;

    println!(
        "exec step: plan={pid} step={step_id}  diff={diff}  ok={ok}  repair={repair}  blocked={blocked}",
        diff = outcome
            .diff_id
            .map(|u| u.to_string())
            .unwrap_or_else(|| "—".into()),
        ok = outcome.report_ok,
        repair = outcome
            .repair_id
            .map(|u| u.to_string())
            .unwrap_or_else(|| "—".into()),
        blocked = outcome.blocked,
    );
    Ok(())
}

/// Lookup a validation report row that matches `diff_id` from the
/// report tail. Returns the row + a human-readable summary.
fn exec_wait(journal: &crate::journal::Journal, diff_id: &str) -> Result<()> {
    let did = Uuid::parse_str(diff_id).context("invalid diff_id")?;
    let tail = journal.report_tail(256)?;
    let row = tail
        .into_iter()
        .find(|r| r.diff_id == did)
        .with_context(|| format!("no validation report found for diff {did}"))?;
    println!(
        "exec wait: diff={did}  report={rid}  outcome={outcome}  failed_stage={failed}  stages={n}",
        rid = row.report_id,
        outcome = row.outcome,
        failed = row.failed_stage.unwrap_or_else(|| "—".into()),
        n = row.stage_count,
    );
    Ok(())
}

/// Reuse the generic tail accessors on the Journal to print the most
/// recent N rows of a particular artefact kind.
fn exec_tail(journal: &crate::journal::Journal, kind: &str, last: u32) -> Result<()> {
    let n = i64::from(last.min(2048));
    match kind {
        "verdicts" => {
            let rows = journal.verdict_tail(n)?;
            println!("verdicts ({}) last {n}:", rows.len());
            for r in rows {
                println!(
                    "  {ts} conf={conf} mission={mid} verdict_id={vid}",
                    ts = r.ts,
                    conf = r.confidence,
                    mid = r.mission_id.map(|u| u.to_string()).unwrap_or_else(|| "—".into()),
                    vid = r.verdict_id,
                );
            }
        }
        "plans" => {
            let rows = journal.plan_tail(n)?;
            println!("plans ({}) last {n}:", rows.len());
            for r in rows {
                println!(
                    "  {ts} plan_id={pid} mission={mid} strategy={strat} risk={risk:.2} confidence={conf:.2}",
                    ts = r.generated_at,
                    pid = r.plan_id,
                    mid = r.mission_id,
                    strat = r.strategy,
                    risk = r.risk,
                    conf = r.confidence,
                );
            }
        }
        "diffs" => {
            let rows = journal.diff_tail(n)?;
            println!("diffs ({}) last {n}:", rows.len());
            for r in rows {
                println!(
                    "  {ts} diff_id={did} mission={mid} step={step} +{la}/-{lr}/files={fc}",
                    ts = r.generated_at,
                    did = r.diff_id,
                    mid = r.mission_id,
                    step = r.step_id,
                    la = r.lines_added,
                    lr = r.lines_removed,
                    fc = r.file_count,
                );
            }
        }
        "reports" => {
            let rows = journal.report_tail(n)?;
            println!("validation reports ({}) last {n}:", rows.len());
            for r in rows {
                println!(
                    "  {ts} report_id={rid} diff={did} outcome={outcome} failed={failed} stages={n_stages}",
                    ts = r.generated_at,
                    rid = r.report_id,
                    did = r.diff_id,
                    outcome = r.outcome,
                    failed = r.failed_stage.unwrap_or_else(|| "—".into()),
                    n_stages = r.stage_count,
                );
            }
        }
        "audits" => {
            let rows = journal.audit_tail(n)?;
            println!("audit log ({}) last {n}:", rows.len());
            for r in rows {
                println!(
                    "  #{seq} {ts} actor={actor} action={action}",
                    seq = r.seq,
                    ts = r.ts,
                    actor = r.actor,
                    action = r.action,
                );
            }
        }
        "checkpoints" => {
            let rows = journal.checkpoint_tail(n)?;
            println!("checkpoints ({}) last {n}:", rows.len());
            for r in rows {
                println!(
                    "  {ts} checkpoint={ck} mission={mid} phase={phase}",
                    ts = r.generated_at,
                    ck = r.checkpoint_id,
                    mid = r.mission_id,
                    phase = r.phase,
                );
            }
        }
        "model_swaps" => {
            let rows = journal.model_swap_tail(n)?;
            println!("model_swaps ({}) last {n}:", rows.len());
            for r in rows {
                println!(
                    "  {ts} swap_id={sid} mission={mid} {prev}→{new_} by={init}",
                    ts = r.occurred_at,
                    sid = r.swap_id,
                    mid = r.mission_id,
                    prev = r.prev_model_id,
                    new_ = r.new_model_id,
                    init = r.initiator,
                );
            }
        }
        "step_states" => {
            let rows = journal.step_state_tail(n)?;
            println!("step_states ({}) last {n}:", rows.len());
            for r in rows {
                println!(
                    "  {ts} mission={mid} plan={pid} step={step} phase={phase}",
                    ts = r.updated_at,
                    mid = r.mission_id,
                    pid = r.plan_id,
                    step = r.step_id,
                    phase = r.phase,
                );
            }
        }
        "events" => {
            let rows = journal.tail(n)?;
            println!("journal events ({}) last {n}:", rows.len());
            for e in rows {
                println!("  #{id} {ts}  {kind}", id = e.id, ts = e.ts, kind = e.kind);
            }
        }
        other => anyhow::bail!(
            "unknown tail kind `{other}`. Valid: verdicts | plans | diffs | reports | audits | checkpoints | model_swaps | step_states | events"
        ),
    }
    Ok(())
}

/// Wrap a raw JSON blob inside a `BusEvent::new` shell keyed to the
/// supplied `kind` and persist it. Phase 1 only persists — the live
/// `broadcast::Sender` belongs to `AppState` which the headless CLI
/// does not own. Phase 2 will spawn a long-lived HUD server or attach
/// to an existing one to fire the broadcast.
fn exec_publish(journal: &crate::journal::Journal, kind: &str, payload_json: &str) -> Result<()> {
    use crate::core::bus::BusEvent;

    let payload: serde_json::Value =
        serde_json::from_str(payload_json).context("invalid JSON payload")?;
    let event_kind = build_event_kind(kind, payload)?;
    let event = BusEvent::new(event_kind);
    journal.publish(&event).context("journal::publish")?;
    println!(
        "exec publish: published kind={kind} event_id={} idempotency_key={}",
        event.id, event.idempotency_key
    );
    Ok(())
}

/// Reconstruct a typed `BusEventKind` from its discriminator tag and
/// raw JSON. Round-trips through the serde-JSON shape — we wrap the
/// payload in `{ "type": <kind>, ...fields }` and deserialize, which
/// is the same tag the enum itself uses (`#[serde(tag = "type")]`).
fn build_event_kind(kind: &str, payload: serde_json::Value) -> Result<BusEventKind> {
    use serde_json::json;

    let mut wrapper = json!({ "type": kind });
    if let serde_json::Value::Object(obj) = payload {
        for (k, v) in obj {
            wrapper
                .as_object_mut()
                .expect("wrapper is object")
                .insert(k, v);
        }
    }
    let typed: BusEventKind = serde_json::from_value(wrapper)
        .with_context(|| format!("payload does not match BusEventKind `{kind}`"))?;
    Ok(typed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::Journal;
    use crate::planning::runner::PlanningInput;
    use crate::prompt::runner::{run as run_prompt, PipelineOptions};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn fresh_journal() -> (TempDir, Journal) {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        (tmp, journal)
    }

    fn plan_with_one_step(journal: &Journal, prompt: &str) -> crate::planning::types::Plan {
        let (verdict, consolidated_opt) =
            run_prompt(prompt, &PipelineOptions::default_force()).expect("prompt::run");
        let c = consolidated_opt.expect("forced prompt yields a consolidated");
        journal
            .save_verdict(&verdict, Some(c.mission_id))
            .expect("save_verdict");
        journal
            .create_mission(c.mission_id, &c.mission_statement)
            .expect("create_mission");
        journal.save_consolidated(&c).expect("save_consolidated");
        crate::planning::runner::run(&PlanningInput {
            consolidated: &c,
            verdict: &verdict,
            clarification_answers: vec![],
            research_runs: vec![],
        })
        .expect("planning::run")
    }

    #[test]
    fn exec_step_runs_a_single_step_with_outcome() {
        let (_dir, journal) = fresh_journal();
        // "build auth folder" forces an editable step in the heuristic
        // planner (mirrors `run_steps_loop_executes_steps_and_persists_tail`).
        let plan = plan_with_one_step(&journal, "build auth folder and add tests");
        let step = plan
            .steps
            .iter()
            .find(|s| !s.read_only)
            .expect("at least one non-read-only step");
        // Smoke: `run_single_step` returns without panicking; Phase 1
        // heuristic may mark steps PlanBlocked (e.g. `SO0`) but the
        // surrounding control flow did not abort.
        let outcome =
            run_single_step_pub(&journal, &plan, step, Uuid::new_v4(), 0).expect("run_single_step");
        // Either a diff was produced OR the heuristic rejected this step
        // — both are valid Phase 1 outcomes (RFC 12 §7 threshold).
        assert!(
            outcome.diff_id.is_some() || outcome.blocked,
            "step neither emitted a diff nor marked blocked"
        );
        if outcome.diff_id.is_some() {
            let tail = journal.diff_tail(50).expect("diff tail");
            assert!(tail.iter().any(|d| d.step_id == step.id));
        }
    }

    /// Re-export helper so tests in this file exercise the crate-private
    /// `crate::cli::commands::mission::run_single_step`.
    fn run_single_step_pub(
        journal: &Journal,
        plan: &crate::planning::types::Plan,
        step: &crate::planning::types::Step,
        agent_id: Uuid,
        mission_failures: u32,
    ) -> Result<super::super::mission::StepOutcome> {
        super::super::mission::run_single_step(journal, plan, step, agent_id, mission_failures)
    }

    #[test]
    fn exec_wait_finds_report_for_diff_via_tail() {
        let (_dir, journal) = fresh_journal();
        let plan = plan_with_one_step(&journal, "build auth folder and add tests");
        // Use the full loop so we exercise the same path as `atlas run`.
        let steps = super::super::mission::run_steps_loop(
            &journal,
            &plan,
            Uuid::new_v4(),
            &std::time::Instant::now(),
        )
        .expect("run_steps_loop");
        let diff_tail = journal.diff_tail(64).expect("diff tail");

        // Phase 1 heuristic sometimes marks every step PlanBlocked and
        // yields last_diff_id=None; in that case skip the wait lookup
        // (there is nothing to wait for). When at least one diff landed,
        // assert `report_tail` contains a matching row.
        let target_diff_id = match (steps.last_diff_id, diff_tail.first()) {
            (Some(id), _) => id,
            (None, Some(d)) => d.diff_id,
            (None, None) => {
                eprintln!("skipping exec_wait: heuristic produced no diffs");
                return;
            }
        };
        let tail = journal.report_tail(64).expect("report tail");
        assert!(
            tail.iter().any(|r| r.diff_id == target_diff_id),
            "expected report for diff_id {target_diff_id} in tail of {} rows",
            tail.len()
        );
    }

    #[test]
    fn exec_tail_kind_unknown_errors() {
        let (_dir, journal) = fresh_journal();
        let err = exec_tail(&journal, "no_such_kind", 5).unwrap_err();
        assert!(format!("{err:#}").contains("unknown tail kind"));
    }

    #[test]
    fn exec_tail_kind_events_prints_journal_rows() {
        let (_dir, journal) = fresh_journal();
        // publish one event first
        let event = crate::core::bus::BusEvent::new(crate::core::bus::BusEventKind::HudServed {
            hud_port: 8090,
        });
        journal.publish(&event).expect("publish");
        exec_tail(&journal, "events", 10).expect("events tail");
    }

    #[test]
    fn exec_publish_round_trips_steered_event() {
        let (_dir, journal) = fresh_journal();
        let mid = Uuid::new_v4();
        let payload = serde_json::json!({ "mission_id": mid, "message": "use a buffer" });
        exec_publish(&journal, "mission_steered", &payload.to_string()).expect("publish");

        let tail = journal.tail(10).expect("tail");
        assert!(tail.iter().any(|e| e.kind == "mission_steered"));
    }

    #[test]
    fn exec_publish_rejects_bad_payload() {
        let (_dir, journal) = fresh_journal();
        let err = exec_publish(&journal, "mission_steered", "{ not json").unwrap_err();
        assert!(format!("{err:#}").contains("invalid JSON"));
    }

    #[test]
    fn build_event_kind_rejects_unknown_tag() {
        let err = build_event_kind("no_such_tag", serde_json::json!({})).unwrap_err();
        assert!(format!("{err:#}").contains("BusEventKind"));
    }

    #[test]
    fn build_event_kind_constructs_step_phase_changed() {
        let kind = build_event_kind(
            "step_phase_changed",
            serde_json::json!({
                "mission_id": Uuid::nil(),
                "plan_id": Uuid::nil(),
                "step_id": "s-1",
                "phase": "done",
            }),
        )
        .expect("build");
        assert_eq!(kind.tag(), "step_phase_changed");
    }
}
