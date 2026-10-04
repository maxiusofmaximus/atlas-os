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
    done: bool,
    #[serde(default)]
    summary: String,
}

/// A parsed model turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentAction {
    Run(String),
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
        Ok(t) if t.tool.as_deref() == Some("run_command") => match t.command {
            Some(c) if !c.trim().is_empty() => AgentAction::Run(c),
            _ => AgentAction::Invalid("run_command without command".into()),
        },
        Ok(_) => AgentAction::Invalid(content.to_string()),
        Err(_) => AgentAction::Invalid(content.to_string()),
    }
}

/// Config for one agent run.
#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub max_steps: u32,
    pub timeout: Duration,
    pub max_attempts: u8,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 25,
            timeout: Duration::from_secs(120),
            max_attempts: 5,
        }
    }
}

/// Result of an agent run.
#[derive(Clone, Debug)]
pub struct AgentOutcome {
    pub steps: Vec<CommandResult>,
    pub done: bool,
    pub summary: String,
    pub turns: u32,
}

/// Drive the model↔terminal loop over `task`. Generic over `ProviderClient` so
/// the loop itself is offline-testable; `run` is injectable for the executor.
#[allow(clippy::too_many_arguments)]
pub async fn run_agent<C, F>(
    client: &C,
    routing: RoutingConfig,
    model: &str,
    deployments: &[Deployment],
    task: &str,
    _root: &Path,
    cfg: &AgentConfig,
    denied: &[String],
    exec: F,
) -> Result<AgentOutcome, crate::orchestrator::code::StepDiffError>
where
    C: ProviderClient,
    F: Fn(&str) -> CommandResult,
{
    let mut messages = vec![
        ChatMessage::system(AGENT_SYSTEM_PROMPT),
        ChatMessage::user(task),
    ];
    let mut steps: Vec<CommandResult> = Vec::new();

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
        messages.push(ChatMessage::assistant(content.clone()));

        match parse_action(&content) {
            AgentAction::Done(summary) => {
                return Ok(AgentOutcome {
                    steps,
                    done: true,
                    summary,
                    turns: turn + 1,
                });
            }
            AgentAction::Run(cmd) => {
                let result = exec(&cmd);
                let rendered = result.render();
                steps.push(result);
                messages.push(ChatMessage::user(format!(
                    "Command output:\n{rendered}\n\nWhat next? Reply with one JSON object."
                )));
            }
            AgentAction::Invalid(reply) => {
                messages.push(ChatMessage::user(
                    "Your reply was not valid protocol JSON. Reply with EXACTLY one object:\n\
                     {\"tool\":\"run_command\",\"command\":\"...\"}  or  {\"done\":true,\"summary\":\"...\"}"
                        .to_string(),
                ));
                let _ = reply;
            }
        }
    }

    Ok(AgentOutcome {
        steps,
        done: false,
        summary: format!("step budget ({}) exhausted", cfg.max_steps),
        turns: cfg.max_steps,
    })
}

/// Convenience: run the loop with the real shell executor rooted at `root`.
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
) -> Result<AgentOutcome, crate::orchestrator::code::StepDiffError> {
    let owned = root.to_path_buf();
    let exec_root = owned.clone();
    let timeout = cfg.timeout;
    run_agent(
        client,
        routing,
        model,
        deployments,
        task,
        &owned,
        cfg,
        denied,
        move |cmd| run_command(&exec_root, cmd, timeout),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_run_command() {
        assert_eq!(
            parse_action(r#"{"tool":"run_command","command":"ls -la"}"#),
            AgentAction::Run("ls -la".into())
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
        assert_eq!(parse_action(r), AgentAction::Run("pwd".into()));
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
            |cmd| run_command(tmp.path(), cmd, Duration::from_secs(10)),
        )
        .await
        .unwrap();
        assert!(out.done);
        assert_eq!(out.steps.len(), 1);
        assert_eq!(out.summary, "ok");
    }
}
