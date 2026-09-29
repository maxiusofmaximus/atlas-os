// Atlas OS — `atlas mobile` artemis lateral 8.6 (RFC 20 Fase 11
// sub-fase 11.0, research 40 SECTOR B 11.0 + RFC 38 §2-§3).
//
// google/artemis is Apache-2.0 + Python/uv: Atlas OS NEVER links,
// bundles, embeds, or vendors it. Integration is lateral-only: when
// the operator already cloned artemis and installed `uv`, `atlas
// mobile` detects the external entry point in `PATH` (or
// `ATLAS_ARTEMIS_BIN` / `ATLAS_ARTEMIS_REPO`) and spawns it as a child
// process via `std::process::Command`. Without artemis/uv every verb
// degrades to a useful message (clone + start.bat + USB debugging +
// `artemis mcp --install all`) and never fabricates a session.
//
// No new crate (RFC 25 §11): detection rides `std::env::split_paths`,
// launch rides `std::process::Command`, output is plain strings for
// the CLI print path.

use std::env;
use std::path::PathBuf;

/// Env var pinning the artemis entry point (operator override for
/// installs outside `PATH`; the `PATH` lookup remains the default).
/// Points at the `artemis` executable OR the `uv` binary when artemis
/// is launched via `uv run artemis`.
pub const ARTEMIS_BIN_ENV: &str = "ATLAS_ARTEMIS_BIN";
/// Env var pinning the cloned artemis repo dir (used as `current_dir`
/// for `uv run artemis ...` so uv resolves the project).
pub const ARTEMIS_REPO_ENV: &str = "ATLAS_ARTEMIS_REPO";
/// Binary name probed in `PATH` when no explicit override is set.
pub const ARTEMIS_BIN: &str = "artemis";
/// `uv` launcher probed in `PATH` (artemis ships as a uv project).
pub const UV_BIN: &str = "uv";
/// Where to clone artemis when it is missing (never fetched by Atlas).
pub const ARTEMIS_REPO_URL: &str = "https://github.com/google/artemis";
/// Lateral-integration license guardrail (RFC 38 §4 + SECTOR C).
pub const ARTEMIS_LICENSE_NOTE: &str =
    "artemis is Apache-2.0 + Python/uv — Atlas OS never bundles or links it; lateral external-process launch only.";
/// Default execution profile (reactive loop ~3-5s/step, RFC 38 §2.3).
pub const MOBILE_DEFAULT_PROFILE: &str = "flash";

/// artemis execution profile (RFC 38 §2.3). The Explorer tier is a user
/// setting — the agent never picks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobileProfile {
    Flash,
    Pro,
}

impl MobileProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            MobileProfile::Flash => "flash",
            MobileProfile::Pro => "pro",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "flash" => Some(MobileProfile::Flash),
            "pro" => Some(MobileProfile::Pro),
            _ => None,
        }
    }
}

/// What `atlas mobile status` found on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MobileStatus {
    Available {
        bin: PathBuf,
        via_uv: bool,
        repo: Option<PathBuf>,
    },
    Missing,
}

impl MobileStatus {
    pub fn available(&self) -> bool {
        matches!(self, MobileStatus::Available { .. })
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

pub fn find_artemis_in_path() -> Option<PathBuf> {
    find_in_path(ARTEMIS_BIN)
}

pub fn find_uv_in_path() -> Option<PathBuf> {
    find_in_path(UV_BIN)
}

fn repo_from_env() -> Option<PathBuf> {
    env::var(ARTEMIS_REPO_ENV)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

/// Resolve the artemis entry point: explicit override wins, then
/// `artemis` in `PATH`, then `uv` in `PATH` (repo from
/// `ATLAS_ARTEMIS_REPO` when set). Returns `Missing` instead of
/// erroring so the CLI can print the useful-message path (research 40
/// §11.0 "sin artemis/uv → mensaje útil").
pub fn resolve(bin_override: Option<&str>) -> MobileStatus {
    if let Some(b) = bin_override.map(str::trim).filter(|s| !s.is_empty()) {
        let via_uv = PathBuf::from(b)
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(UV_BIN) || n.eq_ignore_ascii_case("uv.exe"));
        return MobileStatus::Available {
            bin: PathBuf::from(b),
            via_uv,
            repo: repo_from_env(),
        };
    }
    if let Ok(b) = env::var(ARTEMIS_BIN_ENV) {
        let b = b.trim().to_string();
        if !b.is_empty() {
            let via_uv = PathBuf::from(&b)
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| {
                    n.eq_ignore_ascii_case(UV_BIN) || n.eq_ignore_ascii_case("uv.exe")
                });
            return MobileStatus::Available {
                bin: PathBuf::from(b),
                via_uv,
                repo: repo_from_env(),
            };
        }
    }
    if let Some(bin) = find_artemis_in_path() {
        return MobileStatus::Available {
            bin,
            via_uv: false,
            repo: repo_from_env(),
        };
    }
    match find_uv_in_path() {
        Some(bin) => MobileStatus::Available {
            bin,
            via_uv: true,
            repo: repo_from_env(),
        },
        None => MobileStatus::Missing,
    }
}

/// Args for the external artemis process (caller owns `Command`).
/// Direct binary: `artemis run "<task>" --profile flash|pro`.
/// Via uv: `uv run artemis run "<task>" --profile flash|pro`.
pub fn spawn_args(via_uv: bool, profile: MobileProfile, task: &str) -> Vec<String> {
    let mut args = Vec::new();
    if via_uv {
        args.push("run".to_string());
        args.push(ARTEMIS_BIN.to_string());
    }
    args.push("run".to_string());
    args.push(task.trim().to_string());
    args.push("--profile".to_string());
    args.push(profile.as_str().to_string());
    args
}

/// Spawn the external artemis session. Returns the child PID on
/// success. The lateral boundary lives here: `Command::new` on an
/// operator-owned path (plus optional `current_dir` = cloned repo for
/// uv), no link/bundle/embed of artemis code or assets. Detached on
/// purpose (`mem::forget`) — artemis outlives the CLI like RustDesk.
pub fn spawn_session(
    bin: &PathBuf,
    via_uv: bool,
    repo: Option<&PathBuf>,
    profile: MobileProfile,
    task: &str,
) -> std::io::Result<u32> {
    let args = spawn_args(via_uv, profile, task);
    let mut cmd = std::process::Command::new(bin);
    cmd.args(&args);
    if via_uv {
        if let Some(r) = repo {
            cmd.current_dir(r);
        }
    }
    let child = cmd.spawn()?;
    let pid = child.id();
    std::mem::forget(child);
    Ok(pid)
}

/// Validate the task string before spawning (blank tasks never reach
/// the external process).
pub fn normalize_task(raw: &str) -> Option<String> {
    let t = raw.trim().to_string();
    if t.is_empty() {
        return None;
    }
    Some(t)
}

/// Setup steps printed when artemis/uv is missing (research 40 §11.0).
pub fn setup_steps() -> String {
    format!(
        "artemis setup (lateral, Python/uv — Atlas OS never installs it for you):\n\
         1. Clone: `git clone {url}` (override with ${repo_env}).\n\
         2. Bootstrap on Windows: run `start.bat` inside the clone (auto-installs ADB, scrcpy, FFmpeg, Python via uv).\n\
         3. Device: enable USB debugging on the physical Android device/emulator and accept the host key.\n\
         4. Helper (first-task speedup): `uv run artemis helper install` (remove with `helper uninstall`).\n\
         5. IDE wiring: `uv run artemis mcp --install all` exposes mobile_run_task/manage_task/get_device_state/inspect_trace/diagnose.\n\
         6. Retry: `atlas mobile status`.\n\
         {license}"
        ,
        url = ARTEMIS_REPO_URL,
        repo_env = ARTEMIS_REPO_ENV,
        license = ARTEMIS_LICENSE_NOTE
    )
}

/// Message printed when neither artemis nor uv is installed.
pub fn missing_message() -> String {
    format!(
        "artemis no detectado en PATH ni en ${bin_env} (y `uv` tampoco en PATH).\n\
         {setup}",
        bin_env = ARTEMIS_BIN_ENV,
        setup = setup_steps()
    )
}

/// Full mobile-testing model text printed by `atlas mobile guide`.
pub fn mobile_guide() -> String {
    format!(
        "Mobile testing — artemis lateral 8.6 (RFC 38 §2-§3, research 40 Fase 11):\n\n\
         - Qué es: `google/artemis` convierte instrucciones en lenguaje natural en automatización Android real (99%+ AndroidWorld, Apache-2.0).\n\
         - Superficies: MCP nativo (mobile_run_task/manage_task/get_device_state/inspect_trace/diagnose) + CLI `uv run artemis run \"...\" --profile flash|pro` + Web Console `uv run artemis ui`.\n\
         - Perfiles: `flash` (loop reactivo ~3-5s/paso, rutina determinista) vs `pro` (multi-agent Planner/Operator/Checker ~15-40s/paso). Lo elige el operador, jamás el agente.\n\
         - Flujo Atlas: prueba el HUD mobile (RFC 24 §16) en dispositivo/emulador real — `atlas mobile run --task \"...\" --profile flash`.\n\
         - {license}\n\
         - Sin artemis/uv: sigue los setup steps de `atlas mobile status`."
        ,
        license = ARTEMIS_LICENSE_NOTE
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_clean_env<F: FnOnce()>(f: F) {
        let _g = ENV_LOCK.lock().unwrap();
        let bin_b = env::var_os(ARTEMIS_BIN_ENV);
        let repo_b = env::var_os(ARTEMIS_REPO_ENV);
        let path_b = env::var_os("PATH");
        env::remove_var(ARTEMIS_BIN_ENV);
        env::remove_var(ARTEMIS_REPO_ENV);
        env::set_var("PATH", std::env::temp_dir().join("atlas-os-no-artemis-xyz"));
        f();
        if let Some(v) = bin_b {
            env::set_var(ARTEMIS_BIN_ENV, v);
        } else {
            env::remove_var(ARTEMIS_BIN_ENV);
        }
        if let Some(v) = repo_b {
            env::set_var(ARTEMIS_REPO_ENV, v);
        } else {
            env::remove_var(ARTEMIS_REPO_ENV);
        }
        if let Some(v) = path_b {
            env::set_var("PATH", v);
        } else {
            env::remove_var("PATH");
        }
    }

    #[test]
    fn profile_parse_round_trips() {
        assert_eq!(MobileProfile::parse("flash"), Some(MobileProfile::Flash));
        assert_eq!(MobileProfile::parse("FLASH"), Some(MobileProfile::Flash));
        assert_eq!(MobileProfile::parse("pro"), Some(MobileProfile::Pro));
        assert_eq!(MobileProfile::parse("PRO"), Some(MobileProfile::Pro));
        assert_eq!(MobileProfile::parse("ultra"), None);
        assert_eq!(MobileProfile::parse(""), None);
        assert_eq!(MobileProfile::Flash.as_str(), "flash");
        assert_eq!(MobileProfile::Pro.as_str(), "pro");
    }

    #[test]
    fn explicit_override_wins_over_path() {
        let st = resolve(Some("/opt/artemis/artemis"));
        assert_eq!(
            st,
            MobileStatus::Available {
                bin: PathBuf::from("/opt/artemis/artemis"),
                via_uv: false,
                repo: None,
            }
        );
    }

    #[test]
    fn explicit_uv_override_marks_via_uv() {
        let st = resolve(Some("/usr/local/bin/uv"));
        assert!(matches!(st, MobileStatus::Available { via_uv: true, .. }));
    }

    #[test]
    fn blank_override_falls_through_to_missing() {
        with_clean_env(|| {
            assert_eq!(resolve(Some("   ")), MobileStatus::Missing);
        });
    }

    #[test]
    fn empty_env_resolves_to_missing() {
        with_clean_env(|| {
            assert_eq!(resolve(None), MobileStatus::Missing);
        });
    }

    #[test]
    fn env_override_selects_available_with_repo() {
        with_clean_env(|| {
            env::set_var(ARTEMIS_BIN_ENV, "/usr/local/bin/artemis");
            env::set_var(ARTEMIS_REPO_ENV, "/tmp/artemis-clone");
            let st = resolve(None);
            assert_eq!(
                st,
                MobileStatus::Available {
                    bin: PathBuf::from("/usr/local/bin/artemis"),
                    via_uv: false,
                    repo: Some(PathBuf::from("/tmp/artemis-clone")),
                }
            );
        });
    }

    #[test]
    fn spawn_args_direct_shape_is_fixed() {
        assert_eq!(
            spawn_args(false, MobileProfile::Flash, "open settings"),
            vec![
                "run".to_string(),
                "open settings".to_string(),
                "--profile".to_string(),
                "flash".to_string(),
            ]
        );
        assert_eq!(
            spawn_args(false, MobileProfile::Pro, "open settings"),
            vec![
                "run".to_string(),
                "open settings".to_string(),
                "--profile".to_string(),
                "pro".to_string(),
            ]
        );
    }

    #[test]
    fn spawn_args_uv_shape_is_fixed() {
        assert_eq!(
            spawn_args(true, MobileProfile::Flash, "open settings"),
            vec![
                "run".to_string(),
                "artemis".to_string(),
                "run".to_string(),
                "open settings".to_string(),
                "--profile".to_string(),
                "flash".to_string(),
            ]
        );
    }

    #[test]
    fn normalize_task_rejects_blank() {
        assert_eq!(
            normalize_task("  open settings "),
            Some("open settings".to_string())
        );
        assert_eq!(normalize_task("   "), None);
        assert_eq!(normalize_task(""), None);
    }

    #[test]
    fn missing_message_points_to_setup() {
        let msg = missing_message();
        assert!(msg.contains(ARTEMIS_BIN_ENV));
        assert!(msg.contains("git clone"));
        assert!(msg.contains("start.bat"));
        assert!(msg.contains("USB debugging"));
        assert!(msg.contains("mcp --install all"));
        assert!(msg.contains("Apache-2.0"));
    }

    #[test]
    fn guide_names_profiles_and_task_flow() {
        let guide = mobile_guide();
        assert!(guide.contains("flash"));
        assert!(guide.contains("pro"));
        assert!(guide.contains("99%+"));
        assert!(guide.contains("Apache-2.0"));
        assert!(guide.contains("atlas mobile run"));
    }

    #[test]
    fn setup_steps_mentions_repo_env_and_helper() {
        let steps = setup_steps();
        assert!(steps.contains(ARTEMIS_REPO_URL));
        assert!(steps.contains(ARTEMIS_REPO_ENV));
        assert!(steps.contains("helper install"));
        assert!(steps.contains("atlas mobile status"));
    }

    #[test]
    fn spawn_missing_binary_surfaces_io_error_without_panic() {
        let bin = PathBuf::from("/nonexistent-atlas-artemis-bin-xyz");
        let err =
            spawn_session(&bin, false, None, MobileProfile::Flash, "open settings").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
