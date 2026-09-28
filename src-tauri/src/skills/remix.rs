// Atlas OS — Remixing de Skills (RFC 06 §5/§9, research/39 Phase 10 sub-fase 10.2, M39).
// Zero-dep: std + anyhow/toml/serde already in the tree (RFC 25 §11).
//
// `fork_skill` copia una skill existente bajo una nueva id con provenance
// `remixed_from` (patrón graph/ `Provenance` — cada skill forkeada conserva
// su origen) y lifecycle draft (`verified = false`: el remix debe
// re-verificarse antes de promoverse, RFC 06 §7).

use std::path::{Path, PathBuf};

use crate::skills::{load_skill, validate_skill_id, SkillManifest};

pub const REMIX_TEMPLATE_VERSION: &str = "0.1.0";

fn copy_dir_recursive_skip_signature(src: &Path, dest: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dest)
        .map_err(|e| anyhow::anyhow!("creating fork dest {}: {e}", dest.display()))?;
    for entry in std::fs::read_dir(src)
        .map_err(|e| anyhow::anyhow!("reading skill source {}: {e}", src.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".checksum" {
            continue;
        }
        let from = entry.path();
        let to = dest.join(&name);
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_dir_recursive_skip_signature(&from, &to)?;
        } else if kind.is_file() {
            std::fs::copy(&from, &to).map_err(|e| {
                anyhow::anyhow!("copying {} -> {}: {e}", from.display(), to.display())
            })?;
        }
    }
    Ok(())
}

/// Forkea `src` (directorio de skill con `skill.toml`) como `new_id`
/// dentro de `skills_dir`.
///
/// - `src` debe existir y contener un manifest válido; si no, falla
///   fail-safe sin crear nada (M39: fallo en src inexistente).
/// - `new_id` se valida como skill id (RFC 06 §1) y no debe existir ya
///   en `skills_dir` (duplicado → error).
/// - La copia excluye el sidecar `.checksum` firmado: el fork es una
///   skill nueva sin firma y debe pasar por `atlas skill publish`
///   antes de `install` (RFC 18 §5).
/// - El manifest del fork lleva nueva id + versión `0.1.0`,
///   `remixed_from = <id original>`, `verified = false` (draft) y
///   `home` apuntando al nuevo directorio.
///
/// Devuelve el manifest ya persistido (round-trip verificado).
pub fn fork_skill(src: &Path, new_id: &str, skills_dir: &Path) -> anyhow::Result<SkillManifest> {
    validate_skill_id(new_id)?;
    if !src.is_dir() {
        anyhow::bail!(
            "cannot fork: skill source not found: {} (M39)",
            src.display()
        );
    }
    let original = load_skill(src).map_err(|e| {
        anyhow::anyhow!(
            "cannot fork: invalid skill manifest at {}: {e}",
            src.display()
        )
    })?;
    validate_skill_id(&original.id)?;
    let dest: PathBuf = skills_dir.join(new_id);
    if dest.exists() {
        anyhow::bail!(
            "skill '{new_id}' already exists at {} (duplicate id, RFC 06 §1)",
            dest.display()
        );
    }
    if src == dest || src.starts_with(&dest) || dest.starts_with(src) {
        anyhow::bail!(
            "refusing to fork skill onto itself: {} -> {}",
            src.display(),
            dest.display()
        );
    }
    copy_dir_recursive_skip_signature(src, &dest)?;
    let forked = (|| -> anyhow::Result<SkillManifest> {
        let mut manifest = load_skill(&dest)?;
        manifest.id = new_id.to_owned();
        manifest.version = REMIX_TEMPLATE_VERSION.to_owned();
        manifest.remixed_from = Some(original.id.clone());
        manifest.verified = false;
        manifest.home = Some(dest.to_string_lossy().into());
        let toml_body = toml::to_string_pretty(&manifest)?;
        std::fs::write(dest.join("skill.toml"), &toml_body)
            .map_err(|e| anyhow::anyhow!("writing forked manifest {}: {e}", dest.display()))?;
        let round_trip = load_skill(&dest)?;
        if round_trip.id != new_id
            || round_trip.remixed_from.as_deref() != Some(original.id.as_str())
        {
            anyhow::bail!("fork self-check failed for '{new_id}': manifest round-trip mismatch");
        }
        Ok(round_trip)
    })();
    if forked.is_err() {
        let _ = std::fs::remove_dir_all(&dest);
    }
    forked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::Engine;

    fn seed_skill(dir: &Path, id: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let mut m = SkillManifest {
            id: id.to_owned(),
            version: "1.4.2".to_owned(),
            description: format!("{id} skill"),
            engine: Engine::Coding,
            priority: 90,
            verified: true,
            ..Default::default()
        };
        m.summary = Some(format!("{id} summary"));
        std::fs::write(dir.join("skill.toml"), toml::to_string_pretty(&m).unwrap()).unwrap();
        std::fs::write(dir.join("SKILL.md"), format!("# {id}\n\nBody.\n")).unwrap();
        std::fs::write(dir.join("README.md"), format!("# {id}\n")).unwrap();
    }

    #[test]
    fn fork_is_deterministic_and_keeps_provenance() {
        let root = tempfile::tempdir().unwrap();
        let src = root.path().join("src-skill");
        seed_skill(&src, "src-skill");
        let out_a = tempfile::tempdir().unwrap();
        let out_b = tempfile::tempdir().unwrap();
        let fa = fork_skill(&src, "my-fork", out_a.path()).unwrap();
        let fb = fork_skill(&src, "my-fork", out_b.path()).unwrap();
        assert_eq!(fa.id, "my-fork");
        assert_eq!(fb.id, "my-fork");
        assert_eq!(fa.version, REMIX_TEMPLATE_VERSION);
        assert_eq!(fb.version, REMIX_TEMPLATE_VERSION);
        assert_eq!(fa.remixed_from.as_deref(), Some("src-skill"));
        assert_eq!(fb.remixed_from.as_deref(), Some("src-skill"));
        assert!(!fa.verified);
        assert_eq!(fa.engine, Engine::Coding);
        assert_eq!(fa.priority, 90);
        let ta = std::fs::read_to_string(out_a.path().join("my-fork").join("skill.toml")).unwrap();
        let tb = std::fs::read_to_string(out_b.path().join("my-fork").join("skill.toml")).unwrap();
        let na = ta.replace(
            &out_a.path().join("my-fork").to_string_lossy().into_owned(),
            "<HOME>",
        );
        let nb = tb.replace(
            &out_b.path().join("my-fork").to_string_lossy().into_owned(),
            "<HOME>",
        );
        assert_eq!(na, nb);
        assert!(out_a.path().join("my-fork").join("SKILL.md").is_file());
        assert!(out_a.path().join("my-fork").join("README.md").is_file());
        assert!(!out_a.path().join("my-fork").join(".checksum").exists());
    }

    #[test]
    fn fork_drops_signature_and_resets_lifecycle_to_draft() {
        let root = tempfile::tempdir().unwrap();
        let src = root.path().join("src-skill");
        seed_skill(&src, "src-skill");
        std::fs::write(src.join(".checksum"), "deadbeef").unwrap();
        let out = tempfile::tempdir().unwrap();
        let forked = fork_skill(&src, "clean-fork", out.path()).unwrap();
        assert_eq!(forked.remixed_from.as_deref(), Some("src-skill"));
        assert!(!forked.verified);
        assert!(!out.path().join("clean-fork").join(".checksum").exists());
    }

    #[test]
    fn fork_missing_source_fails_fail_safe() {
        let out = tempfile::tempdir().unwrap();
        let missing = out.path().join("does-not-exist");
        let err = fork_skill(&missing, "new-fork", out.path()).unwrap_err();
        assert!(err.to_string().contains("source not found"), "{err}");
        assert!(!out.path().join("new-fork").exists());
    }

    #[test]
    fn fork_invalid_new_id_rejected() {
        let root = tempfile::tempdir().unwrap();
        let src = root.path().join("src-skill");
        seed_skill(&src, "src-skill");
        let out = tempfile::tempdir().unwrap();
        let err = fork_skill(&src, "Bad Name!", out.path()).unwrap_err();
        assert!(err.to_string().contains("invalid skill id"), "{err}");
        assert!(!out.path().join("Bad Name!").exists());
    }

    #[test]
    fn fork_duplicate_id_fails() {
        let root = tempfile::tempdir().unwrap();
        let src = root.path().join("src-skill");
        seed_skill(&src, "src-skill");
        let out = tempfile::tempdir().unwrap();
        fork_skill(&src, "dup-fork", out.path()).unwrap();
        let err = fork_skill(&src, "dup-fork", out.path()).unwrap_err();
        assert!(err.to_string().contains("already exists"), "{err}");
    }

    #[test]
    fn fork_preserves_original_id_as_provenance_chain() {
        let root = tempfile::tempdir().unwrap();
        let src = root.path().join("v1");
        seed_skill(&src, "v1");
        let mid = tempfile::tempdir().unwrap();
        let f1 = fork_skill(&src, "v2", mid.path()).unwrap();
        assert_eq!(f1.remixed_from.as_deref(), Some("v1"));
        let out = tempfile::tempdir().unwrap();
        let f2 = fork_skill(&mid.path().join("v2"), "v3", out.path()).unwrap();
        assert_eq!(f2.remixed_from.as_deref(), Some("v2"));
        assert!(!f2.verified);
    }
}
