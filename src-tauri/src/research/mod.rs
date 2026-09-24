// Atlas OS — Research Engine module (RFC 10, Phase 3 sub-fase 3.0).
//
// Foundation plus the 3.1 docs gateway (`docs_gateway::DocsGateway`,
// the Context7Max adapter over `ATLAS_CTX7MAX_URL` / `ctx7max` CLI with
// Context7 MCP + official-docs fallback). Still pending on top of these
// types: anydoc ingestion 3.2, collective intelligence 3.3, hands-on
// branches 3.4, feasibility probe 3.5.

pub mod consensus;
pub mod docs_gateway;
pub mod report;

pub use consensus::{
    combined_confidence, ConsensusDimension, ConsensusError, ConsensusScore, ConsensusScorer,
    SourceInput, MIN_SOURCES,
};
pub use docs_gateway::{DocSnippet, DocsBackend, DocsGateway, DocsGatewayError};
pub use report::{
    ConsensusEntry, ReportError, ResearchConsensus, ResearchRunKind, ResearchRunReport,
    ResearchRunStatus,
};
