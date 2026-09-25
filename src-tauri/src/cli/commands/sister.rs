// Atlas OS — `atlas sister` Sister IDE-in-a-terminal frame (RFC 20 Phase
// 8 sub-fase 8.4, research 36 SECTOR B 8.4).
//
// Thin verb over `crate::sister`: renders the owned text-frame Document
// Model from journal state (same Kernel Bus data the HUD WebSocket
// serves). One-shot by default; `--watch` re-renders every poll tick so
// a thin PC over SSH gets the live board without a browser.
use anyhow::Result;
use clap::Args;

use crate::monitor::MONITOR_POLL_SECS;
use crate::sister::{build_snapshot, render_frame, SisterFeed};

#[derive(Args, Debug)]
pub struct SisterCmd {
    /// Re-render the frame every poll tick instead of printing once.
    #[arg(long, default_value_t = false)]
    pub watch: bool,
    /// Max frames to render in `--watch` (default 0 = run until Ctrl-C).
    #[arg(long, default_value_t = 0)]
    pub ticks: u64,
}

pub async fn run(cmd: SisterCmd, profile: &str) -> Result<()> {
    if cmd.ticks > 0 && !cmd.watch {
        anyhow::bail!("--ticks needs --watch (one-shot `atlas sister` renders a single frame)");
    }
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let hud_port: Option<u16> = std::fs::read_to_string(root.join("hud_port.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok());
    let render_once = || -> Result<()> {
        let journal = crate::journal::Journal::open(&root)?;
        let snap = build_snapshot(&journal, profile, hud_port)?;
        print!("{}", render_frame(&snap, &SisterFeed::default()));
        Ok(())
    };
    render_once()?;
    if !cmd.watch {
        return Ok(());
    }
    let mut tick = 1u64;
    loop {
        if cmd.ticks > 0 && tick >= cmd.ticks {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_secs(MONITOR_POLL_SECS));
        render_once()?;
        tick += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ticks_without_watch_is_rejected_before_any_io() {
        let err = run(
            SisterCmd {
                watch: false,
                ticks: 3,
            },
            "nonexistent-profile-xyz",
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("--ticks needs --watch"));
    }
}
