// Atlas OS — Web tools (RFC 63 §4: web.fetch / web.search).
//
// `web.fetch` retrieves a URL and returns its text (HTML stripped to readable
// text, bounded). `web.search` queries a configured search endpoint. Both are
// `NetworkEgress` (RFC 18 §2) so the approval policy gates them, and both are
// opt-in: with no network configured, the tool returns a clear tool-level
// error instead of a panic. Nothing is bundled — this uses the `reqwest` client
// already in the tree (rustls), per RFC 63 §4 "tools externas - nada se bundlea".

use serde::Deserialize;

use crate::security::sandbox::SensitiveAction;

use super::{Tool, ToolContext, ToolResult};

const MAX_BODY: usize = 32_768;
const DEFAULT_TIMEOUT_S: u64 = 30;

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let head = &s[..max / 2];
    let tail = &s[s.len() - max / 2..];
    format!("{head}\n…(truncated)…\n{tail}")
}

/// Very small HTML-to-text: drop `<script>`/`<style>` blocks, strip tags,
/// collapse whitespace and decode the common entities. Good enough for an agent
/// to read a page; the terminal-browser (RFC 28 §I) does the real rendering.
pub fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let mut i = 0;
    let mut skip_tag: Option<&str> = None;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // find end of tag
            if let Some(end) = html[i..].find('>') {
                let tag = &html[i + 1..i + end].trim().to_ascii_lowercase();
                if tag.starts_with("script") {
                    skip_tag = Some("</script");
                } else if tag.starts_with("style") {
                    skip_tag = Some("</style");
                }
                i += end + 1;
                continue;
            }
        }
        if let Some(st) = skip_tag {
            if let Some(pos) = html[i..].to_ascii_lowercase().find(st) {
                i += pos + st.len();
                skip_tag = None;
                continue;
            } else {
                break;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[derive(Deserialize)]
struct FetchArg {
    url: String,
    /// When true (default), strip HTML to text; set false to keep raw HTML.
    #[serde(default = "default_true")]
    text: bool,
}

fn default_true() -> bool {
    true
}

pub struct WebFetchTool;

impl Tool for WebFetchTool {
    fn name(&self) -> &'static str {
        "web.fetch"
    }
    fn description(&self) -> &'static str {
        "Fetch a URL and return its text. Args: {\"url\": \"https://...\", \"text\": true}"
    }
    fn sensitivity(&self) -> SensitiveAction {
        SensitiveAction::NetworkEgress
    }
    fn execute(&self, _ctx: &ToolContext, args: &serde_json::Value) -> ToolResult {
        let a: FetchArg = match serde_json::from_value(args.clone()) {
            Ok(a) => a,
            Err(e) => return ToolResult::err(self.name(), format!("bad arguments: {e}")),
        };
        if !(a.url.starts_with("http://") || a.url.starts_with("https://")) {
            return ToolResult::err(self.name(), "url must start with http:// or https://");
        }
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_S))
            .build()
        {
            Ok(c) => c,
            Err(e) => return ToolResult::err(self.name(), format!("http client: {e}")),
        };
        match client.get(&a.url).send() {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                let rendered = if a.text { html_to_text(&body) } else { body };
                if !status.is_success() {
                    return ToolResult::err(
                        self.name(),
                        format!("HTTP {} for {}", status.as_u16(), a.url),
                    );
                }
                ToolResult::ok(self.name(), truncate(&rendered, MAX_BODY))
            }
            Err(e) => ToolResult::err(self.name(), format!("fetch {}: {e}", a.url)),
        }
    }
}

#[derive(Deserialize)]
struct SearchArg {
    query: String,
}

pub struct WebSearchTool;

impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web.search"
    }
    fn description(&self) -> &'static str {
        "Search the web. Args: {\"query\": \"...\"} (needs ATLAS_SEARCH_URL)"
    }
    fn sensitivity(&self) -> SensitiveAction {
        SensitiveAction::NetworkEgress
    }
    fn execute(&self, _ctx: &ToolContext, args: &serde_json::Value) -> ToolResult {
        let a: SearchArg = match serde_json::from_value(args.clone()) {
            Ok(a) => a,
            Err(e) => return ToolResult::err(self.name(), format!("bad arguments: {e}")),
        };
        // Opt-in: a search backend URL plus `{query}` placeholder, e.g.
        //   ATLAS_SEARCH_URL=https://api.search.example/search?q={query}
        let template = match std::env::var("ATLAS_SEARCH_URL") {
            Ok(t) if !t.trim().is_empty() => t,
            _ => {
                return ToolResult::err(
                    self.name(),
                    "no search backend: set ATLAS_SEARCH_URL (use {query} as the placeholder)",
                )
            }
        };
        let url = template.replace("{query}", &urlencode(&a.query));
        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_S))
            .build()
        {
            Ok(c) => c,
            Err(e) => return ToolResult::err(self.name(), format!("http client: {e}")),
        };
        match client.get(&url).send() {
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().unwrap_or_default();
                if !status.is_success() {
                    return ToolResult::err(
                        self.name(),
                        format!("search backend HTTP {} for {}", status.as_u16(), url),
                    );
                }
                ToolResult::ok(self.name(), truncate(&body, MAX_BODY))
            }
            Err(e) => ToolResult::err(self.name(), format!("search: {e}")),
        }
    }
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_to_text_strips_tags_and_scripts() {
        let html = "<html><head><style>p{color:red}</style></head>\
                    <body><h1>Hi &amp; bye</h1><script>x=1</script><p>Body text</p></body></html>";
        let t = html_to_text(html);
        assert!(t.contains("Hi & bye"), "{t}");
        assert!(t.contains("Body text"));
        assert!(!t.contains("color:red"));
        assert!(!t.contains("x=1"));
        assert!(!t.contains('<'));
    }

    #[test]
    fn fetch_rejects_non_http_scheme() {
        let c = ToolContext::new(".");
        let r = WebFetchTool.execute(&c, &serde_json::json!({"url": "file:///etc/passwd"}));
        assert!(!r.ok);
    }

    #[test]
    fn search_without_backend_is_a_tool_error() {
        // Ensure the env var is absent for this assertion.
        std::env::remove_var("ATLAS_SEARCH_URL");
        let c = ToolContext::new(".");
        let r = WebSearchTool.execute(&c, &serde_json::json!({"query": "rust"}));
        assert!(!r.ok);
        assert!(r.error.unwrap().contains("ATLAS_SEARCH_URL"));
    }

    #[test]
    fn urlencode_escapes() {
        assert_eq!(urlencode("a b&c=d"), "a+b%26c%3Dd");
    }
}
