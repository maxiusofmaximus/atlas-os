// Atlas OS — HUD approval routes (RFC 65 §3/§4; RFC 24 §6 approvals queue).
//
// The approvals queue is real: the supervisor emits `ApprovalRequest` on the
// Kernel Bus when a `Confirm`-class action needs a human (RFC 18 §2), the HUD
// lists them from the WS tail, and the operator answers here. This module serves
// the per-id answer routes and the batch route; each persists the decision (with
// its reason, RFC 67 §20 H-02) in the `approvals` table and publishes a matching
// `ApprovalDecision` event so every connected device (desktop, phone) converges.
//
//   POST /hud/approvals/{id}/approve  body: { user_id? }
//   POST /hud/approvals/{id}/deny     body: { user_id?, reason? }
//   POST /hud/approvals/batch         body: { decision, items[], user_id?, reason? }
//
// The batch answers several approvals at once and is rejected with
// `409 Conflict` when the requested decision contradicts an earlier decision or
// when two items in the batch claim the same file. The pending set itself lives
// in the WS tail / supervisor; these endpoints only persist and fan out.

use std::collections::HashMap;
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
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DecisionAck {
    pub approval_id: String,
    pub decision: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct BatchItem {
    pub approval_id: String,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BatchBody {
    pub decision: String,
    pub items: Vec<BatchItem>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchAck {
    pub decision: String,
    pub user_id: String,
    pub applied: Vec<String>,
}

fn decision_label(kind: ApprovalDecisionKind) -> &'static str {
    match kind {
        ApprovalDecisionKind::Apr => "approve",
        ApprovalDecisionKind::Deny => "deny",
        ApprovalDecisionKind::Steer => "steer",
        ApprovalDecisionKind::Fork => "fork",
    }
}

fn decision_status(kind: ApprovalDecisionKind) -> &'static str {
    match kind {
        ApprovalDecisionKind::Apr => "approved",
        ApprovalDecisionKind::Deny => "denied",
        ApprovalDecisionKind::Steer => "steer",
        ApprovalDecisionKind::Fork => "fork",
    }
}

fn emit_and_persist(
    state: &Arc<AppState>,
    approval_id: Uuid,
    kind: ApprovalDecisionKind,
    user_id: &str,
    reason: Option<&str>,
) -> Result<(), StatusCode> {
    let event = BusEvent::new(BusEventKind::ApprovalDecision {
        approval_id,
        decision: kind,
        user_id: user_id.to_string(),
    });
    state
        .publish_and_broadcast(&event)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    state
        .journal()
        .record_approval_decision(approval_id, decision_status(kind), user_id, reason)
        .map_err(|e| {
            tracing::error!(error = %e, "recording approval decision failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(())
}

fn publish_decision(
    state: &Arc<AppState>,
    approval_id: Uuid,
    kind: ApprovalDecisionKind,
    user_id: String,
    reason: Option<String>,
) -> Result<Json<DecisionAck>, StatusCode> {
    emit_and_persist(state, approval_id, kind, &user_id, reason.as_deref())?;
    Ok(Json(DecisionAck {
        approval_id: approval_id.to_string(),
        decision: decision_label(kind).to_string(),
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

pub async fn batch(
    State(state): State<Arc<AppState>>,
    Json(body): Json<BatchBody>,
) -> Result<Json<BatchAck>, StatusCode> {
    let kind = match body.decision.as_str() {
        "approve" => ApprovalDecisionKind::Apr,
        "deny" => ApprovalDecisionKind::Deny,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    if body.items.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let user_id = body.user_id.unwrap_or_else(|| "operator".into());

    let mut ids: Vec<Uuid> = Vec::with_capacity(body.items.len());
    for item in &body.items {
        let id = Uuid::parse_str(&item.approval_id).map_err(|_| StatusCode::BAD_REQUEST)?;
        if ids.contains(&id) {
            return Err(StatusCode::CONFLICT);
        }
        ids.push(id);
    }

    let mut owner: HashMap<&str, usize> = HashMap::new();
    for (index, item) in body.items.iter().enumerate() {
        for file in &item.files {
            match owner.get(file.as_str()) {
                Some(&other) if other != index => return Err(StatusCode::CONFLICT),
                Some(_) => {}
                None => {
                    owner.insert(file.as_str(), index);
                }
            }
        }
    }

    let target_status = decision_status(kind);
    {
        let journal = state.journal();
        for id in &ids {
            if let Some((existing, _)) = journal
                .approval_decision_row(*id)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            {
                if existing != target_status {
                    return Err(StatusCode::CONFLICT);
                }
            }
        }
    }

    for id in &ids {
        emit_and_persist(&state, *id, kind, &user_id, body.reason.as_deref())?;
    }

    Ok(Json(BatchAck {
        decision: body.decision,
        user_id,
        applied: ids.iter().map(|id| id.to_string()).collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::Journal;

    fn fresh_state(tmp: &tempfile::TempDir) -> Arc<AppState> {
        let journal = Journal::open(tmp.path()).expect("open");
        Arc::new(AppState::from_journal(journal))
    }

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

    #[tokio::test]
    async fn batch_applies_all_and_persists_reason() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let Json(ack) = batch(
            State(state.clone()),
            Json(BatchBody {
                decision: "deny".into(),
                items: vec![
                    BatchItem {
                        approval_id: a.to_string(),
                        files: vec!["src/a.rs".into()],
                    },
                    BatchItem {
                        approval_id: b.to_string(),
                        files: vec!["src/b.rs".into()],
                    },
                ],
                user_id: Some("max".into()),
                reason: Some("unsafe".into()),
            }),
        )
        .await
        .unwrap();
        assert_eq!(ack.decision, "deny");
        assert_eq!(ack.user_id, "max");
        assert_eq!(ack.applied.len(), 2);
        let row = state.journal().approval_decision_row(a).unwrap().unwrap();
        assert_eq!(row.0, "denied");
        assert_eq!(row.1.as_deref(), Some("unsafe"));
    }

    #[tokio::test]
    async fn batch_rejects_shared_file_with_conflict() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let err = batch(
            State(state),
            Json(BatchBody {
                decision: "approve".into(),
                items: vec![
                    BatchItem {
                        approval_id: a.to_string(),
                        files: vec!["src/shared.rs".into()],
                    },
                    BatchItem {
                        approval_id: b.to_string(),
                        files: vec!["src/shared.rs".into()],
                    },
                ],
                user_id: None,
                reason: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn batch_rejects_opposite_prior_decision() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let a = Uuid::new_v4();
        state
            .journal()
            .record_approval_decision(a, "denied", "max", Some("no"))
            .unwrap();
        let err = batch(
            State(state),
            Json(BatchBody {
                decision: "approve".into(),
                items: vec![BatchItem {
                    approval_id: a.to_string(),
                    files: vec![],
                }],
                user_id: None,
                reason: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(err, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn batch_rejects_empty_and_invalid_items() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state = fresh_state(&tmp);
        let empty = batch(
            State(state.clone()),
            Json(BatchBody {
                decision: "approve".into(),
                items: vec![],
                user_id: None,
                reason: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(empty, StatusCode::BAD_REQUEST);

        let invalid = batch(
            State(state),
            Json(BatchBody {
                decision: "approve".into(),
                items: vec![BatchItem {
                    approval_id: "not-a-uuid".into(),
                    files: vec![],
                }],
                user_id: None,
                reason: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(invalid, StatusCode::BAD_REQUEST);
    }
}
