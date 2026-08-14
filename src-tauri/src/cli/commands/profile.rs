// Atlas OS — `atlas profile` subcommand (RFC 25 §4).
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args, Debug)]
pub struct ProfileCmd {
    #[command(subcommand)]
    pub action: ProfileAction,
}

#[derive(Subcommand, Debug)]
pub enum ProfileAction {
    List,
    Switch { name: String },
    Current,
}

pub async fn run(cmd: ProfileCmd, current: &str) -> Result<()> {
    match cmd.action {
        ProfileAction::List => {
            let all = crate::profiles::list_all()?;
            if all.is_empty() {
                println!("(no profiles yet — first run created 'default' next time)");
            } else {
                println!("profiles:");
                for p in all {
                    let marker = if p.0 == current { "*" } else { " " };
                    println!("  {marker} {id}", id = p.0);
                }
            }
        }
        ProfileAction::Switch { name } => {
            let new = crate::profiles::ProfileId::new(&name);
            let _ = crate::profiles::resolve_root(&new)?;
            crate::profiles::set_current(&new)?;
            println!("active profile is now {name}");
            println!("the next opencode command or desktop launch will use it.");
        }
        ProfileAction::Current => {
            println!("{current}");
        }
    }
    Ok(())
}
