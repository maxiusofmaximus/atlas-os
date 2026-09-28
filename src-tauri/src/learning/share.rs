// Atlas OS — Learning social (RFC 16 §3, research/39 Phase 10 sub-fase 10.3, M40).
// Zero-dep: std + anyhow/serde/serde_yaml/sha2/hex/chrono/uuid already in the tree (RFC 25 §11).
//
// `export_rules` comparte el conocimiento VERIFICADO (5.1): solo reglas
// `candidate` / `active` (`Pattern::is_consultable`) salen al YAML. Los
// borradores (`draft`) y retiradas (`deprecated`) nunca se comparten.
// `import_rules` verifica la firma del archivo (hash SHA-256, patrón
// `security/signature.rs`) ANTES de parsear y aplica dedup por `rule_id`
// (first-write-wins, RFC 02 §3.1.2): lo ya existente se salta, nunca se
// sobrescribe.
//
// El YAML compartido es determinista: `rules` ordenado por `id`, sin
// timestamps ni UUIDs aleatorios. La firma vive en el sidecar
// `<file>.checksum` (hex SHA-256 de los bytes del YAML).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::security::ChecksumVerdict;

use super::types::{Pattern, RuleLifecycle, RuleThen, RuleWhen};

pub const SHARE_VERSION: u32 = 1;
pub const SHARE_FORBIDDEN: &str = "Forbidden";
pub const SHARED_IMPORT_MODEL: &str = "shared-import";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SharedRule {
    pub id: String,
    pub when: RuleWhen,
    pub then: RuleThen,
    pub lifecycle: RuleLifecycle,
    pub priority: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SharedRulesFile {
    version: u32,
    rules: Vec<SharedRule>,
}

#[derive(Clone, Debug)]
pub struct ExportReport {
    pub path: PathBuf,
    pub checksum: String,
    pub count: usize,
}

#[derive(Clone, Debug)]
pub struct ImportReport {
    pub path: PathBuf,
    pub imported: usize,
    pub skipped_duplicate: usize,
    pub skipped_unverified: usize,
}

fn sidecar_path(file: &Path) -> PathBuf {
    let mut s = file.as_os_str().to_owned();
    s.push(".checksum");
    PathBuf::from(s)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn file_checksum(file: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(file)
        .map_err(|e| anyhow::anyhow!("reading share file {}: {e}", file.display()))?;
    Ok(sha256_hex(&bytes))
}

pub fn write_file_sidecar(file: &Path, checksum: &str) -> anyhow::Result<()> {
    let sidecar = sidecar_path(file);
    std::fs::write(&sidecar, format!("{checksum}\n"))
        .map_err(|e| anyhow::anyhow!("writing share sidecar {}: {e}", sidecar.display()))?;
    Ok(())
}

pub fn read_file_sidecar(file: &Path) -> anyhow::Result<Option<String>> {
    let sidecar = sidecar_path(file);
    if !sidecar.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&sidecar)
        .map_err(|e| anyhow::anyhow!("reading share sidecar {}: {e}", sidecar.display()))?;
    let trimmed = raw.trim().to_owned();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed))
    }
}

pub fn verify_file_against_sidecar(file: &Path) -> anyhow::Result<ChecksumVerdict> {
    let Some(expected) = read_file_sidecar(file)? else {
        return Ok(ChecksumVerdict::Missing);
    };
    let actual = file_checksum(file)?;
    if actual.eq_ignore_ascii_case(expected.trim()) {
        Ok(ChecksumVerdict::Ok)
    } else {
        Ok(ChecksumVerdict::Mismatch)
    }
}

fn lifecycle_from_tag(tag: &str) -> Option<RuleLifecycle> {
    match tag {
        "draft" => Some(RuleLifecycle::Draft),
        "candidate" => Some(RuleLifecycle::Candidate),
        "active" => Some(RuleLifecycle::Active),
        "deprecated" => Some(RuleLifecycle::Deprecated),
        _ => None,
    }
}

fn confidence_for_lifecycle(lifecycle: RuleLifecycle) -> f32 {
    match lifecycle {
        RuleLifecycle::Candidate => 0.7,
        RuleLifecycle::Active => 0.85,
        RuleLifecycle::Draft => 0.5,
        RuleLifecycle::Deprecated => 0.0,
    }
}

/// Exporta las reglas VERIFICADAS (`candidate` / `active`, RFC 16 §2 +
/// 5.1) del Journal a `dest` como YAML determinista + sidecar
/// `<dest>.checksum` con el SHA-256 del archivo.
///
/// Determinista: `rules` ordenado por `id`; el archivo no contiene
/// timestamps ni UUIDs aleatorios, así que dos exports del mismo estado
/// producen bytes idénticos y el mismo checksum.
pub fn export_rules(
    journal: &crate::journal::Journal,
    dest: &Path,
) -> anyhow::Result<ExportReport> {
    let rows = journal.list_consultable_rules(10_000)?;
    let mut rules = Vec::with_capacity(rows.len());
    for row in &rows {
        let Some((when, then)) = journal.learned_rule_when_then(&row.id)? else {
            continue;
        };
        let Some(lifecycle) = lifecycle_from_tag(row.lifecycle.as_str()) else {
            continue;
        };
        if !matches!(lifecycle, RuleLifecycle::Candidate | RuleLifecycle::Active) {
            continue;
        }
        rules.push(SharedRule {
            id: row.id.clone(),
            when,
            then,
            lifecycle,
            priority: row.priority.max(0) as u32,
        });
    }
    rules.sort_by(|a, b| a.id.cmp(&b.id));
    let file = SharedRulesFile {
        version: SHARE_VERSION,
        rules,
    };
    let yaml = serde_yaml::to_string(&file)?;
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                anyhow::anyhow!("creating share dest dir {}: {e}", parent.display())
            })?;
        }
    }
    std::fs::write(dest, &yaml)
        .map_err(|e| anyhow::anyhow!("writing share file {}: {e}", dest.display()))?;
    let checksum = sha256_hex(yaml.as_bytes());
    write_file_sidecar(dest, &checksum)?;
    match verify_file_against_sidecar(dest)? {
        ChecksumVerdict::Ok => {}
        other => anyhow::bail!(
            "{SHARE_FORBIDDEN}: post-write verification failed ({other:?}) for {}",
            dest.display()
        ),
    }
    Ok(ExportReport {
        path: dest.to_path_buf(),
        checksum,
        count: file.rules.len(),
    })
}

/// Importa un YAML compartido al Journal. Verifica la firma ANTES de
/// parsear (patrón `approval_for` 7.1): sin sidecar o con hash distinto
/// rechaza fail-safe sin tocar el Journal. Dedup por `rule_id`
/// (first-write-wins, RFC 02 §3.1.2): lo ya existente cuenta como
/// `skipped_duplicate`. Solo `candidate` / `active` se importan; el resto
/// cuenta como `skipped_unverified` y nunca entra como verificado.
pub fn import_rules(journal: &crate::journal::Journal, src: &Path) -> anyhow::Result<ImportReport> {
    if !src.is_file() {
        anyhow::bail!("share source not found: {} (M40)", src.display());
    }
    match verify_file_against_sidecar(src)? {
        ChecksumVerdict::Ok => {}
        ChecksumVerdict::Missing => anyhow::bail!(
            "{SHARE_FORBIDDEN}: missing checksum signature for {} — ask the sender to share `<file>.checksum` too (M40)",
            src.display()
        ),
        ChecksumVerdict::Mismatch => anyhow::bail!(
            "{SHARE_FORBIDDEN}: checksum mismatch for {} — refusing to import (M40)",
            src.display()
        ),
    }
    let raw = std::fs::read_to_string(src)
        .map_err(|e| anyhow::anyhow!("reading share file {}: {e}", src.display()))?;
    let file: SharedRulesFile = serde_yaml::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("invalid share file {}: {e} (M40)", src.display()))?;
    if file.version != SHARE_VERSION {
        anyhow::bail!(
            "unsupported share version {} in {} (expected {SHARE_VERSION}, M40)",
            file.version,
            src.display()
        );
    }
    let now = chrono::Utc::now().to_rfc3339();
    let mut imported = 0usize;
    let mut skipped_duplicate = 0usize;
    let mut skipped_unverified = 0usize;
    let mut seen: HashSet<String> = HashSet::new();
    for rule in &file.rules {
        if rule.id.trim().is_empty() {
            anyhow::bail!(
                "invalid share file {}: rule id must not be empty (M40)",
                src.display()
            );
        }
        if !matches!(
            rule.lifecycle,
            RuleLifecycle::Candidate | RuleLifecycle::Active
        ) {
            skipped_unverified += 1;
            continue;
        }
        if !seen.insert(rule.id.clone()) {
            skipped_duplicate += 1;
            continue;
        }
        if journal.get_learned_rule(&rule.id)?.is_some() {
            skipped_duplicate += 1;
            continue;
        }
        let pattern = Pattern {
            pattern_id: uuid::Uuid::new_v4(),
            rule_id: rule.id.clone(),
            when: rule.when.clone(),
            then: rule.then.clone(),
            confidence: confidence_for_lifecycle(rule.lifecycle),
            lifecycle: rule.lifecycle,
            priority: rule.priority,
            evidence: Vec::new(),
            generated_at: now.clone(),
            model_id: SHARED_IMPORT_MODEL.into(),
            elapsed_ms: 0,
        };
        journal.save_learned_rule(&pattern)?;
        imported += 1;
    }
    Ok(ImportReport {
        path: src.to_path_buf(),
        imported,
        skipped_duplicate,
        skipped_unverified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::Journal;
    use crate::learning::types::{RuleThen, RuleWhen};
    use crate::repair::types::RepairStrategy;
    use crate::validation::types::StageKind;

    fn fixture(when_pattern: &str, rule_id: &str) -> Pattern {
        Pattern {
            pattern_id: uuid::Uuid::new_v4(),
            rule_id: rule_id.into(),
            when: RuleWhen {
                stage: StageKind::LintFormat,
                pattern: when_pattern.into(),
                lang: "rust".into(),
            },
            then: RuleThen {
                strategy: RepairStrategy::AutoFix,
                skill: String::new(),
                diff_hint: "remove println".into(),
            },
            confidence: 0.7,
            lifecycle: RuleLifecycle::Draft,
            priority: RuleLifecycle::Draft.default_priority(),
            evidence: Vec::new(),
            generated_at: "2026-07-04T12:00:00Z".into(),
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        }
    }

    fn seed_verified(dir: &Path) -> Journal {
        let j = Journal::open(dir).unwrap();
        j.save_learned_rule(&fixture("no_println", "r-2026-07-04-002"))
            .unwrap();
        j.save_learned_rule(&fixture("no_dbg", "r-2026-07-04-001"))
            .unwrap();
        j.save_learned_rule(&fixture("draft_only", "r-2026-07-04-003"))
            .unwrap();
        j.promote_rule("r-2026-07-04-002").unwrap();
        j.promote_rule("r-2026-07-04-002").unwrap();
        j.promote_rule("r-2026-07-04-001").unwrap();
        j
    }

    fn read_shared(path: &Path) -> SharedRulesFile {
        let raw = std::fs::read_to_string(path).unwrap();
        serde_yaml::from_str(&raw).unwrap()
    }

    #[test]
    fn export_shares_only_verified_sorted_and_deterministic() {
        let root = tempfile::tempdir().unwrap();
        let j = seed_verified(root.path());
        let a = root.path().join("a.yaml");
        let b = root.path().join("b.yaml");
        let ra = export_rules(&j, &a).unwrap();
        let rb = export_rules(&j, &b).unwrap();
        assert_eq!(ra.count, 2);
        assert_eq!(rb.count, 2);
        let fa = read_shared(&a);
        assert_eq!(fa.version, SHARE_VERSION);
        assert_eq!(fa.rules.len(), 2);
        assert_eq!(fa.rules[0].id, "r-2026-07-04-001");
        assert_eq!(fa.rules[1].id, "r-2026-07-04-002");
        assert!(fa.rules.iter().all(|r| matches!(
            r.lifecycle,
            RuleLifecycle::Candidate | RuleLifecycle::Active
        )));
        assert!(!fa.rules.iter().any(|r| r.id == "r-2026-07-04-003"));
        let ya = std::fs::read_to_string(&a).unwrap();
        let yb = std::fs::read_to_string(&b).unwrap();
        assert_eq!(ya, yb);
        assert_eq!(ra.checksum, rb.checksum);
        assert_eq!(
            verify_file_against_sidecar(&a).unwrap(),
            ChecksumVerdict::Ok
        );
    }

    #[test]
    fn export_empty_when_nothing_verified() {
        let root = tempfile::tempdir().unwrap();
        let j = Journal::open(root.path()).unwrap();
        j.save_learned_rule(&fixture("x", "r-draft-001")).unwrap();
        let dest = root.path().join("empty.yaml");
        let report = export_rules(&j, &dest).unwrap();
        assert_eq!(report.count, 0);
        let file = read_shared(&dest);
        assert!(file.rules.is_empty());
        assert_eq!(
            verify_file_against_sidecar(&dest).unwrap(),
            ChecksumVerdict::Ok
        );
    }

    #[test]
    fn import_roundtrip_preserves_when_then_lifecycle_priority() {
        let root = tempfile::tempdir().unwrap();
        let src_dir = root.path().join("src");
        let dst_dir = root.path().join("dst");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        let sender = seed_verified(&src_dir);
        let file = root.path().join("shared.yaml");
        export_rules(&sender, &file).unwrap();
        let receiver = Journal::open(&dst_dir).unwrap();
        let report = import_rules(&receiver, &file).unwrap();
        assert_eq!(report.imported, 2);
        assert_eq!(report.skipped_duplicate, 0);
        assert_eq!(report.skipped_unverified, 0);
        for id in ["r-2026-07-04-001", "r-2026-07-04-002"] {
            let row = receiver.get_learned_rule(id).unwrap().expect("row");
            let (when, then) = receiver.learned_rule_when_then(id).unwrap().expect("blobs");
            let sent_row = sender.get_learned_rule(id).unwrap().expect("sent");
            let (sent_when, sent_then) = sender
                .learned_rule_when_then(id)
                .unwrap()
                .expect("sent blobs");
            assert_eq!(when, sent_when);
            assert_eq!(then, sent_then);
            assert_eq!(row.lifecycle, sent_row.lifecycle);
            assert_eq!(row.priority, sent_row.priority);
        }
    }

    #[test]
    fn import_dedups_first_write_wins() {
        let root = tempfile::tempdir().unwrap();
        let src_dir = root.path().join("src");
        let dst_dir = root.path().join("dst");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        let sender = seed_verified(&src_dir);
        let file = root.path().join("shared.yaml");
        export_rules(&sender, &file).unwrap();
        let receiver = Journal::open(&dst_dir).unwrap();
        let first = import_rules(&receiver, &file).unwrap();
        assert_eq!(first.imported, 2);
        let second = import_rules(&receiver, &file).unwrap();
        assert_eq!(second.imported, 0);
        assert_eq!(second.skipped_duplicate, 2);
        let before = receiver
            .learned_rule_when_then("r-2026-07-04-001")
            .unwrap()
            .unwrap();
        let again = import_rules(&receiver, &file).unwrap();
        assert_eq!(again.skipped_duplicate, 2);
        let after = receiver
            .learned_rule_when_then("r-2026-07-04-001")
            .unwrap()
            .unwrap();
        assert_eq!(before.0, after.0);
        assert_eq!(before.1, after.1);
    }

    #[test]
    fn import_rejects_tampered_file() {
        let root = tempfile::tempdir().unwrap();
        let src_dir = root.path().join("src");
        let dst_dir = root.path().join("dst");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        let sender = seed_verified(&src_dir);
        let file = root.path().join("shared.yaml");
        export_rules(&sender, &file).unwrap();
        std::fs::write(&file, "tampered: true\n").unwrap();
        let receiver = Journal::open(&dst_dir).unwrap();
        let err = import_rules(&receiver, &file).unwrap_err();
        assert!(err.to_string().contains("Forbidden"), "{err}");
        assert!(err.to_string().contains("mismatch"), "{err}");
        assert!(receiver.list_learned_rules(10).unwrap().is_empty());
    }

    #[test]
    fn import_rejects_missing_sidecar() {
        let root = tempfile::tempdir().unwrap();
        let src_dir = root.path().join("src");
        let dst_dir = root.path().join("dst");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        let sender = seed_verified(&src_dir);
        let file = root.path().join("shared.yaml");
        export_rules(&sender, &file).unwrap();
        std::fs::remove_file(sidecar_path(&file)).unwrap();
        let receiver = Journal::open(&dst_dir).unwrap();
        let err = import_rules(&receiver, &file).unwrap_err();
        assert!(err.to_string().contains("Forbidden"), "{err}");
        assert!(err.to_string().contains("missing checksum"), "{err}");
        assert!(receiver.list_learned_rules(10).unwrap().is_empty());
    }

    #[test]
    fn import_missing_source_fails_fail_safe() {
        let root = tempfile::tempdir().unwrap();
        let j = Journal::open(root.path()).unwrap();
        let err = import_rules(&j, &root.path().join("does-not-exist.yaml")).unwrap_err();
        assert!(err.to_string().contains("not found"), "{err}");
    }

    #[test]
    fn import_rejects_unsupported_version() {
        let root = tempfile::tempdir().unwrap();
        let dst_dir = root.path().join("dst");
        std::fs::create_dir_all(&dst_dir).unwrap();
        let file = root.path().join("future.yaml");
        std::fs::write(&file, "version: 999\nrules: []\n").unwrap();
        write_file_sidecar(&file, &file_checksum(&file).unwrap()).unwrap();
        let receiver = Journal::open(&dst_dir).unwrap();
        let err = import_rules(&receiver, &file).unwrap_err();
        assert!(
            err.to_string().contains("unsupported share version"),
            "{err}"
        );
    }

    #[test]
    fn import_skips_unverified_lifecycle() {
        let root = tempfile::tempdir().unwrap();
        let dst_dir = root.path().join("dst");
        std::fs::create_dir_all(&dst_dir).unwrap();
        let when = RuleWhen {
            stage: StageKind::LintFormat,
            pattern: "p".into(),
            lang: "rust".into(),
        };
        let then = RuleThen {
            strategy: RepairStrategy::AutoFix,
            skill: String::new(),
            diff_hint: "hint".into(),
        };
        let file_body = SharedRulesFile {
            version: SHARE_VERSION,
            rules: vec![
                SharedRule {
                    id: "r-draft-1".into(),
                    when: when.clone(),
                    then: then.clone(),
                    lifecycle: RuleLifecycle::Draft,
                    priority: 0,
                },
                SharedRule {
                    id: "r-cand-1".into(),
                    when,
                    then,
                    lifecycle: RuleLifecycle::Candidate,
                    priority: 30,
                },
            ],
        };
        let file = root.path().join("mixed.yaml");
        std::fs::write(&file, serde_yaml::to_string(&file_body).unwrap()).unwrap();
        write_file_sidecar(&file, &file_checksum(&file).unwrap()).unwrap();
        let receiver = Journal::open(&dst_dir).unwrap();
        let report = import_rules(&receiver, &file).unwrap();
        assert_eq!(report.imported, 1);
        assert_eq!(report.skipped_unverified, 1);
        assert!(receiver.get_learned_rule("r-cand-1").unwrap().is_some());
        assert!(receiver.get_learned_rule("r-draft-1").unwrap().is_none());
    }
}
