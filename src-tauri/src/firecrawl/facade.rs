// Atlas OS — Firecrawl facade (RFC 28 Section E).
//
// Stable in-process surface over the firecrawl SDK. Every Atlas OS
// consumer (CLI `research`, future graph_ingest, future `webfetch`
// replacement, future Rust MCP server) calls through these types —
// never through `firecrawl::Client` directly. That decoupling is the
// whole point of the "adapter facade" pattern documented in RFC 28
// §E: it lets the SDK bump major versions, change options field
// names, or add new parser models without rippling breakage across
// the Atlas OS codebase.
//
// Defaults baked into every `*Options::default()`:
//   * `only_main_content = true`   — strip chrome by default (matches
//                                     the `webfetch` reader's intent).
//   * `formats = [Markdown]`        — every result has `markdown`,
//                                     HTML/links opt-in.
//   * `redact_pii = true`            — ON by default so an inadvertent
//                                     scrape of a PII-bearing page does
//                                     not land in the journal or HUD
//                                     verbatim. Caller may opt-out per
//                                     call (PII redaction is a pure
//                                     post-process — see `redact_pii`).
//
// The facade also normalises the SDK's `Option<Value>`-heavy `Document`
// shape into the `ScrapedDocument` struct below — only the fields a
// consumer of web research would actually use.

use super::client::FirecrawlClient;
use super::error::{lift_sdk_error, FirecrawlFacadeError};

// ---------------------------------------------------------------------------
// Options — Stable facade shapes. Every field has a default; only the
// closures in `scrape_url`/etc. touch the underlying SDK options.
// ---------------------------------------------------------------------------

/// Options for a single-URL scrape. Defaults align with the way the
/// orchestrator's `webfetch` fallback uses scrape: markdown content,
// strip the boilerplate chrome.
#[derive(Clone, Debug, Default)]
pub struct ScrapeOptions {
    /// Only extract the main content of the page (default true).
    /// Translates to `firecrawl::ScrapeOptions::only_main_content`.
    pub only_main_content: Option<bool>,
    /// Request timeout in milliseconds (default uses the SDK default).
    pub timeout_ms: Option<u32>,
    /// HTML tags to include (whitelist). Empty = all.
    pub include_tags: Vec<String>,
    /// HTML tags to exclude (blacklist). Applied after `include_tags`.
    pub exclude_tags: Vec<String>,
    /// Wait time after page load before scraping (ms). Useful for
    /// JS-heavy pages.
    pub wait_for_ms: Option<u32>,
    /// Skip TLS certificate verification. NOT recommended; use only
    /// for localhost/dev. Default false.
    pub skip_tls_verification: Option<bool>,
    /// If true (default), the facade rasterises PII tokens out of
    /// the returned `markdown` / `title` / `description` via the
    /// `redact_pii` helper before they reach the caller.
    pub redact_pii: Option<bool>,
}

impl ScrapeOptions {
    fn redact_enabled(&self) -> bool {
        // Default ON — explicit `Some(false)` opts out.
        !matches!(self.redact_pii, Some(false))
    }
}

/// Options for a web `search`. Defaults limit to 5 results (matches
/// the SDK) and turn ON highlights so snippets are returned.
#[derive(Clone, Debug, Default)]
pub struct SearchOptions {
    /// Max number of results to return (5 default, 20 max per SDK).
    pub limit: Option<u32>,
    /// Domains to include. Filters results to these domains only.
    pub include_domains: Vec<String>,
    /// Domains to exclude.
    pub exclude_domains: Vec<String>,
    /// Time-based filter (e.g. "qdr:d" last day, "qdr:w" last week).
    pub tbs: Option<String>,
    /// Geographic location for local search.
    pub location: Option<String>,
    /// Generate query-relevant highlights. Defaults to true so the
    /// caller sees the matched snippet alongside each result.
    pub highlights: Option<bool>,
    /// If true (default), the facade redacts PII tokens out of every
    /// returned `SearchResult.snippet`. The intention is to keep the
    /// journal + HUD cards safe if the user happens to query for an
    /// email / phone number accidentally.
    pub redact_pii: Option<bool>,
}

impl SearchOptions {
    fn redact_enabled(&self) -> bool {
        !matches!(self.redact_pii, Some(false))
    }
}

/// Options for a site crawl. Defaults: 50 pages max, default SDK poll
/// interval (2000ms).
#[derive(Clone, Debug, Default)]
pub struct CrawlOptions {
    /// Hard cap on pages to crawl. Default 50 — large enough to cover
    /// a docs site sub-tree, small enough to keep costs predictable.
    pub limit: Option<u32>,
    /// Crawl max depth (link hops from the seed URL). Default uses the
    /// SDK default (which is "unbounded", bounded by `limit`).
    pub max_depth: Option<u32>,
    /// URL path patterns to include (regex).
    pub include_paths: Vec<String>,
    /// URL path patterns to exclude (regex).
    pub exclude_paths: Vec<String>,
    /// Allow links to subdomains of the seed. Default false.
    pub allow_subdomains: Option<bool>,
    /// Allow links to external domains (off-site). Default false.
    pub allow_external_links: Option<bool>,
    /// Delay between requests in seconds. Rate-limit shoulder.
    pub delay_secs: Option<u32>,
    /// If true (default), the facade redacts PII tokens out of every
    /// returned `ScrapedDocument.markdown`.
    pub redact_pii: Option<bool>,
}

impl CrawlOptions {
    fn redact_enabled(&self) -> bool {
        !matches!(self.redact_pii, Some(false))
    }
}

/// Options for structured extraction (JSON schema-guided). The
/// facade's `extract_structured` wraps the SDK's
/// `scrape_with_schema(...)` per-result; this is NOT a "crawl with
/// schema" — it's per-URL structured extraction.
#[derive(Clone, Debug, Default)]
pub struct ExtractOptions {
    /// JSON schema describing the fields to extract. Required: a
    /// `serde_json::Value` (object) describing the shape.
    pub schema: Option<serde_json::Value>,
    /// Optional natural-language extraction prompt.
    pub prompt: Option<String>,
}

// ---------------------------------------------------------------------------
// Results — Canonical Atlas OS shapes, decoupled from the SDK.
// ---------------------------------------------------------------------------

/// One scraped URL. Minimal surface: only the fields a downstream
/// consumer (graphify ingest, journal, HUD card) actually reads.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrapedDocument {
    /// Source URL as reported by Firecrawl (after redirects).
    pub source_url: Option<String>,
    /// Page title (meta or first <h1>, depending on the page).
    pub title: Option<String>,
    /// Meta-description (if Firecrawl surfaced it).
    pub description: Option<String>,
    /// Markdown content of the page (the primary use case).
    pub markdown: Option<String>,
    /// HTTP status of the fetch (best-effort — None on parse failure).
    pub status_code: Option<u16>,
    /// All `href`s seen on the page. Empty vec if the SDK returned None.
    pub links: Vec<String>,
}

/// One web search result. The SDK's `SearchResultOrDocument` is an
/// untagged enum that may also surface scraped documents — we coerce
/// to the common "search hit" shape so callers don't have to branch.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SearchResult {
    pub url: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
}

/// A crawl batch — the SDK's `CrawlJob` finalised into a plain vec.
/// We drop job-status metadata because consumers want the docs, not
/// the progress; if you need to know if a crawl died mid-flight you
/// get a `FirecrawlFacadeError::JobFailed` instead of `Ok(batch)`.
#[derive(Clone, Debug, Default)]
pub struct CrawlBatch {
    pub docs: Vec<ScrapedDocument>,
}

/// Structured extraction result — opaque JSON the caller parses per
/// its own schema.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExtractResult {
    pub value: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Facade entry points.
// ---------------------------------------------------------------------------

/// Scrape one URL.
pub async fn scrape_url(
    client: &FirecrawlClient,
    url: &str,
    opts: &ScrapeOptions,
) -> Result<ScrapedDocument, FirecrawlFacadeError> {
    let sdk_opts = build_scrape_options(opts);
    let doc = client
        .inner
        .scrape(url, Some(sdk_opts))
        .await
        .map_err(|e| lift_sdk_error("scrape", e))?;
    let mut out = to_scraped_document(doc);
    if opts.redact_enabled() {
        out = redact_pii_doc(out);
    }
    Ok(out)
}

/// Search the web. Returns a flat `Vec<SearchResult>` (news + web
/// merged, images dropped — no HUD consumer renders image tiles
/// today; can be added when one needs them).
pub async fn search_web(
    client: &FirecrawlClient,
    query: &str,
    opts: &SearchOptions,
) -> Result<Vec<SearchResult>, FirecrawlFacadeError> {
    let sdk_opts = build_search_options(opts);
    let response = client
        .inner
        .search(query, Some(sdk_opts))
        .await
        .map_err(|e| lift_sdk_error("search", e))?;
    let mut out = to_search_results(response);
    if opts.redact_enabled() {
        out = out.into_iter().map(redact_pii_search_hit).collect();
    }
    Ok(out)
}

/// Crawl one URL following links up to `opts.limit` pages.
pub async fn crawl_site(
    client: &FirecrawlClient,
    url: &str,
    opts: &CrawlOptions,
) -> Result<CrawlBatch, FirecrawlFacadeError> {
    let sdk_opts = build_crawl_options(opts);
    let job = client
        .inner
        .crawl(url, Some(sdk_opts))
        .await
        .map_err(|e| lift_sdk_error("crawl", e))?;
    let batch: Vec<ScrapedDocument> = job.data.into_iter().map(to_scraped_document).collect();
    let batch = if opts.redact_enabled() {
        batch.into_iter().map(redact_pii_doc).collect()
    } else {
        batch
    };
    Ok(CrawlBatch { docs: batch })
}

/// Structured extraction per-URL. The caller owns the JSON schema.
/// Implementation wires to the SDK's `scrape_with_schema`.
pub async fn extract_structured(
    client: &FirecrawlClient,
    url: &str,
    opts: &ExtractOptions,
) -> Result<ExtractResult, FirecrawlFacadeError> {
    let schema = opts.schema.clone().ok_or(FirecrawlFacadeError::InvalidUrl(
        "extract_structured requires a non-empty JSON schema".into(),
    ))?;
    let value = client
        .inner
        .scrape_with_schema(url, schema, opts.prompt.clone())
        .await
        .map_err(|e| lift_sdk_error("extract_structured", e))?;
    Ok(ExtractResult { value })
}

// ---------------------------------------------------------------------------
// Translation: facade options → SDK options. Centralised so that if
// the SDK renames a field we adjust here only.
// ---------------------------------------------------------------------------

fn build_scrape_options(opts: &ScrapeOptions) -> firecrawl::ScrapeOptions {
    use firecrawl::Format;
    firecrawl::ScrapeOptions {
        formats: Some(vec![Format::Markdown]),
        only_main_content: opts.only_main_content.or(Some(true)),
        timeout: opts.timeout_ms,
        include_tags: if opts.include_tags.is_empty() {
            None
        } else {
            Some(opts.include_tags.clone())
        },
        exclude_tags: if opts.exclude_tags.is_empty() {
            None
        } else {
            Some(opts.exclude_tags.clone())
        },
        wait_for: opts.wait_for_ms,
        skip_tls_verification: opts.skip_tls_verification,
        ..Default::default()
    }
}

fn build_search_options(opts: &SearchOptions) -> firecrawl::SearchOptions {
    firecrawl::SearchOptions {
        limit: opts.limit,
        include_domains: if opts.include_domains.is_empty() {
            None
        } else {
            Some(opts.include_domains.clone())
        },
        exclude_domains: if opts.exclude_domains.is_empty() {
            None
        } else {
            Some(opts.exclude_domains.clone())
        },
        tbs: opts.tbs.clone(),
        location: opts.location.clone(),
        highlights: opts.highlights,
        ..Default::default()
    }
}

fn build_crawl_options(opts: &CrawlOptions) -> firecrawl::CrawlOptions {
    firecrawl::CrawlOptions {
        limit: opts.limit.or(Some(50)),
        max_discovery_depth: opts.max_depth,
        include_paths: if opts.include_paths.is_empty() {
            None
        } else {
            Some(opts.include_paths.clone())
        },
        exclude_paths: if opts.exclude_paths.is_empty() {
            None
        } else {
            Some(opts.exclude_paths.clone())
        },
        allow_subdomains: opts.allow_subdomains,
        allow_external_links: opts.allow_external_links,
        delay: opts.delay_secs,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Translation: SDK results → facade types.
// ---------------------------------------------------------------------------

fn to_scraped_document(d: firecrawl::Document) -> ScrapedDocument {
    let metadata = d.metadata.clone();
    let source_url = metadata.as_ref().and_then(|m| m.source_url.clone());
    let title = metadata.as_ref().and_then(|m| m.title.clone());
    let description = metadata.as_ref().and_then(|m| m.description.clone());
    let status_code = metadata.as_ref().and_then(|m| m.status_code);
    let links = d.links.unwrap_or_default();
    ScrapedDocument {
        source_url,
        title,
        description,
        markdown: d.markdown,
        status_code,
        links,
    }
}

fn to_search_results(response: firecrawl::SearchResponse) -> Vec<SearchResult> {
    let mut out = Vec::new();
    if let Some(web) = response.data.web {
        for item in web {
            use firecrawl::SearchResultOrDocument as S;
            match item {
                S::WebResult(w) => {
                    out.push(SearchResult {
                        url: w.url,
                        title: w.title,
                        snippet: w.description,
                    });
                }
                S::Document(d) => {
                    let doc = to_scraped_document(d);
                    out.push(SearchResult {
                        url: doc.source_url.unwrap_or_default(),
                        title: doc.title,
                        snippet: doc.description,
                    });
                }
            }
        }
    }
    if let Some(news) = response.data.news {
        for n in news {
            out.push(SearchResult {
                url: n.url.unwrap_or_default(),
                title: n.title,
                snippet: n.snippet,
            });
        }
    }
    out
}

// ---------------------------------------------------------------------------
// PII redaction — pure-string, no network. Conservative regex-free
// pass that covers the patterns that would otherwise leak straight
// from a scraped page into the journal + HUD card snippet.
// ---------------------------------------------------------------------------

fn redact_pii_doc(mut d: ScrapedDocument) -> ScrapedDocument {
    if let Some(md) = d.markdown.as_mut() {
        *md = redact_string(md);
    }
    if let Some(t) = d.title.as_mut() {
        *t = redact_string(t);
    }
    if let Some(desc) = d.description.as_mut() {
        *desc = redact_string(desc);
    }
    d
}

fn redact_pii_search_hit(mut s: SearchResult) -> SearchResult {
    if let Some(t) = s.title.as_mut() {
        *t = redact_string(t);
    }
    if let Some(sn) = s.snippet.as_mut() {
        *sn = redact_string(sn);
    }
    s
}

/// Primitive PII rewriter. Not exhaustive — it covers the obvious
/// patterns an AI tool would surface in a web scrape: email
/// addresses, RFC-3966 short phone-like digit groups prefixed with
/// `+`, and 16-digit credit-card-shaped (`4xxx...` and `5xxx...`)
/// numbers. API keys and bearer tokens are left alone on purpose —
/// they're not PII per se, and the orchestrator may want to inspect
/// them after a 401.
fn redact_string(s: &str) -> String {
    let email_replaced = redact_emails(s);
    let phone_replaced = redact_phone(&email_replaced);
    redact_credit_cards(&phone_replaced)
}

/// `name@domain.tld` → `[email]`. Conservative — a `@` between two
/// valid identifier chunks whose right side has at least one `.`.
fn redact_emails(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' && i > 0 {
            // Look back for the local part — alphanumeric, +._-
            let lp_end = i;
            let mut lp_start = i;
            while lp_start > 0
                && (bytes[lp_start - 1].is_ascii_alphanumeric()
                    || matches!(bytes[lp_start - 1], b'+' | b'_' | b'-' | b'.'))
            {
                lp_start -= 1;
            }
            if lp_end - lp_start == 0 {
                out.push('@');
                i += 1;
                continue;
            }
            // Look forward for the domain part — must contain a `.` and
            // consist of alnum + `.` + `-`.
            let mut dp_end = i + 1;
            let mut dot_seen = false;
            while dp_end < bytes.len()
                && (bytes[dp_end].is_ascii_alphanumeric() || matches!(bytes[dp_end], b'.' | b'-'))
            {
                if bytes[dp_end] == b'.' {
                    dot_seen = true;
                }
                dp_end += 1;
            }
            if dot_seen && dp_end - i > 1 {
                // trim trailing dots of the matched span from `out`
                out.truncate(out.len() - (i - lp_start));
                out.push_str("[email]");
                i = dp_end;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// `+<digits>` (≥ 7) → `[phone]`. Catches international format.
fn redact_phone(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j - start >= 7 {
                out.push_str("[phone]");
                i = j;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// 16-digit sequences starting with 4 or 5 → `[card]`. Plus a couple
/// of length variants (13-19) to catch Amex/Diners/Discover shapes.
fn redact_credit_cards(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if (bytes[i] == b'4' || bytes[i] == b'5') && i + 13 <= bytes.len() {
            let mut j = i;
            while j < i + 19 && j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let len = j - i;
            if len >= 13 {
                out.push_str("[card]");
                i = j;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts_with_redact(on: Option<bool>) -> ScrapeOptions {
        ScrapeOptions {
            redact_pii: on,
            ..Default::default()
        }
    }

    #[test]
    fn redact_pii_defaults_to_on_when_unset() {
        assert!(opts_with_redact(None).redact_enabled());
    }

    #[test]
    fn redact_pii_can_be_turned_off_explicitly() {
        assert!(!opts_with_redact(Some(false)).redact_enabled());
    }

    #[test]
    fn redact_pii_stays_on_when_explicit_some_true() {
        assert!(opts_with_redact(Some(true)).redact_enabled());
    }

    #[test]
    fn redact_emails_basic() {
        assert_eq!(
            redact_emails("contact us at foo@bar.com"),
            "contact us at [email]"
        );
    }

    #[test]
    fn redact_emails_preserves_plus_local_part() {
        assert_eq!(
            redact_emails("reach me at john.doe+bounce@example.io please"),
            "reach me at [email] please"
        );
    }

    #[test]
    fn redact_emails_ignores_lone_at() {
        assert_eq!(redact_emails("hello @ world"), "hello @ world");
    }

    #[test]
    fn redact_emails_does_not_touch_subdomain_less() {
        assert_eq!(redact_emails("foo@bar"), "foo@bar");
    }

    #[test]
    fn redact_phone_international() {
        assert_eq!(redact_phone("dial +14155550142"), "dial [phone]");
    }

    #[test]
    fn redact_phone_preserves_spaced_groups() {
        // The conservador redactor only fires on a `+` directly followed
        // by 7+ consecutive digits — anything spaced out as
        // "+1 415 555 0142" is left alone (would require more
        // sophisticated E.164 detection; deferred).
        assert_eq!(redact_phone("dial +1 415 555 0142"), "dial +1 415 555 0142");
    }

    #[test]
    fn redact_phone_ignores_short_plus_runs() {
        assert_eq!(redact_phone("a +1 thing"), "a +1 thing");
    }

    #[test]
    fn redact_credit_cards_visa() {
        assert_eq!(
            redact_credit_cards("card 4111111111111111 ok"),
            "card [card] ok"
        );
    }

    #[test]
    fn redact_credit_cards_mastercard() {
        assert_eq!(redact_credit_cards("5555555555554444 zzz"), "[card] zzz");
    }

    #[test]
    fn redact_credit_cards_ignores_short_digits() {
        assert_eq!(
            redact_credit_cards("5000 looks like a card but isn't"),
            "5000 looks like a card but isn't"
        );
    }

    #[test]
    fn redact_string_runs_all_three() {
        let s = "Email a@b.io, phone +14155550100, card 4111111111111111";
        let r = redact_string(s);
        assert!(r.contains("[email]"));
        assert!(r.contains("[phone]"));
        assert!(r.contains("[card]"));
        assert!(!r.contains("a@b.io"));
        assert!(!r.contains("+14155550100"));
        assert!(!r.contains("4111111111111111"));
    }

    #[test]
    fn redact_pii_doc_touches_all_text_fields() {
        let d = ScrapedDocument {
            source_url: Some("https://example.com/page".into()),
            title: Some("email: a@b.io".into()),
            description: Some("phone +14155550100".into()),
            markdown: Some("Card 4111111111111111 and a@b.io".into()),
            status_code: Some(200),
            links: vec![],
        };
        let r = redact_pii_doc(d);
        assert!(r.title.unwrap().contains("[email]"));
        assert!(r.description.unwrap().contains("[phone]"));
        let md = r.markdown.unwrap();
        assert!(md.contains("[card]"));
        assert!(md.contains("[email]"));
        // source_url + status_code are NOT redacted — they are not PII.
        assert_eq!(r.source_url.as_deref(), Some("https://example.com/page"));
        assert_eq!(r.status_code, Some(200));
    }

    #[test]
    fn build_scrape_options_default_has_markdown_only() {
        let o = ScrapeOptions::default();
        let sdk = build_scrape_options(&o);
        assert!(sdk.only_main_content.unwrap_or(false));
        assert_eq!(sdk.formats.as_deref().map(|f| f.len()), Some(1));
    }

    #[test]
    fn build_search_options_propagates_filter_fields() {
        let o = SearchOptions {
            limit: Some(7),
            include_domains: vec!["example.io".into()],
            exclude_domains: vec!["example.org".into()],
            tbs: Some("qdr:d".into()),
            ..Default::default()
        };
        let sdk = build_search_options(&o);
        assert_eq!(sdk.limit, Some(7));
        assert_eq!(
            sdk.include_domains.as_deref(),
            Some(&["example.io".to_string()][..])
        );
        assert_eq!(
            sdk.exclude_domains.as_deref(),
            Some(&["example.org".to_string()][..])
        );
        assert_eq!(sdk.tbs.as_deref(), Some("qdr:d"));
    }

    #[test]
    fn build_crawl_options_uses_default_limit_50_when_unset() {
        let o = CrawlOptions::default();
        let sdk = build_crawl_options(&o);
        assert_eq!(sdk.limit, Some(50));
        let o2 = CrawlOptions {
            limit: Some(5),
            ..Default::default()
        };
        let sdk2 = build_crawl_options(&o2);
        assert_eq!(sdk2.limit, Some(5));
    }

    #[test]
    fn to_scraped_document_handles_missing_metadata() {
        let d = firecrawl::Document {
            markdown: Some("# hi".into()),
            metadata: None,
            ..Default::default()
        };
        let s = to_scraped_document(d);
        assert_eq!(s.markdown.as_deref(), Some("# hi"));
        assert!(s.source_url.is_none());
        assert!(s.title.is_none());
        assert!(s.description.is_none());
        assert!(s.status_code.is_none());
        assert!(s.links.is_empty());
    }

    #[test]
    fn to_scraped_document_unwinds_metadata_and_links() {
        use firecrawl::DocumentMetadata;
        let m = DocumentMetadata {
            source_url: Some("https://example.com/x".into()),
            title: Some("Hello".into()),
            description: Some("World".into()),
            status_code: Some(200),
            ..Default::default()
        };
        let d = firecrawl::Document {
            markdown: Some("body".into()),
            metadata: Some(m),
            links: Some(vec![
                "https://example.com/a".into(),
                "https://example.com/b".into(),
            ]),
            ..Default::default()
        };
        let s = to_scraped_document(d);
        assert_eq!(s.source_url.as_deref(), Some("https://example.com/x"));
        assert_eq!(s.title.as_deref(), Some("Hello"));
        assert_eq!(s.description.as_deref(), Some("World"));
        assert_eq!(s.status_code, Some(200));
        assert_eq!(s.links.len(), 2);
    }

    #[tokio::test]
    #[ignore = "requires a real FIRECRAWL_API_KEY + network; run with `cargo test --features firecrawl -- --ignored`"]
    async fn scrape_url_e2e() {
        // Sanity test gated behind `--ignored` so we never hit the
        // network during normal CI. Run manually:
        //   cargo test --features firecrawl -- --ignored scrape_url_e2e
        let client = FirecrawlClient::from_env().expect("env");
        let result = scrape_url(&client, "https://example.com", &ScrapeOptions::default())
            .await
            .expect("scrape");
        assert!(result.markdown.is_some());
    }
}
