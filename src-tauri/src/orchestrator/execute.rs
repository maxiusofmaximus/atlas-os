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

use crate::coding::llm::DiffMeta;
use crate::coding::types::Diff;
use crate::orchestrator::code::{call_diff_with_cascade_and_denied, StepDiffError};
use crate::orchestrator::verify::{verify_diff, VerifyConfig};
use crate::repair::types::RepairReport;
use crate::validation::types::ValidationReport;

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

/// A code step driven end-to-end through the LLM-coding bridge: the routed
/// model's `Diff`, the `ValidationReport` it earned, and the `RepairReport` (if
/// validation did not pass).
#[derive(Clone, Debug)]
pub struct CodingStepOutcome {
    pub step_id: String,
    pub model_id: String,
    pub deployment_id: String,
    pub attempts: u8,
    pub usage: Option<Usage>,
    pub cost_usd: f64,
    pub latency_ms: i64,
    pub diff: Diff,
    pub report: ValidationReport,
    pub repair: Option<RepairReport>,
}

/// Drive ONE code step through the LLM-coding bridge (v26.1 + v26.2): ask the
/// routed model for a structured `Diff` and run it through the pure
/// Validation/Repair engines under a mission supervisor. Returns the diff and
/// reports so the caller persists + renders them (the orchestrator stays free
/// of the Journal).
#[allow(clippy::too_many_arguments)]
pub async fn execute_coding_step<C: ProviderClient, P: Fn(&str) -> Option<ModelPrice>>(
    client: &C,
    routing: RoutingConfig,
    step: &ExecuteStep,
    deployments: &[Deployment],
    mission_id: Uuid,
    plan_id: Uuid,
    agent_id: Uuid,
    max_attempts: u8,
    price_of: P,
) -> Result<CodingStepOutcome, StepDiffError> {
    execute_coding_step_denied(
        client,
        routing,
        step,
        deployments,
        mission_id,
        plan_id,
        agent_id,
        max_attempts,
        price_of,
        &[],
    )
    .await
}

/// Like `execute_coding_step`, but the reliability gate can deny model ids
/// (Fase 24): a denied model is not used as the primary nor as a failover.
#[allow(clippy::too_many_arguments)]
pub async fn execute_coding_step_denied<C: ProviderClient, P: Fn(&str) -> Option<ModelPrice>>(
    client: &C,
    routing: RoutingConfig,
    step: &ExecuteStep,
    deployments: &[Deployment],
    mission_id: Uuid,
    plan_id: Uuid,
    agent_id: Uuid,
    max_attempts: u8,
    price_of: P,
    denied: &[String],
) -> Result<CodingStepOutcome, StepDiffError> {
    let started = std::time::Instant::now();

    let meta = DiffMeta {
        mission_id,
        plan_id,
        step_id: step.id.clone(),
        agent_id,
        model_id: step.model_id.clone(),
    };
    let routed = call_diff_with_cascade_and_denied(
        client,
        routing,
        &step.model_id,
        deployments,
        &step.statement,
        meta,
        max_attempts,
        denied,
    )
    .await?;

    let mut ctx = TickContext::new(ExecutionMode::HumanInLoop);
    let state = SupervisorState::new(BudgetCaps::DEFAULT, ExecutionMode::HumanInLoop);
    let state = tick(
        &mut ctx,
        state,
        SupervisorEvent::MissionStarted { mission_id },
    )
    .state;
    let state = tick(&mut ctx, state, SupervisorEvent::PlanGenerated { plan_id }).state;
    let (_state, verify) = verify_diff(
        state,
        &mut ctx,
        &routed.diff,
        &VerifyConfig {
            model_id: step.model_id.clone(),
            ..VerifyConfig::default()
        },
    );

    let usage = routed.usage;
    Ok(CodingStepOutcome {
        step_id: step.id.clone(),
        model_id: step.model_id.clone(),
        deployment_id: routed.deployment_id,
        attempts: routed.attempts,
        usage,
        cost_usd: cost_of(usage, price_of(&step.model_id)),
        latency_ms: started.elapsed().as_millis() as i64,
        diff: routed.diff,
        report: verify.report,
        repair: verify.repair,
    })
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

    #[tokio::test]
    async fn coding_step_parses_a_diff_and_verifies_it() {
        let deployments = vec![dep("m1", "d1")];
        let client = Scripted::new(vec![Ok(ChatResponse {
            model: "m1".into(),
            content: r#"{"narrative":"adds add()","files":[{"path":"src/lib.rs","hunks":[{"old_start":0,"old_end":0,"new_lines":["pub fn add(a: u32, b: u32) -> u32 { a + b }"],"rationale":"r"}]}]}"#.into(),
            usage: None,
        })]);
        let step = ExecuteStep {
            id: "s1".into(),
            model_id: "m1".into(),
            statement: "add add()".into(),
        };
        let out = execute_coding_step(
            &client,
            RoutingConfig::default(),
            &step,
            &deployments,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            5,
            no_price,
        )
        .await
        .expect("diff parses");
        assert_eq!(out.diff.files[0].path, "src/lib.rs");
        assert_eq!(out.deployment_id, "d1");
        // Repair runs iff the diff did not earn a clean pass.
        assert_eq!(
            out.report.is_pass(),
            out.repair.is_none(),
            "a repair must be attempted exactly when validation did not pass"
        );
    }

    #[tokio::test]
    async fn coding_step_rejects_a_non_diff_reply() {
        let deployments = vec![dep("m1", "d1")];
        let client = Scripted::new(vec![Ok(ok("just prose, no json here"))]);
        let step = ExecuteStep {
            id: "s1".into(),
            model_id: "m1".into(),
            statement: "add add()".into(),
        };
        let err = execute_coding_step(
            &client,
            RoutingConfig::default(),
            &step,
            &deployments,
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            5,
            no_price,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, StepDiffError::Parse(_)), "got {err:?}");
    }
}
