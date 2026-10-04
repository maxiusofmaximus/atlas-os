// Atlas OS — Code tools (RFC 63 §4: code.apply_diff).
//
// The one tool that needs the coding engine: apply a unified diff to the
// workspace. The agent emits text patches the way a human would (`@@` hunks);
// this module parses them and applies each file atomically — a hunk whose
// context does not match aborts that file's write rather than corrupting it.
//
// Parsing is deliberately strict: it accepts the standard `--- a/x`, `+++ b/x`,
// `@@ -l,c +l,c @@` shape plus `new file mode` / `deleted file mode` markers, so
// `git diff` output pastes in unchanged. No shell, no `git` binary — pure Rust,
// so it works inside the Linux sandbox and is unit-testable offline.

use serde::Deserialize;

use crate::security::sandbox::SensitiveAction;

use super::fs::resolve;
use super::{Tool, ToolContext, ToolResult};

/// One file's change, parsed from a unified diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePatch {
    pub path: String,
    pub is_new: bool,
    pub is_delete: bool,
    pub hunks: Vec<Hunk>,
}

/// One `@@ ... @@` hunk: the context lines to find plus the replacement lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    /// Lines that must appear (context ` ` and removed `-`), in order.
    pub old_lines: Vec<String>,
    /// The replacement lines (context ` ` and added `+`), in order.
    pub new_lines: Vec<String>,
    /// Parsed old start line (1-based); used only for diagnostics.
    pub old_start: usize,
}

impl FilePatch {
    /// Apply this patch to `current` (the file's text), returning the new text.
    /// Errors if any hunk's old context is not found exactly once.
    pub fn apply_to(&self, current: &str, path: &str) -> Result<String, String> {
        if self.is_delete {
            return Ok(String::new());
        }
        if self.is_new {
            // New files ignore `current`; concatenate all added lines.
            let mut out = String::new();
            for h in &self.hunks {
                for l in &h.new_lines {
                    out.push_str(l);
                    out.push('\n');
                }
            }
            return Ok(out);
        }
        let mut lines: Vec<String> = current.split('\n').map(|s| s.to_string()).collect();
        // A trailing newline yields an empty last element; keep it to preserve EOL.
        for h in &self.hunks {
            let idx = find_sublist(&lines, &h.old_lines).ok_or_else(|| {
                format!("hunk context not found in {path} near line {}", h.old_start)
            })?;
            lines.splice(idx..idx + h.old_lines.len(), h.new_lines.iter().cloned());
        }
        Ok(lines.join("\n"))
    }
}

/// Find the first index where `needle` occurs contiguously in `hay`.
fn find_sublist(hay: &[String], needle: &[String]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle)
}

/// Parse a unified diff into per-file patches. Tolerates `diff --git` headers,
/// `index` lines and `\ No newline` markers (skipped).
pub fn parse_unified(patch: &str) -> Result<Vec<FilePatch>, String> {
    let mut files: Vec<FilePatch> = Vec::new();
    let mut cur: Option<FilePatch> = None;
    let mut cur_hunk: Option<Hunk> = None;

    let flush_hunk = |cur: &mut Option<FilePatch>, hunk: &mut Option<Hunk>| {
        if let (Some(f), Some(h)) = (cur.as_mut(), hunk.take()) {
            if !h.old_lines.is_empty() || !h.new_lines.is_empty() {
                f.hunks.push(h);
            }
        }
    };
    let flush_file =
        |files: &mut Vec<FilePatch>, cur: &mut Option<FilePatch>, hunk: &mut Option<Hunk>| {
            flush_hunk(cur, hunk);
            if let Some(f) = cur.take() {
                if !f.path.is_empty() {
                    files.push(f);
                }
            }
        };

    for raw in patch.lines() {
        let line = raw;
        if line.starts_with("diff --git") {
            flush_file(&mut files, &mut cur, &mut cur_hunk);
            continue;
        }
        if line.starts_with("new file mode") {
            if let Some(f) = cur.as_mut() {
                f.is_new = true;
            }
            continue;
        }
        if line.starts_with("deleted file mode") {
            if let Some(f) = cur.as_mut() {
                f.is_delete = true;
            }
            continue;
        }
        if line.starts_with("index ") || line.starts_with("\\ No newline") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("--- ") {
            // `--- a/path` or for new files `--- /dev/null`.
            flush_file(&mut files, &mut cur, &mut cur_hunk);
            let path = strip_ab(rest);
            let is_new = rest.trim() == "/dev/null";
            cur = Some(FilePatch {
                path: if is_new { String::new() } else { path },
                is_new,
                is_delete: false,
                hunks: Vec::new(),
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            let is_delete = rest.trim() == "/dev/null";
            let path = strip_ab(rest);
            if let Some(f) = cur.as_mut() {
                if !path.is_empty() {
                    f.path = path;
                }
                if is_delete {
                    f.is_delete = true;
                }
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("@@ ") {
            flush_hunk(&mut cur, &mut cur_hunk);
            let old_start = parse_hunk_start(rest).unwrap_or(1);
            cur_hunk = Some(Hunk {
                old_lines: Vec::new(),
                new_lines: Vec::new(),
                old_start,
            });
            continue;
        }
        if let Some(h) = cur_hunk.as_mut() {
            if let Some(l) = line.strip_prefix('-') {
                h.old_lines.push(l.to_string());
            } else if let Some(l) = line.strip_prefix('+') {
                h.new_lines.push(l.to_string());
            } else if let Some(l) = line.strip_prefix(' ') {
                h.old_lines.push(l.to_string());
                h.new_lines.push(l.to_string());
            } else if line.is_empty() {
                // A bare empty line in a hunk is a context blank line.
                h.old_lines.push(String::new());
                h.new_lines.push(String::new());
            }
        }
    }
    flush_file(&mut files, &mut cur, &mut cur_hunk);
    if files.is_empty() {
        return Err("no file patches found in diff".into());
    }
    Ok(files)
}

fn strip_ab(s: &str) -> String {
    let t = s.trim();
    let t = t.split('\t').next().unwrap_or(t).trim();
    t.strip_prefix("a/")
        .or_else(|| t.strip_prefix("b/"))
        .unwrap_or(t)
        .to_string()
}

/// Parse the `-<start>[,<count>]` half of `@@ -1,3 +1,4 @@`.
fn parse_hunk_start(rest: &str) -> Option<usize> {
    let minus = rest.split_whitespace().next()?; // "-1,3"
    let nums = minus.strip_prefix('-')?;
    let start = nums.split(',').next()?;
    start.parse().ok()
}

#[derive(Deserialize)]
struct PatchArg {
    patch: String,
}

pub struct CodeApplyDiffTool;

impl Tool for CodeApplyDiffTool {
    fn name(&self) -> &'static str {
        "code.apply_diff"
    }
    fn description(&self) -> &'static str {
        "Apply a unified diff to the workspace. Args: {\"patch\": \"--- a/f\\n+++ b/f\\n@@ ...\"}"
    }
    fn sensitivity(&self) -> SensitiveAction {
        SensitiveAction::WriteWorkspace
    }
    fn execute(&self, ctx: &ToolContext, args: &serde_json::Value) -> ToolResult {
        let a: PatchArg = match serde_json::from_value(args.clone()) {
            Ok(a) => a,
            Err(e) => return ToolResult::err(self.name(), format!("bad arguments: {e}")),
        };
        let patches = match parse_unified(&a.patch) {
            Ok(p) => p,
            Err(e) => return ToolResult::err(self.name(), e),
        };

        let mut applied: Vec<String> = Vec::new();
        // Two-phase: compute every file's new content first, then write. A
        // failure in any file leaves the workspace untouched (all-or-nothing).
        let mut writes: Vec<(std::path::PathBuf, String)> = Vec::new();
        for p in &patches {
            let path = match resolve(&ctx.root, &p.path) {
                Ok(pth) => pth,
                Err(e) => return ToolResult::err(self.name(), format!("{}: {e}", p.path)),
            };
            let current = std::fs::read_to_string(&path).unwrap_or_default();
            let next = match p.apply_to(&current, &p.path) {
                Ok(n) => n,
                Err(e) => return ToolResult::err(self.name(), e),
            };
            writes.push((path, next));
            applied.push(p.path.clone());
        }
        for (path, content) in writes {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&path, content.as_bytes()) {
                return ToolResult::err(self.name(), format!("write {}: {e}", path.display()));
            }
        }
        ToolResult::ok(
            self.name(),
            format!("applied {} file(s): {}", applied.len(), applied.join(", ")),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> (tempfile::TempDir, ToolContext) {
        let dir = tempfile::TempDir::new().unwrap();
        let c = ToolContext::new(dir.path());
        (dir, c)
    }

    #[test]
    fn parses_and_applies_a_single_hunk() {
        let (_d, c) = ctx();
        std::fs::write(c.root.join("f.txt"), "a\nb\nc\n").unwrap();
        let patch = "--- a/f.txt\n+++ b/f.txt\n@@ -1,3 +1,3 @@\n a\n-b\n+B\n c\n";
        let r = CodeApplyDiffTool.execute(&c, &serde_json::json!({ "patch": patch }));
        assert!(r.ok, "{r:?}");
        assert_eq!(
            std::fs::read_to_string(c.root.join("f.txt")).unwrap(),
            "a\nB\nc\n"
        );
    }

    #[test]
    fn creates_a_new_file() {
        let (_d, c) = ctx();
        let patch = "--- /dev/null\n+++ b/new.txt\n@@ -0,0 +1,2 @@\n+hello\n+world\n";
        let r = CodeApplyDiffTool.execute(&c, &serde_json::json!({ "patch": patch }));
        assert!(r.ok, "{r:?}");
        assert_eq!(
            std::fs::read_to_string(c.root.join("new.txt")).unwrap(),
            "hello\nworld\n"
        );
    }

    #[test]
    fn bad_context_aborts_without_writing() {
        let (_d, c) = ctx();
        std::fs::write(c.root.join("f.txt"), "a\nb\nc\n").unwrap();
        let patch = "--- a/f.txt\n+++ b/f.txt\n@@ -1,3 +1,3 @@\n a\n-MISSING\n+X\n c\n";
        let r = CodeApplyDiffTool.execute(&c, &serde_json::json!({ "patch": patch }));
        assert!(!r.ok);
        assert_eq!(
            std::fs::read_to_string(c.root.join("f.txt")).unwrap(),
            "a\nb\nc\n"
        );
    }

    #[test]
    fn multi_file_patch_applies_all_or_nothing() {
        let (_d, c) = ctx();
        std::fs::write(c.root.join("a.txt"), "1\n").unwrap();
        std::fs::write(c.root.join("b.txt"), "2\n").unwrap();
        let patch = "--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-1\n+one\n\
                     --- a/b.txt\n+++ b/b.txt\n@@ -1 +1 @@\n-nope\n+two\n";
        let r = CodeApplyDiffTool.execute(&c, &serde_json::json!({ "patch": patch }));
        assert!(!r.ok, "second file fails context");
        // First file must remain unmodified (all-or-nothing).
        assert_eq!(
            std::fs::read_to_string(c.root.join("a.txt")).unwrap(),
            "1\n"
        );
    }

    #[test]
    fn rejects_non_diff() {
        let (_d, c) = ctx();
        let r = CodeApplyDiffTool.execute(&c, &serde_json::json!({ "patch": "not a diff" }));
        assert!(!r.ok);
    }
}
