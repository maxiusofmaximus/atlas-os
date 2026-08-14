// Atlas OS — `atlas research` subcommand (RFC 28 Section E).
//
// Three sub-actions gated behind the `firecrawl` Cargo feature:
//
//   * `opencode research scrape <url>`
//   * `opencode research search <query> [--limit N] [--exclude DOMAIN …]`
//   * `opencode research crawl  <url>   [--limit N] [--allow-subdomains]`
//   * `opencode research extract <url> --schema '<json>' [--prompt P]`
//
// Output is JSON line-delimited to stdout so it can be consumed by
// downstream pipes / skills / graphify ingest. When the feature is
// disabled at build time clap reports "no such command" because the
// subcommand isn't registered in `proto::Commands` (gated through
// the `firecrawl` feature flag, exactly like `acp` does for
// `acp-server`).
//
// The CLI never imports `firecrawl::Client` directly — it uses the
// facade (`crate::firecrawl::facade`) and the env-aware
// `FirecrawlClient::from_env()`. Credentials / self-host URL /
// keyless tier are resolved from env vars as documented in
// `firecrawl::client::FirecrawlClient::from_env`.
//
// Networks errors are bubbled up via `FirecrawlFacadeError` to the
// CLI's `anyhow::Result` exit path (verbose `--verbose` adds stacks).

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::firecrawl::client::FirecrawlClient;
use crate::firecrawl::facade;
use crate::firecrawl::FirecrawlFacadeError;

#[derive(Subcommand, Debug)]
pub enum ResearchSub {
    /// Scrape a single URL and print its Markdown content + metadata.
    Scrape(ScrapeArgs),
    /// Search the web and print one JSON line per result.
    Search(SearchArgs),
    /// Crawl a site (up to `--limit` pages) and print one JSON line
    /// per scraped document.
    Crawl(CrawlArgs),
    /// Structured extraction against a single URL using a JSON schema.
    Extract(ExtractArgs),
}

#[derive(Args, Debug)]
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
    let client = FirecrawlClient::from_env().context(
        "firecrawl client build failed; set FIRECRAWL_API_KEY and optionally ATLAS_FIRECRAWL_URL (legacy: ATLAS_FIRECRAWL_URL)",
    )?;
    match cmd.action {
        ResearchSub::Scrape(a) => run_scrape(&client, a).await,
        ResearchSub::Search(a) => run_search(&client, a).await,
        ResearchSub::Crawl(a) => run_crawl(&client, a).await,
        ResearchSub::Extract(a) => run_extract(&client, a).await,
    }
    .map_err(|e| anyhow::anyhow!("research: {e:?}"))
}

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
struct ScrapeOutputJson {
    source_url: Option<String>,
    title: Option<String>,
    description: Option<String>,
    markdown: Option<String>,
    status_code: Option<u16>,
    links: Vec<String>,
}

#[derive(serde::Serialize, Debug)]
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
    fn parse_scrape_subcmd() {
        let cli = TestCli::try_parse_from(["test", "scrape", "https://example.com"]).unwrap();
        match cli.action {
            ResearchSub::Scrape(a) => assert_eq!(a.url, "https://example.com"),
            other => panic!("expected Scrape, got {other:?}"),
        }
    }

    #[test]
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
    fn parse_search_requires_query() {
        let r = TestCli::try_parse_from(["test", "search"]);
        assert!(r.is_err(), "search without query must error");
    }

    #[test]
    fn parse_scrape_missing_url_errors() {
        let r = TestCli::try_parse_from(["test", "scrape"]);
        assert!(r.is_err());
    }
}
