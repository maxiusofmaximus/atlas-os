use crate::journal::Journal;
use crate::planning::types::*;
use tempfile::TempDir;
use uuid::Uuid;

fn make_plan(strategy: Strategy, confidence: f32) -> Plan {
    Plan {
        plan_id: Uuid::new_v4(),
        mission_id: Uuid::new_v4(),
        verdict_id: Uuid::new_v4(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        mission: "m".into(),
        objectives: vec![],
        steps: vec![],
        strategy,
        risk: 0.4,
        impact: Impact::Minor,
        roadmap: vec![],
        skills_used: vec![],
        models_needed: vec![],
        research_runs: vec![],
        confidence,
        resume_point: "S0".into(),
        blocked: vec![],
        model_id: "heuristic-v0".into(),
        elapsed_ms: 0,
    }
}

#[test]
fn save_plan_then_tail_row() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let p = make_plan(Strategy::Strangler, 0.8);
    journal.save_plan(&p).expect("save");
    let tail = journal.plan_tail(10).expect("tail");
    assert_eq!(tail.len(), 1);
    assert_eq!(tail[0].plan_id, p.plan_id);
    assert_eq!(tail[0].strategy, "strangler");
    assert!((tail[0].confidence - 0.8).abs() < 1e-6);
}

#[test]
fn save_plan_idempotent_replay_does_not_overwrite() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let mut p = make_plan(Strategy::BigBang, 0.5);
    journal.save_plan(&p).expect("first");
    p.confidence = 0.95;
    journal.save_plan(&p).expect("replay");
    let tail = journal.plan_tail(1).expect("tail");
    assert!((tail[0].confidence - 0.5).abs() < 1e-6, "first write wins");
}

#[test]
fn latest_plan_for_mission_returns_most_recent() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let p1 = make_plan(Strategy::BigBang, 0.5);
    journal.save_plan(&p1).expect("v1");
    std::thread::sleep(std::time::Duration::from_millis(2));
    let mut p2 = make_plan(Strategy::Incremental, 0.7);
    p2.mission_id = p1.mission_id;
    journal.save_plan(&p2).expect("v2");
    let row = journal
        .latest_plan_for_mission(p1.mission_id)
        .expect("latest")
        .expect("must find");
    assert_eq!(row.plan_id, p2.plan_id);
}
