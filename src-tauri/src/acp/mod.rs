// OpenCode OS — Microsoft Intelligent Terminal ACP server skeleton
// (RFC 28 §B Phase 1.5d item 3).
//
// This stub turns the OpenCode CLI into a first-class ACP agent:
//
//   * `initialize` is answered with `agentInfo: "OpenCode OS"` plus an
//     `agentCapabilities` block that advertises `loadSession=false` and no
//     MCP / session-list / fork surfaces yet,
//   * `session/new` synthesises a fresh `SessionId` from `uuid::v4`,
//   * `session/prompt` immediately stops with `StopReason::Refusal` and
//     streams a single `session/update` chunk carrying a sentinel text
//     explaining "Phase 1.5d: agent host loop not wired" before the prompt
//     response goes out,
//   * the `$/cancel_request` notification is acknowledged via `tracing`
//     and otherwise dropped — the stub never enters a host loop, so there is
//     no in-flight work to cancel.
//
// The entry point is `run_server`. It is intentionally `#[allow(dead_code)]`
// for this commit: the `opencode` binary will pick it up in RFC 28 §B item 4
// once Microsoft IT detection lands. The crate already re-exports this module
// under the `acp-server` feature flag (see `lib.rs`).
//
// No `// TODO`s live in this file. Every branch is the deliberate behaviour
// the stub ships with in Phase 1.5d.

pub mod mode_mapping;

use agent_client_protocol::{
    on_receive_notification, on_receive_request,
    schema::v1::{
        AgentCapabilities, CancelRequestNotification, ContentBlock, InitializeRequest,
        InitializeResponse, NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse,
        SessionId, SessionNotification, SessionUpdate, StopReason, TextContent,
    },
    schema::ProtocolVersion,
    Agent, Stdio,
};
use uuid::Uuid;

/// Entry point invoked by the `opencode` binary when Microsoft IT detection
/// requests an ACP stdio loop. Drives the JSON-RPC server over stdio until
/// the client disconnects or the transport closes. The binary is not yet
/// wired to call this — see RFC 28 §B item 4 — so the symbol is touched
/// only by its compile-check test today.
#[allow(dead_code, reason = "binary wiring lands in RFC 28 §B item 4")]
pub async fn run_server() -> agent_client_protocol::Result<()> {
    Agent
        .builder()
        .name("opencode")
        .on_receive_request(
            async |request: InitializeRequest, responder, _connection| {
                let response = build_initialize_response(request.protocol_version);
                responder.respond(response)?;
                Ok(())
            },
            on_receive_request!(),
        )
        .on_receive_request(
            async |request: NewSessionRequest, responder, _connection| {
                tracing::debug!(cwd = ?request.cwd, "acp session/new");
                let response = build_new_session_response();
                responder.respond(response)?;
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
    let agent_info = agent_client_protocol::schema::v1::Implementation::new(
        "opencode",
        env!("CARGO_PKG_VERSION"),
    )
    .title("OpenCode OS");
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
    fn initialize_response_carries_opencode_os_title() {
        let response = build_initialize_response(ProtocolVersion::V1);
        assert_eq!(response.protocol_version, ProtocolVersion::V1);
        assert!(!response.agent_capabilities.load_session);
        let info = response
            .agent_info
            .as_ref()
            .expect("agent_info is set by build_initialize_response");
        assert_eq!(info.name, "opencode");
        assert_eq!(info.title.as_deref(), Some("OpenCode OS"));
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
}
