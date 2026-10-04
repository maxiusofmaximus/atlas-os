// Atlas OS — Diff merge kernel (RFC 05 §4 "Protocolo de merge", Fase 37).
//
// The Merger subagent fuses the diffs of the other agents into ONE diff that
// the Validation Engine then runs over (RFC 05 §4 steps 3–4). This is the pure
// merge: it takes N per-agent `Diff`s and composes their `FileEdit`s in a
// DETERMINISTIC order (sorted by `agent_id`, then the diff's own file order),
// so the same inputs always yield the same output.
//
// Conflict rule (RFC 05 §4 step 5): if two agents edit the SAME half-open line
// range of the SAME file, the merge does not silently pick one — it returns the
// conflicting paths so the Planner can replan the sub-objective. Editing
// different ranges of a file is allowed (they are concatenated in order).

use std::collections::BTreeMap;

use uuid::Uuid;

use crate::coding::types::{Diff, FileEdit};

/// Result of merging: the composed diff plus any paths where two agents claimed
/// the same line range (empty means a clean merge).
#[derive(Clone, Debug)]
pub struct MergeOutcome {
    pub diff: Diff,
    pub conflicts: Vec<MergeConflict>,
}

/// One same-range collision between two agents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeConflict {
    pub path: String,
    pub first_agent: Uuid,
    pub second_agent: Uuid,
    pub old_start: u32,
    pub old_end: u32,
}

/// Merge `diffs` into one. Agents are processed in ascending `agent_id` order
/// (deterministic), and each agent's files are kept in the order the agent
/// listed them. Files are grouped by path; a path touched by more than one agent
/// gets every hunk, and overlapping half-open ranges are reported as conflicts
/// (the caller decides whether to replan).
pub fn merge_diffs(diffs: &[Diff]) -> MergeOutcome {
    let mut ordered: Vec<&Diff> = diffs.iter().collect();
    ordered.sort_by_key(|d| d.agent_id);

    // Union of files across diffs, preserving first-seen agent order.
    let mut file_order: Vec<String> = Vec::new();
    let mut by_path: BTreeMap<String, Vec<(&Diff, &FileEdit)>> = BTreeMap::new();
    for d in &ordered {
        for f in &d.files {
            if !by_path.contains_key(&f.path) {
                file_order.push(f.path.clone());
            }
            by_path.entry(f.path.clone()).or_default().push((d, f));
        }
    }

    let mut conflicts: Vec<MergeConflict> = Vec::new();
    let mut files: Vec<FileEdit> = Vec::with_capacity(file_order.len());

    for path in &file_order {
        let entries = &by_path[path];
        // Agents that touched this path, in the deterministic order above.
        let mut merged = FileEdit {
            path: path.clone(),
            hunks: Vec::new(),
            is_new_file: entries.iter().all(|(_, f)| f.is_new_file),
            is_delete: entries.iter().any(|(_, f)| f.is_delete),
        };
        // Claimed ranges, to detect same-range double-writes.
        let mut claimed: Vec<(u32, u32, Uuid)> = Vec::new();
        for (d, f) in entries {
            for h in &f.hunks {
                if let Some((_, _, other)) = claimed
                    .iter()
                    .find(|(s, e, _)| *s == h.old_start && *e == h.old_end)
                {
                    conflicts.push(MergeConflict {
                        path: path.clone(),
                        first_agent: *other,
                        second_agent: d.agent_id,
                        old_start: h.old_start,
                        old_end: h.old_end,
                    });
                } else {
                    claimed.push((h.old_start, h.old_end, d.agent_id));
                }
                merged.hunks.push(h.clone());
            }
            if f.is_new_file {
                merged.is_new_file = true;
            }
        }
        files.push(merged);
    }

    let base = ordered.first();
    let diff = Diff {
        diff_id: Uuid::new_v4(),
        plan_id: base.map(|d| d.plan_id).unwrap_or_else(Uuid::nil),
        mission_id: base.map(|d| d.mission_id).unwrap_or_else(Uuid::nil),
        step_id: "merge".into(),
        agent_id: base.map(|d| d.agent_id).unwrap_or_else(Uuid::nil),
        generated_at: chrono::Utc::now().to_rfc3339(),
        files,
        narrative: format!(
            "merged {} agent diff(s) in deterministic agent-id order",
            diffs.len()
        ),
        research_refs: base.map(|d| d.research_refs.clone()).unwrap_or_default(),
        risk_decision: None,
        model_id: "swarm-merger".into(),
        elapsed_ms: 0,
    };

    MergeOutcome { diff, conflicts }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::types::Hunk;

    fn hunk(s: u32, e: u32, lines: &[&str]) -> Hunk {
        Hunk {
            old_start: s,
            old_end: e,
            new_lines: lines.iter().map(|l| l.to_string()).collect(),
            rationale: "r".into(),
        }
    }

    fn diff(agent: Uuid, files: Vec<FileEdit>) -> Diff {
        Diff {
            diff_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            mission_id: Uuid::new_v4(),
            step_id: "s".into(),
            agent_id: agent,
            generated_at: "2026-10-03T00:00:00Z".into(),
            files,
            narrative: String::new(),
            research_refs: vec![],
            risk_decision: None,
            model_id: "m".into(),
            elapsed_ms: 0,
        }
    }

    fn edit(path: &str, hunks: Vec<Hunk>) -> FileEdit {
        FileEdit {
            path: path.into(),
            hunks,
            is_new_file: false,
            is_delete: false,
        }
    }

    #[test]
    fn merges_disjoint_files_from_two_agents() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let d1 = diff(a, vec![edit("a.rs", vec![hunk(0, 0, &["A"])])]);
        let d2 = diff(b, vec![edit("b.rs", vec![hunk(0, 0, &["B"])])]);
        let out = merge_diffs(&[d2, d1]);
        assert!(out.conflicts.is_empty());
        assert_eq!(out.diff.files.len(), 2);
        // Deterministic order: agent a (1) before agent b (2) regardless of input.
        assert_eq!(out.diff.files[0].path, "a.rs");
        assert_eq!(out.diff.files[1].path, "b.rs");
    }

    #[test]
    fn two_agents_edit_different_ranges_of_same_file() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let d1 = diff(a, vec![edit("x.rs", vec![hunk(0, 1, &["head"])])]);
        let d2 = diff(b, vec![edit("x.rs", vec![hunk(5, 6, &["tail"])])]);
        let out = merge_diffs(&[d1, d2]);
        assert!(
            out.conflicts.is_empty(),
            "different ranges are not a conflict"
        );
        assert_eq!(out.diff.files.len(), 1);
        assert_eq!(out.diff.files[0].hunks.len(), 2);
    }

    #[test]
    fn same_range_from_two_agents_is_a_conflict() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let d1 = diff(a, vec![edit("x.rs", vec![hunk(2, 4, &["one"])])]);
        let d2 = diff(b, vec![edit("x.rs", vec![hunk(2, 4, &["two"])])]);
        let out = merge_diffs(&[d1, d2]);
        assert_eq!(out.conflicts.len(), 1);
        let c = &out.conflicts[0];
        assert_eq!(c.path, "x.rs");
        assert_eq!((c.first_agent, c.second_agent), (a, b));
    }

    #[test]
    fn merge_is_deterministic_across_input_order() {
        let a = Uuid::from_u128(7);
        let b = Uuid::from_u128(3);
        let d1 = diff(a, vec![edit("m.rs", vec![hunk(0, 0, &["a"])])]);
        let d2 = diff(b, vec![edit("m.rs", vec![hunk(1, 1, &["b"])])]);
        let x = merge_diffs(&[d1.clone(), d2.clone()]);
        let y = merge_diffs(&[d2, d1]);
        assert_eq!(x.diff.files[0].path, y.diff.files[0].path);
        let nx: Vec<_> = x.diff.files[0]
            .hunks
            .iter()
            .map(|h| h.new_lines.clone())
            .collect();
        let ny: Vec<_> = y.diff.files[0]
            .hunks
            .iter()
            .map(|h| h.new_lines.clone())
            .collect();
        assert_eq!(nx, ny, "same inputs must yield the same merged order");
    }
}
