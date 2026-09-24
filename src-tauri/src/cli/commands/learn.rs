// Atlas OS — `atlas learn` Reflection Engine CLI (RFC 16 §2, RFC 32 Phase 5 sub-fase 5.1).
//
// Thin verbs over the Journal `learned_rules` (M30) writers that 5.0
// materialised and 5.1 formalises:
//
//   * `rules` — list newest rules (id, lifecycle, priority, was_correct, n_applied).
//   * `promote` — advance `draft → candidate → active` via
//     `Journal::promote_rule`, gated on `was_correct >= --min-correct`
//     (default `PROMOTE_THRESHOLD`) so a draft is only trusted after
//     repeated correct runs.
//   * `deprecate` — retire any lifecycle to `deprecated` (priority 0).

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::learning::PROMOTE_THRESHOLD;

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
}

pub async fn run(cmd: LearnCmd, profile: &str) -> Result<()> {
    match cmd.action {
        LearnAction::Rules { last } => list_rules(profile, last),
        LearnAction::Promote { id, min_correct } => promote_rule(profile, &id, min_correct),
        LearnAction::Deprecate { id } => deprecate_rule(profile, &id),
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
