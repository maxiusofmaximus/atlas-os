// Atlas OS — `atlas research` subcommand (RFC 28 Section E, RFC 10 §6).
//
// Two surfaces share one subcommand:
//
//   * `opencode research docs <library> "<question>" [--limit N]`
//     (Phase 3 sub-fase 3.1, always compiled — the docs gateway needs
//     no new crate: Context7Max CLI/API externa plus Context7 MCP and
//     official-docs fallbacks, RFC 25 §11).
//   * `scrape` / `search` / `crawl` / `extract`, gated behind the
//     `firecrawl` Cargo feature (RFC 28 §E):
//
//   * `opencode research scrape <url>`
//   * `opencode research search <query> [--limit N] [--exclude DOMAIN …]`
//   * `opencode research crawl  <url>   [--limit N] [--allow-subdomains]`
//   * `opencode research extract <url> --schema '<json>' [--prompt P]`
//
// Output is JSON line-delimited to stdout so it can be consumed by
// downstream pipes / skills / graphify ingest. When the `firecrawl`
// feature is disabled at build time clap reports "no such command" for
// the gated verbs only — `docs` stays available because the gateway it
// uses is dependency-free.
//
// The CLI never imports `firecrawl::Client` directly — it uses the
// facade (`crate::firecrawl::facade`) and the env-aware
// `FirecrawlClient::from_env()`. Credentials / self-host URL /
// keyless tier are resolved from env vars as documented in
// `firecrawl::client::FirecrawlClient::from_env`. Likewise the docs
// verb only touches `crate::research::docs_gateway::DocsGateway`.
//
// Networks errors are bubbled up via `FirecrawlFacadeError` to the
// CLI's `anyhow::Result` exit path (verbose `--verbose` adds stacks).

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

#[cfg(feature = "firecrawl")]
use crate::firecrawl::client::FirecrawlClient;
#[cfg(feature = "firecrawl")]
use crate::firecrawl::facade;
#[cfg(feature = "firecrawl")]
use crate::firecrawl::FirecrawlFacadeError;

use crate::research::docs_gateway::DocsGateway;

#[derive(Subcommand, Debug)]
pub enum ResearchSub {
    /// Query live library docs via the docs gateway (Context7Max →
    /// Context7 MCP shape → official docs) and print one JSON line
    /// per snippet.
    Docs(DocsArgs),
    /// Scrape a single URL and print its Markdown content + metadata.
    #[cfg(feature = "firecrawl")]
    Scrape(ScrapeArgs),
    /// Search the web and print one JSON line per result.
    #[cfg(feature = "firecrawl")]
    Search(SearchArgs),
    /// Crawl a site (up to `--limit` pages) and print one JSON line
    /// per scraped document.
    #[cfg(feature = "firecrawl")]
    Crawl(CrawlArgs),
    /// Structured extraction against a single URL using a JSON schema.
    #[cfg(feature = "firecrawl")]
    Extract(ExtractArgs),
}

#[derive(Args, Debug)]
pub struct DocsArgs {
    /// Library id as indexed by Context7Max (e.g. `tokio`, `/tokio-rs/tokio`).
    pub library: String,
    /// Natural-language question answered with verbatim code examples.
    pub question: String,
    /// Max number of snippets to print (docs-gateway default 5).
    #[arg(short = 'n', long, default_value_t = 5)]
    pub limit: u32,
}

#[derive(Args, Debug)]
#[cfg(feature = "firecrawl")]
pub struct ScrapeArgs {
    /// URL to scrape. `http(s)://` is required by the SDK.
    pub url: String,
    /// Strip boilerplate chrome (defaults to ON inside the facade).
    /// Pass `--no-main-content` to keep the full DOM output.
    #[arg(long)]
    pub no_main_content: bool,
    /// Disable PII redaction (default ON in the facade). Use only
    /// for trusted sources and never for live user-supplied URLs.
    #[arg(long)]
    pub no_redact: bool,
}

#[derive(Args, Debug)]
#[cfg(feature = "firecrawl")]
pub struct SearchArgs {
    /// Query string. May be quoted for shell-escaping reasons.
    pub query: String,
    /// Max number of results (5 default; 20 max per SDK).
    #[arg(short = 'n', long, default_value_t = 5)]
    pub limit: u32,
    /// Domains to include (filter-in). Comma-separated.
    #[arg(long, value_delimiter = ',')]
    pub include_domains: Vec<String>,
    /// Domains to exclude. Comma-separated.
    #[arg(long, value_delimiter = ',')]
    pub exclude_domains: Vec<String>,
    /// Time-based filter (e.g. "qdr:d" last day, "qdr:w" last week).
    #[arg(long)]
    pub tbs: Option<String>,
    /// Disable PII redaction in snippets.
    #[arg(long)]
    pub no_redact: bool,
}

#[derive(Args, Debug)]
#[cfg(feature = "firecrawl")]
pub struct CrawlArgs {
    /// Seed URL.
    pub url: String,
    /// Hard cap on pages (50 default inside facade).
    #[arg(short = 'n', long)]
    pub limit: Option<u32>,
    /// Max link-hops to follow.
    #[arg(long)]
    pub max_depth: Option<u32>,
    /// Allow following subdomains of the seed.
    #[arg(long)]
    pub allow_subdomains: bool,
    /// Allow following links to external (off-site) domains.
    #[arg(long)]
    pub allow_external: bool,
    /// URL path patterns to include (regex). Comma-separated.
    #[arg(long, value_delimiter = ',')]
    pub include_paths: Vec<String>,
    /// URL path patterns to exclude (regex). Comma-separated.
    #[arg(long, value_delimiter = ',')]
    pub exclude_paths: Vec<String>,
    /// Disable PII redaction in the resulting documents.
    #[arg(long)]
    pub no_redact: bool,
}

#[derive(Args, Debug)]
#[cfg(feature = "firecrawl")]
pub struct ExtractArgs {
    /// URL to scrape + extract from.
    pub url: String,
    /// JSON schema describing the extraction. Pass as a single
    /// string (shell-encoded). Example:
    ///   `--schema '{"type":"object","properties":{"price":{"type":"number"}}}'`
    #[arg(long)]
    pub schema: String,
    /// Optional natural-language prompt guiding the extraction.
    #[arg(long)]
    pub prompt: Option<String>,
}

#[derive(Args, Debug)]
pub struct ResearchCmd {
    #[command(subcommand)]
    pub action: ResearchSub,
}

pub async fn run(cmd: ResearchCmd, _profile: &str) -> Result<()> {
    match cmd.action {
        ResearchSub::Docs(a) => run_docs(a).await,
        #[cfg(feature = "firecrawl")]
        ResearchSub::Scrape(a) => {
            let client = FirecrawlClient::from_env().context(
                "firecrawl client build failed; set FIRECRAWL_API_KEY and optionally ATLAS_FIRECRAWL_URL (legacy: ATLAS_FIRECRAWL_URL)",
            )?;
            run_scrape(&client, a)
                .await
                .map_err(|e| anyhow::anyhow!("research: {e:?}"))
        }
        #[cfg(feature = "firecrawl")]
        ResearchSub::Search(a) => {
            let client = FirecrawlClient::from_env().context(
                "firecrawl client build failed; set FIRECRAWL_API_KEY and optionally ATLAS_FIRECRAWL_URL (legacy: ATLAS_FIRECRAWL_URL)",
            )?;
            run_search(&client, a)
                .await
                .map_err(|e| anyhow::anyhow!("research: {e:?}"))
        }
        #[cfg(feature = "firecrawl")]
        ResearchSub::Crawl(a) => {
            let client = FirecrawlClient::from_env().context(
                "firecrawl client build failed; set FIRECRAWL_API_KEY and optionally ATLAS_FIRECRAWL_URL (legacy: ATLAS_FIRECRAWL_URL)",
            )?;
            run_crawl(&client, a)
                .await
                .map_err(|e| anyhow::anyhow!("research: {e:?}"))
        }
        #[cfg(feature = "firecrawl")]
        ResearchSub::Extract(a) => {
            let client = FirecrawlClient::from_env().context(
                "firecrawl client build failed; set FIRECRAWL_API_KEY and optionally ATLAS_FIRECRAWL_URL (legacy: ATLAS_FIRECRAWL_URL)",
            )?;
            run_extract(&client, a)
                .await
                .map_err(|e| anyhow::anyhow!("research: {e:?}"))
        }
    }
}

async fn run_docs(a: DocsArgs) -> Result<()> {
    let gateway = DocsGateway::from_env();
    let backend = format!("{:?}", gateway.backend());
    let snippets = gateway
        .query_docs(&a.library, &a.question)
        .await
        .map_err(|e| anyhow::anyhow!("research docs: {e}"))?;
    let limit = a.limit as usize;
    let mut printed = 0usize;
    for s in snippets.into_iter().take(limit) {
        let json = serde_json::to_string(&s).context("research docs: snippet serialisation")?;
        println!("{json}");
        printed += 1;
    }
    if printed == 0 {
        eprintln!(
            "research docs: no snippets for `{}` (backend {backend}); set ATLAS_CTX7MAX_URL or install ctx7max, or answer via Context7 MCP (AGENTS.md §5)",
            a.library
        );
    }
    Ok(())
}

#[cfg(feature = "firecrawl")]
async fn run_scrape(client: &FirecrawlClient, a: ScrapeArgs) -> Result<(), FirecrawlFacadeError> {
    let opts = facade::ScrapeOptions {
        only_main_content: if a.no_main_content { Some(false) } else { None },
        redact_pii: if a.no_redact { Some(false) } else { None },
        ..Default::default()
    };
    let doc = facade::scrape_url(client, &a.url, &opts).await?;
    let json = serde_json::to_string(&ScrapeOutputJson {
        source_url: doc.source_url,
        title: doc.title,
        description: doc.description,
        markdown: doc.markdown,
        status_code: doc.status_code,
        links: doc.links,
    })
    .map_err(FirecrawlFacadeError::Parse)?;
    println!("{json}");
    Ok(())
}

#[cfg(feature = "firecrawl")]
async fn run_search(client: &FirecrawlClient, a: SearchArgs) -> Result<(), FirecrawlFacadeError> {
    let opts = facade::SearchOptions {
        limit: Some(a.limit),
        include_domains: a.include_domains,
        exclude_domains: a.exclude_domains,
        tbs: a.tbs,
        redact_pii: if a.no_redact { Some(false) } else { None },
        ..Default::default()
    };
    let hits = facade::search_web(client, &a.query, &opts).await?;
    for h in hits {
        let json = serde_json::to_string(&SearchHitJson {
            url: h.url,
            title: h.title,
            snippet: h.snippet,
        })
        .map_err(FirecrawlFacadeError::Parse)?;
        println!("{json}");
    }
    Ok(())
}

#[cfg(feature = "firecrawl")]
async fn run_crawl(client: &FirecrawlClient, a: CrawlArgs) -> Result<(), FirecrawlFacadeError> {
    let opts = facade::CrawlOptions {
        limit: a.limit,
        max_depth: a.max_depth,
        include_paths: a.include_paths,
        exclude_paths: a.exclude_paths,
        allow_subdomains: a.allow_subdomains.then_some(true),
        allow_external_links: a.allow_external.then_some(true),
        redact_pii: if a.no_redact { Some(false) } else { None },
        ..Default::default()
    };
    let batch = facade::crawl_site(client, &a.url, &opts).await?;
    for d in batch.docs {
        let json = serde_json::to_string(&ScrapeOutputJson {
            source_url: d.source_url,
            title: d.title,
            description: d.description,
            markdown: d.markdown,
            status_code: d.status_code,
            links: d.links,
        })
        .map_err(FirecrawlFacadeError::Parse)?;
        println!("{json}");
    }
    Ok(())
}

#[cfg(feature = "firecrawl")]
async fn run_extract(client: &FirecrawlClient, a: ExtractArgs) -> Result<(), FirecrawlFacadeError> {
    let schema: serde_json::Value =
        serde_json::from_str(&a.schema).map_err(FirecrawlFacadeError::Parse)?;
    let opts = facade::ExtractOptions {
        schema: Some(schema),
        prompt: a.prompt,
    };
    let r = facade::extract_structured(client, &a.url, &opts).await?;
    let json = serde_json::to_string(&r.value).map_err(FirecrawlFacadeError::Parse)?;
    println!("{json}");
    Ok(())
}

// ---------------------------------------------------------------------------
// JSON-DTO shapes printed to stdout. Decoupled from the facade types
// because we want a STABLE wire shape for downstream consumers (the
// `links` field stays an array even if the facade drops it, etc.).
// ---------------------------------------------------------------------------

#[derive(serde::Serialize, Debug)]
#[cfg(feature = "firecrawl")]
struct ScrapeOutputJson {
    source_url: Option<String>,
    title: Option<String>,
    description: Option<String>,
    markdown: Option<String>,
    status_code: Option<u16>,
    links: Vec<String>,
}

#[derive(serde::Serialize, Debug)]
#[cfg(feature = "firecrawl")]
struct SearchHitJson {
    url: String,
    title: Option<String>,
    snippet: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct TestCli {
        #[command(subcommand)]
        action: ResearchSub,
    }

    #[test]
    fn parse_docs_subcmd() {
        let cli = TestCli::try_parse_from(["test", "docs", "tokio", "how to spawn tasks"]).unwrap();
        match cli.action {
            ResearchSub::Docs(a) => {
                assert_eq!(a.library, "tokio");
                assert_eq!(a.question, "how to spawn tasks");
                assert_eq!(a.limit, 5);
            }
            #[cfg(feature = "firecrawl")]
            _ => panic!("expected Docs"),
        }
    }

    #[test]
    fn parse_docs_with_limit() {
        let cli = TestCli::try_parse_from([
            "test",
            "docs",
            "tokio",
            "how to spawn tasks",
            "--limit",
            "3",
        ])
        .unwrap();
        match cli.action {
            ResearchSub::Docs(a) => assert_eq!(a.limit, 3),
            #[cfg(feature = "firecrawl")]
            _ => panic!("expected Docs"),
        }
    }

    #[test]
    fn parse_docs_requires_question() {
        let r = TestCli::try_parse_from(["test", "docs", "tokio"]);
        assert!(r.is_err(), "docs without question must error");
    }

    #[test]
    fn parse_docs_requires_library() {
        let r = TestCli::try_parse_from(["test", "docs"]);
        assert!(r.is_err(), "docs without library must error");
    }

    #[test]
    #[cfg(feature = "firecrawl")]
    fn parse_scrape_subcmd() {
        let cli = TestCli::try_parse_from(["test", "scrape", "https://example.com"]).unwrap();
        match cli.action {
            ResearchSub::Scrape(a) => assert_eq!(a.url, "https://example.com"),
            other => panic!("expected Scrape, got {other:?}"),
        }
    }

    #[test]
    #[cfg(feature = "firecrawl")]
    fn parse_scrape_with_flags() {
        let cli = TestCli::try_parse_from([
            "test",
            "scrape",
            "https://example.com",
            "--no-main-content",
            "--no-redact",
        ])
        .unwrap();
        match cli.action {
            ResearchSub::Scrape(a) => {
                assert_eq!(a.url, "https://example.com");
                assert!(a.no_main_content);
                assert!(a.no_redact);
            }
            _ => panic!("expected Scrape"),
        }
    }

    #[test]
    #[cfg(feature = "firecrawl")]
    fn parse_search_subcmd_with_filters() {
        let cli = TestCli::try_parse_from([
            "test",
            "search",
            "rust async patterns",
            "--limit",
            "3",
            "--include-domains",
            "example.io,example.com",
            "--exclude-domains",
            "example.org",
            "--tbs",
            "qdr:w",
        ])
        .unwrap();
        match cli.action {
            ResearchSub::Search(a) => {
                assert_eq!(a.query, "rust async patterns");
                assert_eq!(a.limit, 3);
                assert_eq!(a.include_domains.len(), 2);
                assert!(a.include_domains.contains(&"example.io".to_string()));
                assert_eq!(a.exclude_domains, vec!["example.org".to_string()]);
                assert_eq!(a.tbs.as_deref(), Some("qdr:w"));
            }
            _ => panic!("expected Search"),
        }
    }

    #[test]
    #[cfg(feature = "firecrawl")]
    fn parse_crawl_subcmd_with_subdomain_flags() {
        let cli = TestCli::try_parse_from([
            "test",
            "crawl",
            "https://example.com",
            "--limit",
            "10",
            "--max-depth",
            "2",
            "--allow-subdomains",
            "--allow-external",
        ])
        .unwrap();
        match cli.action {
            ResearchSub::Crawl(a) => {
                assert_eq!(a.url, "https://example.com");
                assert_eq!(a.limit, Some(10));
                assert_eq!(a.max_depth, Some(2));
                assert!(a.allow_subdomains);
                assert!(a.allow_external);
            }
            _ => panic!("expected Crawl"),
        }
    }

    #[test]
    #[cfg(feature = "firecrawl")]
    fn parse_extract_subcmd() {
        let schema = r#"{"type":"object","properties":{"price":{"type":"number"}}}"#;
        let cli = TestCli::try_parse_from([
            "test",
            "extract",
            "https://example.com/product",
            "--schema",
            schema,
            "--prompt",
            "Extract the product price",
        ])
        .unwrap();
        match cli.action {
            ResearchSub::Extract(a) => {
                assert_eq!(a.url, "https://example.com/product");
                assert_eq!(a.schema, schema);
                assert_eq!(a.prompt.as_deref(), Some("Extract the product price"));
            }
            _ => panic!("expected Extract"),
        }
    }

    #[test]
    #[cfg(feature = "firecrawl")]
    fn parse_search_requires_query() {
        let r = TestCli::try_parse_from(["test", "search"]);
        assert!(r.is_err(), "search without query must error");
    }

    #[test]
    #[cfg(feature = "firecrawl")]
    fn parse_scrape_missing_url_errors() {
        let r = TestCli::try_parse_from(["test", "scrape"]);
        assert!(r.is_err());
    }
}
