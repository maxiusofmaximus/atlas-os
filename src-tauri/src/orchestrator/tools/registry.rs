// Atlas OS — ToolRegistry (RFC 63 §4, element 1).
//
// Holds every available `Tool` and enforces the RFC 18 approval policy in ONE
// place before any tool runs: a `Forbidden` sensitivity is refused, a `Confirm`
// one requires an explicit grant (the caller passes `allow_confirm`). This keeps
// the agent loop from forgetting a permission check and makes the policy
// unit-testable without spawning processes.

use std::collections::BTreeMap;

use crate::security::sandbox::{
    approval_for_with_network, Approval, SandboxLevel, SensitiveAction,
};

use super::{Tool, ToolContext, ToolError, ToolResult};

/// The registry. `sandbox_level` fixes the policy tier (RFC 18 §2).
pub struct ToolRegistry {
    tools: BTreeMap<&'static str, Box<dyn Tool>>,
    sandbox_level: SandboxLevel,
    /// When true, `Confirm`-class tools are allowed without a human in the loop
    /// (AUTOPILOT/AUTONOMOUS per RFC 21). When false, they raise `NeedsApproval`.
    allow_confirm: bool,
    /// When true, `NetworkEgress` tools are allowed (RFC 07 §7 opt-in egress:
    /// web/MCP). Default false — egress is Forbidden (RFC 18).
    allow_network: bool,
}

impl ToolRegistry {
    pub fn new(sandbox_level: SandboxLevel, allow_confirm: bool) -> Self {
        Self {
            tools: BTreeMap::new(),
            sandbox_level,
            allow_confirm,
            allow_network: false,
        }
    }

    /// Default registry: the RFC 63 §4 core tools. `Container` tier (the agent
    /// runs in a task container), confirm allowed (autonomous benchmark).
    pub fn with_core_tools() -> Self {
        let mut r = Self::new(SandboxLevel::Container, true);
        r.register(Box::new(super::fs::FsReadTool));
        r.register(Box::new(super::fs::FsWriteTool));
        r.register(Box::new(super::fs::FsEditTool));
        r.register(Box::new(super::fs::FsListTool));
        r.register(Box::new(super::exec::ExecRunTool));
        r.register(Box::new(super::code::CodeApplyDiffTool));
        r
    }

    /// Core tools plus the network tools (`web.fetch`/`web.search`). Kept
    /// separate so a caller can opt out of egress (RFC 18 `NetworkEgress`) by
    /// using `with_core_tools`, and in so it can enable browsing.
    pub fn with_web_tools() -> Self {
        let mut r = Self::with_core_tools();
        r.allow_network = true;
        r.register(Box::new(super::web::WebFetchTool));
        r.register(Box::new(super::web::WebSearchTool));
        r.register(Box::new(super::browse::BrowseOpenTool));
        r.register(Box::new(super::browse::BrowseSnapshotTool));
        r
    }

    /// Opt into network egress (RFC 07 §7). Required for `web.*`/`browse.*`/MCP
    /// tools to run; without it `NetworkEgress` is Forbidden (RFC 18).
    pub fn with_network(mut self) -> Self {
        self.allow_network = true;
        self
    }

    /// Register every allowlisted tool of a connected MCP bridge (RFC 07 §6) and
    /// opt into network egress (MCP servers are external processes).
    pub fn register_mcp(&mut self, bridge: &std::sync::Arc<crate::mcp::bridge::McpBridge>) {
        if bridge.descriptors().is_empty() {
            return;
        }
        self.allow_network = true;
        for d in bridge.descriptors() {
            self.register(Box::new(crate::mcp::bridge::McpTool::new(
                std::sync::Arc::clone(bridge),
                d,
            )));
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name(), tool);
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.tools.keys().copied().collect()
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// `(name, description)` for the prompt / `atlas agent tools`.
    pub fn catalog(&self) -> Vec<(&'static str, &'static str)> {
        self.tools
            .values()
            .map(|t| (t.name(), t.description()))
            .collect()
    }

    /// Run `tool` with `args`, enforcing the approval policy first. This is the
    /// ONLY entry point the agent loop should use.
    pub fn call(
        &self,
        tool: &str,
        ctx: &ToolContext,
        args: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let t = self
            .tools
            .get(tool)
            .ok_or_else(|| ToolError::UnknownTool(tool.to_string()))?;
        let action: SensitiveAction = t.sensitivity();
        match approval_for_with_network(self.sandbox_level, action, self.allow_network) {
            Approval::Auto => {}
            Approval::Confirm if self.allow_confirm => {}
            Approval::Confirm => {
                return Err(ToolError::NeedsApproval {
                    tool: tool.to_string(),
                    action: action.as_str(),
                })
            }
            Approval::Forbidden => {
                return Err(ToolError::Denied {
                    tool: tool.to_string(),
                    action: action.as_str(),
                })
            }
        }
        Ok(t.execute(ctx, args))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::tools::Tool;

    struct Dummy;
    impl Tool for Dummy {
        fn name(&self) -> &'static str {
            "dummy.denied"
        }
        fn description(&self) -> &'static str {
            "always forbidden under None"
        }
        fn sensitivity(&self) -> SensitiveAction {
            SensitiveAction::WriteWorkspace
        }
        fn execute(&self, _c: &ToolContext, _a: &serde_json::Value) -> ToolResult {
            ToolResult::ok(self.name(), "ran")
        }
    }

    #[test]
    fn core_registry_lists_the_expected_tools() {
        let r = ToolRegistry::with_core_tools();
        let names = r.names();
        for t in [
            "fs.read",
            "fs.write",
            "fs.edit",
            "fs.list",
            "exec.run",
            "code.apply_diff",
        ] {
            assert!(names.contains(&t), "missing {t}");
        }
        assert!(!names.contains(&"web.fetch"), "web is opt-in");
    }

    #[test]
    fn web_registry_adds_the_network_tools() {
        let r = ToolRegistry::with_web_tools();
        let names = r.names();
        for t in ["web.fetch", "web.search", "browse.open", "browse.snapshot"] {
            assert!(names.contains(&t), "missing {t}");
        }
    }

    #[test]
    fn unknown_tool_is_rejected() {
        let r = ToolRegistry::with_core_tools();
        let ctx = ToolContext::new(".");
        let err = r.call("nope", &ctx, &serde_json::json!({})).unwrap_err();
        assert!(matches!(err, ToolError::UnknownTool(_)));
    }

    #[test]
    fn forbidden_sensitivity_is_denied_by_policy() {
        // SandboxLevel::None forbids everything except read → any write tool denied.
        let mut r = ToolRegistry::new(SandboxLevel::None, true);
        r.register(Box::new(Dummy));
        let ctx = ToolContext::new(".");
        let err = r
            .call("dummy.denied", &ctx, &serde_json::json!({}))
            .unwrap_err();
        assert!(matches!(err, ToolError::Denied { .. }), "got {err:?}");
    }

    #[test]
    fn confirm_requires_approval_when_not_autonomous() {
        // VuOnly + write = Confirm; allow_confirm=false → NeedsApproval.
        let mut r = ToolRegistry::new(SandboxLevel::VuOnly, false);
        r.register(Box::new(Dummy));
        let ctx = ToolContext::new(".");
        let err = r
            .call("dummy.denied", &ctx, &serde_json::json!({}))
            .unwrap_err();
        assert!(
            matches!(err, ToolError::NeedsApproval { .. }),
            "got {err:?}"
        );
        // With allow_confirm it runs.
        let mut r2 = ToolRegistry::new(SandboxLevel::VuOnly, true);
        r2.register(Box::new(Dummy));
        assert!(r2
            .call("dummy.denied", &ctx, &serde_json::json!({}))
            .is_ok());
    }

    struct NetDummy;
    impl Tool for NetDummy {
        fn name(&self) -> &'static str {
            "net.dummy"
        }
        fn description(&self) -> &'static str {
            "egress"
        }
        fn sensitivity(&self) -> SensitiveAction {
            SensitiveAction::NetworkEgress
        }
        fn execute(&self, _c: &ToolContext, _a: &serde_json::Value) -> ToolResult {
            ToolResult::ok(self.name(), "ran")
        }
    }

    #[test]
    fn network_egress_is_forbidden_unless_opted_in() {
        let ctx = ToolContext::new(".");
        let mut r = ToolRegistry::new(SandboxLevel::Container, true);
        r.register(Box::new(NetDummy));
        assert!(
            matches!(
                r.call("net.dummy", &ctx, &serde_json::json!({}))
                    .unwrap_err(),
                ToolError::Denied { .. }
            ),
            "egress must be Forbidden by default"
        );
        let mut r2 = ToolRegistry::new(SandboxLevel::Container, true).with_network();
        r2.register(Box::new(NetDummy));
        assert!(r2.call("net.dummy", &ctx, &serde_json::json!({})).is_ok());
    }
}
