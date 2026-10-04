// Atlas OS — LLM-driven Coding: request + parse a `Diff` from a model (Fase 26 v26.1).
//
// Completes the bridge started in v26.0 (`coding::llm::parse_diff_json`): this
// routes the step to a model through `call_with_cascade` (routing + failover +
// reliability gate upstream) and turns the reply into a canonical `Diff`. The
// `Diff` is what the pure Validation/Repair engines consume (v26.2), so a code
// step stops being "a chat message" and becomes an edit that gets verified.

use crate::coding::llm::{parse_diff_json, DiffMeta, DiffParseError, DIFF_CONTRACT_PROMPT};
use crate::coding::types::Diff;
use crate::orchestrator::call::{call_with_cascade, CallError};
use crate::orchestrator::client::{ChatMessage, ChatRequest, ProviderClient, Usage};
use crate::orchestrator::provider::Deployment;
use crate::orchestrator::routing::RoutingConfig;

/// A routed step whose model reply was successfully parsed into a `Diff`.
#[derive(Clone, Debug)]
pub struct StepDiffOutcome {
    pub diff: Diff,
    pub deployment_id: String,
    pub attempts: u8,
    pub usage: Option<Usage>,
}

/// Either the cascade could not reach any model, or the reply was not a usable
/// diff. Kept structured so the caller can branch (a `Parse` error is worth a
/// re-ask; a `Call` error is worth a cascade/cooldown reaction).
#[derive(Debug, thiserror::Error)]
pub enum StepDiffError {
    #[error("cascade failed: {0}")]
    Call(#[from] CallError),
    #[error("model reply was not a usable diff: {0}")]
    Parse(#[from] DiffParseError),
}

/// Ask the routed model for a structured `Diff` for `step_statement` and parse
/// it. `meta` carries the kernel-side identity the model must not supply.
#[allow(clippy::too_many_arguments)]
pub async fn call_diff_with_cascade<C: ProviderClient>(
    client: &C,
    routing: RoutingConfig,
    model: &str,
    deployments: &[Deployment],
    step_statement: &str,
    meta: DiffMeta,
    max_attempts: u8,
) -> Result<StepDiffOutcome, StepDiffError> {
    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage::system(DIFF_CONTRACT_PROMPT),
            ChatMessage::user(step_statement),
        ],
        temperature: None,
        max_tokens: None,
    };

    let outcome =
        call_with_cascade(client, routing, model, deployments, &request, max_attempts).await?;
    let diff = parse_diff_json(&outcome.response.content, meta)?;

    Ok(StepDiffOutcome {
        diff,
        deployment_id: outcome.deployment_id,
        attempts: outcome.attempts,
        usage: outcome.response.usage,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::client::{ChatResponse, ClientError, ClientResult};
    use std::sync::Mutex;
    use uuid::Uuid;

    struct Scripted {
        replies: Mutex<Vec<ClientResult<ChatResponse>>>,
    }

    impl Scripted {
        fn new(replies: Vec<ClientResult<ChatResponse>>) -> Self {
            Self {
                replies: Mutex::new(replies),
            }
        }
    }

    impl ProviderClient for Scripted {
        async fn chat(
            &self,
            _deployment: &Deployment,
            _request: &ChatRequest,
        ) -> ClientResult<ChatResponse> {
            self.replies.lock().unwrap().remove(0)
        }
    }

    fn dep(model: &str, id: &str) -> Deployment {
        let mut d = Deployment::new(model, format!("http://{id}"));
        d.id = id.into();
        d
    }

    fn meta() -> DiffMeta {
        DiffMeta {
            mission_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            step_id: "S1".into(),
            agent_id: Uuid::new_v4(),
            model_id: "m1".into(),
        }
    }

    fn json_reply() -> ChatResponse {
        ChatResponse {
            model: "m1".into(),
            content: r#"{"files":[{"path":"src/lib.rs","hunks":[{"old_start":0,"old_end":0,"new_lines":["pub fn hi() {}"],"rationale":"r"}]}]}"#.into(),
            usage: None,
        }
    }

    #[tokio::test]
    async fn parses_a_routed_diff() {
        let deployments = vec![dep("m1", "d1")];
        let client = Scripted::new(vec![Ok(json_reply())]);
        let out = call_diff_with_cascade(
            &client,
            RoutingConfig::default(),
            "m1",
            &deployments,
            "add a hi() function",
            meta(),
            5,
        )
        .await
        .expect("routed diff parses");
        assert_eq!(out.diff.files[0].path, "src/lib.rs");
        assert_eq!(
            out.diff.files[0].hunks[0].new_lines,
            vec!["pub fn hi() {}".to_string()]
        );
        assert_eq!(out.deployment_id, "d1");
    }

    #[tokio::test]
    async fn non_diff_reply_is_a_parse_error() {
        let deployments = vec![dep("m1", "d1")];
        let client = Scripted::new(vec![Ok(ChatResponse {
            model: "m1".into(),
            content: "Sure! I would add a function.".into(),
            usage: None,
        })]);
        let err = call_diff_with_cascade(
            &client,
            RoutingConfig::default(),
            "m1",
            &deployments,
            "add a hi() function",
            meta(),
            5,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, StepDiffError::Parse(DiffParseError::NoJson)),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn cascade_failure_propagates() {
        let deployments = vec![dep("m1", "d1")];
        let client = Scripted::new(vec![Err(ClientError::Http("boom".into()))]);
        let err = call_diff_with_cascade(
            &client,
            RoutingConfig::default(),
            "m1",
            &deployments,
            "add a hi() function",
            meta(),
            5,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, StepDiffError::Call(_)), "got {err:?}");
    }
}
