# ADR 0002 — SQLite + sqlite-vec as the journal and vector store

- **Status**: Accepted
- **Date**: 2026-07-20
- **Decision owner**: RFC 01 §4, RFC 25 §3.5
- **Supersedes**: —
- **Superseded by**: —

## Context

Atlas OS needs a local, embedded durable store for:

- Append-only audit log (RFC 24 §10) with hash-chained rows.
- Kernel Bus event journal (RFC 02 §3.1) — at-least-once delivery with
  `idempotency_key`.
- Missions, plans, approvals, agent heartbeats — the kernel's working state.
- Vector index of embeddings (RFC 22 §3 Nearest-Prior-Work recall).

The store must be:

- Single-file embedded (no separate server, RFC 25 §11).
- Cross-platform, 32/64-bit Windows/macOS/Linux.
- Reachable from Rust without Python (RFC 25 §11).
- Capable of 90th percentile <5 ms inserts for journal writes (RFC 19).

## Decision

Use **SQLite** via `rusqlite` (bundled, with `chrono`, `serde_json`,
`load_extension` features) and **`sqlite-vec`** for the vector index.
Embeddings are produced by `fastembed-rs` (optional feature, off by default
because `ort-sys` triggers a rustc 1.96 ICE — see RFC 25 §3.6 footnotes).

Schema lives in `src-tauri/src/journal/schema.rs` and migrations are run on
every `Journal::open` (Phase 0 uses `CREATE TABLE IF NOT EXISTS`; Phase 6
switches to versioned migrations per RFC 24 §10).

## Consequences

- **No network store**: Postgres/Redis/DynamoDB are explicitly excluded
  (RFC 25 §1). Profiles sync is a Phase 7+ concern (RFC 21) and will use
  BRAVE-style CRDT replication on top of SQLite, not a separate DB.
- **Vector search**: `sqlite-vec`'s virtual table `vec_items` stores
  `fastembed-rs` ONNX embeddings. Queries are KNN on the embedding column.
- **WAL mode**: enabled on every connection (`journalist = WAL`); this is
  what allows the HUD to read while the kernel writes.
- **Idempotency**: `journal_events.idempotency_key UNIQUE` means replays
  silently drop (RFC 02 §3.1.2).

## Alternatives rejected

- **Postgres + pgvector**: server-based, breaks single-binary.
- **redb / sled**: pure-Rust, but lacks a vector index companion and SQL
  tooling — and ecosystem momentum is on SQLite.
- **DuckDB**: OLAP-shaped; we need single-row append-heavy workloads.
- **LMDB**: no SQL, and we want ergonomic queries.
