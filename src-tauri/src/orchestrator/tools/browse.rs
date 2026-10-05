// Atlas OS — Browse tools (RFC 63 §4/§7: browse.open / browse.snapshot).
//
// Lateral backend: this does NOT bundle a browser. It shells out to the
// `terminal-browser` CLI (RFC 28 §I) when it is on PATH — the same "external
// process, never bundled" contract as the WSL2 sandbox (RFC 25 §11). When the
// CLI is absent, the tool returns a clear, actionable error instead of
// pretending to browse. `NetworkEgress` sensitivity, so the policy gates it.

use serde::Deserialize;

use crate::security::sandbox::SensitiveAction;

use super::{Tool, ToolContext, ToolResult};

/// The external CLI this backend drives (RFC 28 §I). Overridable for tests.
const BROWSE_BIN: &str = "terminal-browser";

fn bin() -> String {
    std::env::var("ATLAS_BROWSE_BIN").unwrap_or_else(|_| BROWSE_BIN.to_string())
}

/// Run `terminal-browser <args>` and render stdout/stderr. Absence of the CLI is
/// a tool error naming how to get it, not a panic.
fn run_browser(args: &[&str]) -> ToolResult {
    let program = bin();
    match std::process::Command::new(&program).args(args).output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success() {
                ToolResult::ok(BROWSE_BIN, stdout.trim())
            } else {
                ToolResult::err(
                    BROWSE_BIN,
                    format!(
                        "{} exited {}: {}",
                        program,
                        out.status.code().unwrap_or(-1),
                        stderr.trim()
                    ),
                )
            }
        }
        Err(e) => ToolResult::err(
            BROWSE_BIN,
            format!("`{program}` not found ({e}); install terminal-browser (RFC 28 §I) or set ATLAS_BROWSE_BIN"),
        ),
    }
}

#[derive(Deserialize)]
struct OpenArg {
    url: String,
}

pub struct BrowseOpenTool;

impl Tool for BrowseOpenTool {
    fn name(&self) -> &'static str {
        "browse.open"
    }
    fn description(&self) -> &'static str {
        "Open a URL in the terminal browser (RFC 28 §I, lateral). Args: {\"url\": \"https://...\"}"
    }
    fn sensitivity(&self) -> SensitiveAction {
        SensitiveAction::NetworkEgress
    }
    fn execute(&self, _ctx: &ToolContext, args: &serde_json::Value) -> ToolResult {
        let a: OpenArg = match serde_json::from_value(args.clone()) {
            Ok(a) => a,
            Err(e) => return ToolResult::err(self.name(), format!("bad arguments: {e}")),
        };
        if !(a.url.starts_with("http://") || a.url.starts_with("https://")) {
            return ToolResult::err(self.name(), "url must start with http:// or https://");
        }
        run_browser(&["open", &a.url])
    }
}

pub struct BrowseSnapshotTool;

impl Tool for BrowseSnapshotTool {
    fn name(&self) -> &'static str {
        "browse.snapshot"
    }
    fn description(&self) -> &'static str {
        "Snapshot the current browser page as text (RFC 28 §I, lateral). Args: {}"
    }
    fn sensitivity(&self) -> SensitiveAction {
        SensitiveAction::NetworkEgress
    }
    fn execute(&self, _ctx: &ToolContext, _args: &serde_json::Value) -> ToolResult {
        run_browser(&["snapshot"])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_rejects_non_http() {
        let c = ToolContext::new(".");
        let r = BrowseOpenTool.execute(&c, &serde_json::json!({"url": "ftp://x"}));
        assert!(!r.ok);
    }

    #[test]
    fn missing_cli_is_an_actionable_error() {
        // Point at a binary that cannot exist, so the spawn fails deterministically.
        std::env::set_var("ATLAS_BROWSE_BIN", "terminal-browser-does-not-exist-xyz");
        let c = ToolContext::new(".");
        let r = BrowseSnapshotTool.execute(&c, &serde_json::json!({}));
        std::env::remove_var("ATLAS_BROWSE_BIN");
        assert!(!r.ok);
        assert!(r.error.unwrap().contains("not found"));
    }
}
