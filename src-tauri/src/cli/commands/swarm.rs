// Atlas OS — `atlas swarm` role presets + agent mailbox (RFC 05 §1/§3,
// Phase 4 sub-fases 4.1/4.3).
// `presets` lists the bundled agency-agents-port presets; `start` spawns one
// Journal-backed agent row per preset member with the tri-model slot resolved
// against the active profile (architect/editor/weak → effective_* fallback).
// `send` / `inbox` drive the munder-difflin mailbox (research/31 §A.2) over
// the M29 `agent_mailbox` table: agent-to-agent coordination outside the
// Kernel Bus broadcast.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use uuid::Uuid;

#[derive(Args, Debug)]
pub struct SwarmCmd {
    #[command(subcommand)]
    pub action: SwarmAction,
}

#[derive(Subcommand, Debug)]
pub enum SwarmAction {
    /// List bundled role presets (agency-agents port).
    Presets,
    /// Spawn one swarm agent per preset member on a mission.
    Start {
        /// Preset id: `atlas-team`, `pair-programming` or `solo-plus`.
        #[arg(long)]
        preset: String,
        /// Mission id the agents attach to.
        #[arg(long)]
        mission: String,
    },
    /// Send a mailbox message from one agent to another.
    Send {
        /// Sender `swarm_agents.id`.
        from: String,
        /// Recipient `swarm_agents.id`.
        to: String,
        /// Message payload (JSON by convention, e.g. `{"text":"…"}`).
        body: String,
    },
    /// Show an agent's mailbox inbox.
    Inbox {
        /// Agent `swarm_agents.id` whose inbox is listed.
        agent: String,
        /// Only list unread messages.
        #[arg(long)]
        unread_only: bool,
        /// Mark every listed message read after printing.
        #[arg(long)]
        mark_read: bool,
    },
    /// Jump to a mission/worktree dir by frecency rank (8.0 zoxide port).
    Jump {
        /// Prefix/substring filter over recorded dirs (empty = all).
        #[arg(default_value = "")]
        prefix: String,
        /// Max matches to show.
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
}

pub async fn run(cmd: SwarmCmd, profile: &str) -> Result<()> {
    match cmd.action {
        SwarmAction::Presets => list_presets(),
        SwarmAction::Start { preset, mission } => start_preset(&preset, &mission, profile),
        SwarmAction::Send { from, to, body } => send_message(&from, &to, &body, profile),
        SwarmAction::Inbox {
            agent,
            unread_only,
            mark_read,
        } => show_inbox(&agent, unread_only, mark_read, profile),
        SwarmAction::Jump { prefix, limit } => jump(&prefix, limit, profile),
    }
}

fn list_presets() -> Result<()> {
    let presets = crate::swarm::bundled_presets();
    println!("swarm presets ({}):", presets.len());
    for p in &presets {
        println!("  [=] {:<18} {}", p.id, p.description);
        for m in &p.members {
            println!(
                "       - {:<10} slot={:<9} {personality}",
                m.role.as_str(),
                m.model_slot.as_str(),
                personality = truncate(&m.personality, 72),
            );
        }
    }
    Ok(())
}

fn start_preset(preset_id: &str, mission_id: &str, profile: &str) -> Result<()> {
    if mission_id.trim().is_empty() {
        anyhow::bail!("mission id must not be empty");
    }
    let Some(preset) = crate::swarm::find_preset(preset_id) else {
        anyhow::bail!(
            "unknown preset '{preset_id}' — run `atlas swarm presets` to list bundled presets"
        );
    };
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let prof = crate::profiles::Profile::load(&root, pid.clone())
        .with_context(|| format!("loading profile {pid}"))?;
    let journal = crate::journal::Journal::open(&root)?;

    println!(
        "spawning preset '{id}' on mission '{mission_id}' [{pid}] ({n} agents):",
        id = preset.id,
        n = preset.members.len(),
    );
    for member in &preset.members {
        let agent_id = Uuid::new_v4();
        let model = member.role.resolve_model(&prof);
        let personality = member.personality_json();
        journal.register_swarm_agent(
            agent_id,
            mission_id,
            member.role,
            model.as_deref(),
            Some(&personality),
        )?;
        println!(
            "  [+] {agent_id}  role={:<10} slot={:<9} model={}",
            member.role.as_str(),
            member.model_slot.as_str(),
            model.as_deref().unwrap_or("(registry default)"),
        );
    }
    Ok(())
}

fn open_journal(profile: &str) -> Result<crate::journal::Journal> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    crate::journal::Journal::open(&root)
}

fn parse_agent_id(raw: &str, role: &str) -> Result<Uuid> {
    Uuid::parse_str(raw).with_context(|| format!("{role} agent id is not a UUID: '{raw}'"))
}

fn send_message(from: &str, to: &str, body: &str, profile: &str) -> Result<()> {
    let from_id = parse_agent_id(from, "sender")?;
    let to_id = parse_agent_id(to, "recipient")?;
    if body.trim().is_empty() {
        anyhow::bail!("message body must not be empty");
    }
    let journal = open_journal(profile)?;
    let id = Uuid::new_v4();
    journal.send_message(id, from_id, to_id, body)?;
    println!("sent {id}  {from_id} → {to_id}");
    Ok(())
}

fn show_inbox(agent: &str, unread_only: bool, mark_read: bool, profile: &str) -> Result<()> {
    let agent_id = parse_agent_id(agent, "inbox")?;
    let journal = open_journal(profile)?;
    let messages = if unread_only {
        journal.unread_inbox_for(agent_id)?
    } else {
        journal.inbox_for(agent_id)?
    };
    let unread = journal.unread_count(agent_id)?;
    println!(
        "inbox {agent_id} ({} shown, {unread} unread):",
        messages.len()
    );
    for m in &messages {
        let flag = if m.is_read() { "read  " } else { "UNREAD" };
        println!(
            "  [{flag}] {}  from={}  {}",
            m.id,
            m.from_agent,
            truncate(&m.body_json, 96),
        );
    }
    if mark_read {
        let mut marked = 0;
        for m in &messages {
            if !m.is_read() && journal.mark_read(m.id)? {
                marked += 1;
            }
        }
        println!("marked {marked} read");
    }
    Ok(())
}

fn jump(prefix: &str, limit: u32, profile: &str) -> Result<()> {
    let journal = open_journal(profile)?;
    let hits = journal.frecency(prefix, i64::from(limit))?;
    if hits.is_empty() {
        println!("no recorded dirs match {prefix:?} — dirs are recorded via Journal::record_dir_access on worktree/mission access");
        return Ok(());
    }
    println!("jump {prefix:?} ({} matches):", hits.len());
    for (dir, score) in &hits {
        println!("  {score:8.2}  {dir}");
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    let end = s
        .char_indices()
        .take_while(|(i, _)| *i < max)
        .map(|(i, c)| i + c.len_utf8())
        .last()
        .unwrap_or(0);
    if end >= s.len() {
        return s.to_string();
    }
    format!("{}…", s[..end].trim_end())
}
