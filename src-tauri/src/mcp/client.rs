// Atlas OS — MCP client over a transport (RFC 07 §1/§4, Fase 29.0).
//
// The client owns the JSON-RPC request/response loop; the transport owns
// the bytes. That split lets the protocol be tested deterministically
// against an in-memory transport (no process, no flake) while the real
// `StdioTransport` spawns the operator's server as a child process with
// `kill_on_drop`, so a dropped client never leaves an orphan.
//
// Only stdio is implemented (RFC 07 §1). Every request is bounded by the
// server's configured timeout; notifications and responses to other
// in-flight ids are skipped rather than mistaken for our answer.

use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use super::config::McpServerConfig;
use super::protocol::{
    initialize_params, parse_tools, tools_call_params, tools_list_params, JsonRpcNotification,
    JsonRpcRequest, JsonRpcResponse, ServerInfo, ToolCallOutcome, ToolInfo, METHOD_INITIALIZE,
    METHOD_INITIALIZED, METHOD_TOOLS_CALL, METHOD_TOOLS_LIST,
};

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("MCP server `{0}` closed the stream")]
    Closed(String),
    #[error("MCP rpc error {code}: {message}")]
    Rpc { code: i64, message: String },
    #[error("MCP request `{method}` timed out after {timeout:?}")]
    Timeout { method: String, timeout: Duration },
    #[error("tool `{0}` is not in this server's allowed_tools (RFC 07 §4)")]
    ToolNotAllowed(String),
}

/// Byte-level channel to an MCP server. One JSON object per line.
#[async_trait]
pub trait McpTransport: Send {
    async fn send(&mut self, line: &str) -> std::io::Result<()>;
    /// `Ok(None)` means the peer closed the stream.
    async fn recv(&mut self) -> std::io::Result<Option<String>>;
}

/// Spawns the server command and speaks newline-delimited JSON-RPC over
/// its stdin/stdout. stderr is discarded so a chatty server cannot stall
/// the client on a full pipe.
#[derive(Debug)]
pub struct StdioTransport {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl StdioTransport {
    pub async fn spawn(cfg: &McpServerConfig) -> std::io::Result<Self> {
        let (program, args) = resolve_launcher(&cfg.command, &cfg.args, cfg!(windows));
        let mut cmd = Command::new(&program);
        cmd.args(&args)
            .envs(&cfg.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().expect("stdin piped above");
        let stdout = BufReader::new(child.stdout.take().expect("stdout piped above"));
        Ok(Self {
            _child: child,
            stdin,
            stdout,
        })
    }
}

/// Decide the program + argv to hand to `Command::new`.
///
/// On Windows the npm/pnpm/npx shims are `.cmd` batch files, which
/// `CreateProcess` cannot execute directly — a bare name therefore
/// routes through `cmd /C` (which resolves it via `PATHEXT`). An explicit
/// `.exe`/`.com` or a path with a separator is spawned as-is. Elsewhere
/// the command is spawned directly. Pure so the decision is testable
/// without spawning anything.
fn resolve_launcher(command: &str, args: &[String], windows: bool) -> (String, Vec<String>) {
    if !windows {
        return (command.to_string(), args.to_vec());
    }
    let lower = command.to_ascii_lowercase();
    let explicit = lower.ends_with(".exe")
        || lower.ends_with(".com")
        || command.contains('\\')
        || command.contains('/');
    if explicit {
        return (command.to_string(), args.to_vec());
    }
    let mut full = Vec::with_capacity(2 + args.len());
    full.push("/C".to_string());
    full.push(command.to_string());
    full.extend(args.iter().cloned());
    ("cmd".to_string(), full)
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn send(&mut self, line: &str) -> std::io::Result<()> {
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await
    }

    async fn recv(&mut self) -> std::io::Result<Option<String>> {
        let mut buf = String::new();
        let n = self.stdout.read_line(&mut buf).await?;
        if n == 0 {
            Ok(None)
        } else {
            Ok(Some(buf.trim_end_matches(['\r', '\n']).to_string()))
        }
    }
}

/// JSON-RPC client bound to one named server.
#[derive(Debug)]
pub struct McpClient<T: McpTransport> {
    server: String,
    transport: T,
    next_id: u64,
    timeout: Duration,
}

impl<T: McpTransport> McpClient<T> {
    pub fn new(server: impl Into<String>, transport: T, timeout: Duration) -> Self {
        Self {
            server: server.into(),
            transport,
            next_id: 1,
            timeout,
        }
    }

    pub fn server_name(&self) -> &str {
        &self.server
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    async fn request(&mut self, method: &str, params: Option<Value>) -> Result<Value, McpError> {
        let id = self.next_id();
        let server = self.server.clone();
        let timeout = self.timeout;
        let line = serde_json::to_string(&JsonRpcRequest::new(id, method, params))?;

        let fut = async {
            self.transport.send(&line).await?;
            loop {
                match self.transport.recv().await? {
                    None => return Err(McpError::Closed(server)),
                    Some(l) if l.trim().is_empty() => continue,
                    Some(l) => {
                        let resp = JsonRpcResponse::parse(&l)?;
                        if resp.is_response_to(id) {
                            if let Some(err) = resp.error {
                                return Err(McpError::Rpc {
                                    code: err.code,
                                    message: err.message,
                                });
                            }
                            return Ok(resp.result.unwrap_or(Value::Null));
                        }
                    }
                }
            }
        };

        tokio::time::timeout(timeout, fut)
            .await
            .map_err(|_| McpError::Timeout {
                method: method.to_string(),
                timeout,
            })?
    }

    /// `initialize` handshake followed by the `notifications/initialized`
    /// ack (best-effort: a server that does not read it is still usable).
    pub async fn initialize(&mut self) -> Result<ServerInfo, McpError> {
        let result = self
            .request(
                METHOD_INITIALIZE,
                Some(initialize_params(env!("CARGO_PKG_VERSION"))),
            )
            .await?;
        let note = JsonRpcNotification::new(METHOD_INITIALIZED, None);
        if let Ok(line) = serde_json::to_string(&note) {
            let _ = self.transport.send(&line).await;
        }
        Ok(serde_json::from_value(result).unwrap_or_default())
    }

    pub async fn tools_list(&mut self) -> Result<Vec<ToolInfo>, McpError> {
        let result = self
            .request(METHOD_TOOLS_LIST, Some(tools_list_params()))
            .await?;
        Ok(parse_tools(&result))
    }

    pub async fn tools_call(
        &mut self,
        tool: &str,
        arguments: Value,
    ) -> Result<ToolCallOutcome, McpError> {
        let result = self
            .request(METHOD_TOOLS_CALL, Some(tools_call_params(tool, &arguments)))
            .await?;
        Ok(ToolCallOutcome::from_result(&result))
    }

    /// RFC 07 §4: refuse a tool the operator did not allowlist, before
    /// any bytes reach the server.
    pub async fn tools_call_checked(
        &mut self,
        cfg: &McpServerConfig,
        tool: &str,
        arguments: Value,
    ) -> Result<ToolCallOutcome, McpError> {
        if !cfg.is_tool_allowed(tool) {
            return Err(McpError::ToolNotAllowed(tool.to_string()));
        }
        self.tools_call(tool, arguments).await
    }
}

impl McpClient<StdioTransport> {
    /// Spawn + handshake in one call. The caller keeps the client alive
    /// for the duration of the session; dropping it kills the child.
    pub async fn connect(name: &str, cfg: &McpServerConfig) -> Result<Self, McpError> {
        cfg.validate()
            .map_err(|e| McpError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?;
        let transport = StdioTransport::spawn(cfg).await?;
        let mut client = McpClient::new(name, transport, cfg.timeout());
        client.initialize().await?;
        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::protocol::CLIENT_NAME;
    use serde_json::json;
    use std::collections::VecDeque;

    /// In-memory transport: canned inbound lines, recorded outbound.
    #[derive(Debug)]
    struct ScriptedTransport {
        inbound: VecDeque<String>,
        sent: Vec<String>,
    }

    impl ScriptedTransport {
        fn new(inbound: Vec<String>) -> Self {
            Self {
                inbound: inbound.into(),
                sent: Vec::new(),
            }
        }
    }

    #[async_trait]
    impl McpTransport for ScriptedTransport {
        async fn send(&mut self, line: &str) -> std::io::Result<()> {
            self.sent.push(line.to_string());
            Ok(())
        }
        async fn recv(&mut self) -> std::io::Result<Option<String>> {
            Ok(self.inbound.pop_front())
        }
    }

    /// Never answers — used to prove the timeout fires.
    #[derive(Debug)]
    struct PendingTransport;

    #[async_trait]
    impl McpTransport for PendingTransport {
        async fn send(&mut self, _line: &str) -> std::io::Result<()> {
            Ok(())
        }
        async fn recv(&mut self) -> std::io::Result<Option<String>> {
            std::future::pending::<()>().await;
            Ok(None)
        }
    }

    fn client(inbound: Vec<String>) -> McpClient<ScriptedTransport> {
        McpClient::new(
            "test",
            ScriptedTransport::new(inbound),
            Duration::from_secs(2),
        )
    }

    #[tokio::test]
    async fn initialize_sends_handshake_and_parses_server_info() {
        let mut c = client(vec![
            r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","serverInfo":{"name":"context7","version":"3.2.4"}}}"#.into(),
        ]);
        let info = c.initialize().await.unwrap();
        assert_eq!(info.display_name(), "context7 3.2.4");
        // First outbound line is `initialize`, second is the notification.
        let first: Value = serde_json::from_str(&c.transport.sent[0]).unwrap();
        assert_eq!(first["method"], "initialize");
        assert_eq!(first["params"]["clientInfo"]["name"], CLIENT_NAME);
        let second: Value = serde_json::from_str(&c.transport.sent[1]).unwrap();
        assert_eq!(second["method"], "notifications/initialized");
        assert!(second.get("id").is_none());
    }

    #[tokio::test]
    async fn tools_list_parses_entries() {
        let mut c = client(vec![
            r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"get_library_docs","description":"d"}]}}"#.into(),
        ]);
        let tools = c.tools_list().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "get_library_docs");
    }

    #[tokio::test]
    async fn tools_call_parses_content() {
        let mut c = client(vec![
            r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"hello"}],"isError":false}}"#.into(),
        ]);
        let out = c.tools_call("echo", json!({"msg":"hi"})).await.unwrap();
        assert_eq!(out.text, "hello");
        assert!(!out.is_error);
        let sent: Value = serde_json::from_str(&c.transport.sent[0]).unwrap();
        assert_eq!(sent["method"], "tools/call");
        assert_eq!(sent["params"]["name"], "echo");
    }

    #[tokio::test]
    async fn skipped_notifications_do_not_confuse_the_loop() {
        let mut c = client(vec![
            r#"{"jsonrpc":"2.0","method":"notifications/progress"}"#.into(),
            r#"{"jsonrpc":"2.0","id":99,"result":{"ignored":true}}"#.into(),
            r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#.into(),
        ]);
        let tools = c.tools_list().await.unwrap();
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn closed_stream_is_reported() {
        let mut c = client(vec![]);
        let err = c.tools_list().await.unwrap_err();
        assert!(matches!(err, McpError::Closed(_)));
    }

    #[tokio::test]
    async fn rpc_error_is_surfaced() {
        let mut c = client(vec![
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"method not found"}}"#
                .into(),
        ]);
        let err = c.tools_list().await.unwrap_err();
        match err {
            McpError::Rpc { code, message } => {
                assert_eq!(code, -32601);
                assert!(message.contains("method not found"));
            }
            other => panic!("expected rpc error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn timeout_fires_on_a_silent_server() {
        let mut c = McpClient::new("slow", PendingTransport, Duration::from_millis(20));
        let err = c.tools_list().await.unwrap_err();
        assert!(matches!(err, McpError::Timeout { .. }));
    }

    #[tokio::test]
    async fn allowlist_is_enforced_before_io() {
        let mut c = client(vec![]);
        let cfg = McpServerConfig::new("cmd"); // empty allowlist
        let err = c
            .tools_call_checked(&cfg, "shell.exec", json!({}))
            .await
            .unwrap_err();
        assert!(matches!(err, McpError::ToolNotAllowed(_)));
        assert!(
            c.transport.sent.is_empty(),
            "no bytes should reach the server"
        );
    }

    #[tokio::test]
    async fn allowlist_permits_a_listed_tool() {
        let mut c = client(vec![
            r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"ok"}]}}"#.into(),
        ]);
        let mut cfg = McpServerConfig::new("cmd");
        cfg.allowed_tools = vec!["navigate".into()];
        let out = c
            .tools_call_checked(&cfg, "navigate", json!({}))
            .await
            .unwrap();
        assert_eq!(out.text, "ok");
    }

    #[tokio::test]
    async fn spawning_a_missing_binary_errors_without_panic() {
        let cfg = McpServerConfig::new("/nonexistent-atlas-mcp-xyz");
        let err = StdioTransport::spawn(&cfg).await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[tokio::test]
    async fn connect_rejects_non_stdio_before_spawning() {
        let mut cfg = McpServerConfig::new("/nonexistent-atlas-mcp-xyz");
        cfg.transport = "sse".into();
        let err = McpClient::connect("x", &cfg).await.unwrap_err();
        assert!(matches!(err, McpError::Io(_)));
    }

    #[test]
    fn windows_routes_bare_shims_through_cmd() {
        let args = vec!["dlx".to_string(), "@upstash/context7-mcp".to_string()];
        let (program, full) = resolve_launcher("pnpm", &args, true);
        assert_eq!(program, "cmd");
        assert_eq!(full, vec!["/C", "pnpm", "dlx", "@upstash/context7-mcp"]);
    }

    #[test]
    fn windows_spawns_explicit_exe_and_paths_directly() {
        let (p1, _) = resolve_launcher("node.exe", &[], true);
        assert_eq!(p1, "node.exe");
        let (p2, _) = resolve_launcher(r"C:\tools\mcp.exe", &[], true);
        assert_eq!(p2, r"C:\tools\mcp.exe");
        let (p3, a3) = resolve_launcher("/usr/bin/mcp", &["x".into()], true);
        assert_eq!(p3, "/usr/bin/mcp");
        assert_eq!(a3, vec!["x"]);
    }

    #[test]
    fn non_windows_spawns_directly() {
        let (program, args) = resolve_launcher("pnpm", &["dlx".into()], false);
        assert_eq!(program, "pnpm");
        assert_eq!(args, vec!["dlx"]);
    }
}
