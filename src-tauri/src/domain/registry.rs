// Atlas OS — domain pack registry + Capability Resolver hook (RFC 64 §3/§4).
//
// The four seed packs are embedded so a default build is useful with no
// install step. Operator packs under `<profile>/domains/<id>/domain.toml`
// override/extend the seed. `resolve_pack` is the Project Map → Domain Pack
// step (RFC 02 Capability Resolver).

use std::path::Path;

use super::manifest::{parse_manifest, DomainPack};

/// Embedded seed packs (id, manifest). Order is stable for deterministic lists.
const BUILTIN: &[(&str, &str)] = &[
    ("coding", include_str!("packs/coding.toml")),
    ("cad", include_str!("packs/cad.toml")),
    ("game", include_str!("packs/game.toml")),
    ("creative-media", include_str!("packs/creative-media.toml")),
];

#[derive(Clone, Debug, Default)]
pub struct DomainRegistry {
    packs: Vec<DomainPack>,
}

impl DomainRegistry {
    pub fn builtin() -> Self {
        let mut packs = Vec::new();
        for (id, text) in BUILTIN {
            match parse_manifest(text) {
                Ok(p) => packs.push(p),
                Err(e) => {
                    tracing::error!(error = %e, pack = %id, "builtin domain pack failed to parse")
                }
            }
        }
        Self { packs }
    }

    /// Seed packs plus any operator packs under `<profile_root>/domains/`.
    /// A pack with the same id replaces the seed (operator override).
    pub fn load(profile_root: &Path) -> Self {
        let mut reg = Self::builtin();
        let dir = profile_root.join("domains");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return reg;
        };
        for entry in entries.flatten() {
            let manifest = entry.path().join("domain.toml");
            let Ok(text) = std::fs::read_to_string(&manifest) else {
                continue;
            };
            match parse_manifest(&text) {
                Ok(pack) => reg.upsert(pack),
                Err(e) => {
                    tracing::warn!(error = %e, path = %manifest.display(), "skipping invalid domain pack")
                }
            }
        }
        reg
    }

    fn upsert(&mut self, pack: DomainPack) {
        if let Some(slot) = self
            .packs
            .iter_mut()
            .find(|p| p.domain.id == pack.domain.id)
        {
            *slot = pack;
        } else {
            self.packs.push(pack);
        }
    }

    pub fn get(&self, id: &str) -> Option<&DomainPack> {
        self.packs.iter().find(|p| p.domain.id == id)
    }

    pub fn list(&self) -> &[DomainPack] {
        &self.packs
    }

    /// Capability Resolver (RFC 02): pick the pack whose `detect` signals best
    /// match the Project Map signals. Case-insensitive substring either way;
    /// highest match count wins, `None` when nothing matches.
    pub fn resolve_pack(&self, signals: &[String]) -> Option<&DomainPack> {
        let lowered: Vec<String> = signals.iter().map(|s| s.to_lowercase()).collect();
        let mut best: Option<(&DomainPack, usize)> = None;
        for pack in &self.packs {
            let score = pack
                .domain
                .detect
                .iter()
                .filter(|d| {
                    let d = d.to_lowercase();
                    lowered
                        .iter()
                        .any(|s| s.contains(&d) || d.contains(s.as_str()))
                })
                .count();
            if score > 0 && best.is_none_or(|(_, b)| score > b) {
                best = Some((pack, score));
            }
        }
        best.map(|(p, _)| p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_seed_has_the_four_packs() {
        let reg = DomainRegistry::builtin();
        assert!(reg.get("coding").is_some());
        assert!(reg.get("cad").is_some());
        assert!(reg.get("game").is_some());
        assert!(reg.get("creative-media").is_some());
        assert!(reg.list().len() >= 4);
    }

    #[test]
    fn resolve_pack_matches_project_signals() {
        let reg = DomainRegistry::builtin();
        let cad = reg
            .resolve_pack(&["*.step".to_string(), "cadquery".to_string()])
            .expect("cad resolves");
        assert_eq!(cad.domain.id, "cad");
        let mobile = reg.resolve_pack(&["Cargo.toml".to_string(), "*.rs".to_string()]);
        assert_eq!(mobile.map(|p| p.domain.id.as_str()), Some("coding"));
        assert!(reg
            .resolve_pack(&["totally-unknown-signal".to_string()])
            .is_none());
    }

    #[test]
    fn operator_pack_overrides_a_seed() {
        let dir = tempfile::TempDir::new().unwrap();
        let packs = dir.path().join("domains").join("cad");
        std::fs::create_dir_all(&packs).unwrap();
        std::fs::write(
            packs.join("domain.toml"),
            "[domain]\nid=\"cad\"\ntitle=\"CAD (overridden)\"\nversion=\"9.9.9\"\n",
        )
        .unwrap();
        let reg = DomainRegistry::load(dir.path());
        assert_eq!(reg.get("cad").unwrap().domain.title, "CAD (overridden)");
        assert_eq!(reg.get("cad").unwrap().domain.version, "9.9.9");
    }
}
