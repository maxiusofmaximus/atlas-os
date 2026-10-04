// Atlas OS — Proactive-turn host glue (RFC 20 Fase 23 v3.1.2.2; research/52).
//
// Ties the pure supervisor FSM to the real world: it computes the operator's
// availability from the calendar busy windows, probes the mission backlog, and
// feeds a `ProactiveCheck` to the pure `tick`. Keeps `tick` free of Journal and
// clock access (the runner stays a pure function).

use uuid::Uuid;

use crate::journal::Journal;
use crate::planning::availability::{availability_now, Availability, TurnPolicy};

use super::runner::{tick, TickContext, TickOutput};
use super::types::{SupervisorEvent, SupervisorState};

/// One proactive probe: the inputs the host computed + the supervisor's pure
/// output.
#[derive(Clone, Debug)]
pub struct ProactiveProbe {
    pub availability: Availability,
    pub pending_mission: Option<Uuid>,
    pub output: TickOutput,
}

/// Run one proactive probe at `now_ms` (deterministic; unit-testable).
pub fn proactive_check_at(
    journal: &Journal,
    ctx: &mut TickContext,
    state: SupervisorState,
    policy: &TurnPolicy,
    now_ms: i64,
) -> anyhow::Result<ProactiveProbe> {
    let availability = availability_now(journal, now_ms, policy)?;
    let pending_mission = journal.next_pending_mission()?;
    let output = tick(
        ctx,
        state,
        SupervisorEvent::ProactiveCheck {
            availability,
            pending_mission,
        },
    );
    Ok(ProactiveProbe {
        availability,
        pending_mission,
        output,
    })
}

/// Convenience wrapper using the wall clock.
pub fn proactive_check(
    journal: &Journal,
    ctx: &mut TickContext,
    state: SupervisorState,
    policy: &TurnPolicy,
) -> anyhow::Result<ProactiveProbe> {
    proactive_check_at(
        journal,
        ctx,
        state,
        policy,
        chrono::Utc::now().timestamp_millis(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::payload::BusySource;
    use crate::calendar::queue::BusyWindowInput;
    use crate::supervisor::types::{BudgetCaps, ExecutionMode, SupervisorAction, SupervisorState};
    use tempfile::TempDir;

    fn journal_with_pending() -> (TempDir, Journal, Uuid) {
        let dir = TempDir::new().unwrap();
        let journal = Journal::open(dir.path()).unwrap();
        let mission = Uuid::new_v4();
        journal.create_mission(mission, "queued").unwrap();
        (dir, journal, mission)
    }

    #[test]
    fn starts_pending_mission_when_free() {
        let (_d, journal, mission) = journal_with_pending();
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let state = SupervisorState::new(BudgetCaps::DEFAULT, ExecutionMode::HumanInLoop);
        let probe =
            proactive_check_at(&journal, &mut ctx, state, &TurnPolicy::default(), 1_000_000)
                .unwrap();
        assert_eq!(probe.availability, Availability::RunNow);
        assert_eq!(probe.pending_mission, Some(mission));
        assert!(matches!(
            probe.output.actions.as_slice(),
            [SupervisorAction::EnqueueProactiveTurn { mission_id }] if *mission_id == mission
        ));
    }

    #[test]
    fn waits_when_busy_and_emits_nothing() {
        let (_d, journal, _mission) = journal_with_pending();
        let now = 2_000_000i64;
        journal
            .busy_window_insert_manual(&BusyWindowInput {
                source: BusySource::Manual,
                external_id: "b1",
                subject: "meeting",
                body: None,
                starts_at: now - 60_000,
                ends_at: now + 600_000,
                weight: 1.0,
            })
            .unwrap();
        let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
        let state = SupervisorState::new(BudgetCaps::DEFAULT, ExecutionMode::HumanInLoop);
        let probe =
            proactive_check_at(&journal, &mut ctx, state, &TurnPolicy::default(), now).unwrap();
        assert!(matches!(probe.availability, Availability::WaitUntil(_)));
        assert!(probe.output.actions.is_empty());
    }
}
