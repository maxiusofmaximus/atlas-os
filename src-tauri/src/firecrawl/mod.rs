// Atlas OS — Firecrawl adapter facade (RFC 28 Section E).
//
// Web ingestion is polyfacetic: the same Firecrawl SDK serves several
// heterogeneous consumers (the `atlas research` CLI, the graphify
// ingest step, future MCP server surface, the eventual `webfetch`
// fallback in the orchestrator). All callers go through this single
// module so credentials, retry, redaction, and option shimming live in
// one place. The upstream `firecrawl = "2.12.1"` crate is never
// imported outside `client.rs` — this facade is the only surface
// the rest of Atlas OS touches.
//
// Uses the official Rust SDK firecrawl = "2.12.1" published by the
// Firecrawl team (MIT). Copyright Mendable AI Inc.
//
// The module is gated behind the `firecrawl` feature flag (default OFF,
// RFC 25 §11 — pure Rust, single-binary safe). A default build does
// not pull `firecrawl` into the dep tree at all.

pub mod client;
pub mod error;
pub mod facade;

pub use client::{FirecrawlClient, FirecrawlEndpoint, FirecrawlKey};
pub use error::FirecrawlFacadeError;
pub use facade::{
    CrawlBatch, CrawlOptions, ExtractOptions, ExtractResult, ScrapeOptions, ScrapedDocument,
    SearchOptions, SearchResult,
};
