// Atlas OS — Swarm coordinator foundation (RFC 05, Phase 4 sub-fases 4.0–4.2).
// Roles + worktree isolation + Journal-backed registry + parallel pool dispatch.
// Mailbox readers, auto-rebase and the Swarm Console land in sub-fases 4.3–4.5
// (research/31 SECTOR B).

pub mod pool;
pub mod presets;
pub mod roles;
pub mod worktrees;

pub use pool::{FileLockRegistry, PoolAgentSpec, PoolConfig, PoolError, PoolOutcome, SwarmRunner};
pub use presets::{bundled_presets, find_preset, Preset, RolePreset};
pub use roles::{ModelSlot, Role};
pub use worktrees::{WorktreeEntry, WorktreeError, WorktreeManager};
