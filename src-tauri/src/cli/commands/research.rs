// Atlas OS — `atlas research` subcommand (RFC 28 Section E, RFC 10 §6).
//
// Four surfaces share one subcommand:
//   * `opencode research docs <library> "<question>" [--limit N]`
//     (Phase 3 sub-fase 3.1, always compiled — the docs gateway needs
//     no new crate: Context7Max CLI/API externa plus Context7 MCP and
//     official-docs fallbacks, RFC 25 §11).
//   * `opencode research query "<pregunta>" [--source URL …]`
//     `[--hands-on URL …] [--library ID] [--gh-repo OWNER/REPO]`
//     (Phase 3 sub-fase 3.3, always compiled — the four dimension
//     scorers plus the weak-model pre-filter and the fail-safe are
//     dependency-free; live collectors degrade to "no data").
//     Prints the canonical RFC 10 §7 YAML report to stdout. Since
//     sub-fase 3.4 every run also mints a `journal_ref` (`jr-…`,
//     auditable via `journal tail`) and renders the `proposal:` lines
//     from the Opción A/B/C `build_branches` (persisted hands-on notes
//     included, expert backing cited).
//   * `opencode research note --title T --decision D [--project P]`
//     `[--outcome O] [--confidence C] [--tags "a, b"] [--signature S]`
//     (Phase 3 sub-fase 3.4, always compiled — RFC 10 §3 expert
//     evidence persisted to M26 `research_notes`, first write wins).
//     Prints the saved note as one JSON line.
//   * `opencode research branches --run-id ID [--project-hint H]`
//     (Phase 3 sub-fase 3.4, always compiled — RFC 10 §4 Opción A/B/C
//     rebuilt from the run's persisted sources + consensus + notes).
//     Prints one JSON object with the branches and proposal lines.
//   * `opencode research feasibility "<topic>" [--domain D,…]`
//     (Phase 3 sub-fase 3.5, always compiled — RFC 10 §11
//     `probe_feasibility`: the single-domain probe shape fanned out over
//     a comma-separated `--domain` list, §11.3 gate, 12-day M27 cache,
//     §10 fail-safe. Prints the §11.8 human-readable block to stdout.)
//   * `opencode research ingest <file> [--run-id ID] [--raw]`
//     (Phase 3 sub-fase 3.2, gated behind the `doc-ingest` Cargo
//     feature, default off — dependency-free minimal parser plus
//     opt-in external `pandoc`, never bundled).
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
// uses is dependency-free. The same holds for `ingest` behind the
// `doc-ingest` feature.
//
// The CLI never imports `firecrawl::Client` directly — it uses the
// facade (`crate::firecrawl::facade`) and the env-aware
// `FirecrawlClient::from_env()`. Credentials / self-host URL /
// keyless tier are resolved from env vars as documented in
// `firecrawl::client::FirecrawlClient::from_env`. Likewise the docs
// verb only touches `crate::research::docs_gateway::DocsGateway` and
// the ingest verb only touches `crate::research::ingest::ingest_file`.
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
    /// Run collective engineering intelligence over a question
    /// (weak-model pre-filter → four dimension scorers → weighted
    /// confidence → fail-safe) and print the RFC 10 §7 YAML report.
    /// Evidence comes from best-effort live collectors (`gh` CLI,
    /// arXiv API, docs gateway) plus explicit `--source` / `--hands-on`
    /// URLs. The run is persisted to the Journal when it opens.
    Query(QueryArgs),
    /// Attach a hands-on expert case (RFC 10 §3, sub-fase 3.4): a closed
    /// real-world decision that weighs ×1.5 against literature in every
    /// future branch build. Persisted to M26 `research_notes`.
    Note(NoteArgs),
    /// Rebuild the Opción A/B/C application branches (RFC 10 §4,
    /// sub-fase 3.4) for a persisted run from its sources + consensus +
    /// hands-on notes. Prints one JSON object to stdout.
    Branches(BranchesArgs),
    /// Verify edge-case capability before the system believes "sí se
    /// puede" (RFC 10 §11, sub-fase 3.5): probe registries per domain,
    /// apply the §11.3 gate, reuse the 12-day M27 cache on repeat
    /// prompts, and fail safe to `needing_human` (RFC 10 §10). Prints
    /// the §11.8 human-readable block to stdout.
    Feasibility(FeasibilityArgs),
    /// Ingest a local document (md/txt/csv/pdf natively; office/epub/
    /// rtf via opt-in external `pandoc`) into Markdown and print one
    /// JSON line. With `--run-id`, also appends a `document` row to
    /// `research_sources` for that run (auto-creating the run).
    #[cfg(feature = "doc-ingest")]
    Ingest(IngestArgs),
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
pub struct QueryArgs {
    /// Natural-language research question (RFC 10 §1 trigger).
    pub question: String,
    /// Extra evidence URLs (repeatable). Kind is auto-classified by
    /// host; weight 1.0.
    #[arg(long = "source")]
    pub sources: Vec<String>,
    /// Hands-on expert evidence URLs (repeatable, RFC 10 §3). They
    /// weigh ×1.5 in the combined confidence.
    #[arg(long = "hands-on")]
    pub hands_on: Vec<String>,
    /// Library id for the official-docs collector (3.1 gateway).
    #[arg(long)]
    pub library: Option<String>,
    /// `OWNER/REPO` scoping the `gh` Issues collector.
    #[arg(long)]
    pub gh_repo: Option<String>,
    /// Max live hits per collector (default 5).
    #[arg(short = 'n', long, default_value_t = 5)]
    pub limit: u8,
    /// Research run id. Defaults to `rr-YYYYMMDD-<8hex>`.
    #[arg(long)]
    pub run_id: Option<String>,
    /// Fail-safe floor override (default `FAIL_SAFE_CONFIDENCE` 0.6,
    /// RFC 10 §10). Confidence below this marks the run
    /// `needing_human`.
    #[arg(long)]
    pub min_confidence: Option<f64>,
}

#[derive(Args, Debug)]
pub struct NoteArgs {
    /// Short case title, e.g. "Lo hice así en producción" (RFC 10 §3).
    #[arg(long)]
    pub title: String,
    /// The decision taken in that closed case, e.g. "Event Sourcing + Kafka".
    #[arg(long)]
    pub decision: String,
    /// Project where it happened, e.g. "fintech X".
    #[arg(long)]
    pub project: Option<String>,
    /// What happened, e.g. "exitoso pero costoso en ops".
    #[arg(long)]
    pub outcome: Option<String>,
    /// Self-reported confidence 0.0..=1.0 (default 0.8).
    #[arg(long, default_value_t = 0.8)]
    pub confidence: f64,
    /// Comma-separated tags, e.g. "architecture, event-sourcing".
    #[arg(long, default_value = "")]
    pub tags: String,
    /// Who signs the note (default `operator`).
    #[arg(long, default_value = "operator")]
    pub signature: String,
    /// Note id. Defaults to `rn-YYYYMMDD-<8hex>`.
    #[arg(long)]
    pub id: Option<String>,
}

#[derive(Args, Debug)]
pub struct BranchesArgs {
    /// Research run id whose persisted sources + consensus feed the
    /// Opción A/B/C rebuild (RFC 10 §4).
    #[arg(long)]
    pub run_id: String,
    /// Current-architecture hint folded into each branch `fits_stack`
    /// (Context Engine Project Map summary when available).
    #[arg(long)]
    pub project_hint: Option<String>,
}

#[derive(Args, Debug)]
pub struct FeasibilityArgs {
    /// Edge-case capability to verify, e.g. "casas con impresora 3D"
    /// (RFC 10 §11.1 `topic`).
    pub topic: String,
    /// Comma-separated domains: software,hardware,academic,vendor.
    /// Empty means all four (RFC 10 §11.2).
    #[arg(long, default_value = "")]
    pub domain: String,
    /// Gate floor: distinct sources required (default 3, RFC 10 §11.3).
    #[arg(long, default_value_t = 3)]
    pub min_sources: u8,
    /// Fail the probe when no artifact evidence was fetched.
    #[arg(long)]
    pub require_artifact_evidence: bool,
    /// Max live hits per domain collector (default 5).
    #[arg(short = 'n', long, default_value_t = 5)]
    pub limit: u8,
    /// Probe id. Defaults to `fp-YYYYMMDD-<8hex>`.
    #[arg(long)]
    pub probe_id: Option<String>,
    /// Skip the 12-day M27 cache and probe live registries.
    #[arg(long)]
    pub no_cache: bool,
}

#[derive(Args, Debug)]
#[cfg(feature = "doc-ingest")]
pub struct IngestArgs {
    /// Local file to ingest (md/markdown/txt/csv/pdf natively;
    /// docx/pptx/xlsx/odt/ods/odp/epub/rtf/doc/xls/ppt via `pandoc`).
    pub file: std::path::PathBuf,
    /// Research run id to attach the `document` source row to. When the
    /// run does not exist yet it is created (`running`) so a single
    /// invocation is demo-complete. Omit for stdout-only (no journal).
    #[arg(long)]
    pub run_id: Option<String>,
    /// Print raw Markdown instead of the JSON envelope.
    #[arg(long)]
    pub raw: bool,
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

pub async fn run(cmd: ResearchCmd, profile: &str) -> Result<()> {
    match cmd.action {
        ResearchSub::Docs(a) => run_docs(a).await,
        ResearchSub::Query(a) => run_query(a, profile).await,
        ResearchSub::Note(a) => run_note(a, profile).await,
        ResearchSub::Branches(a) => run_branches(a, profile).await,
        ResearchSub::Feasibility(a) => run_feasibility(a, profile).await,
        #[cfg(feature = "doc-ingest")]
        ResearchSub::Ingest(a) => run_ingest(a, profile).await,
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

async fn run_query(a: QueryArgs, profile: &str) -> Result<()> {
    use crate::research::{
        branch_proposal_lines, build_branches, build_report, journal_ref_for_run,
        prefilter_sources, score_all, top_reference, ConsensusDimension, DimensionOutcome,
        GatherOptions, ReportSections, ResearchRunStatus, FAIL_SAFE_CONFIDENCE,
    };

    let question = a.question.trim();
    if question.is_empty() {
        anyhow::bail!("research query: question must not be empty");
    }
    let floor = a.min_confidence.unwrap_or(FAIL_SAFE_CONFIDENCE);
    if !floor.is_finite() || !(0.0..=1.0).contains(&floor) {
        anyhow::bail!("research query: --min-confidence must be within 0.0..=1.0");
    }
    let run_id = match a.run_id.as_deref().map(str::trim) {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => {
            let date = chrono::Utc::now().format("%Y%m%d");
            let short = uuid::Uuid::new_v4().simple().to_string();
            format!("rr-{date}-{}", &short[..8])
        }
    };

    let pool = crate::research::collective::gather_evidence(&GatherOptions {
        query: question.to_string(),
        library: a.library.clone(),
        gh_repo: a.gh_repo.clone(),
        manual: a.sources.clone(),
        hands_on: a.hands_on.clone(),
        limit: a.limit,
    })
    .await;
    let filtered = prefilter_sources(&pool);
    let scores = score_all(&filtered);

    let mut outcomes = Vec::new();
    for dimension in ConsensusDimension::ALL {
        match scores.iter().find(|s| s.dimension == dimension) {
            Some(s) => {
                let url = top_reference(dimension, &filtered).unwrap_or_default();
                let host = url
                    .split("://")
                    .nth(1)
                    .unwrap_or(url.as_str())
                    .split('/')
                    .next()
                    .unwrap_or("")
                    .trim_start_matches("www.")
                    .trim();
                let recommendation = if host.is_empty() {
                    dimension.as_str().to_string()
                } else {
                    format!("{}: {host}", dimension.as_str())
                };
                outcomes.push(DimensionOutcome {
                    dimension,
                    recommendation,
                    score_100: s.score,
                    weight: 1.0,
                });
            }
            None => outcomes.push(DimensionOutcome::no_data(dimension)),
        }
    }

    let mut ranked = outcomes.clone();
    ranked.sort_by(|x, y| {
        y.score_100
            .partial_cmp(&x.score_100)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let persisted_notes: Vec<crate::research::ResearchNote> = open_research_journal(profile)
        .ok()
        .and_then(|j| j.list_research_notes(50).ok())
        .unwrap_or_default();
    let branches = build_branches(&outcomes, &persisted_notes, None);
    let proposal: Vec<String> = if branches.is_empty() {
        ranked
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let tag = if i < 26 {
                    ((b'A' + i as u8) as char).to_string()
                } else {
                    format!("P{}", i + 1)
                };
                format!(
                    "{tag}: {} via {} ({:.2})",
                    o.recommendation,
                    o.dimension.as_str(),
                    o.score_100 / 100.0
                )
            })
            .collect()
    } else {
        branch_proposal_lines(&branches)
    };
    let bugs: Vec<String> = filtered
        .iter()
        .map(|s| s.url.as_str())
        .filter(|u| u.contains("/issues/") || u.contains("/discussions/"))
        .take(10)
        .map(String::from)
        .collect();
    let mut authors: Vec<String> = a
        .hands_on
        .iter()
        .map(|s| s.trim())
        .filter(|u| !u.is_empty())
        .map(|u| format!("expert note: {u}"))
        .collect();
    for n in persisted_notes.iter().take(10) {
        authors.push(format!("note {}: {} -> {}", n.id, n.title, n.decision));
    }
    let enterprise_refs: Vec<String> = filtered
        .iter()
        .filter(|s| s.kind.contains("github"))
        .take(10)
        .map(|s| s.url.clone())
        .collect();

    let journal_ref = journal_ref_for_run(&run_id);
    let report = build_report(
        &run_id,
        question,
        &outcomes,
        filtered.len() as u32,
        ReportSections {
            authors,
            enterprise: enterprise_refs,
            bugs,
            proposal,
            journal_ref: Some(journal_ref.clone()),
        },
    )
    .map_err(|e| anyhow::anyhow!("research query: report build failed: {e}"))?;

    let status = match Some(report.confidence) {
        Some(c) if c.is_finite() && c >= floor => ResearchRunStatus::Completed,
        _ => ResearchRunStatus::NeedingHuman,
    };

    if let Ok(journal) = open_research_journal(profile) {
        let _ = journal.create_research_run(&run_id, question, &ResearchRunStatus::Running);
        for s in &filtered {
            let source_id = format!("rs-{}", uuid::Uuid::new_v4());
            let _ = journal.add_research_source(
                &source_id,
                &run_id,
                if s.kind.is_empty() {
                    "web"
                } else {
                    s.kind.as_str()
                },
                &s.url,
                Some(s.weight),
            );
        }
        for s in &scores {
            let _ =
                journal.save_research_consensus(&run_id, &s.dimension, s.score, s.note.as_deref());
        }
        let _ =
            journal.complete_research_run(&run_id, &status, report.confidence, &report.recommended);
        let _ = journal.record_research_journal_ref(&journal_ref, &run_id, question);
    }

    let yaml = serde_yaml::to_string(&report).context("research query: YAML serialisation")?;
    println!("{yaml}");
    eprintln!(
        "research query: run {run_id} status={} confidence={:.2} sources={} journal_ref={journal_ref} (floor {floor})",
        status.as_str(),
        report.confidence,
        filtered.len(),
    );
    Ok(())
}

async fn run_note(a: NoteArgs, profile: &str) -> Result<()> {
    use crate::research::{parse_tags, ResearchNote};

    let title = a.title.trim();
    if title.is_empty() {
        anyhow::bail!("research note: --title must not be empty");
    }
    let decision = a.decision.trim();
    if decision.is_empty() {
        anyhow::bail!("research note: --decision must not be empty");
    }
    if !a.confidence.is_finite() || !(0.0..=1.0).contains(&a.confidence) {
        anyhow::bail!("research note: --confidence must be within 0.0..=1.0");
    }
    let signature = a.signature.trim();
    if signature.is_empty() {
        anyhow::bail!("research note: --signature must not be empty");
    }
    let id = match a.id.as_deref().map(str::trim) {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => {
            let date = chrono::Utc::now().format("%Y%m%d");
            let short = uuid::Uuid::new_v4().simple().to_string();
            format!("rn-{date}-{}", &short[..8])
        }
    };
    let note = ResearchNote {
        id: id.clone(),
        title: title.to_string(),
        project: a
            .project
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        decision: decision.to_string(),
        outcome: a
            .outcome
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        confidence: a.confidence,
        tags: parse_tags(&a.tags),
        attached_at: chrono::Utc::now().to_rfc3339(),
        signature: signature.to_string(),
    };
    note.validate()
        .map_err(|e| anyhow::anyhow!("research note: {e}"))?;
    let journal = open_research_journal(profile)?;
    journal.save_research_note(&note)?;
    let json = serde_json::to_string(&note).context("research note: serialisation")?;
    println!("{json}");
    eprintln!("research note: saved {id}");
    Ok(())
}

async fn run_branches(a: BranchesArgs, profile: &str) -> Result<()> {
    use crate::research::{
        branch_proposal_lines, build_branches, journal_ref_for_run, prefilter_sources, score_all,
        top_reference, ConsensusDimension, DimensionOutcome, SourceInput,
    };

    let run_id = a.run_id.trim();
    if run_id.is_empty() {
        anyhow::bail!("research branches: --run-id must not be empty");
    }
    let journal = open_research_journal(profile)?;
    let run = journal
        .get_research_run(run_id)?
        .ok_or_else(|| anyhow::anyhow!("research branches: unknown run {run_id}"))?;
    let stored = journal.list_research_sources(run_id)?;
    let pool: Vec<SourceInput> = stored
        .iter()
        .map(|s| SourceInput::new(&s.url, &s.kind, s.score.unwrap_or(1.0)))
        .collect();
    let filtered = prefilter_sources(&pool);
    let scores = score_all(&filtered);
    let mut outcomes = Vec::new();
    for dimension in ConsensusDimension::ALL {
        match scores.iter().find(|s| s.dimension == dimension) {
            Some(s) => {
                let url = top_reference(dimension, &filtered).unwrap_or_default();
                let host = url
                    .split("://")
                    .nth(1)
                    .unwrap_or(url.as_str())
                    .split('/')
                    .next()
                    .unwrap_or("")
                    .trim_start_matches("www.")
                    .trim();
                let recommendation = if host.is_empty() {
                    dimension.as_str().to_string()
                } else {
                    format!("{}: {host}", dimension.as_str())
                };
                outcomes.push(DimensionOutcome {
                    dimension,
                    recommendation,
                    score_100: s.score,
                    weight: 1.0,
                });
            }
            None => outcomes.push(DimensionOutcome::no_data(dimension)),
        }
    }
    let notes = journal.list_research_notes(50).unwrap_or_default();
    let branches = build_branches(&outcomes, &notes, a.project_hint.as_deref());
    let proposal = branch_proposal_lines(&branches);
    let out = serde_json::json!({
        "run_id": run.id,
        "query": run.query,
        "journal_ref": journal_ref_for_run(&run.id),
        "branches": branches,
        "proposal": proposal,
    });
    println!(
        "{}",
        serde_json::to_string(&out).context("research branches: serialisation")?
    );
    eprintln!(
        "research branches: run {run_id} branches={} notes={}",
        branches.len(),
        notes.len(),
    );
    Ok(())
}

fn open_research_journal(profile: &str) -> Result<crate::journal::Journal> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    crate::journal::Journal::open(&root)
}

async fn run_feasibility(a: FeasibilityArgs, profile: &str) -> Result<()> {
    use crate::research::{
        cache_key, fail_safe_status, is_cache_fresh, metrics_for, mint_probe_id, parse_domains,
        probe_feasibility, FeasibilityDomain,
    };

    let topic = a.topic.trim();
    if topic.is_empty() {
        anyhow::bail!("research feasibility: topic must not be empty");
    }
    if a.min_sources == 0 {
        anyhow::bail!("research feasibility: --min-sources must be >= 1");
    }
    let domains: Vec<FeasibilityDomain> =
        parse_domains(&a.domain).map_err(|e| anyhow::anyhow!("research feasibility: {e}"))?;
    let probe_id = match a.probe_id.as_deref().map(str::trim) {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => mint_probe_id(),
    };
    let key = cache_key(topic, &domains);
    let domain_tag = domains
        .iter()
        .map(|d| d.as_str().to_string())
        .collect::<Vec<_>>()
        .join(",");

    if !a.no_cache {
        if let Ok(journal) = open_research_journal(profile) {
            if let Ok(Some((cached, created_at))) = journal.get_feasibility_report(&key) {
                let now = chrono::Utc::now().to_rfc3339();
                if is_cache_fresh(&created_at, &now) {
                    print_feasibility(&cached);
                    eprintln!(
                        "research feasibility: probe {probe_id} cache hit (key {key}, age within 12d TTL)"
                    );
                    return Ok(());
                }
            }
        }
    }

    let started = std::time::Instant::now();
    let mut report = probe_feasibility(&probe_id, topic, &domains, a.min_sources, a.limit).await;
    if a.require_artifact_evidence && report.artifact_evidence.is_empty() {
        report.found = false;
        if !report
            .red_flags
            .iter()
            .any(|f| f.contains("no artifact evidence"))
        {
            report.red_flags.push("no artifact evidence".to_string());
            report.red_flags.sort();
        }
        report.confidence = 0.0;
        report.recommended_next_step = format!(
            "bloquear el avance y pedir confirmación del usuario (RFC 23 §5) \
             antes de creer que '{topic}' existe: --require-artifact-evidence \
             sin evidencia. Sin evidencia no hay output (RFC 10 §10 fail-safe)."
        );
    }
    report
        .validate()
        .map_err(|e| anyhow::anyhow!("research feasibility: {e}"))?;
    let status = fail_safe_status(&report);
    let metrics = metrics_for(&report, started.elapsed().as_millis() as u64);

    if let Ok(journal) = open_research_journal(profile) {
        let _ = journal.save_feasibility_report(&key, topic, &domain_tag, &report);
    }

    print_feasibility(&report);
    eprintln!(
        "research feasibility: probe {probe_id} status={} confidence={:.2} evidence={} red_flags={} elapsed_ms={} (floor {})",
        status.as_str(),
        report.confidence,
        metrics.evidence_count,
        metrics.red_flag_count,
        metrics.elapsed_ms,
        crate::research::FAIL_SAFE_CONFIDENCE,
    );
    Ok(())
}

fn print_feasibility(report: &crate::research::FeasibilityReport) {
    println!("Found: {}", report.found);
    println!("Confidence: {:.2}", report.confidence);
    println!("Evidence:");
    if report.artifact_evidence.is_empty() {
        println!("  [none]");
    }
    for (i, e) in report.artifact_evidence.iter().enumerate() {
        let extra = match (e.stars_or_stargazers, e.last_release_at.as_deref()) {
            (Some(s), Some(r)) => format!(" ({s} stars, last release {r})"),
            (Some(s), None) => format!(" ({s} stars)"),
            (None, Some(r)) => format!(" (last release {r})"),
            (None, None) => String::new(),
        };
        println!(
            "  [{}] {} ({}, fetched {}){extra}",
            i + 1,
            e.url,
            e.kind.as_str(),
            e.fetched_at
        );
    }
    if report.red_flags.is_empty() {
        println!("Red flags: [none]");
    } else {
        println!("Red flags: [{}]", report.red_flags.join("; "));
    }
    println!("Recommended: {}", report.recommended_next_step);
}

#[cfg(feature = "doc-ingest")]
async fn run_ingest(a: IngestArgs, profile: &str) -> Result<()> {
    let doc = crate::research::ingest::ingest_file(&a.file)
        .map_err(|e| anyhow::anyhow!("research ingest: {e}"))?;
    if a.raw {
        println!("{}", doc.markdown);
    } else {
        let json = serde_json::to_string(&IngestOutputJson {
            source_path: doc.source_path.clone(),
            format: doc.format.clone(),
            bytes: doc.bytes,
            title: doc.title.clone(),
            markdown: doc.markdown.clone(),
        })
        .context("research ingest: output serialisation")?;
        println!("{json}");
    }
    if let Some(run_id) = a.run_id.as_deref() {
        let run_id = run_id.trim();
        if run_id.is_empty() {
            anyhow::bail!("research ingest: --run-id must not be empty");
        }
        let pid = crate::profiles::ProfileId::new(profile);
        let root = crate::profiles::resolve_root(&pid)?;
        let journal = crate::journal::Journal::open(&root)?;
        if journal.get_research_run(run_id)?.is_none() {
            journal.create_research_run(
                run_id,
                &doc.title.clone().unwrap_or_else(|| doc.source_path.clone()),
                &crate::research::ResearchRunStatus::Running,
            )?;
        }
        let source_id = format!("rs-{}", uuid::Uuid::new_v4());
        journal.add_research_source(
            &source_id,
            run_id,
            crate::research::DOCUMENT_KIND,
            &doc.source_path,
            None,
        )?;
        eprintln!("research ingest: recorded {source_id} (document) on run {run_id}");
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

#[derive(serde::Serialize, Debug)]
#[cfg(feature = "doc-ingest")]
struct IngestOutputJson {
    source_path: String,
    format: crate::research::DocFormat,
    bytes: u64,
    title: Option<String>,
    markdown: String,
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
            ResearchSub::Query(_) => panic!("expected Docs"),
            ResearchSub::Note(_) | ResearchSub::Branches(_) | ResearchSub::Feasibility(_) => {
                panic!("expected Docs")
            }
            #[cfg(feature = "doc-ingest")]
            ResearchSub::Ingest(_) => panic!("expected Docs"),
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
            ResearchSub::Query(_) => panic!("expected Docs"),
            ResearchSub::Note(_) | ResearchSub::Branches(_) | ResearchSub::Feasibility(_) => {
                panic!("expected Docs")
            }
            #[cfg(feature = "doc-ingest")]
            ResearchSub::Ingest(_) => panic!("expected Docs"),
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
    #[cfg(feature = "doc-ingest")]
    fn parse_ingest_subcmd() {
        let cli = TestCli::try_parse_from(["test", "ingest", "spec.pdf"]).unwrap();
        match cli.action {
            ResearchSub::Ingest(a) => {
                assert_eq!(a.file, std::path::PathBuf::from("spec.pdf"));
                assert_eq!(a.run_id, None);
                assert!(!a.raw);
            }
            ResearchSub::Docs(_) => panic!("expected Ingest"),
            ResearchSub::Query(_) => panic!("expected Ingest"),
            #[cfg(feature = "firecrawl")]
            _ => panic!("expected Ingest"),
        }
    }

    #[test]
    #[cfg(feature = "doc-ingest")]
    fn parse_ingest_with_run_id_and_raw() {
        let cli = TestCli::try_parse_from([
            "test",
            "ingest",
            "notes.docx",
            "--run-id",
            "rr-001",
            "--raw",
        ])
        .unwrap();
        match cli.action {
            ResearchSub::Ingest(a) => {
                assert_eq!(a.run_id.as_deref(), Some("rr-001"));
                assert!(a.raw);
            }
            ResearchSub::Docs(_) => panic!("expected Ingest"),
            ResearchSub::Query(_) => panic!("expected Ingest"),
            #[cfg(feature = "firecrawl")]
            _ => panic!("expected Ingest"),
        }
    }

    #[test]
    #[cfg(feature = "doc-ingest")]
    fn parse_ingest_requires_file() {
        let r = TestCli::try_parse_from(["test", "ingest"]);
        assert!(r.is_err(), "ingest without file must error");
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

    #[test]
    fn parse_query_subcmd_defaults() {
        let cli = TestCli::try_parse_from(["test", "query", "Event sourcing vs CRDT?"]).unwrap();
        match cli.action {
            ResearchSub::Query(a) => {
                assert_eq!(a.question, "Event sourcing vs CRDT?");
                assert!(a.sources.is_empty());
                assert!(a.hands_on.is_empty());
                assert_eq!(a.library, None);
                assert_eq!(a.gh_repo, None);
                assert_eq!(a.limit, 5);
                assert_eq!(a.run_id, None);
                assert_eq!(a.min_confidence, None);
            }
            ResearchSub::Docs(_) => panic!("expected Query"),
            ResearchSub::Note(_) | ResearchSub::Branches(_) | ResearchSub::Feasibility(_) => {
                panic!("expected Query")
            }
            #[cfg(feature = "doc-ingest")]
            ResearchSub::Ingest(_) => panic!("expected Query"),
            #[cfg(feature = "firecrawl")]
            _ => panic!("expected Query"),
        }
    }

    #[test]
    fn parse_query_with_evidence_flags() {
        let cli = TestCli::try_parse_from([
            "test",
            "query",
            "Which runtime?",
            "--source",
            "https://github.com/tokio-rs/tokio/issues/1",
            "--source",
            "https://arxiv.org/abs/2401.00001",
            "--hands-on",
            "https://internal.example/notes/runbook",
            "--library",
            "tokio",
            "--gh-repo",
            "tokio-rs/tokio",
            "--limit",
            "3",
            "--run-id",
            "rr-2026-07-04-001",
            "--min-confidence",
            "0.7",
        ])
        .unwrap();
        match cli.action {
            ResearchSub::Query(a) => {
                assert_eq!(a.sources.len(), 2);
                assert_eq!(a.hands_on.len(), 1);
                assert_eq!(a.library.as_deref(), Some("tokio"));
                assert_eq!(a.gh_repo.as_deref(), Some("tokio-rs/tokio"));
                assert_eq!(a.limit, 3);
                assert_eq!(a.run_id.as_deref(), Some("rr-2026-07-04-001"));
                assert_eq!(a.min_confidence, Some(0.7));
            }
            ResearchSub::Docs(_) => panic!("expected Query"),
            ResearchSub::Note(_) | ResearchSub::Branches(_) | ResearchSub::Feasibility(_) => {
                panic!("expected Query")
            }
            #[cfg(feature = "doc-ingest")]
            ResearchSub::Ingest(_) => panic!("expected Query"),
            #[cfg(feature = "firecrawl")]
            _ => panic!("expected Query"),
        }
    }

    #[test]
    fn parse_query_requires_question() {
        let r = TestCli::try_parse_from(["test", "query"]);
        assert!(r.is_err(), "query without question must error");
    }

    #[test]
    fn parse_note_subcmd_defaults() {
        let cli = TestCli::try_parse_from([
            "test",
            "note",
            "--title",
            "Lo hice así en producción",
            "--decision",
            "Event Sourcing + Kafka",
        ])
        .unwrap();
        match cli.action {
            ResearchSub::Note(a) => {
                assert_eq!(a.title, "Lo hice así en producción");
                assert_eq!(a.decision, "Event Sourcing + Kafka");
                assert_eq!(a.project, None);
                assert_eq!(a.confidence, 0.8);
                assert_eq!(a.signature, "operator");
                assert_eq!(a.id, None);
            }
            _ => panic!("expected Note"),
        }
    }

    #[test]
    fn parse_note_with_all_fields() {
        let cli = TestCli::try_parse_from([
            "test",
            "note",
            "--title",
            "t",
            "--decision",
            "d",
            "--project",
            "fintech X",
            "--outcome",
            "exitoso pero costoso en ops",
            "--confidence",
            "0.81",
            "--tags",
            "architecture, event-sourcing",
            "--signature",
            "max",
            "--id",
            "rn-2026-07-04-001",
        ])
        .unwrap();
        match cli.action {
            ResearchSub::Note(a) => {
                assert_eq!(a.project.as_deref(), Some("fintech X"));
                assert_eq!(a.outcome.as_deref(), Some("exitoso pero costoso en ops"));
                assert_eq!(a.confidence, 0.81);
                assert_eq!(a.tags, "architecture, event-sourcing");
                assert_eq!(a.signature, "max");
                assert_eq!(a.id.as_deref(), Some("rn-2026-07-04-001"));
            }
            _ => panic!("expected Note"),
        }
    }

    #[test]
    fn parse_note_requires_title_and_decision() {
        let r = TestCli::try_parse_from(["test", "note", "--title", "t"]);
        assert!(r.is_err(), "note without decision must error");
        let r = TestCli::try_parse_from(["test", "note", "--decision", "d"]);
        assert!(r.is_err(), "note without title must error");
    }

    #[test]
    fn parse_branches_subcmd() {
        let cli = TestCli::try_parse_from([
            "test",
            "branches",
            "--run-id",
            "rr-2026-07-04-001",
            "--project-hint",
            "atlas-os",
        ])
        .unwrap();
        match cli.action {
            ResearchSub::Branches(a) => {
                assert_eq!(a.run_id, "rr-2026-07-04-001");
                assert_eq!(a.project_hint.as_deref(), Some("atlas-os"));
            }
            _ => panic!("expected Branches"),
        }
    }

    #[test]
    fn parse_branches_requires_run_id() {
        let r = TestCli::try_parse_from(["test", "branches"]);
        assert!(r.is_err(), "branches without run-id must error");
    }

    #[test]
    fn parse_feasibility_subcmd_defaults() {
        let cli =
            TestCli::try_parse_from(["test", "feasibility", "casas con impresora 3D"]).unwrap();
        match cli.action {
            ResearchSub::Feasibility(a) => {
                assert_eq!(a.topic, "casas con impresora 3D");
                assert_eq!(a.domain, "");
                assert_eq!(a.min_sources, 3);
                assert!(!a.require_artifact_evidence);
                assert_eq!(a.limit, 5);
                assert_eq!(a.probe_id, None);
                assert!(!a.no_cache);
            }
            _ => panic!("expected Feasibility"),
        }
    }

    #[test]
    fn parse_feasibility_with_domains_and_flags() {
        let cli = TestCli::try_parse_from([
            "test",
            "feasibility",
            "casas con impresora 3D",
            "--domain",
            "hardware,vendor,academic",
            "--min-sources",
            "2",
            "--require-artifact-evidence",
            "--limit",
            "3",
            "--probe-id",
            "fp-2026-07-13-00000001",
            "--no-cache",
        ])
        .unwrap();
        match cli.action {
            ResearchSub::Feasibility(a) => {
                assert_eq!(a.domain, "hardware,vendor,academic");
                assert_eq!(a.min_sources, 2);
                assert!(a.require_artifact_evidence);
                assert_eq!(a.limit, 3);
                assert_eq!(a.probe_id.as_deref(), Some("fp-2026-07-13-00000001"));
                assert!(a.no_cache);
            }
            _ => panic!("expected Feasibility"),
        }
    }

    #[test]
    fn parse_feasibility_requires_topic() {
        let r = TestCli::try_parse_from(["test", "feasibility"]);
        assert!(r.is_err(), "feasibility without topic must error");
    }
}
