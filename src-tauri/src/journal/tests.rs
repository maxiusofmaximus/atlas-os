// Atlas OS — Rust tests for the Journal store.
// Phase 0 happy-path: open a temp-profile journal, publish an event, tail it.
#[cfg(test)]
mod journal_store_tests {
    // Reach into the crate root: tests live as a child mod of `journal`,
    // so `super::Journal` would resolve to `journal::tests::Journal`.
    use crate::core::bus::{BusEvent, BusEventKind};
    use crate::journal::Journal;
    use tempfile::TempDir;

    #[test]
    fn journal_happy_path_publish_and_tail() {
        let tmp = TempDir::new().expect("tmp");
        let root = tmp.path().to_path_buf();
        let journal = Journal::open(&root).expect("open");

        let evt = BusEvent::new(BusEventKind::TaskReceived {
            raw_prompt: "hello world".into(),
            session_id: uuid::Uuid::new_v4(),
        });
        journal.publish(&evt).expect("publish");

        let tail = journal.tail(10).expect("tail");
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].kind, "task_received");
    }

    #[test]
    fn journal_idempotent_replay() {
        let tmp = TempDir::new().expect("tmp");
        let root = tmp.path().to_path_buf();
        let journal = Journal::open(&root).expect("open");

        let evt = BusEvent::new(BusEventKind::AgentStatusChanged {
            agent_id: uuid::Uuid::new_v4(),
            status: crate::core::bus::AgentStatus::Queued,
        });
        journal.publish(&evt).expect("publish");
        // Replay the same event (same `id`) — must NOT duplicate.
        journal.publish(&evt).expect("publish replay");

        let tail = journal.tail(10).expect("tail");
        assert_eq!(
            tail.len(),
            1,
            "idempotency_key collision must drop the replay (RFC 02 §3.1.2)"
        );
    }
}

#[cfg(test)]
mod journal_verdict_tests {
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
}

#[cfg(test)]
mod journal_plan_tests {
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
}

#[cfg(test)]
mod journal_diff_tests {
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
}

#[cfg(test)]
mod journal_validation_tests {
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
}

#[cfg(test)]
mod journal_repair_tests {
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
        let report =
            make_repair_report(&journal, &d, RepairOutcome::Applied, vec![attempt], Some(0));

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
}

#[cfg(test)]
mod journal_pattern_tests {
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
}

#[cfg(test)]
mod journal_checkpoint_tests {
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
        use crate::supervisor::types::{
            BudgetCaps, ExecutionMode, SupervisorEvent, SupervisorState,
        };

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
}

#[cfg(test)]
mod journal_skill_tests {
    use crate::journal::Journal;
    use crate::skills::{Engine, SkillManifest};
    use tempfile::TempDir;

    fn skill(id: &str, version: &str, priority: u32, engine: Engine) -> SkillManifest {
        SkillManifest {
            id: id.into(),
            version: version.into(),
            description: format!("{id} v{version}"),
            engine,
            priority,
            domain: Some("frontend".into()),
            language: Some("typescript".into()),
            framework: Some("react".into()),
            confidence: 0.85,
            conflicts: vec!["vue-ui-expert".into()],
            verified: true,
            ..Default::default()
        }
    }

    #[test]
    fn save_skill_roundtrip_preserves_payload() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let s = skill("react-ui-expert", "1.4.2", 90, Engine::Coding);
        journal.save_skill(&s).expect("save");

        let raw = journal
            .skill_payload("react-ui-expert", "1.4.2")
            .expect("payload")
            .unwrap();
        let back: SkillManifest = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.id, "react-ui-expert");
        assert_eq!(back.version, "1.4.2");
        assert_eq!(back.engine, Engine::Coding);
        assert_eq!(back.priority, 90);
        assert_eq!(back.domain.as_deref(), Some("frontend"));
        assert_eq!(back.conflicts, vec!["vue-ui-expert"]);

        let row = journal.skill_tail(10).expect("tail")[0].clone();
        assert_eq!(row.skill_id, "react-ui-expert");
        assert_eq!(row.version, "1.4.2");
        assert_eq!(row.engine, "coding");
        assert_eq!(row.priority, 90);
        assert!((row.confidence - 0.85_f64).abs() < 1e-6);
        assert!(row.verified);
        assert!(!row.auto_generated);
    }

    #[test]
    fn save_skill_idempotent_replay_does_not_overwrite() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let s = skill("tailwind-expert", "0.2.0", 80, Engine::Coding);
        journal.save_skill(&s).expect("first");
        journal.save_skill(&s).expect("replay");
        let tail = journal.skill_tail(10).expect("tail");
        assert_eq!(tail.len(), 1, "idempotent replay (RFC 02 §3.1.2)");
    }

    #[test]
    fn new_version_keeps_old_history() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        // Same skill id, two versions.
        journal
            .save_skill(&skill("react-ui-expert", "1.4.0", 70, Engine::Coding))
            .expect("v1");
        std::thread::sleep(std::time::Duration::from_millis(10));
        journal
            .save_skill(&skill("react-ui-expert", "1.4.2", 90, Engine::Coding))
            .expect("v2");
        let tail = journal.skill_tail(10).expect("tail");
        assert_eq!(tail.len(), 2, "both versions persisted");
        // Newest first.
        assert_eq!(tail[0].version, "1.4.2");
        assert_eq!(tail[1].version, "1.4.0");
        // Latest lookup returns the newest.
        let latest = journal
            .latest_skill("react-ui-expert")
            .expect("latest")
            .unwrap();
        assert_eq!(latest.version, "1.4.2");
        // And the old version's payload is still recoverable.
        let raw = journal
            .skill_payload("react-ui-expert", "1.4.0")
            .expect("payload")
            .unwrap();
        let old: SkillManifest = serde_json::from_str(&raw).unwrap();
        assert_eq!(old.priority, 70);
    }

    #[test]
    fn unknown_skill_returns_none() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let payload = journal.skill_payload("ghost", "1.0.0").expect("query");
        assert!(payload.is_none());
        let latest = journal.latest_skill("ghost").expect("query");
        assert!(latest.is_none());
    }
}

#[cfg(test)]
mod mission_graph_schema_tests {
    use crate::journal::Journal;
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn fresh_conn() -> (TempDir, Connection) {
        let tmp = TempDir::new().expect("tmp");
        let _journal = Journal::open(tmp.path()).expect("open runs migrate");
        let conn = Connection::open(tmp.path().join("journal.db")).expect("open raw conn");
        (tmp, conn)
    }

    #[test]
    fn m15_advances_schema_version_to_15() {
        let (_tmp, conn) = fresh_conn();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(
            v >= 15,
            "expected schema version >= 15 (M16 runs after M15), got {v}"
        );
    }

    #[test]
    fn m15_creates_mission_graph_nodes_and_edges_and_learning_graphs() {
        let (_tmp, conn) = fresh_conn();
        for table in [
            "mission_graph_nodes",
            "mission_graph_edges",
            "learning_graphs",
        ] {
            let exists: i64 = conn
                .query_row(
                    &format!(
                        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='{table}'"
                    ),
                    [],
                    |r| r.get(0),
                )
                .expect("query");
            assert_eq!(exists, 1, "{table} should exist after M15");
        }
    }

    #[test]
    fn m15_node_kind_check_rejects_unknown_value() {
        let (_tmp, conn) = fresh_conn();
        // No missions row exists so we expect FK violation BEFORE the
        // CHECK — guard around the FK first by inserting a mission.
        conn.execute(
            "INSERT INTO missions (id, label, status, created_at, updated_at)
             VALUES ('00000000-0000-0000-0000-000000000001', 'l', 'received', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
            [],
        )
        .expect("seed mission");
        let err = conn
            .execute(
                "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance) \
                 VALUES ('n1', '00000000-0000-0000-0000-000000000001', 'unknown_kind', 'x', 'EXTRACTED')",
                [],
            )
            .expect_err("CHECK should reject");
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("constraint") || msg.to_lowercase().contains("check"),
            "expected CHECK constraint failure, got: {msg}"
        );
    }

    #[test]
    fn m15_provenance_check_accepts_extracted_inferred_ambiguous() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO missions (id, label, status, created_at, updated_at)
             VALUES ('00000000-0000-0000-0000-000000000001', 'l', 'received', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
            [],
        )
        .expect("seed mission");
        for (idx, p) in ["EXTRACTED", "INFERRED", "AMBIGUOUS"].iter().enumerate() {
            conn.execute(
                "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance) \
                 VALUES (?1, '00000000-0000-0000-0000-000000000001', 'mission', ?2, ?3)",
                rusqlite::params![format!("n{idx}"), format!("lbl{idx}"), p],
            )
            .expect("valid provenance");
        }
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM mission_graph_nodes WHERE mission_id = ?1",
                rusqlite::params!["00000000-0000-0000-0000-000000000001"],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(count, 3);
    }

    #[test]
    fn m15_edge_kind_check_rejects_unknown_value() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO missions (id, label, status, created_at, updated_at)
             VALUES ('00000000-0000-0000-0000-000000000001', 'l', 'received', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
            [],
        )
        .expect("seed mission");
        conn.execute(
            "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance) \
             VALUES ('n1', '00000000-0000-0000-0000-000000000001', 'mission', 'a', 'EXTRACTED'),
                    ('n2', '00000000-0000-0000-0000-000000000001', 'mission', 'b', 'EXTRACTED')",
            [],
        )
        .expect("seed nodes");
        let err = conn
            .execute(
                "INSERT INTO mission_graph_edges (id, mission_id, src, dst, kind) \
                 VALUES ('e1', '00000000-0000-0000-0000-000000000001', 'n1', 'n2', 'teleports_to')",
                [],
            )
            .expect_err("CHECK should reject unknown edge kind");
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("constraint") || msg.to_lowercase().contains("check"),
            "expected CHECK constraint failure, got: {msg}"
        );
    }

    #[test]
    fn m15_edge_visit_count_defaults_to_zero() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO missions (id, label, status, created_at, updated_at)
             VALUES ('00000000-0000-0000-0000-000000000001', 'l', 'received', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
            [],
        )
        .expect("seed mission");
        conn.execute(
            "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance) \
             VALUES ('n1', '00000000-0000-0000-0000-000000000001', 'mission', 'a', 'EXTRACTED'),
                    ('n2', '00000000-0000-0000-0000-000000000001', 'mission', 'b', 'EXTRACTED')",
            [],
        )
        .expect("seed nodes");
        conn.execute(
            "INSERT INTO mission_graph_edges (id, mission_id, src, dst, kind) \
             VALUES ('e1', '00000000-0000-0000-0000-000000000001', 'n1', 'n2', 'calls')",
            [],
        )
        .expect("insert edge without visit_count");
        let count: i64 = conn
            .query_row(
                "SELECT visit_count FROM mission_graph_edges WHERE id = 'e1'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(count, 0);
    }

    #[test]
    fn m15_learning_graphs_accepts_zero_one_success() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO learning_graphs (id, intent_signature, success, graph_json, created_ts) \
             VALUES ('g0', 'sig-0', 0, '{}'            , 100), \
                    ('g1', 'sig-1', 1, '{\"a\":1}'     , 200)",
            [],
        )
        .expect("insert both success variants");
        let err = conn
            .execute(
                "INSERT INTO learning_graphs (id, intent_signature, success, graph_json, created_ts) \
                 VALUES ('g2', 'sig-2', 2, '{}', 300)",
                [],
            )
            .expect_err("CHECK should reject success=2");
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("constraint") || msg.to_lowercase().contains("check"),
            "expected CHECK constraint failure, got: {msg}"
        );
    }

    #[test]
    fn m15_cascade_delete_removes_edges_and_nodes_children() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO missions (id, label, status, created_at, updated_at)
             VALUES ('00000000-0000-0000-0000-000000000001', 'l', 'received', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
            [],
        )
        .expect("seed mission");
        conn.execute(
            "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance) \
             VALUES ('n1', '00000000-0000-0000-0000-000000000001', 'mission', 'a', 'EXTRACTED'), \
                    ('n2', '00000000-0000-0000-0000-000000000001', 'mission', 'b', 'EXTRACTED')",
            [],
        )
        .expect("seed nodes");
        conn.execute(
            "INSERT INTO mission_graph_edges (id, mission_id, src, dst, kind) \
             VALUES ('e1', '00000000-0000-0000-0000-000000000001', 'n1', 'n2', 'calls')",
            [],
        )
        .expect("seed edge");
        conn.execute(
            "DELETE FROM missions WHERE id = '00000000-0000-0000-0000-000000000001'",
            [],
        )
        .expect("delete mission");
        let nodes: i64 = conn
            .query_row("SELECT count(*) FROM mission_graph_nodes", [], |r| r.get(0))
            .expect("query");
        let edges: i64 = conn
            .query_row("SELECT count(*) FROM mission_graph_edges", [], |r| r.get(0))
            .expect("query");
        assert_eq!(nodes, 0);
        assert_eq!(edges, 0);
    }

    #[test]
    fn m16_advances_schema_version_to_at_least_16() {
        let (_tmp, conn) = fresh_conn();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(v >= 16, "expected schema version >= 16 after M16, got {v}");
    }

    #[test]
    fn m17_advances_schema_version_to_at_least_17() {
        // M17 (toast_queue + toast_history) runs unconditionally — the
        // tables exist whether or not the `toast` feature is on. This
        // keeps the schema idempotent across feature combos.
        let (_tmp, conn) = fresh_conn();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(v >= 17, "expected schema version >= 17 after M17, got {v}");
    }

    #[test]
    fn m17_creates_toast_queue_and_history_tables() {
        let (_tmp, conn) = fresh_conn();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .expect("prepare")
            .query_map([], |r| r.get(0))
            .expect("query_map")
            .map(Result::unwrap)
            .collect();
        assert!(
            tables.iter().any(|t| t == "toast_queue"),
            "expected `toast_queue` table after M17, got: {tables:?}"
        );
        assert!(
            tables.iter().any(|t| t == "toast_history"),
            "expected `toast_history` table after M17, got: {tables:?}"
        );
    }

    #[test]
    fn m16_adds_embedding_and_emb_model_columns_to_learning_graphs() {
        let (_tmp, conn) = fresh_conn();
        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(learning_graphs)")
            .expect("prepare")
            .query_map([], |r| {
                let name: String = r.get(1)?;
                Ok(name)
            })
            .expect("query_map")
            .map(Result::unwrap)
            .collect();
        assert!(
            cols.iter().any(|c| c == "embedding"),
            "expected `embedding` column after M16, got: {cols:?}"
        );
        assert!(
            cols.iter().any(|c| c == "emb_model"),
            "expected `emb_model` column after M16, got: {cols:?}"
        );
    }

    #[test]
    fn m16_learning_graphs_accepts_nullable_embedding_and_emb_model() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO learning_graphs (id, intent_signature, success, graph_json, created_ts)
             VALUES ('g0', 'sig', 1, '{}', 1)",
            [],
        )
        .expect("insert without embedding should succeed (nullable)");
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM learning_graphs WHERE embedding IS NULL AND emb_model IS NULL",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(n, 1);
    }

    #[test]
    fn m16_learning_graphs_accepts_dense_embedding_blob() {
        let (_tmp, conn) = fresh_conn();
        let emb: [u8; 8] = [
            0x00, 0x00, 0x80, 0x3f, // 1.0f32 LE
            0x00, 0x00, 0x00, 0x40, // 2.0f32 LE
        ];
        conn.execute(
            "INSERT INTO learning_graphs (id, intent_signature, success, graph_json, created_ts, embedding, emb_model)
             VALUES ('g1', 'sig', 1, '{}', 1, ?1, 'fastembed-bge-small')",
            rusqlite::params![emb.to_vec()],
        )
        .expect("insert with embedding blob");
        let model: String = conn
            .query_row(
                "SELECT emb_model FROM learning_graphs WHERE id = 'g1'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(model, "fastembed-bge-small");
    }

    #[test]
    fn m14_advances_schema_version_to_at_least_14() {
        let (_tmp, conn) = fresh_conn();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(
            v >= 14,
            "expected schema version >= 14 (M14 runs before M15/M16), got {v}"
        );
    }

    #[test]
    fn m14_creates_agent_session_events_table() {
        let (_tmp, conn) = fresh_conn();
        let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='agent_session_events'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(exists, 1, "agent_session_events should exist after M14");
    }

    #[test]
    fn m14_creates_ts_and_task_indexes() {
        let (_tmp, conn) = fresh_conn();
        for idx in ["idx_ase_ts", "idx_ase_task"] {
            let exists: i64 = conn
                .query_row(
                    &format!(
                        "SELECT count(*) FROM sqlite_master WHERE type='index' AND name='{idx}'"
                    ),
                    [],
                    |r| r.get(0),
                )
                .expect("query");
            assert_eq!(exists, 1, "index {idx} should exist after M14");
        }
    }

    #[test]
    fn m14_accepts_nullable_pane_id_and_task_id() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO agent_session_events
                (ts, pane_id, event_type, agent, task_id, payload_json)
             VALUES (1722000000, NULL, 'agent.idle', 'opencode', NULL, '{}')",
            [],
        )
        .expect("insert with NULL pane_id/task_id");
        let n: i64 = conn
            .query_row("SELECT count(*) FROM agent_session_events", [], |r| {
                r.get(0)
            })
            .expect("query");
        assert_eq!(n, 1);
    }

    #[test]
    fn m14_roundtrips_envelope_payload_json() {
        let (_tmp, conn) = fresh_conn();
        let payload = r#"{"event":"agent.task.completed","hud_url":"http://127.0.0.1:57457"}"#;
        conn.execute(
            "INSERT INTO agent_session_events
                (ts, pane_id, event_type, agent, task_id, payload_json)
             VALUES (1722000001, 'pane-3', 'agent.task.completed', 'opencode', 'm-1', ?1)",
            rusqlite::params![payload],
        )
        .expect("insert");
        let stored: String = conn
            .query_row(
                "SELECT payload_json FROM agent_session_events WHERE ts = 1722000001",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(stored, payload);
    }

    #[test]
    fn m18_advances_schema_version_to_at_least_18() {
        // M18 (calendar_busy_windows + calendar_auth) runs
        // unconditionally — the tables exist whether or not the
        // `calendar-*` features are on. This keeps the schema
        // idempotent across feature combos, same as M17.
        let (_tmp, conn) = fresh_conn();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(v >= 18, "expected schema version >= 18 after M18, got {v}");
    }

    #[test]
    fn m18_creates_calendar_busy_windows_and_auth_tables() {
        let (_tmp, conn) = fresh_conn();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .expect("prepare")
            .query_map([], |r| r.get(0))
            .expect("query_map")
            .map(Result::unwrap)
            .collect();
        assert!(
            tables.iter().any(|t| t == "calendar_busy_windows"),
            "expected `calendar_busy_windows` table after M18, got: {tables:?}"
        );
        assert!(
            tables.iter().any(|t| t == "calendar_auth"),
            "expected `calendar_auth` table after M18, got: {tables:?}"
        );
    }

    #[test]
    fn m18_calendar_busy_windows_source_check_constraint_rejects_unknown() {
        let (_tmp, conn) = fresh_conn();
        let ok = conn.execute(
            "INSERT INTO calendar_busy_windows
                (source, external_id, subject, starts_at, ends_at, weight)
             VALUES ('graph', 'evt-1', 'Sprint planning', 1_000, 2_000, 1.0)",
            [],
        );
        assert!(ok.is_ok(), "graph source should be accepted: {:?}", ok);
        let err = conn.execute(
            "INSERT INTO calendar_busy_windows
                (source, external_id, subject, starts_at, ends_at, weight)
             VALUES ('unknown', 'evt-2', 'Bad source', 1_000, 2_000, 1.0)",
            [],
        );
        assert!(
            err.is_err(),
            "unknown source should be rejected by CHECK constraint"
        );
    }

    #[test]
    fn m18_calendar_busy_windows_unique_source_external_id_dedupe() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO calendar_busy_windows
                (source, external_id, subject, starts_at, ends_at, weight)
             VALUES ('graph', 'evt-7', 'First insert', 1_000, 2_000, 1.0)",
            [],
        )
        .expect("first insert");
        let dup = conn.execute(
            "INSERT INTO calendar_busy_windows
                (source, external_id, subject, starts_at, ends_at, weight)
             VALUES ('graph', 'evt-7', 'Dup insert', 3_000, 4_000, 1.0)",
            [],
        );
        assert!(
            dup.is_err(),
            "duplicate (source, external_id) should be rejected by UNIQUE constraint"
        );
    }

    #[test]
    fn m18_calendar_busy_windows_accepts_soft_busy_weight() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO calendar_busy_windows
                (source, external_id, subject, starts_at, ends_at, weight)
             VALUES ('ics_local', 'mtg-soft', 'Maybe join', 5_000, 6_000, 0.5)",
            [],
        )
        .expect("insert with weight=0.5 (soft busy)");
        let w: f64 = conn
            .query_row(
                "SELECT weight FROM calendar_busy_windows WHERE external_id = 'mtg-soft'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert!((w - 0.5).abs() < 1e-6, "soft-busy weight should roundtrip");
    }

    #[test]
    fn m18_calendar_auth_accepts_encrypted_token_blob() {
        let (_tmp, conn) = fresh_conn();
        let ciphertext: Vec<u8> = vec![0u8; 32];
        conn.execute(
            "INSERT INTO calendar_auth (account, token_ciphertext, key_hint)
             VALUES ('alice@contoso.com', ?1, 'host-key-1')",
            rusqlite::params![ciphertext],
        )
        .expect("insert");
        let stored_hint: String = conn
            .query_row(
                "SELECT key_hint FROM calendar_auth WHERE account = 'alice@contoso.com'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(stored_hint, "host-key-1");
    }
}

mod model_resets_schema_tests {
    use crate::journal::Journal;
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn fresh_conn() -> (TempDir, Connection) {
        let tmp = TempDir::new().expect("tmp");
        let _journal = Journal::open(tmp.path()).expect("open runs migrate");
        let conn = Connection::open(tmp.path().join("journal.db")).expect("open raw conn");
        (tmp, conn)
    }

    #[test]
    fn m19_advances_schema_version_to_at_least_19() {
        let (_tmp, conn) = fresh_conn();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(v >= 19, "expected schema version >= 19 after M19, got {v}");
    }

    #[test]
    fn m19_creates_model_resets_table() {
        let (_tmp, conn) = fresh_conn();
        let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='model_resets'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(exists, 1, "model_resets table should exist after M19");
    }

    #[test]
    fn m19_unique_constraint_dedupes_same_provider_model_resets_at() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO model_resets (provider, model, status_code, error_type, resets_at, request_id)
             VALUES ('anthropic', 'claude-3-5-sonnet', 429, 'rate_limit', 1_700_000_000_000, 'req_1')",
            [],
        )
        .expect("first insert");
        let dup = conn.execute(
            "INSERT INTO model_resets (provider, model, status_code, error_type, resets_at, request_id)
             VALUES ('anthropic', 'claude-3-5-sonnet', 429, 'rate_limit', 1_700_000_000_000, 'req_2')",
            [],
        );
        assert!(
            dup.is_err(),
            "duplicate (provider, model, resets_at) should be rejected by UNIQUE index"
        );
    }

    #[test]
    fn m19_roundtrips_all_columns() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO model_resets
                (provider, model, status_code, error_type, resets_at, request_id, observed_at, toast_id, toast_dismissed_at)
             VALUES ('openai', 'gpt-5', 402, 'spend_limit', 1_700_000_001_000, 'req_3', 1_700_000_000_500, 42, 1_700_000_002_000)",
            [],
        )
        .expect("insert full row");
        let (
            provider,
            model,
            status_code,
            error_type,
            resets_at,
            request_id,
            observed_at,
            toast_id,
            toast_dismissed_at,
        ): (String, String, i64, String, i64, String, i64, i64, i64) = conn
            .query_row(
                "SELECT provider, model, status_code, error_type, resets_at, request_id, observed_at, toast_id, toast_dismissed_at
                 FROM model_resets WHERE provider = 'openai'",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                    ))
                },
            )
            .expect("query");
        assert_eq!(provider, "openai");
        assert_eq!(model, "gpt-5");
        assert_eq!(status_code, 402);
        assert_eq!(error_type, "spend_limit");
        assert_eq!(resets_at, 1_700_000_001_000);
        assert_eq!(request_id, "req_3");
        assert_eq!(observed_at, 1_700_000_000_500);
        assert_eq!(toast_id, 42);
        assert_eq!(toast_dismissed_at, 1_700_000_002_000);
    }

    #[test]
    fn m19_accepts_nullable_error_type_and_request_id() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO model_resets (provider, model, status_code, error_type, resets_at, request_id)
             VALUES ('omniroute', 'claude-3-5-sonnet', 429, NULL, 1_700_000_000_000, NULL)",
            [],
        )
        .expect("insert with NULL error_type/request_id");
        let n: i64 = conn
            .query_row("SELECT count(*) FROM model_resets", [], |r| r.get(0))
            .expect("query");
        assert_eq!(n, 1);
    }

    #[test]
    fn m19_pending_partial_index_lists_undismissed_rows() {
        // The `model_resets_pending_idx` partial index covers rows
        // where `toast_dismissed_at IS NULL` — the scheduler's working
        // set. Sanity-check that rows with `toast_dismissed_at = NULL`
        // are listed by the index (alias via SELECT FROM the table with
        // same filter).
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO model_resets (provider, model, status_code, error_type, resets_at)
             VALUES ('anthropic', 'claude-3-5-sonnet', 429, 'rate_limit', 1_700_000_000_000)",
            [],
        )
        .expect("pending row");
        conn.execute(
            "INSERT INTO model_resets (provider, model, status_code, error_type, resets_at, toast_dismissed_at)
             VALUES ('openai', 'gpt-5', 402, 'spend_limit', 1_700_000_001_000, 1_800_000_000_000)",
            [],
        )
        .expect("dismissed row");
        let pending_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM model_resets WHERE toast_dismissed_at IS NULL",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(
            pending_count, 1,
            "only the undismissed row should be pending"
        );
    }

    #[test]
    fn m22_advances_schema_version_to_at_least_22() {
        let (_tmp, conn) = fresh_conn();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(v >= 22, "expected schema version >= 22 after M22, got {v}");
    }

    #[test]
    fn m22_creates_reflection_episodes_table() {
        let (_tmp, conn) = fresh_conn();
        let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='reflection_episodes'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(
            exists, 1,
            "reflection_episodes table should exist after M22"
        );
    }

    #[test]
    fn m22_creates_council_votes_table() {
        let (_tmp, conn) = fresh_conn();
        let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='council_votes'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(exists, 1, "council_votes table should exist after M22");
    }

    #[test]
    fn m22_reflection_episodes_rejects_attempt_no_above_3() {
        let (_tmp, conn) = fresh_conn();
        let res = conn.execute(
            "INSERT INTO reflection_episodes (episode_id, mission_id, attempt_no, executor_model,
                reflexor_model, failure_signal, verbal_reflection, injected_prompt_delta, created_at)
             VALUES ('ep1', 'm1', 4, 'gpt-4o', 'haiku', 'sig', 'reflection', 'delta', '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(
            res.is_err(),
            "attempt_no=4 must be rejected by the CHECK constraint"
        );
    }

    #[test]
    fn m22_reflection_episodes_unique_mission_attempt() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO reflection_episodes (episode_id, mission_id, attempt_no, executor_model,
                reflexor_model, failure_signal, verbal_reflection, injected_prompt_delta, created_at)
             VALUES ('ep1', 'm1', 1, 'gpt-4o', 'haiku', 'sig', 'r1', 'd1', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("first insert ok");
        let dup = conn.execute(
            "INSERT INTO reflection_episodes (episode_id, mission_id, attempt_no, executor_model,
                reflexor_model, failure_signal, verbal_reflection, injected_prompt_delta, created_at)
             VALUES ('ep2', 'm1', 1, 'gpt-4o', 'haiku', 'sig2', 'r2', 'd2', '2026-01-01T01:00:00Z')",
            [],
        );
        assert!(
            dup.is_err(),
            "duplicate (mission_id, attempt_no) must be rejected by the UNIQUE constraint"
        );
    }

    #[test]
    fn m23_advances_schema_version_to_at_least_23() {
        let (_tmp, conn) = fresh_conn();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(
            v >= crate::journal::schema::CURRENT_SCHEMA_VERSION,
            "expected schema version >= CURRENT_SCHEMA_VERSION after migrate(), got {v}"
        );
    }

    #[test]
    fn m23_creates_task_classifier_decisions_table() {
        let (_tmp, conn) = fresh_conn();
        let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='task_classifier_decisions'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(
            exists, 1,
            "task_classifier_decisions table should exist after M23"
        );
    }

    #[test]
    fn m23_creates_model_affinity_cache_table() {
        let (_tmp, conn) = fresh_conn();
        let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='model_affinity_cache'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(
            exists, 1,
            "model_affinity_cache table should exist after M23"
        );
    }

    #[test]
    fn m23_task_classifier_decisions_rejects_unknown_kind() {
        let (_tmp, conn) = fresh_conn();
        let res = conn.execute(
            "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                confidence, classifier_kind, created_at)
             VALUES ('d1', 'h1', 'coding', 0.9, 'unknown_kind', '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(
            res.is_err(),
            "classifier_kind='unknown_kind' must be rejected by the CHECK constraint"
        );
    }

    #[test]
    fn m23_task_classifier_decisions_dedupes_prompt_hash_per_kind() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                confidence, classifier_kind, created_at)
             VALUES ('d1', 'h1', 'coding', 0.9, 'lexical', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("first insert ok");
        let dup = conn.execute(
            "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                confidence, classifier_kind, created_at)
             VALUES ('d2', 'h1', 'plan', 0.8, 'lexical', '2026-01-02T00:00:00Z')",
            [],
        );
        assert!(
            dup.is_err(),
            "duplicate (prompt_hash, classifier_kind) must be rejected by the UNIQUE constraint"
        );
        // Different classifier_kind on same prompt_hash is allowed.
        conn.execute(
            "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                confidence, classifier_kind, created_at)
             VALUES ('d3', 'h1', 'plan', 0.85, 'logreg', '2026-01-03T00:00:00Z')",
            [],
        )
        .expect("different kind allowed for same prompt_hash");
    }

    #[test]
    fn m23_model_affinity_cache_upsert_replaces_existing_row() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO model_affinity_cache (task_type, model_id, success_rate, p95_latency_ms,
                mean_cost_usd, n_samples, updated_at)
              VALUES ('coding', 'gpt-5', 0.85, 1200, 0.01, 12, '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("insert ok");
        conn.execute(
            "INSERT OR REPLACE INTO model_affinity_cache (task_type, model_id, success_rate,
                p95_latency_ms, mean_cost_usd, n_samples, updated_at)
              VALUES ('coding', 'gpt-5', 0.9, 1100, 0.011, 18, '2026-01-02T00:00:00Z')",
            [],
        )
        .expect("upsert ok");
        let (rate, n): (f64, i64) = conn
            .query_row(
                "SELECT success_rate, n_samples FROM model_affinity_cache
                 WHERE task_type='coding' AND model_id='gpt-5'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("query");
        assert!(
            (rate - 0.9).abs() < 1e-6,
            "upsert should overwrite success_rate"
        );
        assert_eq!(n, 18, "upsert should overwrite n_samples");
    }

    #[test]
    fn m33_task_classifier_decisions_accepts_laya_kind() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                confidence, classifier_kind, created_at)
             VALUES ('l1', 'hlaya', 'coding', 0.9, 'laya', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("classifier_kind='laya' must be accepted after M33");
        let kind: String = conn
            .query_row(
                "SELECT classifier_kind FROM task_classifier_decisions WHERE id='l1'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(kind, "laya");
    }

    #[test]
    fn m33_task_classifier_decisions_still_rejects_unknown_kind() {
        let (_tmp, conn) = fresh_conn();
        let res = conn.execute(
            "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                confidence, classifier_kind, created_at)
             VALUES ('x1', 'hx', 'coding', 0.9, 'jev', '2026-01-01T00:00:00Z')",
            [],
        );
        assert!(
            res.is_err(),
            "classifier_kind='jev' must still be rejected by the CHECK constraint"
        );
    }

    #[test]
    fn m33_task_classifier_decisions_preserves_preexisting_rows() {
        let (_tmp, conn) = fresh_conn();
        for (id, kind) in [
            ('a', "lexical"),
            ('b', "logreg"),
            ('c', "embedding"),
            ('d', "main"),
            ('e', "mf_ab"),
        ] {
            conn.execute(
                "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                    confidence, classifier_kind, created_at)
                 VALUES (?1, ?2, 'coding', 0.8, ?3, '2026-01-01T00:00:00Z')",
                rusqlite::params![format!("p{id}"), format!("h{id}"), kind],
            )
            .expect("pre-existing kind insert ok");
        }
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM task_classifier_decisions", [], |r| {
                r.get(0)
            })
            .expect("query");
        assert_eq!(
            n, 5,
            "all pre-M33 kinds must survive the M33 recreate-and-copy"
        );
    }
}

#[cfg(test)]
mod research_m25_schema_tests {
    use crate::journal::Journal;
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn fresh_conn() -> (TempDir, Connection) {
        let tmp = TempDir::new().expect("tmp");
        let _journal = Journal::open(tmp.path()).expect("open runs migrate");
        let conn = Connection::open(tmp.path().join("journal.db")).expect("open raw conn");
        (tmp, conn)
    }

    #[test]
    fn m25_advances_schema_version_to_25() {
        let (_tmp, conn) = fresh_conn();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(
            v >= crate::journal::schema::CURRENT_SCHEMA_VERSION,
            "expected schema version >= CURRENT_SCHEMA_VERSION after migrate(), got {v}"
        );
        assert!(v >= 25, "M25 must have run, got {v}");
    }

    #[test]
    fn m25_migration_is_idempotent() {
        let (tmp, _) = fresh_conn();
        let db_path = tmp.path().join("journal.db");
        let conn = Connection::open(&db_path).expect("reopen");
        crate::journal::schema::migrate(&conn).expect("second migrate ok");
        crate::journal::schema::migrate(&conn).expect("third migrate ok");
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(v >= 25, "version must stay >= 25 after re-migrate, got {v}");
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table'
                 AND name IN ('research_sources','research_consensus')",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(count, 2, "both M25 tables must exist after re-migrate");
    }

    #[test]
    fn m25_extends_research_runs_with_status_and_recommended() {
        let (_tmp, conn) = fresh_conn();
        let cols: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('research_runs')")
            .expect("prepare")
            .query_map([], |r| r.get(0))
            .expect("query")
            .flatten()
            .collect();
        for expected in ["id", "query", "status", "confidence", "recommended"] {
            assert!(
                cols.contains(&expected.to_string()),
                "research_runs must carry column {expected}, got {cols:?}"
            );
        }
    }

    #[test]
    fn m25_consensus_rejects_unknown_dimension() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO research_runs (id, query, created_at) VALUES ('rr-x','q','2026-01-01T00:00:00Z')",
            [],
        )
        .expect("seed run");
        let res = conn.execute(
            "INSERT INTO research_consensus (run_id, dimension, score) VALUES ('rr-x','press',50.0)",
            [],
        );
        assert!(
            res.is_err(),
            "dimension='press' must be rejected by the CHECK constraint"
        );
    }

    #[test]
    fn m25_consensus_rejects_score_above_100() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO research_runs (id, query, created_at) VALUES ('rr-y','q','2026-01-01T00:00:00Z')",
            [],
        )
        .expect("seed run");
        let res = conn.execute(
            "INSERT INTO research_consensus (run_id, dimension, score)
             VALUES ('rr-y','official',120.0)",
            [],
        );
        assert!(
            res.is_err(),
            "score=120 must be rejected by the 0..100 CHECK constraint"
        );
    }
}

#[cfg(test)]
mod research_m26_schema_tests {
    use crate::journal::Journal;
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn fresh_conn() -> (TempDir, Connection) {
        let tmp = TempDir::new().expect("tmp");
        let _journal = Journal::open(tmp.path()).expect("open runs migrate");
        let conn = Connection::open(tmp.path().join("journal.db")).expect("open raw conn");
        (tmp, conn)
    }

    #[test]
    fn m26_advances_schema_version_to_26() {
        let (_tmp, conn) = fresh_conn();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(
            v >= crate::journal::schema::CURRENT_SCHEMA_VERSION,
            "expected schema version >= CURRENT_SCHEMA_VERSION after migrate(), got {v}"
        );
        assert!(v >= 26, "M26 must have run, got {v}");
    }

    #[test]
    fn m26_migration_is_idempotent() {
        let (tmp, _) = fresh_conn();
        let db_path = tmp.path().join("journal.db");
        let conn = Connection::open(&db_path).expect("reopen");
        crate::journal::schema::migrate(&conn).expect("second migrate ok");
        crate::journal::schema::migrate(&conn).expect("third migrate ok");
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(v >= 26, "version must stay >= 26 after re-migrate, got {v}");
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table'
                 AND name IN ('research_notes')",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(count, 1, "M26 research_notes must exist after re-migrate");
    }

    #[test]
    fn m26_notes_carry_hands_on_columns() {
        let (_tmp, conn) = fresh_conn();
        let cols: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('research_notes')")
            .expect("prepare")
            .query_map([], |r| r.get(0))
            .expect("query")
            .flatten()
            .collect();
        for expected in [
            "id",
            "title",
            "project",
            "decision",
            "outcome",
            "confidence",
            "tags_json",
            "attached_at",
            "signature",
        ] {
            assert!(
                cols.contains(&expected.to_string()),
                "research_notes must carry column {expected}, got {cols:?}"
            );
        }
    }

    #[test]
    fn m26_notes_reject_confidence_above_one() {
        let (_tmp, conn) = fresh_conn();
        let res = conn.execute(
            "INSERT INTO research_notes
                (id, title, decision, confidence, attached_at, signature, created_at)
             VALUES ('rn-x','t','d',1.5,'2026-07-04','op','2026-07-04T00:00:00Z')",
            [],
        );
        assert!(
            res.is_err(),
            "confidence=1.5 must be rejected by the 0..1 CHECK constraint"
        );
    }
}

mod research_m27_schema_tests {
    use crate::journal::Journal;
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn fresh_conn() -> (TempDir, Connection) {
        let tmp = TempDir::new().expect("tmp");
        let _journal = Journal::open(tmp.path()).expect("open runs migrate");
        let conn = Connection::open(tmp.path().join("journal.db")).expect("open raw conn");
        (tmp, conn)
    }

    #[test]
    fn m27_advances_schema_version_to_27() {
        let (_tmp, conn) = fresh_conn();
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(
            v >= crate::journal::schema::CURRENT_SCHEMA_VERSION,
            "expected schema version >= CURRENT_SCHEMA_VERSION after migrate(), got {v}"
        );
        assert!(v >= 27, "M27 must have run, got {v}");
    }

    #[test]
    fn m27_migration_is_idempotent() {
        let (tmp, _) = fresh_conn();
        let db_path = tmp.path().join("journal.db");
        let conn = Connection::open(&db_path).expect("reopen");
        crate::journal::schema::migrate(&conn).expect("second migrate ok");
        crate::journal::schema::migrate(&conn).expect("third migrate ok");
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(v >= 27, "version must stay >= 27 after re-migrate, got {v}");
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table'
                 AND name IN ('feasibility_cache')",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(
            count, 1,
            "M27 feasibility_cache must exist after re-migrate"
        );
    }

    #[test]
    fn m27_cache_carries_probe_columns() {
        let (_tmp, conn) = fresh_conn();
        let cols: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('feasibility_cache')")
            .expect("prepare")
            .query_map([], |r| r.get(0))
            .expect("query")
            .flatten()
            .collect();
        for expected in ["cache_key", "topic", "domains", "report_json", "created_at"] {
            assert!(
                cols.contains(&expected.to_string()),
                "feasibility_cache must carry column {expected}, got {cols:?}"
            );
        }
    }

    #[test]
    fn m27_cache_refresh_overwrites_same_key() {
        let (_tmp, conn) = fresh_conn();
        conn.execute(
            "INSERT INTO feasibility_cache
                (cache_key, topic, domains, report_json, created_at)
             VALUES ('k','t','software','{}','2026-07-13T14:02:00+00:00')",
            [],
        )
        .expect("first insert");
        conn.execute(
            "INSERT OR REPLACE INTO feasibility_cache
                (cache_key, topic, domains, report_json, created_at)
             VALUES ('k','t','software','{\"found\":true}','2026-07-14T14:02:00+00:00')",
            [],
        )
        .expect("refresh");
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM feasibility_cache WHERE cache_key = 'k'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(count, 1, "cache refresh must replace, never duplicate");
    }
}

#[cfg(test)]
mod journal_phase80_tests {
    use crate::core::bus::{BusEvent, BusEventKind};
    use crate::journal::Journal;
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn open() -> (TempDir, Journal) {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open runs migrate");
        (tmp, journal)
    }

    fn publish(journal: &Journal, prompt: &str) {
        let evt = BusEvent::new(BusEventKind::TaskReceived {
            raw_prompt: prompt.into(),
            session_id: uuid::Uuid::new_v4(),
        });
        journal.publish(&evt).expect("publish");
    }

    #[test]
    fn m31_advances_schema_version_and_creates_dir_access() {
        let (tmp, _) = open();
        let conn = Connection::open(tmp.path().join("journal.db")).expect("raw conn");
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(
            v >= crate::journal::schema::CURRENT_SCHEMA_VERSION,
            "expected version >= CURRENT after migrate(), got {v}"
        );
        assert!(v >= 31, "M31 must have run, got {v}");
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='dir_access'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(count, 1, "M31 dir_access must exist");
    }

    #[test]
    fn m31_migration_is_idempotent() {
        let (tmp, _) = open();
        let db_path = tmp.path().join("journal.db");
        let conn = Connection::open(&db_path).expect("reopen");
        crate::journal::schema::migrate(&conn).expect("second migrate ok");
        crate::journal::schema::migrate(&conn).expect("third migrate ok");
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(v >= 31, "version must stay >= 31 after re-migrate, got {v}");
    }

    #[test]
    fn search_round_trip_finds_published_payload() {
        let (_tmp, journal) = open();
        publish(&journal, "migrate the unicorn billing pipeline");
        publish(&journal, "something entirely unrelated here");
        let hits = journal
            .search_events("unicorn billing", 10)
            .expect("search");
        assert_eq!(hits.len(), 1, "only the matching event must hit");
        assert_eq!(hits[0].kind, "task_received");
    }

    #[test]
    fn search_with_fts_operators_falls_back_to_like_without_error() {
        let (_tmp, journal) = open();
        publish(&journal, "plain event about caching");
        let hits = journal.search_events("*** (((", 10).expect("search");
        assert!(
            hits.is_empty(),
            "operator-only input must degrade to LIKE with no hits, not an error"
        );
        let hits = journal.search_events("caching", 10).expect("search");
        assert_eq!(hits.len(), 1, "LIKE fallback must still match 'caching'");
    }

    #[test]
    fn search_empty_query_returns_empty_without_error() {
        let (_tmp, journal) = open();
        publish(&journal, "anything");
        let hits = journal.search_events("   ", 10).expect("search");
        assert!(hits.is_empty());
    }

    #[test]
    fn record_dir_access_upserts_counter_and_frecency_ranks() {
        let (_tmp, journal) = open();
        journal
            .record_dir_access("/wts/mission-a/backend")
            .expect("rec");
        journal
            .record_dir_access("/wts/mission-a/backend")
            .expect("rec");
        journal
            .record_dir_access("/wts/mission-a/backend")
            .expect("rec");
        journal
            .record_dir_access("/wts/mission-b/frontend")
            .expect("rec");
        let hits = journal.frecency("", 10).expect("frecency");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].0, "/wts/mission-a/backend");
        assert!(
            hits[0].1 > hits[1].1,
            "3 accesses must outrank 1 at equal recency"
        );
    }

    #[test]
    fn frecency_prefix_filters_and_aging_decay_applies() {
        let (_tmp, journal) = open();
        journal.record_dir_access("/wts/alpha").expect("rec");
        journal.record_dir_access("/wts/beta").expect("rec");
        let hits = journal.frecency("alpha", 10).expect("frecency");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, "/wts/alpha");

        let stale = crate::journal::frecency::frecency_score(4, 0, 20_000_000_000);
        let fresh = crate::journal::frecency::frecency_score(1, 19_999_999_000, 20_000_000_000);
        assert!(
            fresh > stale,
            "aging decay must let a fresh dir beat a stale 4x-used one (4.0 vs 1.0)"
        );
    }

    #[test]
    fn record_dir_access_rejects_empty_dir() {
        let (_tmp, journal) = open();
        assert!(journal.record_dir_access("   ").is_err());
        let hits = journal.frecency("", 10).expect("frecency");
        assert!(hits.is_empty());
    }
}
