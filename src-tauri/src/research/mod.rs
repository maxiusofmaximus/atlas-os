// Atlas OS — Research Engine module (RFC 10, Phase 3 sub-fase 3.0).
//
// Foundation only: canonical report types (RFC 10 §7) plus the
// `ConsensusScorer` trait shape (RFC 10 §5). Network-backed pieces
// (docs gateway 3.1, anydoc ingestion 3.2, collective intelligence
// 3.3, hands-on branches 3.4, feasibility probe 3.5) build on top of
// these types without changing them.

pub mod consensus;
pub mod report;

pub use consensus::{
    combined_confidence, ConsensusDimension, ConsensusError, ConsensusScore, ConsensusScorer,
    SourceInput, MIN_SOURCES,
};
pub use report::{
    ConsensusEntry, ReportError, ResearchConsensus, ResearchRunKind, ResearchRunReport,
    ResearchRunStatus,
};
