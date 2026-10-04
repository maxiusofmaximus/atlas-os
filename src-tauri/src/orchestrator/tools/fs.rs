// Atlas OS — Filesystem tools (RFC 63 §4: fs.read/write/edit/list).
//
// Path handling refuses escapes outside the workspace root (belt-and-braces on
// top of the RFC 18 `SensitiveAction` policy): a `..` that leaves `root` is an
// error, never a silent read/write elsewhere. Args are JSON objects; a malformed
// arg is `BadArgs`, a tool-level failure is a normal `ToolResult` (never panic).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::security::sandbox::SensitiveAction;

use super::{Tool, ToolContext, ToolResult};

/// Resolve `rel` inside `root`, refusing any escape. Returns the absolute path
/// only if it stays within `root` after normalization.
fn resolve(root: &Path, rel: &str) -> Result<PathBuf, String> {
    if rel.trim().is_empty() {
        return Err("empty path".into());
    }
    let candidate = Path::new(rel);
    let joined = if candidate.is_absolute() {
        // Absolute paths are only allowed if they are already under root.
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    // Lexical normalization (no filesystem access): collapse `.` and `..`.
    let mut out = PathBuf::new();
    for comp in joined.components() {
        use std::path::Component::*;
        match comp {
            CurDir => {}
            ParentDir => {
                if !out.pop() {
                    return Err("path escapes workspace root".into());
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    let root_norm = root.components().fold(PathBuf::new(), |mut a, c| {
        a.push(c.as_os_str());
        a
    });
    if !out.starts_with(&root_norm) {
        return Err("path escapes workspace root".into());
    }
    Ok(out)
}

#[derive(Deserialize)]
struct PathArg {
    path: String,
}

pub struct FsReadTool;
impl Tool for FsReadTool {
    fn name(&self) -> &'static str {
        "fs.read"
    }
    fn description(&self) -> &'static str {
        "Read a file inside the workspace. Args: {\"path\": \"src/lib.rs\"}"
    }
    fn sensitivity(&self) -> SensitiveAction {
        SensitiveAction::ReadWorkspace
    }
    fn execute(&self, ctx: &ToolContext, args: &serde_json::Value) -> ToolResult {
        let a: PathArg = match serde_json::from_value(args.clone()) {
            Ok(a) => a,
            Err(e) => return ToolResult::err(self.name(), format!("bad arguments: {e}")),
        };
        let path = match resolve(&ctx.root, &a.path) {
            Ok(p) => p,
            Err(e) => return ToolResult::err(self.name(), e),
        };
        match std::fs::read_to_string(&path) {
            Ok(mut s) => {
                if s.len() > ctx.max_output_bytes {
                    s.truncate(ctx.max_output_bytes);
                    s.push_str("\n…(truncated)…");
                }
                ToolResult::ok(self.name(), s)
            }
            Err(e) => ToolResult::err(self.name(), format!("read {}: {e}", a.path)),
        }
    }
}

#[derive(Deserialize)]
struct WriteArg {
    path: String,
    content: String,
}

pub struct FsWriteTool;
impl Tool for FsWriteTool {
    fn name(&self) -> &'static str {
        "fs.write"
    }
    fn description(&self) -> &'static str {
        "Create/overwrite a file. Args: {\"path\": \"...\", \"content\": \"...\"}"
    }
    fn sensitivity(&self) -> SensitiveAction {
        SensitiveAction::WriteWorkspace
    }
    fn execute(&self, ctx: &ToolContext, args: &serde_json::Value) -> ToolResult {
        let a: WriteArg = match serde_json::from_value(args.clone()) {
            Ok(a) => a,
            Err(e) => return ToolResult::err(self.name(), format!("bad arguments: {e}")),
        };
        let path = match resolve(&ctx.root, &a.path) {
            Ok(p) => p,
            Err(e) => return ToolResult::err(self.name(), e),
        };
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolResult::err(self.name(), format!("mkdir {}: {e}", parent.display()));
            }
        }
        match std::fs::write(&path, a.content.as_bytes()) {
            Ok(()) => ToolResult::ok(
                self.name(),
                format!("wrote {} ({} bytes)", a.path, a.content.len()),
            ),
            Err(e) => ToolResult::err(self.name(), format!("write {}: {e}", a.path)),
        }
    }
}

#[derive(Deserialize)]
struct EditArg {
    path: String,
    old: String,
    new: String,
    #[serde(default)]
    replace_all: bool,
}

pub struct FsEditTool;
impl Tool for FsEditTool {
    fn name(&self) -> &'static str {
        "fs.edit"
    }
    fn description(&self) -> &'static str {
        "Replace `old` with `new` in a file. Args: {\"path\", \"old\", \"new\", \"replace_all\"?}"
    }
    fn sensitivity(&self) -> SensitiveAction {
        SensitiveAction::WriteWorkspace
    }
    fn execute(&self, ctx: &ToolContext, args: &serde_json::Value) -> ToolResult {
        let a: EditArg = match serde_json::from_value(args.clone()) {
            Ok(a) => a,
            Err(e) => return ToolResult::err(self.name(), format!("bad arguments: {e}")),
        };
        let path = match resolve(&ctx.root, &a.path) {
            Ok(p) => p,
            Err(e) => return ToolResult::err(self.name(), e),
        };
        let body = match std::fs::read_to_string(&path) {
            Ok(b) => b,
            Err(e) => return ToolResult::err(self.name(), format!("read {}: {e}", a.path)),
        };
        if !body.contains(&a.old) {
            return ToolResult::err(self.name(), format!("`old` not found in {}", a.path));
        }
        let (new_body, count) = if a.replace_all {
            let n = body.matches(&a.old).count();
            (body.replace(&a.old, &a.new), n)
        } else {
            (body.replacen(&a.old, &a.new, 1), 1)
        };
        match std::fs::write(&path, new_body.as_bytes()) {
            Ok(()) => ToolResult::ok(
                self.name(),
                format!("edited {} ({count} replacement(s))", a.path),
            ),
            Err(e) => ToolResult::err(self.name(), format!("write {}: {e}", a.path)),
        }
    }
}

#[derive(Deserialize)]
struct ListArg {
    #[serde(default = "default_path")]
    path: String,
}

fn default_path() -> String {
    ".".into()
}

pub struct FsListTool;
impl Tool for FsListTool {
    fn name(&self) -> &'static str {
        "fs.list"
    }
    fn description(&self) -> &'static str {
        "List a directory. Args: {\"path\": \".\"} (defaults to root)"
    }
    fn sensitivity(&self) -> SensitiveAction {
        SensitiveAction::ReadWorkspace
    }
    fn execute(&self, ctx: &ToolContext, args: &serde_json::Value) -> ToolResult {
        let a: ListArg = serde_json::from_value(args.clone()).unwrap_or(ListArg {
            path: default_path(),
        });
        let path = match resolve(&ctx.root, &a.path) {
            Ok(p) => p,
            Err(e) => return ToolResult::err(self.name(), e),
        };
        let entries = match std::fs::read_dir(&path) {
            Ok(e) => e,
            Err(e) => return ToolResult::err(self.name(), format!("list {}: {e}", a.path)),
        };
        let mut lines: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            lines.push(if is_dir { format!("{name}/") } else { name });
        }
        lines.sort();
        ToolResult::ok(self.name(), lines.join("\n"))
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
    fn write_then_read_round_trips() {
        let (_d, c) = ctx();
        let w = FsWriteTool.execute(
            &c,
            &serde_json::json!({"path": "a/b.txt", "content": "hello"}),
        );
        assert!(w.ok, "{w:?}");
        let r = FsReadTool.execute(&c, &serde_json::json!({"path": "a/b.txt"}));
        assert!(r.ok);
        assert_eq!(r.output, "hello");
    }

    #[test]
    fn read_missing_file_is_tool_error_not_panic() {
        let (_d, c) = ctx();
        let r = FsReadTool.execute(&c, &serde_json::json!({"path": "nope.txt"}));
        assert!(!r.ok);
    }

    #[test]
    fn path_escape_is_refused() {
        let (_d, c) = ctx();
        let r = FsReadTool.execute(&c, &serde_json::json!({"path": "../../etc/passwd"}));
        assert!(!r.ok);
        assert!(r.error.unwrap().contains("escapes"));
    }

    #[test]
    fn edit_replaces_once_by_default() {
        let (_d, c) = ctx();
        FsWriteTool.execute(
            &c,
            &serde_json::json!({"path": "x.txt", "content": "a a a"}),
        );
        let e = FsEditTool.execute(
            &c,
            &serde_json::json!({"path": "x.txt", "old": "a", "new": "b"}),
        );
        assert!(e.ok);
        let r = FsReadTool.execute(&c, &serde_json::json!({"path": "x.txt"}));
        assert_eq!(r.output, "b a a");
    }

    #[test]
    fn edit_missing_needle_is_tool_error() {
        let (_d, c) = ctx();
        FsWriteTool.execute(&c, &serde_json::json!({"path": "x.txt", "content": "abc"}));
        let e = FsEditTool.execute(
            &c,
            &serde_json::json!({"path": "x.txt", "old": "zzz", "new": "y"}),
        );
        assert!(!e.ok);
    }

    #[test]
    fn list_shows_dirs_with_slash() {
        let (_d, c) = ctx();
        FsWriteTool.execute(
            &c,
            &serde_json::json!({"path": "sub/f.txt", "content": "x"}),
        );
        let l = FsListTool.execute(&c, &serde_json::json!({}));
        assert!(l.ok);
        assert!(l.output.contains("sub/"));
    }

    #[test]
    fn bad_args_do_not_panic() {
        let (_d, c) = ctx();
        let r = FsWriteTool.execute(&c, &serde_json::json!({"missing": true}));
        assert!(!r.ok);
    }
}
