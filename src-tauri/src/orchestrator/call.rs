// Atlas OS — Call-with-cascade (RFC 20 Fase 25 v25.1; plan research/54).
//
// Composes the provider HTTP client (`client.rs`) with the pure cascade
// (`cascade.rs`). The cascade does NOT select the primary — the caller tries
// the primary group first, then hands each failure mode to `Cascade::next_target`
// for same-group failover + bucket escalation. This function owns that flow:
// primary → on failure, cascade fallbacks (excluding tried deployments) until
// success or exhaustion.
//
// Generic over `ProviderClient` (no `dyn`), so the loop is tested offline with a
// scripted mock and runs against `HttpProviderClient` in production.

use crate::orchestrator::cascade::{Cascade, CascadeStep, ExhaustionReason, FailureMode};
use crate::orchestrator::client::{ChatRequest, ChatResponse, ClientError, ProviderClient};
use crate::orchestrator::provider::Deployment;
use crate::orchestrator::routing::RoutingConfig;

/// Outcome of a successful call: the response plus which deployment answered.
#[derive(Clone, Debug, PartialEq)]
pub struct CallOutcome {
    pub response: ChatResponse,
    pub deployment_id: String,
    pub attempts: u8,
}

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("cascade exhausted ({reason:?}) after {attempts} attempt(s)")]
    Exhausted {
        reason: ExhaustionReason,
        attempts: u8,
        last: Option<ClientError>,
    },
}

/// Pure: map a provider error to the cascade's failure discriminator.
pub fn mode_of(err: &ClientError) -> FailureMode {
    match err {
        ClientError::Status { status, body } => match *status {
            429 => FailureMode::RateLimited,
            s if (500..600).contains(&s) => FailureMode::RateLimited,
            401 | 403 | 404 | 408 => FailureMode::BadConfigOrNetwork,
            400 | 413 | 422 => {
                let body = body.to_ascii_lowercase();
                if body.contains("context") || body.contains("too long") || body.contains("maximum")
                {
                    FailureMode::ContextWindowOverflow
                } else if body.contains("content")
                    || body.contains("policy")
                    || body.contains("filter")
                {
                    FailureMode::ContentPolicyRefusal
                } else {
                    FailureMode::BadConfigOrNetwork
                }
            }
            _ => FailureMode::BadConfigOrNetwork,
        },
        _ => FailureMode::BadConfigOrNetwork,
    }
}

/// First healthy (non-drained) deployment of `model_id`.
fn primary_of<'a>(deployments: &'a [Deployment], model_id: &str) -> Option<&'a Deployment> {
    deployments
        .iter()
        .find(|d| d.model_id == model_id && d.weight > 0.0)
}

/// Route `request` to the primary model and, on failure, fall back through the
/// cascade. `deployments` is the full (already reliability-gated) candidate set.
pub async fn call_with_cascade<C: ProviderClient>(
    client: &C,
    config: RoutingConfig,
    primary_model_id: &str,
    deployments: &[Deployment],
    request: &ChatRequest,
    max_attempts: u8,
) -> Result<CallOutcome, CallError> {
    call_with_cascade_and_denied(
        client,
        config,
        primary_model_id,
        deployments,
        request,
        max_attempts,
        &[],
    )
    .await
}

/// Like `call_with_cascade`, but excludes `denied` model ids from BOTH the
/// primary choice and every failover group. The reliability gate computes this
/// set (`filter_deployments` + `denied`), so a model whose historical pass rate
/// is below threshold — or that has too few samples — never gets a Diff step.
#[allow(clippy::too_many_arguments)]
pub async fn call_with_cascade_and_denied<'a, C: ProviderClient>(
    client: &C,
    config: RoutingConfig,
    primary_model_id: &str,
    deployments: &'a [Deployment],
    request: &ChatRequest,
    max_attempts: u8,
    denied: &[String],
) -> Result<CallOutcome, CallError> {
    let is_denied = |id: &str| denied.iter().any(|d| d == id);
    let mut cascade = Cascade::new(config, primary_model_id);
    let mut last: Option<ClientError> = None;
    let mut tries = 0u8;

    // 1. Primary group (unless the gate denied it).
    let primary = if is_denied(primary_model_id) {
        None
    } else {
        primary_of(deployments, primary_model_id)
    };
    if let Some(deployment) = primary {
        tries += 1;
        match client.chat(deployment, request).await {
            Ok(response) => {
                return Ok(CallOutcome {
                    response,
                    deployment_id: deployment.id.clone(),
                    attempts: tries,
                });
            }
            Err(err) => {
                cascade.exclude(deployment.id.clone());
                last = Some(err);
            }
        }
    }

    // 2. Fallbacks + same-group failover via the cascade.
    for _ in 0..max_attempts {
        let mode = last
            .as_ref()
            .map_or(FailureMode::BadConfigOrNetwork, mode_of);
        let healthy_for = |model_id: &str| -> Vec<&'a Deployment> {
            if is_denied(model_id) {
                return Vec::new();
            }
            deployments
                .iter()
                .filter(|d| d.model_id == model_id)
                .collect()
        };
        match cascade.next_target(mode, healthy_for) {
            CascadeStep::TryNext { deployment, .. } => {
                tries += 1;
                match client.chat(deployment, request).await {
                    Ok(response) => {
                        return Ok(CallOutcome {
                            response,
                            deployment_id: deployment.id.clone(),
                            attempts: tries,
                        });
                    }
                    Err(err) => {
                        cascade.exclude(deployment.id.clone());
                        last = Some(err);
                    }
                }
            }
            CascadeStep::Exhausted { reason, attempts } => {
                return Err(CallError::Exhausted {
                    reason,
                    attempts,
                    last,
                });
            }
        }
    }

    Err(CallError::Exhausted {
        reason: ExhaustionReason::MaxFallbacksReached,
        attempts: tries,
        last,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::client::{ChatMessage, ClientResult};
    use std::sync::Mutex;

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

    fn ok(model: &str) -> ChatResponse {
        ChatResponse {
            model: model.into(),
            content: "ok".into(),
            usage: None,
        }
    }

    fn request() -> ChatRequest {
        ChatRequest {
            model: "m1".into(),
            messages: vec![ChatMessage::user("hi")],
            temperature: None,
            max_tokens: None,
        }
    }

    fn dep(model: &str, id: &str) -> Deployment {
        let mut deployment = Deployment::new(model, format!("http://{id}"));
        deployment.id = id.into();
        deployment
    }

    #[tokio::test]
    async fn primary_succeeds_first_try() {
        let deployments = vec![dep("m1", "d1")];
        let client = Scripted::new(vec![Ok(ok("m1"))]);
        let outcome = call_with_cascade(
            &client,
            RoutingConfig::default(),
            "m1",
            &deployments,
            &request(),
            5,
        )
        .await
        .unwrap();
        assert_eq!(outcome.deployment_id, "d1");
        assert_eq!(outcome.attempts, 1);
    }

    #[tokio::test]
    async fn falls_back_to_next_model_on_rate_limit() {
        let mut config = RoutingConfig::default();
        config
            .fallback
            .fallbacks
            .insert("m1".into(), vec!["m2".into()]);
        let deployments = vec![dep("m1", "d1"), dep("m2", "d2")];
        let client = Scripted::new(vec![
            Err(ClientError::Status {
                status: 429,
                body: "rate limited".into(),
            }),
            Ok(ok("m2")),
        ]);
        let outcome = call_with_cascade(&client, config, "m1", &deployments, &request(), 5)
            .await
            .unwrap();
        assert_eq!(outcome.deployment_id, "d2");
        assert_eq!(outcome.attempts, 2);
    }

    #[tokio::test]
    async fn exhausts_when_all_fail() {
        let deployments = vec![dep("m1", "d1")];
        let client = Scripted::new(vec![
            Err(ClientError::Http("boom".into())),
            Err(ClientError::Http("boom".into())),
        ]);
        let err = call_with_cascade(
            &client,
            RoutingConfig::default(),
            "m1",
            &deployments,
            &request(),
            5,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CallError::Exhausted { .. }));
    }

    #[test]
    fn mode_of_maps_status_codes() {
        let s = |status: u16, body: &str| ClientError::Status {
            status,
            body: body.into(),
        };
        assert_eq!(mode_of(&s(429, "")), FailureMode::RateLimited);
        assert_eq!(mode_of(&s(503, "")), FailureMode::RateLimited);
        assert_eq!(mode_of(&s(401, "")), FailureMode::BadConfigOrNetwork);
        assert_eq!(
            mode_of(&s(400, "maximum context length exceeded")),
            FailureMode::ContextWindowOverflow
        );
        assert_eq!(
            mode_of(&s(400, "content policy violation")),
            FailureMode::ContentPolicyRefusal
        );
        assert_eq!(
            mode_of(&ClientError::Http("x".into())),
            FailureMode::BadConfigOrNetwork
        );
    }
}
