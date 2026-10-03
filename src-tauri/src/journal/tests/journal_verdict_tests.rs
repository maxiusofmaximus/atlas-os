use crate::journal::Journal;
use crate::prompt::types::*;
use tempfile::TempDir;
use uuid::Uuid;

fn make_verdict(raw: &str, confidence: ConfidenceLevel) -> PublicUnderstandingVerdict {
    PublicUnderstandingVerdict {
        verdict_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        raw_prompt: raw.into(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        intent: format!("fix intent for {raw:?}"),
        intent_hypotheses: vec![IntentHypothesis {
            rank: 1,
            text: "fix the failing test".into(),
            feasibility_score: 0.5,
            rejection_reason: None,
        }],
        keys: vec!["fix".into()],
        named_entities: vec![],
        domain: Domain::Unknown,
        technology: vec![],
        desired_action: DesiredAction::Debug,
        scope: Scope::Unknown,
        implicit_signals: vec![],
        gaps: vec![],
        similar_missions: vec![],
        clarification_questions: vec![],
        confidence,
        confidence_rubric: ConfidenceRubric {
            intent_clarity: 0.5,
            scope_clarity: 0.5,
            feasibility_clarity: 0.5,
            context_clarity: 0.5,
        },
        observations: vec![],
        recommended_mode: RecommendedMode::Ask,
        model_id: "heuristic-v0".into(),
        judge_model_id: None,
        elapsed_ms: 0,
    }
}

#[test]
fn save_verdict_then_tail_row() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let v = make_verdict("fix", ConfidenceLevel::Low);
    journal
        .save_verdict(&v, Some(Uuid::new_v4()))
        .expect("save");
    let tail = journal.verdict_tail(10).expect("tail");
    assert_eq!(tail.len(), 1);
    assert_eq!(tail[0].verdict_id, v.verdict_id);
    assert_eq!(tail[0].confidence, "LOW");
    assert_eq!(tail[0].recommended_mode, "ask");
}

#[test]
fn save_verdict_idempotent_replay_does_not_overwrite() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let mut v = make_verdict("fix", ConfidenceLevel::Low);
    journal
        .save_verdict(&v, Some(Uuid::new_v4()))
        .expect("save first");
    // Mutate then re-save with the same verdict_id — must NOT replace.
    v.confidence = ConfidenceLevel::High;
    v.recommended_mode = RecommendedMode::Code;
    journal
        .save_verdict(&v, Some(Uuid::new_v4()))
        .expect("save replay");
    let row = journal.verdict_tail(1).expect("tail")[0].clone();
    assert_eq!(row.confidence, "LOW", "first write wins (RFC 02 §3.1.2)");
    assert_eq!(row.recommended_mode, "ask");
}

#[test]
fn save_consolidated_then_tail_and_partial_lock_flip() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let mission = Uuid::new_v4();
    let verdict = make_verdict("ship migration", ConfidenceLevel::Medium);
    journal.save_verdict(&verdict, Some(mission)).expect("v");
    let c = MissionConsolidated {
        mission_id: mission,
        verdict_id: verdict.verdict_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        mission_statement: "migrate rating to sqlite".into(),
        success_criteria: vec!["tests pass".into()],
        non_goals: vec![],
        accepted_assumptions: vec![],
        suggested_mode: RecommendedMode::Architect,
        suggested_tooling: vec![],
        forbidden_actions: vec![],
        requires_research_first: false,
        research_queries: None,
        locked: false,
        locked_at: None,
        locked_by: LockSource::AutoThreshold,
        planning_session_id: None,
        execution_session_id: None,
    };
    journal.save_consolidated(&c).expect("save c");

    let row = journal.consolidated_tail(10).expect("tail")[0].clone();
    assert!(!row.locked);
    assert_eq!(row.suggested_mode, "architect");

    // Step 9 lock: re-save the consolidated with `locked=true` — the
    // ON CONFLICT branch must UPDATE the row.
    let locked_c = MissionConsolidated {
        locked: true,
        locked_at: Some(chrono::Utc::now().to_rfc3339()),
        ..c.clone()
    };
    journal.save_consolidated(&locked_c).expect("lock");
    let row2 = journal.consolidated_tail(10).expect("tail")[0].clone();
    assert!(
        row2.locked,
        "lock flip must persist via ON CONFLICT DO UPDATE"
    );
}

#[test]
fn verdict_payload_round_trip_preserves_full_json() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let v = make_verdict("go", ConfidenceLevel::Block);
    journal.save_verdict(&v, None).expect("save");
    let raw = journal
        .verdict_payload(v.verdict_id)
        .expect("payload")
        .unwrap();
    let back: PublicUnderstandingVerdict = serde_json::from_str(&raw).unwrap();
    assert_eq!(back.raw_prompt, "go");
    assert_eq!(back.confidence, ConfidenceLevel::Block);
}

#[test]
fn latest_verdict_for_mission_returns_newest_bound_verdict() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let mission = Uuid::new_v4();

    // one verdict bound to the mission  — must come back
    let v = make_verdict("first prompt", ConfidenceLevel::Medium);
    journal.save_verdict(&v, Some(mission)).expect("save v1");
    let back = journal
        .latest_verdict_for_mission(mission)
        .expect("lookup")
        .expect("present");
    assert_eq!(back.verdict_id, v.verdict_id);

    // a second verdict for the same mission must win as "latest"
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let v2 = make_verdict("second prompt", ConfidenceLevel::High);
    journal.save_verdict(&v2, Some(mission)).expect("save v2");
    let back2 = journal
        .latest_verdict_for_mission(mission)
        .expect("lookup")
        .expect("present");
    assert_eq!(back2.verdict_id, v2.verdict_id);
}

#[test]
fn latest_verdict_for_mission_returns_none_when_no_match() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let other = Uuid::new_v4();
    let v = make_verdict("orphan", ConfidenceLevel::Low);
    journal.save_verdict(&v, Some(Uuid::new_v4())).expect("v");
    assert!(journal
        .latest_verdict_for_mission(other)
        .expect("ok")
        .is_none());
}

#[test]
fn latest_consolidated_for_mission_round_trips_payload() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let mission = Uuid::new_v4();
    let verdict = make_verdict("ship migration", ConfidenceLevel::High);
    journal.save_verdict(&verdict, Some(mission)).expect("v");
    let mut c = MissionConsolidated {
        mission_id: mission,
        verdict_id: verdict.verdict_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        mission_statement: "migrate rating to sqlite".into(),
        success_criteria: vec!["tests pass".into()],
        non_goals: vec![],
        accepted_assumptions: vec![],
        suggested_mode: RecommendedMode::Code,
        suggested_tooling: vec![],
        forbidden_actions: vec![],
        requires_research_first: false,
        research_queries: None,
        locked: true,
        locked_at: None,
        locked_by: LockSource::User,
        planning_session_id: None,
        execution_session_id: None,
    };
    journal.save_consolidated(&c).expect("save");
    let back = journal
        .latest_consolidated_for_mission(mission)
        .expect("ok")
        .expect("present");
    assert_eq!(back.mission_id, mission);
    assert!(back.locked);

    // missing mission — None
    assert!(journal
        .latest_consolidated_for_mission(Uuid::new_v4())
        .expect("ok")
        .is_none());

    // sanity: unused-so-far field assignment still compiles
    c.accepted_assumptions.push(AcceptedAssumption {
        assumption: "x".into(),
        user_confirmed: true,
        auto_resolved: false,
        source: AssumptionSource::UserAnswer,
    });
    assert_eq!(c.accepted_assumptions.len(), 1);
}
