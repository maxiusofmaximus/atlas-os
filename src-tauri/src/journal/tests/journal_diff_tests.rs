use crate::coding::types::*;
use crate::journal::Journal;
use tempfile::TempDir;
use uuid::Uuid;

fn make_diff(path: &str, lines: Vec<String>, is_new_file: bool) -> Diff {
    Diff {
        diff_id: Uuid::new_v4(),
        plan_id: Uuid::new_v4(),
        mission_id: Uuid::new_v4(),
        step_id: "S1".into(),
        agent_id: Uuid::new_v4(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        files: vec![FileEdit {
            path: path.into(),
            is_new_file,
            is_delete: false,
            hunks: vec![Hunk {
                old_start: 0,
                old_end: 0,
                new_lines: lines,
                rationale: "r".into(),
            }],
        }],
        narrative: String::new(),
        research_refs: vec![],
        risk_decision: None,
        model_id: "heuristic-v0".into(),
        elapsed_ms: 0,
    }
}

#[test]
fn save_diff_then_tail_and_payload() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let d = make_diff(
        "src/lib.rs",
        vec!["fn add(a: u32, b: u32) -> u32 { a + b }".into()],
        false,
    );
    journal.save_diff(&d).expect("save");
    let tail = journal.diff_tail(10).expect("tail");
    assert_eq!(tail.len(), 1);
    assert_eq!(tail[0].diff_id, d.diff_id);
    assert_eq!(tail[0].step_id, "S1");
    assert!(tail[0].lines_added >= 1);
    let raw = journal.diff_payload(d.diff_id).expect("payload").unwrap();
    let back: Diff = serde_json::from_str(&raw).unwrap();
    assert_eq!(back.files[0].path, "src/lib.rs");
}

#[test]
fn save_diff_idempotent_replay_does_not_overwrite() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let d = make_diff("a.rs", vec!["fn x() {}".into()], false);
    journal.save_diff(&d).expect("first");
    journal.save_diff(&d).expect("replay");
    let tail = journal.diff_tail(10).expect("tail");
    assert_eq!(tail.len(), 1, "replay must NOT duplicate");
}
