// OpenCode OS — Planning Engine tests (RFC 12).
//
// Phase 1: behavioural tests for the heuristic runner. The contract under
// test is the RFC 12 §7 gate (`verdict.confidence` floor + `Plan.confidence`
// threshold), the §2 `mission.locked` handoff, and the round-trip through
// the Journal `plans` table. The cases mirror the runner_tests already
// living under `prompt::tests` so a regression report reads top-to-bottom
// through the kernel.

#[cfg(test)]
mod fixtures {
    use crate::prompt::types::{
        AcceptedAssumption, AssumptionSource, ConfidenceLevel, ConfidenceRubric, DesiredAction,
        Domain, IntentHypothesis, LockSource, MissionConsolidated, NamedEntity,
        PublicUnderstandingVerdict, RecommendedMode, Scope,
    };
    use uuid::Uuid;

    pub fn make_verdict(raw: &str, confidence: ConfidenceLevel) -> PublicUnderstandingVerdict {
        PublicUnderstandingVerdict {
            verdict_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            raw_prompt: raw.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            intent: format!("Plan-worthy intent for {raw:?}"),
            intent_hypotheses: vec![IntentHypothesis {
                rank: 1,
                text: format!("intent for {raw}"),
                feasibility_score: 0.7,
                rejection_reason: None,
            }],
            keys: vec!["rating_service".into(), "compute_weight".into()],
            named_entities: vec![NamedEntity {
                text: "src/lib.rs".into(),
                r#type: "file_path".into(),
                in_kb: false,
            }],
            domain: Domain::Backend,
            technology: vec!["rust".into(), "axum".into()],
            desired_action: DesiredAction::Modify,
            scope: Scope::Module,
            implicit_signals: vec![],
            gaps: vec![],
            similar_missions: vec![],
            clarification_questions: vec![],
            confidence,
            confidence_rubric: ConfidenceRubric {
                intent_clarity: 0.9,
                scope_clarity: 0.85,
                feasibility_clarity: 0.8,
                context_clarity: 0.85,
            },
            observations: vec![],
            recommended_mode: RecommendedMode::Code,
            model_id: "heuristic-v0".into(),
            judge_model_id: None,
            elapsed_ms: 0,
        }
    }

    pub fn make_consolidated(
        verdict: &PublicUnderstandingVerdict,
        locked: bool,
        locked_by: LockSource,
    ) -> MissionConsolidated {
        let _ = AcceptedAssumption {
            assumption: String::new(),
            user_confirmed: false,
            auto_resolved: false,
            source: AssumptionSource::ArchitectureMemory,
        };
        MissionConsolidated {
            mission_id: Uuid::new_v4(),
            verdict_id: verdict.verdict_id,
            generated_at: chrono::Utc::now().to_rfc3339(),
            mission_statement: "Migrate the rating service to sqlite-vec.".into(),
            success_criteria: vec!["`cargo test -p opencode-os` passes after edits.".into()],
            non_goals: vec![],
            accepted_assumptions: vec![],
            suggested_mode: RecommendedMode::Code,
            suggested_tooling: vec!["rust".into(), "axum".into()],
            forbidden_actions: vec![],
            requires_research_first: false,
            research_queries: None,
            locked,
            locked_at: if locked {
                Some(chrono::Utc::now().to_rfc3339())
            } else {
                None
            },
            locked_by,
            planning_session_id: None,
            execution_session_id: None,
        }
    }
}

#[cfg(test)]
mod runner_tests {
    use super::super::runner::{run, PlanningInput};
    use super::fixtures::{make_consolidated, make_verdict};
    use crate::planning::types::{BlockerKind, Impact, Plan, Strategy, PLAN_CONFIDENCE_THRESHOLD};
    use crate::prompt::types::{
        ConfidenceLevel, Gap, GapType, LockSource, MissionConsolidated, PublicUnderstandingVerdict,
        RecommendedMode, Scope, Severity,
    };

    fn happy_input<'a>(
        verdict: &'a PublicUnderstandingVerdict,
        consolidated: &'a MissionConsolidated,
    ) -> PlanningInput<'a> {
        PlanningInput {
            consolidated,
            verdict,
            clarification_answers: vec![],
            research_runs: vec![],
        }
    }

    #[test]
    fn happy_path_high_confidence_locked_mission_emits_unblocked_code_plan() {
        let v = make_verdict(
            "migrate `rating_service` to sqlite-vec",
            ConfidenceLevel::High,
        );
        let c = make_consolidated(&v, true, LockSource::AutoThreshold);
        let plan: Plan = run(&happy_input(&v, &c)).expect("run");
        assert!(
            plan.blocked.is_empty(),
            "no blockers expected; got {:?}",
            plan.blocked
        );
        assert!(
            plan.confidence >= PLAN_CONFIDENCE_THRESHOLD,
            "confidence {} >= 0.7",
            plan.confidence
        );
        assert_eq!(plan.mission_id, c.mission_id);
        assert_eq!(plan.verdict_id, v.verdict_id);
        // RFC 12 §2.2 — `code` mode + Backend + Module → TDD.
        assert_eq!(plan.strategy, Strategy::Tdd, "code mode + backend → TDD");
        assert!(!plan.steps.is_empty());
        // Module scope + TDD backend mod → Major (conservative escalation
        // per RFC 12 §4 — the runner counts editable Test + impl steps and
        // bumps risk accordingly).
        assert_eq!(
            plan.impact,
            Impact::Major,
            "module + multi-step edits → Major"
        );
        // TDD inverts the first objective: at least one test step precedes an
        // editable step.
        assert!(
            plan.steps
                .iter()
                .any(|s| matches!(s.action, crate::planning::types::StepAction::Test)),
            "TDD plan must include at least one Test step"
        );
    }

    #[test]
    fn unlocked_mission_blocks_planning_with_mission_not_locked() {
        let v = make_verdict("ship migration", ConfidenceLevel::High);
        let c = make_consolidated(&v, false, LockSource::AutoThreshold);
        let plan = run(&happy_input(&v, &c)).expect("run");
        assert!(plan
            .blocked
            .iter()
            .any(|b| b.kind == BlockerKind::MissionNotLocked));
    }

    #[test]
    fn low_verdict_blocks_planning_with_verdict_too_low() {
        let v = make_verdict("ship migration", ConfidenceLevel::Low);
        let c = make_consolidated(&v, true, LockSource::AutoThreshold);
        let plan = run(&happy_input(&v, &c)).expect("run");
        assert!(plan
            .blocked
            .iter()
            .any(|b| b.kind == BlockerKind::VerdictTooLow));
        // The Coding Engine cannot dispatch — the lower bound threshold is
        // also violated; ensure PlanConfidenceBelowThreshold also fires.
        assert!(plan
            .blocked
            .iter()
            .any(|b| b.kind == BlockerKind::PlanConfidenceBelowThreshold));
    }

    #[test]
    fn block_verdict_force_locked_by_user_still_emits_verdict_too_low_blocker() {
        let v = make_verdict("garbage prompt", ConfidenceLevel::Block);
        let c = make_consolidated(&v, true, LockSource::User);
        // AutoThreshold would have aborted mission-locking entirely.
        let plan = run(&happy_input(&v, &c)).expect("run");
        assert!(
            plan.blocked
                .iter()
                .any(|b| b.kind == BlockerKind::VerdictTooLow),
            "user override generates the plan but keeps the audit blocker"
        );
    }

    #[test]
    fn missing_clarification_blocks_planning() {
        // Build a verdict with a single non-auto-resolvable gap and no
        // matching answer.
        let mut v = make_verdict(
            "fix the bug in `rating_service` that overflows",
            ConfidenceLevel::High,
        );
        v.gaps = vec![Gap {
            kind: GapType::Underspecified,
            evidence: "fix the bug".into(),
            severity: Severity::High,
            auto_resolvable: false,
        }];
        let c = make_consolidated(&v, true, LockSource::AutoThreshold);
        let plan = run(&happy_input(&v, &c)).expect("run");
        assert!(plan
            .blocked
            .iter()
            .any(|b| b.kind == BlockerKind::MissingClarification));
    }

    #[test]
    fn missing_research_blocks_planning_when_requires_research_first() {
        let v = make_verdict("use exotic 3d printer sdk", ConfidenceLevel::High);
        let c = MissionConsolidated {
            requires_research_first: true,
            ..make_consolidated(&v, true, LockSource::AutoThreshold)
        };
        let plan = run(&happy_input(&v, &c)).expect("run");
        assert!(plan
            .blocked
            .iter()
            .any(|b| b.kind == BlockerKind::MissingResearch));
    }

    #[test]
    fn ask_mode_produces_read_only_steps_and_incremental_strategy() {
        let v = make_verdict("explain the rating module", ConfidenceLevel::High);
        let c = MissionConsolidated {
            suggested_mode: RecommendedMode::Ask,
            ..make_consolidated(&v, true, LockSource::AutoThreshold)
        };
        let plan = run(&happy_input(&v, &c)).expect("run");
        assert_eq!(plan.strategy, Strategy::Incremental);
        assert!(
            plan.steps.iter().all(|s| s.read_only),
            "ask mode produces only read-only steps"
        );
    }

    #[test]
    fn project_scope_yields_big_bang_strategy_single_milestone() {
        let mut v = make_verdict("repo-wide migration", ConfidenceLevel::High);
        v.scope = Scope::Project;
        let c = MissionConsolidated {
            suggested_mode: RecommendedMode::Code,
            ..make_consolidated(&v, true, LockSource::AutoThreshold)
        };
        let plan = run(&happy_input(&v, &c)).expect("run");
        assert_eq!(plan.strategy, Strategy::BigBang);
        assert_eq!(plan.roadmap.len(), 1, "big bang collapses milestones");
    }
}

#[cfg(test)]
mod journal_plan_persistence_tests {
    use super::super::runner::{run, PlanningInput};
    use super::fixtures::{make_consolidated, make_verdict};
    use crate::journal::Journal;
    use crate::planning::types::Plan;
    use crate::prompt::types::{ConfidenceLevel, LockSource};
    use tempfile::TempDir;

    fn happy_plan() -> Plan {
        let v = make_verdict(
            "migrate `rating_service` to sqlite-vec",
            ConfidenceLevel::High,
        );
        let c = make_consolidated(&v, true, LockSource::AutoThreshold);
        run(&PlanningInput {
            consolidated: &c,
            verdict: &v,
            clarification_answers: vec![],
            research_runs: vec![],
        })
        .expect("run")
    }

    #[test]
    fn plan_round_trips_through_journal_and_tail_lists_it() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let plan = happy_plan();
        journal.save_plan(&plan).expect("save");
        let tail = journal.plan_tail(10).expect("tail");
        assert_eq!(tail.len(), 1);
        let row = &tail[0];
        assert_eq!(row.plan_id, plan.plan_id);
        assert_eq!(row.mission_id, plan.mission_id);
        assert_eq!(row.verdict_id, plan.verdict_id);
        assert_eq!(row.strategy, plan.strategy.tag());
        assert!((row.confidence - plan.confidence).abs() < 1e-6);
        assert_eq!(row.blocker_count, plan.blocked.len() as i64);

        let raw = journal
            .plan_payload(plan.plan_id)
            .expect("payload")
            .unwrap();
        let back: Plan = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.plan_id, plan.plan_id);
        assert_eq!(back.mission_id, plan.mission_id);
        assert_eq!(back.strategy, plan.strategy);
        assert_eq!(back.blocked.len(), plan.blocked.len());
    }

    #[test]
    fn save_plan_is_idempotent_on_replay() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let plan = happy_plan();
        journal.save_plan(&plan).expect("first save");
        // Replay with a mutated payload — the original row MUST win per
        // RFC 02 §3.1.2 (first-write-wins).
        let mut replayed = plan.clone();
        replayed.confidence = 0.99;
        journal.save_plan(&replayed).expect("replay save");
        let row = journal.plan_tail(1).expect("tail")[0].clone();
        assert!(
            (row.confidence - plan.confidence).abs() < 1e-6,
            "first write wins; replay did NOT overwrite"
        );
    }

    #[test]
    fn latest_plan_for_mission_returns_newest() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let p1 = happy_plan();
        journal.save_plan(&p1).expect("save p1");
        // Same mission, different plan id.
        let mut p2 = p1.clone();
        p2.plan_id = uuid::Uuid::new_v4();
        // SPACE so generated_at strictly increases; save_plan uses the
        // struct's ts, not CURRENT_TIMESTAMP, so we need to bump it.
        p2.generated_at = chrono::Utc::now().to_rfc3339();
        std::thread::sleep(std::time::Duration::from_millis(5));
        journal.save_plan(&p2).expect("save p2");
        let latest = journal
            .latest_plan_for_mission(p1.mission_id)
            .expect("latest")
            .expect("some row");
        assert_eq!(latest.plan_id, p2.plan_id);
    }
}
