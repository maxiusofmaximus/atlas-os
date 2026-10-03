// Atlas OS — LSP host (RFC 25 §3.6, RFC 21 §2.3).
// Phase 19: real Tower-LSP server over stdio now ships behind `lsp` (default on).

pub mod confidence;
pub mod host;
pub mod server;

pub use confidence::{
    affects_where, diagnostic_for_symbol, hover_for_symbol, who_owns, SymbolConfidence,
};
pub use host::serve;
pub use server::{atlas_capabilities, hover_markdown, AtlasLspBackend};
