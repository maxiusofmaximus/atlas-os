// Atlas OS — Deterministic `Diff` application (Fase 35).
//
// The Coding Engine emits structured `Diff`s and the Validation Engine checks
// them, but nothing WRITES them to the workspace. This is the pure kernel that
// closes that loop: given the workspace lines (the Coding Engine explicitly does
// not touch the filesystem — the caller supplies the contents) it applies each
// `FileEdit`'s hunks in RFC 13 §8 order and returns the new contents.
//
// `old_start..old_end` is a HALF-OPEN 0-based range. `(0,0)` inserts at the top;
// an empty `new_lines` deletes the range. Line indices are clamped and hunks are
// applied by descending `old_start` so a multi-hunk edit never shifts a later
// hunk's coordinates. Nothing panics: an out-of-range hunk yields an `Err` so
// the caller can refuse to write.

use std::collections::BTreeMap;

use crate::coding::types::{Diff, FileEdit, Hunk};

/// One file's contents after applying a `Diff`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppliedFile {
    pub path: String,
    pub lines: Vec<String>,
    pub created: bool,
    pub deleted: bool,
}

/// Why a `Diff` could not be applied. Structured so the caller can report which
/// file/hunk failed instead of a stringly panic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyError {
    /// `old_end < old_start` (a malformed hunk range).
    InvalidRange {
        path: String,
        old_start: u32,
        old_end: u32,
    },
    /// The hunk extends past the end of the file it edits.
    OutOfRange {
        path: String,
        old_start: u32,
        old_end: u32,
        len: usize,
    },
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRange {
                path,
                old_start,
                old_end,
            } => write!(f, "`{path}`: hunk range {old_start}..{old_end} is inverted"),
            Self::OutOfRange {
                path,
                old_start,
                old_end,
                len,
            } => write!(
                f,
                "`{path}`: hunk {old_start}..{old_end} is out of range (file has {len} lines)"
            ),
        }
    }
}

impl std::error::Error for ApplyError {}

/// Apply one `FileEdit` to `existing` (the file's current lines). `is_new_file`
/// forces `existing` to be ignored (lifecycle: create). `is_delete` returns an
/// empty result. Always returns one `AppliedFile`.
pub fn apply_file_edit(existing: &[String], edit: &FileEdit) -> Result<AppliedFile, ApplyError> {
    if edit.is_delete {
        return Ok(AppliedFile {
            path: edit.path.clone(),
            lines: Vec::new(),
            created: false,
            deleted: true,
        });
    }

    let mut lines: Vec<String> = if edit.is_new_file {
        Vec::new()
    } else {
        existing.to_vec()
    };

    // Sort by `old_start` descending so applying a later hunk never shifts the
    // coordinates of an earlier one.
    let mut hunks: Vec<&Hunk> = edit.hunks.iter().collect();
    hunks.sort_by_key(|h| std::cmp::Reverse(h.old_start));

    for hunk in hunks {
        if hunk.old_end < hunk.old_start {
            return Err(ApplyError::InvalidRange {
                path: edit.path.clone(),
                old_start: hunk.old_start,
                old_end: hunk.old_end,
            });
        }
        let start = hunk.old_start as usize;
        let end = hunk.old_end as usize;
        if end > lines.len() {
            return Err(ApplyError::OutOfRange {
                path: edit.path.clone(),
                old_start: hunk.old_start,
                old_end: hunk.old_end,
                len: lines.len(),
            });
        }
        lines.splice(start..end, hunk.new_lines.iter().cloned());
    }

    Ok(AppliedFile {
        path: edit.path.clone(),
        lines,
        created: edit.is_new_file,
        deleted: false,
    })
}

/// Apply every `FileEdit` in `diff` to the workspace, keyed by path. Callers
/// pass the CURRENT contents (`path -> lines`); files the diff creates may be
/// absent from `workspace`. Returns the resulting contents per path, in the
/// order the diff lists them.
pub fn apply_diff(
    diff: &Diff,
    workspace: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<AppliedFile>, ApplyError> {
    let mut out = Vec::with_capacity(diff.files.len());
    for edit in &diff.files {
        let existing: &[String] = workspace
            .get(&edit.path)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        out.push(apply_file_edit(existing, edit)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn edit(path: &str, is_new: bool, is_delete: bool, hunks: Vec<Hunk>) -> FileEdit {
        FileEdit {
            path: path.into(),
            hunks,
            is_new_file: is_new,
            is_delete,
        }
    }

    fn hunk(old_start: u32, old_end: u32, new_lines: &[&str]) -> Hunk {
        Hunk {
            old_start,
            old_end,
            new_lines: new_lines.iter().map(|s| s.to_string()).collect(),
            rationale: "r".into(),
        }
    }

    fn diff_of(files: Vec<FileEdit>) -> Diff {
        Diff {
            diff_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            mission_id: Uuid::new_v4(),
            step_id: "S1".into(),
            agent_id: Uuid::new_v4(),
            generated_at: "2026-10-03T00:00:00Z".into(),
            files,
            narrative: String::new(),
            research_refs: vec![],
            risk_decision: None,
            model_id: "m".into(),
            elapsed_ms: 0,
        }
    }

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn inserts_at_the_top_of_a_file() {
        let e = edit("a.rs", false, false, vec![hunk(0, 0, &["// header"])]);
        let out = apply_file_edit(&lines(&["fn main() {}"]), &e).unwrap();
        assert_eq!(out.lines, lines(&["// header", "fn main() {}"]));
    }

    #[test]
    fn replaces_a_range() {
        let e = edit("a.rs", false, false, vec![hunk(1, 2, &["B2", "B3"])]);
        let out = apply_file_edit(&lines(&["A", "B", "C"]), &e).unwrap();
        assert_eq!(out.lines, lines(&["A", "B2", "B3", "C"]));
    }

    #[test]
    fn deletes_a_range() {
        let e = edit("a.rs", false, false, vec![hunk(1, 3, &[])]);
        let out = apply_file_edit(&lines(&["A", "B", "C", "D"]), &e).unwrap();
        assert_eq!(out.lines, lines(&["A", "D"]));
    }

    #[test]
    fn multi_hunk_edits_keep_their_coordinates() {
        // Replace line 0 and line 3; descending application must not shift.
        let e = edit(
            "a.rs",
            false,
            false,
            vec![hunk(0, 1, &["A0"]), hunk(3, 4, &["D3"])],
        );
        let out = apply_file_edit(&lines(&["A", "B", "C", "D"]), &e).unwrap();
        assert_eq!(out.lines, lines(&["A0", "B", "C", "D3"]));
    }

    #[test]
    fn new_file_starts_empty() {
        let e = edit("new.rs", true, false, vec![hunk(0, 0, &["pub fn x() {}"])]);
        let out = apply_file_edit(&lines(&["ignored"]), &e).unwrap();
        assert!(out.created);
        assert_eq!(out.lines, lines(&["pub fn x() {}"]));
    }

    #[test]
    fn delete_file_empties_contents() {
        let e = edit("gone.rs", false, true, vec![]);
        let out = apply_file_edit(&lines(&["A", "B"]), &e).unwrap();
        assert!(out.deleted);
        assert!(out.lines.is_empty());
    }

    #[test]
    fn out_of_range_hunk_is_rejected() {
        let e = edit("a.rs", false, false, vec![hunk(5, 6, &["x"])]);
        let err = apply_file_edit(&lines(&["A"]), &e).unwrap_err();
        assert!(matches!(err, ApplyError::OutOfRange { .. }), "got {err:?}");
    }

    #[test]
    fn inverted_range_is_rejected() {
        let e = edit("a.rs", false, false, vec![hunk(3, 1, &["x"])]);
        let err = apply_file_edit(&lines(&["A", "B", "C", "D"]), &e).unwrap_err();
        assert!(
            matches!(err, ApplyError::InvalidRange { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn apply_diff_creates_and_edits_files() {
        let mut ws = BTreeMap::new();
        ws.insert("src/lib.rs".to_string(), lines(&["mod a;"]));
        let d = diff_of(vec![
            edit("src/lib.rs", false, false, vec![hunk(1, 1, &["mod b;"])]),
            edit(
                "src/new.rs",
                true,
                false,
                vec![hunk(0, 0, &["pub fn n() {}"])],
            ),
        ]);
        let applied = apply_diff(&d, &ws).unwrap();
        assert_eq!(applied.len(), 2);
        assert_eq!(applied[0].lines, lines(&["mod a;", "mod b;"]));
        assert!(applied[1].created);
        assert_eq!(applied[1].lines, lines(&["pub fn n() {}"]));
    }
}
