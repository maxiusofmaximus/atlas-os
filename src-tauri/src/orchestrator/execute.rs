// Atlas OS — Orchestrator execution loop (RFC 20 Fase 25 v25.2/v25.3; research/54).
//
// The first code that DRIVES a mission: for each plan step it routes to a model
// through `call_with_cascade`, computes cost from the pricing table, feeds a
// `ToolCall` to the Execution Supervisor (bumping the budget tally and possibly
// tripping a cap → HaltSession), honours a cancellation flag, and records the
// outcome. Generic over `ProviderClient`, so it is unit-tested offline with a
// scripted mock and runs against `HttpProviderClient` in production.

use std::sync::atomic::{AtomicBool, Ordering};

use uuid::Uuid;

use crate::orchestrator::call::call_with_cascade;
use crate::orchestrator::client::{ChatMessage, ChatRequest, ProviderClient, Usage};
use crate::orchestrator::provider::Deployment;
use crate::orchestrator::routing::RoutingConfig;
use crate::supervisor::runner::{tick, TickContext};
use crate::supervisor::types::{
    BudgetCaps, BudgetTally, ExecutionMode, MissionPhase, SupervisorAction, SupervisorEvent,
    SupervisorState,
};

/// Price for a model, per 1M tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelPrice {
    pub input_per_1m: f64,
    pub output_per_1m: f64,
}

/// One step to execute (a projection of `planning::types::Step`).
#[derive(Clone, Debug, PartialEq)]
pub struct ExecuteStep {
    pub id: String,
    pub model_id: String,
    pub statement: String,
}

/// Result of one executed step.
#[derive(Clone, Debug, PartialEq)]
pub struct StepOutcome {
    pub step_id: String,
    pub model_id: String,
    pub deployment_id: String,
    pub content: String,
    pub attempts: u8,
    pub usage: Option<Usage>,
    pub cost_usd: f64,
    pub latency_ms: i64,
}

#[derive(Clone, Debug)]
pub struct ExecuteConfig {
    pub max_attempts: u8,
    pub caps: BudgetCaps,
}

impl Default for ExecuteConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            caps: BudgetCaps::DEFAULT,
        }
    }
}

/// Summary of an execution pass.
#[derive(Clone, Debug)]
pub struct ExecuteReport {
    pub mission_id: Uuid,
    pub outcomes: Vec<StepOutcome>,
    pub halted: Option<String>,
    pub tally: BudgetTally,
    pub phase: MissionPhase,
    pub elapsed_ms: i64,
}

/// Pure: blended cost of a request given its usage and the model price.
pub fn cost_of(usage: Option<Usage>, price: Option<ModelPrice>) -> f64 {
    match (usage, price) {
        (Some(u), Some(p)) => {
            (u.prompt_tokens as f64 / 1_000_000.0) * p.input_per_1m
                + (u.completion_tokens as f64 / 1_000_000.0) * p.output_per_1m
        }
        _ => 0.0,
    }
}

/// Drive `steps` through the orchestrator. Never panics; failures are recorded
/// in `report.halted`. `price_of` resolves a model id to its token price (or
/// `None` when unknown → cost 0).
#[allow(clippy::too_many_arguments)]
pub async fn execute_steps<C: ProviderClient, P: Fn(&str) -> Option<ModelPrice>>(
    client: &C,
    deployments: &[Deployment],
    routing: RoutingConfig,
    mission_id: Uuid,
    plan_id: Uuid,
    steps: &[ExecuteStep],
    system_prompt: &str,
    cfg: &ExecuteConfig,
    cancel: &AtomicBool,
    price_of: P,
) -> ExecuteReport {
    let started = std::time::Instant::now();
    let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
    let mut state = SupervisorState::new(cfg.caps, ExecutionMode::HumanInLoop);
    state = tick(
        &mut ctx,
        state,
        SupervisorEvent::MissionStarted { mission_id },
    )
    .state;
    state = tick(&mut ctx, state, SupervisorEvent::PlanGenerated { plan_id }).state;

    let mut outcomes: Vec<StepOutcome> = Vec::new();
    let mut halted: Option<String> = None;

    for step in steps {
        if cancel.load(Ordering::Relaxed) {
            halted = Some("cancelled".into());
            break;
        }
        if cfg.caps.exceeded(&state.budget_tally) {
            halted = Some("budget cap reached (pre-step)".into());
            break;
        }

        let request = ChatRequest {
            model: step.model_id.clone(),
            messages: vec![
                ChatMessage::system(system_prompt),
                ChatMessage::user(step.statement.clone()),
            ],
            temperature: None,
            max_tokens: None,
        };

        let step_started = std::time::Instant::now();
        match call_with_cascade(
            client,
            routing.clone(),
            &step.model_id,
            deployments,
            &request,
            cfg.max_attempts,
        )
        .await
        {
            Ok(outcome) => {
                let latency_ms = step_started.elapsed().as_millis() as i64;
                let usage = outcome.response.usage;
                let cost = cost_of(usage, price_of(&step.model_id));
                outcomes.push(StepOutcome {
                    step_id: step.id.clone(),
                    model_id: step.model_id.clone(),
                    deployment_id: outcome.deployment_id.clone(),
                    content: outcome.response.content.clone(),
                    attempts: outcome.attempts,
                    usage,
                    cost_usd: cost,
                    latency_ms,
                });
                // Feed the supervisor so the budget tally (iterations + cost)
                // advances and a cap can trip (`HaltSession`).
                let out = tick(
                    &mut ctx,
                    state,
                    SupervisorEvent::ToolCall {
                        tool: "model".into(),
                        input_hash: format!("{}:{}", step.id, outcome.deployment_id),
                        cost_usd: cost,
                        ts: chrono::Utc::now().to_rfc3339(),
                    },
                );
                state = out.state;
                if let Some(reason) = out.actions.iter().find_map(|a| match a {
                    SupervisorAction::HaltSession { reason, .. } => Some(reason.clone()),
                    _ => None,
                }) {
                    halted = Some(reason);
                    break;
                }
            }
            Err(err) => {
                halted = Some(format!("step `{}` failed: {err}", step.id));
                break;
            }
        }
    }

    ExecuteReport {
        mission_id,
        outcomes,
        halted,
        tally: state.budget_tally,
        phase: state.phase,
        elapsed_ms: started.elapsed().as_millis() as i64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::client::{ChatResponse, ClientError, ClientResult};
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

    fn ok(content: &str) -> ChatResponse {
        ChatResponse {
            model: "m".into(),
            content: content.into(),
            usage: None,
        }
    }

    fn dep(model: &str, id: &str) -> Deployment {
        let mut d = Deployment::new(model, format!("http://{id}"));
        d.id = id.into();
        d
    }

    fn steps() -> Vec<ExecuteStep> {
        vec![
            ExecuteStep {
                id: "s1".into(),
                model_id: "m1".into(),
                statement: "do one".into(),
            },
            ExecuteStep {
                id: "s2".into(),
                model_id: "m2".into(),
                statement: "do two".into(),
            },
        ]
    }

    fn no_price(_: &str) -> Option<ModelPrice> {
        None
    }

    #[tokio::test]
    async fn executes_all_steps_when_they_succeed() {
        let deployments = vec![dep("m1", "d1"), dep("m2", "d2")];
        let client = Scripted::new(vec![Ok(ok("a")), Ok(ok("b"))]);
        let report = execute_steps(
            &client,
            &deployments,
            RoutingConfig::default(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            &steps(),
            "sys",
            &ExecuteConfig::default(),
            &AtomicBool::new(false),
            no_price,
        )
        .await;
        assert_eq!(report.outcomes.len(), 2);
        assert!(report.halted.is_none());
        assert_eq!(report.outcomes[0].deployment_id, "d1");
        assert_eq!(report.outcomes[1].deployment_id, "d2");
    }

    #[tokio::test]
    async fn halts_when_cancelled() {
        let deployments = vec![dep("m1", "d1")];
        let client = Scripted::new(vec![Ok(ok("a"))]);
        let report = execute_steps(
            &client,
            &deployments,
            RoutingConfig::default(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            &steps(),
            "sys",
            &ExecuteConfig::default(),
            &AtomicBool::new(true),
            no_price,
        )
        .await;
        assert!(report.outcomes.is_empty());
        assert_eq!(report.halted.as_deref(), Some("cancelled"));
    }

    #[tokio::test]
    async fn halts_on_budget_cap() {
        let deployments = vec![dep("m1", "d1"), dep("m2", "d2")];
        let client = Scripted::new(vec![Ok(ok("a")), Ok(ok("b"))]);
        let cfg = ExecuteConfig {
            max_attempts: 5,
            caps: BudgetCaps {
                max_iterations: 1,
                max_minutes: 0,
                max_cost_usd: 0.0,
                max_consecutive_failures: 0,
                max_context_compactions: 0,
            },
        };
        let report = execute_steps(
            &client,
            &deployments,
            RoutingConfig::default(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            &steps(),
            "sys",
            &cfg,
            &AtomicBool::new(false),
            no_price,
        )
        .await;
        assert_eq!(
            report.outcomes.len(),
            1,
            "one step runs, then the cap trips"
        );
        assert!(report.halted.is_some());
        assert_eq!(report.phase, MissionPhase::Halted);
    }

    #[tokio::test]
    async fn halts_when_a_step_fails() {
        let deployments = vec![dep("m1", "d1")];
        let client = Scripted::new(vec![Err(ClientError::Http("boom".into()))]);
        let report = execute_steps(
            &client,
            &deployments,
            RoutingConfig::default(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            &steps(),
            "sys",
            &ExecuteConfig::default(),
            &AtomicBool::new(false),
            no_price,
        )
        .await;
        assert!(report.halted.is_some());
        assert!(report.outcomes.is_empty());
    }

    #[tokio::test]
    async fn computes_cost_from_usage_and_price() {
        let deployments = vec![dep("m1", "d1")];
        let client = Scripted::new(vec![Ok(ChatResponse {
            model: "m1".into(),
            content: "x".into(),
            usage: Some(Usage {
                prompt_tokens: 1_000_000,
                completion_tokens: 500_000,
            }),
        })]);
        let price = |_: &str| {
            Some(ModelPrice {
                input_per_1m: 3.0,
                output_per_1m: 15.0,
            })
        };
        let report = execute_steps(
            &client,
            &deployments,
            RoutingConfig::default(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            &steps()[..1],
            "sys",
            &ExecuteConfig::default(),
            &AtomicBool::new(false),
            price,
        )
        .await;
        // 1M in * 3 + 0.5M out * 15 = 3 + 7.5 = 10.5
        assert!((report.outcomes[0].cost_usd - 10.5).abs() < 1e-9);
        assert!((report.tally.cost_usd - 10.5).abs() < 1e-9);
    }

    #[test]
    fn cost_of_handles_missing_inputs() {
        let p = Some(ModelPrice {
            input_per_1m: 1.0,
            output_per_1m: 1.0,
        });
        assert_eq!(cost_of(None, p), 0.0);
        assert_eq!(cost_of(None, None), 0.0);
        assert_eq!(
            cost_of(
                Some(Usage {
                    prompt_tokens: 2_000_000,
                    completion_tokens: 0
                }),
                None
            ),
            0.0
        );
    }
}
