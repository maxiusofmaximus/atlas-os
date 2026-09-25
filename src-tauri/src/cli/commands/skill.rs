// Atlas OS — `atlas skill` verbs (RFC 06; `compress` landed Phase 5.2).
use anyhow::Result;
use clap::{Args, Subcommand};

use crate::learning::DEFAULT_COMPRESS_THRESHOLD;
use crate::skills::DEFAULT_PICK_THRESHOLD;

#[derive(Args, Debug)]
pub struct SkillCmd {
    #[command(subcommand)]
    pub action: SkillAction,
}

#[derive(Subcommand, Debug)]
pub enum SkillAction {
    List,
    Install {
        name: String,
    },
    Activate {
        agent_id: String,
        skill_id: String,
    },
    /// Detect redundant skills by Jaccard token-overlap (RFC 06 §5).
    /// Dry-run by default; `--apply` writes the fused `skill.toml`
    /// and marks the absorbed profile skill `DEPRECATED`.
    Compress {
        /// Minimum similarity in [0.0, 1.0] for a merge proposal.
        #[arg(long, default_value_t = DEFAULT_COMPRESS_THRESHOLD)]
        threshold: f64,
        /// Write fused manifests + DEPRECATED markers (default: dry-run).
        #[arg(long, default_value_t = false)]
        apply: bool,
    },
    /// Rank skills for a prompt: lit (suggested) vs dimmed (RFC 17 §4).
    Pick {
        /// Prompt describing the task (keyword match, deterministic).
        prompt: String,
        /// Minimum relevance in [0.0, 1.0] to light a skill up.
        #[arg(long, default_value_t = DEFAULT_PICK_THRESHOLD)]
        threshold: f64,
    },
}

pub async fn run(cmd: SkillCmd, profile: &str) -> Result<()> {
    match cmd.action {
        SkillAction::List => list_skills(profile)?,
        SkillAction::Install { name } => install_skill(profile, &name)?,
        SkillAction::Activate { agent_id, skill_id } => println!(
            "(activate skill '{skill_id}' on agent '{agent_id}' not implemented — Phase 5 RFC 06)"
        ),
        SkillAction::Compress { threshold, apply } => compress_skills(profile, threshold, apply)?,
        SkillAction::Pick { prompt, threshold } => pick_skills_cmd(profile, &prompt, threshold)?,
    }
    Ok(())
}

fn install_skill(profile: &str, name: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let dir = root.join("skills").join(name);
    if dir.is_dir() {
        crate::security::install_gate(&dir)?;
        let m = crate::skills::load_skill(&dir)?;
        println!(
            "skill '{name}' verified (checksum OK) v{ver}",
            ver = m.version
        );
    } else {
        println!("(install skill '{name}' not implemented — Phase 5 RFC 06)")
    }
    Ok(())
}

fn compress_skills(profile: &str, threshold: f64, apply: bool) -> Result<()> {
    if !(0.0..=1.0).contains(&threshold) {
        anyhow::bail!("--threshold must be within [0.0, 1.0], got {threshold}");
    }
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let skills_dir = root.join("skills");
    let manifests = crate::learning::collect_skills(&skills_dir)?;
    let proposals = crate::learning::find_compress_proposals(&manifests, threshold);
    println!(
        "skill compression [{pid}] ({} skills, threshold {threshold}): {} proposal(s)",
        manifests.len(),
        proposals.len()
    );
    for p in &proposals {
        println!(
            "  [~] keep `{keep}` <- merge `{merge}` (similarity {sim:.3})",
            keep = p.keep,
            merge = p.merge,
            sim = p.similarity,
        );
    }
    if proposals.is_empty() {
        return Ok(());
    }
    if !apply {
        println!("(dry-run: re-run with --apply to merge)");
        return Ok(());
    }
    let journal = crate::journal::Journal::open(&root)?;
    let applied =
        crate::learning::apply_proposals(&proposals, &manifests, &skills_dir, Some(&journal))?;
    for a in &applied {
        if let Some(reason) = &a.skipped_reason {
            println!("  [s] `{merge}` skipped: {reason}", merge = a.merge,);
        } else {
            println!(
                "  [+] `{keep}` fused <- `{merge}` (similarity {sim:.3})",
                keep = a.keep,
                merge = a.merge,
                sim = a.similarity,
            );
        }
    }
    Ok(())
}

fn list_skills(profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let skills_dir = root.join("skills");
    if !skills_dir.is_dir() {
        println!("(no skills installed for profile {pid})");
    } else {
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
    }

    let bundled = crate::skills::bundled::bundled_manifests().unwrap_or_default();
    println!("bundled [{}]:", bundled.len());
    let mut bundled_sorted = bundled;
    bundled_sorted.sort_by(|a, b| a.id.cmp(&b.id));
    for m in &bundled_sorted {
        println!(
            "  [=] {name:<20} v{ver}  {desc}",
            name = m.id,
            ver = m.version,
            desc = m.description
        );
    }
    Ok(())
}

fn pick_skills_cmd(profile: &str, prompt: &str, threshold: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&threshold) {
        anyhow::bail!("--threshold must be within [0.0, 1.0], got {threshold}");
    }
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let skills_dir = root.join("skills");
    let manifests = crate::learning::collect_skills(&skills_dir)?;
    if manifests.is_empty() {
        println!("(no skills available for profile {pid})");
        return Ok(());
    }
    let scored = crate::skills::picker::pick_skills_with_threshold(prompt, &manifests, threshold);
    let lit = scored.iter().filter(|s| s.suggested).count();
    println!(
        "skill pick [{pid}] \"{prompt}\" ({lit} suggested / {} dimmed, threshold {threshold}):",
        scored.len() - lit
    );
    for s in &scored {
        let mark = if s.suggested { "★" } else { "·" };
        let tag = if s.suggested { "Suggested" } else { "Dimmed" };
        println!(
            "  {mark} {name:<28} [{tag:<9}] relevance {rel:.3}  {desc}",
            name = s.manifest.id,
            rel = s.relevance,
            desc = s
                .manifest
                .summary
                .as_deref()
                .unwrap_or(s.manifest.description.as_str()),
        );
    }
    Ok(())
}
