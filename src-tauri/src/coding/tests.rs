// OpenCode OS — Coding Engine tests (RFC 13).
//
// Phase 1: behavioural tests for the heuristic runner. Covers the two
// return shapes (`Emitted` / `Rejected`) and the four rejection reasons
// (RFC 12 §7 Plan gate, RFC 13 §2 read-only step, RFC 13 §3 unapproved
// skill, RFC 13 §8 overlapping hunks). Plus the round-trip through the
// Journal `diffs` table.

#[cfg(test)]
mod runner_tests {
    use super::super::runner::{run, CodingInput, WorkspaceFile};
    use super::fixtures::{make_code_step, make_creatable_plan};
    use crate::coding::types::{ApprovedSkill, CodingOutcome, CodingRejection};
    use crate::planning::types::{Blocker, BlockerKind, Plan, Step, StepAction};
    use uuid::Uuid;

    fn base_input<'a>(plan: &'a Plan, step: &'a Step) -> CodingInput<'a> {
        CodingInput {
            plan,
            step,
            agent_id: Uuid::new_v4(),
            approved_skills: vec![],
            workspace_files: vec![WorkspaceFile {
                path: "src/lib.rs".into(),
                lines: vec!["fn existing() {}".into()],
            }],
            narrative: "what it did / what it does now / why".into(),
            research_refs: vec![],
        }
    }

    #[test]
    fn happy_path_create_step_emits_diff_with_module_doc() {
        let plan = make_creatable_plan();
        let step = make_code_step(StepAction::Create);
        let outcome = run(&base_input(&plan, &step)).expect("run");
        let diff = match outcome {
            CodingOutcome::Emitted { diff } => *diff,
            other => panic!("expected Emitted, got {other:?}"),
        };
        assert_eq!(diff.plan_id, plan.plan_id);
        assert_eq!(diff.mission_id, plan.mission_id);
        assert_eq!(diff.step_id, step.id);
        assert!(!diff.files.is_empty());
        let edit = &diff.files[0];
        assert!(edit.is_new_file, "Create action → new_file = true");
        assert!(edit.hunks_are_disjoint_and_sorted());
        assert!(!edit.hunks[0].new_lines.is_empty());
        // RFC 13 §5 — first lines must be the module-level doc.
        assert!(
            edit.hunks[0].new_lines[0].contains("//!"),
            "module doc emitted"
        );
        assert!(!diff.narrative.is_empty(), "narrative carried verbatim");
    }

    #[test]
    fn modify_step_appends_sentinel_hunk_after_existing_rows() {
        let plan = make_creatable_plan();
        let step = make_code_step(StepAction::Modify);
        let outcome = run(&base_input(&plan, &step)).expect("run");
        let diff = match outcome {
            CodingOutcome::Emitted { diff } => *diff,
            other => panic!("expected Emitted, got {other:?}"),
        };
        let edit = &diff.files[0];
        assert!(!edit.is_new_file);
        // Workspace file has 1 line; the appended hunk starts at line 1
        // (0-indexed) → old_start == old_end == 1.
        assert_eq!(edit.hunks[0].old_start, 1);
        assert_eq!(edit.hunks[0].old_end, 1);
        assert_eq!(edit.hunks[0].new_lines.len(), 1);
        assert!(edit.hunks[0].new_lines[0].contains("todo"));
    }

    #[test]
    fn test_step_emits_failing_test_stub_for_tdd_red_state() {
        let plan = make_creatable_plan();
        let step = make_code_step(StepAction::Test);
        let outcome = run(&base_input(&plan, &step)).expect("run");
        let diff = match outcome {
            CodingOutcome::Emitted { diff } => *diff,
            other => panic!("expected Emitted, got {other:?}"),
        };
        let edit = &diff.files[0];
        assert!(edit.is_new_file);
        assert!(edit.path.starts_with("tests/"));
        assert!(edit.hunks[0].new_lines[0].contains("#[test]"));
        assert!(edit.hunks[0].new_lines[0].contains("assert!(false"));
    }

    #[test]
    fn plan_below_confidence_threshold_is_rejected() {
        let mut plan = make_creatable_plan();
        plan.confidence = 0.5; // < PLAN_CONFIDENCE_THRESHOLD (0.7)
        plan.blocked.push(Blocker {
            kind: BlockerKind::PlanConfidenceBelowThreshold,
            message: "below threshold".into(),
        });
        let step = make_code_step(StepAction::Modify);
        let outcome = run(&base_input(&plan, &step)).expect("run");
        assert_rejected(outcome, CodingRejection::PlanBlocked);
    }

    #[test]
    fn read_only_step_is_rejected() {
        let plan = make_creatable_plan();
        let step = Step {
            id: "S-read_only".into(),
            milestone_id: "M1".into(),
            statement: "research-only step".into(),
            action: StepAction::Research,
            depends_on: vec![],
            skills: vec![],
            models: vec![],
            read_only: true,
        };
        let outcome = run(&base_input(&plan, &step)).expect("run");
        assert_rejected(outcome, CodingRejection::ReadOnlyStep);
    }

    #[test]
    fn unapproved_skill_is_rejected() {
        let plan = make_creatable_plan();
        let step = make_code_step(StepAction::Modify);
        let mut input = base_input(&plan, &step);
        // Inject a skill id absent from plan.skills_used.
        input.approved_skills.push(ApprovedSkill {
            skill_id: "rogue-skill".into(),
            version: "v0".into(),
        });
        let outcome = run(&input).expect("run");
        assert_rejected(outcome, CodingRejection::SkillNotApproved);
    }

    #[test]
    fn breaking_plan_emits_risk_decision_in_diff() {
        let mut plan = make_creatable_plan();
        plan.impact = crate::planning::types::Impact::Breaking;
        let step = make_code_step(StepAction::Modify);
        let outcome = run(&base_input(&plan, &step)).expect("run");
        let diff = match outcome {
            CodingOutcome::Emitted { diff } => *diff,
            other => panic!("expected Emitted, got {other:?}"),
        };
        assert!(
            diff.risk_decision.is_some(),
            "breaking plan carries risk_decision"
        );
        assert!(diff.risk_decision.unwrap().contains("impact=breaking"));
    }

    fn assert_rejected(outcome: CodingOutcome, expected: CodingRejection) {
        match outcome {
            CodingOutcome::Rejected { reason } if reason == expected => {}
            other => panic!("expected Rejected({expected:?}), got {other:?}"),
        }
    }
}

#[cfg(test)]
mod hunks_tests {
    use super::super::types::{FileEdit, Hunk};

    #[test]
    fn lines_added_and_removed_count_across_hunks() {
        let edit = FileEdit {
            path: "src/x.rs".into(),
            is_new_file: false,
            is_delete: false,
            hunks: vec![
                Hunk {
                    old_start: 0,
                    old_end: 2,
                    new_lines: vec!["a".into(), "b".into(), "c".into()],
                    rationale: "r1".into(),
                },
                Hunk {
                    old_start: 5,
                    old_end: 5,
                    new_lines: vec!["d".into()],
                    rationale: "r2".into(),
                },
                Hunk {
                    old_start: 10,
                    old_end: 12,
                    new_lines: vec![],
                    rationale: "pure delete".into(),
                },
            ],
        };
        assert_eq!(edit.lines_added(), 4); // 3 + 1 + 0
        assert_eq!(edit.lines_removed(), 4); // 2 + 0 + 2
        assert!(edit.hunks_are_disjoint_and_sorted());
    }

    #[test]
    fn overlapping_hunks_are_detected() {
        let edit = FileEdit {
            path: "src/x.rs".into(),
            is_new_file: false,
            is_delete: false,
            hunks: vec![
                Hunk {
                    old_start: 0,
                    old_end: 5,
                    new_lines: vec!["a".into()],
                    rationale: "r1".into(),
                },
                Hunk {
                    old_start: 3,
                    old_end: 4,
                    new_lines: vec!["b".into()],
                    rationale: "r2".into(),
                },
            ],
        };
        assert!(!edit.hunks_are_disjoint_and_sorted());
    }

    #[test]
    fn unsorted_hunks_are_detected() {
        let edit = FileEdit {
            path: "src/x.rs".into(),
            is_new_file: false,
            is_delete: false,
            hunks: vec![
                Hunk {
                    old_start: 5,
                    old_end: 6,
                    new_lines: vec!["a".into()],
                    rationale: "r1".into(),
                },
                Hunk {
                    old_start: 0,
                    old_end: 1,
                    new_lines: vec!["b".into()],
                    rationale: "r2".into(),
                },
            ],
        };
        assert!(!edit.hunks_are_disjoint_and_sorted());
    }
}

#[cfg(test)]
mod journal_diff_persistence_tests {
    use super::super::runner::{run, CodingInput};
    use super::fixtures::{make_code_step, make_creatable_plan};
    use crate::coding::types::CodingOutcome;
    use crate::journal::Journal;
    use crate::planning::types::StepAction;
    use tempfile::TempDir;

    fn emit_one() -> crate::coding::types::Diff {
        let plan = make_creatable_plan();
        let step = make_code_step(StepAction::Create);
        let outcome = run(&CodingInput {
            plan: &plan,
            step: &step,
            agent_id: uuid::Uuid::new_v4(),
            approved_skills: vec![],
            workspace_files: vec![],
            narrative: "happy path".into(),
            research_refs: vec![],
        })
        .expect("run");
        match outcome {
            CodingOutcome::Emitted { diff } => *diff,
            other => panic!("expected Emitted, got {other:?}"),
        }
    }

    #[test]
    fn diff_round_trips_through_journal_and_tail_lists_it() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let diff = emit_one();
        journal.save_diff(&diff).expect("save");

        let tail = journal.diff_tail(10).expect("tail");
        assert_eq!(tail.len(), 1);
        let row = &tail[0];
        assert_eq!(row.diff_id, diff.diff_id);
        assert_eq!(row.plan_id, diff.plan_id);
        assert_eq!(row.mission_id, diff.mission_id);
        assert_eq!(row.step_id, diff.step_id);
        assert_eq!(row.agent_id, diff.agent_id);
        let expected_added: i64 = diff.files.iter().map(|f| f.lines_added() as i64).sum();
        let expected_removed: i64 = diff.files.iter().map(|f| f.lines_removed() as i64).sum();
        assert_eq!(row.lines_added, expected_added);
        assert_eq!(row.lines_removed, expected_removed);
        assert_eq!(row.file_count, diff.files.len() as i64);

        let raw = journal
            .diff_payload(diff.diff_id)
            .expect("payload")
            .unwrap();
        let back: crate::coding::types::Diff = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.diff_id, diff.diff_id);
        assert_eq!(back.files.len(), diff.files.len());
        assert_eq!(back.narrative, diff.narrative);
    }

    #[test]
    fn save_diff_is_idempotent_on_replay() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let diff = emit_one();
        journal.save_diff(&diff).expect("first save");
        // Replay with mutated narrative — must NOT overwrite.
        let mut replayed = diff.clone();
        replayed.narrative = "ATTACK".into();
        journal.save_diff(&replayed).expect("replay save");
        let row = journal.diff_tail(1).expect("tail")[0].clone();
        let raw = journal
            .diff_payload(diff.diff_id)
            .expect("payload")
            .unwrap();
        let back: crate::coding::types::Diff = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.narrative, diff.narrative, "first write wins");
        // row.file_count is computed from the persisted payload rows
        // (no replacement) — sanity check it is unchanged.
        assert_eq!(row.file_count, diff.files.len() as i64);
    }
}

#[cfg(test)]
mod fixtures {
    use crate::planning::types::{
        Impact, Milestone, ModelRef, ModelTier, Objective, Plan, SkillRef, Step, StepAction,
        Strategy, VerificationCriterion, VerificationKind, PLAN_CONFIDENCE_THRESHOLD,
    };
    use uuid::Uuid;

    /// A plan that PASSES the RFC 12 §7 gate (confidence >= 0.7, no
    /// blockers) so the Coding Engine reaches the heuristic file producer.
    pub fn make_creatable_plan() -> Plan {
        Plan {
            plan_id: Uuid::new_v4(),
            mission_id: Uuid::new_v4(),
            verdict_id: Uuid::new_v4(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            mission: "ship migration to sqlite-vec".into(),
            objectives: vec![Objective {
                id: "O0".into(),
                statement: "Migrate the rating service to sqlite-vec.".into(),
                verifiable_via: vec![VerificationCriterion {
                    kind: VerificationKind::Test,
                    description: "tests pass".into(),
                }],
                depends_on: vec![],
            }],
            steps: vec![],
            strategy: Strategy::Incremental,
            risk: 0.2,
            impact: Impact::Minor,
            roadmap: vec![Milestone {
                id: "M1".into(),
                label: "first milestone".into(),
                objectives: vec!["O0".into()],
                depends_on: vec![],
            }],
            skills_used: vec![SkillRef {
                skill_id: "rust-edit".into(),
                version: "v0".into(),
            }],
            models_needed: vec![ModelRef {
                model_id: "heuristic-backend".into(),
                provider: "local".into(),
                tier: ModelTier::Local,
            }],
            research_runs: vec![],
            confidence: PLAN_CONFIDENCE_THRESHOLD + 0.1, // >= 0.7
            resume_point: "S0".into(),
            blocked: vec![],
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        }
    }

    pub fn make_code_step(action: StepAction) -> Step {
        Step {
            id: format!("S-{:?}", action),
            milestone_id: "M1".into(),
            statement: "do the thing".into(),
            action,
            depends_on: vec![],
            skills: vec![],
            models: vec![],
            read_only: false,
        }
    }
}
