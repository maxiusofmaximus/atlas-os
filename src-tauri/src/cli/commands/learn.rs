// Atlas OS — `atlas learn` Reflection Engine CLI (RFC 16 §2, RFC 32 Phase 5
// sub-fases 5.1 + 5.3, research/39 Phase 10 sub-fase 10.3 M40).
//
// Thin verbs over the Journal `learned_rules` (M30) writers that 5.0
// materialised and 5.1 formalises, plus the System One compaction tail
// (5.3) over `journal_events`:
//
//   * `rules` — list newest rules (id, lifecycle, priority, was_correct, n_applied).
//   * `promote` — advance `draft → candidate → active` via
//     `Journal::promote_rule`, gated on `was_correct >= --min-correct`
//     (default `PROMOTE_THRESHOLD`) so a draft is only trusted after
//     repeated correct runs.
//   * `deprecate` — retire any lifecycle to `deprecated` (priority 0).
//   * `compact` — compact a mission's `journal_events` tail once it
//     exceeds `COMPACTION_THRESHOLD` (rolling-window stub summary →
//     `compaction_events`).
//   * `summary` — print the latest compacted summary for a mission.
//   * `export` — comparte reglas VERIFICADAS (`candidate`/`active`, 5.1)
//     a YAML determinista + sidecar `<file>.checksum` (M40, research/39
//     10.3). Solo lo verificado sale; drafts nunca se comparten.
//   * `import` — valida la firma ANTES de parsear (Missing/Mismatch →
//     Forbidden fail-safe, patrón `approval_for` 7.1) e importa con
//     dedup por `rule_id` (first-write-wins, RFC 02 §3.1.2).

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::learning::{
    needs_compaction_with_threshold, summarize, COMPACTION_THRESHOLD, PROMOTE_THRESHOLD,
};

#[derive(Args, Debug)]
pub struct LearnCmd {
    #[command(subcommand)]
    pub action: LearnAction,
}

#[derive(Subcommand, Debug)]
pub enum LearnAction {
    /// List learned rules (newest first).
    Rules {
        /// Max rows to show.
        #[arg(short = 'n', long, default_value_t = 20)]
        last: u32,
    },
    /// Promote a rule one lifecycle step (draft → candidate → active).
    Promote {
        /// Rule id (`r-YYYY-MM-DD-NNN`).
        id: String,
        /// Minimum `was_correct` required before promoting.
        #[arg(long, default_value_t = PROMOTE_THRESHOLD)]
        min_correct: u32,
    },
    /// Retire a rule (any lifecycle → deprecated, priority 0).
    Deprecate {
        /// Rule id (`r-YYYY-MM-DD-NNN`).
        id: String,
    },
    /// Compact a mission's `journal_events` tail (System One, 5.3).
    /// No-op when the tail is at or below `--threshold`.
    Compact {
        /// Mission id (UUID string).
        #[arg(long)]
        mission: String,
        /// Tail length above which compaction fires.
        #[arg(long, default_value_t = COMPACTION_THRESHOLD)]
        threshold: usize,
    },
    /// Print the latest compacted summary for a mission (5.3).
    Summary {
        /// Mission id (UUID string).
        mission_id: String,
    },
    /// Export VERIFIED rules (`candidate`/`active`, 5.1) to a shareable
    /// YAML file + `<FILE>.checksum` sidecar (M40, research/39 10.3).
    /// Deterministic: same verified set → same bytes. Share both files
    /// (e.g. via git); `import` refuses the YAML without its sidecar.
    Export {
        /// Destination YAML file to write.
        #[arg(value_name = "FILE")]
        file: String,
    },
    /// Import a shared rules file (M40). Verifies the `<FILE>.checksum`
    /// signature BEFORE parsing (Missing/Mismatch → Forbidden) and
    /// dedups by `rule_id` (first-write-wins: existing ids are skipped).
    Import {
        /// Shared YAML file to import (sidecar `<FILE>.checksum` required).
        #[arg(value_name = "FILE")]
        file: String,
    },
}

pub async fn run(cmd: LearnCmd, profile: &str) -> Result<()> {
    match cmd.action {
        LearnAction::Rules { last } => list_rules(profile, last),
        LearnAction::Promote { id, min_correct } => promote_rule(profile, &id, min_correct),
        LearnAction::Deprecate { id } => deprecate_rule(profile, &id),
        LearnAction::Compact { mission, threshold } => {
            compact_mission(profile, &mission, threshold)
        }
        LearnAction::Summary { mission_id } => print_summary(profile, &mission_id),
        LearnAction::Export { file } => export_rules_cmd(profile, &file),
        LearnAction::Import { file } => import_rules_cmd(profile, &file),
    }
}

fn open_journal(profile: &str) -> Result<crate::journal::Journal> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    crate::journal::Journal::open(&root)
}

fn list_rules(profile: &str, last: u32) -> Result<()> {
    let journal = open_journal(profile)?;
    let rows = journal.list_learned_rules(i64::from(last))?;
    let pid = crate::profiles::ProfileId::new(profile);
    println!("learned rules [{pid}] ({}):", rows.len());
    for r in rows {
        let correct = r
            .was_correct
            .map_or_else(|| "-".to_string(), |v| v.to_string());
        println!(
            "  {id:<22} {lifecycle:<10} prio={prio:<3} correct={correct:<4} applied={applied}",
            id = r.id,
            lifecycle = r.lifecycle,
            prio = r.priority,
            applied = r.n_applied,
        );
    }
    Ok(())
}

fn promote_rule(profile: &str, id: &str, min_correct: u32) -> Result<()> {
    let journal = open_journal(profile)?;
    let current = journal
        .get_learned_rule(id)?
        .ok_or_else(|| anyhow::anyhow!("unknown learned rule `{id}`"))?;
    let was_correct: u32 = current.was_correct.unwrap_or(0).max(0) as u32;
    if was_correct < min_correct {
        anyhow::bail!(
            "rule `{id}` has was_correct={was_correct} below --min-correct={min_correct}; record passing runs first"
        );
    }
    let next = journal.promote_rule(id)?;
    println!(
        "promoted `{id}`: {} → {} (priority {})",
        current.lifecycle, next.lifecycle, next.priority
    );
    Ok(())
}

fn deprecate_rule(profile: &str, id: &str) -> Result<()> {
    let journal = open_journal(profile)?;
    let next = journal.deprecate_rule(id)?;
    println!("deprecated `{id}` (lifecycle {})", next.lifecycle);
    Ok(())
}

fn compact_mission(profile: &str, mission_id: &str, threshold: usize) -> Result<()> {
    let journal = open_journal(profile)?;
    let count = journal.mission_entry_count(mission_id)?;
    if !needs_compaction_with_threshold(count, threshold) {
        println!("no compaction needed for `{mission_id}` ({count} entries <= {threshold})");
        return Ok(());
    }
    let entries = journal.mission_entries(mission_id, count as i64)?;
    let summary = summarize(mission_id, &entries);
    let event_id = journal.save_compaction_event(&summary)?;
    println!(
        "compacted `{mission_id}`: {} → {} entries (event {event_id})",
        summary.entries_before, summary.entries_after,
    );
    println!("  {}", summary.summary);
    Ok(())
}

fn print_summary(profile: &str, mission_id: &str) -> Result<()> {
    let journal = open_journal(profile)?;
    match journal.compacted_summary(mission_id)? {
        Some(summary) => println!("compacted summary for `{mission_id}`:\n  {summary}"),
        None => println!("(no compaction recorded yet for `{mission_id}`)"),
    }
    Ok(())
}

fn export_rules_cmd(profile: &str, file: &str) -> Result<()> {
    let journal = open_journal(profile)?;
    let dest = std::path::PathBuf::from(file);
    let report = crate::learning::export_rules(&journal, &dest)?;
    let pid = crate::profiles::ProfileId::new(profile);
    println!(
        "exported {} verified rule(s) [{pid}] to {path} (checksum {sum})",
        report.count,
        path = report.path.display(),
        sum = &report.checksum[..12.min(report.checksum.len())],
    );
    println!("next step: share both `{file}` and `{file}.checksum` via git/repo — `atlas learn import` verifies BEFORE parsing (M40)");
    Ok(())
}

fn import_rules_cmd(profile: &str, file: &str) -> Result<()> {
    let journal = open_journal(profile)?;
    let src = std::path::PathBuf::from(file);
    let report = crate::learning::import_rules(&journal, &src)?;
    let pid = crate::profiles::ProfileId::new(profile);
    println!(
        "imported {imported} rule(s) [{pid}] from {path} ({dup} duplicate(s) skipped, {unv} unverified skipped)",
        imported = report.imported,
        path = report.path.display(),
        dup = report.skipped_duplicate,
        unv = report.skipped_unverified,
    );
    Ok(())
}
