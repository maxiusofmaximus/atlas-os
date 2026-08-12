// OpenCode OS — MCP tool-capability-aware routing pre-filter (RFC 04 §7
// sub-fase 2.3, "tag pre-filter hot-path" research/29 line 242, gap G19).
//
// Before the auto-router invokes the `TaskTypeClassifier`, the
// registry's deployment candidate set should already be narrowed by:
//
//   1. RFC 04 §1 `capability_tags` (vision, tools, audio…) — a model
//      without `Capability::Vision` should never see a prompt that
//      carries an image input. Same for `Capability::ToolUse` for
//      prompts with tool calls.
//   2. Available MCP server tool capabilities (G19). The set of MCP
//      servers registered in the workspace exposes a set of tool
//      names (`run_tests`, `read_file`, …). A model that lacks
//      `Capability::ToolUse` cannot invoke any MCP tool — but more
//      importantly, a model SHOULD be invocable only when the
//      required MCP tool is reachable through some registered server.
//      The pre-filter shrinks the deployment candidate set before
//      the routing strategy even starts its score function — the
//      classifier therefore only predicts the task type, not the
//      capability mask.
//
// This module exposes the data shapes and a pure dispatcher that
// consumes a snapshot of:
//
//   * the in-memory `Registry`,
//   * the `required_capabilities` (vision, tools, audio, …),
//   * the `required_tool_names` (MCP tool identifiers that must be
//     reachable through some registered server in the workspace).
//
// The MCP server registry itself is not implemented in Phase 2.3 —
// the orchestrator runtime supplies the snapshot via the
// `McpServerCatalog` trait. The default `NoMcpCatalog` implementation
// reports no MCP tools available and reduces the pre-filter to a
// pure `capability_tags` intersection. The 2.3 tests exercise both
// surfaces (pure capability + capability-with-tools).

use std::collections::HashSet;
use std::sync::Arc;

use crate::orchestrator::provider::{Capability, ModelDescriptor};
use crate::orchestrator::registry::Registry;

/// Trait the MCP server registry implements (sub-fase 2.5+ ships the
/// real impl backed by a `Vec<McpServer>` snapshot from `AppState`).
/// Tests use the `NoMcpCatalog` default or the `StaticMcpCatalog`
/// test helper below.
pub trait McpServerCatalog: Send + Sync {
    /// Returns the set of tool names available across every
    /// registered MCP server. Empty when no servers are configured
    /// (the orchestrator's pre-filter then only honours the
    /// `capability_tags` intersection).
    fn available_tool_names(&self) -> HashSet<String>;
}

/// Default zero-MCP catalog. Used when no servers are registered
/// (fresh install, CI, headless CLI). Clonable + cheap.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoMcpCatalog;

impl McpServerCatalog for NoMcpCatalog {
    fn available_tool_names(&self) -> HashSet<String> {
        HashSet::new()
    }
}

/// In-memory catalog for tests — holds a fixed set of tool names.
/// Production code should never use this directly (the real catalog
/// is built from `AppState.mcp_servers`).
#[derive(Debug, Default, Clone)]
pub struct StaticMcpCatalog {
    pub tools: HashSet<String>,
}

impl McpServerCatalog for StaticMcpCatalog {
    fn available_tool_names(&self) -> HashSet<String> {
        self.tools.clone()
    }
}

/// Pre-filter dispatcher. Returns the registry descriptors that:
///
///   * carry every `Capability` in `required_capabilities` (vision,
///     tools, audio, …), AND
///   * have a registered MCP tool path for every entry in
///     `required_tool_names` (the catalog checks availability —
///     model→tool binding is a Phase 2.5+ concern).
///
/// Empty `required_capabilities` AND empty `required_tool_names`
/// collapse to "no filter" → returns every descriptor the registry
/// knows. This is the orchestrator's default hot-path: prompts without
/// a tool-call envelope and without an image input bypass the
/// pre-filter entirely (the auto-router calls `pre_filter` once, but
/// an empty intersection is O(N) cheap and lets the routing strategy
/// downstream run unfiltered).
pub struct McpToolFilter;

impl McpToolFilter {
    /// Run the pre-filter over the registry given the requirement
    /// snapshots. Returns a `Vec` of shared descriptors — the routing
    /// pipeline then narrows further via `RoutingStrategy::select`.
    pub fn pre_filter(
        registry: &Registry,
        required_capabilities: &[Capability],
        required_tool_names: &[String],
        catalog: &dyn McpServerCatalog,
    ) -> Vec<Arc<ModelDescriptor>> {
        let tools_available: HashSet<String> = catalog.available_tool_names();
        let tools_required: HashSet<String> = required_tool_names.iter().cloned().collect();
        // Every required tool must appear in the catalog. If any is
        // missing the search space collapses to empty — there's no
        // point in classifying the prompt when no deployment can
        // honour the required tool.
        let tools_ok = tools_required.is_empty() || tools_required.is_subset(&tools_available);
        if !tools_ok {
            return Vec::new();
        }
        registry
            .by_id
            .values()
            .filter(|d| {
                required_capabilities
                    .iter()
                    .all(|c| d.capabilities.contains(c))
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::provider::{Capability, ModelDescriptor, ProviderWire, Tier};

    fn descriptor(id: &str, caps: Vec<Capability>) -> ModelDescriptor {
        ModelDescriptor {
            id: id.into(),
            aliases: vec![],
            provider: ProviderWire::OpenAI,
            display_name: id.into(),
            tier: Tier::Paid,
            context_window: 8192,
            max_output_tokens: 4096,
            capabilities: caps,
            input_cost_per_1m_tokens: 1.0,
            output_cost_per_1m_tokens: 2.0,
            cache_read_cost_per_1m_tokens: 0.1,
            latency_ms_p50: 500,
        }
    }

    fn registry_with(models: Vec<ModelDescriptor>) -> Registry {
        let mut reg = Registry::default();
        for m in models {
            reg.by_id.insert(m.id.clone(), Arc::new(m));
        }
        reg
    }

    #[test]
    fn pre_filter_no_requirements_returns_all_descriptors() {
        let reg = registry_with(vec![
            descriptor("gpt-5", vec![Capability::Text]),
            descriptor("claude-opus-4", vec![Capability::Text, Capability::Vision]),
        ]);
        let catalog = NoMcpCatalog;
        let out = McpToolFilter::pre_filter(&reg, &[], &[], &catalog);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn pre_filter_vision_requirement_drops_non_vision_descriptors() {
        let reg = registry_with(vec![
            descriptor("gpt-5", vec![Capability::Text]),
            descriptor("claude-opus-4", vec![Capability::Text, Capability::Vision]),
            descriptor(
                "gemini-2.5-flash",
                vec![Capability::Text, Capability::Vision],
            ),
        ]);
        let catalog = NoMcpCatalog;
        let out = McpToolFilter::pre_filter(&reg, &[Capability::Vision], &[], &catalog);
        let ids: Vec<String> = out.iter().map(|d| d.id.clone()).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"claude-opus-4".to_string()));
        assert!(ids.contains(&"gemini-2.5-flash".to_string()));
        assert!(!ids.contains(&"gpt-5".to_string()));
    }

    #[test]
    fn pre_filter_tool_capability_requirement_keeps_only_tooluse_models() {
        let reg = registry_with(vec![
            descriptor("llama-3.1-8b", vec![Capability::Text]),
            descriptor("gpt-5", vec![Capability::Text, Capability::ToolUse]),
            descriptor("claude-opus-4", vec![Capability::Text, Capability::ToolUse]),
        ]);
        let catalog = NoMcpCatalog;
        let out = McpToolFilter::pre_filter(&reg, &[Capability::ToolUse], &[], &catalog);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn pre_filter_mcp_tool_requirement_empty_catalog_yields_empty() {
        let reg = registry_with(vec![descriptor("gpt-5", vec![Capability::Text])]);
        let catalog = NoMcpCatalog;
        let out = McpToolFilter::pre_filter(&reg, &[], &["run_tests".to_string()], &catalog);
        assert!(out.is_empty(), "no MCP server exposes run_tests → empty");
    }

    #[test]
    fn pre_filter_mcp_tool_requirement_with_static_catalog_returns_descriptors() {
        let reg = registry_with(vec![
            descriptor("gpt-5", vec![Capability::Text, Capability::ToolUse]),
            descriptor("gpt-5-mini", vec![Capability::Text, Capability::ToolUse]),
        ]);
        let mut tools = HashSet::new();
        tools.insert("run_tests".to_string());
        tools.insert("read_file".to_string());
        let catalog = StaticMcpCatalog { tools };
        let out = McpToolFilter::pre_filter(
            &reg,
            &[Capability::ToolUse],
            &["run_tests".to_string()],
            &catalog,
        );
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn pre_filter_mcp_partial_match_required_subset_yields_empty() {
        let reg = registry_with(vec![descriptor("gpt-5", vec![Capability::ToolUse])]);
        let mut tools = HashSet::new();
        tools.insert("run_tests".to_string());
        // catalog has run_tests but NOT read_file
        let catalog = StaticMcpCatalog { tools };
        let out = McpToolFilter::pre_filter(
            &reg,
            &[],
            &["run_tests".to_string(), "read_file".to_string()],
            &catalog,
        );
        assert!(out.is_empty(), "missing read_file tool → empty");
    }

    #[test]
    fn pre_filter_combined_vision_and_tool_requirements() {
        let reg = registry_with(vec![
            descriptor("gpt-5", vec![Capability::Text, Capability::ToolUse]),
            descriptor(
                "claude-opus-4",
                vec![Capability::Text, Capability::Vision, Capability::ToolUse],
            ),
            descriptor(
                "gemini-2.5-flash",
                vec![Capability::Text, Capability::Vision],
            ),
        ]);
        let mut tools = HashSet::new();
        tools.insert("screenshot".to_string());
        let catalog = StaticMcpCatalog { tools };
        let out = McpToolFilter::pre_filter(
            &reg,
            &[Capability::Vision, Capability::ToolUse],
            &["screenshot".to_string()],
            &catalog,
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "claude-opus-4");
    }
}
