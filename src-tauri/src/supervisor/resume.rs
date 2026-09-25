// Atlas OS — Execution Supervisor resume from any state (RFC 19 §5 / §6.4, Phase 6 sub-fase 6.0).
//
// Rebuilds the mission-level `SupervisorState` from the Journal's latest
// `MissionCheckpoint` so the pure `tick()` FSM continues exactly where the
// mission stopped, without external scripts. When the mission has no
// checkpoint yet, returns a fresh idle state plus a synthesized checkpoint
// (nothing is persisted; the next `tick()` emits `PersistCheckpoint`).

use anyhow::{Context, Result};
use uuid::Uuid;

use super::types::{BudgetCaps, ExecutionMode, MissionCheckpoint, MissionPhase, SupervisorState};
use crate::journal::Journal;

pub fn parse_phase_tag(tag: &str) -> Result<MissionPhase> {
    MissionPhase::parse(tag)
        .with_context(|| format!("unknown supervisor phase tag `{tag}` (RFC 19 §6.1)"))
}

pub fn resume_state(
    journal: &Journal,
    mission_id: Uuid,
) -> Result<(SupervisorState, MissionCheckpoint)> {
    let Some(row) = journal.latest_checkpoint(mission_id)? else {
        let mut state = SupervisorState::new(BudgetCaps::DEFAULT, ExecutionMode::HumanInLoop);
        state.mission_id = Some(mission_id);
        let checkpoint = state.snapshot();
        return Ok((state, checkpoint));
    };
    let checkpoint = match journal.checkpoint_payload(row.checkpoint_id)? {
        Some(raw) => serde_json::from_str::<MissionCheckpoint>(&raw).with_context(|| {
            format!(
                "checkpoint {} payload is not a MissionCheckpoint",
                row.checkpoint_id
            )
        })?,
        None => MissionCheckpoint {
            checkpoint_id: row.checkpoint_id,
            mission_id: row.mission_id,
            phase: parse_phase_tag(&row.phase)?,
            current_plan_id: None,
            last_validation_report_id: None,
            last_repair_id: None,
            budget_tally: Default::default(),
            caps: None,
            mode: None,
            generated_at: row.generated_at,
        },
    };
    let state = SupervisorState {
        mission_id: Some(checkpoint.mission_id),
        phase: checkpoint.phase,
        mode: checkpoint.mode.unwrap_or_default(),
        budget_tally: checkpoint.budget_tally,
        caps: checkpoint.caps.unwrap_or_default(),
        active_agent_id: None,
        last_checkpoint_id: Some(checkpoint.checkpoint_id),
        consecutive_stalls: 0,
        consecutive_kicks: 0,
        recovery_attempts: 0,
        mission_restarts: 0,
        step_restarts: 0,
        current_step_id: None,
    };
    Ok((state, checkpoint))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supervisor::types::{BudgetTally, SupervisorEvent};
    use tempfile::TempDir;

    fn checkpoint_in(phase: MissionPhase) -> (TempDir, Journal, Uuid, MissionCheckpoint) {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let mission_id = Uuid::new_v4();
        let ckpt = MissionCheckpoint {
            checkpoint_id: Uuid::new_v4(),
            mission_id,
            phase,
            current_plan_id: Some(Uuid::new_v4()),
            last_validation_report_id: None,
            last_repair_id: None,
            budget_tally: BudgetTally {
                iterations: 3,
                elapsed_minutes: 1.5,
                cost_usd: 0.2,
                consecutive_failures: 1,
                context_compactions: 0,
            },
            caps: None,
            mode: None,
            generated_at: chrono::Utc::now().to_rfc3339(),
        };
        journal.save_checkpoint(&ckpt).expect("save");
        (tmp, journal, mission_id, ckpt)
    }

    #[test]
    fn resume_from_each_phase_restores_phase_and_tally() {
        for phase in [
            MissionPhase::Planning,
            MissionPhase::Executing,
            MissionPhase::Verifying,
            MissionPhase::Recovering,
            MissionPhase::Halted,
        ] {
            let (_tmp, journal, mission_id, ckpt) = checkpoint_in(phase);
            let (state, back) = resume_state(&journal, mission_id).expect("resume");
            assert_eq!(state.phase, phase);
            assert_eq!(state.mission_id, Some(mission_id));
            assert_eq!(state.last_checkpoint_id, Some(ckpt.checkpoint_id));
            assert_eq!(state.budget_tally, ckpt.budget_tally);
            assert_eq!(back.checkpoint_id, ckpt.checkpoint_id);
            assert_eq!(back.current_plan_id, ckpt.current_plan_id);
        }
    }

    #[test]
    fn resume_without_checkpoints_returns_fresh_idle() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let mission_id = Uuid::new_v4();
        let (state, ckpt) = resume_state(&journal, mission_id).expect("resume");
        assert_eq!(state.phase, MissionPhase::Idle);
        assert_eq!(state.mission_id, Some(mission_id));
        assert!(state.last_checkpoint_id.is_none());
        assert_eq!(ckpt.phase, MissionPhase::Idle);
        assert_eq!(ckpt.mission_id, mission_id);
    }

    #[test]
    fn resume_round_trip_persist_resume_persist() {
        let (_tmp, journal, mission_id, _) = checkpoint_in(MissionPhase::Verifying);
        let (state, _) = resume_state(&journal, mission_id).expect("first resume");
        let again = state.snapshot();
        journal.save_checkpoint(&again).expect("re-persist");
        let (restored, back) = resume_state(&journal, mission_id).expect("second resume");
        assert_eq!(restored.phase, MissionPhase::Verifying);
        assert_eq!(restored.budget_tally, state.budget_tally);
        assert_eq!(back.checkpoint_id, again.checkpoint_id);
    }

    #[test]
    fn resume_restores_persisted_caps_and_mode() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let mission_id = Uuid::new_v4();
        let caps = BudgetCaps {
            max_iterations: 7,
            max_minutes: 9,
            max_cost_usd: 2.5,
            max_consecutive_failures: 3,
            max_context_compactions: 1,
        };
        journal
            .save_checkpoint(&MissionCheckpoint {
                checkpoint_id: Uuid::new_v4(),
                mission_id,
                phase: MissionPhase::Executing,
                current_plan_id: None,
                last_validation_report_id: None,
                last_repair_id: None,
                budget_tally: BudgetTally::default(),
                caps: Some(caps),
                mode: Some(ExecutionMode::Autonomous),
                generated_at: chrono::Utc::now().to_rfc3339(),
            })
            .expect("save");
        let (state, _) = resume_state(&journal, mission_id).expect("resume");
        assert_eq!(state.caps, caps);
        assert_eq!(state.mode, ExecutionMode::Autonomous);
    }

    #[test]
    fn resume_legacy_payload_without_caps_mode_defaults() {
        let raw = serde_json::json!({
            "checkpoint_id": Uuid::new_v4(),
            "mission_id": Uuid::new_v4(),
            "phase": "executing",
            "current_plan_id": null,
            "last_validation_report_id": null,
            "last_repair_id": null,
            "budget_tally": {
                "iterations": 0,
                "elapsed_minutes": 0.0,
                "cost_usd": 0.0,
                "consecutive_failures": 0,
                "context_compactions": 0
            },
            "generated_at": chrono::Utc::now().to_rfc3339(),
        });
        let ckpt: MissionCheckpoint = serde_json::from_value(raw).expect("legacy payload parses");
        assert!(ckpt.caps.is_none());
        assert!(ckpt.mode.is_none());
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        journal.save_checkpoint(&ckpt).expect("save");
        let (state, _) = resume_state(&journal, ckpt.mission_id).expect("resume");
        assert_eq!(state.caps, BudgetCaps::DEFAULT);
        assert_eq!(state.mode, ExecutionMode::HumanInLoop);
    }

    #[test]
    fn resumed_state_continues_with_tick() {
        let (_tmp, journal, mission_id, _) = checkpoint_in(MissionPhase::Executing);
        let (state, _) = resume_state(&journal, mission_id).expect("resume");
        let mut ctx = crate::supervisor::runner::TickContext::new(ExecutionMode::HumanInLoop);
        let out =
            crate::supervisor::runner::tick(&mut ctx, state, SupervisorEvent::ValidationStarted);
        assert_eq!(out.state.phase, MissionPhase::Verifying);
    }

    #[test]
    fn parse_phase_tag_rejects_unknown() {
        assert!(parse_phase_tag("time-travel").is_err());
        assert_eq!(parse_phase_tag("halted").expect("tag").tag(), "halted");
    }
}
