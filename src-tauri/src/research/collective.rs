// Atlas OS — Collective Engineering Intelligence (RFC 10 §5, Phase 3 sub-fase 3.3).
//
// Formalises the "social wisdom" pipeline: raw evidence hits go through a
// deterministic weak-model pre-filter (plan 30 §A.4 — the cheap System One
// pass that classifies and de-noises before any strong-model synthesis),
// then one `ConsensusScorer` per dimension scores the survivors 0–100 with
// cited references, and `combined_confidence` folds the four scores into a
// single 0..1 confidence where hands-on expert notes weigh ×1.5 (RFC 10 §3).
// A confidence below `FAIL_SAFE_CONFIDENCE` (or no evidence at all) yields
// `ResearchRunStatus::NeedingHuman` — the RFC 10 §10 hard anti-hallucination
// escape: no evidence, no recommendation.
//
// Source collectors (GitHub via the `gh` CLI, papers via the arXiv API,
// official docs via the 3.1 `DocsGateway`) are best-effort and dependency-
// free (existing `reqwest` + `tokio::process` only, RFC 25 §11): any failure
// contributes zero sources instead of failing the run, and the fail-safe
// reports the shortfall honestly. SECTOR C out-of-scope items (dedicated
// Reddit/SO scrapers, arXiv MCP, Semantic Scholar formal API) stay out —
// puntual fetches plus explicit `--source` URLs are enough for 3.3.
//
// Scoring is deliberately deterministic: the same `SourceInput` list always
// produces the same `ConsensusScore`, which is what makes the fixture tests
// stable. Each scorer weights every source by kind-relevance (a docs URL
// counts fully for Official and only partially for Academic, etc.), so a
// mixed fixture yields distinct per-dimension scores.

use serde::{Deserialize, Serialize};

use super::consensus::{
    combined_confidence, ConsensusDimension, ConsensusScore, ConsensusScorer, SourceInput,
    MIN_SOURCES,
};
use super::report::{
    ConsensusEntry, ReportError, ResearchConsensus, ResearchRunReport, ResearchRunStatus,
};

/// Hands-on expert notes (RFC 10 §3) weigh ×1.5 against literature.
/// Applied to the `weight` column before `combined_confidence`.
pub const HANDS_ON_WEIGHT: f64 = 1.5;
/// RFC 10 §10 fail-safe floor: confidence below this (or no evidence at
/// all) marks the run `needing_human` instead of synthesising an answer.
pub const FAIL_SAFE_CONFIDENCE: f64 = 0.6;
/// Binary probed for the GitHub collector (same `PATH` pattern the 3.1
/// docs gateway uses for `ctx7max`).
pub const GH_BIN: &str = "gh";
/// arXiv query API endpoint (plan 30 §B 3.3 academic via webfetch shape).
pub const ARXIV_API: &str = "https://export.arxiv.org/api/query";
/// Per-collector network ceiling so one slow source never stalls the run.
const COLLECT_TIMEOUT_SECS: u64 = 10;
/// Cap per live collector; the CLI `--limit` trims further.
const COLLECT_MAX_RESULTS: u8 = 10;

/// `research_sources.kind` for operator-attached expert evidence (RFC 10 §3).
pub const HANDS_ON_KIND: &str = "hands-on";

/// Fold the hands-on multiplier into a base weight.
pub fn hands_on_weight(base: f64) -> f64 {
    base * HANDS_ON_WEIGHT
}

/// Classify a URL into a `research_sources.kind` bucket by host. Used for
/// `--source` URLs where the operator did not pin a kind explicitly.
pub fn classify_source_kind(url: &str) -> &'static str {
    let lower = url.to_ascii_lowercase();
    let host = lower
        .split("://")
        .nth(1)
        .unwrap_or(&lower)
        .split('/')
        .next()
        .unwrap_or("");
    let host = host.strip_prefix("www.").unwrap_or(host);
    if host == "github.com" || host.ends_with(".github.com") || host == "gist.github.com" {
        return "github";
    }
    if host == "arxiv.org" || host == "export.arxiv.org" {
        return "paper";
    }
    if host == "stackoverflow.com"
        || host.ends_with(".stackexchange.com")
        || host == "reddit.com"
        || host == "news.ycombinator.com"
        || host == "lobste.rs"
        || host == "dev.to"
    {
        return "community";
    }
    if host == "docs.rs"
        || host.starts_with("docs.")
        || host.ends_with(".dev")
        || lower.contains("/docs/")
        || lower.contains("/documentation/")
    {
        return "documentation";
    }
    "web"
}

/// Weak-model pre-filter (plan 30 §A.4): drop empty URLs, non-positive or
/// non-finite weights, de-duplicate by URL (first occurrence wins), sort by
/// URL. Deterministic — the scorers see the same order every run.
pub fn prefilter_sources(sources: &[SourceInput]) -> Vec<SourceInput> {
    let mut seen: Vec<String> = Vec::new();
    let mut out: Vec<SourceInput> = Vec::new();
    for s in sources {
        let url = s.url.trim();
        if url.is_empty() || !s.weight.is_finite() || s.weight <= 0.0 {
            continue;
        }
        if seen.iter().any(|u| u == url) {
            continue;
        }
        seen.push(url.to_string());
        out.push(SourceInput {
            url: url.to_string(),
            kind: s.kind.trim().to_string(),
            weight: s.weight,
        });
    }
    out.sort_by(|a, b| a.url.cmp(&b.url));
    out
}

fn relevance(dimension: ConsensusDimension, kind: &str) -> f64 {
    let k = kind.to_ascii_lowercase();
    let k = k.as_str();
    let expert = k.contains("hands-on")
        || k.contains("hands_on")
        || k.contains("handson")
        || k == "note"
        || k == "expert";
    if expert {
        return 0.7;
    }
    match dimension {
        ConsensusDimension::Community => {
            if k.contains("communit")
                || k.contains("reddit")
                || k.contains("hackernews")
                || k.contains("lobster")
                || k.contains("stackoverflow")
                || k.contains("discussion")
            {
                1.0
            } else if k.contains("blog") {
                0.8
            } else if k.contains("web") {
                0.6
            } else if k.contains("document") || k.contains("docu") {
                0.5
            } else if k.contains("github") || k.contains("enterprise") || k.contains("maintainer") {
                0.4
            } else if k.contains("paper") || k.contains("arxiv") || k.contains("academic") {
                0.2
            } else {
                0.5
            }
        }
        ConsensusDimension::Enterprise => {
            if k.contains("github")
                || k.contains("enterprise")
                || k.contains("maintainer")
                || k.contains("vendor")
                || k.contains("release")
            {
                1.0
            } else if k.contains("discussion") {
                0.7
            } else if k.contains("document") || k.contains("docu") || k.contains("official") {
                0.5
            } else if k.contains("web") {
                0.4
            } else if k.contains("blog") || k.contains("communit") {
                0.3
            } else if k.contains("paper") || k.contains("arxiv") || k.contains("academic") {
                0.2
            } else {
                0.4
            }
        }
        ConsensusDimension::Academic => {
            if k.contains("paper")
                || k.contains("arxiv")
                || k.contains("academic")
                || k.contains("scholar")
            {
                1.0
            } else if k.contains("document") || k.contains("docu") {
                0.3
            } else if k.contains("web") || k.contains("blog") {
                0.25
            } else if k.contains("github") || k.contains("communit") || k.contains("enterprise") {
                0.2
            } else {
                0.25
            }
        }
        ConsensusDimension::Official => {
            if k.contains("document")
                || k.contains("docu")
                || k.contains("official")
                || k.contains("rfc")
            {
                1.0
            } else if k.contains("vendor") {
                0.7
            } else if k.contains("github") || k.contains("release") {
                0.5
            } else if k.contains("web") || k.contains("blog") {
                0.3
            } else if k.contains("paper") || k.contains("communit") {
                0.2
            } else {
                0.3
            }
        }
    }
}

fn score_pool(dimension: ConsensusDimension, sources: &[SourceInput]) -> Option<ConsensusScore> {
    let items = prefilter_sources(sources);
    if items.len() < MIN_SOURCES {
        return None;
    }
    let mut rel_sum = 0.0;
    let mut weight_sum = 0.0;
    for s in &items {
        rel_sum += s.weight * relevance(dimension, &s.kind);
        weight_sum += s.weight;
    }
    if weight_sum <= 0.0 {
        return None;
    }
    let score = (rel_sum / weight_sum) * 100.0;
    Some(ConsensusScore {
        dimension,
        score: score.clamp(0.0, 100.0),
        references: items.iter().map(|s| s.url.clone()).collect(),
        note: Some(format!(
            "{} pre-filtered sources, kind-weighted relevance",
            items.len()
        )),
    })
}

/// RFC 10 §5 community consensus: SO / Reddit / HN / discussions / blogs
/// (SECTOR C: puntual fetches, no dedicated scrapers in this iteration).
pub struct CommunityScorer;
/// RFC 10 §5 enterprise consensus: maintainers via `gh` Issues/Discussions.
pub struct EnterpriseScorer;
/// RFC 10 §5 academic consensus: papers via arXiv webfetch shape.
pub struct AcademicScorer;
/// RFC 10 §5 official consensus: docs via the 3.1 `DocsGateway`.
pub struct OfficialScorer;

impl ConsensusScorer for CommunityScorer {
    fn dimension(&self) -> ConsensusDimension {
        ConsensusDimension::Community
    }
    fn score_sources(&self, sources: &[SourceInput]) -> Option<ConsensusScore> {
        score_pool(ConsensusDimension::Community, sources)
    }
}

impl ConsensusScorer for EnterpriseScorer {
    fn dimension(&self) -> ConsensusDimension {
        ConsensusDimension::Enterprise
    }
    fn score_sources(&self, sources: &[SourceInput]) -> Option<ConsensusScore> {
        score_pool(ConsensusDimension::Enterprise, sources)
    }
}

impl ConsensusScorer for AcademicScorer {
    fn dimension(&self) -> ConsensusDimension {
        ConsensusDimension::Academic
    }
    fn score_sources(&self, sources: &[SourceInput]) -> Option<ConsensusScore> {
        score_pool(ConsensusDimension::Academic, sources)
    }
}

impl ConsensusScorer for OfficialScorer {
    fn dimension(&self) -> ConsensusDimension {
        ConsensusDimension::Official
    }
    fn score_sources(&self, sources: &[SourceInput]) -> Option<ConsensusScore> {
        score_pool(ConsensusDimension::Official, sources)
    }
}

/// Score every dimension over the same pre-filtered pool. Returns one
/// entry per dimension that cleared the `MIN_SOURCES` floor; an empty vec
/// means "no data" and the caller must fail safe.
pub fn score_all(sources: &[SourceInput]) -> Vec<ConsensusScore> {
    let scorers: Vec<Box<dyn ConsensusScorer>> = vec![
        Box::new(CommunityScorer),
        Box::new(EnterpriseScorer),
        Box::new(AcademicScorer),
        Box::new(OfficialScorer),
    ];
    scorers
        .iter()
        .filter_map(|s| s.score_sources(sources))
        .collect()
}

/// RFC 10 §10 fail-safe gate: `None` (no evidence) or confidence below
/// `FAIL_SAFE_CONFIDENCE` escapes to `NeedingHuman` instead of forcing a
/// synthesised answer.
pub fn apply_fail_safe(confidence: Option<f64>) -> ResearchRunStatus {
    match confidence {
        Some(c) if c.is_finite() && c >= FAIL_SAFE_CONFIDENCE => ResearchRunStatus::Completed,
        _ => ResearchRunStatus::NeedingHuman,
    }
}

/// One dimension's offline synthesis: which reference the dimension
/// converges on (`recommendation`), its 0–100 score, and its fold weight
/// into `combined_confidence` (hands-on evidence already carries ×1.5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DimensionOutcome {
    pub dimension: ConsensusDimension,
    pub recommendation: String,
    pub score_100: f64,
    pub weight: f64,
}

impl DimensionOutcome {
    pub fn no_data(dimension: ConsensusDimension) -> Self {
        Self {
            dimension,
            recommendation: "no-data".into(),
            score_100: 0.0,
            weight: 1.0,
        }
    }
}

/// Free-form report sections (RFC 10 §7): cited authors, enterprise
/// witnesses, bug refs, application branches, and the journal link owned
/// by sub-fase 3.4. Bundled so `build_report` stays under the
/// seven-argument clippy lint.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ReportSections {
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub enterprise: Vec<String>,
    #[serde(default)]
    pub bugs: Vec<String>,
    #[serde(default)]
    pub proposal: Vec<String>,
    #[serde(default)]
    pub journal_ref: Option<String>,
}

/// Assemble the canonical RFC 10 §7 `ResearchRunReport` from the four
/// dimension outcomes. Scores convert 0–100 → 0–1 entries; `confidence` is
/// the weighted mean; `recommended` is the top-scoring dimension's
/// recommendation (deterministic: `ConsensusDimension::ALL` order breaks
/// ties). The fail-safe status itself is NOT stored in the report — it
/// lives in the `research_runs` row via `apply_fail_safe`.
pub fn build_report(
    id: &str,
    query: &str,
    outcomes: &[DimensionOutcome],
    source_count: u32,
    sections: ReportSections,
) -> Result<ResearchRunReport, ReportError> {
    let entry_for = |dimension: ConsensusDimension| -> Result<ConsensusEntry, ReportError> {
        let o = outcomes
            .iter()
            .find(|o| o.dimension == dimension)
            .ok_or(ReportError::EmptyRecommendation)?;
        if o.recommendation.trim().is_empty() {
            return Err(ReportError::EmptyRecommendation);
        }
        if !o.score_100.is_finite() || o.score_100 < 0.0 || o.score_100 > 100.0 {
            return Err(ReportError::ScoreOutOfRange {
                value: o.score_100 / 100.0,
            });
        }
        Ok(ConsensusEntry {
            recommendation: o.recommendation.clone(),
            score: o.score_100 / 100.0,
        })
    };
    let consensus = ResearchConsensus {
        community: entry_for(ConsensusDimension::Community)?,
        enterprise: entry_for(ConsensusDimension::Enterprise)?,
        academic: entry_for(ConsensusDimension::Academic)?,
        official: entry_for(ConsensusDimension::Official)?,
    };
    let weighted: Vec<(ConsensusDimension, f64, f64)> = outcomes
        .iter()
        .map(|o| (o.dimension, o.score_100, o.weight))
        .collect();
    let confidence = combined_confidence(&weighted).unwrap_or(0.0);
    let mut best: Option<&DimensionOutcome> = None;
    for d in ConsensusDimension::ALL {
        if let Some(o) = outcomes.iter().find(|o| o.dimension == d) {
            match &best {
                None => best = Some(o),
                Some(b) if o.score_100 > b.score_100 => best = Some(o),
                _ => {}
            }
        }
    }
    let recommended = best
        .map(|o| o.recommendation.clone())
        .unwrap_or_else(|| "no-data".to_string());
    let report = ResearchRunReport {
        id: id.to_string(),
        query: query.to_string(),
        sources: source_count,
        consensus,
        authors: sections.authors,
        enterprise: sections.enterprise,
        bugs: sections.bugs,
        proposal: sections.proposal,
        confidence,
        recommended,
        journal_ref: sections.journal_ref,
    };
    report.validate()?;
    Ok(report)
}

/// Pick the report-level recommendation label for a scored dimension: the
/// leading reference host. Offline synthesis cannot name a strategy (that
/// is the strong model's job); the host keeps the YAML citable and honest.
pub fn leading_host(sources: &[SourceInput]) -> String {
    top_reference(ConsensusDimension::Community, sources)
        .and_then(|url| {
            url.split("://")
                .nth(1)
                .unwrap_or(url.as_str())
                .split('/')
                .next()
                .map(|h| h.trim_start_matches("www.").trim().to_string())
        })
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "no-data".to_string())
}

/// Highest-relevance reference for one dimension: ties break by URL so the
/// pick is deterministic. Returns `None` on an empty (or fully filtered)
/// pool — the caller renders `"no-data"` instead of inventing a source.
pub fn top_reference(dimension: ConsensusDimension, sources: &[SourceInput]) -> Option<String> {
    let mut items = prefilter_sources(sources);
    items.sort_by(|a, b| {
        relevance(dimension, &b.kind)
            .partial_cmp(&relevance(dimension, &a.kind))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.url.cmp(&b.url))
    });
    items.into_iter().next().map(|s| s.url)
}

// ---------------------------------------------------------------------------
// Live collectors (best-effort, never fail the run)
// ---------------------------------------------------------------------------

/// What the CLI `query` verb feeds into the scorers: operator URLs plus
/// best-effort live hits. Every collector degrades to `vec![]`.
pub struct GatherOptions {
    pub query: String,
    pub library: Option<String>,
    pub gh_repo: Option<String>,
    pub manual: Vec<String>,
    pub hands_on: Vec<String>,
    pub limit: u8,
}

/// Run every collector and return the merged evidence pool (unfiltered —
/// the scorers pre-filter internally, and the CLI persists the filtered
/// set so the journal matches the scores).
pub async fn gather_evidence(opts: &GatherOptions) -> Vec<SourceInput> {
    let mut pool: Vec<SourceInput> = Vec::new();
    for url in &opts.manual {
        let url = url.trim();
        if url.is_empty() {
            continue;
        }
        pool.push(SourceInput::new(url, classify_source_kind(url), 1.0));
    }
    for url in &opts.hands_on {
        let url = url.trim();
        if url.is_empty() {
            continue;
        }
        pool.push(SourceInput::new(url, HANDS_ON_KIND, hands_on_weight(1.0)));
    }
    let limit = opts.limit.clamp(1, COLLECT_MAX_RESULTS);
    let (gh, arxiv) = tokio::join!(
        collect_github(&opts.query, opts.gh_repo.as_deref(), limit),
        collect_arxiv(&opts.query, limit),
    );
    pool.extend(gh);
    pool.extend(arxiv);
    if let Some(library) = opts.library.as_deref() {
        let library = library.trim();
        if !library.is_empty() {
            pool.extend(collect_official_docs(library, &opts.query, limit).await);
        }
    }
    pool
}

/// GitHub Issues/Discussions via the `gh` CLI (RFC 10 §6). Missing binary,
/// missing auth, or non-zero exit → empty vec.
pub async fn collect_github(query: &str, repo: Option<&str>, limit: u8) -> Vec<SourceInput> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let mut cmd = tokio::process::Command::new(GH_BIN);
    cmd.arg("search").arg("issues").arg(query);
    cmd.arg("--limit").arg(limit.to_string());
    cmd.arg("--json").arg("url");
    if let Some(repo) = repo {
        let repo = repo.trim();
        if !repo.is_empty() {
            cmd.arg("--repo").arg(repo);
        }
    }
    let run = async {
        let out = cmd.output().await?;
        if !out.status.success() {
            return Ok::<Vec<SourceInput>, std::io::Error>(Vec::new());
        }
        Ok(parse_gh_urls(&String::from_utf8_lossy(&out.stdout), limit))
    };
    match tokio::time::timeout(std::time::Duration::from_secs(COLLECT_TIMEOUT_SECS), run).await {
        Ok(Ok(hits)) => hits,
        _ => Vec::new(),
    }
}

fn parse_gh_urls(stdout: &str, limit: u8) -> Vec<SourceInput> {
    let value: serde_json::Value = match serde_json::from_str(stdout) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let arr = match value.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };
    arr.iter()
        .filter_map(|item| item.get("url").and_then(|u| u.as_str()))
        .map(str::trim)
        .filter(|u| u.starts_with("http://") || u.starts_with("https://"))
        .take(limit as usize)
        .map(|u| SourceInput::new(u, "github", 1.0))
        .collect()
}

/// arXiv papers via the query API webfetch shape (SECTOR C: no dedicated
/// MCP in this iteration). Transport or parse failure → empty vec.
pub async fn collect_arxiv(query: &str, limit: u8) -> Vec<SourceInput> {
    let terms: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .take(5)
        .map(|w| format!("all:{w}"))
        .collect();
    if terms.is_empty() {
        return Vec::new();
    }
    let search = terms.join("+AND+");
    let run = async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(COLLECT_TIMEOUT_SECS))
            .user_agent("atlas-os/0.1 (research collective)")
            .build()?;
        let body = client
            .get(ARXIV_API)
            .query(&[
                ("search_query", search.as_str()),
                ("start", "0"),
                ("max_results", limit.to_string().as_str()),
            ])
            .header("Accept", "application/atom+xml")
            .send()
            .await?
            .text()
            .await?;
        Ok::<Vec<SourceInput>, anyhow::Error>(parse_arxiv_ids(&body, limit))
    };
    match tokio::time::timeout(
        std::time::Duration::from_secs(COLLECT_TIMEOUT_SECS + 2),
        run,
    )
    .await
    {
        Ok(Ok(hits)) => hits,
        _ => Vec::new(),
    }
}

fn parse_arxiv_ids(body: &str, limit: u8) -> Vec<SourceInput> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("<id>") {
        let after = &rest[start + 4..];
        let Some(end) = after.find("</id>") else {
            break;
        };
        let id = after[..end].trim();
        if (id.starts_with("http://") || id.starts_with("https://"))
            && id.contains("arxiv.org")
            && out.len() < limit as usize
        {
            out.push(SourceInput::new(id, "paper", 1.0));
        }
        rest = &after[end + 5..];
    }
    out
}

/// Official docs via the 3.1 `DocsGateway` (Context7Max → Context7 MCP
/// shape → official docs). Gateway errors or empty hits → empty vec.
pub async fn collect_official_docs(library: &str, question: &str, limit: u8) -> Vec<SourceInput> {
    let gateway = super::docs_gateway::DocsGateway::from_env();
    let hits = match gateway.query_docs(library, question).await {
        Ok(h) => h,
        Err(_) => return Vec::new(),
    };
    hits.into_iter()
        .take(limit as usize)
        .filter_map(|h| {
            let url = h.url.unwrap_or_else(|| {
                super::docs_gateway::official_docs_url(&h.library_id).unwrap_or_default()
            });
            let url = url.trim();
            if url.is_empty() {
                return None;
            }
            Some(SourceInput::new(url, "documentation", 1.0))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mixed_fixture() -> Vec<SourceInput> {
        vec![
            SourceInput::new("https://docs.rs/crate/tokio/latest", "documentation", 1.0),
            SourceInput::new("https://doc.rust-lang.org/std/", "documentation", 1.0),
            SourceInput::new("https://tokio.rs/docs", "documentation", 1.0),
            SourceInput::new("https://github.com/tokio-rs/tokio/issues/1", "github", 1.0),
            SourceInput::new(
                "https://github.com/tokio-rs/tokio/discussions/2",
                "github",
                1.0,
            ),
            SourceInput::new("https://stackoverflow.com/questions/1", "community", 1.0),
            SourceInput::new("https://arxiv.org/abs/2401.00001", "paper", 1.0),
        ]
    }

    #[test]
    fn scorers_are_deterministic_over_fixture() {
        let fixture = mixed_fixture();
        let once = score_all(&fixture);
        let twice = score_all(&fixture);
        assert_eq!(once, twice);
        assert_eq!(once.len(), 4);
        for s in &once {
            assert!((0.0..=100.0).contains(&s.score), "{s:?}");
            assert_eq!(s.references.len(), fixture.len());
        }
    }

    #[test]
    fn scorers_separate_dimensions_on_mixed_evidence() {
        let fixture = mixed_fixture();
        let official = OfficialScorer
            .score_sources(&fixture)
            .expect("official score");
        let academic = AcademicScorer
            .score_sources(&fixture)
            .expect("academic score");
        assert!(
            official.score > academic.score,
            "docs-heavy fixture must favour official ({}) over academic ({})",
            official.score,
            academic.score
        );
        let community = CommunityScorer
            .score_sources(&fixture)
            .expect("community score");
        let enterprise = EnterpriseScorer
            .score_sources(&fixture)
            .expect("enterprise score");
        assert_ne!(community.score, enterprise.score);
    }

    #[test]
    fn scorers_return_no_data_below_min_sources() {
        let few = &mixed_fixture()[..MIN_SOURCES - 1];
        assert!(CommunityScorer.score_sources(few).is_none());
        assert!(EnterpriseScorer.score_sources(few).is_none());
        assert!(AcademicScorer.score_sources(few).is_none());
        assert!(OfficialScorer.score_sources(few).is_none());
        assert!(score_all(&[]).is_empty());
    }

    #[test]
    fn prefilter_dedupes_and_drops_noise_deterministically() {
        let sources = vec![
            SourceInput::new("https://example.com/b", "web", 1.0),
            SourceInput::new("https://example.com/a", "web", 1.0),
            SourceInput::new("https://example.com/a", "web", 2.0),
            SourceInput::new("   ", "web", 1.0),
            SourceInput::new("https://example.com/c", "web", 0.0),
            SourceInput::new("https://example.com/d", "web", f64::NAN),
        ];
        let once = prefilter_sources(&sources);
        let twice = prefilter_sources(&sources);
        assert_eq!(once, twice);
        let urls: Vec<&str> = once.iter().map(|s| s.url.as_str()).collect();
        assert_eq!(urls, vec!["https://example.com/a", "https://example.com/b"]);
    }

    #[test]
    fn hands_on_notes_weigh_one_point_five() {
        assert!((hands_on_weight(1.0) - HANDS_ON_WEIGHT).abs() < 1e-12);
        assert!((HANDS_ON_WEIGHT - 1.5).abs() < 1e-12);
        let plain = combined_confidence(&[
            (ConsensusDimension::Community, 70.0, 1.0),
            (ConsensusDimension::Official, 80.0, 1.0),
        ])
        .unwrap();
        let expert = combined_confidence(&[
            (ConsensusDimension::Community, 70.0, 1.0),
            (ConsensusDimension::Official, 80.0, hands_on_weight(1.0)),
        ])
        .unwrap();
        assert!(expert > plain, "expert note must pull confidence up");
        let expected = (70.0 * 1.0 + 80.0 * 1.5) / 2.5 / 100.0;
        assert!((expert - expected).abs() < 1e-9);
    }

    #[test]
    fn fail_safe_escapes_below_threshold_and_on_no_data() {
        assert_eq!(apply_fail_safe(None), ResearchRunStatus::NeedingHuman);
        assert_eq!(apply_fail_safe(Some(0.0)), ResearchRunStatus::NeedingHuman);
        assert_eq!(
            apply_fail_safe(Some(FAIL_SAFE_CONFIDENCE - 0.01)),
            ResearchRunStatus::NeedingHuman
        );
        assert_eq!(
            apply_fail_safe(Some(FAIL_SAFE_CONFIDENCE)),
            ResearchRunStatus::Completed
        );
        assert_eq!(apply_fail_safe(Some(0.9)), ResearchRunStatus::Completed);
        assert_eq!(
            apply_fail_safe(Some(f64::NAN)),
            ResearchRunStatus::NeedingHuman
        );
    }

    #[test]
    fn build_report_folds_confidence_and_picks_top_dimension() {
        let outcomes = vec![
            DimensionOutcome {
                dimension: ConsensusDimension::Community,
                recommendation: "strategy-pattern".into(),
                score_100: 71.0,
                weight: 1.0,
            },
            DimensionOutcome {
                dimension: ConsensusDimension::Enterprise,
                recommendation: "event-sourcing".into(),
                score_100: 66.0,
                weight: 1.0,
            },
            DimensionOutcome {
                dimension: ConsensusDimension::Academic,
                recommendation: "crdt".into(),
                score_100: 55.0,
                weight: 1.0,
            },
            DimensionOutcome {
                dimension: ConsensusDimension::Official,
                recommendation: "event-sourcing".into(),
                score_100: 78.0,
                weight: 1.5,
            },
        ];
        let report = build_report(
            "rr-2026-07-04-001",
            "Event sourcing vs CRDT?",
            &outcomes,
            87,
            ReportSections {
                authors: vec!["Martin Kleppmann -> crdt".into()],
                enterprise: vec!["Microsoft -> event-sourcing".into()],
                bugs: vec!["GitHub issue #1234".into()],
                proposal: vec!["A: Event Sourcing + Kafka".into()],
                journal_ref: Some("jr-2026-07-04-001".into()),
            },
        )
        .unwrap();
        let expected = (71.0 + 66.0 + 55.0 + 78.0 * 1.5) / 4.5 / 100.0;
        assert!((report.confidence - expected).abs() < 1e-9);
        assert_eq!(report.recommended, "event-sourcing");
        let yaml = serde_yaml::to_string(&report).unwrap();
        let back: ResearchRunReport = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(report, back);
        assert_eq!(
            apply_fail_safe(Some(report.confidence)),
            ResearchRunStatus::Completed
        );
    }

    #[test]
    fn build_report_no_data_stays_valid_and_fails_safe() {
        let outcomes = vec![
            DimensionOutcome::no_data(ConsensusDimension::Community),
            DimensionOutcome::no_data(ConsensusDimension::Enterprise),
            DimensionOutcome::no_data(ConsensusDimension::Academic),
            DimensionOutcome::no_data(ConsensusDimension::Official),
        ];
        let report = build_report(
            "rr-no-data",
            "obscure topic with zero evidence?",
            &outcomes,
            0,
            ReportSections::default(),
        )
        .unwrap();
        assert_eq!(report.confidence, 0.0);
        assert_eq!(
            apply_fail_safe(Some(report.confidence)),
            ResearchRunStatus::NeedingHuman
        );
    }

    #[test]
    fn build_report_rejects_missing_dimension() {
        let err = build_report("rr-x", "q", &[], 0, ReportSections::default()).unwrap_err();
        assert_eq!(err, ReportError::EmptyRecommendation);
    }

    #[test]
    fn classify_source_kind_covers_plan_families() {
        assert_eq!(
            classify_source_kind("https://github.com/tokio-rs/tokio/issues/1"),
            "github"
        );
        assert_eq!(
            classify_source_kind("https://arxiv.org/abs/2401.00001"),
            "paper"
        );
        assert_eq!(
            classify_source_kind("https://stackoverflow.com/questions/1"),
            "community"
        );
        assert_eq!(
            classify_source_kind("https://news.ycombinator.com/item?id=1"),
            "community"
        );
        assert_eq!(
            classify_source_kind("https://docs.rs/crate/tokio/latest"),
            "documentation"
        );
        assert_eq!(classify_source_kind("https://example.com/essay"), "web");
    }

    #[test]
    fn top_reference_picks_dimension_leading_source() {
        let fixture = mixed_fixture();
        let official =
            top_reference(ConsensusDimension::Official, &fixture).expect("official top ref");
        assert!(
            official.contains("docs.rs")
                || official.contains("doc.rust-lang")
                || official.contains("tokio.rs"),
            "official top ref must be a docs URL, got {official}"
        );
        let academic =
            top_reference(ConsensusDimension::Academic, &fixture).expect("academic top ref");
        assert!(
            academic.contains("arxiv.org"),
            "academic top ref must be the paper, got {academic}"
        );
        assert!(top_reference(ConsensusDimension::Community, &[]).is_none());
    }

    #[test]
    fn parse_gh_urls_skips_non_http_rows() {
        let stdout = r#"[{"url":"https://github.com/o/r/issues/1"},{"url":""},{"nope":1}]"#;
        let hits = parse_gh_urls(stdout, 5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, "github");
    }

    #[test]
    fn parse_gh_urls_rejects_broken_json() {
        assert!(parse_gh_urls("not json", 5).is_empty());
    }

    #[test]
    fn parse_arxiv_ids_collects_only_arxiv_links() {
        let body = "<feed><entry><id>http://arxiv.org/abs/2401.00001v1</id></entry>\
             <entry><id>http://example.com/other</id></entry>\
             <entry><id>https://arxiv.org/abs/2401.00002</id></entry></feed>";
        let hits = parse_arxiv_ids(body, 5);
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|h| h.kind == "paper"));
    }
}
