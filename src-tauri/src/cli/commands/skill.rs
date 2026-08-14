// Atlas OS — `atlas skill` stub (RFC 06; full impl Phase 5).
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct SkillCmd {
    #[command(subcommand)]
    pub action: SkillAction,
}

#[derive(Subcommand, Debug)]
pub enum SkillAction {
    List,
    Install { name: String },
    Activate { agent_id: String, skill_id: String },
}

pub async fn run(cmd: SkillCmd, profile: &str) -> Result<()> {
    match cmd.action {
        SkillAction::List => list_skills(profile)?,
        SkillAction::Install { name } => {
            println!("(install skill '{name}' not implemented — Phase 5 RFC 06)")
        }
        SkillAction::Activate { agent_id, skill_id } => println!(
            "(activate skill '{skill_id}' on agent '{agent_id}' not implemented — Phase 5 RFC 06)"
        ),
    }
    Ok(())
}

fn list_skills(profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let skills_dir = root.join("skills");
    if !skills_dir.is_dir() {
        println!("(no skills installed for profile {pid})");
        return Ok(());
    }

    // Collect entries first so the listing is a stable, sorted view.
    let mut entries: Vec<_> = std::fs::read_dir(&skills_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    println!("skills [{pid}] ({}):", entries.len());
    for entry in entries {
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let dir = entry.path();
        match crate::skills::load_skill(&dir) {
            Ok(m) => {
                println!(
                    "  [+] {name:<20} v{ver}  {desc}",
                    name = m.id,
                    ver = m.version,
                    desc = m.description
                );
                if !m.capabilities.is_empty() {
                    println!("       capabilities: {}", m.capabilities.join(", "));
                }
            }
            Err(err) => {
                println!("  [-] {name:<20}  manifest invalid: {err}");
            }
        }
    }
    Ok(())
}
