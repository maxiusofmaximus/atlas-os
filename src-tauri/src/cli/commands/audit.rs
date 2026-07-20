// OpenCode OS — `opencode audit` stub (RFC 24 §10; full impl Phase 6).
use anyhow::Result;
use clap::Args;

#[derive(Args, Debug)]
pub struct AuditCmd {
    /// Last N entries.
    #[arg(short = 'n', long, default_value_t = 50)]
    pub last: u32,

    /// Verify hash chain integrity (Phase 6, RFC 24 §10).
    #[arg(long)]
    pub verify: bool,
}

pub async fn run(cmd: AuditCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    let entries = journal.audit_tail(i64::from(cmd.last))?;
    if cmd.verify {
        println!("audit hash-chain verify: feature arrives Phase 6 (RFC 24 §10).\n");
    }
    if entries.is_empty() {
        println!("(no audit entries yet for profile {pid})");
    } else {
        println!("audit tail [{pid}] (last {}):", entries.len());
        for e in entries {
            println!(
                "  #{seq} {ts}  actor={actor}  action={action}",
                seq = e.seq,
                ts = e.ts,
                actor = e.actor,
                action = e.action
            );
        }
    }
    Ok(())
}
