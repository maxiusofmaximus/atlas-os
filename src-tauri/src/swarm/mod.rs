// Atlas OS — Swarm coordinator foundation (RFC 05, Phase 4 sub-fases 4.0–4.2 + 4.4).
// Roles + worktree isolation + Journal-backed registry + parallel pool dispatch
// + auto-rebase post-merge (CN-003). Mailbox readers landed in 4.3; the Swarm
// Console lands in 4.5 (research/31 SECTOR B).

pub mod merge;
pub mod pool;
pub mod presets;
pub mod rebase;
pub mod roles;
pub mod worktrees;

pub use merge::{merge_diffs, MergeConflict, MergeOutcome};
pub use pool::{FileLockRegistry, PoolAgentSpec, PoolConfig, PoolError, PoolOutcome, SwarmRunner};
pub use presets::{bundled_presets, find_preset, Preset, RolePreset};
pub use rebase::{
    abort_rebase, rebase_after_merge, rebase_many, rebase_worktree, RebaseError, RebaseOutcome,
    RebaseResult, RebaseStatus,
};
pub use roles::{ModelSlot, Role};
pub use worktrees::{WorktreeEntry, WorktreeError, WorktreeManager};
