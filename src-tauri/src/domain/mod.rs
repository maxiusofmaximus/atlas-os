// Atlas OS — Domain Engines & Vertical Harnesses (RFC 64, Fase 28).
//
// A *domain pack* is a declarative bundle (not core code): engine routing,
// bundled skills, MCP servers, lateral tools and validation/artifact policy.
// The core stays generic; a domain connects through this contract. Tools are
// always *lateral* (external process, never bundled — RFC 25 §11).

pub mod lateral;
pub mod manifest;
pub mod registry;

pub use manifest::{pack_sha256, parse_manifest, DomainPack};
pub use registry::DomainRegistry;

use crate::coding::types::Diff;

/// RFC 64 §8 — validation commands declared by the pack that best matches
/// `signals`, using the embedded seed registry. Empty when no pack matches or
/// the pack declares no commands.
pub fn commands_for_signals(signals: &[String]) -> Vec<String> {
    commands_for_signals_in(&DomainRegistry::builtin(), signals)
}

/// Same as [`commands_for_signals`] against a caller-supplied registry
/// (operator packs included). Kept separate so it is testable with a fixture.
pub fn commands_for_signals_in(reg: &DomainRegistry, signals: &[String]) -> Vec<String> {
    reg.resolve_pack(signals)
        .map(|p| p.validation.commands.clone())
        .unwrap_or_default()
}

/// Project-Map-lite signals for a `Diff`: the paths it touched. Used by the
/// Validation bridge to activate a domain pack's domain checks (RFC 64 §8).
pub fn commands_for_diff(diff: &Diff) -> Vec<String> {
    let signals: Vec<String> = diff
        .files
        .iter()
        .map(|f| f.path.replace('\\', "/").to_lowercase())
        .collect();
    commands_for_signals(&signals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_packs_declare_no_commands_by_default() {
        assert!(commands_for_signals(&["model.step".to_string()]).is_empty());
        assert!(commands_for_signals(&["src/lib.rs".to_string()]).is_empty());
    }

    #[test]
    fn operator_pack_commands_resolve_by_signal() {
        let dir = tempfile::TempDir::new().unwrap();
        let pack_dir = dir.path().join("domains").join("cad");
        std::fs::create_dir_all(&pack_dir).unwrap();
        std::fs::write(
            pack_dir.join("domain.toml"),
            "[domain]\nid=\"cad\"\ntitle=\"CAD\"\ndetect=[\"*.step\"]\n\
             [validation]\ncommands=[\"openscad --check model.step\"]\n",
        )
        .unwrap();
        let reg = DomainRegistry::load(dir.path());
        let cmds = commands_for_signals_in(&reg, &["model.step".to_string()]);
        assert_eq!(cmds, vec!["openscad --check model.step"]);
        assert!(commands_for_signals_in(&reg, &["src/lib.rs".to_string()]).is_empty());
    }
}
