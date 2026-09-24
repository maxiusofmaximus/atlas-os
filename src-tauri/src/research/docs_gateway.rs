// Atlas OS — Docs gateway (RFC 10 §6, Phase 3 sub-fase 3.1).
//
// Adapter facade over the operator's Context7Max installation, mirroring
// the firecrawl facade pattern (RFC 28 §E): every Atlas OS consumer that
// needs live library documentation (the `atlas research docs` CLI today,
// the 3.3 collective-intelligence `Official` scorer tomorrow, the HUD
// results surface as follow-up) calls through `DocsGateway` and never
// shells out to `ctx7max` or guesses an HTTP contract directly.
//
// Backend resolution (`from_env`, highest priority first):
//   1. `ATLAS_CTX7MAX_URL` set → Context7Max HTTP API (self-hosted
//      Supabase + Vercel deployment, plan 30 §A.1).
//   2. `ATLAS_CTX7MAX_BIN` set, or a `ctx7max` binary found in `PATH`
//      → Context7Max CLI (`ctx7max docs <library> "<question>"`).
//   3. Neither → `Context7Mcp` marker: the Rust core ships no MCP
//      client, so this backend always yields to the next one and tells
//      the agent host (AGENTS.md §5) to answer via Context7 MCP.
//   4. Last resort → `OfficialDocs`: plain HTTPS fetch of the official
//      docs index. Never fabricates snippets.
//
// `query_docs` walks the fallback chain and returns the first non-empty
// hit list; an exhausted chain yields `Ok(vec![])` ("no data", never a
// hallucinated answer — RFC 10 §10 fail-safe). Only caller bugs (empty
// library id / empty question) are hard errors.
//
// No new crate (RFC 25 §11): HTTP rides the existing `reqwest`, the CLI
// rides `tokio::process`, parsing is `serde_json` only.

use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::time::Duration;

/// Env var carrying the Context7Max HTTP API base URL (plan 30 §B 3.1).
pub const CTX7MAX_URL_ENV: &str = "ATLAS_CTX7MAX_URL";
/// Env var pinning the Context7Max CLI binary (operator override for
/// installs outside `PATH`; the `PATH` lookup remains the default).
pub const CTX7MAX_BIN_ENV: &str = "ATLAS_CTX7MAX_BIN";
/// Binary name probed in `PATH` when no explicit override is set.
pub const CTX7MAX_BIN: &str = "ctx7max";

const API_TIMEOUT_SECS: u64 = 15;
const OFFICIAL_DOCS_TIMEOUT_SECS: u64 = 10;
const GATEWAY_USER_AGENT: &str = "atlas-os/0.1 (research docs-gateway)";

/// One documentation hit: verbatim example code plus hybrid-search
/// provenance (plan 30 §B 3.1). The struct is the stable wire shape the
/// CLI prints as JSON lines, so downstream pipes can consume it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocSnippet {
    pub library_id: String,
    pub title: Option<String>,
    pub url: Option<String>,
    pub content: String,
    pub score: Option<f64>,
}

impl DocSnippet {
    pub fn new(library_id: &str, content: &str) -> Self {
        Self {
            library_id: library_id.into(),
            title: None,
            url: None,
            content: content.into(),
            score: None,
        }
    }
}

/// Which documentation source the gateway talks to. Ordered by
/// preference; `fallback_chain` appends the downgrades.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocsBackend {
    Ctx7MaxApi { base_url: String },
    Ctx7MaxCli { bin: PathBuf },
    Context7Mcp,
    OfficialDocs,
}

/// Resolved gateway. Cheap to clone; build once per CLI invocation.
#[derive(Clone, Debug)]
pub struct DocsGateway {
    backend: DocsBackend,
}

impl DocsGateway {
    pub fn from_env() -> Self {
        let url = env::var(CTX7MAX_URL_ENV).ok();
        let bin = env::var(CTX7MAX_BIN_ENV).ok();
        Self {
            backend: resolve(url.as_deref(), bin.as_deref()),
        }
    }

    pub fn from_explicit(url: Option<&str>, bin: Option<&str>) -> Self {
        Self {
            backend: resolve(url, bin),
        }
    }

    pub fn backend(&self) -> &DocsBackend {
        &self.backend
    }

    pub fn fallback_chain(&self) -> Vec<DocsBackend> {
        let mut chain = vec![self.backend.clone()];
        if self.backend != DocsBackend::Context7Mcp {
            chain.push(DocsBackend::Context7Mcp);
        }
        chain.push(DocsBackend::OfficialDocs);
        chain
    }

    pub async fn query_docs(
        &self,
        library_id: &str,
        question: &str,
    ) -> Result<Vec<DocSnippet>, DocsGatewayError> {
        let library = library_id.trim();
        if library.is_empty() {
            return Err(DocsGatewayError::EmptyLibrary);
        }
        let question = question.trim();
        if question.is_empty() {
            return Err(DocsGatewayError::EmptyQuestion);
        }
        for backend in self.fallback_chain() {
            match &backend {
                DocsBackend::Ctx7MaxApi { base_url } => {
                    if let Ok(hits) = query_api(base_url, library, question).await {
                        if !hits.is_empty() {
                            return Ok(hits);
                        }
                    }
                }
                DocsBackend::Ctx7MaxCli { bin } => {
                    if let Ok(hits) = query_cli(bin, library, question).await {
                        if !hits.is_empty() {
                            return Ok(hits);
                        }
                    }
                }
                DocsBackend::Context7Mcp => continue,
                DocsBackend::OfficialDocs => return Ok(query_official_docs(library).await),
            }
        }
        Ok(Vec::new())
    }
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum DocsGatewayError {
    #[error("docs gateway: library id must not be empty")]
    EmptyLibrary,
    #[error("docs gateway: question must not be empty")]
    EmptyQuestion,
    #[error("docs gateway: ctx7max CLI `{bin}` failed: {detail}")]
    CliFailed { bin: String, detail: String },
    #[error("docs gateway: ctx7max API `{url}` failed: {detail}")]
    ApiFailed { url: String, detail: String },
    #[error("docs gateway: Context7 MCP fallback requires the agent host (AGENTS.md §5); the Rust core ships no MCP client")]
    McpRequiresAgent,
    #[error("docs gateway: response parse error: {0}")]
    Parse(String),
}

fn resolve(url: Option<&str>, bin: Option<&str>) -> DocsBackend {
    let url = url.map(str::trim).filter(|s| !s.is_empty());
    if let Some(u) = url {
        return DocsBackend::Ctx7MaxApi {
            base_url: u.to_string(),
        };
    }
    let bin = bin.map(str::trim).filter(|s| !s.is_empty());
    if let Some(b) = bin {
        return DocsBackend::Ctx7MaxCli {
            bin: PathBuf::from(b),
        };
    }
    if let Some(found) = find_ctx7max_in_path() {
        return DocsBackend::Ctx7MaxCli { bin: found };
    }
    DocsBackend::Context7Mcp
}

fn find_ctx7max_in_path() -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let names: &[&str] = if cfg!(windows) {
        &[CTX7MAX_BIN, "ctx7max.exe"]
    } else {
        &[CTX7MAX_BIN]
    };
    for dir in env::split_paths(&path) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

async fn query_cli(
    bin: &PathBuf,
    library: &str,
    question: &str,
) -> Result<Vec<DocSnippet>, DocsGatewayError> {
    let bin_str = bin.to_string_lossy().to_string();
    let out = tokio::process::Command::new(bin)
        .arg("docs")
        .arg(library)
        .arg(question)
        .output()
        .await
        .map_err(|e| DocsGatewayError::CliFailed {
            bin: bin_str.clone(),
            detail: e.to_string(),
        })?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let detail = stderr.trim();
        let detail = if detail.is_empty() {
            format!("exit {}", out.status)
        } else if detail.len() > 300 {
            detail[..300].to_string()
        } else {
            detail.to_string()
        };
        return Err(DocsGatewayError::CliFailed {
            bin: bin_str,
            detail,
        });
    }
    Ok(parse_cli_output(
        &String::from_utf8_lossy(&out.stdout),
        library,
    ))
}

async fn query_api(
    base_url: &str,
    library: &str,
    question: &str,
) -> Result<Vec<DocSnippet>, DocsGatewayError> {
    let fail = |detail: String| DocsGatewayError::ApiFailed {
        url: base_url.to_string(),
        detail,
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(API_TIMEOUT_SECS))
        .user_agent(GATEWAY_USER_AGENT)
        .build()
        .map_err(|e| fail(e.to_string()))?;
    let endpoint = format!("{}/docs", base_url.trim_end_matches('/'));
    let resp = client
        .get(&endpoint)
        .query(&[("library", library), ("question", question)])
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| fail(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(fail(format!("HTTP {}", resp.status())));
    }
    let value: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| DocsGatewayError::Parse(e.to_string()))?;
    parse_snippets_json(&value, library)
}

/// Lenient `ctx7max docs` stdout parser: JSON array / known envelope
/// shapes first, plain-text lines as final shape (one snippet per
/// non-empty line). JSON that parses but matches no known shape yields
/// no rows so the fallback chain keeps walking.
pub fn parse_cli_output(stdout: &str, library_id: &str) -> Vec<DocSnippet> {
    match serde_json::from_str::<serde_json::Value>(stdout) {
        Ok(value) => parse_snippets_json(&value, library_id).unwrap_or_default(),
        Err(_) => stdout
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| DocSnippet::new(library_id, l))
            .collect(),
    }
}

/// Lenient snippet-list parser shared by the CLI and API backends.
/// Accepts a bare array, an envelope object holding the array under
/// `snippets` / `docs` / `results` / `items` / `data`, or a single
/// snippet object. Items without usable text are skipped.
pub fn parse_snippets_json(
    value: &serde_json::Value,
    library_id: &str,
) -> Result<Vec<DocSnippet>, DocsGatewayError> {
    if let Some(arr) = value.as_array() {
        return Ok(map_items(arr, library_id));
    }
    if let Some(obj) = value.as_object() {
        for key in ["snippets", "docs", "results", "items", "data"] {
            if let Some(arr) = obj.get(key).and_then(|v| v.as_array()) {
                return Ok(map_items(arr, library_id));
            }
        }
        if let Some(one) = map_item(value, library_id) {
            return Ok(vec![one]);
        }
    }
    Err(DocsGatewayError::Parse(
        "unrecognised docs payload shape".into(),
    ))
}

fn map_items(arr: &[serde_json::Value], library_id: &str) -> Vec<DocSnippet> {
    arr.iter().filter_map(|v| map_item(v, library_id)).collect()
}

fn map_item(value: &serde_json::Value, library_id: &str) -> Option<DocSnippet> {
    if let Some(s) = value.as_str() {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        return Some(DocSnippet::new(library_id, s));
    }
    let obj = value.as_object()?;
    let content = ["content", "snippet", "code", "text"]
        .iter()
        .filter_map(|k| obj.get(*k).and_then(|v| v.as_str()))
        .map(str::trim)
        .find(|s| !s.is_empty())?;
    let str_field = |k: &str| {
        obj.get(k)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    Some(DocSnippet {
        library_id: library_id.into(),
        title: str_field("title").or_else(|| str_field("name")),
        url: str_field("url").or_else(|| str_field("link")),
        content: content.to_string(),
        score: obj.get("score").and_then(|v| v.as_f64()),
    })
}

/// Official-docs index URL for a crate-style library id. Returns `None`
/// for ids outside `[A-Za-z0-9_-]` so the fetch below never builds a
/// URL from attacker-shaped input.
pub fn official_docs_url(library_id: &str) -> Option<String> {
    let id = library_id.trim();
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some(format!("https://docs.rs/crate/{id}/latest"))
}

/// Last-resort backend: HTTPS fetch of the official docs index. Returns
/// a single pointer snippet on HTTP 200, an empty vec on any other
/// status or transport failure — the gateway reports "no data", never
/// a fabricated answer.
async fn query_official_docs(library: &str) -> Vec<DocSnippet> {
    let Some(url) = official_docs_url(library) else {
        return Vec::new();
    };
    let Ok(client) = reqwest::Client::builder()
        .timeout(Duration::from_secs(OFFICIAL_DOCS_TIMEOUT_SECS))
        .user_agent(GATEWAY_USER_AGENT)
        .build()
    else {
        return Vec::new();
    };
    let Ok(resp) = client.get(&url).header("Accept", "text/html").send().await else {
        return Vec::new();
    };
    if !resp.status().is_success() {
        return Vec::new();
    }
    vec![DocSnippet {
        library_id: library.into(),
        title: Some("official docs index".into()),
        url: Some(url.clone()),
        content: format!("Official docs index for `{library}`: {url}"),
        score: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_clean_env<F: FnOnce()>(f: F) {
        let _g = ENV_LOCK.lock().unwrap();
        let url_b = env::var_os(CTX7MAX_URL_ENV);
        let bin_b = env::var_os(CTX7MAX_BIN_ENV);
        let path_b = env::var_os("PATH");
        env::remove_var(CTX7MAX_URL_ENV);
        env::remove_var(CTX7MAX_BIN_ENV);
        env::set_var("PATH", std::env::temp_dir().join("atlas-os-no-ctx7max-xyz"));
        f();
        if let Some(v) = url_b {
            env::set_var(CTX7MAX_URL_ENV, v);
        } else {
            env::remove_var(CTX7MAX_URL_ENV);
        }
        if let Some(v) = bin_b {
            env::set_var(CTX7MAX_BIN_ENV, v);
        } else {
            env::remove_var(CTX7MAX_BIN_ENV);
        }
        if let Some(v) = path_b {
            env::set_var("PATH", v);
        } else {
            env::remove_var("PATH");
        }
    }

    #[test]
    fn explicit_api_url_wins_over_everything() {
        let gw = DocsGateway::from_explicit(Some("http://localhost:8090"), Some("/bin/ctx7max"));
        assert_eq!(
            gw.backend(),
            &DocsBackend::Ctx7MaxApi {
                base_url: "http://localhost:8090".into()
            }
        );
    }

    #[test]
    fn explicit_bin_selects_cli_backend() {
        let gw = DocsGateway::from_explicit(None, Some("/usr/local/bin/ctx7max"));
        assert_eq!(
            gw.backend(),
            &DocsBackend::Ctx7MaxCli {
                bin: PathBuf::from("/usr/local/bin/ctx7max")
            }
        );
    }

    #[test]
    fn blank_url_falls_through_to_explicit_bin() {
        let gw = DocsGateway::from_explicit(Some("   "), Some("/bin/ctx7max"));
        assert!(matches!(gw.backend(), DocsBackend::Ctx7MaxCli { .. }));
    }

    #[test]
    fn empty_env_resolves_to_context7_mcp_marker() {
        with_clean_env(|| {
            let gw = DocsGateway::from_env();
            assert_eq!(gw.backend(), &DocsBackend::Context7Mcp);
        });
    }

    #[test]
    fn env_url_selects_api_backend() {
        with_clean_env(|| {
            env::set_var(CTX7MAX_URL_ENV, "https://ctx7max.example.invalid");
            let gw = DocsGateway::from_env();
            assert_eq!(
                gw.backend(),
                &DocsBackend::Ctx7MaxApi {
                    base_url: "https://ctx7max.example.invalid".into()
                }
            );
        });
    }

    #[test]
    fn env_bin_selects_cli_backend() {
        with_clean_env(|| {
            env::set_var(CTX7MAX_BIN_ENV, "/opt/ctx7max/bin/ctx7max");
            let gw = DocsGateway::from_env();
            assert_eq!(
                gw.backend(),
                &DocsBackend::Ctx7MaxCli {
                    bin: PathBuf::from("/opt/ctx7max/bin/ctx7max")
                }
            );
        });
    }

    #[test]
    fn fallback_chain_walks_mcp_then_official_docs() {
        let gw = DocsGateway::from_explicit(Some("http://localhost:8090"), None);
        let chain = gw.fallback_chain();
        assert_eq!(chain.len(), 3);
        assert!(matches!(chain[0], DocsBackend::Ctx7MaxApi { .. }));
        assert_eq!(chain[1], DocsBackend::Context7Mcp);
        assert_eq!(chain[2], DocsBackend::OfficialDocs);
    }

    #[test]
    fn fallback_chain_from_mcp_skips_duplicate_mcp() {
        let gw = DocsGateway::from_explicit(None, None);
        let _ = gw.backend().clone();
        let mcp = DocsGateway {
            backend: DocsBackend::Context7Mcp,
        };
        assert_eq!(
            mcp.fallback_chain(),
            vec![DocsBackend::Context7Mcp, DocsBackend::OfficialDocs]
        );
    }

    #[tokio::test]
    async fn query_rejects_empty_library() {
        let gw = DocsGateway::from_explicit(Some("http://localhost:8090"), None);
        assert_eq!(
            gw.query_docs("   ", "how to spawn?").await.unwrap_err(),
            DocsGatewayError::EmptyLibrary
        );
    }

    #[tokio::test]
    async fn query_rejects_empty_question() {
        let gw = DocsGateway::from_explicit(Some("http://localhost:8090"), None);
        assert_eq!(
            gw.query_docs("tokio", "  ").await.unwrap_err(),
            DocsGatewayError::EmptyQuestion
        );
    }

    #[tokio::test]
    async fn query_with_dead_cli_falls_through_to_ok_empty() {
        let gw = DocsGateway::from_explicit(None, Some("/nonexistent-atlas-ctx7max-bin-xyz"));
        let out = gw
            .query_docs("no-such-crate-atlas-xyz-123", "how to X?")
            .await;
        assert!(out.is_ok());
    }

    #[test]
    fn parse_cli_json_array_shape() {
        let stdout = r#"[{"title":"spawn","url":"https://example.invalid/s","content":"tokio::spawn(async {});","score":0.9}]"#;
        let hits = parse_cli_output(stdout, "tokio");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].library_id, "tokio");
        assert_eq!(hits[0].title.as_deref(), Some("spawn"));
        assert_eq!(hits[0].url.as_deref(), Some("https://example.invalid/s"));
        assert_eq!(hits[0].content, "tokio::spawn(async {});");
        assert_eq!(hits[0].score, Some(0.9));
    }

    #[test]
    fn parse_cli_envelope_object_shape() {
        let stdout = r#"{"docs":[{"code":"let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;"}]}"#;
        let hits = parse_cli_output(stdout, "tokio");
        assert_eq!(hits.len(), 1);
        assert!(hits[0].content.contains("new_multi_thread"));
    }

    #[test]
    fn parse_cli_single_object_shape() {
        let stdout = r#"{"snippet":"use tokio::fs;"}"#;
        let hits = parse_cli_output(stdout, "tokio");
        assert_eq!(hits, vec![DocSnippet::new("tokio", "use tokio::fs;")]);
    }

    #[test]
    fn parse_cli_unknown_json_shape_yields_no_rows() {
        let hits = parse_cli_output(r#"{"status":"ok","count":0}"#, "tokio");
        assert!(hits.is_empty());
    }

    #[test]
    fn parse_cli_plain_text_falls_back_to_lines() {
        let hits = parse_cli_output("first snippet\n\nsecond snippet\n", "tokio");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].content, "first snippet");
        assert_eq!(hits[1].content, "second snippet");
    }

    #[test]
    fn parse_skips_items_without_text() {
        let value: serde_json::Value =
            serde_json::from_str(r#"[{"title":"empty"},{"content":"kept"}]"#).unwrap();
        let hits = parse_snippets_json(&value, "tokio").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].content, "kept");
    }

    #[test]
    fn official_docs_url_accepts_crate_ids() {
        assert_eq!(
            official_docs_url("tokio"),
            Some("https://docs.rs/crate/tokio/latest".into())
        );
    }

    #[test]
    fn official_docs_url_rejects_shaped_input() {
        assert_eq!(official_docs_url(""), None);
        assert_eq!(official_docs_url("../evil"), None);
        assert_eq!(official_docs_url("tokio;rm"), None);
        assert_eq!(official_docs_url("tokio latest"), None);
    }

    #[test]
    fn snippet_json_roundtrip() {
        let s = DocSnippet {
            library_id: "tokio".into(),
            title: Some("spawn".into()),
            url: Some("https://example.invalid".into()),
            content: "tokio::spawn(async {});".into(),
            score: Some(0.9),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: DocSnippet = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
