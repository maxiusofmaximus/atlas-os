// Atlas OS — Research Engine module (RFC 10, Phase 3 sub-fase 3.0–3.3).
//
// Foundation plus the 3.1 docs gateway (`docs_gateway::DocsGateway`,
// the Context7Max adapter over `ATLAS_CTX7MAX_URL` / `ctx7max` CLI with
// Context7 MCP + official-docs fallback), the 3.2 document ingester
// (`ingest::ingest_file`, dependency-free by anydoc audit decision —
// gated behind the `doc-ingest` Cargo feature, default off), and the 3.3
// collective intelligence (`collective::score_all` + the four dimension
// scorers + fail-safe + `atlas research query` report builder). Still
// pending on top of these types: hands-on branches 3.4, feasibility
// probe 3.5.

pub mod collective;
pub mod consensus;
pub mod docs_gateway;
#[cfg(feature = "doc-ingest")]
pub mod ingest;
pub mod report;

pub use collective::{
    apply_fail_safe, build_report, classify_source_kind, collect_arxiv, collect_github,
    collect_official_docs, gather_evidence, hands_on_weight, leading_host, prefilter_sources,
    score_all, top_reference, AcademicScorer, CommunityScorer, DimensionOutcome, EnterpriseScorer,
    GatherOptions, OfficialScorer, ReportSections, ARXIV_API, FAIL_SAFE_CONFIDENCE, GH_BIN,
    HANDS_ON_KIND, HANDS_ON_WEIGHT,
};
pub use consensus::{
    combined_confidence, ConsensusDimension, ConsensusError, ConsensusScore, ConsensusScorer,
    SourceInput, MIN_SOURCES,
};
pub use docs_gateway::{DocSnippet, DocsBackend, DocsGateway, DocsGatewayError};
#[cfg(feature = "doc-ingest")]
pub use ingest::{DocFormat, IngestError, IngestedDocument, DOCUMENT_KIND};
pub use report::{
    ConsensusEntry, ReportError, ResearchConsensus, ResearchRunKind, ResearchRunReport,
    ResearchRunStatus,
};
