// Atlas OS — LSP host (RFC 25 §3.6, RFC 21 §2.3).
// Phase 0: skeleton placeholder; real Tower-LSP server live in Roadmap Fase 2.

pub mod confidence;
pub mod host;

pub use confidence::{
    affects_where, diagnostic_for_symbol, hover_for_symbol, who_owns, SymbolConfidence,
};
pub use host::serve;
