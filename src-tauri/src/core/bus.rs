// OpenCode OS — Kernel Bus: typed events + commands.
// See RFC 02 §3.1 and RFC 24 §4.1 for the taxonomy of `HudEvent`s.
// Implements at-least-once delivery with `idempotency_key` (RFC 02 §3.1.2).
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusEvent {
    pub id: Uuid,
    pub idempotency_key: String,
    pub kind: BusEventKind,
    pub ts: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BusEventKind {
    TaskReceived {
        raw_prompt: String,
        session_id: Uuid,
    },
    MissionConsolidated {
        mission_id: Uuid,
        verdict_id: Uuid,
        confidence: Confidence,
    },
    MissionLocked {
        mission_id: Uuid,
        planning_session_id: Uuid,
    },
    PlanGenerated {
        plan_id: Uuid,
        mission_id: Uuid,
    },
    AgentStatusChanged {
        agent_id: Uuid,
        status: AgentStatus,
    },
    AgentDiff {
        agent_id: Uuid,
        files: u32,
        lines_added: u32,
        lines_removed: u32,
    },
    AgentTokens {
        agent_id: Uuid,
        tokens_in: u64,
        tokens_out: u64,
        cost_usd: f64,
    },
    AgentHeartbeat {
        agent_id: Uuid,
    },
    ApprovalRequest {
        approval_id: Uuid,
        agent_id: Uuid,
        action: String,
    },
    ApprovalDecision {
        approval_id: Uuid,
        decision: ApprovalDecisionKind,
        user_id: String,
    },
    DoomLoopDetected {
        agent_id: Uuid,
        count: u32,
    },
    GoalDriftDetected {
        agent_id: Uuid,
        drift: f64,
    },
    JournalCheckpoint {
        checkpoint_id: Uuid,
    },
    CostThresholdCrossed {
        agent_id: Uuid,
        threshold: f64,
        cumulative: f64,
    },
    WorktreeDirty {
        agent_id: Uuid,
        path: String,
        dirty: bool,
    },
    SkillActivated {
        agent_id: Uuid,
        skill_id: String,
    },
    ResearchCompleted {
        research_run_id: Uuid,
    },
    HudServed {
        hud_port: u16,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Confidence {
    High,
    Medium,
    Low,
    Block,
}

impl Confidence {
    pub fn threshold(self) -> f32 {
        match self {
            Confidence::High => 0.85,
            Confidence::Medium => 0.6,
            Confidence::Low => 0.3,
            Confidence::Block => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Queued,
    Reading,
    Planning,
    Coding,
    Reviewing,
    Idle,
    Paused,
    DoomLoop,
    Error,
    Success,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecisionKind {
    Apr,
    Deny,
    Steer,
    Fork,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum KernelCommand {
    NewMissionFromPrompt {
        raw_prompt: String,
    },
    SwitchProfile {
        profile_id: String,
    },
    StopAgent {
        agent_id: Uuid,
    },
    PauseAgent {
        agent_id: Uuid,
    },
    ResumeAgent {
        agent_id: Uuid,
    },
    SteerAgent {
        agent_id: Uuid,
        message: String,
    },
    ForkAgent {
        agent_id: Uuid,
    },
    DecideApproval {
        approval_id: Uuid,
        decision: ApprovalDecisionKind,
        user_id: String,
    },
}

impl BusEvent {
    pub fn new(kind: BusEventKind) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            idempotency_key: format!("evt-{id}"),
            kind,
            ts: chrono::Utc::now(),
        }
    }
}

impl BusEventKind {
    /// Stable tag string usable as a SQL `kind` column value.
    /// Mirrors the `type` discriminator used in serde JSON tagged enums.
    pub fn tag(&self) -> &'static str {
        match self {
            BusEventKind::TaskReceived { .. } => "task_received",
            BusEventKind::MissionConsolidated { .. } => "mission_consolidated",
            BusEventKind::MissionLocked { .. } => "mission_locked",
            BusEventKind::PlanGenerated { .. } => "plan_generated",
            BusEventKind::AgentStatusChanged { .. } => "agent_status",
            BusEventKind::AgentDiff { .. } => "agent_diff",
            BusEventKind::AgentTokens { .. } => "agent_tokens",
            BusEventKind::AgentHeartbeat { .. } => "agent_heartbeat",
            BusEventKind::ApprovalRequest { .. } => "approval_request",
            BusEventKind::ApprovalDecision { .. } => "approval_decision",
            BusEventKind::DoomLoopDetected { .. } => "doom_loop_detected",
            BusEventKind::GoalDriftDetected { .. } => "goal_drift_detected",
            BusEventKind::JournalCheckpoint { .. } => "journal_checkpoint",
            BusEventKind::CostThresholdCrossed { .. } => "cost_threshold_crossed",
            BusEventKind::WorktreeDirty { .. } => "worktree_dirty",
            BusEventKind::SkillActivated { .. } => "skill_activated",
            BusEventKind::ResearchCompleted { .. } => "research_completed",
            BusEventKind::HudServed { .. } => "hud_served",
        }
    }
}
