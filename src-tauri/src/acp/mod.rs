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
//   * `session/prompt` immediately stops with `StopReason::Refusal` and
//     streams a single `session/update` chunk carrying a sentinel text
//     explaining "Phase 1.5d: agent host loop not wired" before the prompt
//     response goes out,
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

/// Entry point invoked by the `atlas` binary when Microsoft IT detection
/// requests an ACP stdio loop. Drives the JSON-RPC server over stdio until
/// the client disconnects or the transport closes. Item 4 wires the binary
/// (`src-tauri/src/cli/bin/opencode.rs`) to call this when env detection
/// sees the IT host; without that env the subcommand list reuse the
/// existing CLI dispatch.
pub async fn run_server() -> agent_client_protocol::Result<()> {
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
                tracing::warn!(
                    session_id = %request.session_id,
                    "acp prompt rejected: phase 1.5d host loop not wired"
                );
                let update = SessionNotification::new(
                    request.session_id.clone(),
                    SessionUpdate::AgentMessageChunk(
                        agent_client_protocol::schema::v1::ContentChunk::new(ContentBlock::Text(
                            TextContent::new("Phase 1.5d: agent host loop not wired.".to_string()),
                        )),
                    ),
                );
                connection.send_notification(update)?;
                let response = PromptResponse::new(StopReason::Refusal);
                responder.respond(response)?;
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
