// Atlas OS — Bundled skills catalog (RFC 29 §3.D / RFC 06 §1).
//
// The 12 `opencode-*` manifests plus the 3 Phase-7 `atlas-*` compliance
// skills and the Phase-1.5 `browser-pane` lateral skill under `skills/`
// are compiled into the binary via `include_str!`, single-binary safe per
// RFC 25 §11. The `bundled-skills` feature (default on) controls the
// catalog; builds with `--no-default-features` skip it for server-only
// distributions.
// Discovery of user-installed skills still goes through
// `SkillGraph::scan_directory` (RFC 06 §6).

use crate::skills::{SkillGraph, SkillManifest};

#[cfg(feature = "bundled-skills")]
const BUNDLED_SOURCES: &[&str] = &[
    include_str!("../../../skills/opencode-fix/skill.toml"),
    include_str!("../../../skills/opencode-restart/skill.toml"),
    include_str!("../../../skills/opencode-format/skill.toml"),
    include_str!("../../../skills/opencode-test/skill.toml"),
    include_str!("../../../skills/opencode-refactor-extract-method/skill.toml"),
    include_str!("../../../skills/opencode-spec-show/skill.toml"),
    include_str!("../../../skills/opencode-doc-from-code/skill.toml"),
    include_str!("../../../skills/opencode-changelog-from-commits/skill.toml"),
    include_str!("../../../skills/opencode-research-query/skill.toml"),
    include_str!("../../../skills/opencode-arxiv-lookup/skill.toml"),
    include_str!("../../../skills/opencode-figma-to-code/skill.toml"),
    include_str!("../../../skills/opencode-ui-from-screenshot/skill.toml"),
    include_str!("../../../skills/browser-pane/skill.toml"),
    include_str!("../../../skills/atlas-owasp-check/skill.toml"),
    include_str!("../../../skills/atlas-gdpr-check/skill.toml"),
    include_str!("../../../skills/atlas-hipaa-check/skill.toml"),
];

#[cfg(not(feature = "bundled-skills"))]
const BUNDLED_SOURCES: &[&str] = &[];

pub const BUNDLED_COUNT: usize = 16;

pub fn bundled_manifests() -> anyhow::Result<Vec<SkillManifest>> {
    let mut out = Vec::with_capacity(BUNDLED_SOURCES.len());
    for src in BUNDLED_SOURCES {
        let manifest: SkillManifest = toml::from_str(src)?;
        out.push(manifest);
    }
    Ok(out)
}

pub fn load_bundled(graph: &mut SkillGraph) -> anyhow::Result<usize> {
    let manifests = bundled_manifests()?;
    let loaded = manifests.len();
    for manifest in manifests {
        graph.register(manifest);
    }
    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "bundled-skills")]
    use crate::skills::Engine;

    #[test]
    #[cfg(feature = "bundled-skills")]
    fn bundled_catalog_parses_all_sixteen() {
        let manifests = bundled_manifests().unwrap();
        assert_eq!(manifests.len(), BUNDLED_COUNT);
        let mut ids: Vec<&str> = manifests.iter().map(|m| m.id.as_str()).collect();
        ids.sort_unstable();
        assert_eq!(
            ids,
            vec![
                "atlas-gdpr-check",
                "atlas-hipaa-check",
                "atlas-owasp-check",
                "browser-pane",
                "opencode-arxiv-lookup",
                "opencode-changelog-from-commits",
                "opencode-doc-from-code",
                "opencode-figma-to-code",
                "opencode-fix",
                "opencode-format",
                "opencode-refactor-extract-method",
                "opencode-research-query",
                "opencode-restart",
                "opencode-spec-show",
                "opencode-test",
                "opencode-ui-from-screenshot",
            ]
        );
    }

    #[test]
    #[cfg(feature = "bundled-skills")]
    fn bundled_manifests_carry_full_rfc06_fields() {
        let manifests = bundled_manifests().unwrap();
        for m in &manifests {
            assert!(!m.version.is_empty(), "version set for {}", m.id);
            assert!(!m.description.is_empty(), "description set for {}", m.id);
            assert_ne!(m.engine, Engine::Unspecified, "engine set for {}", m.id);
            assert!(m.priority > 0, "priority set for {}", m.id);
            assert!(m.verified, "bundled skills ship verified for {}", m.id);
            assert!(!m.auto_generated, "bundled skills are curated for {}", m.id);
        }
        let fix = manifests.iter().find(|m| m.id == "opencode-fix").unwrap();
        assert_eq!(fix.engine, Engine::Coding);
        assert_eq!(fix.priority, 90);
        let arxiv = manifests
            .iter()
            .find(|m| m.id == "opencode-arxiv-lookup")
            .unwrap();
        assert_eq!(arxiv.dependencies, vec!["opencode-research-query"]);
    }

    #[test]
    #[cfg(feature = "bundled-skills")]
    fn bundled_ui_pair_conflicts_both_ways_in_graph() {
        let mut graph = SkillGraph::new();
        let loaded = load_bundled(&mut graph).unwrap();
        assert_eq!(loaded, BUNDLED_COUNT);
        assert_eq!(graph.len(), BUNDLED_COUNT);
        assert!(graph.are_in_conflict("opencode-figma-to-code", "opencode-ui-from-screenshot"));
        assert!(graph.are_in_conflict("opencode-ui-from-screenshot", "opencode-figma-to-code"));
        let coding = graph.find_candidates(Some(Engine::Coding), 20);
        assert!(coding.iter().any(|s| s.id == "opencode-fix"));
    }

    #[test]
    #[cfg(feature = "bundled-skills")]
    fn compliance_skills_route_to_security_engine() {
        let manifests = bundled_manifests().unwrap();
        for id in ["atlas-owasp-check", "atlas-gdpr-check", "atlas-hipaa-check"] {
            let m = manifests.iter().find(|m| m.id == id).unwrap();
            assert_eq!(m.engine, Engine::Security, "engine set for {id}");
            assert!(m.priority > 0, "priority set for {id}");
            assert!(m.verified, "compliance skills ship verified for {id}");
            assert!(!m.auto_generated, "compliance skills are curated for {id}");
            let loaded = crate::skills::manifest::load_skill(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../skills")
                    .join(id)
                    .as_path(),
            )
            .unwrap();
            assert_eq!(loaded.id, id);
            assert_eq!(loaded.engine, Engine::Security);
        }
    }

    #[test]
    fn malformed_bundled_source_is_rejected() {
        let bad = "id = 123\nversion = [\"not\", \"semver\"]\n";
        let parsed: Result<SkillManifest, _> = toml::from_str(bad);
        assert!(parsed.is_err());
    }
}
