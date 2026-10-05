// Atlas OS — HUD approval routes (RFC 65 §3/§4; RFC 24 §6 approvals queue).
//
// The approvals queue is real: the supervisor emits `ApprovalRequest` on the
// Kernel Bus when a `Confirm`-class action needs a human (RFC 18 §2), the HUD
// lists them from the WS tail, and the operator answers here. This module serves
// the two answer routes; each publishes a matching `ApprovalDecision` event so
// every connected device (desktop, phone) converges.
//
//   POST /hud/approvals/{id}/approve  body: { user_id? }
//   POST /hud/approvals/{id}/deny     body: { user_id?, reason? }
//
// The endpoints are stateless about the request itself (the pending set lives in
// the WS tail / supervisor); they only fan the decision out on the bus.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::bus::{ApprovalDecisionKind, BusEvent, BusEventKind};
use crate::core::state::AppState;

#[derive(Debug, Deserialize, Default)]
pub struct DecisionBody {
    /// Operator handle recorded on the decision (defaults to `operator`).
    #[serde(default)]
    pub user_id: Option<String>,
    /// Free-form reason (deny path).
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DecisionAck {
    pub approval_id: String,
    pub decision: String,
    pub user_id: String,
}

fn publish_decision(
    state: &Arc<AppState>,
    approval_id: Uuid,
    kind: ApprovalDecisionKind,
    user_id: String,
    reason: Option<String>,
) -> Result<Json<DecisionAck>, StatusCode> {
    let decision = match &kind {
        ApprovalDecisionKind::Apr => "approve",
        ApprovalDecisionKind::Deny => "deny",
        ApprovalDecisionKind::Steer => "steer",
        ApprovalDecisionKind::Fork => "fork",
    };
    let event = BusEvent::new(BusEventKind::ApprovalDecision {
        approval_id,
        decision: kind,
        user_id: user_id.clone(),
    });
    state
        .publish_and_broadcast(&event)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let _ = reason; // recorded by the supervisor; the decision event carries the outcome
    Ok(Json(DecisionAck {
        approval_id: approval_id.to_string(),
        decision: decision.to_string(),
        user_id,
    }))
}

pub async fn approve(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Option<Json<DecisionBody>>,
) -> Result<Json<DecisionAck>, StatusCode> {
    let approval_id = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let user_id = body
        .and_then(|b| b.0.user_id)
        .unwrap_or_else(|| "operator".into());
    publish_decision(
        &state,
        approval_id,
        ApprovalDecisionKind::Apr,
        user_id,
        None,
    )
}

pub async fn deny(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Option<Json<DecisionBody>>,
) -> Result<Json<DecisionAck>, StatusCode> {
    let approval_id = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let (user_id, reason) = body
        .map(|b| (b.0.user_id, b.0.reason))
        .unwrap_or((None, None));
    let user_id = user_id.unwrap_or_else(|| "operator".into());
    publish_decision(
        &state,
        approval_id,
        ApprovalDecisionKind::Deny,
        user_id,
        reason,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_defaults_are_none() {
        let b: DecisionBody = serde_json::from_str("{}").unwrap();
        assert!(b.user_id.is_none());
        assert!(b.reason.is_none());
    }

    #[test]
    fn body_parses_user_and_reason() {
        let b: DecisionBody =
            serde_json::from_str(r#"{"user_id":"max","reason":"unsafe"}"#).unwrap();
        assert_eq!(b.user_id.as_deref(), Some("max"));
        assert_eq!(b.reason.as_deref(), Some("unsafe"));
    }
}
