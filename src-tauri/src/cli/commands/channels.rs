// Atlas OS — `atlas channels` (RFC 29 §3.B, A3.0).
//
// Status of the multi-channel gateway core. No network: it reports what is
// configured (a Telegram bot token present?) and the command/event surface the
// adapters will use. The `teloxide` adapter is the next increment, behind the
// `multi-channel` feature (default off).

use anyhow::Result;
use clap::Args;

/// Env var (and keychain slot) holding the Telegram bot token (RFC 29 §3.B).
const TELEGRAM_TOKEN: &str = "ATLAS_TELEGRAM_BOT_TOKEN";

#[derive(Args, Debug)]
pub struct ChannelsCmd {
    /// Print the status as JSON.
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

pub async fn run(cmd: ChannelsCmd, _profile: &str) -> Result<()> {
    let from_env = std::env::var(TELEGRAM_TOKEN)
        .ok()
        .filter(|t| !t.trim().is_empty());
    // Secrets stored in the OS keychain use the same slot name (RFC 25 §3.10).
    let from_keychain = crate::secrets::get(TELEGRAM_TOKEN).ok().flatten();
    let configured = from_env.is_some() || from_keychain.is_some();
    let source = if from_env.is_some() {
        "env"
    } else if from_keychain.is_some() {
        "keychain"
    } else {
        "none"
    };

    if cmd.json {
        let v = serde_json::json!({
            "telegram": {
                "configured": configured,
                "source": source,
                "token_slot": TELEGRAM_TOKEN,
            },
            "commands": ["pause", "resume", "steer", "approve", "deny", "status"],
            "events": [
                "approval_request",
                "cost_threshold_crossed",
                "doom_loop_detected",
                "goal_drift_detected",
            ],
            "adapter": "pending: `teloxide`, feature `multi-channel` (default off)",
        });
        println!("{}", serde_json::to_string_pretty(&v)?);
        return Ok(());
    }

    println!("channels: multi-channel gateway (RFC 29 §3.B, A3.0)");
    println!(
        "  telegram: {}",
        if configured {
            "configured"
        } else {
            "not configured"
        }
    );
    println!("    token: {TELEGRAM_TOKEN} (env, or `atlas secrets set {TELEGRAM_TOKEN}`)");
    println!("    source: {source}");
    println!("  adapter: pending — `teloxide` behind feature `multi-channel` (default off)");
    println!("  commands: /pause /resume /steer <text> /approve <id> /deny <id> /status");
    println!("  forwarded: approval.request, cost.threshold.crossed, doom_loop, goal_drift");
    Ok(())
}
