// Atlas OS — HUD HTTP tail routes (RFC 24 §2 / §3 / §16).
//
// Phase 1 exposes each of the 9 Journal `*_tail` helpers as a single
// `GET /tail/<kind>` route. Each route returns the latest `N` rows
// (default 20, max 200) as a JSON array. The HUD Mission Control UI
// polls these on mount and on user refresh; the WebSocket remains the
// live-stream channel for in-flight events (RFC 24 §4).
//
// All tail routes share the same path/query-shape, so the frontend can
// implement a generic `<TailBox kind="verdicts">` component that hits
// `/tail/<kind>` and re-renders on a single fetch helper. The backend
// shares the orchestration through the `TailRow` trait — one generic
// `tail_of::<R>()` handler replaces 9 near-identical functions.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Serialize;
use serde_json::Value;

use crate::core::state::AppState;
use crate::journal::{
    AgentStepRow, CheckpointRow, ConsolidatedRow, DiffRow, Mission, ModelSwapRow, PatternRow,
    PlanRow, RepairRunRow, SkillRow, StepStateRow, ValidationReportRow, VerdictRow,
};

/// Default and maximum number of rows returned by any tail route.
pub const DEFAULT_LAST: i64 = 20;
pub const MAX_LAST: i64 = 200;

/// `?last=N` query parameter. Clamped to `[1, MAX_LAST]`; when absent
/// `DEFAULT_LAST` is used.
#[derive(Debug, serde::Deserialize)]
pub struct TailQuery {
    #[serde(default)]
    pub last: Option<i64>,
}

impl TailQuery {
    fn resolved(&self) -> i64 {
        self.last.unwrap_or(DEFAULT_LAST).clamp(1, MAX_LAST)
    }
}

type TailResult = Result<Json<Vec<Value>>, axum::http::StatusCode>;

/// Trampoline: every row already implements `Serialize`, so funnel via
/// `serde_json::Value` and let `axum::Json` handle the rest. Failed
/// serializations become `Value::Null` rather than a 500 — the HUD
/// shows a blank cell instead of discarding the whole batch.
fn to_json_value<T: Serialize>(row: T) -> Value {
    serde_json::to_value(row).unwrap_or(Value::Null)
}

/// One-fetch-per-kind abstraction. Each Journal tail method has the
/// shape `fn(&self, last: i64) -> anyhow::Result<Vec<R>>` where `R:
/// Serialize`. Implementing this trait per row type lets the route
/// module emit a single generic `tail_of::<R>` instead of 9
/// copy-pasted handlers.
pub(super) trait TailRow: Serialize + Sized {
    fn fetch(journal: &crate::journal::Journal, last: i64) -> anyhow::Result<Vec<Self>>;
}

/// Generic tail handler — the only tail-shaped thing the router needs
/// to know about. Replaces `tail_verdicts`, `tail_consolidated`,
/// `tail_plans`, `tail_diffs`, `tail_validation_reports`, `tail_repairs`,
/// `tail_patterns`, `tail_checkpoints`, `tail_skills` (9 functions).
pub(super) async fn tail_of<R: TailRow>(
    State(state): State<Arc<AppState>>,
    Query(q): Query<TailQuery>,
) -> TailResult {
    let journal = state.journal();
    let rows = R::fetch(&journal, q.resolved())
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(to_json_value).collect()))
}

// ── per-kind TailRow implementations ────────────────────────────────
// One trivial impl per Journal row type. The body is a single
// delegating call; the trait machinery guarantees that adding a new
// tail kind stays mechanical (no handler to copy-paste).

impl TailRow for VerdictRow {
    fn fetch(j: &crate::journal::Journal, n: i64) -> anyhow::Result<Vec<Self>> {
        j.verdict_tail(n)
    }
}
impl TailRow for ConsolidatedRow {
    fn fetch(j: &crate::journal::Journal, n: i64) -> anyhow::Result<Vec<Self>> {
        j.consolidated_tail(n)
    }
}
impl TailRow for PlanRow {
    fn fetch(j: &crate::journal::Journal, n: i64) -> anyhow::Result<Vec<Self>> {
        j.plan_tail(n)
    }
}
impl TailRow for DiffRow {
    fn fetch(j: &crate::journal::Journal, n: i64) -> anyhow::Result<Vec<Self>> {
        j.diff_tail(n)
    }
}
impl TailRow for ValidationReportRow {
    fn fetch(j: &crate::journal::Journal, n: i64) -> anyhow::Result<Vec<Self>> {
        j.report_tail(n)
    }
}
impl TailRow for RepairRunRow {
    fn fetch(j: &crate::journal::Journal, n: i64) -> anyhow::Result<Vec<Self>> {
        j.repair_tail(n)
    }
}
impl TailRow for PatternRow {
    fn fetch(j: &crate::journal::Journal, n: i64) -> anyhow::Result<Vec<Self>> {
        j.pattern_tail(n)
    }
}
impl TailRow for CheckpointRow {
    fn fetch(j: &crate::journal::Journal, n: i64) -> anyhow::Result<Vec<Self>> {
        j.checkpoint_tail(n)
    }
}
impl TailRow for SkillRow {
    fn fetch(j: &crate::journal::Journal, n: i64) -> anyhow::Result<Vec<Self>> {
        j.skill_tail(n)
    }
}
impl TailRow for ModelSwapRow {
    fn fetch(j: &crate::journal::Journal, n: i64) -> anyhow::Result<Vec<Self>> {
        j.model_swap_tail(n)
    }
}
impl TailRow for StepStateRow {
    fn fetch(j: &crate::journal::Journal, n: i64) -> anyhow::Result<Vec<Self>> {
        j.step_state_tail(n)
    }
}
impl TailRow for AgentStepRow {
    fn fetch(j: &crate::journal::Journal, n: i64) -> anyhow::Result<Vec<Self>> {
        j.agent_step_tail(n)
    }
}

// ── public route wrappers (kept thin so `hud/server.rs` keeps its
// existing imports; the route registration table is unchanged). ─────
//
// Each one-liner pins the row type into the generic `tail_of::<R>`
// handler. The `async fn` wrapper is needed because axum's Router
// expects `Handler<T,S>`-shaped function pointers; a bare
// `tail_of::<VerdictRow>` would also work but reads worse at call
// sites and confuses `cargo doc`.

pub async fn tail_verdicts(state: State<Arc<AppState>>, q: Query<TailQuery>) -> TailResult {
    tail_of::<VerdictRow>(state, q).await
}
pub async fn tail_consolidated(state: State<Arc<AppState>>, q: Query<TailQuery>) -> TailResult {
    tail_of::<ConsolidatedRow>(state, q).await
}
pub async fn tail_plans(state: State<Arc<AppState>>, q: Query<TailQuery>) -> TailResult {
    tail_of::<PlanRow>(state, q).await
}
pub async fn tail_diffs(state: State<Arc<AppState>>, q: Query<TailQuery>) -> TailResult {
    tail_of::<DiffRow>(state, q).await
}
pub async fn tail_validation_reports(
    state: State<Arc<AppState>>,
    q: Query<TailQuery>,
) -> TailResult {
    tail_of::<ValidationReportRow>(state, q).await
}
pub async fn tail_repairs(state: State<Arc<AppState>>, q: Query<TailQuery>) -> TailResult {
    tail_of::<RepairRunRow>(state, q).await
}
pub async fn tail_patterns(state: State<Arc<AppState>>, q: Query<TailQuery>) -> TailResult {
    tail_of::<PatternRow>(state, q).await
}
pub async fn tail_checkpoints(state: State<Arc<AppState>>, q: Query<TailQuery>) -> TailResult {
    tail_of::<CheckpointRow>(state, q).await
}
pub async fn tail_skills(state: State<Arc<AppState>>, q: Query<TailQuery>) -> TailResult {
    tail_of::<SkillRow>(state, q).await
}
pub async fn tail_model_swaps(state: State<Arc<AppState>>, q: Query<TailQuery>) -> TailResult {
    tail_of::<ModelSwapRow>(state, q).await
}
pub async fn tail_step_states(state: State<Arc<AppState>>, q: Query<TailQuery>) -> TailResult {
    tail_of::<StepStateRow>(state, q).await
}
pub async fn tail_agent_steps(state: State<Arc<AppState>>, q: Query<TailQuery>) -> TailResult {
    tail_of::<AgentStepRow>(state, q).await
}

// `tail_journal` and `tail_missions` keep dedicated handlers — the
// former returns `JournalEntry` (which has its own `tail` method
// without the `_tail` suffix and without the same `resolved` clamp
// invariants), and the latter returns `Mission` via `list_missions`
// without a `last` parameter. Keeping them dedicated avoids forcing
// the rest of the API into the trait shape just for symmetry.

pub async fn tail_journal(
    State(state): State<Arc<AppState>>,
    Query(q): Query<TailQuery>,
) -> TailResult {
    let journal = state.journal();
    let rows = journal
        .tail(q.resolved())
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(to_json_value).collect()))
}

pub async fn tail_missions(
    State(state): State<Arc<AppState>>,
    Query(_q): Query<TailQuery>,
) -> TailResult {
    let journal = state.journal();
    let rows: Vec<Mission> = journal
        .list_missions()
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(to_json_value).collect()))
}

/// `GET /payload/:kind/:id` — return raw payload string for an
/// artefact whose payload is keyed by UUID. Phase 1 covers verdicts,
/// plans, diffs, validation_reports, repairs, patterns, checkpoints,
/// (skills uses `(skill_id, version)` composite key — handled by a
/// dedicated route below). Useful for the HUD Audit drill-down.
#[derive(Debug, serde::Deserialize)]
pub struct PayloadPath {
    pub kind: String,
    pub id: uuid::Uuid,
}

pub async fn payload(
    State(state): State<Arc<AppState>>,
    Path(pp): Path<PayloadPath>,
) -> Result<String, axum::http::StatusCode> {
    let journal = state.journal();
    let raw: anyhow::Result<Option<String>> = match pp.kind.as_str() {
        "verdict" => journal.verdict_payload(pp.id),
        "plan" => journal.plan_payload(pp.id),
        "diff" => journal.diff_payload(pp.id),
        "validation_report" => journal.report_payload(pp.id),
        "repair" => journal.repair_payload(pp.id),
        "pattern" => journal.pattern_payload(pp.id),
        "checkpoint" => journal.checkpoint_payload(pp.id),
        _ => return Err(axum::http::StatusCode::NOT_FOUND),
    };
    let raw = raw.map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    raw.ok_or(axum::http::StatusCode::NOT_FOUND)
}

/// `GET /payload/skill/:skill_id/:version` — skill_payload needs a
/// composite key.
#[derive(Debug, serde::Deserialize)]
pub struct SkillPayloadPath {
    pub skill_id: String,
    pub version: String,
}

pub async fn payload_skill(
    State(state): State<Arc<AppState>>,
    Path(pp): Path<SkillPayloadPath>,
) -> Result<String, axum::http::StatusCode> {
    let journal = state.journal();
    let raw = journal
        .skill_payload(&pp.skill_id, &pp.version)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    raw.ok_or(axum::http::StatusCode::NOT_FOUND)
}

// ────────────── unit tests for the tail routes ──────────────
//
// Phase 1 has no `axum::TestServer` here (we'd need `tower::ServiceExt`
// + `hyper` extra deps). The validation lives in `journal/tests.rs`
// via the synchronous `Journal` API. To keep `cargo test` failing
// never silently, we exercise the `TailQuery::resolved` clamp logic
// and the `to_json_value` trampoline as pure Rust unit tests.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_defaults_to_default_last() {
        assert_eq!(TailQuery { last: None }.resolved(), DEFAULT_LAST);
    }

    #[test]
    fn resolved_clamps_low_to_one() {
        assert_eq!(TailQuery { last: Some(0) }.resolved(), 1);
        assert_eq!(TailQuery { last: Some(-5) }.resolved(), 1);
    }

    #[test]
    fn resolved_clamps_high_to_max() {
        assert_eq!(TailQuery { last: Some(9999) }.resolved(), MAX_LAST);
    }

    /// Sanity: every TailRow impl should resolve to at least an empty
    /// list against a fresh Journal (no panic, no error). Catches
    /// typos in the trait wiring that would only show up at HTTP
    /// request time.
    #[test]
    fn tail_of_verdicts_against_empty_journal_yows_empty_vec() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let journal = crate::journal::Journal::open(tmp.path()).expect("open");
        let rows = VerdictRow::fetch(&journal, 10).expect("fetch");
        assert!(rows.is_empty());
    }

    /// RFC 27 §G/§B — freshly added tail kinds also resolve cleanly
    /// against an empty Journal so the HUD's `/tail/model_swaps` and
    /// `/tail/step_states` never panic before the operator has run
    /// any mission.
    #[test]
    fn tail_of_step_states_against_empty_journal_yows_empty_vec() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let journal = crate::journal::Journal::open(tmp.path()).expect("open");
        let rows = StepStateRow::fetch(&journal, 10).expect("fetch");
        assert!(rows.is_empty());
    }

    #[test]
    fn tail_of_model_swaps_against_empty_journal_yows_empty_vec() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let journal = crate::journal::Journal::open(tmp.path()).expect("open");
        let rows = ModelSwapRow::fetch(&journal, 10).expect("fetch");
        assert!(rows.is_empty());
    }
}
