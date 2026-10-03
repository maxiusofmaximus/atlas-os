use crate::coding::types::{Diff, FileEdit, Hunk};
use crate::journal::Journal;
use crate::learning::runner as learning_runner;
use crate::learning::{LearnInput, LearnOutcome, RuleLifecycle};
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
                rationale: "fixture".into(),
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

fn applied_repair_report(journal: &Journal, diff: &Diff) -> RepairReport {
    let input = ValidationInput::new(diff);
    let report = crate::validation::runner::run(&input);
    journal.save_report(&report).expect("save_report");

    let attempt = RepairAttempt {
        error_class: ErrorClass::SyntaxFormat,
        strategy: RepairStrategy::AutoFix,
        success: true,
        root_cause: "lint rule `no_println` fired".into(),
        proposed_diff: Some(Box::new(diff.clone())),
    };
    RepairReport {
        repair_id: Uuid::new_v4(),
        triggered_by_report_id: report.report_id,
        source_diff_id: diff.diff_id,
        plan_id: diff.plan_id,
        mission_id: diff.mission_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        attempts: vec![attempt],
        outcome: RepairOutcome::Applied,
        triggering_stage: StageKind::LintFormat,
        successful_attempt: Some(0),
        final_diff: None,
        model_id: "heuristic-v0".into(),
        elapsed_ms: 0,
    }
}

fn no_pattern_repair_report(journal: &Journal, diff: &Diff) -> RepairReport {
    let report = crate::validation::runner::run(&ValidationInput::new(diff));
    journal.save_report(&report).expect("save_report");
    RepairReport {
        repair_id: Uuid::new_v4(),
        triggered_by_report_id: report.report_id,
        source_diff_id: diff.diff_id,
        plan_id: diff.plan_id,
        mission_id: diff.mission_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        attempts: vec![],
        outcome: RepairOutcome::Proposed,
        triggering_stage: StageKind::LintFormat,
        successful_attempt: None,
        final_diff: None,
        model_id: "heuristic-v0".into(),
        elapsed_ms: 0,
    }
}

#[test]
fn save_pattern_roundtrip_preserves_payload_and_pattern() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let d = make_diff(vec!["println!(\"hi\");".into()]);
    journal.save_diff(&d).expect("diff");
    let repair = applied_repair_report(&journal, &d);
    journal.save_repair(&repair).expect("save_repair");

    let outcome = learning_runner::run(LearnInput::new(&repair).with_pattern_index(0));
    journal.save_pattern(&outcome).expect("save_pattern");

    let raw = journal
        .pattern_payload(outcome.learn_id)
        .expect("payload")
        .unwrap();
    let back: LearnOutcome = serde_json::from_str(&raw).unwrap();
    assert_eq!(back.learn_id, outcome.learn_id);
    let p = back.pattern.expect("applied -> draft");
    assert_eq!(p.lifecycle, RuleLifecycle::Draft);
    assert!((p.confidence - 0.7).abs() < 1e-6);
    assert_eq!(p.when.pattern, "no_println");
    assert!(back.evidence_diff.is_some());

    let row = journal.pattern_tail(10).expect("tail")[0].clone();
    assert_eq!(row.rule_id, p.rule_id);
    assert_eq!(row.lifecycle, "draft");
    assert_eq!(row.stage, "lint_format");
    assert_eq!(row.strategy, "auto_fix");
    assert!((row.confidence - 0.7_f64).abs() < 1e-6);
    assert_eq!(row.was_correct, 1);
}

#[test]
fn save_pattern_idempotent_replay_does_not_overwrite() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let d = make_diff(vec!["println!(\"hi\");".into()]);
    journal.save_diff(&d).expect("diff");
    let repair = applied_repair_report(&journal, &d);
    journal.save_repair(&repair).expect("save_repair");
    let outcome = learning_runner::run(LearnInput::new(&repair).with_pattern_index(0));

    journal.save_pattern(&outcome).expect("first");
    journal.save_pattern(&outcome).expect("replay");

    let tail = journal.pattern_tail(10).expect("tail");
    assert_eq!(tail.len(), 1, "idempotent replay (RFC 02 §3.1.2)");
}

#[test]
fn no_pattern_outcome_still_persisted_for_audit() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let d = make_diff(vec!["fn x() {}".into()]);
    journal.save_diff(&d).expect("diff");
    let repair = no_pattern_repair_report(&journal, &d);
    journal.save_repair(&repair).expect("save_repair");
    let outcome = learning_runner::run(LearnInput::new(&repair).with_pattern_index(0));
    assert!(outcome.pattern.is_none(), "no attempts -> NoPattern");

    journal.save_pattern(&outcome).expect("save_pattern");

    let row = journal.pattern_tail(10).expect("tail")[0].clone();
    assert_eq!(row.lifecycle, "none");
    assert_eq!(row.stage, "none");
    assert_eq!(row.was_correct, 0);
    assert!(row.rule_id.starts_with("no-pattern-"));
}

#[test]
fn pattern_tail_returns_newest_first() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");

    let d1 = make_diff(vec!["println!(\"a\");".into()]);
    let d2 = make_diff(vec!["println!(\"b\");".into()]);
    journal.save_diff(&d1).expect("diff1");
    journal.save_diff(&d2).expect("diff2");
    let r1 = applied_repair_report(&journal, &d1);
    let r2 = applied_repair_report(&journal, &d2);
    journal.save_repair(&r1).expect("repair1");
    journal.save_repair(&r2).expect("repair2");

    let o1 = learning_runner::run(LearnInput::new(&r1).with_pattern_index(0));
    std::thread::sleep(std::time::Duration::from_millis(10));
    let o2 = learning_runner::run(LearnInput::new(&r2).with_pattern_index(1));

    journal.save_pattern(&o1).expect("save1");
    journal.save_pattern(&o2).expect("save2");

    let tail = journal.pattern_tail(10).expect("tail");
    assert_eq!(tail.len(), 2);
    assert_eq!(tail[0].learn_id, o2.learn_id);
    assert_eq!(tail[1].learn_id, o1.learn_id);
    assert_ne!(tail[0].rule_id, tail[1].rule_id);
}
