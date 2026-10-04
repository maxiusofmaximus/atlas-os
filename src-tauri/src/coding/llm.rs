// Atlas OS — LLM-driven Coding Engine codec (RFC 13 Phase 2; Fase 26 v26.0).
//
// Phase 1's `runner` is heuristic (no LLM call). This module is the contract
// for the Phase 2 path: the orchestrator asks a model for a STRUCTURED `Diff`,
// and this codec turns that reply into a canonical `coding::types::Diff` which
// then flows through the SAME pure Validation/Repair engines the heuristic path
// uses (`validation::runner::run` / `repair::runner::run`). Parsing is pure and
// offline — the model reply is injected — so the whole loop is unit-testable
// without a network (the `model_id` field of `Diff` is what RFC 13 §2 calls the
// "Phase 2" provenance).

use serde::Deserialize;
use uuid::Uuid;

use crate::coding::types::{Diff, FileEdit, Hunk};

/// System instruction that pins the model's reply to the `Diff` JSON contract.
/// Sent as the `system` message; the step statement is the `user` message.
pub const DIFF_CONTRACT_PROMPT: &str = r#"You are the Atlas Coding Engine. Reply with ONE JSON object and nothing else — no prose, no markdown fences.

Schema:
{
  "narrative": "<what it did / what it does now / why>",
  "files": [
    {
      "path": "relative/path.ext",
      "is_new_file": false,
      "is_delete": false,
      "hunks": [
        { "old_start": 10, "old_end": 12, "new_lines": ["line a", "line b"], "rationale": "why" }
      ]
    }
  ]
}

Rules:
- Emit STRUCTURED hunks, never a unified-diff blob.
- `old_start`..`old_end` is the half-open range of lines being replaced in the CURRENT file; `0,0` inserts at the top.
- `new_lines` are the replacement lines WITHOUT trailing newlines; an empty array means deletion.
- Paths are relative to the repo root and use `/`.
- For a brand-new file set `"is_new_file": true` and use `0,0` for its hunks.
"#;

/// Provenance the codec stamps onto the `Diff` it builds (the model only
/// supplies `narrative` + `files`; identity comes from the kernel).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffMeta {
    pub mission_id: Uuid,
    pub plan_id: Uuid,
    pub step_id: String,
    pub agent_id: Uuid,
    pub model_id: String,
}

/// Why a model reply could not be turned into a `Diff`. Structured (not a
/// stringly error) so the caller can branch: `BadJson` → re-ask, `NoFiles` →
/// reject, etc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffParseError {
    /// The reply was empty/whitespace.
    Empty,
    /// No `{ ... }` object could be located in the reply.
    NoJson,
    /// An object was found but did not deserialize into the contract.
    BadJson(String),
    /// `files` was present but empty (a `Diff` with no edits is meaningless).
    NoFiles,
    /// A file entry had a blank `path`.
    FileWithoutPath,
    /// A file entry had no hunks and was not marked `is_delete`.
    FileWithoutHunks(String),
}

impl std::fmt::Display for DiffParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "model reply was empty"),
            Self::NoJson => write!(f, "no JSON object found in model reply"),
            Self::BadJson(e) => write!(f, "model reply is not valid diff JSON: {e}"),
            Self::NoFiles => write!(f, "diff has no files"),
            Self::FileWithoutPath => write!(f, "diff file entry has an empty path"),
            Self::FileWithoutHunks(p) => write!(f, "diff file `{p}` has no hunks"),
        }
    }
}

impl std::error::Error for DiffParseError {}

#[derive(Deserialize)]
struct RawDiff {
    #[serde(default)]
    narrative: String,
    files: Vec<RawFile>,
}

#[derive(Deserialize)]
struct RawFile {
    #[serde(default)]
    path: String,
    #[serde(default)]
    is_new_file: bool,
    #[serde(default)]
    is_delete: bool,
    #[serde(default)]
    hunks: Vec<RawHunk>,
}

#[derive(Deserialize)]
struct RawHunk {
    #[serde(default)]
    old_start: u32,
    #[serde(default)]
    old_end: u32,
    #[serde(default)]
    new_lines: Vec<String>,
    #[serde(default)]
    rationale: String,
}

/// Turn a model reply into a canonical `Diff`. Tolerates prose and ```json
/// fences around the object (models add them constantly); rejects anything that
/// cannot yield a usable edit.
pub fn parse_diff_json(content: &str, meta: DiffMeta) -> Result<Diff, DiffParseError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(DiffParseError::Empty);
    }
    let json = extract_json_object(trimmed).ok_or(DiffParseError::NoJson)?;
    let raw: RawDiff =
        serde_json::from_str(json).map_err(|e| DiffParseError::BadJson(e.to_string()))?;
    if raw.files.is_empty() {
        return Err(DiffParseError::NoFiles);
    }

    let mut files = Vec::with_capacity(raw.files.len());
    for f in raw.files {
        let path = f.path.trim().replace('\\', "/");
        if path.is_empty() {
            return Err(DiffParseError::FileWithoutPath);
        }
        if f.hunks.is_empty() && !f.is_delete {
            return Err(DiffParseError::FileWithoutHunks(path));
        }
        let hunks = f
            .hunks
            .into_iter()
            .map(|h| Hunk {
                old_start: h.old_start,
                // Clamp so `old_end < old_start` (a model slip) cannot produce a
                // diff that violates `FileEdit::hunks_are_disjoint_and_sorted`.
                old_end: h.old_end.max(h.old_start),
                new_lines: h.new_lines,
                rationale: h.rationale,
            })
            .collect();
        files.push(FileEdit {
            path,
            hunks,
            is_new_file: f.is_new_file,
            is_delete: f.is_delete,
        });
    }

    Ok(Diff {
        diff_id: Uuid::new_v4(),
        plan_id: meta.plan_id,
        mission_id: meta.mission_id,
        step_id: meta.step_id,
        agent_id: meta.agent_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        files,
        narrative: raw.narrative,
        research_refs: Vec::new(),
        risk_decision: None,
        model_id: meta.model_id,
        elapsed_ms: 0,
    })
}

/// Locate the outermost `{ ... }` in `s`, tolerating surrounding prose/fences.
fn extract_json_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(&s[start..=end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> DiffMeta {
        DiffMeta {
            mission_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            step_id: "S1".into(),
            agent_id: Uuid::new_v4(),
            model_id: "gpt-test".into(),
        }
    }

    #[test]
    fn parses_a_clean_contract_object() {
        let reply = r#"{"narrative":"add helper","files":[{"path":"src/lib.rs","hunks":[{"old_start":0,"old_end":0,"new_lines":["pub fn hi() {}"],"rationale":"new fn"}]}]}"#;
        let d = parse_diff_json(reply, meta()).expect("valid");
        assert_eq!(d.files.len(), 1);
        assert_eq!(d.files[0].path, "src/lib.rs");
        assert_eq!(d.files[0].hunks[0].new_lines, vec!["pub fn hi() {}"]);
        assert_eq!(d.narrative, "add helper");
        assert_eq!(
            d.model_id, "gpt-test",
            "provenance is the kernel's, not the model's"
        );
    }

    #[test]
    fn tolerates_markdown_fences_and_surrounding_prose() {
        let reply = "Sure, here is the diff:\n```json\n{\"files\":[{\"path\":\"a.rs\",\"hunks\":[{\"old_start\":1,\"old_end\":1,\"new_lines\":[\"x\"]}]}]}\n```\nHope that helps!";
        let d = parse_diff_json(reply, meta()).expect("fenced object parses");
        assert_eq!(d.files[0].path, "a.rs");
    }

    #[test]
    fn normalizes_backslash_paths() {
        let reply = r#"{"files":[{"path":"src\\nested\\mod.rs","hunks":[{"old_start":0,"old_end":0,"new_lines":["y"]}]}]}"#;
        let d = parse_diff_json(reply, meta()).expect("valid");
        assert_eq!(d.files[0].path, "src/nested/mod.rs");
    }

    #[test]
    fn clamps_inverted_hunk_range() {
        let reply = r#"{"files":[{"path":"a.rs","hunks":[{"old_start":5,"old_end":2,"new_lines":["z"]}]}]}"#;
        let d = parse_diff_json(reply, meta()).expect("valid");
        assert_eq!(
            d.files[0].hunks[0].old_end, 5,
            "old_end clamped up to old_start"
        );
        assert!(d.files[0].hunks_are_disjoint_and_sorted());
    }

    #[test]
    fn delete_file_without_hunks_is_allowed() {
        let reply = r#"{"files":[{"path":"gone.rs","is_delete":true}]}"#;
        let d = parse_diff_json(reply, meta()).expect("delete is legal without hunks");
        assert!(d.files[0].is_delete);
        assert!(d.files[0].hunks.is_empty());
    }

    #[test]
    fn empty_reply_is_rejected() {
        assert_eq!(
            parse_diff_json("   \n", meta()).unwrap_err(),
            DiffParseError::Empty
        );
    }

    #[test]
    fn reply_without_an_object_is_rejected() {
        assert_eq!(
            parse_diff_json("no json here", meta()).unwrap_err(),
            DiffParseError::NoJson
        );
    }

    #[test]
    fn malformed_object_is_bad_json() {
        let err = parse_diff_json("{ \"files\": [ }", meta()).unwrap_err();
        assert!(matches!(err, DiffParseError::BadJson(_)), "got {err:?}");
    }

    #[test]
    fn empty_files_is_rejected() {
        assert_eq!(
            parse_diff_json(r#"{"files":[]}"#, meta()).unwrap_err(),
            DiffParseError::NoFiles
        );
    }

    #[test]
    fn file_without_hunks_is_rejected() {
        let err = parse_diff_json(r#"{"files":[{"path":"a.rs"}]}"#, meta()).unwrap_err();
        assert_eq!(err, DiffParseError::FileWithoutHunks("a.rs".into()));
    }

    #[test]
    fn file_without_path_is_rejected() {
        let err = parse_diff_json(
            r#"{"files":[{"hunks":[{"old_start":0,"old_end":0,"new_lines":["x"]}]}]}"#,
            meta(),
        )
        .unwrap_err();
        assert_eq!(err, DiffParseError::FileWithoutPath);
    }
}
