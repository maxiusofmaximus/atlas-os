// Atlas OS — Skill loader (RFC 06).
// Phase 1: SkillGraph + manifest parser + conflict index + top-K
// candidate selection. Phase 2 añade embeddings (sqlite-vec), la
// compresión de skills (RFC 06 §5) y la generación automática
// (RFC 06 §7) vía el Learning Engine.

pub mod bundled;
#[cfg(feature = "dag_mode")]
pub mod graph_loader;
pub mod manifest;
pub mod marketplace;
pub mod picker;
pub mod remix;
pub mod sdk;

pub use bundled::{load_bundled, BUNDLED_COUNT};
pub use manifest::{load_skill, Engine, SkillGraph, SkillManifest};
pub use marketplace::{install_skill, publish_skill, InstalledSkill};
pub use picker::{pick_skills, ScoredSkill, DEFAULT_PICK_THRESHOLD};
pub use remix::{fork_skill, REMIX_TEMPLATE_VERSION};
pub use sdk::{parse_engine, scaffold_skill, validate_skill_id};
