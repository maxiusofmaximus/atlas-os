// Atlas OS — Affinity reader + ArcSwap cache (RFC 04 §8, Phase 2
// sub-fase 2.4, research/29 line 250).
//
// The feedback loop that closes Phase 2's data-driven routing: after
// every mission the orchestrator re-crunches the historical
// `model_invocations` window grouping by `(task_type, model_id)` and
// writes the resulting `AffinityRow { success_rate, p95_latency_ms,
// mean_cost_usd, n_samples }` to both the SQLite mirror
// (`model_affinity_cache`, M23) and an in-memory `ArcSwap<HashMap>`
// that the router consults on the hot path.
//
// `task_type` is **not** denormalised onto `model_invocations` (the M24
// bump is reserved for Phase 2.5+ if the JOIN ever proves a hot-path
// bottleneck). 2.4 derives `task_type` by joining
// `task_classifier_decisions.predicted_task_type ON mission_id` — the
// classifier's prediction is the affinity signal we want anyway
// (operator-corrected task types are an A/B signal, persisted to
// `task_classifier_decisions.classifier_kind = 'mf_ab'`).
//
// The query shape (single-pass GROUP BY over a bounded window) keeps
// the reader O(n) in samples even on slow Windowssqlite, and the
// `ArcSwap::store` swap is one atomic write — readers holding a `Guard`
// see the previous snapshot until they drop + reload.

use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};

use crate::orchestrator::classifier::TaskType;

/// One row of the per-`(task_type, model_id)` affinity table. Mirrors
/// the M23 `model_affinity_cache` SQLite shape 1:1 so the SQL reader
/// and the in-memory reader return identical structs — the router
/// always sees the same type regardless of provenance.
///
/// Field semantics (research/29 line 250):
/// * `success_rate` — fraction of `model_invocations` rows with
///   `was_correct = 1` over the rolling window. `was_correct` is set
///   by the Validation engine (RFC 14 §3) when the user accepts the
///   diff or the verdict autofocus marks it as accepted.
/// * `p95_latency_ms` — 95th-percentile of `latency_ms` over the
///   window. Computed client-side because SQLite has no built-in
///   `PERCENTILE_CONT` — we pull `latency_ms` values into a `Vec` and
///   sort + index.
/// * `mean_cost_usd` — arithmetic mean of `cost_usd` over the window.
/// * `n_samples` — number of `model_invocations` rows that landed in
///   the window. The router uses this as a confidence weight: a pair
///   with `n_samples < 3` is treated as "no data" and falls back to
///   the strategy-only score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AffinityRow {
    pub task_type: TaskType,
    pub model_id: String,
    pub success_rate: f64,
    pub p95_latency_ms: Option<i64>,
    pub mean_cost_usd: Option<f64>,
    pub n_samples: i64,
}

impl AffinityRow {
    /// Sentinel for `n_samples < MIN_SAMPLES`. The router checks this
    /// first to decide whether to use the affinity signal at all.
    pub const MIN_SAMPLES: i64 = 3;
}

/// Lock-free in-memory affinity index. Cloning yields a new handle to
/// the *same* underlying `ArcSwap` — every clone observes subsequent
/// `store` calls. The orchestrator owns one clone on the request loop;
/// the journal refresh task owns another; the HUD owns a clone via
/// `AppState`.
///
/// `load()` returns an `arc_swap::Guard` which derefs to
/// `&Arc<HashMap<...>>` — cheap, wait-free, no allocations. Hot-path
/// readers should prefer `Cache<ArcSwap<...>>` if measurements ever
/// show contention; for 2.4 the plain `load()` is fine (the registry
/// is consulted once per request, not in a tight loop).
#[derive(Debug, Clone, Default)]
pub struct AffinityIndex {
    inner: Arc<ArcSwap<HashMap<(TaskType, String), AffinityRow>>>,
}

impl AffinityIndex {
    /// Build an empty index. Used by `Registry::default()`; the
    /// refresh loop populates it via `store_all` once journal telemetry
    /// is available.
    pub fn empty() -> Self {
        Self {
            inner: Arc::new(ArcSwap::new(Arc::new(HashMap::new()))),
        }
    }

    /// Atomically swap the whole map. Readers holding a `Guard` keep
    /// observing the previous snapshot until they drop + reload.
    pub fn store_all(&self, map: HashMap<(TaskType, String), AffinityRow>) {
        self.inner.store(Arc::new(map));
    }

    /// Borrow the current snapshot. Cheap (a few ns); callers should
    /// hold the `Guard` for the minimum scope possible to let the
    /// underlying `Arc` keep its refcount while reading.
    pub fn load(&self) -> arc_swap::Guard<Arc<HashMap<(TaskType, String), AffinityRow>>> {
        self.inner.load()
    }

    /// Convenience: look up an `AffinityRow` for `(task, model_id)` in
    /// the current snapshot. Returns `None` when absent or below the
    /// `MIN_SAMPLES` confidence floor — the router treats both as
    /// "no data" and falls back to the strategy-only score.
    pub fn get(&self, task: TaskType, model_id: &str) -> Option<AffinityRow> {
        let g = self.load();
        let row = g.get(&(task, model_id.to_string()))?;
        if row.n_samples < AffinityRow::MIN_SAMPLES {
            return None;
        }
        Some(row.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(task: TaskType, model: &str, samples: i64) -> AffinityRow {
        AffinityRow {
            task_type: task,
            model_id: model.into(),
            success_rate: 0.8,
            p95_latency_ms: Some(500),
            mean_cost_usd: Some(0.01),
            n_samples: samples,
        }
    }

    #[test]
    fn empty_index_returns_none_on_lookup() {
        let idx = AffinityIndex::empty();
        assert!(idx.get(TaskType::Coding, "gpt-4o").is_none());
    }

    #[test]
    fn store_all_then_get_returns_row() {
        let idx = AffinityIndex::empty();
        let mut m = HashMap::new();
        m.insert(
            (TaskType::Coding, "gpt-4o".into()),
            row(TaskType::Coding, "gpt-4o", 10),
        );
        idx.store_all(m);
        let r = idx.get(TaskType::Coding, "gpt-4o").expect("row present");
        assert_eq!(r.model_id, "gpt-4o");
        assert_eq!(r.n_samples, 10);
    }

    #[test]
    fn store_all_replaces_previous_snapshot_atomically() {
        let idx = AffinityIndex::empty();
        let mut m1 = HashMap::new();
        m1.insert(
            (TaskType::Coding, "gpt-4o".into()),
            row(TaskType::Coding, "gpt-4o", 5),
        );
        idx.store_all(m1);
        let mut m2 = HashMap::new();
        m2.insert(
            (TaskType::Coding, "claude-opus-4".into()),
            row(TaskType::Coding, "claude-opus-4", 9),
        );
        idx.store_all(m2);
        // We swapped the whole map: gpt-4o is gone, claude-opus-4 is in.
        assert!(idx.get(TaskType::Coding, "gpt-4o").is_none());
        assert!(idx.get(TaskType::Coding, "claude-opus-4").is_some());
    }

    #[test]
    fn get_filters_out_rows_below_min_samples() {
        let idx = AffinityIndex::empty();
        let mut m = HashMap::new();
        m.insert(
            (TaskType::Coding, "weak".into()),
            row(TaskType::Coding, "weak", AffinityRow::MIN_SAMPLES - 1),
        );
        m.insert(
            (TaskType::Coding, "strong".into()),
            row(TaskType::Coding, "strong", AffinityRow::MIN_SAMPLES),
        );
        idx.store_all(m);
        assert!(idx.get(TaskType::Coding, "weak").is_none(), "below MIN");
        assert!(idx.get(TaskType::Coding, "strong").is_some(), "at MIN");
    }

    #[test]
    fn cloned_index_shares_underlying_snapshot() {
        let idx = AffinityIndex::empty();
        let idx2 = idx.clone();
        let mut m = HashMap::new();
        m.insert(
            (TaskType::Plan, "opus".into()),
            row(TaskType::Plan, "opus", 4),
        );
        idx.store_all(m);
        // The clone observes the swap.
        assert!(idx2.get(TaskType::Plan, "opus").is_some());
    }

    #[test]
    fn affinity_row_roundtrips_through_serde() {
        let r = row(TaskType::Chat, "gemini-flash", 7);
        let json = serde_json::to_string(&r).unwrap();
        let back: AffinityRow = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }
}
