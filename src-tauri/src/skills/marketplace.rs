// Atlas OS — Marketplace local con firma obligatoria (RFC 06 §9, RFC 18 §5, research/39 Phase 10 sub-fase 10.1, M38).
// Zero-dep: std + anyhow/toml already in the tree (RFC 25 §11). Sin red en MVP:
// el marketplace es LOCAL firmado (research/39 SECTOR C) — el operador comparte
// por git/repos y `install` verifica la firma ANTES de copiar (patrón approval_for 7.1).

use std::path::{Path, PathBuf};

use crate::security::{self, ChecksumVerdict};
use crate::skills::{load_skill, validate_skill_id, SkillManifest};

pub const MARKETPLACE_FORBIDDEN: &str = "Forbidden";

#[derive(Clone, Debug)]
pub struct InstalledSkill {
    pub manifest: SkillManifest,
    pub dest: PathBuf,
    pub checksum: String,
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dest)
        .map_err(|e| anyhow::anyhow!("creating skill dest {}: {e}", dest.display()))?;
    for entry in std::fs::read_dir(src)
        .map_err(|e| anyhow::anyhow!("reading skill source {}: {e}", src.display()))?
    {
        let entry = entry?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if kind.is_file() {
            std::fs::copy(&from, &to).map_err(|e| {
                anyhow::anyhow!("copying {} -> {}: {e}", from.display(), to.display())
            })?;
        }
    }
    Ok(())
}

pub fn publish_skill(dir: &Path) -> anyhow::Result<String> {
    let manifest = load_skill(dir)?;
    validate_skill_id(&manifest.id)?;
    let checksum = security::skill_checksum(dir)?;
    security::write_sidecar(dir, &checksum)?;
    match security::verify_against_sidecar(dir)? {
        ChecksumVerdict::Ok => Ok(checksum),
        ChecksumVerdict::Mismatch => anyhow::bail!(
            "publish failed: checksum self-check mismatch for {} (RFC 18 §5)",
            dir.display()
        ),
        ChecksumVerdict::Missing => anyhow::bail!(
            "publish failed: checksum sidecar missing after write for {} (RFC 18 §5)",
            dir.display()
        ),
    }
}

pub fn install_skill(source: &Path, skills_dir: &Path) -> anyhow::Result<InstalledSkill> {
    if !source.is_dir() {
        anyhow::bail!(
            "{MARKETPLACE_FORBIDDEN}: skill source not found: {} (RFC 18 §5)",
            source.display()
        );
    }
    let manifest = load_skill(source).map_err(|e| {
        anyhow::anyhow!(
            "{MARKETPLACE_FORBIDDEN}: invalid skill manifest at {}: {e}",
            source.display()
        )
    })?;
    validate_skill_id(&manifest.id)?;
    match security::verify_against_sidecar(source)? {
        ChecksumVerdict::Ok => {}
        ChecksumVerdict::Missing => anyhow::bail!(
            "{MARKETPLACE_FORBIDDEN}: missing checksum signature for {} — publish first with `atlas skill publish` (RFC 18 §5)",
            source.display()
        ),
        ChecksumVerdict::Mismatch => anyhow::bail!(
            "{MARKETPLACE_FORBIDDEN}: checksum mismatch for {} — refusing to install (RFC 18 §5)",
            source.display()
        ),
    }
    let dest = skills_dir.join(&manifest.id);
    if dest.exists() {
        anyhow::bail!(
            "skill '{}' already installed at {} (duplicate id, RFC 06 §1)",
            manifest.id,
            dest.display()
        );
    }
    if source == dest || source.starts_with(&dest) || dest.starts_with(source) {
        anyhow::bail!(
            "refusing to install skill onto itself: {} -> {}",
            source.display(),
            dest.display()
        );
    }
    copy_dir_recursive(source, &dest)?;
    match security::verify_against_sidecar(&dest)? {
        ChecksumVerdict::Ok => {}
        other => {
            let _ = std::fs::remove_dir_all(&dest);
            anyhow::bail!(
                "{MARKETPLACE_FORBIDDEN}: post-copy verification failed ({other:?}) for {} — rolled back (RFC 18 §5)",
                dest.display()
            );
        }
    }
    let checksum = security::read_sidecar(&dest)?.unwrap_or_default();
    Ok(InstalledSkill {
        manifest,
        dest,
        checksum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::Engine;

    fn seed_unsigned(dir: &Path, id: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let m = SkillManifest {
            id: id.to_owned(),
            version: "1.0.0".to_owned(),
            description: format!("{id} skill"),
            engine: Engine::Coding,
            priority: 50,
            ..Default::default()
        };
        std::fs::write(dir.join("skill.toml"), toml::to_string_pretty(&m).unwrap()).unwrap();
        std::fs::write(dir.join("README.md"), format!("# {id}\n")).unwrap();
    }

    #[test]
    fn install_without_signature_rejects() {
        let src_root = tempfile::tempdir().unwrap();
        let src = src_root.path().join("demo-skill");
        seed_unsigned(&src, "demo-skill");
        let dest_root = tempfile::tempdir().unwrap();
        let err = install_skill(&src, dest_root.path()).unwrap_err();
        assert!(err.to_string().contains("Forbidden"), "{err}");
        assert!(err.to_string().contains("missing checksum"), "{err}");
        assert!(!dest_root.path().join("demo-skill").exists());
    }

    #[test]
    fn install_with_valid_signature_copies() {
        let src_root = tempfile::tempdir().unwrap();
        let src = src_root.path().join("demo-skill");
        seed_unsigned(&src, "demo-skill");
        let checksum = publish_skill(&src).unwrap();
        let dest_root = tempfile::tempdir().unwrap();
        let installed = install_skill(&src, dest_root.path()).unwrap();
        assert_eq!(installed.manifest.id, "demo-skill");
        assert_eq!(installed.checksum, checksum);
        assert!(installed.dest.join("skill.toml").is_file());
        assert!(installed.dest.join(".checksum").is_file());
        assert_eq!(
            security::verify_against_sidecar(&installed.dest).unwrap(),
            ChecksumVerdict::Ok
        );
    }

    #[test]
    fn install_with_tampered_source_rejects() {
        let src_root = tempfile::tempdir().unwrap();
        let src = src_root.path().join("demo-skill");
        seed_unsigned(&src, "demo-skill");
        publish_skill(&src).unwrap();
        std::fs::write(src.join("README.md"), "# tampered\n").unwrap();
        let dest_root = tempfile::tempdir().unwrap();
        let err = install_skill(&src, dest_root.path()).unwrap_err();
        assert!(err.to_string().contains("Forbidden"), "{err}");
        assert!(err.to_string().contains("mismatch"), "{err}");
        assert!(!dest_root.path().join("demo-skill").exists());
    }

    #[test]
    fn publish_is_deterministic() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("demo-skill");
        seed_unsigned(&dir, "demo-skill");
        let first = publish_skill(&dir).unwrap();
        let second = publish_skill(&dir).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        std::fs::write(dir.join("README.md"), "# changed\n").unwrap();
        let third = publish_skill(&dir).unwrap();
        assert_ne!(first, third);
    }

    #[test]
    fn install_duplicate_id_fails() {
        let src_root = tempfile::tempdir().unwrap();
        let src = src_root.path().join("demo-skill");
        seed_unsigned(&src, "demo-skill");
        publish_skill(&src).unwrap();
        let dest_root = tempfile::tempdir().unwrap();
        install_skill(&src, dest_root.path()).unwrap();
        let err = install_skill(&src, dest_root.path()).unwrap_err();
        assert!(err.to_string().contains("already installed"), "{err}");
    }

    #[test]
    fn install_missing_source_fails_fail_safe() {
        let dest_root = tempfile::tempdir().unwrap();
        let err =
            install_skill(&dest_root.path().join("does-not-exist"), dest_root.path()).unwrap_err();
        assert!(err.to_string().contains("Forbidden"), "{err}");
    }
}
