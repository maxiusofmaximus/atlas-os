use crate::coding::types::{Diff, FileEdit, Hunk};
use crate::journal::Journal;
use crate::repair::types::{
    ErrorClass, RepairAttempt, RepairOutcome, RepairReport, RepairStrategy,
};
use crate::validation::runner::ValidationInput;
use crate::validation::types::*;
use tempfile::TempDir;
use uuid::Uuid;

fn make_diff(lines: Vec<String>) -> Diff {
    Diff {
        diff_id: Uuid::new_v4(),
        plan_id: Uuid::new_v4(),
        mission_id: Uuid::new_v4(),
        step_id: "s1".into(),
        agent_id: Uuid::new_v4(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        files: vec![FileEdit {
            path: "src/lib.rs".into(),
            hunks: vec![Hunk {
                old_start: 1,
                old_end: 1,
                new_lines: lines,
                rationale: "test fixture".into(),
            }],
            is_new_file: false,
            is_delete: false,
        }],
        narrative: "test".into(),
        research_refs: vec![],
        risk_decision: None,
        model_id: "heuristic-v0".into(),
        elapsed_ms: 0,
    }
}

// ────────────────────────── Repair (M6) �─────────────────────────

fn make_repair_report(
    journal: &Journal,
    diff: &Diff,
    outcome: RepairOutcome,
    attempts: Vec<RepairAttempt>,
    successful_attempt: Option<u32>,
) -> RepairReport {
    let input = ValidationInput::new(diff);
    let report = crate::validation::runner::run(&input);
    journal.save_report(&report).expect("save_report");
    RepairReport {
        repair_id: Uuid::new_v4(),
        triggered_by_report_id: report.report_id,
        source_diff_id: diff.diff_id,
        plan_id: diff.plan_id,
        mission_id: diff.mission_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        attempts,
        outcome,
        triggering_stage: report.failed_stage().unwrap_or(StageKind::LintFormat),
        successful_attempt,
        final_diff: None,
        model_id: "heuristic-v0".into(),
        elapsed_ms: 0,
    }
}

fn fixed_attempt(success: bool) -> RepairAttempt {
    RepairAttempt {
        error_class: ErrorClass::SyntaxFormat,
        strategy: RepairStrategy::AutoFix,
        success,
        root_cause: "removed stray println!".into(),
        proposed_diff: None,
    }
}

#[test]
fn save_repair_roundtrip_preserves_payload() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let d = make_diff(vec!["fn x() {}".into()]);
    journal.save_diff(&d).expect("diff");

    let attempt = fixed_attempt(true);
    let report = make_repair_report(&journal, &d, RepairOutcome::Applied, vec![attempt], Some(0));

    journal.save_repair(&report).expect("save_repair");

    let raw = journal
        .repair_payload(report.repair_id)
        .expect("payload")
        .unwrap();
    let back: RepairReport = serde_json::from_str(&raw).unwrap();
    assert_eq!(back.repair_id, report.repair_id);
    assert_eq!(back.outcome, RepairOutcome::Applied);
    assert_eq!(back.triggering_stage, report.triggering_stage);
    assert_eq!(back.attempts.len(), 1);
    assert_eq!(back.successful_attempt, Some(0));
}

#[test]
fn save_repair_idempotent_replay_does_not_overwrite() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let d = make_diff(vec!["fn x() {}".into()]);
    journal.save_diff(&d).expect("diff");
    let report = make_repair_report(&journal, &d, RepairOutcome::Proposed, vec![], None);

    journal.save_repair(&report).expect("first");
    journal.save_repair(&report).expect("replay");

    let tail = journal.repair_tail(10).expect("tail");
    assert_eq!(tail.len(), 1, "idempotent replay (RFC 02 §3.1.2)");
}

#[test]
fn repair_tail_returns_newest_first_and_denormalized_fields() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");

    let d1 = make_diff(vec!["fn a() {}".into()]);
    let d2 = make_diff(vec!["fn b() {}".into()]);
    journal.save_diff(&d1).expect("diff1");
    journal.save_diff(&d2).expect("diff2");

    let r1 = make_repair_report(
        &journal,
        &d1,
        RepairOutcome::Applied,
        vec![fixed_attempt(true)],
        Some(0),
    );
    std::thread::sleep(std::time::Duration::from_millis(10));
    let r2 = make_repair_report(
        &journal,
        &d2,
        RepairOutcome::EscalatedPlanning,
        vec![],
        None,
    );

    journal.save_repair(&r1).expect("save1");
    journal.save_repair(&r2).expect("save2");

    let tail = journal.repair_tail(10).expect("tail");
    assert_eq!(tail.len(), 2);
    assert_eq!(tail[0].repair_id, r2.repair_id);
    assert_eq!(tail[0].outcome, "escalated_planning");
    assert_eq!(tail[1].repair_id, r1.repair_id);
    assert_eq!(tail[1].outcome, "applied");
    assert_eq!(tail[1].successful_attempt, Some(0));
    assert_eq!(tail[0].successful_attempt, None);
}
