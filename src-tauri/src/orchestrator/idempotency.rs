// OpenCode OS — Request-frame idempotency (RFC 04 §6, sub-fase 2.1, G12).
//
// When the orchestrator cascades to a fallback deployment, the
// retried request must not silently re-execute tool calls that the
// primary already completed. `RequestFrame` is the small ledger the
// orchestrator carries between attempts:
//
// - `idempotency_key` — a v4 UUID generated per logical request
//   (UUID per `Cargo.toml` v1.11; we chose UUID over ULID so the key
//   is sortable-independent and matches the journal's existing PK
//   convention). The orchestrator sends this as the provider's
//   idempotency-key header when supported (OpenAI, Anthropic, Stripe-
//   style backends) so a replay of the same request after a 5xx is
//   deduplicated by the provider.
// - `executed_tool_calls` — the IDs of tool calls the primary has
//   already received full or partial results for, in order. The
//   fallback request's prompt is patched to exclude those entries
//   from the assistant-tool-call block and to instruct the model to
//   continue from where the primary left off.
// - `tool_calls_complete` — true when every tool call in the
//   `executed_tool_calls` list has a *final* result attached (no
//   pending `tool_use` awaiting `tool_result`). The cascade is
//   allowed to switch models only when this is true (or when the
//   primary produced no tool calls at all); switching mid-tool-call
//   would leak partial state and confuse the downstream tool
//   dispatcher. When `tool_calls_complete` is false and the primary
//   errored, the orchestrator surfaces a paused-mission card to the
//   HUD instead of cascading (per research/29 line 222).
//
// `RequestFrame` is intentionally `Serialize`/`Deserialize` so it can
// be persisted in the journal's `model_invocations.idempotency_key`
// column (M21) and replayed on restart.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::orchestrator::wire::OpShape;

/// Ledger carried between orchestrator attempts so the fallback
/// request can be patched to skip tool calls the primary already
/// completed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestFrame {
    /// v4 UUID generated per logical request. Used as the wire-level
    /// idempotency key for providers that support it.
    pub idempotency_key: Uuid,
    /// IDs of tool calls the primary has already executed, in order.
    /// The fallback prompt is patched to exclude these from the
    /// assistant's tool-call block.
    pub executed_tool_calls: Vec<String>,
    /// True when every entry in `executed_tool_calls` has a *final*
    /// result attached. The cascade is only allowed to switch models
    /// when this is true (or when `executed_tool_calls` is empty).
    pub tool_calls_complete: bool,
}

impl RequestFrame {
    /// Fresh frame for a new request. Generates a v4 UUID, empty
    /// executed list, and `tool_calls_complete = true` (no calls
    /// yet means we can cascade freely).
    pub fn new() -> Self {
        Self {
            idempotency_key: Uuid::new_v4(),
            executed_tool_calls: Vec::new(),
            tool_calls_complete: true,
        }
    }

    /// Append a tool call to the executed ledger. The caller is
    /// responsible for setting `tool_calls_complete` separately when
    /// the corresponding `tool_result` arrives.
    pub fn mark_executed(&mut self, tool_call_id: impl Into<String>) {
        let id = tool_call_id.into();
        if !self.executed_tool_calls.contains(&id) {
            self.executed_tool_calls.push(id);
        }
    }

    /// Mark the ledger as having every tool call resolved with a
    /// final result. The caller invokes this when the last pending
    /// `tool_use` receives its `tool_result`.
    pub fn mark_complete(&mut self) {
        self.tool_calls_complete = true;
    }

    /// Mark the ledger as having at least one pending tool call
    /// (mid-flight). The caller invokes this when a `tool_use` block
    /// is emitted but its `tool_result` has not yet arrived.
    pub fn mark_pending(&mut self) {
        self.tool_calls_complete = false;
    }

    /// Whether the cascade is allowed to switch to a fallback model
    /// for this request. The cascade is allowed when either no tool
    /// calls have been emitted at all, or every emitted call has a
    /// final result. Mid-tool-call failures stay pinned to the same
    /// model group — the orchestrator surfaces a paused-mission card
    /// instead of cascading.
    pub fn can_cascade(&self) -> bool {
        self.executed_tool_calls.is_empty() || self.tool_calls_complete
    }

    /// Patch a list of tool calls to exclude the ones already in the
    /// ledger. Used by the orchestrator when reshaping the prompt for
    /// the fallback attempt so the model doesn't re-execute the
    /// calls already present in the journal.
    pub fn filter_unexecuted<'a>(&self, calls: &'a [OpShape]) -> Vec<&'a OpShape> {
        calls
            .iter()
            .filter(|c| !self.executed_tool_calls.contains(&c.id))
            .collect()
    }
}

impl Default for RequestFrame {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_frame_cascades_freely() {
        let f = RequestFrame::new();
        assert!(f.can_cascade());
        assert!(f.executed_tool_calls.is_empty());
        assert!(f.tool_calls_complete);
    }

    #[test]
    fn mark_executed_appends_without_duplicates() {
        let mut f = RequestFrame::new();
        f.mark_executed("call_1");
        f.mark_executed("call_2");
        f.mark_executed("call_1"); // dedupe
        assert_eq!(f.executed_tool_calls, vec!["call_1", "call_2"]);
    }

    #[test]
    fn mark_pending_blocks_cascade() {
        let mut f = RequestFrame::new();
        f.mark_executed("call_1");
        f.mark_pending();
        assert!(
            !f.can_cascade(),
            "mid-tool-call should not cascade to fallback model"
        );
    }

    #[test]
    fn mark_pending_then_complete_releases_cascade() {
        let mut f = RequestFrame::new();
        f.mark_executed("call_1");
        f.mark_pending();
        assert!(!f.can_cascade());
        f.mark_complete();
        assert!(f.can_cascade());
    }

    #[test]
    fn idempotency_key_is_unique_v4() {
        let a = RequestFrame::new();
        let b = RequestFrame::new();
        assert_ne!(a.idempotency_key, b.idempotency_key);
        assert_eq!(a.idempotency_key.get_version(), Some(uuid::Version::Random));
    }

    #[test]
    fn request_frame_roundtrips_through_serde() {
        let mut f = RequestFrame::new();
        f.mark_executed("call_1");
        f.mark_executed("call_2");
        f.mark_pending();
        let json = serde_json::to_string(&f).unwrap();
        let back: RequestFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(f, back);
    }

    #[test]
    fn can_cascade_when_no_tool_calls_emitted() {
        let mut f = RequestFrame::new();
        f.mark_pending(); // pending but no calls yet — still OK.
        assert!(f.can_cascade());
    }
}
