// Atlas OS — Planning Engine runner (RFC 12 §2, §7).
//
// The runner is a free function over a locked `MissionConsolidated` plus its
// associated `PublicUnderstandingVerdict`. It produces a `Plan` even when
// the plan is rejected (the `blocked[]` array carries the rejection
// reason) so the HUD + Journal can render the rejection path without a
// separate error channel.
//
// Phase 1 is heuristic: no LLM call. The skill/model lists come from
// keyword-driven guesses off the verdict `technology[]` and `domain`. The
// real planner (LLM-driven) will keep the same signature so the rest of
// the kernel is forward-compatible (Plan is the cross-RFC contract).

use std::time::Instant;
use uuid::Uuid;

use crate::planning::types::{
    Blocker, BlockerKind, Impact, Milestone, ModelRef, ModelTier, Objective, Plan, RRRef, SkillRef,
    Step, StepAction, Strategy, VerificationCriterion, VerificationKind, PLAN_CONFIDENCE_THRESHOLD,
    VERDICT_CONFIDENCE_FLOOR,
};
use crate::prompt::types::{
    ConfidenceLevel, DesiredAction, Domain, LockSource, MissionConsolidated,
    PublicUnderstandingVerdict, RecommendedMode, Scope,
};

/// Inputs the runner needs to fence both RFC 12 §2 (mission.locked) and RFC
/// 12 §7 (verdict.confidence guard). The verdict is passed separately
/// from the consolidated because they are two distinct RFC contracts and
/// Planning is the first engine that consumes *both* — keeping them apart
/// in the API means a future IPC handler can fetch them independently from
/// the Journal (`prompt_verdicts` / `mission_consolidated` tables).
#[derive(Clone, Debug)]
pub struct PlanningInput<'a> {
    pub consolidated: &'a MissionConsolidated,
    pub verdict: &'a PublicUnderstandingVerdict,
    /// User-supplied clarification answers. The Planning Engine refuses to
    /// emit a `code` plan if any gap with `auto_resolvable == false` has
    /// no matching answer key here (RFC 12 §2 —
    /// `task.rejected: reason=missing_clarification`).
    pub clarification_answers: Vec<ClarificationAnswer>,
    /// Research-run reports attached to the mission (RFC 10). Empty in
    /// Phase 1 (Phase 3 will populate this); only consulted when
    /// `consolidated.requires_research_first == true`.
    pub research_runs: Vec<AttachedResearchRun>,
}

#[derive(Clone, Debug)]
pub struct ClarificationAnswer {
    pub perspective_tag: String,
    pub question_id: String,
    pub answer: String,
}

#[derive(Clone, Debug)]
pub struct AttachedResearchRun {
    pub research_run_id: Uuid,
    pub outcome_tag: String,
}

/// Outcome of `planning::run`. `Plan` is always returned (even on rejection)
/// — the caller inspects `plan.blocked` to decide whether to forward to
/// the Coding Engine or surface the rejection in the HUD. The `PlanGenerated`
/// Kernel Bus event (RFC 02 §3.1) is only emitted when `plan.blocked.is_empty()`.
pub fn run(input: &PlanningInput) -> anyhow::Result<Plan> {
    let started = Instant::now();
    let mut blockers: Vec<Blocker> = Vec::new();

    let consolidated = input.consolidated;
    let verdict = input.verdict;

    // RFC 12 §2 — mission must be locked.
    if !consolidated.locked {
        blockers.push(Blocker {
            kind: BlockerKind::MissionNotLocked,
            message: "MissionConsolidated.locked == false; Planning requires a locked mission."
                .into(),
        });
    }

    // RFC 12 §2 — missing clarification answers for non-auto-resolvable gaps.
    let missing_clarification = missing_clarification(verdict, &input.clarification_answers);
    if missing_clarification {
        blockers.push(Blocker {
            kind: BlockerKind::MissingClarification,
            message: "One or more gaps with auto_resolvable=false have no user answer.".into(),
        });
    }

    // RFC 12 §7 — verdict confidence gate. We always read the guard from
    // the verdict (the Planning Engine never replays a verdict). When the
    // mission was force-locked by the user (`locked_by = User`), the plan
    // is still generated so the override is auditable, but the
    // `VerdictTooLow` blocker stays so Coding cannot proceed.
    let verdict_below_floor =
        confidence_rank(verdict.confidence) < confidence_rank(VERDICT_CONFIDENCE_FLOOR);
    let user_override = matches!(consolidated.locked_by, LockSource::User);
    if verdict_below_floor && !user_override {
        blockers.push(Blocker {
            kind: BlockerKind::VerdictTooLow,
            message: format!(
                "verdict.confidence = {:?}; Planning requires at least {:?}.",
                verdict.confidence, VERDICT_CONFIDENCE_FLOOR
            ),
        });
    } else if verdict_below_floor && user_override {
        blockers.push(Blocker {
            kind: BlockerKind::VerdictTooLow,
            message: "verdict.confidence below floor but mission force-locked by user; blocker kept until refine or override release."
                .into(),
        });
    }

    // RFC 12 §2 — requires_research_first gate. If the consolidated asked
    // for research first and no research runs are attached, the planner
    // cannot safely pick a strategy. The plan is still emitted with a
    // read-first milestone; Coding refuses because of the blocker.
    if consolidated.requires_research_first && input.research_runs.is_empty() {
        blockers.push(Blocker {
            kind: BlockerKind::MissingResearch,
            message:
                "MissionConsolidated.requires_research_first = true but no ResearchRun attached."
                    .into(),
        });
    }

    let strategy = pick_strategy(consolidated, verdict, &blockers);
    let objectives = derive_objectives(consolidated, verdict, &strategy);
    let milestones = derive_milestones(&objectives, &strategy);
    let steps = derive_steps(&objectives, &milestones, &strategy, consolidated, verdict);
    let risk = compute_risk(consolidated, verdict, &steps);
    let impact = pick_impact(consolidated, verdict, risk);
    let skills_used = suggest_skills(verdict);
    let models_needed = suggest_models(verdict, &strategy);
    let research_runs = input
        .research_runs
        .iter()
        .map(|r| RRRef {
            research_run_id: r.research_run_id,
            outcome_tag: r.outcome_tag.clone(),
        })
        .collect();

    let plan_confidence = compute_plan_confidence(consolidated, verdict, &blockers, risk);
    if plan_confidence < PLAN_CONFIDENCE_THRESHOLD {
        blockers.push(Blocker {
            kind: BlockerKind::PlanConfidenceBelowThreshold,
            message: format!(
                "Plan.confidence = {plan_confidence:.3} < PLAN_CONFIDENCE_THRESHOLD ({PLAN_CONFIDENCE_THRESHOLD}); Coding Engine blocked."
            ),
        });
    }

    if matches!(impact, Impact::Breaking) {
        blockers.push(Blocker {
            kind: BlockerKind::BreakingNeedsHuman,
            message: "impact=breaking; RFC 02 §4.1 escalates to `confirm` even under `auto` mode."
                .into(),
        });
    }

    let resume_point = steps
        .first()
        .map(|s| s.id.clone())
        .unwrap_or_else(|| "<no-steps>".into());

    let elapsed_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);

    Ok(Plan {
        plan_id: Uuid::new_v4(),
        mission_id: consolidated.mission_id,
        verdict_id: verdict.verdict_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        mission: consolidated.mission_statement.clone(),
        objectives,
        steps,
        strategy,
        risk,
        impact,
        roadmap: milestones,
        skills_used,
        models_needed,
        research_runs,
        confidence: plan_confidence,
        resume_point,
        blocked: blockers,
        model_id: "heuristic-v0".into(),
        elapsed_ms,
    })
}

/// Map `ConfidenceLevel` to a 0..=3 rank ordered from worst to best
/// (Block < Low < Medium < High). Used by the runner to fence the verdict
/// guard — the floor is a *minimum* acceptable confidence, so we accept
/// the verdict iff `rank(verdict) >= rank(floor)`. The enum's discriminant
/// order is High < Medium < Low < Block (see `types.rs`), which is the
/// opposite direction, hence we invert here.
fn confidence_rank(level: ConfidenceLevel) -> u32 {
    match level {
        ConfidenceLevel::Block => 0,
        ConfidenceLevel::Low => 1,
        ConfidenceLevel::Medium => 2,
        ConfidenceLevel::High => 3,
    }
}

fn missing_clarification(
    verdict: &PublicUnderstandingVerdict,
    answers: &[ClarificationAnswer],
) -> bool {
    let unanswered = verdict.gaps.iter().filter(|g| !g.auto_resolvable).any(|g| {
        let qid = verdict
            .clarification_questions
            .iter()
            .find(|q| {
                q.justification
                    .starts_with(&format!("gap={},", g.kind.tag()))
            })
            .map(|q| q.id.clone())
            .unwrap_or_default();
        !answers
            .iter()
            .any(|a| a.question_id == qid && !a.answer.is_empty())
    });
    unanswered
}

fn pick_strategy(
    consolidated: &MissionConsolidated,
    verdict: &PublicUnderstandingVerdict,
    blockers: &[Blocker],
) -> Strategy {
    // Read-only plans (ask mode, or any "needs clarify / research" blocker)
    // always start incremental — the swarm forces read-only steps first.
    let needs_read_only = matches!(consolidated.suggested_mode, RecommendedMode::Ask)
        || blockers.iter().any(|b| {
            matches!(
                b.kind,
                BlockerKind::MissingClarification
                    | BlockerKind::MissingResearch
                    | BlockerKind::VerdictTooLow
            )
        });

    if needs_read_only {
        return Strategy::Incremental;
    }
    if matches!(verdict.desired_action, DesiredAction::Refactor) {
        return Strategy::Strangler;
    }
    if matches!(verdict.scope, Scope::Project) {
        return Strategy::BigBang;
    }
    if matches!(verdict.domain, Domain::Backend | Domain::Frontend)
        && matches!(consolidated.suggested_mode, RecommendedMode::Code)
    {
        return Strategy::Tdd;
    }
    Strategy::Incremental
}

fn derive_objectives(
    consolidated: &MissionConsolidated,
    verdict: &PublicUnderstandingVerdict,
    strategy: &Strategy,
) -> Vec<Objective> {
    let mut objectives: Vec<Objective> = Vec::new();

    // Always-on first objective: confirm the consolidated mission.
    objectives.push(Objective {
        id: "O0".into(),
        statement: consolidated.mission_statement.clone(),
        verifiable_via: consolidated
            .success_criteria
            .iter()
            .map(|c| VerificationCriterion {
                kind: VerificationKind::ManualReview,
                description: c.clone(),
            })
            .collect(),
        depends_on: Vec::new(),
    });

    if matches!(*strategy, Strategy::Strangler) {
        objectives.push(Objective {
            id: "O1".into(),
            statement: "Introduce the replacement module behind a facade.".into(),
            verifiable_via: vec![VerificationCriterion {
                kind: VerificationKind::TypeCheck,
                description: "New module compiles alongside the legacy module.".into(),
            }],
            depends_on: vec!["O0".into()],
        });
        objectives.push(Objective {
            id: "O2".into(),
            statement: "Route callers to the new module incrementally.".into(),
            verifiable_via: vec![VerificationCriterion {
                kind: VerificationKind::Test,
                description: "Existing tests pass against the new module.".into(),
            }],
            depends_on: vec!["O1".into()],
        });
        objectives.push(Objective {
            id: "O3".into(),
            statement: "Delete the legacy module.".into(),
            verifiable_via: vec![VerificationCriterion {
                kind: VerificationKind::Lint,
                description: "No references to the legacy module remain.".into(),
            }],
            depends_on: vec!["O2".into()],
        });
    } else {
        objectives.push(Objective {
            id: "O1".into(),
            statement: "Research / clarify any open gap before edits.".into(),
            verifiable_via: verdict
                .gaps
                .iter()
                .filter(|g| !g.auto_resolvable)
                .map(|g| VerificationCriterion {
                    kind: VerificationKind::ResearchOutcome,
                    description: format!("Resolve gap {} ({})", g.kind.tag(), g.evidence),
                })
                .collect(),
            depends_on: vec!["O0".into()],
        });

        let action_label = match verdict.desired_action {
            DesiredAction::Create => "Create the new artefact.",
            DesiredAction::Modify => "Apply the modification.",
            DesiredAction::Debug => "Fix the underlying defect.",
            DesiredAction::Refactor => "Refactor per the consolidated mission.",
            DesiredAction::Test => "Write the missing test coverage.",
            DesiredAction::Research => "Deliver the research report.",
            DesiredAction::Explain => "Explain the code path.",
            DesiredAction::Unknown => "Investigate and propose a concrete next step.",
        };
        objectives.push(Objective {
            id: "O2".into(),
            statement: action_label.into(),
            verifiable_via: vec![VerificationCriterion {
                kind: VerificationKind::Test,
                description: "Tests pass after edits (RFC 14 Validation).".into(),
            }],
            depends_on: vec!["O1".into()],
        });

        objectives.push(Objective {
            id: "O3".into(),
            statement: "Validate end-to-end against success criteria.".into(),
            verifiable_via: consolidated
                .success_criteria
                .iter()
                .map(|c| VerificationCriterion {
                    kind: VerificationKind::Test,
                    description: c.clone(),
                })
                .collect(),
            depends_on: vec!["O2".into()],
        });
    }

    objectives
}

fn derive_milestones(objectives: &[Objective], strategy: &Strategy) -> Vec<Milestone> {
    if matches!(*strategy, Strategy::BigBang) {
        return vec![Milestone {
            id: "M1".into(),
            label: "Single milestone — big bang rollout".into(),
            objectives: objectives.iter().map(|o| o.id.clone()).collect(),
            depends_on: Vec::new(),
        }];
    }
    objectives
        .iter()
        .enumerate()
        .map(|(i, o)| Milestone {
            id: format!("M{}", i + 1),
            label: format!("Milestone {}", o.id),
            objectives: vec![o.id.clone()],
            depends_on: if i == 0 {
                Vec::new()
            } else {
                vec![format!("M{i}")]
            },
        })
        .collect()
}

fn derive_steps(
    objectives: &[Objective],
    milestones: &[Milestone],
    strategy: &Strategy,
    consolidated: &MissionConsolidated,
    verdict: &PublicUnderstandingVerdict,
) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    let tdd = matches!(*strategy, Strategy::Tdd);

    for o in objectives {
        let milestone_id = milestones
            .iter()
            .find(|m| m.objectives.contains(&o.id))
            .map(|m| m.id.clone())
            .unwrap_or_default();

        let read_only = matches!(consolidated.suggested_mode, RecommendedMode::Ask)
            || o.statement.starts_with("Research")
            || o.statement.starts_with("Explain");

        let action = match verdict.desired_action {
            DesiredAction::Create => StepAction::Create,
            DesiredAction::Modify => StepAction::Modify,
            DesiredAction::Debug => StepAction::Debug,
            DesiredAction::Refactor => StepAction::Refactor,
            DesiredAction::Test => StepAction::Test,
            DesiredAction::Research => StepAction::Research,
            DesiredAction::Explain => StepAction::Context,
            DesiredAction::Unknown => StepAction::Research,
        };
        let action = if read_only {
            if matches!(consolidated.suggested_mode, RecommendedMode::Ask) {
                StepAction::Clarify
            } else {
                StepAction::Research
            }
        } else {
            action
        };

        let mut step = Step {
            id: format!("S{}", o.id),
            milestone_id,
            statement: o.statement.clone(),
            action,
            depends_on: o.depends_on.iter().map(|d| format!("S{d}")).collect(),
            skills: Vec::new(),
            models: Vec::new(),
            read_only,
        };

        if tdd && !read_only {
            let test_step = Step {
                id: format!("S{}-test", o.id),
                milestone_id: step.milestone_id.clone(),
                statement: format!("Write the failing test for objective {}.", o.id),
                action: StepAction::Test,
                depends_on: step.depends_on.clone(),
                skills: Vec::new(),
                models: Vec::new(),
                read_only: false,
            };
            step.depends_on.push(test_step.id.clone());
            steps.push(test_step);
        }

        steps.push(step);
    }

    steps
}

fn compute_risk(
    consolidated: &MissionConsolidated,
    verdict: &PublicUnderstandingVerdict,
    steps: &[Step],
) -> f32 {
    let mut risk: f32 = 0.2;
    if matches!(verdict.scope, Scope::Project) {
        risk += 0.3;
    }
    if matches!(verdict.scope, Scope::Feature) {
        risk += 0.15;
    }
    if !consolidated.non_goals.is_empty() {
        risk += 0.1;
    }
    let editable = steps.iter().filter(|s| !s.read_only).count() as f32;
    risk += (editable / 10.0).min(0.3);
    risk.min(1.0)
}

fn pick_impact(
    consolidated: &MissionConsolidated,
    verdict: &PublicUnderstandingVerdict,
    risk: f32,
) -> Impact {
    if risk >= 0.7
        || matches!(verdict.scope, Scope::Project)
        || consolidated
            .forbidden_actions
            .iter()
            .any(|f| f.contains("do not"))
    {
        return Impact::Breaking;
    }
    if risk >= 0.4 {
        return Impact::Major;
    }
    Impact::Minor
}

fn suggest_skills(verdict: &PublicUnderstandingVerdict) -> Vec<SkillRef> {
    let mut out = Vec::new();
    for tech in &verdict.technology {
        if let Some(skill) = tech_to_skill(tech) {
            out.push(SkillRef {
                skill_id: skill.into(),
                version: "v0".into(),
            });
        }
    }
    out.sort_by(|a, b| a.skill_id.cmp(&b.skill_id));
    out.dedup_by(|a, b| a.skill_id == b.skill_id);
    out
}

fn tech_to_skill(tech: &str) -> Option<&'static str> {
    match tech {
        "rust" => Some("rust-edit"),
        "tauri" => Some("tauri-edit"),
        "svelte" | "sveltekit" => Some("svelte-edit"),
        "typescript" => Some("ts-edit"),
        "tailwind" => Some("tailwind-edit"),
        "react" | "next.js" => Some("react-edit"),
        "postgres" | "sqlite" => Some("sql-edit"),
        "docker" | "kubernetes" | "terraform" => Some("devops-edit"),
        _ => None,
    }
}

fn suggest_models(verdict: &PublicUnderstandingVerdict, strategy: &Strategy) -> Vec<ModelRef> {
    let tier = match verdict.confidence {
        ConfidenceLevel::High => ModelTier::FreeCloud,
        ConfidenceLevel::Medium => ModelTier::Paid,
        ConfidenceLevel::Low => ModelTier::Frontier,
        ConfidenceLevel::Block => ModelTier::Frontier,
    };

    let mut models = vec![ModelRef {
        model_id: format!("heuristic-{}", verdict.domain.tag()),
        provider: "local".into(),
        tier,
    }];

    if matches!(*strategy, Strategy::Strangler) {
        models.push(ModelRef {
            model_id: "architect-v0".into(),
            provider: "openrouter".into(),
            tier: ModelTier::FreeCloud,
        });
    }

    models
}

fn compute_plan_confidence(
    consolidated: &MissionConsolidated,
    verdict: &PublicUnderstandingVerdict,
    blockers: &[Blocker],
    risk: f32,
) -> f32 {
    // Start from the verdict rubric mean, then dampen by:
    //   - presence of any blocker that is NOT the threshold blocker (the
    //     threshold blocker is computed FROM this value, so it can't dampen)
    //   - risk
    //   - requires_research_first (research not yet landed = uncertainty)
    let mut score = verdict.confidence_rubric.mean();

    for b in blockers {
        match b.kind {
            BlockerKind::PlanConfidenceBelowThreshold => {}
            _ => score -= 0.15,
        }
    }

    score -= risk * 0.15;
    if consolidated.requires_research_first {
        score -= 0.2;
    }

    score.clamp(0.0, 1.0)
}
