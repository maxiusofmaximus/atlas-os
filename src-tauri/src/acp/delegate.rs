// Atlas OS — ACP delegate helpers for `/opencode exec step` and
// `/opencode fix` (RFC 28 §B Phase 1.5d item 4).
//
// Per RFC 28 §B line 123 the binary registers `opencode exec step
// <mission_id> <step_id>` as a delegate CLI (Alt+Shift+B in IT). The
// `/opencode fix [hint]` slash command captures the active IT pane
// scrollback via `wtcli active-pane` + `capture-pane --last-prompt`,
// packages the scrollback as a `Resource` content block, and enqueues
// a Repair engine run (RFC 15). Both flows share a structure:
//
//   1. Resolve the requested command + unstructured hint.
//   2. Shell out to `wtcli` to read terminal state. Windows-only.
//   3. Hand the captured artefact back to the host loop, which turns
//      it into a `session/prompt` request and drives the engines.
//
// Phase 1.5d ships this module with **typed stubs**: every fn returns a
// `DelegateOutcome` describing what the host loop should do, but no
// `wtcli` subprocess is spawned yet (the host loop itself is the Phase 2
// deliverable that follows in item 5+). The stubs are exercised by unit
// tests so the contract is pinned before the subprocess plumbing lands.

use std::path::PathBuf;

/// Trailing tail of an IT pane captured by `wtcli capture-pane
/// --last-prompt`. Owned because `wtcli` may emit a long scrollback the
/// host loop wants to attach in full to the Repair engine (RFC 15 §1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapturedScrollback {
    pub pane_id: String,
    pub cwd: PathBuf,
    pub last_prompt: String,
    pub scrollback: String,
}

/// Outcome of an ACP slash command delegated to the OpenCode host loop.
/// Every variant owns enough information for the host loop (item 5+) to
/// either dispatch a real CLI subcommand or emit a user-visible message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DelegateOutcome {
    /// `/opencode exec step <mission_id> <step_id>` resolved to a runnable
    /// CLI invocation. The strings are passed verbatim to
    /// `opencode exec step <plan_id> <step_id>` via the existing CLI
    /// dispatch path (`crate::cli::commands::exec::run`).
    ExecStep { mission_id: String, step_id: String },
    /// `/opencode fix [hint]` captured a pane scrollback + an optional
    /// human hint. RFC 28 §B line 122 + RFC 15 §1. The Repair engine
    /// should consume both; if `scrollback` is `None` the capture could
    /// not be performed (non-Windows, no `wtcli`, pane unreachable) and
    /// the host loop must tell the operator instead of silently bailing.
    FixRequested {
        hint: Option<String>,
        scrollback: Option<CapturedScrollback>,
    },
    /// `/opencode restart` requests a `session/close` + `session/new`
    /// with the same `cwd`. Item 5+ wires the ACP method pair; the stub
    /// only remembers the requested cwd so a future observer can assert
    /// it survives the round-trip.
    RestartRequested { cwd: PathBuf },
    /// A slash command we do not handle as a delegate (mission new,
    /// fork, resume). The Phase 1.5d host loop leaves these to the
    /// CLI surface directly.
    NotImplemented { command: String },
}

/// Parse the ACP slash-command line into a `DelegateOutcome`. The ACP
/// `PromptRequest` carries the slash command name in the first content
/// block of the prompt; IT does not pre-parse argv. Phase 1.5d recognises
/// only the four delegate-style commands listed in RFC 28 §B line 123.
///
/// `cwd` is the cwd of the ACP session issuing the request, supplied by
/// the host loop from `NewSessionRequest.cwd`. It is forwarded to
/// `RestartRequested` unmodified.
/// Parse the leading word cluster of `stripped` against the known slash
/// command prefixes (`opencode_fix`, `opencode exec step`, etc.). The ACP
/// catalogue commands are multi-token (e.g. `opencode exec step`) so a
/// naive single-token match would bail on the second word. We split a
/// sliding prefix of up to the longest command name (`CMD_EXEC_STEP` —
/// three words) and return `Some((matched_cmd, trailing_rest))` when a
/// known command is found; otherwise `None` so the caller surface the
/// full first word as `NotImplemented`.
fn match_command_head(stripped: &str) -> Option<(&'static str, &str)> {
    let candidates = [
        super::commands::CMD_EXEC_STEP,
        super::commands::CMD_MISSION_NEW,
        super::commands::CMD_RESTART,
        super::commands::CMD_FIX,
        super::commands::CMD_FORK,
        super::commands::CMD_RESUME,
    ];
    for cand in candidates {
        if cand.len() > stripped.len() {
            continue;
        }
        let (prefix, suffix) = stripped.split_at(cand.len());
        if prefix == cand {
            let rest = suffix.trim_start();
            return Some((cand, rest));
        }
    }
    None
}

pub fn parse_delegate(line: &str, cwd: &str) -> DelegateOutcome {
    let trimmed = line.trim();
    let stripped = trimmed.strip_prefix('/').unwrap_or(trimmed);
    let Some((head, rest)) = match_command_head(stripped) else {
        let first_word = stripped
            .split_whitespace()
            .next()
            .unwrap_or(stripped)
            .to_string();
        return DelegateOutcome::NotImplemented {
            command: first_word,
        };
    };
    match head {
        super::commands::CMD_EXEC_STEP => parse_exec_step(rest),
        super::commands::CMD_FIX => DelegateOutcome::FixRequested {
            hint: if rest.is_empty() {
                None
            } else {
                Some(rest.to_string())
            },
            scrollback: None,
        },
        super::commands::CMD_RESTART => DelegateOutcome::RestartRequested {
            cwd: PathBuf::from(cwd.to_string()),
        },
        other => DelegateOutcome::NotImplemented {
            command: other.to_string(),
        },
    }
}

/// Parse the trailing argv of `/opencode exec step <mission_id> <step_id>`.
/// Returns `NotImplemented` when the argv is malformed so the host loop
/// can surface a friendly error instead of panicking.
fn parse_exec_step(rest: &str) -> DelegateOutcome {
    let mut it = rest.split_whitespace();
    let mission_id = match it.next() {
        Some(m) if !m.is_empty() => m.to_string(),
        _ => {
            return DelegateOutcome::NotImplemented {
                command: super::commands::CMD_EXEC_STEP.to_string(),
            }
        }
    };
    let step_id = match it.next() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            return DelegateOutcome::NotImplemented {
                command: super::commands::CMD_EXEC_STEP.to_string(),
            }
        }
    };
    DelegateOutcome::ExecStep {
        mission_id,
        step_id,
    }
}

/// Phase 1.5d stub for `wtcli active-pane` + `wtcli capture-pane
/// --last-prompt`. Returns `Err` on any non-Windows host or when the
/// `WT_COM_CLSID` discovery env var is absent (RFC 28 §B line 143). The
/// real subprocess plumbing lands with the `wtcli listen` worker of
/// item 5; until then the host loop treats `Err` as "no scrollback
/// available" and surfaces a user-visible message.
pub fn capture_active_pane_scrollback() -> Result<CapturedScrollback, CaptureError> {
    if cfg!(not(target_os = "windows")) {
        return Err(CaptureError::UnsupportedPlatform);
    }
    if std::env::var_os("WT_COM_CLSID").is_none() {
        return Err(CaptureError::NoWtComClsid);
    }
    Err(CaptureError::StubNotWired)
}

/// Failure modes for `capture_active_pane_scrollback`. Mirrors the three
/// reasons the Phase 1.5d stub returns `Err`; item 5 will keep the same
/// enum shape and add a `SubprocessFailed { stderr: String }` variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaptureError {
    UnsupportedPlatform,
    NoWtComClsid,
    StubNotWired,
}

#[cfg(test)]
mod tests {
    use super::super::commands::{CMD_EXEC_STEP, CMD_FIX, CMD_MISSION_NEW, CMD_RESTART};
    use super::*;

    #[test]
    fn parse_exec_step_with_both_args() {
        let out = parse_delegate(&format!("{} abc-123 s-4", CMD_EXEC_STEP), "/tmp");
        assert_eq!(
            out,
            DelegateOutcome::ExecStep {
                mission_id: "abc-123".into(),
                step_id: "s-4".into(),
            }
        );
    }

    #[test]
    fn parse_exec_step_missing_step_falls_back_to_not_implemented() {
        let out = parse_delegate(&format!("{} abc-123", CMD_EXEC_STEP), "/tmp");
        assert!(matches!(out, DelegateOutcome::NotImplemented { .. }));
    }

    #[test]
    fn parse_exec_step_missing_both_args_falls_back_to_not_implemented() {
        let out = parse_delegate(CMD_EXEC_STEP, "/tmp");
        assert!(matches!(out, DelegateOutcome::NotImplemented { .. }));
    }

    #[test]
    fn parse_fix_with_hint_captures_hint() {
        let out = parse_delegate(&format!("{} add more error handling", CMD_FIX), "/tmp");
        assert_eq!(
            out,
            DelegateOutcome::FixRequested {
                hint: Some("add more error handling".into()),
                scrollback: None,
            }
        );
    }

    #[test]
    fn parse_fix_without_hint_records_none_hint() {
        let out = parse_delegate(CMD_FIX, "/tmp");
        assert_eq!(
            out,
            DelegateOutcome::FixRequested {
                hint: None,
                scrollback: None,
            }
        );
    }

    #[test]
    fn parse_restart_captures_cwd() {
        let out = parse_delegate(CMD_RESTART, "C:\\Users\\dev\\repo");
        assert_eq!(
            out,
            DelegateOutcome::RestartRequested {
                cwd: PathBuf::from("C:\\Users\\dev\\repo"),
            }
        );
    }

    #[test]
    fn parse_mission_new_is_not_a_delegate_in_phase_1_5d() {
        let out = parse_delegate(&format!("{} build auth", CMD_MISSION_NEW), "/tmp");
        assert!(matches!(out, DelegateOutcome::NotImplemented { .. }));
    }

    #[test]
    fn parse_unknown_command_is_not_implemented() {
        let out = parse_delegate("/help", "/tmp");
        match out {
            DelegateOutcome::NotImplemented { command } => assert_eq!(command, "help"),
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    #[test]
    fn capture_stub_errors_with_predictable_reason() {
        let err = capture_active_pane_scrollback().unwrap_err();
        assert!(
            matches!(
                err,
                CaptureError::UnsupportedPlatform
                    | CaptureError::NoWtComClsid
                    | CaptureError::StubNotWired
            ),
            "stub must return a typed CaptureError, got {err:?}"
        );
    }

    #[test]
    fn captured_scrollback_is_owned_and_eq() {
        let a = CapturedScrollback {
            pane_id: "3".into(),
            cwd: PathBuf::from("/tmp"),
            last_prompt: "cargo test".into(),
            scrollback: "ok".into(),
        };
        let b = a.clone();
        assert_eq!(a, b);
    }
}
