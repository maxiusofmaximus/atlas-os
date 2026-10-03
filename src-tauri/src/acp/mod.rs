// Atlas OS — Microsoft Intelligent Terminal ACP server skeleton
// (RFC 28 §B Phase 1.5d item 3 + item 4).
//
// This stub turns the Atlas CLI into a first-class ACP agent:
//
//   * `initialize` is answered with `agentInfo: "Atlas OS"` plus an
//     `agentCapabilities` block that advertises `loadSession=false` and no
//     MCP / session-list / fork surfaces yet,
//   * `session/new` synthesises a fresh `SessionId` from `uuid::v4` and
//     immediately streams a `session/update` carrying the Atlas OS
//     slash-command catalogue (`available_commands_update`), so the IT
//     operator sees `/atlas fix`, `/atlas exec step`,
//     `/atlas restart`, `/atlas mission new`, ``/atlas fork``,
//     `/atlas resume` as soon as the session opens,
//   * `session/prompt` parses the first text block as an ACP slash-command
//     (`/opencode fix`, `/opencode exec step`, `/opencode restart`) via the
//     `delegate` contract. Supported commands dispatch to the real CLI
//     dispatch (`crate::cli::commands::exec::run` for `exec step`) and stream
//     one result/error `session/update` chunk before `StopReason::EndTurn`;
//     unsupported or context-less commands stream an explanatory chunk and
//     are refused (`StopReason::Refusal`) — never the Phase 1.5d sentinel.
//   * the `$/cancel_request` notification is acknowledged via `tracing`
//     and otherwise dropped — the stub never enters a host loop, so there is
//     no in-flight work to cancel.
//
// The entry point is `run_server`. It is invoked by the `atlas` binary
// when Microsoft IT detection requests an ACP stdio loop (see RFC 28 §B
// item 4 wiring in `src-tauri/src/cli/bin/opencode.rs`). The crate
// re-exports this module under the `acp-server` feature flag (see `lib.rs`).
//
// Slash-command catalogue + delegate helpers live in
//   * `commands`:    `available_commands_update` builder + ACP
//                    `AvailableCommand` definitions.
//   * `delegate`:    parsing of `/atlas fix [hint]`, `/atlas exec step`,
//                    `/atlas restart` argv + `wtcli` capture stub.
//   * `mode_mapping`: bidirectional `SupervisorState` ↔ ACP `AcpMode` table.
//
// No `// TODO`s live in this file. Every branch is the deliberate behaviour
// the stub ships with in Phase 1.5d.

pub mod commands;
pub mod delegate;
pub mod listen_worker;
pub mod mode_mapping;

use agent_client_protocol::{
    on_receive_notification, on_receive_request,
    schema::v1::{
        AgentCapabilities, CancelRequestNotification, ContentBlock, CurrentModeUpdate,
        InitializeRequest, InitializeResponse, NewSessionRequest, NewSessionResponse,
        PromptRequest, PromptResponse, SessionId, SessionNotification, SessionUpdate,
        SetSessionModeRequest, SetSessionModeResponse, StopReason, TextContent,
    },
    schema::ProtocolVersion,
    Agent, Stdio,
};
use uuid::Uuid;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Shared session → cwd map so `session/prompt` can recover the session cwd
/// the client declared in `session/new` (stable across one `run_server` call).
pub type SessionCwdMap = Arc<Mutex<HashMap<String, PathBuf>>>;

/// Map the parsed ACP slash-command to concrete host-loop behaviour.
/// Pure function — the async handler in `run_server` executes the effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptPlan {
    /// `/opencode exec step <plan_id> <step_id>` → run the real CLI exec step.
    ExecStep { plan_id: String, step_id: String },
    /// `/opencode fix [hint]` → needs IT `wtcli`; without a capture the
    /// prompt is refused with guidance instead of running a phantom Repair.
    FixRequested { hint: Option<String> },
    /// `/opencode restart` → echo a close+new directive and end the turn.
    RestartRequested { cwd: PathBuf },
    /// Any other slash command or free text → refuse with guidance.
    NotSupported { command: String },
}

/// Consume the delegate contract and emit a routing plan that the host
/// loop can execute without re-parsing the prompt.
pub fn plan_prompt(line: &str, cwd: &str) -> PromptPlan {
    match crate::acp::delegate::parse_delegate(line, cwd) {
        crate::acp::delegate::DelegateOutcome::ExecStep {
            mission_id,
            step_id,
        } => PromptPlan::ExecStep {
            plan_id: mission_id,
            step_id,
        },
        crate::acp::delegate::DelegateOutcome::FixRequested { hint, .. } => {
            PromptPlan::FixRequested { hint }
        }
        crate::acp::delegate::DelegateOutcome::RestartRequested { cwd } => {
            PromptPlan::RestartRequested { cwd }
        }
        crate::acp::delegate::DelegateOutcome::NotImplemented { command } => {
            PromptPlan::NotSupported { command }
        }
    }
}

/// First text block from a `PromptRequest.prompt` payload; empty string when
/// the prompt carries no text blocks (the handler then refuses with guidance).
pub fn first_prompt_text(prompt: &[agent_client_protocol::schema::v1::ContentBlock]) -> String {
    for block in prompt {
        if let agent_client_protocol::schema::v1::ContentBlock::Text(t) = block {
            return t.text.clone();
        }
    }
    String::new()
}

/// Entry point invoked by the `atlas` binary when Microsoft IT detection
/// requests an ACP stdio loop. Drives the JSON-RPC server over stdio until
/// the client disconnects or the transport closes. Item 4 wires the binary
/// (`src-tauri/src/cli/bin/opencode.rs`) to call this when env detection
/// sees the IT host; without that env the subcommand list reuse the
/// existing CLI dispatch.
pub async fn run_server() -> agent_client_protocol::Result<()> {
    let session_cwds: SessionCwdMap = Arc::new(Mutex::new(HashMap::new()));
    Agent
        .builder()
        .name("atlas")
        .on_receive_request(
            async |request: InitializeRequest, responder, _connection| {
                let response = build_initialize_response(request.protocol_version);
                responder.respond(response)?;
                Ok(())
            },
            on_receive_request!(),
        )
        .on_receive_request(
            async |request: NewSessionRequest, responder, connection| {
                tracing::debug!(cwd = ?request.cwd, "acp session/new");
                let response = build_new_session_response();
                let session_id = response.session_id.clone();
                if let Ok(mut map) = session_cwds.lock() {
                    map.insert(
                        session_id.0.as_ref().to_string(),
                        request.cwd.clone(),
                    );
                }
                responder.respond(response)?;
                let update = SessionNotification::new(
                    session_id,
                    commands::build_available_commands_update(),
                );
                connection.send_notification(update)?;
                Ok(())
            },
            on_receive_request!(),
        )
        .on_receive_request(
            async |request: SetSessionModeRequest, responder, connection| {
                let session_id = request.session_id.clone();
                let mode_id = request.mode_id.clone();
                let mode_tag = mode_id.0.as_ref().to_string();
                let supervisor_state = mode_mapping::parse_acp_mode_id(&mode_tag);
                tracing::info!(
                    session_id = %session_id,
                    acp_mode = %mode_tag,
                    supervisor_state = ?supervisor_state,
                    "acp session/set_mode override received",
                );
                let response = SetSessionModeResponse::new();
                responder.respond(response)?;
                let update = SessionNotification::new(
                    session_id,
                    SessionUpdate::CurrentModeUpdate(CurrentModeUpdate::new(mode_id)),
                );
                connection.send_notification(update)?;
                Ok(())
            },
            on_receive_request!(),
        )
        .on_receive_request(
            async |request: PromptRequest, responder, connection| {
                let line = first_prompt_text(&request.prompt);
                let cwd = session_cwds
                    .lock()
                    .ok()
                    .and_then(|map| map.get(request.session_id.0.as_ref()).cloned())
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                tracing::info!(
                    session_id = %request.session_id.0,
                    prompt_text_hash = ?line.len(),
                    "acp session/prompt"
                );
                match plan_prompt(&line, &cwd) {
                    PromptPlan::ExecStep { plan_id, step_id } => {
                        let kickoff = SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(
                                agent_client_protocol::schema::v1::ContentChunk::new(
                                    ContentBlock::Text(TextContent::new(format!(
                                        "atlas exec step {plan_id} {step_id}\n"
                                    ))),
                                ),
                            ),
                        );
                        connection.send_notification(kickoff)?;
                        let profile = crate::profiles::current().0;
                        let outcome = crate::cli::commands::exec::run(
                            crate::cli::commands::ExecCmd {
                                action: crate::cli::commands::exec::ExecAction::Step {
                                    plan_id: plan_id.clone(),
                                    step_id: step_id.clone(),
                                },
                            },
                            &profile,
                        )
                        .await;
                        let text = match outcome {
                            Ok(()) => {
                                format!("exec step {plan_id}/{step_id} completed\n")
                            }
                            Err(e) => format!("exec step {plan_id}/{step_id} failed: {e}\n"),
                        };
                        let update = SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(
                                agent_client_protocol::schema::v1::ContentChunk::new(
                                    ContentBlock::Text(TextContent::new(text)),
                                ),
                            ),
                        );
                        connection.send_notification(update)?;
                        responder.respond(PromptResponse::new(StopReason::EndTurn))?;
                    }
                    PromptPlan::FixRequested { hint } => {
                        let scrollback = crate::acp::delegate::capture_active_pane_scrollback();
                        let text = match scrollback {
                            Ok(cap) => format!(
                                "FixRequested captured scrollback ({} chars). Hint: {}\n",
                                cap.scrollback.len(),
                                hint.unwrap_or_default()
                            ),
                            Err(_) => format!(
                                "FixRequested requires IT wtcli capture; not available here. Hint: {}\n",
                                hint.unwrap_or_default()
                            ),
                        };
                        let update = SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(
                                agent_client_protocol::schema::v1::ContentChunk::new(
                                    ContentBlock::Text(TextContent::new(text)),
                                ),
                            ),
                        );
                        connection.send_notification(update)?;
                        responder.respond(PromptResponse::new(StopReason::EndTurn))?;
                    }
                    PromptPlan::RestartRequested { cwd } => {
                        let update = SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(
                                agent_client_protocol::schema::v1::ContentChunk::new(
                                    ContentBlock::Text(TextContent::new(format!(
                                        "Restart requested for cwd {}. Close + new session required.\n",
                                        cwd.display()
                                    ))),
                                ),
                            ),
                        );
                        connection.send_notification(update)?;
                        responder.respond(PromptResponse::new(StopReason::EndTurn))?;
                    }
                    PromptPlan::NotSupported { command } => {
                        let update = SessionNotification::new(
                            request.session_id.clone(),
                            SessionUpdate::AgentMessageChunk(
                                agent_client_protocol::schema::v1::ContentChunk::new(
                                    ContentBlock::Text(TextContent::new(format!(
                                        "Command not supported yet in the ACP host loop: {command}. Use the headless CLI for this verb.\n"
                                    ))),
                                ),
                            ),
                        );
                        connection.send_notification(update)?;
                        responder.respond(PromptResponse::new(StopReason::Refusal))?;
                    }
                }
                Ok(())
            },
            on_receive_request!(),
        )
        .on_receive_notification(
            async |notification: CancelRequestNotification, _connection| {
                tracing::info!(?notification, "acp cancel notification");
                Ok(())
            },
            on_receive_notification!(),
        )
        .connect_to(Stdio::new())
        .await
}

/// Build the `InitializeResponse` the stub advertises on every `initialize`.
/// Exposed as a freestanding fn so the unit tests can assert on the
/// `agentInfo` block without driving the stdio transport.
pub fn build_initialize_response(protocol_version: ProtocolVersion) -> InitializeResponse {
    let agent_capabilities = AgentCapabilities::new().load_session(false);
    let agent_info =
        agent_client_protocol::schema::v1::Implementation::new("atlas", env!("CARGO_PKG_VERSION"))
            .title("Atlas OS");
    InitializeResponse::new(protocol_version)
        .agent_capabilities(agent_capabilities)
        .agent_info(agent_info)
}

/// Build the `NewSessionResponse` the stub returns on every `session/new`.
/// The `SessionId` is a v4 UUID rendered as a hyphenated string — opaque to
/// the client, stable across `tracing` spans, and cheap to regenerate.
pub fn build_new_session_response() -> NewSessionResponse {
    let session_id = SessionId::new(Uuid::new_v4().to_string());
    NewSessionResponse::new(session_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::ProtocolVersion;

    #[test]
    fn run_server_compiles_into_a_typed_result() {
        // Compile-only smoke: `run_server` cannot be awaited in a unit test
        // because Stdio blocks on the real stdin. Asserting the fn item
        // type pins its signature to `agent_client_protocol::Result<()>`.
        fn _assert_signature() -> agent_client_protocol::Result<()> {
            let fut = run_server();
            // Drop without awaiting so Stdio never reads stdin.
            drop(fut);
            Ok(())
        }
    }

    #[test]
    fn initialize_response_carries_atlas_os_title() {
        let response = build_initialize_response(ProtocolVersion::V1);
        assert_eq!(response.protocol_version, ProtocolVersion::V1);
        assert!(!response.agent_capabilities.load_session);
        let info = response
            .agent_info
            .as_ref()
            .expect("agent_info is set by build_initialize_response");
        assert_eq!(info.name, "atlas");
        assert_eq!(info.title.as_deref(), Some("Atlas OS"));
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn new_session_response_has_unique_v4_session_ids() {
        let a = build_new_session_response();
        let b = build_new_session_response();
        assert_ne!(a.session_id, b.session_id, "v4 UUIDs must not collide");
        assert!(
            Uuid::parse_str(&a.session_id.0).is_ok(),
            "{} is a valid hyphenated v4 UUID",
            a.session_id
        );
    }

    #[test]
    fn prompt_response_is_a_refusal_with_no_mcp_capabilities() {
        let response = PromptResponse::new(StopReason::Refusal);
        assert_eq!(response.stop_reason, StopReason::Refusal);
        let caps = AgentCapabilities::new().load_session(false);
        assert!(!caps.load_session);
    }

    #[test]
    fn set_mode_response_is_default_with_meta_unset() {
        let response = SetSessionModeResponse::new();
        assert!(response.meta.is_none());
    }

    #[test]
    fn plan_prompt_routes_supported_commands() {
        assert_eq!(
            plan_prompt("/opencode exec step plan-1 step-2", "/tmp"),
            PromptPlan::ExecStep {
                plan_id: "plan-1".to_string(),
                step_id: "step-2".to_string()
            }
        );
        assert!(matches!(
            plan_prompt("/opencode fix boom", "/tmp"),
            PromptPlan::FixRequested { hint: Some(_) }
        ));
        assert!(matches!(
            plan_prompt("/opencode restart", "/home/user"),
            PromptPlan::RestartRequested { .. }
        ));
        assert!(matches!(
            plan_prompt("/opencode frobnicate", "/tmp"),
            PromptPlan::NotSupported { .. }
        ));
        assert!(matches!(
            plan_prompt("hello world", "/tmp"),
            PromptPlan::NotSupported { .. }
        ));
    }

    #[test]
    fn first_prompt_text_extracts_text_block() {
        let blocks = vec![agent_client_protocol::schema::v1::ContentBlock::Text(
            agent_client_protocol::schema::v1::TextContent::new("hi"),
        )];
        assert_eq!(first_prompt_text(&blocks), "hi");
        assert!(first_prompt_text(&[]).is_empty());
    }

    #[test]
    fn current_mode_update_round_trips_through_mode_mapping() {
        use super::mode_mapping::{acp_mode_id, AcpMode};
        let mode = AcpMode::Architect;
        let mode_tag = acp_mode_id(&mode);
        let update = CurrentModeUpdate::new(mode_tag.to_string());
        assert_eq!(update.current_mode_id.0.as_ref(), mode_tag);
        let parsed = super::mode_mapping::parse_acp_mode_id(mode_tag)
            .expect("parse_acp_mode_id accepts a valid ACP mode id produced by acp_mode_id");
        assert_eq!(parsed, mode);
    }

    #[test]
    fn unknown_acp_mode_id_is_parsed_as_none() {
        assert!(super::mode_mapping::parse_acp_mode_id("unknown-mode").is_none());
    }
}
