// Atlas OS — `atlas browser` terminal-browser lateral integration
// (RFC 28 §I items 2-7, lateral pattern 8.5).
//
// Read-only `probe`; shell-out `open`/`ls`/`action` that delegate to the
// external `terminal-browser` binary. Never a silent no-op: without the
// binary `open`/`ls`/`action` exit non-zero with the install recipe.

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::browser::{
    browser_guide, probe_report, resolve, run_capture, spawn_action, spawn_args_ls, spawn_open,
    BrowserAction, BrowserIntegrationError, BrowserStatus,
};

#[derive(Args, Debug)]
pub struct BrowserCmd {
    #[command(subcommand)]
    pub sub: BrowserSub,
}

#[derive(Subcommand, Debug)]
pub enum BrowserSub {
    /// Report install + terminal capability (read-only, never spawns the browser).
    Probe {
        /// Explicit binary path (overrides PATH + ATLAS_TERMINAL_BROWSER_BIN).
        #[arg(long)]
        bin: Option<String>,
    },
    /// Open a URL or local artifact in a terminal-browser pane.
    Open {
        /// URL or local path, e.g. https://example.com or ./plan.html.
        url: String,
        /// Split direction passed through to `terminal-browser open`.
        #[arg(long)]
        split: Option<String>,
        /// Mission id to attach the `artifact_preview_opened` event to.
        #[arg(long)]
        mission: Option<String>,
        /// Skip publishing the `artifact_preview_opened` journal event.
        #[arg(long, default_value_t = false)]
        no_publish: bool,
        /// Explicit binary path (overrides PATH + ATLAS_TERMINAL_BROWSER_BIN).
        #[arg(long)]
        bin: Option<String>,
    },
    /// List open terminal-browser panes (`terminal-browser ls`).
    Ls {
        #[arg(long)]
        bin: Option<String>,
    },
    /// Forward raw args to `terminal-browser action` (agent-browser contract, unpinned).
    Action {
        /// Args forwarded verbatim after the `action` verb.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        #[arg(long)]
        bin: Option<String>,
    },
    /// Print the lateral integration model + commands.
    Guide,
}

pub async fn run(cmd: BrowserCmd, profile: &str) -> Result<()> {
    match cmd.sub {
        BrowserSub::Probe { bin } => {
            println!("{}", probe_report(bin.as_deref()));
            Ok(())
        }
        BrowserSub::Guide => {
            println!("{}", browser_guide());
            Ok(())
        }
        BrowserSub::Open {
            url,
            split,
            mission,
            no_publish,
            bin,
        } => open(
            &url,
            split.as_deref(),
            mission.as_deref(),
            no_publish,
            bin.as_deref(),
            profile,
        ),
        BrowserSub::Ls { bin } => ls(bin.as_deref()),
        BrowserSub::Action { args, bin } => action(&args, bin.as_deref()),
    }
}

fn availability(bin_override: Option<&str>) -> Result<std::path::PathBuf> {
    match resolve(bin_override) {
        BrowserStatus::Available { bin } => Ok(bin),
        BrowserStatus::Missing => Err(anyhow::anyhow!(
            "{}",
            BrowserIntegrationError::NotInstalled.remediation()
        )),
    }
}

fn open(
    url: &str,
    split: Option<&str>,
    mission: Option<&str>,
    no_publish: bool,
    bin_override: Option<&str>,
    profile: &str,
) -> Result<()> {
    let url = url.trim();
    if url.is_empty() {
        anyhow::bail!("`browser open` needs a non-empty <url> (or local path)");
    }
    let bin = availability(bin_override)?;
    let capability = crate::browser::detect_capability();
    if !capability.is_kitty() {
        eprintln!("warning: {}", capability.summary());
    }
    let pid = spawn_open(&bin, url, split).map_err(|e| {
        anyhow::anyhow!(
            "{}",
            BrowserIntegrationError::SpawnFailed {
                message: e.to_string()
            }
            .remediation()
        )
    })?;
    let pid_txt = crate::profiles::ProfileId::new(profile);
    println!("browser [{pid_txt}] terminal-browser open spawned (pid {pid}): {url}");
    if !no_publish {
        if let Err(e) = publish_preview(profile, mission, url) {
            eprintln!("warning: artifact_preview_opened not persisted: {e}");
        }
    }
    Ok(())
}

fn publish_preview(profile: &str, mission: Option<&str>, url: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    let mission_id = match mission.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => Some(uuid::Uuid::parse_str(raw)?),
        None => None,
    };
    let event =
        crate::core::bus::BusEvent::new(crate::core::bus::BusEventKind::ArtifactPreviewOpened {
            mission_id,
            artifact: url.to_string(),
            url: url.to_string(),
            source: "atlas-cli".to_string(),
        });
    journal.publish(&event)?;
    Ok(())
}

fn ls(bin_override: Option<&str>) -> Result<()> {
    let bin = availability(bin_override)?;
    let out = run_capture(&bin, &spawn_args_ls()).map_err(|e| {
        anyhow::anyhow!(
            "{}",
            BrowserIntegrationError::SpawnFailed {
                message: e.to_string()
            }
            .remediation()
        )
    })?;
    if out.is_empty() {
        println!("browser: no open terminal-browser panes.");
    } else {
        println!("{out}");
    }
    Ok(())
}

fn action(args: &[String], bin_override: Option<&str>) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!(
            "`browser action` needs at least one argument (forwarded to terminal-browser action)"
        );
    }
    let bin = availability(bin_override)?;
    let mut full = vec!["action".to_string()];
    full.extend(args.iter().cloned());
    let out = run_capture(&bin, &full).map_err(|e| {
        anyhow::anyhow!(
            "{}",
            BrowserIntegrationError::SpawnFailed {
                message: e.to_string()
            }
            .remediation()
        )
    })?;
    if !out.is_empty() {
        println!("{out}");
    }
    Ok(())
}

/// Exposed for the Orchestrator's web tool (RFC 63 §4 `browse.open`):
/// spawn a typed action against the operator's open pane.
pub fn run_typed_action(action: &BrowserAction, bin_override: Option<&str>) -> Result<u32> {
    let bin = availability(bin_override)?;
    spawn_action(&bin, action).map_err(|e| {
        anyhow::anyhow!(
            "{}",
            BrowserIntegrationError::SpawnFailed {
                message: e.to_string()
            }
            .remediation()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_never_fails() {
        run(
            BrowserCmd {
                sub: BrowserSub::Probe {
                    bin: Some("/nonexistent-tb".to_string()),
                },
            },
            "nonexistent-profile-xyz",
        )
        .await
        .expect("probe is read-only and never errors");
    }

    #[tokio::test]
    async fn open_blank_url_is_rejected_before_io() {
        let err = run(
            BrowserCmd {
                sub: BrowserSub::Open {
                    url: "   ".to_string(),
                    split: None,
                    mission: None,
                    no_publish: true,
                    bin: Some("/nonexistent-tb".to_string()),
                },
            },
            "nonexistent-profile-xyz",
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("non-empty"));
    }

    #[tokio::test]
    async fn open_missing_binary_errors_with_remediation() {
        let err = run(
            BrowserCmd {
                sub: BrowserSub::Open {
                    url: "https://example.com".to_string(),
                    split: None,
                    mission: None,
                    no_publish: true,
                    bin: Some("/nonexistent-atlas-tb-xyz".to_string()),
                },
            },
            "nonexistent-profile-xyz",
        )
        .await
        .unwrap_err();
        assert!(err
            .to_string()
            .contains(crate::browser::TERMINAL_BROWSER_BIN));
    }

    #[tokio::test]
    async fn action_requires_args() {
        let err = run(
            BrowserCmd {
                sub: BrowserSub::Action {
                    args: vec![],
                    bin: Some("/nonexistent-tb".to_string()),
                },
            },
            "nonexistent-profile-xyz",
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("at least one argument"));
    }
}
