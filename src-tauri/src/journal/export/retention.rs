// OpenCode OS — Journal export retention worker (RFC 28 §D — Phase 1.5a §D-3).
//
// SQLite-driven hook that exports aging audit_log rows to disk before purging
// them. Atomic write port is GR-004 (graphify/paths.py:_atomic_replace); here
// we use `tempfile` + `rename` directly (already a Rust idiom) — no new
// dependency required. The Windows fallback `copy + remove` for `rename`
// failures is handled by `std::fs::rename`'s existing behavior on the same
// drive; cross-drive paths return an error we surface to the caller.

use std::path::PathBuf;

use thiserror::Error;

use crate::journal::export::posting::entry_to_posting_yaml;
use crate::journal::AuditEntry;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("snapshot dir creation failed at {0}: {1}")]
    CreateDir(PathBuf, #[source] std::io::Error),
    #[error("atomic write temp file creation failed at {0}: {1}")]
    TempCreate(PathBuf, #[source] std::io::Error),
    #[error("atomic write final rename failed from {from} to {to}: {source}")]
    Rename {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("YAML serialization for entry seq={0} failed: {1}")]
    Yaml(i64, #[source] serde_yaml::Error),
}

#[derive(Debug, Clone)]
pub struct RetentionConfig {
    /// Maximum number of entries per snapshot file. Packing hits this limit
    /// rolls a new file with `_n` suffix (RFC 28 §D — "bundle by day, max
    /// 100 entries/arch").
    pub max_entries_per_file: usize,
    /// Directory under which day-stamped subdirs are written. Typically
    /// `~/.opencode/snapshots/`.
    pub snapshot_root: PathBuf,
    /// Export-only flag (Phase 1.5a). When `purge_after_export` is true the
    /// retention worker also `DELETE FROM audit_log WHERE seq IN (...)` after
    /// successful export. Default false during §D rollout — we want users to
    /// verify the snapshots are usable before we let the worker delete rows.
    pub purge_after_export: bool,
}

impl RetentionConfig {
    pub fn new(snapshot_root: PathBuf) -> Self {
        Self {
            max_entries_per_file: 100,
            snapshot_root,
            purge_after_export: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotResult {
    pub files_written: Vec<PathBuf>,
    pub entries_packed: usize,
    pub entries_purged: usize,
}

/// Export a slice of `AuditEntry` rows to disk as `.posting.yaml` files.
///
/// Packing: all entries whose `ts` falls on the same calendar day go into the
/// same `<YYYY-MM-DD>/audit_<n>.posting.yaml` file (1-indexed), capped at
/// `cfg.max_entries_per_file` per file. Day buckets are derived from the
/// entry's `ts` prefix `[..10]` — RFC 02 §3.4 guarantees audit_log TS format
/// is ISO 8601 UTC `YYYY-MM-DDTHH:MM:SSZ`, so the slice is stable.
///
/// Atomic write: each file is written to `<name>.tmp` then `rename`'d. On
/// Windows, failures in `rename` across drives surface as `ExportError::Rename`
/// — the caller is expected to choose a snapshot_root on the same drive as
/// the user's home dir, which the `~/.opencode/snapshots/` default satisfies.
pub fn snapshot_entries(
    entries: &[AuditEntry],
    cfg: &RetentionConfig,
) -> Result<SnapshotResult, ExportError> {
    if entries.is_empty() {
        return Ok(SnapshotResult {
            files_written: Vec::new(),
            entries_packed: 0,
            entries_purged: 0,
        });
    }

    // Bucket by day key (entry.ts[..10]).
    let mut day_buckets: Vec<(String, Vec<&AuditEntry>)> = Vec::new();
    for entry in entries {
        let day_key = entry.ts.get(..10).unwrap_or("unknown").to_string();
        match day_buckets.iter_mut().find(|(k, _)| *k == day_key) {
            Some((_, bucket)) => bucket.push(entry),
            None => day_buckets.push((day_key, vec![entry])),
        }
    }

    let mut files_written = Vec::new();
    let mut entries_packed = 0usize;
    for (day, bucket) in day_buckets {
        let day_dir = cfg.snapshot_root.join(&day);
        std::fs::create_dir_all(&day_dir)
            .map_err(|e| ExportError::CreateDir(day_dir.clone(), e))?;
        let chunks: Vec<Vec<&AuditEntry>> = bucket
            .chunks(cfg.max_entries_per_file)
            .map(|c| c.to_vec())
            .collect();
        for (idx, chunk) in chunks.into_iter().enumerate() {
            let file_name = format!("audit_{}.posting.yaml", idx + 1);
            let target = day_dir.join(&file_name);
            let tmp = day_dir.join(format!("{}.tmp", file_name));
            let mut combined_yaml = String::new();
            for entry in &chunk {
                combined_yaml.push_str("---\n");
                combined_yaml.push_str(&entry_to_posting_yaml(entry));
                combined_yaml.push('\n');
            }
            std::fs::write(&tmp, &combined_yaml)
                .map_err(|e| ExportError::TempCreate(tmp.clone(), e))?;
            std::fs::rename(&tmp, &target).map_err(|e| ExportError::Rename {
                from: tmp.clone(),
                to: target.clone(),
                source: e,
            })?;
            files_written.push(target);
            entries_packed += chunk.len();
        }
    }

    Ok(SnapshotResult {
        files_written,
        entries_packed,
        entries_purged: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::AuditEntry;

    fn make_entry(seq: i64, ts: &str, action: &str) -> AuditEntry {
        AuditEntry {
            seq,
            ts: ts.to_string(),
            actor: "test".to_string(),
            action: action.to_string(),
            inputs: serde_json::json!({}),
            outputs: serde_json::Value::Null,
        }
    }

    #[test]
    fn empty_entries_returns_empty_result() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = RetentionConfig::new(dir.path().to_path_buf());
        let result = snapshot_entries(&[], &cfg).unwrap();
        assert!(result.files_written.is_empty());
        assert_eq!(result.entries_packed, 0);
    }

    #[test]
    fn single_entry_writes_single_file_in_day_bucket() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = RetentionConfig::new(dir.path().to_path_buf());
        let entries = vec![make_entry(1, "2026-07-25T20:00:00Z", "http_request")];
        let result = snapshot_entries(&entries, &cfg).unwrap();
        assert_eq!(result.files_written.len(), 1);
        assert_eq!(result.entries_packed, 1);
        let written_path = &result.files_written[0];
        assert!(written_path
            .to_string_lossy()
            .contains("2026-07-25\\audit_1.posting.yaml"));
        let content = std::fs::read_to_string(written_path).unwrap();
        assert!(content.contains("# x-opencode-exported: RFC 28 §D"));
        assert!(content.contains("posting_version: '1'"));
    }

    #[test]
    fn splits_chunks_over_max_entries_per_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = RetentionConfig::new(dir.path().to_path_buf());
        cfg.max_entries_per_file = 2;
        let entries: Vec<AuditEntry> = (0..5)
            .map(|i| make_entry(i + 1, "2026-07-25T20:00:00Z", "http_request"))
            .collect();
        let result = snapshot_entries(&entries, &cfg).unwrap();
        assert_eq!(result.files_written.len(), 3);
        assert_eq!(result.entries_packed, 5);
        for path in &result.files_written {
            assert!(path.to_string_lossy().contains("2026-07-25"));
        }
    }

    #[test]
    fn buckets_by_day_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = RetentionConfig::new(dir.path().to_path_buf());
        let entries = vec![
            make_entry(1, "2026-07-24T23:59:00Z", "http_request"),
            make_entry(2, "2026-07-25T00:01:00Z", "http_request"),
            make_entry(3, "2026-07-25T23:59:00Z", "http_request"),
        ];
        let result = snapshot_entries(&entries, &cfg).unwrap();
        assert_eq!(result.files_written.len(), 2);
        let day_dirs: Vec<String> = result
            .files_written
            .iter()
            .map(|p| {
                p.parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert!(day_dirs.contains(&"2026-07-24".to_string()));
        assert!(day_dirs.contains(&"2026-07-25".to_string()));
    }

    #[test]
    fn writes_each_entry_as_yaml_document_with_separator() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = RetentionConfig::new(dir.path().to_path_buf());
        let entries = vec![
            make_entry(1, "2026-07-25T01:00:00Z", "http_request"),
            make_entry(2, "2026-07-25T02:00:00Z", "http_request"),
        ];
        let result = snapshot_entries(&entries, &cfg).unwrap();
        let content = std::fs::read_to_string(&result.files_written[0]).unwrap();
        let doc_markers = content.matches("---").count();
        // One leading --- per entry, plus the embedded header content uses --- in
        // body strings occasionally; we only assert at least 2 leading separators.
        assert!(doc_markers >= 2);
    }

    #[test]
    fn atomic_file_rename_leaves_no_tmp_residue() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = RetentionConfig::new(dir.path().to_path_buf());
        let entries = vec![make_entry(1, "2026-07-25T01:00:00Z", "http_request")];
        let result = snapshot_entries(&entries, &cfg).unwrap();
        let day_dir = dir.path().join("2026-07-25");
        let tmp_files: Vec<_> = std::fs::read_dir(&day_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(
            tmp_files.is_empty(),
            "expected no .tmp residue after atomic rename, found {:?}",
            tmp_files
        );
        let _ = result;
    }

    #[test]
    fn reject_scripts_in_emit_when_third_party_yaml_imported() {
        // Sanity: our exported YAML should NEVER contain a top-level "scripts:" key.
        let dir = tempfile::tempdir().unwrap();
        let cfg = RetentionConfig::new(dir.path().to_path_buf());
        let entries = vec![make_entry(1, "2026-07-25T01:00:00Z", "http_request")];
        let result = snapshot_entries(&entries, &cfg).unwrap();
        let content = std::fs::read_to_string(&result.files_written[0]).unwrap();
        // Look for `scripts:` as a top-level YAML key (not nested inside a body string).
        let lines = content.lines();
        let mut found_top_level_scripts = false;
        for line in lines {
            if line.starts_with("scripts:") {
                found_top_level_scripts = true;
                break;
            }
            if !line.starts_with(' ')
                && !line.starts_with('-')
                && !line.starts_with('#')
                && !line.is_empty()
                && !line.starts_with("---")
            {
                // reached another top-level key; keep scanning in case scripts appears later
            }
        }
        assert!(
            !found_top_level_scripts,
            "exported YAML must NOT contain top-level scripts: key (AGENTS.md §6 security boundary)"
        );
    }
}
