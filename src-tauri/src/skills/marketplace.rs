// Atlas OS — Marketplace local con firma obligatoria (RFC 06 §9, RFC 18 §5, research/39 Phase 10 sub-fase 10.1, M38).
// Zero-dep: std + anyhow/toml already in the tree (RFC 25 §11). Sin red en MVP:
// el marketplace es LOCAL firmado (research/39 SECTOR C) — el operador comparte
// por git/repos y `install` verifica la firma ANTES de copiar (patrón approval_for 7.1).

use std::path::{Path, PathBuf};
use std::process::Command;

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

pub fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn git_stderr_detail(stderr: &[u8]) -> String {
    let s = String::from_utf8_lossy(stderr);
    let t = s.trim();
    if t.is_empty() {
        return "git exited with an error status".into();
    }
    if t.len() > 300 {
        t[..300].to_string()
    } else {
        t.to_string()
    }
}

fn manifest_id_at(dir: &Path) -> Option<String> {
    load_skill(dir).ok().map(|m| m.id)
}

fn collect_clone_candidates(clone_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if clone_root.join("skill.toml").is_file() {
        out.push(clone_root.to_path_buf());
    }
    let mut scan_one_level = |parent: &Path| {
        let Ok(entries) = std::fs::read_dir(parent) else {
            return;
        };
        let mut names: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        names.sort();
        for path in names {
            if path.is_dir() && path.join("skill.toml").is_file() {
                out.push(path);
            }
        }
    };
    scan_one_level(clone_root);
    scan_one_level(&clone_root.join("skills"));
    out.sort();
    out.dedup();
    out
}

fn resolve_skill_dir_in_clone(clone_root: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let wanted = name.trim();
    if !wanted.is_empty() {
        for join in [
            clone_root.join(wanted),
            clone_root.join("skills").join(wanted),
        ] {
            if join.join("skill.toml").is_file() {
                return Ok(join);
            }
        }
        for candidate in collect_clone_candidates(clone_root) {
            if manifest_id_at(&candidate).as_deref() == Some(wanted) {
                return Ok(candidate);
            }
        }
        let available: Vec<String> = collect_clone_candidates(clone_root)
            .iter()
            .filter_map(|p| manifest_id_at(p))
            .collect();
        if available.is_empty() {
            anyhow::bail!(
                "no signed skill found in git clone {root} for '{wanted}' (expected <repo>/<name>/skill.toml or skills/<name>/skill.toml)",
                root = clone_root.display(),
            );
        }
        anyhow::bail!(
            "skill '{wanted}' not found in git clone {root} (available: {list})",
            root = clone_root.display(),
            list = available.join(", "),
        );
    }
    let candidates = collect_clone_candidates(clone_root);
    match candidates.len() {
        1 => Ok(candidates.into_iter().next().expect("len checked")),
        0 => anyhow::bail!(
            "no skill found in git clone {root} (expected skill.toml at the repo root or in <name>/skill.toml)",
            root = clone_root.display(),
        ),
        _ => {
            let available: Vec<String> = candidates
                .iter()
                .filter_map(|p| manifest_id_at(p))
                .collect();
            anyhow::bail!(
                "git clone {root} holds several skills (available: {list}) — pass the skill name: `atlas skill install <name> --from <git-url>`",
                root = clone_root.display(),
                list = available.join(", "),
            );
        }
    }
}

pub fn install_from_git(
    url: &str,
    name: &str,
    skills_dir: &Path,
) -> anyhow::Result<InstalledSkill> {
    let url = url.trim();
    if url.is_empty() {
        anyhow::bail!("{MARKETPLACE_FORBIDDEN}: empty --from git URL (pass a repo URL)");
    }
    if url.starts_with('-') {
        anyhow::bail!("{MARKETPLACE_FORBIDDEN}: invalid --from git URL {url:?}");
    }
    if !git_available() {
        anyhow::bail!(
            "git not found on PATH — install git to use `atlas skill install --from` (patron swarm 4.0)"
        );
    }
    let tmp = tempfile::tempdir().map_err(|e| anyhow::anyhow!("creating temp clone dir: {e}"))?;
    let clone_dest = tmp.path().join("repo");
    let out = Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--")
        .arg(url)
        .arg(&clone_dest)
        .output()
        .map_err(|e| anyhow::anyhow!("spawning `git clone --depth 1`: {e}"))?;
    if !out.status.success() {
        anyhow::bail!(
            "git clone --depth 1 failed for {url}: {detail}",
            detail = git_stderr_detail(&out.stderr),
        );
    }
    let skill_dir = resolve_skill_dir_in_clone(&clone_dest, name)?;
    install_skill(&skill_dir, skills_dir)
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

    fn git_cmd(args: &[&str], dir: &Path) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("spawn git")
            .status;
        assert!(status.success(), "git {args:?} failed");
    }

    fn init_origin_with(repo: &Path, skills: &[&str], signed: bool) {
        std::fs::create_dir_all(repo).unwrap();
        git_cmd(&["init"], repo);
        git_cmd(&["config", "user.email", "atlas@test.invalid"], repo);
        git_cmd(&["config", "user.name", "atlas-test"], repo);
        for id in skills {
            let dir = if skills.len() == 1 && *id == "__root__" {
                repo.to_path_buf()
            } else {
                repo.join(id)
            };
            let real_id = if *id == "__root__" { "demo-skill" } else { id };
            seed_unsigned(&dir, real_id);
            if signed {
                publish_skill(&dir).unwrap();
            }
        }
        git_cmd(&["add", "-A"], repo);
        git_cmd(&["commit", "-m", "seed skills"], repo);
    }

    fn require_git() -> bool {
        if git_available() {
            return true;
        }
        eprintln!("skipping — git not on PATH");
        false
    }

    #[test]
    fn install_from_git_single_skill_at_root() {
        if !require_git() {
            return;
        }
        let origin = tempfile::tempdir().unwrap();
        init_origin_with(origin.path(), &["__root__"], true);
        let dest_root = tempfile::tempdir().unwrap();
        let installed = install_from_git(
            origin.path().to_str().unwrap(),
            "demo-skill",
            dest_root.path(),
        )
        .unwrap();
        assert_eq!(installed.manifest.id, "demo-skill");
        assert!(installed.dest.join("skill.toml").is_file());
        assert!(installed.dest.join(".checksum").is_file());
        assert_eq!(
            security::verify_against_sidecar(&installed.dest).unwrap(),
            ChecksumVerdict::Ok
        );
    }

    #[test]
    fn install_from_git_rejects_unsigned_clone() {
        if !require_git() {
            return;
        }
        let origin = tempfile::tempdir().unwrap();
        init_origin_with(origin.path(), &["__root__"], false);
        let dest_root = tempfile::tempdir().unwrap();
        let err = install_from_git(
            origin.path().to_str().unwrap(),
            "demo-skill",
            dest_root.path(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("Forbidden"), "{err}");
        assert!(err.to_string().contains("missing checksum"), "{err}");
        assert!(!dest_root.path().join("demo-skill").exists());
    }

    #[test]
    fn install_from_git_rejects_tampered_clone() {
        if !require_git() {
            return;
        }
        let origin = tempfile::tempdir().unwrap();
        init_origin_with(origin.path(), &["__root__"], true);
        std::fs::write(origin.path().join("README.md"), "# tampered\n").unwrap();
        git_cmd(&["add", "-A"], origin.path());
        git_cmd(&["commit", "-m", "tamper"], origin.path());
        let dest_root = tempfile::tempdir().unwrap();
        let err = install_from_git(
            origin.path().to_str().unwrap(),
            "demo-skill",
            dest_root.path(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("Forbidden"), "{err}");
        assert!(err.to_string().contains("mismatch"), "{err}");
        assert!(!dest_root.path().join("demo-skill").exists());
    }

    #[test]
    fn install_from_git_selects_named_subdir() {
        if !require_git() {
            return;
        }
        let origin = tempfile::tempdir().unwrap();
        init_origin_with(origin.path(), &["alpha-skill", "beta-skill"], true);
        let dest_root = tempfile::tempdir().unwrap();
        let installed = install_from_git(
            origin.path().to_str().unwrap(),
            "beta-skill",
            dest_root.path(),
        )
        .unwrap();
        assert_eq!(installed.manifest.id, "beta-skill");
        assert!(dest_root.path().join("beta-skill").is_dir());
        assert!(!dest_root.path().join("alpha-skill").exists());
    }

    #[test]
    fn install_from_git_unknown_name_lists_available() {
        if !require_git() {
            return;
        }
        let origin = tempfile::tempdir().unwrap();
        init_origin_with(origin.path(), &["alpha-skill", "beta-skill"], true);
        let dest_root = tempfile::tempdir().unwrap();
        let err = install_from_git(
            origin.path().to_str().unwrap(),
            "ghost-skill",
            dest_root.path(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("not found"), "{err}");
        assert!(err.to_string().contains("alpha-skill"), "{err}");
        assert!(!dest_root.path().join("ghost-skill").exists());
    }

    #[test]
    fn install_from_git_invalid_repo_fails_fail_safe() {
        if !require_git() {
            return;
        }
        let dest_root = tempfile::tempdir().unwrap();
        let before: Vec<_> = std::fs::read_dir(dest_root.path())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(before.is_empty());
        let err = install_from_git(
            "/nonexistent-atlas-git-origin-xyz",
            "demo-skill",
            dest_root.path(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("git clone"), "{err}");
        let after: Vec<_> = std::fs::read_dir(dest_root.path())
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(after.is_empty(), "failed clone must not install anything");
    }

    #[test]
    fn install_from_git_rejects_option_injection_without_spawning() {
        let dest_root = tempfile::tempdir().unwrap();
        let err = install_from_git("--upload-pack=touch pwned", "demo-skill", dest_root.path())
            .unwrap_err();
        assert!(err.to_string().contains("Forbidden"), "{err}");
        assert!(err.to_string().contains("invalid --from"), "{err}");
    }

    #[test]
    fn install_from_git_empty_url_fails_fail_safe() {
        let dest_root = tempfile::tempdir().unwrap();
        let err = install_from_git("   ", "demo-skill", dest_root.path()).unwrap_err();
        assert!(err.to_string().contains("Forbidden"), "{err}");
        assert!(err.to_string().contains("empty --from"), "{err}");
    }
}
