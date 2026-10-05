// Atlas OS — Terminal agent loop (Fase 39).
//
// Gives the coding loop the one capability Terminal-Bench tasks require: running
// shell commands. The model is asked to reply with EITHER a JSON tool call
// (`{"tool":"run_command","command":"..."}`) or a final answer
// (`{"done":true,"summary":"..."}`). We execute the command in the workspace,
// feed stdout/stderr back, and repeat until the model declares done or the step
// budget runs out.
//
// The command executor is pure-ish (a process spawn, no model), so the loop is
// unit-tested offline with a scripted mock client; production wires the real
// HttpProviderClient and the real workspace root.

use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use serde::Deserialize;

use crate::orchestrator::call::call_with_cascade_and_denied;
use crate::orchestrator::client::{ChatMessage, ChatRequest, ProviderClient};
use crate::orchestrator::provider::Deployment;
use crate::orchestrator::routing::RoutingConfig;

/// System prompt pinning the tool protocol.
pub const AGENT_SYSTEM_PROMPT: &str = r#"You are Atlas OS operating a Linux terminal to complete the user's task. Work step by step.

Reply with EXACTLY ONE JSON object per turn, nothing else:
- To run a command:  {"tool":"run_command","command":"<shell command>"}
- When the task is complete: {"done":true,"summary":"<what you did>"}

Rules:
- You see the output (stdout+stderr) of each command; use it to decide the next one.
- Prefer non-interactive commands; avoid editors/pagers that wait for input.
- Create/modify files with shell redirection or one-shot commands.
- Verify your work with a command that proves it before declaring done.
"#;

/// One executed command and its captured output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandResult {
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    /// Render for the model: exit code + trimmed output (bounded).
    pub fn render(&self) -> String {
        const MAX: usize = 4000;
        let mut out = format!("$ {}\n(exit {})\n", self.command, self.exit_code);
        let so = trim_to(&self.stdout, MAX);
        let se = trim_to(&self.stderr, MAX);
        if !so.is_empty() {
            out.push_str(&so);
            out.push('\n');
        }
        if !se.is_empty() {
            out.push_str("[stderr]\n");
            out.push_str(&se);
            out.push('\n');
        }
        out
    }
}

fn trim_to(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.len() <= max {
        return t.to_string();
    }
    // keep head + tail (errors usually matter at the end)
    let head = &t[..max / 2];
    let tail = &t[t.len() - max / 2..];
    format!("{head}\n…(truncated)…\n{tail}")
}

/// Run a shell command in `root`, capturing stdout/stderr, bounded by `timeout`.
/// Never panics: a spawn failure becomes a synthetic non-zero result.
pub fn run_command(root: &Path, command: &str, timeout: Duration) -> CommandResult {
    let (shell, flag) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    let child = std::process::Command::new(shell)
        .arg(flag)
        .arg(command)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            return CommandResult {
                command: command.to_string(),
                exit_code: 127,
                stdout: String::new(),
                stderr: format!("failed to spawn shell: {e}"),
            }
        }
    };

    // Bounded wait: poll for completion up to `timeout`, then kill.
    let start = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => break None,
        }
    };

    let out = child.wait_with_output().ok();
    let (stdout, stderr, code) = match out {
        Some(o) => (
            String::from_utf8_lossy(&o.stdout).to_string(),
            String::from_utf8_lossy(&o.stderr).to_string(),
            o.status.code().unwrap_or(-1),
        ),
        None => (String::new(), String::new(), -1),
    };
    if status.is_none() {
        return CommandResult {
            command: command.to_string(),
            exit_code: 124,
            stdout,
            stderr: format!("{stderr}\n[timeout after {}s]", timeout.as_secs()),
        };
    }
    CommandResult {
        command: command.to_string(),
        exit_code: code,
        stdout,
        stderr,
    }
}

#[derive(Deserialize)]
struct AgentTurn {
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Option<serde_json::Value>,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    summary: String,
}

/// A parsed model turn.
#[derive(Clone, Debug, PartialEq)]
pub enum AgentAction {
    /// Legacy shell action (`{"tool":"run_command","command":"..."}`).
    Run(String),
    /// Generic tool call (`{"tool":"fs.write","args":{...}}`, RFC 63 §4).
    Call {
        tool: String,
        args: serde_json::Value,
    },
    Done(String),
    /// The reply wasn't valid protocol JSON.
    Invalid(String),
}

/// Pure: parse one model reply into an action (tolerant of fences/prose).
pub fn parse_action(content: &str) -> AgentAction {
    let s = content.trim();
    let (Some(start), Some(end)) = (s.find('{'), s.rfind('}')) else {
        return AgentAction::Invalid(content.to_string());
    };
    if end <= start {
        return AgentAction::Invalid(content.to_string());
    }
    match serde_json::from_str::<AgentTurn>(&s[start..=end]) {
        Ok(t) if t.done => AgentAction::Done(t.summary),
        // `run_command` is the shell tool; keep accepting its `command` field and
        // normalise it to the generic `exec.run` call so one code path handles it.
        Ok(t) if t.tool.as_deref() == Some("run_command") => match t.command {
            Some(c) if !c.trim().is_empty() => AgentAction::Call {
                tool: "exec.run".into(),
                args: serde_json::json!({ "command": c }),
            },
            _ => AgentAction::Invalid("run_command without command".into()),
        },
        Ok(t) if t.tool.as_deref() == Some("exec.run") => match t.command {
            Some(c) if !c.trim().is_empty() => AgentAction::Call {
                tool: "exec.run".into(),
                args: serde_json::json!({ "command": c }),
            },
            _ => match t.args {
                Some(a) => AgentAction::Call {
                    tool: "exec.run".into(),
                    args: a,
                },
                None => AgentAction::Invalid("exec.run without command".into()),
            },
        },
        Ok(t) => match t.tool {
            Some(tool) if !tool.trim().is_empty() => AgentAction::Call {
                tool,
                args: t.args.unwrap_or(serde_json::json!({})),
            },
            _ => AgentAction::Invalid(content.to_string()),
        },
        Err(_) => AgentAction::Invalid(content.to_string()),
    }
}

/// Config for one agent run.
#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub max_steps: u32,
    pub timeout: Duration,
    pub max_attempts: u8,
    /// Declarative artifact predicates that MUST pass before `done` is accepted
    /// (RFC 63 §4/§8, success_predicate). Empty = accept the model's `done`.
    pub success_predicate: Vec<crate::orchestrator::artifacts::ArtifactCheck>,
    /// `(name, description)` of every tool the model may call (RFC 63 §4). The
    /// host fills this from the `ToolRegistry`; empty = the legacy shell-only
    /// prompt (back-compat).
    pub tools: Vec<(String, String)>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 25,
            timeout: Duration::from_secs(120),
            max_attempts: 5,
            success_predicate: Vec::new(),
            tools: Vec::new(),
        }
    }
}

/// Build the system prompt for a run: the tool protocol plus the live tool
/// catalog, so the model knows exactly which tools exist (RFC 63 §4).
pub fn agent_system_prompt(cfg: &AgentConfig) -> String {
    if cfg.tools.is_empty() {
        return AGENT_SYSTEM_PROMPT.to_string();
    }
    let mut catalog = String::new();
    for (name, desc) in &cfg.tools {
        catalog.push_str(&format!("- {name}: {desc}\n"));
    }
    format!(
        "{AGENT_SYSTEM_PROMPT}\n\
         Available tools (reply with {{\"tool\":\"<name>\",\"args\":{{...}}}}):\n{catalog}\n\
         You may also reply {{\"done\":true,\"summary\":\"...\"}}.\n"
    )
}

/// Result of an agent run.
#[derive(Clone, Debug)]
pub struct AgentOutcome {
    /// Every tool call's structured result, in order (RFC 63 §6). Shell commands
    /// arrive as `exec.run` results here — `CommandResult` is derived from them.
    pub tool_results: Vec<crate::orchestrator::tools::ToolResult>,
    pub done: bool,
    pub summary: String,
    pub turns: u32,
    /// Per-turn forensic detail (RFC 63 §6): the observation the model saw and
    /// the verdict the evidence gate returned. One entry per completed turn,
    /// including the final `done` turn. Persisted to `agent_steps` by the host.
    pub turn_records: Vec<AgentTurnRecord>,
}

impl AgentOutcome {
    /// Shell steps (the `exec.run` results) rendered back as `CommandResult`, for
    /// callers that only care about commands.
    pub fn steps(&self) -> Vec<CommandResult> {
        self.tool_results
            .iter()
            .filter(|r| r.tool == "exec.run")
            .map(|r| CommandResult {
                command: r.args_command().unwrap_or_default(),
                exit_code: r.exit_code.unwrap_or(if r.ok { 0 } else { 1 }),
                stdout: r.output.clone(),
                stderr: r.error.clone().unwrap_or_default(),
            })
            .collect()
    }
}

/// Forensic record of one loop turn, ready to persist to M49/M50.
#[derive(Clone, Debug, PartialEq)]
pub struct AgentTurnRecord {
    /// 0-based turn index.
    pub turn: u32,
    /// The raw model reply for this turn.
    pub thought: String,
    /// A compact action label: `run_command`, `done`, or `invalid`.
    pub action: String,
    /// What the executor/model observed (command output, verification failures).
    pub observation: Option<String>,
    /// Evidence verdict when a predicate ran this turn (`pass`/`fail`/`unknown`).
    pub verdict: Option<String>,
    pub tokens_in: i64,
    pub tokens_out: i64,
}

fn usage_tokens(u: Option<&crate::orchestrator::client::Usage>) -> (i64, i64) {
    match u {
        Some(u) => (u.prompt_tokens as i64, u.completion_tokens as i64),
        None => (0, 0),
    }
}

/// Drive the model↔terminal loop over `task`. Generic over `ProviderClient` so
/// the loop itself is offline-testable; `run` is injectable for the executor.
#[allow(clippy::too_many_arguments)]
pub async fn run_agent<C, F, V>(
    client: &C,
    routing: RoutingConfig,
    model: &str,
    deployments: &[Deployment],
    task: &str,
    _root: &Path,
    cfg: &AgentConfig,
    denied: &[String],
    exec: F,
    verify: V,
) -> Result<AgentOutcome, crate::orchestrator::code::StepDiffError>
where
    C: ProviderClient,
    F: Fn(&str, &serde_json::Value) -> crate::orchestrator::tools::ToolResult,
    V: Fn() -> crate::orchestrator::artifacts::VerifyVerdict,
{
    let mut messages = vec![
        ChatMessage::system(agent_system_prompt(cfg)),
        ChatMessage::user(task),
    ];
    let mut tool_results: Vec<crate::orchestrator::tools::ToolResult> = Vec::new();
    let mut turn_records: Vec<AgentTurnRecord> = Vec::new();

    for turn in 0..cfg.max_steps {
        let request = ChatRequest {
            model: model.to_string(),
            messages: messages.clone(),
            temperature: None,
            max_tokens: None,
        };
        let out = call_with_cascade_and_denied(
            client,
            routing.clone(),
            model,
            deployments,
            &request,
            cfg.max_attempts,
            denied,
        )
        .await?;
        let content = out.response.content.clone();
        let (tokens_in, tokens_out) = usage_tokens(out.response.usage.as_ref());
        messages.push(ChatMessage::assistant(content.clone()));

        match parse_action(&content) {
            AgentAction::Done(summary) => {
                // RFC 63 §8: `done` only counts if the success predicate holds.
                if cfg.success_predicate.is_empty() {
                    turn_records.push(AgentTurnRecord {
                        turn,
                        thought: content,
                        action: "done".into(),
                        observation: Some(summary.clone()),
                        verdict: None,
                        tokens_in,
                        tokens_out,
                    });
                    return Ok(AgentOutcome {
                        tool_results,
                        done: true,
                        summary,
                        turns: turn + 1,
                        turn_records,
                    });
                }
                let verdict = verify();
                if verdict.allowed {
                    turn_records.push(AgentTurnRecord {
                        turn,
                        thought: content,
                        action: "done".into(),
                        observation: Some(summary.clone()),
                        verdict: Some("pass".into()),
                        tokens_in,
                        tokens_out,
                    });
                    return Ok(AgentOutcome {
                        tool_results,
                        done: true,
                        summary,
                        turns: turn + 1,
                        turn_records,
                    });
                }
                // Evidence missing: tell the model exactly what failed and loop.
                let failures = verdict
                    .failures()
                    .iter()
                    .map(|r| format!("- {}", r.detail))
                    .collect::<Vec<_>>()
                    .join("\n");
                turn_records.push(AgentTurnRecord {
                    turn,
                    thought: content,
                    action: "done".into(),
                    observation: Some(format!("evidence gate rejected done:\n{failures}")),
                    verdict: Some("fail".into()),
                    tokens_in,
                    tokens_out,
                });
                messages.push(ChatMessage::user(format!(
                    "You declared done but the required evidence does not hold yet:\n{failures}\n\n\
                     Fix it and reply with one JSON object (a tool call or done)."
                )));
            }
            AgentAction::Run(cmd) => {
                // Defensive: `parse_action` normalises shell calls to `exec.run`
                // `Call`, but a hand-built `Run` (tests) still routes through the
                // executor as a shell command.
                let args = serde_json::json!({ "command": cmd });
                let args_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".into());
                let result = exec("exec.run", &args).with_args(args_json);
                let rendered = result.render();
                turn_records.push(AgentTurnRecord {
                    turn,
                    thought: content,
                    action: "run_command".into(),
                    observation: Some(rendered.clone()),
                    verdict: None,
                    tokens_in,
                    tokens_out,
                });
                tool_results.push(result);
                messages.push(ChatMessage::user(format!(
                    "Tool `exec.run` output:\n{rendered}\n\nWhat next? Reply with one JSON object."
                )));
            }
            AgentAction::Call { tool, args } => {
                let args_json = serde_json::to_string(&args).unwrap_or_else(|_| "{}".into());
                let result = exec(&tool, &args).with_args(args_json);
                let rendered = result.render();
                let action = if tool == "exec.run" {
                    "run_command".to_string()
                } else {
                    tool.clone()
                };
                turn_records.push(AgentTurnRecord {
                    turn,
                    thought: content,
                    action,
                    observation: Some(rendered.clone()),
                    verdict: None,
                    tokens_in,
                    tokens_out,
                });
                tool_results.push(result);
                messages.push(ChatMessage::user(format!(
                    "Tool `{tool}` output:\n{rendered}\n\nWhat next? Reply with one JSON object."
                )));
            }
            AgentAction::Invalid(reply) => {
                turn_records.push(AgentTurnRecord {
                    turn,
                    thought: content,
                    action: "invalid".into(),
                    observation: Some(reply),
                    verdict: None,
                    tokens_in,
                    tokens_out,
                });
                messages.push(ChatMessage::user(
                    "Your reply was not valid protocol JSON. Reply with EXACTLY one object:\n\
                     {\"tool\":\"<name>\",\"args\":{...}}  or  {\"done\":true,\"summary\":\"...\"}"
                        .to_string(),
                ));
            }
        }
    }

    Ok(AgentOutcome {
        tool_results,
        done: false,
        summary: format!("step budget ({}) exhausted", cfg.max_steps),
        turns: cfg.max_steps,
        turn_records,
    })
}

/// Convenience: run the loop with the real tool executor rooted at `root`.
/// `registry` is the live `ToolRegistry` (RFC 63 §4); when `None`, a core
/// registry is built so the agent still gets fs/exec/code.
#[allow(clippy::too_many_arguments)]
pub async fn run_agent_real<C: ProviderClient>(
    client: &C,
    routing: RoutingConfig,
    model: &str,
    deployments: &[Deployment],
    task: &str,
    root: &Path,
    cfg: &AgentConfig,
    denied: &[String],
    registry: Option<crate::orchestrator::tools::ToolRegistry>,
) -> Result<AgentOutcome, crate::orchestrator::code::StepDiffError> {
    let owned = root.to_path_buf();
    let exec_root = owned.clone();
    let verify_root = owned.clone();
    let timeout = cfg.timeout;
    let checks = cfg.success_predicate.clone();
    // RFC 63 §4: one sandbox runtime for both the tools and the verifier.
    let sandbox: std::sync::Arc<dyn crate::orchestrator::sandbox::Sandbox> =
        std::sync::Arc::from(crate::orchestrator::sandbox::resolve_sandbox());
    let tool_sandbox = sandbox.clone();
    let registry =
        registry.unwrap_or_else(crate::orchestrator::tools::ToolRegistry::with_core_tools);
    run_agent(
        client,
        routing,
        model,
        deployments,
        task,
        &owned,
        cfg,
        denied,
        move |tool, args| {
            let ctx = crate::orchestrator::tools::ToolContext::new(exec_root.clone())
                .with_sandbox(tool_sandbox.clone());
            // `exec.run` keeps the loop's timeout when the model omits one.
            let mut args = args.clone();
            if tool == "exec.run" {
                if let Some(obj) = args.as_object_mut() {
                    obj.entry("timeout_ms")
                        .or_insert_with(|| serde_json::json!(timeout.as_millis() as u64));
                }
            }
            match registry.call(tool, &ctx, &args) {
                Ok(r) => r,
                Err(e) => crate::orchestrator::tools::ToolResult::err(tool, e.to_string()),
            }
        },
        move || {
            if checks.is_empty() {
                crate::orchestrator::artifacts::VerifyVerdict {
                    allowed: true,
                    results: Vec::new(),
                    summary: "no predicate".into(),
                }
            } else {
                crate::orchestrator::artifacts::verify_artifacts(
                    &verify_root,
                    &checks,
                    sandbox.as_ref(),
                )
            }
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_run_command_as_an_exec_call() {
        assert_eq!(
            parse_action(r#"{"tool":"run_command","command":"ls -la"}"#),
            AgentAction::Call {
                tool: "exec.run".into(),
                args: serde_json::json!({ "command": "ls -la" })
            }
        );
    }

    #[test]
    fn parses_a_generic_tool_call() {
        assert_eq!(
            parse_action(r#"{"tool":"fs.write","args":{"path":"a.txt","content":"hi"}}"#),
            AgentAction::Call {
                tool: "fs.write".into(),
                args: serde_json::json!({ "path": "a.txt", "content": "hi" })
            }
        );
    }

    #[test]
    fn parses_done() {
        assert_eq!(
            parse_action(r#"{"done":true,"summary":"wrote file"}"#),
            AgentAction::Done("wrote file".into())
        );
    }

    #[test]
    fn tolerates_fences() {
        let r = "here:\n```json\n{\"tool\":\"run_command\",\"command\":\"pwd\"}\n```";
        assert_eq!(
            parse_action(r),
            AgentAction::Call {
                tool: "exec.run".into(),
                args: serde_json::json!({ "command": "pwd" })
            }
        );
    }

    #[test]
    fn rejects_prose() {
        assert!(matches!(
            parse_action("I will list files"),
            AgentAction::Invalid(_)
        ));
    }

    #[test]
    fn run_command_captures_output() {
        let tmp = tempfile::TempDir::new().unwrap();
        let r = run_command(tmp.path(), "echo hello", Duration::from_secs(30));
        assert_eq!(r.exit_code, 0);
        assert!(r.stdout.contains("hello"));
    }

    #[test]
    fn run_command_reports_nonzero() {
        let tmp = tempfile::TempDir::new().unwrap();
        let r = run_command(tmp.path(), "exit 3", Duration::from_secs(30));
        assert_ne!(r.exit_code, 0);
    }

    fn dep(model: &str) -> Deployment {
        let mut d = Deployment::new(model, "http://x");
        d.id = "d1".into();
        d
    }

    #[tokio::test]
    async fn loop_runs_commands_then_done() {
        use crate::orchestrator::client::{ChatResponse, ClientResult};
        use std::sync::Mutex;
        struct Scripted {
            replies: Mutex<Vec<String>>,
        }
        impl ProviderClient for Scripted {
            async fn chat(&self, _d: &Deployment, _r: &ChatRequest) -> ClientResult<ChatResponse> {
                let c = self.replies.lock().unwrap().remove(0);
                Ok(ChatResponse {
                    model: "m".into(),
                    content: c,
                    usage: None,
                })
            }
        }
        let client = Scripted {
            replies: Mutex::new(vec![
                r#"{"tool":"run_command","command":"echo hi"}"#.into(),
                r#"{"done":true,"summary":"ok"}"#.into(),
            ]),
        };
        let tmp = tempfile::TempDir::new().unwrap();
        let out = run_agent(
            &client,
            RoutingConfig::default(),
            "m",
            &[dep("m")],
            "say hi",
            tmp.path(),
            &AgentConfig::default(),
            &[],
            test_exec(tmp.path()),
            || crate::orchestrator::artifacts::VerifyVerdict {
                allowed: true,
                results: Vec::new(),
                summary: "no predicate".into(),
            },
        )
        .await
        .unwrap();
        assert!(out.done);
        assert_eq!(out.tool_results.len(), 1);
        assert_eq!(out.tool_results[0].tool, "exec.run");
        assert_eq!(out.summary, "ok");
        // Forensic trail (RFC 63 §6): one record for the command turn + the done.
        assert_eq!(out.turn_records.len(), 2);
        assert_eq!(out.turn_records[0].action, "run_command");
        assert!(out.turn_records[0].observation.is_some());
        assert_eq!(out.turn_records[1].action, "done");
    }

    /// Test executor: a core `ToolRegistry` rooted at `root`.
    fn test_exec(
        root: &Path,
    ) -> impl Fn(&str, &serde_json::Value) -> crate::orchestrator::tools::ToolResult + '_ {
        let reg = crate::orchestrator::tools::ToolRegistry::with_core_tools();
        let root = root.to_path_buf();
        move |tool, args| {
            let ctx = crate::orchestrator::tools::ToolContext::new(root.clone());
            reg.call(tool, &ctx, args).unwrap_or_else(|e| {
                crate::orchestrator::tools::ToolResult::err(tool, e.to_string())
            })
        }
    }

    #[tokio::test]
    async fn done_is_blocked_until_the_success_predicate_holds() {
        use crate::orchestrator::artifacts::{verify_artifacts, ArtifactCheck};
        use crate::orchestrator::client::{ChatResponse, ClientResult};
        use crate::orchestrator::sandbox::LocalSandbox;
        use std::sync::Mutex;
        struct Scripted {
            replies: Mutex<Vec<String>>,
        }
        impl ProviderClient for Scripted {
            async fn chat(&self, _d: &Deployment, _r: &ChatRequest) -> ClientResult<ChatResponse> {
                let c = self.replies.lock().unwrap().remove(0);
                Ok(ChatResponse {
                    model: "m".into(),
                    content: c,
                    usage: None,
                })
            }
        }
        // Model claims done twice; only after it actually creates the file does
        // the predicate (`FileExists out.txt`) hold. The loop must reject the
        // first `done` and accept the second.
        let client = Scripted {
            replies: Mutex::new(vec![
                r#"{"done":true,"summary":"claim"}"#.into(),
                r#"{"tool":"run_command","command":"echo ok > out.txt"}"#.into(),
                r#"{"done":true,"summary":"really done"}"#.into(),
            ]),
        };
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let cfg = AgentConfig {
            success_predicate: vec![ArtifactCheck::FileExists {
                path: "out.txt".into(),
            }],
            ..AgentConfig::default()
        };
        let vr = root.clone();
        let out = run_agent(
            &client,
            RoutingConfig::default(),
            "m",
            &[dep("m")],
            "make out.txt",
            &root,
            &cfg,
            &[],
            test_exec(&root),
            move || verify_artifacts(&vr, &cfg_predicate(), &LocalSandbox),
        )
        .await
        .unwrap();
        assert!(out.done, "second done must be accepted once evidence holds");
        assert_eq!(out.summary, "really done");
    }

    fn cfg_predicate() -> Vec<crate::orchestrator::artifacts::ArtifactCheck> {
        vec![crate::orchestrator::artifacts::ArtifactCheck::FileExists {
            path: "out.txt".into(),
        }]
    }

    #[tokio::test]
    async fn loop_routes_a_generic_tool_call_through_the_registry() {
        // RFC 63 §4: the agent calls `fs.write` (not `run_command`) and the
        // registry executes it. Proves the loop is tool-generic.
        use crate::orchestrator::client::{ChatResponse, ClientResult};
        use std::sync::Mutex;
        struct Scripted {
            replies: Mutex<Vec<String>>,
        }
        impl ProviderClient for Scripted {
            async fn chat(&self, _d: &Deployment, _r: &ChatRequest) -> ClientResult<ChatResponse> {
                let c = self.replies.lock().unwrap().remove(0);
                Ok(ChatResponse {
                    model: "m".into(),
                    content: c,
                    usage: None,
                })
            }
        }
        let client = Scripted {
            replies: Mutex::new(vec![
                r#"{"tool":"fs.write","args":{"path":"made.txt","content":"hi"}}"#.into(),
                r#"{"done":true,"summary":"wrote it"}"#.into(),
            ]),
        };
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let cfg = AgentConfig {
            tools: vec![("fs.write".into(), "write a file".into())],
            success_predicate: vec![crate::orchestrator::artifacts::ArtifactCheck::FileExists {
                path: "made.txt".into(),
            }],
            ..AgentConfig::default()
        };
        let vr = root.clone();
        let out = run_agent(
            &client,
            RoutingConfig::default(),
            "m",
            &[dep("m")],
            "make made.txt",
            &root,
            &cfg,
            &[],
            test_exec(&root),
            move || {
                crate::orchestrator::artifacts::verify_artifacts(
                    &vr,
                    &cfg_predicate_named("made.txt"),
                    &crate::orchestrator::sandbox::LocalSandbox,
                )
            },
        )
        .await
        .unwrap();
        assert!(out.done, "done accepted once fs.write created the file");
        assert_eq!(out.tool_results.len(), 1);
        assert_eq!(out.tool_results[0].tool, "fs.write");
        assert_eq!(out.turn_records[0].action, "fs.write");
        assert!(root.join("made.txt").exists());
    }

    fn cfg_predicate_named(path: &str) -> Vec<crate::orchestrator::artifacts::ArtifactCheck> {
        vec![crate::orchestrator::artifacts::ArtifactCheck::FileExists { path: path.into() }]
    }
}
