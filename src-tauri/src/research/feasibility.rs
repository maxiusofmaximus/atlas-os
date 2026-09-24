// Atlas OS — probe_feasibility + fail-safe (RFC 10 §10–§11, Phase 3 sub-fase 3.5).
//
// Capability consumed by the Prompt Understanding Pipeline (RFC 23 §2.2 paso 6)
// when the `gap_type` signals `capacity_hallucination` or
// `unknown_tool_dependency`: before the system finishes believing "sí se
// puede", the Research Engine mechanically verifies whether the
// tool/dependency/route/domain exists. Sources per domain follow RFC 10
// §11.2; the gate follows §11.3 (≥ `min_sources` distinct sources, ≥ 1
// primary artifact, zero `red_flags`); reports cache for 12 days (§11.6).
// Confidence below `FAIL_SAFE_CONFIDENCE` (or `found == false`) escapes to
// `NeedingHuman` — the RFC 10 §10 hard anti-hallucination rule: no evidence,
// no output.
//
// Collectors are best-effort and dependency-free (existing `reqwest` +
// `tokio::process` only, RFC 25 §11): any transport/auth failure contributes
// zero evidence instead of failing the probe, and `evaluate` reports the
// shortfall honestly via `red_flags`. The pure `evaluate` core is fully
// deterministic so the fixture tests are stable; the strong model still owns
// any prose built on top of the report.

use serde::{Deserialize, Serialize};

/// Default `min_sources` (RFC 10 §11.1): three distinct sources.
pub const DEFAULT_MIN_SOURCES: u8 = 3;
/// Report cache TTL (RFC 10 §11.6): 12 days in seconds.
pub const CACHE_TTL_SECS: i64 = 12 * 24 * 3600;
/// Abandonment window (RFC 10 §11.3): zero releases in 12 months (≈365 days).
pub const ABANDONED_DAYS: i64 = 365;
/// Per-collector network ceiling (RFC 10 §11.7): three parallel sources must
/// resolve in under 5 s in the typical case.
pub const COLLECT_TIMEOUT_SECS: u64 = 5;
/// Cap per live collector; the CLI `--limit` trims further.
pub const COLLECT_MAX_RESULTS: u8 = 5;

/// RFC 10 §11.1 — which registry family the probe checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeasibilityDomain {
    Software,
    Hardware,
    Academic,
    Vendor,
}

impl FeasibilityDomain {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeasibilityDomain::Software => "software",
            FeasibilityDomain::Hardware => "hardware",
            FeasibilityDomain::Academic => "academic",
            FeasibilityDomain::Vendor => "vendor",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "software" => Some(FeasibilityDomain::Software),
            "hardware" => Some(FeasibilityDomain::Hardware),
            "academic" => Some(FeasibilityDomain::Academic),
            "vendor" => Some(FeasibilityDomain::Vendor),
            _ => None,
        }
    }

    pub const ALL: [FeasibilityDomain; 4] = [
        FeasibilityDomain::Software,
        FeasibilityDomain::Hardware,
        FeasibilityDomain::Academic,
        FeasibilityDomain::Vendor,
    ];
}

/// RFC 10 §11.1 — what kind of primary artifact an evidence hit points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Package,
    GithubRepo,
    Release,
    Paper,
    VendorProduct,
    VendorDocs,
    AcademicDataset,
    BenchmarkRun,
}

impl ArtifactKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactKind::Package => "package",
            ArtifactKind::GithubRepo => "github_repo",
            ArtifactKind::Release => "release",
            ArtifactKind::Paper => "paper",
            ArtifactKind::VendorProduct => "vendor_product",
            ArtifactKind::VendorDocs => "vendor_docs",
            ArtifactKind::AcademicDataset => "academic_dataset",
            ArtifactKind::BenchmarkRun => "benchmark_run",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "package" => Some(ArtifactKind::Package),
            "github_repo" | "githubrepo" | "repo" => Some(ArtifactKind::GithubRepo),
            "release" => Some(ArtifactKind::Release),
            "paper" => Some(ArtifactKind::Paper),
            "vendor_product" | "vendorproduct" | "product" => Some(ArtifactKind::VendorProduct),
            "vendor_docs" | "vendordocs" | "docs" => Some(ArtifactKind::VendorDocs),
            "academic_dataset" | "academicdataset" | "dataset" => {
                Some(ArtifactKind::AcademicDataset)
            }
            "benchmark_run" | "benchmarkrun" | "benchmark" => Some(ArtifactKind::BenchmarkRun),
            _ => None,
        }
    }

    /// RFC 10 §11.3 rule 2: a primary artifact is the original package /
    /// repo / release / paper / product / dataset / benchmark — a docs page
    /// alone never counts as primary.
    pub fn is_primary(&self) -> bool {
        !matches!(self, ArtifactKind::VendorDocs)
    }
}

/// RFC 10 §11.1 — one fetched artifact backing (or failing) the probe.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactEvidence {
    pub kind: ArtifactKind,
    pub url: String,
    #[serde(default)]
    pub fetched_at: String,
    #[serde(default)]
    pub raw_metadata: serde_json::Value,
    #[serde(default)]
    pub stars_or_stargazers: Option<u64>,
    #[serde(default)]
    pub last_release_at: Option<String>,
}

impl ArtifactEvidence {
    pub fn new(kind: ArtifactKind, url: &str) -> Self {
        Self {
            kind,
            url: url.to_string(),
            fetched_at: chrono::Utc::now().to_rfc3339(),
            raw_metadata: serde_json::Value::Null,
            stars_or_stargazers: None,
            last_release_at: None,
        }
    }

    pub fn validate(&self) -> Result<(), FeasibilityError> {
        if self.url.trim().is_empty() {
            return Err(FeasibilityError::EmptyUrl);
        }
        if !(self.url.starts_with("http://") || self.url.starts_with("https://")) {
            return Err(FeasibilityError::NonHttpUrl {
                url: self.url.clone(),
            });
        }
        Ok(())
    }
}

/// RFC 10 §11.1 — a single-domain probe request. Multi-domain CLI invocations
/// (`--domain hardware,vendor,academic`) fan out one probe per domain and
/// merge the evidence before `evaluate`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeasibilityProbe {
    pub topic: String,
    pub domain: FeasibilityDomain,
    #[serde(default = "default_min_sources")]
    pub min_sources: u8,
    #[serde(default)]
    pub require_artifact_evidence: bool,
}

fn default_min_sources() -> u8 {
    DEFAULT_MIN_SOURCES
}

impl FeasibilityProbe {
    pub fn new(topic: &str, domain: FeasibilityDomain) -> Self {
        Self {
            topic: topic.to_string(),
            domain,
            min_sources: DEFAULT_MIN_SOURCES,
            require_artifact_evidence: false,
        }
    }

    pub fn validate(&self) -> Result<(), FeasibilityError> {
        if self.topic.trim().is_empty() {
            return Err(FeasibilityError::EmptyTopic);
        }
        if self.min_sources == 0 {
            return Err(FeasibilityError::MinSourcesZero);
        }
        Ok(())
    }
}

/// RFC 10 §11.1 — the probe verdict handed to Prompt Understanding.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeasibilityReport {
    pub probe_id: String,
    pub topic: String,
    pub found: bool,
    #[serde(default)]
    pub artifact_evidence: Vec<ArtifactEvidence>,
    pub confidence: f32,
    #[serde(default)]
    pub red_flags: Vec<String>,
    #[serde(default)]
    pub recommended_next_step: String,
}

impl FeasibilityReport {
    pub fn validate(&self) -> Result<(), FeasibilityError> {
        if self.probe_id.trim().is_empty() {
            return Err(FeasibilityError::EmptyProbeId);
        }
        if self.topic.trim().is_empty() {
            return Err(FeasibilityError::EmptyTopic);
        }
        if !self.confidence.is_finite() || self.confidence < 0.0 || self.confidence > 1.0 {
            return Err(FeasibilityError::ConfidenceOutOfRange {
                value: self.confidence,
            });
        }
        for e in &self.artifact_evidence {
            e.validate()?;
        }
        if self.found && !self.red_flags.is_empty() {
            return Err(FeasibilityError::FoundWithRedFlags);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum FeasibilityError {
    #[error("feasibility probe topic must not be empty")]
    EmptyTopic,
    #[error("feasibility probe min_sources must be >= 1")]
    MinSourcesZero,
    #[error("feasibility probe id must not be empty")]
    EmptyProbeId,
    #[error("feasibility evidence url must not be empty")]
    EmptyUrl,
    #[error("feasibility evidence url must be http(s): {url}")]
    NonHttpUrl { url: String },
    #[error("confidence {value} out of range (expected 0.0..=1.0)")]
    ConfidenceOutOfRange { value: f32 },
    #[error("found=true with red_flags is a gate violation (RFC 10 §11.3 rule 3)")]
    FoundWithRedFlags,
    #[error("unknown feasibility domain: {0}")]
    UnknownDomain(String),
}

/// Mint a `fp-YYYYMMDD-<8hex>` probe id.
pub fn mint_probe_id() -> String {
    let date = chrono::Utc::now().format("%Y%m%d");
    let short = uuid::Uuid::new_v4().simple().to_string();
    format!("fp-{date}-{}", &short[..8])
}

/// Parse a CLI `--domain hardware,vendor,academic` value. Empty input means
/// all four domains. Unknown names are an error, never silently dropped.
pub fn parse_domains(raw: &str) -> Result<Vec<FeasibilityDomain>, FeasibilityError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(FeasibilityDomain::ALL.to_vec());
    }
    let mut out = Vec::new();
    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match FeasibilityDomain::parse(part) {
            Some(d) => {
                if !out.contains(&d) {
                    out.push(d);
                }
            }
            None => return Err(FeasibilityError::UnknownDomain(part.to_string())),
        }
    }
    if out.is_empty() {
        return Ok(FeasibilityDomain::ALL.to_vec());
    }
    Ok(out)
}

/// Normalised cache key: lowercased topic + sorted domain tags.
pub fn cache_key(topic: &str, domains: &[FeasibilityDomain]) -> String {
    let mut tags: Vec<&str> = domains.iter().map(|d| d.as_str()).collect();
    tags.sort_unstable();
    format!("{}|{}", topic.trim().to_ascii_lowercase(), tags.join(","))
}

/// RFC 10 §11.6: a cached report is reusable while younger than 12 days.
pub fn is_cache_fresh(cached_at_rfc3339: &str, now_rfc3339: &str) -> bool {
    let Ok(cached) = chrono::DateTime::parse_from_rfc3339(cached_at_rfc3339) else {
        return false;
    };
    let Ok(now) = chrono::DateTime::parse_from_rfc3339(now_rfc3339) else {
        return false;
    };
    let age = now.signed_duration_since(cached).num_seconds();
    (0..=CACHE_TTL_SECS).contains(&age)
}

/// Count distinct evidence URLs (first occurrence wins, blanks dropped).
pub fn distinct_count(evidence: &[ArtifactEvidence]) -> usize {
    let mut seen: Vec<&str> = Vec::new();
    for e in evidence {
        let url = e.url.trim();
        if url.is_empty() || seen.contains(&url) {
            continue;
        }
        seen.push(url);
    }
    seen.len()
}

/// RFC 10 §11.3 + §11.5: derive every red flag for the evidence set.
/// `probed` lists the domains the probe fanned out to (drives the academic
/// "no results" flag); `min_sources` is the gate floor.
pub fn detect_red_flags(
    evidence: &[ArtifactEvidence],
    probed: &[FeasibilityDomain],
    min_sources: u8,
) -> Vec<String> {
    let mut flags: Vec<String> = Vec::new();
    let distinct = distinct_count(evidence);
    if evidence.is_empty() {
        flags.push("no artifact evidence".to_string());
    }
    if distinct < min_sources as usize {
        flags.push(format!(
            "below min_sources ({distinct} < {min_sources}): no artifact evidence"
        ));
    }
    if !evidence.is_empty() && !evidence.iter().any(|e| e.kind.is_primary()) {
        flags.push("no primary source".to_string());
    }
    if probed.contains(&FeasibilityDomain::Academic)
        && !evidence.iter().any(|e| e.kind == ArtifactKind::Paper)
    {
        flags.push("no academic source".to_string());
    }
    let now = chrono::Utc::now();
    for e in evidence {
        if let Some(last) = e.last_release_at.as_deref() {
            if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(last) {
                if now.signed_duration_since(ts).num_days() > ABANDONED_DAYS {
                    flags.push(format!("abandoned: no release in 12 months ({})", e.url));
                }
            }
        }
    }
    flags.sort();
    flags.dedup();
    flags
}

fn confidence_for(
    distinct: usize,
    min_sources: u8,
    has_primary: bool,
    red_flags: &[String],
) -> f32 {
    if distinct == 0 || min_sources == 0 {
        return 0.0;
    }
    let ratio = (distinct as f32 / min_sources as f32).min(1.0);
    let mut confidence = ratio * if has_primary { 0.75 } else { 0.35 };
    if !red_flags.is_empty() {
        confidence = confidence.min(0.39);
    }
    confidence.clamp(0.0, 0.95)
}

fn next_step_for(
    topic: &str,
    found: bool,
    confidence: f32,
    evidence_count: usize,
    red_flags: &[String],
) -> String {
    if found {
        format!(
            "confirmar con el usuario; pídele que especifique marca/modelo/versión de '{topic}' \
             antes de construir el pipeline ({evidence_count} evidencias, confianza {confidence:.2})"
        )
    } else {
        let flags = if red_flags.is_empty() {
            "sin evidencia suficiente".to_string()
        } else {
            red_flags.join("; ")
        };
        format!(
            "bloquear el avance y pedir confirmación del usuario (RFC 23 §5) \
             antes de creer que '{topic}' existe: {flags}. Sin evidencia no hay output (RFC 10 §10 fail-safe)."
        )
    }
}

/// RFC 10 §11.3 gate: fold one merged evidence set into the verdict.
/// `probed` is the full domain fan-out (for the academic flag), `probe_id`
/// identifies the report. Deterministic: same inputs, same report.
pub fn evaluate(
    probe_id: &str,
    topic: &str,
    evidence: &[ArtifactEvidence],
    probed: &[FeasibilityDomain],
    min_sources: u8,
) -> FeasibilityReport {
    let mut deduped: Vec<ArtifactEvidence> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for e in evidence {
        let url = e.url.trim().to_string();
        if url.is_empty() || seen.contains(&url) {
            continue;
        }
        seen.push(url.clone());
        let mut one = e.clone();
        one.url = url;
        deduped.push(one);
    }
    deduped.sort_by(|a, b| a.url.cmp(&b.url));
    let distinct = deduped.len();
    let has_primary = deduped.iter().any(|e| e.kind.is_primary());
    let red_flags = detect_red_flags(&deduped, probed, min_sources);
    let found = distinct >= min_sources.max(1) as usize && has_primary && red_flags.is_empty();
    let confidence = confidence_for(distinct, min_sources.max(1), has_primary, &red_flags);
    let recommended_next_step = next_step_for(topic, found, confidence, distinct, &red_flags);
    FeasibilityReport {
        probe_id: probe_id.to_string(),
        topic: topic.to_string(),
        found,
        artifact_evidence: deduped,
        confidence,
        red_flags,
        recommended_next_step,
    }
}

/// RFC 10 §10 fail-safe for probes: only a found report at or above the
/// collective floor completes; anything else needs a human.
pub fn fail_safe_status(report: &FeasibilityReport) -> super::report::ResearchRunStatus {
    if report.found
        && report.confidence.is_finite()
        && report.confidence as f64 >= super::collective::FAIL_SAFE_CONFIDENCE
    {
        super::report::ResearchRunStatus::Completed
    } else {
        super::report::ResearchRunStatus::NeedingHuman
    }
}

/// RFC 10 §11.7 — probe telemetry: hit flag, red-flag count, evidence count,
/// wall-clock milliseconds. The CLI logs one line per probe run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProbeMetrics {
    pub hit: bool,
    pub red_flag_count: usize,
    pub evidence_count: usize,
    pub elapsed_ms: u64,
}

pub fn metrics_for(report: &FeasibilityReport, elapsed_ms: u64) -> ProbeMetrics {
    ProbeMetrics {
        hit: report.found,
        red_flag_count: report.red_flags.len(),
        evidence_count: report.artifact_evidence.len(),
        elapsed_ms,
    }
}

/// Run the live probe: fan out one collector per domain in parallel, merge,
/// then `evaluate`. Every collector degrades to `vec![]` — a fully offline
/// host yields `found=false` with red flags, never an error.
pub async fn probe_feasibility(
    probe_id: &str,
    topic: &str,
    domains: &[FeasibilityDomain],
    min_sources: u8,
    limit: u8,
) -> FeasibilityReport {
    let topic = topic.trim();
    let limit = limit.clamp(1, COLLECT_MAX_RESULTS);
    let mut sets: Vec<Vec<ArtifactEvidence>> = Vec::new();
    if domains.contains(&FeasibilityDomain::Software) {
        sets.push(collect_software(topic, limit).await);
    }
    if domains.contains(&FeasibilityDomain::Hardware) {
        sets.push(collect_hardware(topic, limit).await);
    }
    if domains.contains(&FeasibilityDomain::Academic) {
        sets.push(collect_academic(topic, limit).await);
    }
    if domains.contains(&FeasibilityDomain::Vendor) {
        sets.push(collect_vendor(topic, limit).await);
    }
    let merged: Vec<ArtifactEvidence> = sets.into_iter().flatten().collect();
    evaluate(probe_id, topic, &merged, domains, min_sources.max(1))
}

fn http_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(COLLECT_TIMEOUT_SECS))
        .user_agent("atlas-os/0.1 (research feasibility)")
        .build()
        .ok()
}

fn slug_candidate(topic: &str) -> Option<String> {
    let t = topic.trim().to_ascii_lowercase();
    if t.is_empty() || t.contains(' ') || t.contains("://") {
        return None;
    }
    if t.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
    {
        Some(t)
    } else {
        None
    }
}

fn first_http_url(topic: &str) -> Option<String> {
    topic
        .split_whitespace()
        .map(str::trim)
        .map(|w| {
            w.trim_matches(|c| {
                c == '"' || c == '\'' || c == '(' || c == ')' || c == ',' || c == '.'
            })
        })
        .find(|w| w.starts_with("http://") || w.starts_with("https://"))
        .map(str::to_string)
}

/// RFC 10 §11.2 Software: npm registry, crates.io, PyPI, plus GitHub REST
/// `GET /repos/{o}/{r}` when the topic names an `owner/repo`.
pub async fn collect_software(topic: &str, limit: u8) -> Vec<ArtifactEvidence> {
    let mut out = Vec::new();
    let Some(client) = http_client() else {
        return out;
    };
    if let Some(slug) = slug_candidate(topic) {
        let name = slug.rsplit('/').next().unwrap_or(&slug);
        let npm = client
            .get(format!("https://registry.npmjs.org/{name}"))
            .header("Accept", "application/json")
            .send();
        let crates = client
            .get(format!("https://crates.io/api/v1/crates/{name}"))
            .header("Accept", "application/json")
            .send();
        let pypi = client
            .get(format!("https://pypi.org/pypi/{name}/json"))
            .header("Accept", "application/json")
            .send();
        let (npm, crates, pypi) = tokio::join!(npm, crates, pypi);
        if let Ok(r) = npm {
            if r.status().is_success() {
                let mut e = ArtifactEvidence::new(
                    ArtifactKind::Package,
                    format!("https://www.npmjs.com/package/{name}").as_str(),
                );
                if let Ok(v) = r.json::<serde_json::Value>().await {
                    e.raw_metadata = v;
                }
                out.push(e);
            }
        }
        if let Ok(r) = crates {
            if r.status().is_success() {
                let mut e = ArtifactEvidence::new(
                    ArtifactKind::Package,
                    format!("https://crates.io/crates/{name}").as_str(),
                );
                if let Ok(v) = r.json::<serde_json::Value>().await {
                    e.last_release_at = v
                        .pointer("/crate/updated_at")
                        .and_then(|u| u.as_str())
                        .map(str::to_string);
                    e.raw_metadata = v;
                }
                out.push(e);
            }
        }
        if let Ok(r) = pypi {
            if r.status().is_success() {
                let mut e = ArtifactEvidence::new(
                    ArtifactKind::Package,
                    format!("https://pypi.org/project/{name}/").as_str(),
                );
                if let Ok(v) = r.json::<serde_json::Value>().await {
                    e.raw_metadata = v;
                }
                out.push(e);
            }
        }
    }
    if topic.contains('/') {
        out.extend(collect_github_repo(topic, &client).await);
    }
    out.truncate(limit as usize);
    out
}

async fn collect_github_repo(topic: &str, client: &reqwest::Client) -> Vec<ArtifactEvidence> {
    let shape = slug_candidate(topic).unwrap_or_default();
    let mut parts = shape.split('/').filter(|p| !p.is_empty());
    let (Some(owner), Some(repo)) = (parts.next(), parts.next()) else {
        return Vec::new();
    };
    if parts.next().is_some() {
        return Vec::new();
    }
    let url = format!("https://api.github.com/repos/{owner}/{repo}");
    let run = async {
        let v: serde_json::Value = client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "atlas-os/0.1 (research feasibility)")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok::<serde_json::Value, anyhow::Error>(v)
    };
    match tokio::time::timeout(std::time::Duration::from_secs(COLLECT_TIMEOUT_SECS), run).await {
        Ok(Ok(v)) => {
            let mut e = ArtifactEvidence::new(
                ArtifactKind::GithubRepo,
                format!("https://github.com/{owner}/{repo}").as_str(),
            );
            e.stars_or_stargazers = v.get("stargazers_count").and_then(|s| s.as_u64());
            e.last_release_at = v
                .get("pushed_at")
                .and_then(|p| p.as_str())
                .map(str::to_string);
            e.raw_metadata = v;
            vec![e]
        }
        _ => Vec::new(),
    }
}

/// RFC 10 §11.2 Hardware: vendor doc URLs — HTTP fetch with `<title>`
/// extraction plus link-rot detection (non-2xx yields no evidence).
pub async fn collect_hardware(topic: &str, limit: u8) -> Vec<ArtifactEvidence> {
    let mut out = Vec::new();
    let Some(url) = first_http_url(topic) else {
        return out;
    };
    if let Some(e) = fetch_vendor_page(&url, ArtifactKind::VendorProduct).await {
        out.push(e);
    }
    out.truncate(limit as usize);
    out
}

/// RFC 10 §11.2 Academic: arXiv `/abs/<id>` direct links plus an API query
/// over the topic terms (SECTOR C: no dedicated MCP in this iteration).
pub async fn collect_academic(topic: &str, limit: u8) -> Vec<ArtifactEvidence> {
    let mut out = Vec::new();
    for word in topic.split_whitespace() {
        let w = word.trim_matches(|c| {
            c == '"' || c == '\'' || c == '(' || c == ')' || c == ',' || c == '.'
        });
        if w.contains("arxiv.org/abs/") {
            let id = w
                .rsplit("arxiv.org/abs/")
                .next()
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(['.', ',', ')']);
            if !id.is_empty() {
                out.push(ArtifactEvidence::new(
                    ArtifactKind::Paper,
                    format!("https://arxiv.org/abs/{id}").as_str(),
                ));
            }
        }
    }
    let terms: Vec<String> = topic
        .split(|c: char| !c.is_alphanumeric())
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .take(5)
        .map(|w| format!("all:{w}"))
        .collect();
    if terms.is_empty() {
        out.truncate(limit as usize);
        return out;
    }
    let search = terms.join("+AND+");
    let run = async {
        let Some(client) = http_client() else {
            return Ok::<Vec<ArtifactEvidence>, anyhow::Error>(Vec::new());
        };
        let body = client
            .get(super::collective::ARXIV_API)
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
        Ok(parse_arxiv_feed(&body, limit))
    };
    if let Ok(Ok(hits)) = tokio::time::timeout(
        std::time::Duration::from_secs(COLLECT_TIMEOUT_SECS + 2),
        run,
    )
    .await
    {
        out.extend(hits);
    }
    out.truncate(limit as usize);
    out
}

fn parse_arxiv_feed(body: &str, limit: u8) -> Vec<ArtifactEvidence> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("<id>") {
        let after = &rest[start + 4..];
        let Some(end) = after.find("</id>") else {
            break;
        };
        let id = after[..end].trim();
        if (id.starts_with("http://") || id.starts_with("https://"))
            && id.contains("arxiv.org/abs/")
            && out.len() < limit as usize
        {
            out.push(ArtifactEvidence::new(ArtifactKind::Paper, id));
        }
        rest = &after[end + 5..];
    }
    out
}

/// RFC 10 §11.2 Vendor: official site fetch plus a Wayback availability
/// fallback when the live page link-rots.
pub async fn collect_vendor(topic: &str, limit: u8) -> Vec<ArtifactEvidence> {
    let mut out = Vec::new();
    let Some(url) = first_http_url(topic) else {
        return out;
    };
    if let Some(e) = fetch_vendor_page(&url, ArtifactKind::VendorDocs).await {
        out.push(e);
        out.truncate(limit as usize);
        return out;
    }
    let run = async {
        let Some(client) = http_client() else {
            return Ok::<Option<ArtifactEvidence>, anyhow::Error>(None);
        };
        let v: serde_json::Value = client
            .get("https://archive.org/wayback/available")
            .query(&[("url", url.as_str())])
            .header("Accept", "application/json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let snapshot = v
            .pointer("/archived_snapshots/closest/url")
            .and_then(|u| u.as_str())
            .map(str::to_string);
        Ok(snapshot.map(|snap| {
            let mut e = ArtifactEvidence::new(ArtifactKind::VendorDocs, snap.as_str());
            e.raw_metadata = v;
            e
        }))
    };
    if let Ok(Ok(Some(e))) =
        tokio::time::timeout(std::time::Duration::from_secs(COLLECT_TIMEOUT_SECS), run).await
    {
        out.push(e);
    }
    out.truncate(limit as usize);
    out
}

async fn fetch_vendor_page(url: &str, kind: ArtifactKind) -> Option<ArtifactEvidence> {
    let client = http_client()?;
    let run = async {
        let body = client
            .get(url)
            .header("Accept", "text/html")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        Ok::<String, anyhow::Error>(body)
    };
    match tokio::time::timeout(std::time::Duration::from_secs(COLLECT_TIMEOUT_SECS), run).await {
        Ok(Ok(body)) => {
            let mut e = ArtifactEvidence::new(kind, url);
            let title = body
                .find("<title>")
                .and_then(|s| {
                    body[s + 7..]
                        .find("</title>")
                        .map(|end| body[s + 7..s + 7 + end].trim().to_string())
                })
                .filter(|t| !t.is_empty());
            if let Some(t) = title {
                e.raw_metadata = serde_json::json!({ "title": t });
            }
            Some(e)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(kind: ArtifactKind, url: &str) -> ArtifactEvidence {
        ArtifactEvidence::new(kind, url)
    }

    fn three_primaries() -> Vec<ArtifactEvidence> {
        vec![
            evidence(ArtifactKind::VendorProduct, "https://iconbuild.com/vulcan"),
            evidence(ArtifactKind::Paper, "https://arxiv.org/abs/2006.03002"),
            evidence(
                ArtifactKind::GithubRepo,
                "https://github.com/ForkRobotics/foldable-house-printer",
            ),
        ]
    }

    #[test]
    fn gate_passes_on_three_distinct_primaries() {
        let ev = three_primaries();
        let report = evaluate(
            "fp-2026-07-13-00000001",
            "casas con impresión 3D",
            &ev,
            &[
                FeasibilityDomain::Hardware,
                FeasibilityDomain::Vendor,
                FeasibilityDomain::Academic,
            ],
            3,
        );
        assert!(report.found);
        assert!(report.red_flags.is_empty());
        assert!((0.0..=1.0).contains(&report.confidence));
        assert!(report.recommended_next_step.contains("confirmar"));
        report.validate().unwrap();
        assert_eq!(
            fail_safe_status(&report),
            super::super::report::ResearchRunStatus::Completed
        );
    }

    #[test]
    fn galaxy_probe_fails_safe_with_no_evidence() {
        let report = evaluate(
            "fp-2026-07-13-00000002",
            "fly to the galaxy",
            &[],
            &[
                FeasibilityDomain::Hardware,
                FeasibilityDomain::Vendor,
                FeasibilityDomain::Academic,
            ],
            3,
        );
        assert!(!report.found);
        assert_eq!(report.confidence, 0.0);
        assert!(report
            .red_flags
            .iter()
            .any(|f| f.contains("no artifact evidence")));
        assert!(report
            .red_flags
            .iter()
            .any(|f| f.contains("no academic source")));
        assert!(report.recommended_next_step.contains("bloquear"));
        report.validate().unwrap();
        assert_eq!(
            fail_safe_status(&report),
            super::super::report::ResearchRunStatus::NeedingHuman
        );
    }

    #[test]
    fn abandoned_package_is_a_red_flag_despite_stars() {
        let mut e = evidence(
            ArtifactKind::GithubRepo,
            "https://github.com/example/abandoned",
        );
        e.stars_or_stargazers = Some(9000);
        e.last_release_at = Some("2018-03-01T00:00:00+00:00".to_string());
        let report = evaluate(
            "fp-2026-07-13-00000003",
            "example/abandoned",
            &[e],
            &[FeasibilityDomain::Software],
            1,
        );
        assert!(!report.found);
        assert!(report.red_flags.iter().any(|f| f.starts_with("abandoned")));
        report.validate().unwrap();
    }

    #[test]
    fn docs_only_evidence_has_no_primary_source() {
        let ev = vec![
            evidence(ArtifactKind::VendorDocs, "https://example.com/docs/a"),
            evidence(ArtifactKind::VendorDocs, "https://example.com/docs/b"),
            evidence(ArtifactKind::VendorDocs, "https://example.com/docs/c"),
        ];
        let report = evaluate(
            "fp-2026-07-13-00000004",
            "example docs",
            &ev,
            &[FeasibilityDomain::Vendor],
            3,
        );
        assert!(!report.found);
        assert!(report
            .red_flags
            .iter()
            .any(|f| f.contains("no primary source")));
        report.validate().unwrap();
    }

    #[test]
    fn duplicate_urls_count_once_toward_min_sources() {
        let ev = vec![
            evidence(ArtifactKind::Package, "https://example.com/pkg"),
            evidence(ArtifactKind::Package, "https://example.com/pkg"),
            evidence(ArtifactKind::Package, "https://example.com/pkg"),
        ];
        let report = evaluate(
            "fp-2026-07-13-00000005",
            "dup pkg",
            &ev,
            &[FeasibilityDomain::Software],
            3,
        );
        assert!(!report.found);
        assert_eq!(report.artifact_evidence.len(), 1);
    }

    #[test]
    fn parse_domains_covers_cli_shapes() {
        assert_eq!(
            parse_domains("hardware,vendor,academic").unwrap(),
            vec![
                FeasibilityDomain::Hardware,
                FeasibilityDomain::Vendor,
                FeasibilityDomain::Academic
            ]
        );
        assert_eq!(parse_domains("").unwrap(), FeasibilityDomain::ALL.to_vec());
        assert_eq!(
            FeasibilityDomain::parse("Software"),
            Some(FeasibilityDomain::Software)
        );
        assert!(parse_domains("quantum").is_err());
        assert!(parse_domains("software,quantum").is_err());
    }

    #[test]
    fn cache_ttl_is_twelve_days() {
        assert_eq!(CACHE_TTL_SECS, 12 * 24 * 3600);
        assert!(is_cache_fresh(
            "2026-07-13T14:02:00+00:00",
            "2026-07-20T14:02:00+00:00"
        ));
        assert!(!is_cache_fresh(
            "2026-07-01T14:02:00+00:00",
            "2026-07-13T14:02:01+00:00"
        ));
        assert!(!is_cache_fresh("not-a-date", "2026-07-13T14:02:00+00:00"));
        assert!(!is_cache_fresh(
            "2026-07-13T14:02:00+00:00",
            "2026-07-01T14:02:00+00:00"
        ));
    }

    #[test]
    fn cache_key_normalises_topic_and_sorts_domains() {
        let a = cache_key(
            "  Casas con Impresión 3D ",
            &[FeasibilityDomain::Vendor, FeasibilityDomain::Hardware],
        );
        let b = cache_key(
            "casas con impresión 3d",
            &[FeasibilityDomain::Hardware, FeasibilityDomain::Vendor],
        );
        assert_eq!(a, b);
    }

    #[test]
    fn report_roundtrips_through_json() {
        let report = evaluate(
            "fp-2026-07-13-00000006",
            "casas con impresión 3D",
            &three_primaries(),
            &[FeasibilityDomain::Hardware],
            3,
        );
        let json = serde_json::to_string(&report).unwrap();
        let back: FeasibilityReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, back);
    }

    #[test]
    fn arxiv_feed_parser_skips_query_level_ids() {
        let body = "<feed><id>http://arxiv.org/api/query?search_query=all:galaxy</id>\
             <entry><id>http://arxiv.org/abs/2401.00001v1</id></entry>\
             <entry><id>https://arxiv.org/abs/2401.00002</id></entry></feed>";
        let hits = parse_arxiv_feed(body, 5);
        assert_eq!(hits.len(), 2);
        assert!(hits.iter().all(|h| h.kind == ArtifactKind::Paper));
        assert!(hits.iter().all(|h| h.url.contains("/abs/")));
    }

    #[test]
    fn validation_rejects_found_with_red_flags() {
        let mut report = evaluate(
            "fp-2026-07-13-00000007",
            "x",
            &[],
            &[FeasibilityDomain::Software],
            3,
        );
        report.found = true;
        assert_eq!(
            report.validate().unwrap_err(),
            FeasibilityError::FoundWithRedFlags
        );
    }

    #[test]
    fn probe_validation_rejects_empty_topic_and_zero_floor() {
        let mut p = FeasibilityProbe::new("tokio", FeasibilityDomain::Software);
        p.validate().unwrap();
        p.topic = "   ".into();
        assert_eq!(p.validate().unwrap_err(), FeasibilityError::EmptyTopic);
        let mut p = FeasibilityProbe::new("tokio", FeasibilityDomain::Software);
        p.min_sources = 0;
        assert_eq!(p.validate().unwrap_err(), FeasibilityError::MinSourcesZero);
    }
}
