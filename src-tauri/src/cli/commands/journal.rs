// OpenCode OS — `opencode journal` tail command (RFC 19).
use anyhow::Result;
use clap::Args;

#[derive(Args, Debug)]
pub struct JournalCmd {
    /// Last N entries.
    #[arg(short = 'n', long, default_value_t = 50)]
    pub last: u32,
    /// Filter by event kind.
    #[arg(short = 'k', long)]
    pub kind: Option<String>,
}

pub async fn run(cmd: JournalCmd, profile: &str) -> Result<()> {
    let pid = crate::profiles::ProfileId::new(profile);
    let root = crate::profiles::resolve_root(&pid)?;
    let journal = crate::journal::Journal::open(&root)?;
    let entries = journal.tail(i64::from(cmd.last))?;
    println!("journal [{pid}] (last {}):", entries.len());
    for e in entries {
        if let Some(filter) = &cmd.kind {
            if !e.kind.contains(filter) {
                continue;
            }
        }
        println!("  #{id} {ts}  {kind}", id = e.id, ts = e.ts, kind = e.kind);
        if let Some(obj) = e.payload.as_object() {
            for (k, v) in obj.iter().take(4) {
                let v_str = if let Some(s) = v.as_str() {
                    s.to_string()
                } else {
                    v.to_string()
                };
                let v_short = if v_str.len() > 80 {
                    format!("{}…", &v_str[..78])
                } else {
                    v_str
                };
                println!("      {k} = {v_short}");
            }
        }
    }
    Ok(())
}
