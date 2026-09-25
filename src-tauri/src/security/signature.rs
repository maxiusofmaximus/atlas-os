// Atlas OS — Skill checksums SHA-256 (RFC 18 §5, Phase 7 sub-fase 7.0).
//
// MVP integrity: canonical SHA-256 over `skill.toml` + `README.md`
// (stable order) verified at `atlas skill install` time. Fail-safe:
// mismatch or missing signature blocks the install as `Forbidden`
// (RFC 18 §5). ed25519/minisign stays deferred pending audit
// (research/34 SECTOR A.1, RFC 25 §11 — no new crates).

use sha2::{Digest, Sha256};
use std::path::Path;

pub const MANIFEST_NAME: &str = "skill.toml";
pub const README_NAME: &str = "README.md";
pub const SIDECAR_NAME: &str = ".checksum";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChecksumVerdict {
    Ok,
    Mismatch,
    Missing,
}

pub fn skill_checksum(dir: &Path) -> anyhow::Result<String> {
    let manifest_path = dir.join(MANIFEST_NAME);
    let manifest_bytes = std::fs::read(&manifest_path)
        .map_err(|e| anyhow::anyhow!("reading skill manifest {}: {e}", manifest_path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(b"skill.toml\x00");
    hasher.update((manifest_bytes.len() as u64).to_le_bytes());
    hasher.update(b"\x00");
    hasher.update(&manifest_bytes);
    let readme_path = dir.join(README_NAME);
    if readme_path.is_file() {
        let readme_bytes = std::fs::read(&readme_path)
            .map_err(|e| anyhow::anyhow!("reading skill readme {}: {e}", readme_path.display()))?;
        hasher.update(b"README.md\x00");
        hasher.update((readme_bytes.len() as u64).to_le_bytes());
        hasher.update(b"\x00");
        hasher.update(&readme_bytes);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn verify_skill(dir: &Path, expected: &str) -> anyhow::Result<ChecksumVerdict> {
    if !dir.join(MANIFEST_NAME).is_file() {
        return Ok(ChecksumVerdict::Missing);
    }
    let actual = skill_checksum(dir)?;
    if actual.eq_ignore_ascii_case(expected.trim()) {
        Ok(ChecksumVerdict::Ok)
    } else {
        Ok(ChecksumVerdict::Mismatch)
    }
}

pub fn write_sidecar(dir: &Path, checksum: &str) -> anyhow::Result<()> {
    let sidecar = dir.join(SIDECAR_NAME);
    std::fs::write(&sidecar, format!("{checksum}\n"))
        .map_err(|e| anyhow::anyhow!("writing checksum sidecar {}: {e}", sidecar.display()))?;
    Ok(())
}

pub fn read_sidecar(dir: &Path) -> anyhow::Result<Option<String>> {
    let sidecar = dir.join(SIDECAR_NAME);
    if !sidecar.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&sidecar)
        .map_err(|e| anyhow::anyhow!("reading checksum sidecar {}: {e}", sidecar.display()))?;
    let trimmed = raw.trim().to_owned();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed))
    }
}

pub fn verify_against_sidecar(dir: &Path) -> anyhow::Result<ChecksumVerdict> {
    match read_sidecar(dir)? {
        None => Ok(ChecksumVerdict::Missing),
        Some(expected) => verify_skill(dir, &expected),
    }
}

pub fn install_gate(dir: &Path) -> anyhow::Result<()> {
    match verify_against_sidecar(dir)? {
        ChecksumVerdict::Ok => Ok(()),
        ChecksumVerdict::Mismatch => anyhow::bail!(
            "skill install Forbidden: checksum mismatch for {} (RFC 18 §5)",
            dir.display()
        ),
        ChecksumVerdict::Missing => anyhow::bail!(
            "skill install Forbidden: missing checksum signature for {} (RFC 18 §5)",
            dir.display()
        ),
    }
}

pub fn catalog_checksum(manifests: &[crate::skills::SkillManifest]) -> String {
    let mut sorted: Vec<&crate::skills::SkillManifest> = manifests.iter().collect();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    let mut hasher = Sha256::new();
    for m in sorted {
        hasher.update(b"id\x00");
        hasher.update(m.id.as_bytes());
        hasher.update(b"\x00version\x00");
        hasher.update(m.version.as_bytes());
        hasher.update(b"\x00description\x00");
        hasher.update(m.description.as_bytes());
        hasher.update(b"\x00engine\x00");
        hasher.update(m.engine.tag().as_bytes());
        hasher.update(b"\x00priority\x00");
        hasher.update(m.priority.to_le_bytes());
        hasher.update(b"\x00");
    }
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_skill(dir: &Path, manifest_body: &str, readme: Option<&str>) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(MANIFEST_NAME), manifest_body).unwrap();
        if let Some(body) = readme {
            std::fs::write(dir.join(README_NAME), body).unwrap();
        }
    }

    const MANIFEST: &str = "id = \"demo\"\nversion = \"1.0.0\"\ndescription = \"demo skill\"\n";

    #[test]
    fn checksum_is_deterministic_and_hex() {
        let tmp = tempfile::tempdir().unwrap();
        seed_skill(tmp.path(), MANIFEST, Some("# demo"));
        let a = skill_checksum(tmp.path()).unwrap();
        let b = skill_checksum(tmp.path()).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn checksum_covers_readme_changes() {
        let tmp = tempfile::tempdir().unwrap();
        seed_skill(tmp.path(), MANIFEST, Some("# v1"));
        let with_readme = skill_checksum(tmp.path()).unwrap();
        std::fs::remove_file(tmp.path().join(README_NAME)).unwrap();
        let without_readme = skill_checksum(tmp.path()).unwrap();
        assert_ne!(with_readme, without_readme);
        std::fs::write(tmp.path().join(README_NAME), "# v2").unwrap();
        let changed = skill_checksum(tmp.path()).unwrap();
        assert_ne!(with_readme, changed);
    }

    #[test]
    fn verify_detects_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        seed_skill(tmp.path(), MANIFEST, None);
        let good = skill_checksum(tmp.path()).unwrap();
        assert_eq!(
            verify_skill(tmp.path(), &good).unwrap(),
            ChecksumVerdict::Ok
        );
        std::fs::write(tmp.path().join(MANIFEST_NAME), "id = \"tampered\"\n").unwrap();
        assert_eq!(
            verify_skill(tmp.path(), &good).unwrap(),
            ChecksumVerdict::Mismatch
        );
    }

    #[test]
    fn verify_reports_missing_without_manifest() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            verify_skill(tmp.path(), "abc").unwrap(),
            ChecksumVerdict::Missing
        );
        assert_eq!(
            verify_against_sidecar(tmp.path()).unwrap(),
            ChecksumVerdict::Missing
        );
    }

    #[test]
    fn sidecar_round_trip_verifies() {
        let tmp = tempfile::tempdir().unwrap();
        seed_skill(tmp.path(), MANIFEST, Some("# demo"));
        let checksum = skill_checksum(tmp.path()).unwrap();
        write_sidecar(tmp.path(), &checksum).unwrap();
        assert_eq!(
            read_sidecar(tmp.path()).unwrap().as_deref(),
            Some(checksum.as_str())
        );
        assert_eq!(
            verify_against_sidecar(tmp.path()).unwrap(),
            ChecksumVerdict::Ok
        );
    }

    #[test]
    fn install_gate_is_fail_safe() {
        let tmp = tempfile::tempdir().unwrap();
        seed_skill(tmp.path(), MANIFEST, None);
        let err = install_gate(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("Forbidden"), "{err}");
        let checksum = skill_checksum(tmp.path()).unwrap();
        write_sidecar(tmp.path(), &checksum).unwrap();
        install_gate(tmp.path()).unwrap();
        std::fs::write(tmp.path().join(MANIFEST_NAME), "id = \"evil\"\n").unwrap();
        let err = install_gate(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("Forbidden"), "{err}");
        assert!(err.to_string().contains("mismatch"), "{err}");
    }

    #[test]
    fn bundled_catalog_checksum_is_stable() {
        let manifests = crate::skills::bundled::bundled_manifests().unwrap();
        assert_eq!(manifests.len(), crate::skills::BUNDLED_COUNT);
        let a = catalog_checksum(&manifests);
        let fresh = crate::skills::bundled::bundled_manifests().unwrap();
        let b = catalog_checksum(&fresh);
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }
}
