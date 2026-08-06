// OpenCode OS — Orchestrator module (RFC 27 §B).
//
// Hosts the model hot-swap primitive. The full Model Orchestrator
// routing policy (RFC 04 §2 / §6) arrives in Phase 2; this Phase-1
// shim exposes the one primitive RFC 27 §B elevates to a foundational
// requirement: an operator (or the auto-fail-over policy) can swap the
// model driving a mission mid-flight without losing the journal state
// it already produced.
//
// The primitive is intentionally side-effect-light on the in-memory
// struct side: it rewrites the `model_id` / `judge_model_id` fields of
// the latest `PublicUnderstandingVerdict` for the mission and the
// `model_id` field of the latest `Plan`, then persists both back via
// the Journal and publishes a `BusEventKind::ModelSwapped` so the HUD
// renders the transition. The persisted histories (`journal_events`,
// `model_swaps`) keep the original values intact for the audit trail.

pub mod error;
pub mod parse_error;
pub mod provider;
pub mod registry;
pub mod retry;

pub use error::{ResetKind, SpendLimitError};
pub use parse_error::ParseError;
pub use provider::{
    Capability, Config as ProviderConfig, Deployment, ModelDescriptor, Provider, ProviderWire, Tier,
};
pub use registry::{Registry, RegistrySeed, RegistrySeedMeta, ResourceMode};
pub use retry::{BailDecision, RetryPolicy};

use anyhow::Context;
use chrono::Utc;
use uuid::Uuid;

use crate::core::bus::{BusEvent, BusEventKind, SwapInitiator};
use crate::journal::Journal;
use crate::planning::types::Plan;

/// Outcome of `handle_spend_limit_error` — the orchestrator's
/// host-side error path when a 429 / 402 / 403 carrying a
/// `resets_at` future timestamp is observed. Mirrors the
/// `SpendLimitError`-to-`model_resets` persistence + optional
/// Toast enqueue described by RFC 28 §H.4 / §H.6.
///
/// `toast_enqueued_id` is `Some(queue_id)` only when the `toast`
/// feature is enabled and the Toast row was successfully inserted.
/// When `toast` is off, the reset is still persisted (the next user
/// Observed HUD Tail surfaces it as the next pending reset) and
/// `toast_enqueued_id = None`.
#[derive(Clone, Debug, PartialEq)]
pub struct HandleSpendLimitOutcome {
    pub reset_id: i64,
    pub toast_enqueued_id: Option<i64>,
}

/// RFC 28 §H.4 — host-side error path when the orchestrator receives
/// a `SpendLimitError` payload from an upstream provider (or from
/// OmniRoute's envelope parser). The fn:
///
/// 1. Persists the reset row in `model_resets` (idempotent UPSERT
///    on `(provider, model, resets_at)`).
/// 2. Enqueues a `kind='model_ready'` Toast scheduled for
///    `resets_at` (when the `toast` feature is on).
/// 3. Links the Toast queue row id back to the reset row
///    (`model_resets.toast_id`).
/// 4. Publishes a `BusEventKind::SpendLimitObserved` event so the
///    HUD live-stream renders a card.
///
/// All errors are logged and swallowed — the orchestrator's request
/// loop must not crash because the Toast sub-system was unavailable.
/// `handle_spend_limit_error` is idempotent: a replay of the same
/// `(provider, model, resets_at)` returns the existing `reset_id`
/// (because `insert_model_reset` uses `INSERT OR IGNORE`) and does
/// NOT enqueue a duplicate Toast (caller should consult the existing
/// row's `toast_id` before calling this — see §H.4 design notes).
pub fn handle_spend_limit_error(
    journal: &Journal,
    error: &crate::orchestrator::error::SpendLimitError,
) -> anyhow::Result<HandleSpendLimitOutcome> {
    let now = Utc::now();

    // 1. Persist (or fetch existing) reset row + Toast link.
    let (reset_id, already_linked_toast_id) = journal.model_reset_upsert(error, now)?;

    // 2. Enqueue Toast + link (only when toast feature is on and this
    //    reset row was not yet linked — idempotent replays skip).
    #[cfg(feature = "toast")]
    let toast_enqueued_id: Option<i64> = {
        if already_linked_toast_id.is_some() {
            already_linked_toast_id
        } else {
            match journal.enqueue_model_ready_toast(error) {
                Ok(queue_id) => {
                    let _ = journal.link_model_reset_toast(reset_id, queue_id);
                    Some(queue_id)
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "toast enqueue failed for model_reset row {reset_id} — reset still persisted"
                    );
                    None
                }
            }
        }
    };
    #[cfg(not(feature = "toast"))]
    let toast_enqueued_id: Option<i64> = {
        // Suppress the dead-code warning (already_linked_toast_id
        // is read for idempotent-replay accounting when toast is on).
        let _ = already_linked_toast_id;
        None
    };

    // 3. Publish HUD event (best-effort — a publish failure must not
    //    mask the already-persisted reset row).
    let event = BusEvent::new(BusEventKind::SpendLimitObserved {
        provider: error.provider.clone(),
        model: error.model.clone(),
        status_code: error.status_code as i64,
        resets_at_ms: error.resets_at.timestamp_millis(),
        error_type: error.kind.as_str().to_string(),
        toast_enqueued_id,
    });
    if let Err(e) = journal.publish(&event) {
        tracing::warn!(error = %e, "BusEvent publish for SpendLimitObserved failed");
    }

    Ok(HandleSpendLimitOutcome {
        reset_id,
        toast_enqueued_id,
    })
}

/// Output of a successful `swap_model` call. The supervisor / HUD reads
/// `swap_id` to render a toast, `verdict_id` to re-route the next prompt
/// through the new model, and `plan_touched` to know whether to re-queue
/// the Coding Engine (when the swap happened mid-Planning) or to keep it
/// running (when the swap happened post-Planning).
#[derive(Clone, Debug)]
pub struct SwapOutcome {
    pub swap_id: Uuid,
    pub mission_id: Uuid,
    pub prev_model_id: String,
    pub new_model_id: String,
    /// Whether the latest `Plan` for the mission was rewritten in-place.
    /// `false` when no plan exists yet (the swap happened mid-Prompt).
    pub plan_touched: bool,
}

/// RFC 27 §B — swap the model driving a mission mid-flight.
///
/// Re-serialises the latest `PublicUnderstandingVerdict` for the
/// mission with `model_id = new_model_id` (also `judge_model_id` when
/// present, so a judge model is hot-swapped in lock-step) and the
/// latest `Plan` with `model_id = new_model_id`. Persists both back via
/// the Journal (UPSERT on `mission_id` / `plan_id`). Records a
/// `model_swaps` row and publishes a `BusEventKind::ModelSwapped`.
///
/// Returns `Err` when no verdict exists for the mission (the mission
/// never passed step 7 of the Prompt Understanding Pipeline) — there is
/// nothing to swap onto. Returns `Ok` with `plan_touched == false`
/// when a verdict exists but no plan yet (typical mid-Prompt swap).
///
/// `initiator` distinguishes operator-driven swaps from
/// orchestrator-auto-fail-over swaps (RFC 04 §6) so the HUD tail can
/// filter "auto swaps" without parsing the JSON payload.
pub fn swap_model(
    journal: &Journal,
    mission_id: Uuid,
    new_model_id: &str,
    initiator: SwapInitiator,
) -> anyhow::Result<SwapOutcome> {
    let mut verdict = journal
        .latest_verdict_for_mission(mission_id)?
        .context("no verdict for mission; the mission never passed step 7")?;
    let prev_model_id = verdict.model_id.clone();
    // Hot-swap keeps the original verdict immutable (RFC 02 §3.1.2 —
    // append-only idempotency). We mint a fresh `verdict_id` and bump
    // `timestamp` so the new row is row-of-record via `ORDER BY ts DESC`.
    verdict.verdict_id = Uuid::new_v4();
    verdict.timestamp = chrono::Utc::now().to_rfc3339();
    verdict.model_id = new_model_id.to_string();
    if verdict.judge_model_id.is_some() {
        verdict.judge_model_id = Some(new_model_id.to_string());
    }
    journal.save_verdict(&verdict, Some(mission_id))?;

    let plan_row = journal.latest_plan_for_mission(mission_id)?;
    let plan_touched = if let Some(row) = plan_row {
        let payload = journal
            .plan_payload(row.plan_id)?
            .context("plan row exists but payload missing — schema inconsistency")?;
        let mut plan: Plan = serde_json::from_str(&payload)?;
        // Hot-swap keeps the original plan immutable (RFC 02 §3.1.2 —
        // append-only idempotency). We mint a fresh `plan_id` and bump
        // `generated_at` so the swapped plan becomes the row-of-record
        // via `ORDER BY generated_at DESC`. The original plan remains in
        // the journal as historical provenance.
        plan.plan_id = Uuid::new_v4();
        plan.generated_at = chrono::Utc::now().to_rfc3339();
        plan.model_id = new_model_id.to_string();
        journal.save_plan(&plan)?;
        true
    } else {
        false
    };

    let occurred_at = chrono::Utc::now();
    let swap_id = journal.save_model_swap(
        mission_id,
        &prev_model_id,
        new_model_id,
        &initiator,
        occurred_at,
    )?;

    let event = BusEvent::new(BusEventKind::ModelSwapped {
        mission_id,
        prev_model_id: prev_model_id.clone(),
        new_model_id: new_model_id.to_string(),
        initiator,
    });
    journal.publish(&event)?;

    Ok(SwapOutcome {
        swap_id,
        mission_id,
        prev_model_id,
        new_model_id: new_model_id.to_string(),
        plan_touched,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::Journal;
    use crate::planning::types::{
        Impact, Milestone, ModelRef, ModelTier, Objective, Plan, RRRef, SkillRef, Step, StepAction,
        Strategy, VerificationCriterion, VerificationKind,
    };
    use crate::prompt::types::{
        ConfidenceLevel, ConfidenceRubric, DesiredAction, Domain, IntentHypothesis, NamedEntity,
        PublicUnderstandingVerdict, RecommendedMode, Scope,
    };
    use tempfile::TempDir;
    use uuid::Uuid;

    fn fresh_journal() -> (TempDir, Journal) {
        let dir = TempDir::new().unwrap();
        let journal = Journal::open(dir.path()).unwrap();
        (dir, journal)
    }

    fn seed_verdict(journal: &Journal, mission_id: Uuid, model_id: &str) -> Uuid {
        let verdict_id = Uuid::new_v4();
        let verdict = PublicUnderstandingVerdict {
            verdict_id,
            session_id: Uuid::new_v4(),
            raw_prompt: "swap me".into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            intent: "adiós gpt-4o".into(),
            intent_hypotheses: vec![IntentHypothesis {
                rank: 1,
                text: "swap the model driving this mission mid-flight".into(),
                feasibility_score: 0.7,
                rejection_reason: None,
            }],
            keys: vec!["swap-model".into()],
            named_entities: vec![NamedEntity {
                text: model_id.into(),
                r#type: "model".into(),
                in_kb: false,
            }],
            domain: Domain::Backend,
            technology: vec!["rust".into()],
            desired_action: DesiredAction::Create,
            scope: Scope::Module,
            implicit_signals: vec![],
            gaps: vec![],
            similar_missions: vec![],
            clarification_questions: vec![],
            confidence: ConfidenceLevel::Medium,
            confidence_rubric: ConfidenceRubric {
                intent_clarity: 0.7,
                scope_clarity: 0.7,
                feasibility_clarity: 0.7,
                context_clarity: 0.7,
            },
            observations: vec![],
            recommended_mode: RecommendedMode::Context,
            model_id: model_id.into(),
            judge_model_id: Some(model_id.into()),
            elapsed_ms: 0,
        };
        journal.save_verdict(&verdict, Some(mission_id)).unwrap();
        verdict_id
    }

    fn seed_plan(journal: &Journal, mission_id: Uuid, verdict_id: Uuid, model_id: &str) -> Uuid {
        let plan_id = Uuid::new_v4();
        let plan = Plan {
            plan_id,
            mission_id,
            verdict_id,
            generated_at: chrono::Utc::now().to_rfc3339(),
            mission: "swap me".into(),
            objectives: vec![Objective {
                id: "o-1".into(),
                statement: "objective".into(),
                verifiable_via: vec![VerificationCriterion {
                    kind: VerificationKind::Test,
                    description: "tests pass".into(),
                }],
                depends_on: vec![],
            }],
            steps: vec![Step {
                id: "s-1".into(),
                milestone_id: "m-1".into(),
                statement: "edit".into(),
                action: StepAction::Modify,
                depends_on: vec![],
                skills: vec![SkillRef {
                    skill_id: "rust-tdd".into(),
                    version: "0.1".into(),
                }],
                models: vec![ModelRef {
                    model_id: model_id.into(),
                    provider: "openai".into(),
                    tier: ModelTier::Paid,
                }],
                read_only: false,
            }],
            strategy: Strategy::Incremental,
            risk: 0.2,
            impact: Impact::Minor,
            roadmap: vec![Milestone {
                id: "m-1".into(),
                label: "milestone".into(),
                objectives: vec!["o-1".into()],
                depends_on: vec![],
            }],
            skills_used: vec![],
            models_needed: vec![],
            research_runs: vec![RRRef {
                research_run_id: Uuid::nil(),
                outcome_tag: "n/a".into(),
            }],
            confidence: 0.8,
            resume_point: "n/a".into(),
            blocked: vec![],
            model_id: model_id.into(),
            elapsed_ms: 0,
        };
        journal.save_plan(&plan).unwrap();
        plan_id
    }

    #[test]
    fn swap_rewrites_verdict_and_plan_model_ids() {
        let (_dir, journal) = fresh_journal();
        let mission_id = Uuid::new_v4();
        let verdict_id = seed_verdict(&journal, mission_id, "gpt-4o");
        let _orig_plan_id = seed_plan(&journal, mission_id, verdict_id, "gpt-4o");

        let outcome = swap_model(&journal, mission_id, "claude-sonnet-4", SwapInitiator::User)
            .expect("swap should succeed");

        assert_eq!(outcome.prev_model_id, "gpt-4o");
        assert_eq!(outcome.new_model_id, "claude-sonnet-4");
        assert!(outcome.plan_touched);
        assert_ne!(outcome.swap_id, Uuid::nil());

        let refreshed_verdict = journal
            .latest_verdict_for_mission(mission_id)
            .unwrap()
            .unwrap();
        assert_eq!(refreshed_verdict.model_id, "claude-sonnet-4");
        assert_eq!(
            refreshed_verdict.judge_model_id.as_deref(),
            Some("claude-sonnet-4")
        );

        // The original plan stays immutable; a new plan row is the latest.
        let latest_row = journal
            .latest_plan_for_mission(mission_id)
            .unwrap()
            .unwrap();
        let latest_payload = journal.plan_payload(latest_row.plan_id).unwrap().unwrap();
        let refreshed_plan: Plan = serde_json::from_str(&latest_payload).unwrap();
        assert_eq!(refreshed_plan.model_id, "claude-sonnet-4");
        assert_ne!(refreshed_plan.plan_id, Uuid::nil());
        assert_ne!(refreshed_plan.plan_id, _orig_plan_id);

        let swaps = journal.model_swaps_for_mission(mission_id).unwrap();
        assert_eq!(swaps.len(), 1);
        assert_eq!(swaps[0].prev_model_id, "gpt-4o");
        assert_eq!(swaps[0].new_model_id, "claude-sonnet-4");
        assert_eq!(swaps[0].initiator, "user");
    }

    #[test]
    fn swap_without_plan_only_touches_verdict() {
        let (_dir, journal) = fresh_journal();
        let mission_id = Uuid::new_v4();
        seed_verdict(&journal, mission_id, "gpt-4o");

        let outcome = swap_model(&journal, mission_id, "llama-3.1-70b", SwapInitiator::Auto)
            .expect("swap should succeed");

        assert!(!outcome.plan_touched);
        let swaps = journal.model_swaps_for_mission(mission_id).unwrap();
        assert_eq!(swaps.len(), 1);
        assert_eq!(swaps[0].initiator, "auto");
    }

    #[test]
    fn swap_fails_when_no_verdict_for_mission() {
        let (_dir, journal) = fresh_journal();
        let mission_id = Uuid::new_v4();
        let err = swap_model(&journal, mission_id, "claude", SwapInitiator::User).unwrap_err();
        assert!(
            format!("{err:#}").contains("no verdict"),
            "unexpected error: {err:#}"
        );
    }
}

#[cfg(test)]
mod handle_spend_limit_tests {
    use super::*;
    use crate::journal::Journal;
    use crate::orchestrator::error::{ResetKind, SpendLimitError};
    use tempfile::TempDir;

    fn fresh_journal() -> (TempDir, Journal) {
        let dir = TempDir::new().unwrap();
        let journal = Journal::open(dir.path()).unwrap();
        (dir, journal)
    }

    fn make_error(resets_at: &str, kind: ResetKind) -> SpendLimitError {
        let (status, kind) = match kind {
            ResetKind::RateLimit => (429u16, ResetKind::RateLimit),
            ResetKind::SpendLimit => (402, ResetKind::SpendLimit),
        };
        SpendLimitError {
            provider: "anthropic".into(),
            model: "claude-3-5-sonnet".into(),
            status_code: status,
            kind,
            resets_at: chrono::DateTime::parse_from_rfc3339(resets_at)
                .unwrap()
                .with_timezone(&Utc),
            request_id: Some("req_1".into()),
            message: Some("rate limited".into()),
        }
    }

    #[test]
    fn handle_without_toast_feature_still_persists_reset() {
        // Default build has no `toast` feature — Toast enqueue is
        // skipped, but the reset row must still be created.
        let (_dir, journal) = fresh_journal();
        let outcome = handle_spend_limit_error(
            &journal,
            &make_error("2026-08-03T13:00:00Z", ResetKind::SpendLimit),
        )
        .unwrap();
        assert!(outcome.reset_id > 0);
        #[cfg(not(feature = "toast"))]
        assert_eq!(
            outcome.toast_enqueued_id, None,
            "toast feature off → no Toast enqueued id"
        );
    }

    #[test]
    fn handle_persists_reset_row_in_model_resets() {
        let (_dir, journal) = fresh_journal();
        let outcome = handle_spend_limit_error(
            &journal,
            &make_error("2026-08-03T13:00:00Z", ResetKind::RateLimit),
        )
        .unwrap();
        assert!(outcome.reset_id > 0);
        let n = journal
            .count_model_resets_for_test("anthropic", "claude-3-5-sonnet")
            .unwrap();
        assert_eq!(n, 1, "exactly one row should have been persisted");
    }

    #[test]
    fn handle_is_idempotent_on_replay() {
        let (_dir, journal) = fresh_journal();
        let err = make_error("2026-08-03T13:00:00Z", ResetKind::RateLimit);
        let first = handle_spend_limit_error(&journal, &err).unwrap();
        let second = handle_spend_limit_error(&journal, &err).unwrap();
        assert_eq!(
            first.reset_id, second.reset_id,
            "replay should return the same reset_id"
        );
        let n = journal
            .count_model_resets_for_test("anthropic", "claude-3-5-sonnet")
            .unwrap();
        assert_eq!(n, 1, "deduped to one row");
    }
}
