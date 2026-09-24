// Atlas OS — Swarm coordinator foundation (RFC 05, Phase 4 sub-fase 4.0).
// Roles + worktree isolation + Journal-backed registry. Parallel dispatch
// (pool), mailbox readers, auto-rebase and the Swarm Console land in
// sub-fases 4.1–4.5 (research/31 SECTOR B).

pub mod roles;
pub mod worktrees;

pub use roles::Role;
pub use worktrees::{WorktreeEntry, WorktreeError, WorktreeManager};
