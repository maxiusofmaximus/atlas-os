// Atlas OS — ACP SessionMode ↔ RFC 19 Execution Supervisor state
// mapping (RFC 28 §B Phase 1.5d item 3).
//
// Microsoft Intelligent Terminal exposes three canonical session modes the
// ACP wire calls `architect`, `code` and `ask`. RFC 19's mission-level
// state machine owns three supervisor phases that need an ACP face: PLAN,
// EXEC and REVIEW. The autonomous phases (`LEARNING`, `RECOVERING` …) stay
// off the ACP wire — IT only steers the human-in-loop triplet.
//
// This module is intentionally pure: no tokio, no `agent_client_protocol`
// import. The ACP server (`super::run_server`) consults it whenever a
// `session/update` carrying `currentModeUpdate` has to be emitted or
// whenever an inbound `session/set_mode` request arrives.

use std::fmt;
use std::str::FromStr;

/// RFC 19 §6.1 phases that participate in the ACP mode triplet.
///
/// Only `Plan`, `Exec` and `Review` map onto ACP session modes; the
/// autonomous phases (idle, recovering, halted, done, learning) have no ACP
/// counterpart and are intentionally absent from this enum. The mapping is
/// one-to-one, so the `Default` impl is arbitrary and exists only to make
// the helper fns total over the `AcpMode` set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SupervisorState {
    /// RFC 19 Planning Engine is producing a plan → ACP `architect`.
    Plan,
    /// RFC 19 Coding Engine is emitting diffs → ACP `code`.
    Exec,
    /// RFC 19 Validation Engine is running its cascade → ACP `ask`.
    Review,
}

impl fmt::Display for SupervisorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

impl FromStr for SupervisorState {
    type Err = UnknownSupervisorState;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "plan" => Ok(Self::Plan),
            "exec" => Ok(Self::Exec),
            "review" => Ok(Self::Review),
            other => Err(UnknownSupervisorState(other.into())),
        }
    }
}

impl SupervisorState {
    /// Lowercase RFC 19 wire tag for this phase.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Exec => "exec",
            Self::Review => "review",
        }
    }
}

/// Error returned when a `SupervisorState` wire tag cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSupervisorState(pub String);

impl fmt::Display for UnknownSupervisorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown supervisor state: `{}`", self.0)
    }
}

impl std::error::Error for UnknownSupervisorState {}

/// The three canonical ACP session modes Microsoft Intelligent Terminal
/// advertises. These mirror `SupervisorState` one-to-one and never escape
/// the module's mapping fns except via `acp_mode_id` for wire serialisation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AcpMode {
    /// ACP `architect` — design / planning surface.
    Architect,
    /// ACP `code` — coding surface.
    Code,
    /// ACP `ask` — review / ask surface.
    Ask,
}

impl fmt::Display for AcpMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

impl FromStr for AcpMode {
    type Err = UnknownAcpMode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "architect" => Ok(Self::Architect),
            "code" => Ok(Self::Code),
            "ask" => Ok(Self::Ask),
            other => Err(UnknownAcpMode(other.into())),
        }
    }
}

impl AcpMode {
    /// Lowercase ACP wire tag for this mode (`"architect" | "code" | "ask"`).
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Architect => "architect",
            Self::Code => "code",
            Self::Ask => "ask",
        }
    }
}

/// Error returned when an `AcpMode` wire tag cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownAcpMode(pub String);

impl fmt::Display for UnknownAcpMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown ACP mode: `{}`", self.0)
    }
}

impl std::error::Error for UnknownAcpMode {}

/// RFC 19 `SupervisorState` → ACP `AcpMode`. Direct match — the two enums
/// are the same triplet relabelled.
pub fn supervisor_to_acp(state: &SupervisorState) -> AcpMode {
    match state {
        SupervisorState::Plan => AcpMode::Architect,
        SupervisorState::Exec => AcpMode::Code,
        SupervisorState::Review => AcpMode::Ask,
    }
}

/// ACP `AcpMode` → RFC 19 `SupervisorState`. Inverse of
/// [`supervisor_to_acp`].
pub fn acp_to_supervisor(mode: &AcpMode) -> SupervisorState {
    match mode {
        AcpMode::Architect => SupervisorState::Plan,
        AcpMode::Code => SupervisorState::Exec,
        AcpMode::Ask => SupervisorState::Review,
    }
}

/// Stable wire string for an `AcpMode` (`"architect" | "code" | "ask"`).
/// Equivalent to `mode.tag()` but kept as a named fn so the ACP server's
/// call sites read intent at the point of serialisation.
pub fn acp_mode_id(mode: &AcpMode) -> &'static str {
    mode.tag()
}

/// Parse a wire `SessionModeId` into an `AcpMode`. Unknown tags return
/// `None` so callers can fall back to the current mode instead of
/// surfacing an ACP-level error to the client.
pub fn parse_acp_mode_id(s: &str) -> Option<AcpMode> {
    AcpMode::from_str(s).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_roundtrips_to_architect() {
        let state = SupervisorState::Plan;
        let mode = supervisor_to_acp(&state);
        assert_eq!(mode, AcpMode::Architect);
        assert_eq!(acp_to_supervisor(&mode), state);
    }

    #[test]
    fn exec_roundtrips_to_code() {
        let state = SupervisorState::Exec;
        let mode = supervisor_to_acp(&state);
        assert_eq!(mode, AcpMode::Code);
        assert_eq!(acp_to_supervisor(&mode), state);
    }

    #[test]
    fn review_roundtrips_to_ask() {
        let state = SupervisorState::Review;
        let mode = supervisor_to_acp(&state);
        assert_eq!(mode, AcpMode::Ask);
        assert_eq!(acp_to_supervisor(&mode), state);
    }

    #[test]
    fn acp_mode_id_emits_canonical_wire_tags() {
        assert_eq!(acp_mode_id(&AcpMode::Architect), "architect");
        assert_eq!(acp_mode_id(&AcpMode::Code), "code");
        assert_eq!(acp_mode_id(&AcpMode::Ask), "ask");
    }

    #[test]
    fn parse_acp_mode_id_roundtrips_canonical_tags() {
        for mode in [AcpMode::Architect, AcpMode::Code, AcpMode::Ask] {
            let parsed = parse_acp_mode_id(acp_mode_id(&mode));
            assert_eq!(parsed, Some(mode));
        }
    }

    #[test]
    fn parse_acp_mode_id_rejects_unknown_tag() {
        assert_eq!(parse_acp_mode_id("plan"), None);
        assert_eq!(parse_acp_mode_id("architect-mode"), None);
        assert_eq!(parse_acp_mode_id(""), None);
        assert_eq!(parse_acp_mode_id("EXEC"), None);
    }

    #[test]
    fn display_and_fromstr_cohere_for_both_enums() {
        for state in [
            SupervisorState::Plan,
            SupervisorState::Exec,
            SupervisorState::Review,
        ] {
            let tag = state.to_string();
            assert_eq!(tag, state.tag());
            assert_eq!(SupervisorState::from_str(&tag).unwrap(), state);
        }
        for mode in [AcpMode::Architect, AcpMode::Code, AcpMode::Ask] {
            let tag = mode.to_string();
            assert_eq!(tag, mode.tag());
            assert_eq!(AcpMode::from_str(&tag).unwrap(), mode);
        }
    }
}
