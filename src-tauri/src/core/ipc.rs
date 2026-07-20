// OpenCode OS — Tauri IPC command handlers.
// Invoked from the SvelteKit frontend via `@tauri-apps/api/core::invoke`.
// Each handler is thin — persists to the Journal, fans out to the Kernel
// Bus, and returns a friendly result. Heavy work happens in the engines
// (see RFC 02 §3.1.1 SOP, Phase 1+).
use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use uuid::Uuid;

use super::bus::{BusEvent, BusEventKind};
use super::state::AppState;
use crate::journal::store::JournalEntry;

/// Create a new mission from a raw user prompt.
/// Persists to `missions`, publishes `TaskReceived` on the Kernel Bus so
/// the HUD renders it live, and returns the new mission id. Phase 0 stores
/// the raw prompt verbatim; Phase 1 routes it through the Prompt Understanding
/// Pipeline (RFC 23).
#[tauri::command]
pub async fn mission_new(
    state: State<'_, Arc<AppState>>,
    raw_prompt: String,
) -> Result<Uuid, String> {
    let mission_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();

    // Persist the mission row first — if this fails, we abort with a useful
    // error and never publish a half-state on the bus.
    state
        .journal()
        .create_mission(mission_id, raw_prompt.as_str())
        .map_err(|e| format!("failed to persist mission: {e}"))?;

    // Publish the event to journal + bus atomically (RFC 02 §3.1).
    let event = BusEvent::new(BusEventKind::TaskReceived {
        raw_prompt,
        session_id,
    });
    state
        .publish_and_broadcast(&event)
        .map_err(|e| format!("failed to publish TaskReceived: {e}"))?;

    Ok(mission_id)
}

#[derive(Serialize)]
pub struct MissionListItem {
    pub id: Uuid,
    pub label: String,
    pub status: String,
}

/// List active missions from the Journal.
#[tauri::command]
pub async fn mission_list(state: State<'_, Arc<AppState>>) -> Result<Vec<MissionListItem>, String> {
    Ok(state
        .journal()
        .list_missions()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|m| MissionListItem {
            id: m.id,
            label: m.label,
            status: m.status,
        })
        .collect())
}

#[tauri::command]
pub async fn hud_url(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    Ok(state.hud_url())
}

#[tauri::command]
pub async fn journal_tail(
    state: State<'_, Arc<AppState>>,
    last: Option<i64>,
) -> Result<Vec<JournalEntry>, String> {
    state
        .journal()
        .tail(last.unwrap_or(50))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn audit_tail(
    state: State<'_, Arc<AppState>>,
    last: Option<i64>,
) -> Result<Vec<crate::journal::store::AuditEntry>, String> {
    state
        .journal()
        .audit_tail(last.unwrap_or(50))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn approval_decide(
    state: State<'_, Arc<AppState>>,
    approval_id: Uuid,
    decision: super::bus::ApprovalDecisionKind,
    user_id: String,
) -> Result<(), String> {
    // Phase 0: log the decision; Phase 4+ routes it to the engine owning
    // the agent. We persist it via the journal audit log so it survives reboots.
    let action = format!("approval:{decision:?}");
    let _inputs = serde_json::json!({ "approval_id": approval_id, "user_id": user_id });
    let event = BusEvent::new(BusEventKind::ApprovalDecision {
        approval_id,
        decision,
        user_id: user_id.clone(),
    });
    state
        .publish_and_broadcast(&event)
        .map_err(|e| e.to_string())?;
    tracing::info!(%approval_id, %action, %user_id, "approval decision recorded");
    Ok(())
}
