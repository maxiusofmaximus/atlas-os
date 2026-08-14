// Atlas OS — `atlas audit` (RFC 24 §10 / RFC 28 §D — Phase 1.5a §D-4).
//
// Subcommand surface:
//   * `opencode audit [-n N]`      — print tail (existing behavior)
//   * `opencode audit --verify`   — verify hash chain (RFC 24 §10; Phase 6 stub)
//   * `opencode audit --export-posting [-n N] [-o DIR]`
//                                  — export last N audit_log rows to
//                                    `<DIR>/YYYY-MM-DD/audit_<n>.posting.yaml`
//                                  (RFC 28 §D — Phase 1.5a)
//   * `opencode audit --snapshot-maybe`
//                                  — invoke retention worker manually;
//                                    used by cron planificado en supervisor.
//                                    Uses default snapshot_root
//                                    (<profile_root>/snapshots/).
//
// The export path uses the Posting `.posting.yaml` schema (RFC 28 §D — Phase
// 1.5a §D-2). The atomic-write + day-bucketing packing is handled by
// `journal::export::retention::snapshot_entries`. Per RFC 28 §D we ignore the
// posting `scripts` field on emit (security boundary AGENTS.md §6).

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::Args;

#[derive(Args, Debug)]
pub struct AuditCmd {
    /// Last N entries.
    #[arg(short = 'n', long, default_value_t = 50)]
    pub last: u32,

    /// Verify hash chain integrity (Phase 6, RFC 24 §10).
    #[arg(long)]
    pub verify: bool,

    /// Export last N audit entries to `.posting.yaml` snapshot files under DIR
    /// (RFC 28 Phase 1.5a §D). DIR defaults to <profile_root>/snapshots/.
    #[arg(long, value_name = "DIR", num_args = 0..=1, default_missing_value = "")]
    pub export_posting: Option<String>,

    /// Run the retention snapshot worker manually against the default
    /// <profile_root>/snapshots/ dir. Equivalent to `--export-posting` with the
    /// default dir and the last 100 audit entries — a convenience flag for
    /// the supervisor's cron planificado path.
    #[arg(long)]
    pub snapshot_maybe: bool,
}

pub async fn run(cmd: AuditCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;

    if cmd.verify {
        println!("audit hash-chain verify: feature arrives Phase 6 (RFC 24 §10).\n");
    }

    if cmd.snapshot_maybe {
        let snapshot_root_default = root.join("snapshots");
        let entries = journal.audit_tail(100)?;
        let cfg =
            crate::journal::export::retention::RetentionConfig::new(snapshot_root_default.clone());
        let result = crate::journal::export::retention::snapshot_entries(&entries, &cfg)
            .map_err(|e| anyhow!(e.to_string()))?;
        println!(
            "audit snapshot: {} entries packed into {} file(s) at {}",
            result.entries_packed,
            result.files_written.len(),
            snapshot_root_default.display()
        );
        for f in &result.files_written {
            println!("  {}", f.display());
        }
        return Ok(());
    }

    if let Some(arg) = cmd.export_posting.as_ref() {
        let snapshot_root = if arg.is_empty() {
            root.join("snapshots")
        } else {
            PathBuf::from(arg)
        };
        let entries = journal.audit_tail(i64::from(cmd.last))?;
        let cfg = crate::journal::export::retention::RetentionConfig::new(snapshot_root.clone());
        let result = crate::journal::export::retention::snapshot_entries(&entries, &cfg)
            .map_err(|e| anyhow!(e.to_string()))?;
        println!(
            "audit export-posting: {} entries packed into {} file(s) at {} (posting_version=1)",
            result.entries_packed,
            result.files_written.len(),
            snapshot_root.display()
        );
        for f in &result.files_written {
            println!("  {}", f.display());
        }
        if result.entries_purged > 0 {
            println!(
                "  ({} entries purged from SQLite — purge_after_export=true)",
                result.entries_purged
            );
        }
        return Ok(());
    }

    let entries = journal.audit_tail(i64::from(cmd.last))?;
    if entries.is_empty() {
        println!("(no audit entries yet for profile {pid})");
    } else {
        println!("audit tail [{pid}] (last {}):", entries.len());
        for e in entries {
            println!(
                "  #{seq} {ts}  actor={actor}  action={action}",
                seq = e.seq,
                ts = e.ts,
                actor = e.actor,
                action = e.action
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct Probe {
        #[command(flatten)]
        audit: AuditCmd,
    }

    #[test]
    fn default_flags_clamp_to_tail_only() {
        let p = Probe::try_parse_from(["probe"]).unwrap();
        assert_eq!(p.audit.last, 50);
        assert!(!p.audit.verify);
        assert!(p.audit.export_posting.is_none());
        assert!(!p.audit.snapshot_maybe);
    }

    #[test]
    fn export_posting_flag_no_dir_yields_empty_string_default() {
        let p = Probe::try_parse_from(["probe", "--export-posting"]).unwrap();
        assert!(p.audit.export_posting.is_some());
        assert_eq!(p.audit.export_posting.as_deref(), Some(""));
    }

    #[test]
    fn export_posting_flag_with_dir_captures_path() {
        let p = Probe::try_parse_from(["probe", "--export-posting", "/tmp/snapshots"]).unwrap();
        assert_eq!(p.audit.export_posting.as_deref(), Some("/tmp/snapshots"));
    }

    #[test]
    fn export_posting_flag_with_n_50_picks_up_both() {
        let p =
            Probe::try_parse_from(["probe", "-n", "100", "--export-posting", "./snap"]).unwrap();
        assert_eq!(p.audit.last, 100);
        assert_eq!(p.audit.export_posting.as_deref(), Some("./snap"));
    }

    #[test]
    fn snapshot_maybe_flag_parses_alone() {
        let p = Probe::try_parse_from(["probe", "--snapshot-maybe"]).unwrap();
        assert!(p.audit.snapshot_maybe);
        assert!(p.audit.export_posting.is_none());
    }

    #[test]
    fn verify_flag_parses_alone() {
        let p = Probe::try_parse_from(["probe", "--verify"]).unwrap();
        assert!(p.audit.verify);
    }

    #[test]
    fn verify_and_export_combine() {
        let p = Probe::try_parse_from(["probe", "--verify", "--export-posting"]).unwrap();
        assert!(p.audit.verify);
        assert!(p.audit.export_posting.is_some());
    }
}
