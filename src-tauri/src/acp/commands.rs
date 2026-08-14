// Atlas OS — ACP slash commands advertised via `available_commands_update`
// (RFC 28 §B Phase 1.5d item 4).
//
// Microsoft Intelligent Terminal surfaces agent-provided slash commands to
// the operator UI when the agent emits a `session/update` notification
// whose `SessionUpdate` variant is `AvailableCommandsUpdate`. Per RFC 28 §B
// (lines 122-123) we advertise five commands mapping onto the existing
// `atlas` CLI surface:
//
//   /opencode mission new    → `atlas mission new` (RFC 25 §3.9)
//   /opencode fork            → `atlas fork` (RFC 25 §3.9)
//   /opencode resume          → `atlas resume` (RFC 19)
//   /opencode exec step       → `opencode exec step` (RFC 27 §F)
//   /opencode fix [hint]      → Repair engine (RFC 15). Captures the active
//                                IT pane scrollback via `wtcli active-pane`
//                                + `capture-pane --last-prompt`, packages it
//                                as a `Resource` content block, drives a
//                                Repair run.
//   /opencode restart         → `session/close` + `session/new` with the
//                                same `cwd`. Wires to the ACP method pair
//                                directly inside the host loop.
//
// This module owns the **catalogue** + **serialisation** of the commands.
// The actual `/opencode fix` capture flow lives in `acp/delegate.rs`; the
// `/opencode restart` ACP method pair is wired in `acp/mod.rs`.
//
// The module is pure (no tokio, no `Agent` handle): every fn returns a
// `Vec<AvailableCommand>` or `SessionUpdate` the caller stuffs into a
// notification. This keeps it directly unit-testable.

use agent_client_protocol::schema::v1::{
    AvailableCommand, AvailableCommandInput, AvailableCommandsUpdate, SessionUpdate,
    UnstructuredCommandInput,
};

/// Slash command names advertised to IT. Kept as `&'static str` so callers
/// in `delegate.rs` can match on them without re-typing the strings.
pub const CMD_MISSION_NEW: &str = "opencode mission new";
pub const CMD_FORK: &str = "opencode fork";
pub const CMD_RESUME: &str = "opencode resume";
pub const CMD_EXEC_STEP: &str = "opencode exec step";
pub const CMD_FIX: &str = "opencode fix";
pub const CMD_RESTART: &str = "opencode restart";

/// Build the canonical catalogue of Atlas OS slash commands for ACP.
///
/// The order is stable on the wire so IT can render a predictable menu:
/// mission creation first, then session lifecycle (fork/resume), then the
/// step-level delegate, and finally the interactive repair pair. This is
/// the same conceptual order as `cf. proto::Commands` in the CLI.
pub fn build_available_commands() -> Vec<AvailableCommand> {
    vec![
        AvailableCommand::new(
            CMD_MISSION_NEW,
            "Create a new Atlas OS mission from a raw prompt.",
        ),
        AvailableCommand::new(
            CMD_FORK,
            "Fork the current session under a new mission id (Cursor pattern).",
        ),
        AvailableCommand::new(
            CMD_RESUME,
            "Resume the current mission from its latest checkpoint.",
        ),
        AvailableCommand::new(
            CMD_EXEC_STEP,
            "Execute one step of the current plan (Alt+Shift+B delegate).",
        ),
        AvailableCommand::new(
            CMD_FIX,
            "Capture the active pane scrollback and route it to the Repair engine.",
        )
        .input(AvailableCommandInput::Unstructured(
            UnstructuredCommandInput::new("optional hint for the Repair engine"),
        )),
        AvailableCommand::new(
            CMD_RESTART,
            "Close and recreate the session with the same cwd.",
        ),
    ]
}

/// Wrap the canonical catalogue in a `SessionUpdate::AvailableCommandsUpdate`
/// ready to be sent via `SessionNotification::new(session_id, update)`.
pub fn build_available_commands_update() -> SessionUpdate {
    SessionUpdate::AvailableCommandsUpdate(AvailableCommandsUpdate::new(build_available_commands()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_has_six_commands_in_stable_order() {
        let cmds = build_available_commands();
        let names: Vec<&str> = cmds.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            [
                CMD_MISSION_NEW,
                CMD_FORK,
                CMD_RESUME,
                CMD_EXEC_STEP,
                CMD_FIX,
                CMD_RESTART,
            ]
        );
    }

    #[test]
    fn only_fix_carries_unstructured_input() {
        for cmd in build_available_commands() {
            if cmd.name == CMD_FIX {
                let input = cmd
                    .input
                    .as_ref()
                    .expect("/opencode fix must carry an UnstructuredCommandInput");
                let AvailableCommandInput::Unstructured(u) = input else {
                    panic!("expected Unstructured variant, got {input:?}");
                };
                assert!(!u.hint.is_empty());
            } else {
                assert!(
                    cmd.input.is_none(),
                    "{} unexpectedly advertises input",
                    cmd.name
                );
            }
        }
    }

    #[test]
    fn every_command_has_a_non_empty_description() {
        for cmd in build_available_commands() {
            assert!(
                !cmd.description.is_empty(),
                "{} missing description",
                cmd.name
            );
        }
    }

    #[test]
    fn available_commands_update_wraps_catalogue_into_session_update() {
        let update = build_available_commands_update();
        match update {
            SessionUpdate::AvailableCommandsUpdate(acu) => {
                assert_eq!(acu.available_commands.len(), 6);
            }
            other => panic!("expected AvailableCommandsUpdate, got {other:?}"),
        }
    }

    #[test]
    fn command_names_round_trip_through_const_match() {
        for cmd in build_available_commands() {
            let matched = match cmd.name.as_str() {
                CMD_MISSION_NEW => "mission",
                CMD_FORK => "fork",
                CMD_RESUME => "resume",
                CMD_EXEC_STEP => "exec",
                CMD_FIX => "fix",
                CMD_RESTART => "restart",
                other => panic!("unknown command name `{other}`"),
            };
            assert!(!matched.is_empty());
        }
    }
}
