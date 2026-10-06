// Atlas OS — MCP → ToolRegistry bridge (RFC 07 §6/§10, RFC 63 §4).
//
// Exposes each allowlisted MCP tool as an agent `Tool`. MCP clients are async
// and `Tool::execute` is synchronous, so the clients live on a dedicated worker
// thread with their own current-thread tokio runtime; `execute` blocks on a
// channel to that worker. It never calls `block_on` inside the caller's runtime
// (the toast dispatcher uses the same pattern).
//
// Fail-safe: a server that fails to connect is skipped (never fatal), and the
// per-server allowlist is enforced by the client (`tools_call_checked`).

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::mcp::client::{McpClient, StdioTransport};
use crate::mcp::config::{McpRegistry, McpServerConfig};
use crate::orchestrator::tools::{Tool, ToolContext, ToolResult};
use crate::security::sandbox::SensitiveAction;

/// One allowlisted tool of a connected server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpToolDescriptor {
    pub server: String,
    pub tool: String,
    pub description: String,
}

enum Request {
    Call {
        server: String,
        tool: String,
        args: serde_json::Value,
        reply: Sender<Result<String, String>>,
    },
}

/// RFC 07 §9: a tool that fails this many times in a row is rotated out for the
/// rest of the process; a success resets the counter.
const MAX_CONSECUTIVE_FAILURES: u32 = 3;

/// Handle to the MCP worker thread. Shared by every `McpTool` of the bridge.
pub struct McpBridge {
    tx: Sender<Request>,
    descriptors: Vec<McpToolDescriptor>,
    /// `server\ttool` → consecutive failures (RFC 07 §9 rotation).
    failures: Mutex<HashMap<String, u32>>,
}

impl McpBridge {
    /// Connect (best-effort) to every enabled stdio server that exposes tools,
    /// list its allowlisted tools, and start the worker. Servers that fail to
    /// connect/list are skipped.
    pub fn connect(registry: &McpRegistry) -> McpBridge {
        let servers: Vec<(String, McpServerConfig)> = registry
            .names()
            .into_iter()
            .filter_map(|n| registry.get(n).map(|c| (n.to_string(), c.clone())))
            .filter(|(_, c)| c.enabled && c.exposes_any() && c.is_stdio())
            .collect();

        let (tx, rx) = mpsc::channel::<Request>();
        let (ready_tx, ready_rx) = mpsc::channel::<Vec<McpToolDescriptor>>();
        let spawned = std::thread::Builder::new()
            .name("mcp-bridge".into())
            .spawn(move || worker(servers, rx, ready_tx));
        let descriptors = match spawned {
            Ok(_) => ready_rx.recv().unwrap_or_default(),
            Err(e) => {
                tracing::warn!(error = %e, "mcp bridge: could not spawn worker");
                Vec::new()
            }
        };
        McpBridge {
            tx,
            descriptors,
            failures: Mutex::new(HashMap::new()),
        }
    }

    pub fn descriptors(&self) -> &[McpToolDescriptor] {
        &self.descriptors
    }

    /// Synchronous call used by `Tool::execute`; blocks on the worker. Enforces
    /// the RFC 07 §9 rotation: a tool that fails `MAX_CONSECUTIVE_FAILURES`
    /// times in a row is disabled (the Kernel signals the Learning Engine to
    /// lower its priority); a success clears the streak.
    pub fn call(
        &self,
        server: &str,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<String, String> {
        let key = format!("{server}\t{tool}");
        if let Ok(f) = self.failures.lock() {
            if f.get(&key).copied().unwrap_or(0) >= MAX_CONSECUTIVE_FAILURES {
                return Err(format!(
                    "MCP tool `{server}/{tool}` temporarily disabled after \
                     {MAX_CONSECUTIVE_FAILURES} consecutive failures (RFC 07 §9)"
                ));
            }
        }
        let out = self.dispatch(server, tool, args);
        if let Ok(mut f) = self.failures.lock() {
            match &out {
                Ok(_) => {
                    f.remove(&key);
                }
                Err(_) => {
                    let n = f.entry(key).or_insert(0);
                    *n += 1;
                    if *n >= MAX_CONSECUTIVE_FAILURES {
                        tracing::warn!(
                            server,
                            tool,
                            failures = *n,
                            "mcp bridge: tool rotated out (RFC 07 §9); signal Learning Engine to lower its priority"
                        );
                    }
                }
            }
        }
        out
    }

    fn dispatch(
        &self,
        server: &str,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<String, String> {
        let (reply, rx) = mpsc::channel();
        self.tx
            .send(Request::Call {
                server: server.to_string(),
                tool: tool.to_string(),
                args: args.clone(),
                reply,
            })
            .map_err(|_| "MCP worker is gone".to_string())?;
        rx.recv().map_err(|_| "MCP worker is gone".to_string())?
    }
}

fn worker(
    servers: Vec<(String, McpServerConfig)>,
    rx: Receiver<Request>,
    ready_tx: Sender<Vec<McpToolDescriptor>>,
) {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::warn!(error = %e, "mcp bridge: runtime build failed");
            let _ = ready_tx.send(Vec::new());
            return;
        }
    };

    rt.block_on(async move {
        let mut clients: HashMap<String, (McpClient<StdioTransport>, McpServerConfig)> =
            HashMap::new();
        let mut descriptors = Vec::new();

        for (name, cfg) in servers {
            match McpClient::connect(&name, &cfg).await {
                Ok(mut client) => match client.tools_list().await {
                    Ok(tools) => {
                        for t in tools {
                            if cfg.is_tool_allowed(&t.name) {
                                descriptors.push(McpToolDescriptor {
                                    server: name.clone(),
                                    tool: t.name.clone(),
                                    description: t.description.clone().unwrap_or_default(),
                                });
                            }
                        }
                        clients.insert(name.clone(), (client, cfg));
                    }
                    Err(e) => {
                        tracing::warn!(server = %name, error = %e, "mcp bridge: tools/list failed")
                    }
                },
                Err(e) => tracing::warn!(server = %name, error = %e, "mcp bridge: connect failed"),
            }
        }

        let _ = ready_tx.send(descriptors);

        while let Ok(req) = rx.recv() {
            let Request::Call {
                server,
                tool,
                args,
                reply,
            } = req;
            let out = match clients.get_mut(&server) {
                Some((client, cfg)) => match client.tools_call_checked(cfg, &tool, args).await {
                    Ok(o) if o.is_error => Err(format!("MCP tool `{tool}` reported an error")),
                    Ok(o) => Ok(o.text),
                    Err(e) => Err(e.to_string()),
                },
                None => Err(format!("MCP server `{server}` is not connected")),
            };
            let _ = reply.send(out);
        }
    });
}

/// An MCP tool exposed to the agent. Name/description are leaked once at
/// startup — the `Tool` trait requires `&'static str` (RFC 63 §4) and the set
/// is bounded by the configured servers.
pub struct McpTool {
    bridge: Arc<McpBridge>,
    server: String,
    tool: String,
    name: &'static str,
    description: &'static str,
}

impl McpTool {
    pub fn new(bridge: Arc<McpBridge>, d: &McpToolDescriptor) -> Self {
        let name: &'static str = Box::leak(format!("mcp.{}.{}", d.server, d.tool).into_boxed_str());
        let desc = if d.description.is_empty() {
            format!("MCP `{}` on `{}`", d.tool, d.server)
        } else {
            format!("{} (MCP {}/{})", d.description, d.server, d.tool)
        };
        let description: &'static str = Box::leak(desc.into_boxed_str());
        Self {
            bridge,
            server: d.server.clone(),
            tool: d.tool.clone(),
            name,
            description,
        }
    }
}

impl Tool for McpTool {
    fn name(&self) -> &'static str {
        self.name
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn sensitivity(&self) -> SensitiveAction {
        // MCP servers are external → network egress (RFC 18); the registry only
        // allows it when the caller opted in (RFC 07 §7).
        SensitiveAction::NetworkEgress
    }
    fn execute(&self, _ctx: &ToolContext, args: &serde_json::Value) -> ToolResult {
        let args_json = args.to_string();
        match self.bridge.call(&self.server, &self.tool, args) {
            Ok(text) => ToolResult::ok(self.name, text).with_args(args_json),
            Err(e) => ToolResult::err(self.name, e).with_args(args_json),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_maps_to_a_well_formed_tool() {
        let d = McpToolDescriptor {
            server: "context7".into(),
            tool: "resolve-library-id".into(),
            description: "docs lookup".into(),
        };
        // A bridge with no worker: the name/description mapping is pure, and a
        // call fails cleanly (never blocks/panics) because the channel's
        // receiver is already closed.
        let (tx, rx) = mpsc::channel();
        drop(rx);
        let bridge = Arc::new(McpBridge {
            tx,
            descriptors: vec![d.clone()],
            failures: Mutex::new(HashMap::new()),
        });
        let tool = McpTool::new(Arc::clone(&bridge), &d);
        assert_eq!(tool.name(), "mcp.context7.resolve-library-id");
        assert!(tool
            .description()
            .contains("MCP context7/resolve-library-id"));
        assert_eq!(tool.sensitivity(), SensitiveAction::NetworkEgress);

        let r = tool.execute(&ToolContext::new("."), &serde_json::json!({}));
        assert!(!r.ok);
        assert_eq!(r.tool, "mcp.context7.resolve-library-id");
    }

    #[test]
    fn repeated_failures_rotate_a_tool_out() {
        // RFC 07 §9: after MAX_CONSECUTIVE_FAILURES failures in a row the tool is
        // disabled without touching the worker; the message says so.
        let d = McpToolDescriptor {
            server: "s".into(),
            tool: "t".into(),
            description: String::new(),
        };
        let (tx, rx) = mpsc::channel();
        drop(rx); // every call fails (worker gone)
        let bridge = Arc::new(McpBridge {
            tx,
            descriptors: vec![d.clone()],
            failures: Mutex::new(HashMap::new()),
        });
        let tool = McpTool::new(Arc::clone(&bridge), &d);
        let ctx = ToolContext::new(".");

        for _ in 0..MAX_CONSECUTIVE_FAILURES {
            let r = tool.execute(&ctx, &serde_json::json!({}));
            assert!(!r.ok);
            assert!(!r.error.unwrap().contains("temporarily disabled"));
        }
        // The next call is refused by the rotation guard, not the worker.
        let r = tool.execute(&ctx, &serde_json::json!({}));
        assert!(!r.ok);
        assert!(
            r.error.unwrap().contains("temporarily disabled"),
            "expected the rotation guard to fire"
        );
    }
}
