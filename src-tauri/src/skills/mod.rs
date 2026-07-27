// OpenCode OS — Skill loader (RFC 06).
// Phase 1: SkillGraph + manifest parser + conflict index + top-K
// candidate selection. Phase 2 añade embeddings (sqlite-vec), la
// compresión de skills (RFC 06 §5) y la generación automática
// (RFC 06 §7) vía el Learning Engine.

#[cfg(feature = "dag_mode")]
pub mod graph_loader;
pub mod manifest;

pub use manifest::{load_skill, Engine, SkillGraph, SkillManifest};
