// Atlas OS — `atlas swarm` role presets (RFC 05 §1/§3, Phase 4 sub-fase 4.1).
// `presets` lists the bundled agency-agents-port presets; `start` spawns one
// Journal-backed agent row per preset member with the tri-model slot resolved
// against the active profile (architect/editor/weak → effective_* fallback).

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
}

pub async fn run(cmd: SwarmCmd, profile: &str) -> Result<()> {
    match cmd.action {
        SwarmAction::Presets => list_presets(),
        SwarmAction::Start { preset, mission } => start_preset(&preset, &mission, profile),
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
