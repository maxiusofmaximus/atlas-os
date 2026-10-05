// Atlas OS — HUD skill hot-swap activation (RFC 65 §10; RFC 24 §8).
//
// The Skill & MCP rail's drag-to-stage becomes a real activation: this endpoint
// publishes `SkillActivated` on the Kernel Bus so the supervisor and every
// connected HUD observe the change (RFC 06 Skill Graph / RFC 24 §8 hot-swap).
//
//   POST /hud/skills/{id}/activate   body: { agent_id? }
//
// Operator-initiated activation is global scope, so `agent_id` defaults to the
// nil UUID sentinel (meaning "the operator, not a specific subagent") unless the
// caller names one. Stateless about persistence: the active set lives in the WS
// tail, exactly like the approvals queue.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::bus::{BusEvent, BusEventKind};
use crate::core::state::AppState;

/// Bound the id the same way mission ids are bounded elsewhere (RFC 28 §C).
const MAX_SKILL_ID_LEN: usize = 128;

#[derive(Debug, Deserialize, Default)]
pub struct ActivateBody {
    /// Target subagent; omitted ⇒ operator/global scope (`Uuid::nil()`).
    #[serde(default)]
    pub agent_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ActivateAck {
    pub skill_id: String,
    pub agent_id: String,
}

pub async fn activate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Option<Json<ActivateBody>>,
) -> Result<Json<ActivateAck>, StatusCode> {
    let skill_id = id.trim();
    if skill_id.is_empty() || skill_id.len() > MAX_SKILL_ID_LEN {
        return Err(StatusCode::BAD_REQUEST);
    }
    let agent_id = match body.and_then(|b| b.0.agent_id) {
        Some(raw) => Uuid::parse_str(raw.trim()).map_err(|_| StatusCode::BAD_REQUEST)?,
        None => Uuid::nil(),
    };
    let event = BusEvent::new(BusEventKind::SkillActivated {
        agent_id,
        skill_id: skill_id.to_string(),
    });
    state
        .publish_and_broadcast(&event)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ActivateAck {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_defaults_to_no_agent() {
        let b: ActivateBody = serde_json::from_str("{}").unwrap();
        assert!(b.agent_id.is_none());
    }

    #[test]
    fn body_parses_agent_id() {
        let b: ActivateBody =
            serde_json::from_str(r#"{"agent_id":"00000000-0000-0000-0000-000000000001"}"#).unwrap();
        assert_eq!(
            b.agent_id.as_deref(),
            Some("00000000-0000-0000-0000-000000000001")
        );
    }

    #[test]
    fn skill_id_bound_is_sane() {
        assert!(MAX_SKILL_ID_LEN >= 64);
    }
}
