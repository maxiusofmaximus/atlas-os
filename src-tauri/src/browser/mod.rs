// Atlas OS — `atlas browser` terminal-browser lateral integration
// (RFC 28 §I, Phase 1.5 item 2-7).
//
// `zenbu-labs/terminal-browser` (MIT) renders a real Chromium browser
// inside the terminal via the kitty graphics protocol. Atlas OS NEVER
// links, bundles, embeds, or vendors it: the Electron/Chromium runtime
// plus (on macOS) a Swift input helper violate RFC 25 §11 (single
// 30-45 MB binary, no Electron). Integration is the lateral pattern 8.5
// (RustDesk/artemis): detect the operator-owned `terminal-browser` in
// `PATH` (or `ATLAS_TERMINAL_BROWSER_BIN`) and spawn it as a child
// process. Without it every verb degrades to a useful install message;
// `open` exits non-zero rather than silently no-op (RFC 28 §I "nunca un
// no-op silencioso").
//
// No new crate (RFC 25 §11): detection rides `std::env::split_paths`,
// launch rides `std::process::Command`, output is plain strings.
//
// Attribution: pattern from zenbu-labs/terminal-browser (MIT). Atlas OS
// does not redistribute the binary.

use std::env;
use std::fmt;
use std::path::PathBuf;

/// Binary probed in `PATH` when no override is set.
pub const TERMINAL_BROWSER_BIN: &str = "terminal-browser";
/// Env var pinning the entry point for installs outside `PATH`.
pub const TERMINAL_BROWSER_BIN_ENV: &str = "ATLAS_TERMINAL_BROWSER_BIN";
/// Upstream project (operator clones/installs it themselves).
pub const TERMINAL_BROWSER_URL: &str = "https://github.com/zenbu-labs/terminal-browser";
/// Lateral-integration license guardrail (RFC 25 §11).
pub const LICENSE_NOTE: &str = "terminal-browser is MIT — Atlas OS never bundles or links it (Electron/Chromium runtime + macOS Swift helper); lateral external-process launch only.";
/// `agent-browser` action contract is not pinned yet (RFC 28 §I item 7):
/// the wrapper never invents flags — it forwards the operator's own args.
pub const ACTION_CONTRACT_PINNED: bool = false;

/// Whether the terminal declares the kitty graphics protocol. Honesty
/// rule (RFC 28 §Riesgos 6): support is NEVER claimed without a signal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalCapability {
    Kitty { signal: String },
    Unverified { reason: String },
}

impl TerminalCapability {
    pub fn is_kitty(&self) -> bool {
        matches!(self, TerminalCapability::Kitty { .. })
    }

    /// One-line human summary for `atlas browser probe`.
    pub fn summary(&self) -> String {
        match self {
            TerminalCapability::Kitty { signal } => {
                format!("kitty graphics protocol declared ({signal})")
            }
            TerminalCapability::Unverified { reason } => {
                format!("terminal not verified for kitty graphics protocol — {reason}")
            }
        }
    }
}

/// Detect kitty-graphics capability from an env lookup (pure, so tests
/// drive it). Ordered strongest-signal-first; the first hit wins.
pub fn detect_capability_from<F>(get: F) -> TerminalCapability
where
    F: Fn(&str) -> Option<String>,
{
    let read = |k: &str| -> Option<String> {
        get(k)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    if let Some(v) = read("KITTY_WINDOW_ID") {
        return TerminalCapability::Kitty {
            signal: format!("KITTY_WINDOW_ID={v}"),
        };
    }
    if read("GHOSTTY_RESOURCES_DIR").is_some() {
        return TerminalCapability::Kitty {
            signal: "GHOSTTY_RESOURCES_DIR".to_string(),
        };
    }
    if read("WEZTERM_PANE").is_some() {
        return TerminalCapability::Kitty {
            signal: "WEZTERM_PANE".to_string(),
        };
    }
    if let Some(t) = read("TERM") {
        let l = t.to_lowercase();
        if l.contains("kitty") || l.contains("ghostty") {
            return TerminalCapability::Kitty {
                signal: format!("TERM={t}"),
            };
        }
    }
    if let Some(p) = read("TERM_PROGRAM") {
        let l = p.to_lowercase();
        if l == "ghostty" || l == "wezterm" || l == "kitty" {
            return TerminalCapability::Kitty {
                signal: format!("TERM_PROGRAM={p}"),
            };
        }
    }
    let reason = match read("TERM") {
        Some(t) => format!("TERM={t} no declara kitty graphics protocol"),
        None => "sin $TERM ni $TERM_PROGRAM señalando kitty graphics protocol".to_string(),
    };
    TerminalCapability::Unverified { reason }
}

/// `detect_capability_from` against the real process environment.
pub fn detect_capability() -> TerminalCapability {
    detect_capability_from(|k| env::var(k).ok())
}

/// What `atlas browser probe` found on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserStatus {
    Available { bin: PathBuf },
    Missing,
}

impl BrowserStatus {
    pub fn available(&self) -> bool {
        matches!(self, BrowserStatus::Available { .. })
    }
}

/// Probe `PATH` for one binary name (Windows also tries `.exe`).
fn find_in_path(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let names: Vec<String> = if cfg!(windows) {
        vec![name.to_string(), format!("{name}.exe")]
    } else {
        vec![name.to_string()]
    };
    for dir in env::split_paths(&path) {
        for n in &names {
            let candidate = dir.join(n);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn find_terminal_browser_in_path() -> Option<PathBuf> {
    find_in_path(TERMINAL_BROWSER_BIN)
}

/// Resolve the entry point: explicit override wins, then the env var,
/// then `PATH`. Returns `Missing` instead of erroring so the CLI can
/// print the useful-message path.
pub fn resolve(bin_override: Option<&str>) -> BrowserStatus {
    if let Some(b) = bin_override.map(str::trim).filter(|s| !s.is_empty()) {
        return BrowserStatus::Available {
            bin: PathBuf::from(b),
        };
    }
    if let Some(b) = env::var(TERMINAL_BROWSER_BIN_ENV)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        return BrowserStatus::Available {
            bin: PathBuf::from(b),
        };
    }
    match find_terminal_browser_in_path() {
        Some(bin) => BrowserStatus::Available { bin },
        None => BrowserStatus::Missing,
    }
}

/// Failed lateral operation, each variant carrying textual remediation
/// (RFC 28 §I "enum con remediación textual").
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserIntegrationError {
    NotInstalled,
    TerminalUnsupported { reason: String },
    SpawnFailed { message: String },
}

impl BrowserIntegrationError {
    pub fn remediation(&self) -> String {
        match self {
            BrowserIntegrationError::NotInstalled => missing_message(),
            BrowserIntegrationError::TerminalUnsupported { reason } => format!(
                "terminal no verificado para kitty graphics protocol: {reason}\n\
                 terminal-browser puede funcionar parcialmente; verifica `{}` (ghostty/kitty/WezTerm).",
                "atlas browser probe"
            ),
            BrowserIntegrationError::SpawnFailed { message } => format!(
                "no se pudo lanzar `{TERMINAL_BROWSER_BIN}`: {message}\n\
                 Verifica que el binario esté en PATH (`atlas browser probe`) y reintenta."
            ),
        }
    }
}

impl fmt::Display for BrowserIntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.remediation())
    }
}

impl std::error::Error for BrowserIntegrationError {}

/// Typed wrapper over `terminal-browser action` (RFC 28 §I item 5). The
/// `agent-browser` contract is not pinned (`ACTION_CONTRACT_PINNED`), so
/// these arg shapes are provisional: the verb + operands are forwarded
/// verbatim to the external CLI (no invented flags).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserAction {
    Navigate { url: String },
    Click { selector: String },
    Type { selector: String, text: String },
    Snapshot,
}

impl BrowserAction {
    pub fn to_args(&self) -> Vec<String> {
        match self {
            BrowserAction::Navigate { url } => {
                vec!["action".to_string(), "navigate".to_string(), url.clone()]
            }
            BrowserAction::Click { selector } => {
                vec!["action".to_string(), "click".to_string(), selector.clone()]
            }
            BrowserAction::Type { selector, text } => vec![
                "action".to_string(),
                "type".to_string(),
                selector.clone(),
                text.clone(),
            ],
            BrowserAction::Snapshot => vec!["action".to_string(), "snapshot".to_string()],
        }
    }
}

/// Args for `terminal-browser open <url> [--split <dir>]` (caller owns
/// `Command`).
pub fn spawn_args_open(url: &str, split: Option<&str>) -> Vec<String> {
    let mut args = vec!["open".to_string(), url.trim().to_string()];
    if let Some(s) = split.map(str::trim).filter(|s| !s.is_empty()) {
        args.push("--split".to_string());
        args.push(s.to_string());
    }
    args
}

/// Args for `terminal-browser ls`.
pub fn spawn_args_ls() -> Vec<String> {
    vec!["ls".to_string()]
}

fn build_command(bin: &PathBuf, args: &[String]) -> std::process::Command {
    let mut cmd = std::process::Command::new(bin);
    cmd.args(args);
    cmd
}

/// Spawn the external browser detached (it outlives the CLI). Returns the
/// child PID. The lateral boundary lives here: `Command::new` on an
/// operator-owned path, no link/bundle/embed.
pub fn spawn_open(bin: &PathBuf, url: &str, split: Option<&str>) -> std::io::Result<u32> {
    let child = build_command(bin, &spawn_args_open(url, split)).spawn()?;
    let pid = child.id();
    std::mem::forget(child);
    Ok(pid)
}

/// Spawn an external `action` passthrough detached (RFC 28 §I item 5).
pub fn spawn_action(bin: &PathBuf, action: &BrowserAction) -> std::io::Result<u32> {
    let child = build_command(bin, &action.to_args()).spawn()?;
    let pid = child.id();
    std::mem::forget(child);
    Ok(pid)
}

/// Run a short-lived external command and capture combined stdout/stderr
/// (used for `ls` and `--version`; `open` never blocks).
pub fn run_capture(bin: &PathBuf, args: &[String]) -> std::io::Result<String> {
    let out = build_command(bin, args).output()?;
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.stderr.is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    Ok(text.trim_end().to_string())
}

/// Best-effort `terminal-browser --version` (item 7: detect contract
/// drift so the pin/urgencia is visible, never assumed).
pub fn probe_version(bin: &PathBuf) -> Option<String> {
    run_capture(bin, &["--version".to_string()])
        .ok()
        .map(|s| s.lines().next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Read-only probe report (item 2: never spawns the browser).
pub fn probe_report(bin_override: Option<&str>) -> String {
    let capability = detect_capability();
    match resolve(bin_override) {
        BrowserStatus::Available { bin } => {
            let version = probe_version(&bin).unwrap_or_else(|| "(unknown — no --version)".into());
            let contract = if ACTION_CONTRACT_PINNED {
                "pinned"
            } else {
                "UNPINNED — `action` shapes provisional (RFC 28 §I item 7)"
            };
            format!(
                "browser: available\n  bin: {}\n  version: {version}\n  terminal: {}\n  action contract: {contract}\n  {}",
                bin.display(),
                capability.summary(),
                LICENSE_NOTE
            )
        }
        BrowserStatus::Missing => format!(
            "browser: terminal-browser NOT found in PATH nor ${TERMINAL_BROWSER_BIN_ENV}\n  terminal: {}\n{}",
            capability.summary(),
            setup_steps()
        ),
    }
}

/// Setup steps printed when `terminal-browser` is missing.
pub fn setup_steps() -> String {
    format!(
        "terminal-browser setup (lateral, MIT — Atlas OS never installs it for you):\n\
         1. Install upstream: `curl -fsSL https://terminal-browser.sh/install | bash` or `brew install terminal-browser`.\n\
         2. Use a kitty-graphics terminal (ghostty, kitty, WezTerm); check with `atlas browser probe`.\n\
         3. Upgrade later with `terminal-browser upgrade`.\n\
         4. Retry: `atlas browser probe`.\n\
         Project: {url}\n\
         {license}",
        url = TERMINAL_BROWSER_URL,
        license = LICENSE_NOTE
    )
}

/// Message printed when neither the binary nor an override is present.
pub fn missing_message() -> String {
    format!(
        "terminal-browser no detectado en PATH ni en ${TERMINAL_BROWSER_BIN_ENV}.\n{setup}",
        setup = setup_steps()
    )
}

/// Full integration text for `atlas browser guide`.
pub fn browser_guide() -> String {
    format!(
        "Browser pane — terminal-browser lateral (RFC 28 §I, lateral pattern 8.5):\n\n\
         - Qué es: navegador Chromium real dibujado DENTRO del terminal vía kitty graphics protocol (zenbu-labs/terminal-browser, MIT).\n\
         - Superficies: `terminal-browser open <url> [--split right]` (abre), `ls` (lista), `action` (control agent-browser-compatible).\n\
         - Atlas: `atlas browser probe` (sólo lectura), `atlas browser open <url> [--split right]`, `atlas browser ls`, `atlas browser action -- <args>`.\n\
         - Seguridad (RFC 18): Chromium es superficie de ataque; capacidad de red opt-in, SensitiveAction en HUMAN_IN_LOOP/AUTOPILOT.\n\
         - Frontera lateral: sin crates, sin features, sin migraciones; proceso externo, no hereda secretos del keychain.\n\
         - {license}\n\
         - Sin el binario: sigue los setup steps de `atlas browser probe`.",
        license = LICENSE_NOTE
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_map(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k: &str| map.get(k).cloned()
    }

    #[test]
    fn capability_unverified_without_signal() {
        let cap = detect_capability_from(env_map(&[("TERM", "xterm-256color")]));
        assert!(!cap.is_kitty());
        assert!(matches!(cap, TerminalCapability::Unverified { .. }));
    }

    #[test]
    fn capability_detects_kitty_by_env_var() {
        let cap = detect_capability_from(env_map(&[("KITTY_WINDOW_ID", "3")]));
        assert!(cap.is_kitty());
    }

    #[test]
    fn capability_detects_ghostty_and_term() {
        assert!(detect_capability_from(env_map(&[("GHOSTTY_RESOURCES_DIR", "/x")])).is_kitty());
        assert!(detect_capability_from(env_map(&[("TERM", "xterm-kitty")])).is_kitty());
        assert!(detect_capability_from(env_map(&[("TERM_PROGRAM", "WezTerm")])).is_kitty());
    }

    #[test]
    fn resolve_override_wins() {
        let st = resolve(Some("/opt/tb/terminal-browser"));
        assert_eq!(
            st,
            BrowserStatus::Available {
                bin: PathBuf::from("/opt/tb/terminal-browser")
            }
        );
    }

    #[test]
    fn blank_override_falls_through_to_path_lookup() {
        // A blank override must not error nor fabricate a path: it behaves
        // exactly like no override (deterministic regardless of the machine).
        assert_eq!(resolve(Some("   ")), resolve(None));
    }

    #[test]
    fn open_args_shape_is_fixed() {
        assert_eq!(
            spawn_args_open("https://example.com", None),
            vec!["open".to_string(), "https://example.com".to_string()]
        );
        assert_eq!(
            spawn_args_open(" ./plan.html ", Some("right")),
            vec![
                "open".to_string(),
                "./plan.html".to_string(),
                "--split".to_string(),
                "right".to_string(),
            ]
        );
        assert_eq!(spawn_args_ls(), vec!["ls".to_string()]);
    }

    #[test]
    fn browser_action_forwards_verb_and_operands() {
        assert_eq!(
            BrowserAction::Navigate {
                url: "https://a.b".into()
            }
            .to_args(),
            vec!["action", "navigate", "https://a.b"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            BrowserAction::Snapshot.to_args(),
            vec!["action", "snapshot"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn not_installed_remediation_points_to_install() {
        let msg = BrowserIntegrationError::NotInstalled.remediation();
        assert!(msg.contains("terminal-browser.sh/install"));
        assert!(msg.contains(TERMINAL_BROWSER_BIN_ENV));
        assert!(msg.contains("MIT"));
    }

    #[test]
    fn spawn_failed_remediation_names_binary() {
        let msg = BrowserIntegrationError::SpawnFailed {
            message: "boom".into(),
        }
        .remediation();
        assert!(msg.contains(TERMINAL_BROWSER_BIN));
        assert!(msg.contains("boom"));
    }

    #[test]
    fn missing_binary_surfaces_io_error_without_panic() {
        let bin = PathBuf::from("/nonexistent-atlas-terminal-browser-xyz");
        let err = spawn_open(&bin, "https://example.com", None).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn guide_names_commands_and_license() {
        let g = browser_guide();
        assert!(g.contains("atlas browser open"));
        assert!(g.contains("atlas browser probe"));
        assert!(g.contains("kitty graphics"));
        assert!(g.contains("MIT"));
    }
}
