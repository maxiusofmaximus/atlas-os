// Atlas OS — `atlas-tui` binary (RFC 20 Phase 8 sub-fase 8.4, research
// 36 SECTOR B 8.4, research 28 SECTOR B).
//
// Standalone Sister IDE-in-a-terminal entry point for thin PCs over SSH:
// renders the same owned text-frame Document Model as `atlas sister`
// (same Kernel Bus data the HUD WebSocket serves). Compiled only with
// `--features tui` (`required-features` in Cargo.toml); default builds
// skip it. The future alternate-screen renderer (ratatui, post-audit)
// reuses `crate::sister::{build_snapshot, render_frame}` unchanged.
use atlas_os::sister::{build_snapshot, render_frame, SisterFeed};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "atlas-tui", about = "Atlas OS Sister IDE-in-a-terminal")]
struct TuiCli {
    #[arg(long, global = true, env = "OC_PROFILE")]
    profile: Option<String>,
    #[arg(long, default_value_t = false)]
    watch: bool,
    #[arg(long, default_value_t = 0)]
    ticks: u64,
}

fn main() -> anyhow::Result<()> {
    let cli = TuiCli::parse();
    if cli.ticks > 0 && !cli.watch {
        anyhow::bail!("--ticks needs --watch");
    }
    let profile = cli
        .profile
        .unwrap_or_else(|| atlas_os::profiles::current().0);
    let pid = atlas_os::profiles::ProfileId::new(&profile);
    let root = atlas_os::profiles::resolve_root(&pid)?;
    let hud_port: Option<u16> = std::fs::read_to_string(root.join("hud_port.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok());
    let render_once = || -> anyhow::Result<()> {
        let journal = atlas_os::journal::Journal::open(&root)?;
        let snap = build_snapshot(&journal, &profile, hud_port)?;
        print!("{}", render_frame(&snap, &SisterFeed::default()));
        Ok(())
    };
    render_once()?;
    if !cli.watch {
        return Ok(());
    }
    let mut tick = 1u64;
    loop {
        if cli.ticks > 0 && tick >= cli.ticks {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_secs(
            atlas_os::monitor::MONITOR_POLL_SECS,
        ));
        render_once()?;
        tick += 1;
    }
}
