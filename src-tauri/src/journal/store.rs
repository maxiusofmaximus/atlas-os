// OpenCode OS — Journal store: typed row representations exposed via IPC.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: i64,
    pub ts: String,
    pub kind: String,
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mission {
    pub id: Uuid,
    pub label: String,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub seq: i64,
    pub ts: String,
    pub actor: String,
    pub action: String,
    pub inputs: serde_json::Value,
    pub outputs: serde_json::Value,
}
