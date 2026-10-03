use crate::journal::Journal;
use crate::supervisor::types::{BudgetTally, MissionCheckpoint, MissionPhase};
use tempfile::TempDir;
use uuid::Uuid;

fn cp(mission_id: Uuid, phase: MissionPhase) -> MissionCheckpoint {
    MissionCheckpoint {
        checkpoint_id: Uuid::new_v4(),
        mission_id,
        phase,
        current_plan_id: None,
        last_validation_report_id: None,
        last_repair_id: None,
        budget_tally: BudgetTally::default(),
        caps: None,
        mode: None,
        generated_at: chrono::Utc::now().to_rfc3339(),
    }
}

#[test]
fn save_checkpoint_roundtrip_preserves_payload_and_phase() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let mission = Uuid::new_v4();
    let ck = cp(mission, MissionPhase::Verifying);
    journal.save_checkpoint(&ck).expect("save");

    let raw = journal
        .checkpoint_payload(ck.checkpoint_id)
        .expect("payload")
        .unwrap();
    let back: MissionCheckpoint = serde_json::from_str(&raw).unwrap();
    assert_eq!(back.checkpoint_id, ck.checkpoint_id);
    assert_eq!(back.phase, MissionPhase::Verifying);
    assert_eq!(back.budget_tally, ck.budget_tally);

    let row = journal.checkpoint_tail(10).expect("tail")[0].clone();
    assert_eq!(row.checkpoint_id, ck.checkpoint_id);
    assert_eq!(row.phase, "verifying");
}

#[test]
fn save_checkpoint_idempotent_replay_does_not_overwrite() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let mission = Uuid::new_v4();
    let ck = cp(mission, MissionPhase::Planning);
    journal.save_checkpoint(&ck).expect("first");
    journal.save_checkpoint(&ck).expect("replay");
    let tail = journal.checkpoint_tail(10).expect("tail");
    assert_eq!(tail.len(), 1, "idempotent replay (RFC 02 §3.1.2)");
}

#[test]
fn latest_checkpoint_returns_newest_for_mission() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let mission = Uuid::new_v4();

    let mut older = cp(mission, MissionPhase::Planning);
    older.generated_at = "2026-07-22T10:00:00Z".into();
    journal.save_checkpoint(&older).expect("save old");

    std::thread::sleep(std::time::Duration::from_millis(10));

    let newer = cp(mission, MissionPhase::Executing);
    journal.save_checkpoint(&newer).expect("save new");

    let latest = journal.latest_checkpoint(mission).expect("latest").unwrap();
    assert_eq!(latest.checkpoint_id, newer.checkpoint_id);
    assert_eq!(latest.phase, "executing");

    // A non-existing mission returns None.
    let absent = journal.latest_checkpoint(Uuid::new_v4()).expect("query");
    assert!(absent.is_none());
}

#[test]
fn checkpoint_tail_returns_newest_first_across_missions() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let m1 = Uuid::new_v4();
    let m2 = Uuid::new_v4();

    let c1 = cp(m1, MissionPhase::Planning);
    journal.save_checkpoint(&c1).expect("c1");
    std::thread::sleep(std::time::Duration::from_millis(10));
    let c2 = cp(m2, MissionPhase::Done);
    journal.save_checkpoint(&c2).expect("c2");

    let tail = journal.checkpoint_tail(10).expect("tail");
    assert_eq!(tail.len(), 2);
    assert_eq!(tail[0].checkpoint_id, c2.checkpoint_id);
    assert_eq!(tail[1].checkpoint_id, c1.checkpoint_id);
}

/// RFC 19 §6.1 — drive the supervisor through several phases and
/// verify each emitted `PersistCheckpoint` action survives a
/// Journal round-trip with the right phase tag.
#[test]
fn supervisor_driven_checkpoints_round_trip_through_journal() {
    use crate::supervisor::runner::{tick, TickContext};
    use crate::supervisor::types::{BudgetCaps, ExecutionMode, SupervisorEvent, SupervisorState};

    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
    let mut state = SupervisorState::new(BudgetCaps::DEFAULT, ExecutionMode::HumanInLoop);

    let mission = Uuid::new_v4();
    let plan = Uuid::new_v4();

    let events = vec![
        SupervisorEvent::MissionStarted {
            mission_id: mission,
        },
        SupervisorEvent::PlanGenerated { plan_id: plan },
        SupervisorEvent::ValidationStarted,
    ];
    let mut final_phase = state.phase;
    for e in events {
        let out = tick(&mut ctx, state, e);
        for a in &out.actions {
            if let crate::supervisor::types::SupervisorAction::PersistCheckpoint(ckpt) = a {
                journal.save_checkpoint(ckpt).expect("save");
            }
        }
        state = out.state;
        final_phase = state.phase;
    }
    assert_eq!(final_phase, MissionPhase::Verifying);

    let tail = journal.checkpoint_tail(10).expect("tail");
    assert_eq!(
        tail.len(),
        3,
        "one checkpoint per phase transition (MissionStarted/Planning/Verifying)"
    );
    // Newest first.
    assert_eq!(tail[0].phase, "verifying");
    assert_eq!(tail[1].phase, "executing");
    assert_eq!(tail[2].phase, "planning");

    let latest = journal.latest_checkpoint(mission).expect("latest").unwrap();
    assert_eq!(latest.phase, "verifying");
}
