// OpenCode OS — Journal export subsystem (RFC 28 §D).
//
// Three responsibilities:
//   1. `posting` — map an `AuditEntry` to posting's `.posting.yaml` schema and
//      serialize via `serde_yaml`. Format-compatible with darrenburns/posting
//      (Apache-2.0) — no runtime dependency on the posting package itself.
//   2. `retention` — SQLite-driven hook that exports aging audit_log rows to
//      disk before purging them (RFC 27 §3.H Brecha H close).
//   3. Tests for both, including the canonical curl roundtrip ported from
//      `posting/tests/test_curl_export.py` (PT-006).
//
// Attribution: format schema derived from `darrenburns/posting` (Apache-2.0,
// Copyright Darren Burns). See `OpenCode OS/research/28 - portable inventory.md`
// item PT-001 / PT-002 / PT-003 / PT-006.

pub mod posting;
pub mod retention;

pub use posting::{entry_to_posting_yaml, PostingCollection, PostingRequestModel};
pub use retention::{snapshot_entries, ExportError, RetentionConfig, SnapshotResult};
