// Atlas OS — Skill SDK scaffold (RFC 06 §1, research/39 Phase 10 sub-fase 10.0, M37).
// Zero-dep: std + serde/toml/anyhow already in the tree (RFC 25 §11).

use std::path::{Path, PathBuf};

use crate::skills::{Engine, SkillManifest};

pub const SKILL_TEMPLATE_VERSION: &str = "0.1.0";
pub const SKILL_MANIFEST_NAME: &str = "skill.toml";
pub const SKILL_README_NAME: &str = "README.md";
pub const SKILL_DOC_NAME: &str = "SKILL.md";

pub fn validate_skill_id(name: &str) -> anyhow::Result<()> {
    let len = name.len();
    if !(2..=64).contains(&len) {
        anyhow::bail!("invalid skill id '{name}': length must be 2..=64 chars (RFC 06 §1)");
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => anyhow::bail!("invalid skill id '{name}': must start with [a-z] (RFC 06 §1)"),
    }
    for c in name.chars() {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_') {
            anyhow::bail!("invalid skill id '{name}': only [a-z0-9-_], lowercase (RFC 06 §1)");
        }
    }
    match name.chars().last() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => Ok(()),
        _ => anyhow::bail!("invalid skill id '{name}': must end with [a-z0-9] (RFC 06 §1)"),
    }
}

pub fn parse_engine(raw: &str) -> anyhow::Result<Engine> {
    match raw.trim().to_lowercase().as_str() {
        "planning" => Ok(Engine::Planning),
        "research" => Ok(Engine::Research),
        "coding" => Ok(Engine::Coding),
        "validation" => Ok(Engine::Validation),
        "security" => Ok(Engine::Security),
        "reasoning" => Ok(Engine::Reasoning),
        "learning" => Ok(Engine::Learning),
        "unspecified" => Ok(Engine::Unspecified),
        other => anyhow::bail!(
            "unknown engine '{other}': expected one of planning|research|coding|validation|security|reasoning|learning|unspecified (RFC 06 §2)"
        ),
    }
}

fn stub_markdown(name: &str, engine_tag: &str) -> String {
    format!(
        "# {name}\n\
         \n\
         > SDK scaffold (RFC 06 §1, Phase 10.0 M37). Lifecycle: draft — edit, verify, then promote.\n\
         > Engine: `{engine_tag}`.\n\
         \n\
         ## What this skill does\n\
         \n\
         TODO: describe what the `{name}` skill does in one paragraph.\n\
         \n\
         ## Inputs\n\
         \n\
         - `input` (string) — TODO: document the inputs.\n\
         \n\
         ## Outputs\n\
         \n\
         TODO: document the outputs (diff proposal, report, checklist).\n\
         \n\
         ## Behavior\n\
         \n\
         1. TODO: step one.\n\
         2. TODO: step two.\n\
         3. Never assume: reproduce or cite evidence before proposing changes.\n\
         \n\
         ## Verify\n\
         \n\
         - [ ] `description` + `summary` rewritten (no TODO left).\n\
         - [ ] `engine` + `priority` reviewed in `skill.toml`.\n\
         - [ ] Set `verified = true` only after X successful runs (RFC 06 §7).\n"
    )
}

pub fn scaffold_skill(
    name: &str,
    engine: Engine,
    skills_dir: &Path,
) -> anyhow::Result<SkillManifest> {
    validate_skill_id(name)?;
    let target: PathBuf = skills_dir.join(name);
    if target.exists() {
        anyhow::bail!(
            "skill '{name}' already exists at {} (duplicate id, RFC 06 §1)",
            target.display()
        );
    }
    std::fs::create_dir_all(&target)
        .map_err(|e| anyhow::anyhow!("creating skill dir {}: {e}", target.display()))?;
    let manifest = SkillManifest {
        id: name.to_owned(),
        version: SKILL_TEMPLATE_VERSION.to_owned(),
        description: format!("TODO: describe what the `{name}` skill does (RFC 06 §1)."),
        engine,
        priority: 50,
        summary: Some(format!("TODO: one-line summary for `{name}`.")),
        confidence: 0.5,
        verified: false,
        auto_generated: false,
        license: Some("MIT".to_owned()),
        ..Default::default()
    };
    let toml_body = toml::to_string_pretty(&manifest)?;
    std::fs::write(target.join(SKILL_MANIFEST_NAME), &toml_body)
        .map_err(|e| anyhow::anyhow!("writing skill manifest {}: {e}", target.display()))?;
    let stub = stub_markdown(name, engine.tag());
    std::fs::write(target.join(SKILL_README_NAME), &stub)
        .map_err(|e| anyhow::anyhow!("writing skill readme {}: {e}", target.display()))?;
    std::fs::write(target.join(SKILL_DOC_NAME), &stub)
        .map_err(|e| anyhow::anyhow!("writing skill doc {}: {e}", target.display()))?;
    let round_trip = crate::skills::load_skill(&target)?;
    if round_trip.id != manifest.id || round_trip.engine != manifest.engine {
        anyhow::bail!("scaffold self-check failed for '{name}': manifest round-trip mismatch");
    }
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_engine_accepts_all_tags_and_rejects_unknown() {
        for (raw, expected) in [
            ("planning", Engine::Planning),
            ("research", Engine::Research),
            ("coding", Engine::Coding),
            ("validation", Engine::Validation),
            ("security", Engine::Security),
            ("reasoning", Engine::Reasoning),
            ("learning", Engine::Learning),
            ("unspecified", Engine::Unspecified),
            ("Coding", Engine::Coding),
            (" SECURITY ", Engine::Security),
        ] {
            assert_eq!(parse_engine(raw).unwrap(), expected, "raw={raw}");
        }
        let err = parse_engine("design").unwrap_err();
        assert!(err.to_string().contains("unknown engine"), "{err}");
    }

    #[test]
    fn invalid_names_rejected() {
        for bad in [
            "",
            "a",
            "A-skill",
            "UPPER",
            "-bad",
            "_bad",
            "bad-",
            "bad_",
            "has space",
            "foo/bar",
            "foo.bar",
            "foo:bar",
            "áccent",
        ] {
            assert!(validate_skill_id(bad).is_err(), "must reject {bad:?}");
        }
        assert!(validate_skill_id(&"a".repeat(65)).is_err());
        for good in ["my-skill", "my_skill", "opencode-fix2", "a1", "x-y_z-9"] {
            assert!(validate_skill_id(good).is_ok(), "must accept {good:?}");
        }
    }

    #[test]
    fn duplicate_scaffold_fails() {
        let tmp = tempfile::tempdir().unwrap();
        scaffold_skill("demo-skill", Engine::Coding, tmp.path()).unwrap();
        let err = scaffold_skill("demo-skill", Engine::Coding, tmp.path()).unwrap_err();
        assert!(err.to_string().contains("already exists"), "{err}");
        let err = scaffold_skill("Bad Name!", Engine::Coding, tmp.path()).unwrap_err();
        assert!(err.to_string().contains("invalid skill id"), "{err}");
    }

    #[test]
    fn scaffold_is_deterministic() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let ma = scaffold_skill("demo-skill", Engine::Research, a.path()).unwrap();
        let mb = scaffold_skill("demo-skill", Engine::Research, b.path()).unwrap();
        assert_eq!(ma.id, mb.id);
        assert_eq!(ma.version, mb.version);
        assert_eq!(ma.engine, mb.engine);
        assert_eq!(ma.priority, mb.priority);
        assert_eq!(ma.verified, mb.verified);
        assert!(!ma.verified);
        assert_eq!(ma.version, SKILL_TEMPLATE_VERSION);
        let ta =
            std::fs::read_to_string(a.path().join("demo-skill").join(SKILL_MANIFEST_NAME)).unwrap();
        let tb =
            std::fs::read_to_string(b.path().join("demo-skill").join(SKILL_MANIFEST_NAME)).unwrap();
        assert_eq!(ta, tb);
        let ra =
            std::fs::read_to_string(a.path().join("demo-skill").join(SKILL_README_NAME)).unwrap();
        let rb =
            std::fs::read_to_string(b.path().join("demo-skill").join(SKILL_README_NAME)).unwrap();
        assert_eq!(ra, rb);
    }

    #[test]
    fn manifest_round_trip_via_load_skill() {
        let tmp = tempfile::tempdir().unwrap();
        let m = scaffold_skill("round-trip", Engine::Security, tmp.path()).unwrap();
        let loaded = crate::skills::load_skill(&tmp.path().join("round-trip")).unwrap();
        assert_eq!(loaded.id, m.id);
        assert_eq!(loaded.version, m.version);
        assert_eq!(loaded.engine, Engine::Security);
        assert_eq!(loaded.priority, 50);
        assert!(!loaded.verified);
        let reparsed: SkillManifest = toml::from_str(
            &std::fs::read_to_string(tmp.path().join("round-trip").join(SKILL_MANIFEST_NAME))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(reparsed.id, "round-trip");
        assert_eq!(reparsed.engine, Engine::Security);
    }
}
