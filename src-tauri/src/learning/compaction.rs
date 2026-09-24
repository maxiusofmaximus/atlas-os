// Atlas OS — System One compaction (RFC 32 Phase 5 sub-fase 5.3, RFC 30 §2
// fast-jev-compaction pattern).
//
// When a mission's `journal_events` tail exceeds `COMPACTION_THRESHOLD`
// (default 100), the history is compacted by the small/cheap model
// (`weak_model` tri-model) — never by truncation, never by the large
// model. Phase 1 ships a deterministic stub (rolling window: per-kind
// counts + latest headlines) that fixes the `CompactionSummary` shape;
// the LLM-driven `weak_model` summariser is a documented follow-up
// that plugs into `summarize` without changing callers.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::journal::JournalEntry;

/// Tail length above which a mission's history is compacted
/// (RFC 32 SECTOR B 5.3).
pub const COMPACTION_THRESHOLD: usize = 100;

/// Rolling window the stub retains: the compacted view keeps at most
/// this many of the newest entries, so `entries_after < entries_before`
/// whenever `needs_compaction` fired.
pub const COMPACTION_KEEP_LAST: usize = 20;

/// Headlines surfaced in the stub summary (newest entries first).
pub const COMPACTION_HEADLINES: usize = 5;

/// Provenance of the stub summariser. The `weak_model` LLM wiring
/// (RFC 32 §C follow-up) replaces this id while keeping the shape.
pub const COMPACTION_MODEL_ID: &str = "heuristic-rolling-v0";

/// Deterministic compaction result for one mission. Persisted 1:1 into
/// `compaction_events` by `Journal::save_compaction_event` (M30 layout).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionSummary {
    pub mission_id: String,
    pub entries_before: usize,
    pub entries_after: usize,
    pub summary: String,
    pub model_id: String,
}

/// True when `count` exceeds the default threshold.
pub fn needs_compaction(count: usize) -> bool {
    needs_compaction_with_threshold(count, COMPACTION_THRESHOLD)
}

/// Same as `needs_compaction` with an explicit threshold (the CLI
/// `--threshold` flag threads through here).
pub fn needs_compaction_with_threshold(count: usize, threshold: usize) -> bool {
    count > threshold
}

/// Stub summariser (RFC 32 §A.2): per-kind counts over the whole tail
/// plus headlines (kind + payload head) of the newest entries.
/// Pure and deterministic: kinds sorted, headlines newest-first.
pub fn summarize(mission_id: &str, entries: &[JournalEntry]) -> CompactionSummary {
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for e in entries {
        *kinds.entry(e.kind.as_str()).or_default() += 1;
    }
    let mut counts: Vec<String> = kinds.iter().map(|(k, n)| format!("{k}:{n}")).collect();
    counts.sort();
    let headlines: Vec<String> = entries
        .iter()
        .rev()
        .take(COMPACTION_HEADLINES)
        .map(|e| format!("{}: {}", e.kind, payload_head(&e.payload)))
        .collect();
    let entries_before = entries.len();
    let entries_after = entries_before.min(COMPACTION_KEEP_LAST);
    let summary = format!(
        "mission {mission_id}: {entries_before} entries across {} kinds ({}) | latest: {}",
        kinds.len(),
        counts.join(", "),
        if headlines.is_empty() {
            "(empty)".to_string()
        } else {
            headlines.join(" | ")
        },
    );
    CompactionSummary {
        mission_id: mission_id.to_string(),
        entries_before,
        entries_after,
        summary,
        model_id: COMPACTION_MODEL_ID.to_string(),
    }
}

fn payload_head(payload: &serde_json::Value) -> String {
    let raw = match payload {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let flat: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= 80 {
        flat
    } else {
        let mut out: String = flat.chars().take(80).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: i64, kind: &str, payload: &str) -> JournalEntry {
        JournalEntry {
            id,
            ts: "2026-07-04T12:00:00Z".into(),
            kind: kind.into(),
            payload: serde_json::Value::String(payload.into()),
        }
    }

    #[test]
    fn threshold_fires_only_above_default() {
        assert!(!needs_compaction(0));
        assert!(!needs_compaction(COMPACTION_THRESHOLD));
        assert!(needs_compaction(COMPACTION_THRESHOLD + 1));
        assert!(!needs_compaction_with_threshold(5, 10));
        assert!(needs_compaction_with_threshold(11, 10));
    }

    #[test]
    fn summary_counts_kinds_and_headlines_newest_first() {
        let entries = vec![
            entry(1, "task_received", "first prompt"),
            entry(2, "agent_diff", "diff one"),
            entry(3, "agent_diff", "diff two"),
        ];
        let s = summarize("m-1", &entries);
        assert_eq!(s.mission_id, "m-1");
        assert_eq!(s.entries_before, 3);
        assert_eq!(s.entries_after, 3);
        assert_eq!(s.model_id, COMPACTION_MODEL_ID);
        assert!(s.summary.contains("3 entries across 2 kinds"));
        assert!(s.summary.contains("agent_diff:2"));
        assert!(s.summary.contains("task_received:1"));
        let pos_new = s.summary.find("diff two").expect("newest headline");
        let pos_old = s.summary.find("diff one").expect("older headline");
        assert!(pos_new < pos_old, "headlines newest-first");
    }

    #[test]
    fn compacted_view_is_strictly_smaller_than_threshold_tail() {
        let entries: Vec<JournalEntry> = (0..(COMPACTION_THRESHOLD + 1) as i64)
            .map(|i| entry(i, "agent_diff", "x"))
            .collect();
        assert!(needs_compaction(entries.len()));
        let s = summarize("m-big", &entries);
        assert!(s.entries_after < s.entries_before);
        assert_eq!(s.entries_after, COMPACTION_KEEP_LAST);
    }

    #[test]
    fn summary_is_deterministic_for_same_input() {
        let entries = vec![
            entry(1, "b_kind", "payload b"),
            entry(2, "a_kind", "payload a"),
        ];
        assert_eq!(summarize("m-d", &entries), summarize("m-d", &entries));
    }

    #[test]
    fn empty_tail_summarises_without_panic() {
        let s = summarize("m-empty", &[]);
        assert_eq!(s.entries_before, 0);
        assert_eq!(s.entries_after, 0);
        assert!(!needs_compaction(0));
        assert!(s.summary.contains("(empty)"));
    }
}
