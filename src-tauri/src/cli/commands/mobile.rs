// Atlas OS — `atlas mobile` artemis lateral 8.6 (RFC 20 Fase 11
// sub-fase 11.0, research 40 SECTOR B 11.0).
//
// Thin verb over `crate::mobile`: detects the external artemis/uv
// entry point (lateral Python — never bundled/linked), prints the
// setup guide, and spawns `uv run artemis run` / `artemis run` via
// `std::process` only on explicit `run`. Without artemis/uv every verb
// degrades to the useful-message path instead of failing.

use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

use crate::mobile::mcp_template::{
    default_repo_dir, merge_into_opencode_config, render_full_file, render_guide,
};
use crate::mobile::{
    find_artemis_in_path, find_uv_in_path, missing_message, mobile_guide, normalize_task, resolve,
    setup_steps, spawn_session, MobileProfile, MobileStatus, ARTEMIS_REPO_ENV,
};

#[derive(Args, Debug)]
pub struct MobileCmd {
    #[command(subcommand)]
    pub sub: MobileSub,
}

#[derive(Subcommand, Debug)]
pub enum MobileSub {
    /// Detect artemis/uv in PATH / ATLAS_ARTEMIS_BIN (never installs it).
    Status,
    /// Print the artemis lateral model + setup steps.
    Guide,
    /// Run a natural-language task on the Android device/emulator.
    Run {
        /// Natural-language task, e.g. "open settings and toggle wifi".
        #[arg(long)]
        task: String,
        /// Execution profile: flash (reactive) or pro (multi-agent).
        #[arg(long, default_value = "flash")]
        profile: String,
        /// Explicit artemis/uv binary path (overrides PATH + ATLAS_ARTEMIS_BIN).
        #[arg(long)]
        bin: Option<String>,
        /// Explicit artemis repo dir for `uv run` (overrides ATLAS_ARTEMIS_REPO).
        #[arg(long)]
        repo: Option<String>,
    },
    /// Print (or --write) the `.opencode/mcp.json` artemis wiring template (RFC 20 Fase 11.1).
    McpTemplate {
        /// Merge the artemis entry into the target mcp.json instead of only printing.
        #[arg(long, default_value_t = false)]
        write: bool,
        /// Target mcp.json path (default `.opencode/mcp.json` under cwd).
        #[arg(long)]
        path: Option<String>,
        /// Artemis repo dir baked into the template (overrides ATLAS_ARTEMIS_REPO).
        #[arg(long)]
        repo: Option<String>,
    },
}

pub async fn run(cmd: MobileCmd, profile: &str) -> Result<()> {
    match cmd.sub {
        MobileSub::Status => status(profile),
        MobileSub::Guide => guide(),
        MobileSub::Run {
            task,
            profile: p,
            bin,
            repo,
        } => run_task(
            task.as_str(),
            p.as_str(),
            bin.as_deref(),
            repo.as_deref(),
            profile,
        ),
        MobileSub::McpTemplate { write, path, repo } => {
            mcp_template(write, path.as_deref(), repo.as_deref(), profile)
        }
    }
}

fn status(profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    match resolve(None) {
        MobileStatus::Available { bin, via_uv, repo } => {
            let how = if via_uv { "via uv" } else { "direct" };
            let repo_txt = repo
                .map(|r| r.display().to_string())
                .unwrap_or_else(|| "(no ATLAS_ARTEMIS_REPO — uv resolves from cwd)".to_string());
            println!(
                "mobile [{pid}] artemis available ({how}): {}",
                bin.display()
            );
            println!("repo: {repo_txt}");
            println!("profiles: flash (reactive) | pro (multi-agent) — see `atlas mobile guide`.");
            Ok(())
        }
        MobileStatus::Missing => {
            println!(
                "mobile [{pid}] artemis missing (artemis: {}, uv: {})",
                path_hit(find_artemis_in_path().is_some()),
                path_hit(find_uv_in_path().is_some())
            );
            println!("{}", missing_message());
            Ok(())
        }
    }
}

fn path_hit(hit: bool) -> &'static str {
    if hit {
        "found"
    } else {
        "not found"
    }
}

fn guide() -> Result<()> {
    println!("{}", mobile_guide());
    println!();
    println!("{}", setup_steps());
    Ok(())
}

fn run_task(
    task: &str,
    profile_raw: &str,
    bin_override: Option<&str>,
    repo_override: Option<&str>,
    cli_profile: &str,
) -> Result<()> {
    let task = normalize_task(task).ok_or_else(|| {
        anyhow::anyhow!("--task is blank — describe the Android task, e.g. --task \"open settings and toggle wifi\"")
    })?;
    let prof = MobileProfile::parse(profile_raw).ok_or_else(|| {
        anyhow::anyhow!("--profile `{profile_raw}` invalid (expected `flash` or `pro`)")
    })?;
    let pid = crate::profiles::ProfileId::new(cli_profile);
    match resolve(bin_override) {
        MobileStatus::Available { bin, via_uv, repo } => {
            let repo = repo_override
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .or(repo);
            let child = spawn_session(&bin, via_uv, repo.as_ref(), prof, &task)?;
            let how = if via_uv { "uv run artemis" } else { "artemis" };
            println!(
                "mobile [{pid}] {how} spawned (pid {child}, profile {}): {task}",
                prof.as_str()
            );
            Ok(())
        }
        MobileStatus::Missing => {
            println!("mobile [{pid}] artemis missing — task not launched (`{task}`)");
            println!("{}", missing_message());
            Ok(())
        }
    }
}

fn mcp_template(
    write: bool,
    path: Option<&str>,
    repo: Option<&str>,
    cli_profile: &str,
) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(cli_profile);
    let repo_dir = repo
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(default_repo_dir);
    let template = render_full_file(&repo_dir);
    if !write {
        println!("{template}");
        println!();
        println!("{}", render_guide(&repo_dir));
        return Ok(());
    }
    let target = path
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".opencode").join("mcp.json"));
    let existing = std::fs::read_to_string(&target).unwrap_or_else(|_| "{}".to_string());
    let merged = merge_into_opencode_config(&existing, &repo_dir)?;
    if let Some(parent) = target.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, format!("{merged}\n"))?;
    println!(
        "mobile [{pid}] mcp-template merged `artemis` (repo `{repo_dir}`, env {repo_env}) into {}",
        target.display(),
        repo_env = ARTEMIS_REPO_ENV
    );
    println!("{}", render_guide(&repo_dir));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn blank_task_is_rejected_before_any_io() {
        let err = run(
            MobileCmd {
                sub: MobileSub::Run {
                    task: "   ".to_string(),
                    profile: "flash".to_string(),
                    bin: None,
                    repo: None,
                },
            },
            "nonexistent-profile-xyz",
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("--task is blank"));
    }

    #[tokio::test]
    async fn invalid_profile_is_rejected_before_any_io() {
        let err = run(
            MobileCmd {
                sub: MobileSub::Run {
                    task: "open settings".to_string(),
                    profile: "ultra".to_string(),
                    bin: None,
                    repo: None,
                },
            },
            "nonexistent-profile-xyz",
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("--profile"));
    }

    #[tokio::test]
    async fn mcp_template_prints_without_touching_disk() {
        run(
            MobileCmd {
                sub: MobileSub::McpTemplate {
                    write: false,
                    path: None,
                    repo: Some("/tmp/artemis-clone".to_string()),
                },
            },
            "nonexistent-profile-xyz",
        )
        .await
        .expect("print path never fails");
    }

    #[tokio::test]
    async fn mcp_template_write_merges_and_preserves_existing() {
        let dir = std::env::temp_dir().join(format!(
            "atlas-mobile-mcp-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("mcp.json");
        std::fs::write(&target, r#"{"mcp":{"context7":{"type":"stdio"}}}"#).unwrap();
        run(
            MobileCmd {
                sub: MobileSub::McpTemplate {
                    write: true,
                    path: Some(target.display().to_string()),
                    repo: Some("/tmp/artemis-clone".to_string()),
                },
            },
            "nonexistent-profile-xyz",
        )
        .await
        .expect("write path succeeds");
        let back = std::fs::read_to_string(&target).unwrap();
        let v: serde_json::Value = serde_json::from_str(&back).unwrap();
        assert!(v.pointer("/mcp/context7").is_some());
        assert!(v.pointer("/mcp/artemis/command").is_some());
        std::fs::remove_dir_all(&dir).ok();
    }
}
