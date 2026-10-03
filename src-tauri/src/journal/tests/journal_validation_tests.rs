use crate::coding::types::*;
use crate::journal::Journal;
use crate::validation::runner::ValidationInput;
use crate::validation::types::*;
use tempfile::TempDir;
use uuid::Uuid;

fn make_diff(lines: Vec<String>) -> Diff {
    Diff {
        diff_id: Uuid::new_v4(),
        plan_id: Uuid::new_v4(),
        mission_id: Uuid::new_v4(),
        step_id: "S1".into(),
        agent_id: Uuid::new_v4(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        files: vec![FileEdit {
            path: "src/lib.rs".into(),
            is_new_file: false,
            is_delete: false,
            hunks: vec![Hunk {
                old_start: 0,
                old_end: 0,
                new_lines: lines,
                rationale: "r".into(),
            }],
        }],
        narrative: "test fixture: pure helper with unit coverage".into(),
        research_refs: vec![Uuid::new_v4()],
        risk_decision: None,
        model_id: "heuristic-v0".into(),
        elapsed_ms: 0,
    }
}

#[test]
fn save_report_then_tail_and_payload_round_trip() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let d = make_diff(vec!["fn add(a: u32, b: u32) -> u32 { a + b }".into()]);
    // Persist the diff first — `validation_reports.diff_id` is a FK
    // into `diffs` (foreign_keys=ON).
    journal.save_diff(&d).expect("save diff");

    let report = crate::validation::runner::run(&ValidationInput::new(&d));
    journal.save_report(&report).expect("save report");

    let tail = journal.report_tail(10).expect("tail");
    assert_eq!(tail.len(), 1);
    assert_eq!(tail[0].report_id, report.report_id);
    assert_eq!(tail[0].outcome, "pass");
    assert!(tail[0].failed_stage.is_none());
    assert_eq!(tail[0].mode, "strict");
    assert_eq!(
        tail[0].stage_count,
        StageKind::pipeline_order().len() as i64
    );

    let raw = journal
        .report_payload(report.report_id)
        .expect("payload")
        .unwrap();
    let back: ValidationReport = serde_json::from_str(&raw).unwrap();
    assert_eq!(back.report_id, report.report_id);
    assert_eq!(back.outcome, ValidationOutcome::Pass);
    assert_eq!(back.stages.len(), StageKind::pipeline_order().len());
}

#[test]
fn save_report_idempotent_replay_does_not_overwrite() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let d = make_diff(vec!["fn x() {}".into()]);
    journal.save_diff(&d).expect("diff");
    let report = crate::validation::runner::run(&ValidationInput::new(&d));
    journal.save_report(&report).expect("first");
    // Replay — must NOT overwrite.
    journal.save_report(&report).expect("replay");
    let tail = journal.report_tail(10).expect("tail");
    assert_eq!(tail.len(), 1, "idempotent replay (RFC 02 §3.1.2)");
}

#[test]
fn save_failed_report_records_failed_stage_tag() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    // Unbalanced braces → TypeCheck fails.
    let d = make_diff(vec!["fn x() {".into()]);
    journal.save_diff(&d).expect("diff");
    let report = crate::validation::runner::run(&ValidationInput::new(&d));
    assert_eq!(report.outcome, ValidationOutcome::Fail);
    journal.save_report(&report).expect("save");

    let row = journal.report_tail(10).expect("tail")[0].clone();
    assert_eq!(row.outcome, "fail");
    assert_eq!(row.failed_stage, Some("type_check".into()));
}
