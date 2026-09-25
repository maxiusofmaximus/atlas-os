// Atlas OS — Sandbox levels + approval policy (RFC 18 §2/§6, Phase 7 sub-fase 7.1).
//
// MVP is types + policy mapping; real Docker/Podman execution stays a
// documented follow-up (research/34 SECTOR A.2 — external infra).
// Fail-safe: unknown levels or actions resolve to `Forbidden`.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SandboxLevel {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "vuOnly")]
    VuOnly,
    #[serde(rename = "container")]
    Container,
    #[serde(rename = "wasm")]
    Wasm,
}

impl SandboxLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SandboxLevel::None => "none",
            SandboxLevel::VuOnly => "vuOnly",
            SandboxLevel::Container => "container",
            SandboxLevel::Wasm => "wasm",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "none" => Some(SandboxLevel::None),
            "vuOnly" => Some(SandboxLevel::VuOnly),
            "container" => Some(SandboxLevel::Container),
            "wasm" => Some(SandboxLevel::Wasm),
            _ => None,
        }
    }

    pub const ALL: [SandboxLevel; 4] = [
        SandboxLevel::None,
        SandboxLevel::VuOnly,
        SandboxLevel::Container,
        SandboxLevel::Wasm,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Approval {
    Auto,
    Confirm,
    Forbidden,
}

impl Approval {
    pub fn as_str(&self) -> &'static str {
        match self {
            Approval::Auto => "auto",
            Approval::Confirm => "confirm",
            Approval::Forbidden => "forbidden",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Approval::Auto),
            "confirm" => Some(Approval::Confirm),
            "forbidden" => Some(Approval::Forbidden),
            _ => None,
        }
    }

    pub const ALL: [Approval; 3] = [Approval::Auto, Approval::Confirm, Approval::Forbidden];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitiveAction {
    ReadWorkspace,
    WriteWorkspace,
    WriteOutsideWorkspace,
    NetworkEgress,
    ShellInteractive,
    ShellSandbox,
    InstallOpenSkill,
    InstallClosedSkillUnsigned,
    PulumiApply,
    SendSecrets,
    AccessSecrets,
    DeleteLargeFile,
    GitPush,
}

impl SensitiveAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            SensitiveAction::ReadWorkspace => "read_workspace",
            SensitiveAction::WriteWorkspace => "write_workspace",
            SensitiveAction::WriteOutsideWorkspace => "write_outside_workspace",
            SensitiveAction::NetworkEgress => "network_egress",
            SensitiveAction::ShellInteractive => "shell_interactive",
            SensitiveAction::ShellSandbox => "shell_sandbox",
            SensitiveAction::InstallOpenSkill => "install_open_skill",
            SensitiveAction::InstallClosedSkillUnsigned => "install_closed_skill_unsigned",
            SensitiveAction::PulumiApply => "pulumi_apply",
            SensitiveAction::SendSecrets => "send_secrets",
            SensitiveAction::AccessSecrets => "access_secrets",
            SensitiveAction::DeleteLargeFile => "delete_large_file",
            SensitiveAction::GitPush => "git_push",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "read_workspace" => Some(SensitiveAction::ReadWorkspace),
            "write_workspace" => Some(SensitiveAction::WriteWorkspace),
            "write_outside_workspace" => Some(SensitiveAction::WriteOutsideWorkspace),
            "network_egress" => Some(SensitiveAction::NetworkEgress),
            "shell_interactive" => Some(SensitiveAction::ShellInteractive),
            "shell_sandbox" => Some(SensitiveAction::ShellSandbox),
            "install_open_skill" => Some(SensitiveAction::InstallOpenSkill),
            "install_closed_skill_unsigned" => Some(SensitiveAction::InstallClosedSkillUnsigned),
            "pulumi_apply" => Some(SensitiveAction::PulumiApply),
            "send_secrets" => Some(SensitiveAction::SendSecrets),
            "access_secrets" => Some(SensitiveAction::AccessSecrets),
            "delete_large_file" => Some(SensitiveAction::DeleteLargeFile),
            "git_push" => Some(SensitiveAction::GitPush),
            _ => None,
        }
    }

    pub const ALL: [SensitiveAction; 13] = [
        SensitiveAction::ReadWorkspace,
        SensitiveAction::WriteWorkspace,
        SensitiveAction::WriteOutsideWorkspace,
        SensitiveAction::NetworkEgress,
        SensitiveAction::ShellInteractive,
        SensitiveAction::ShellSandbox,
        SensitiveAction::InstallOpenSkill,
        SensitiveAction::InstallClosedSkillUnsigned,
        SensitiveAction::PulumiApply,
        SensitiveAction::SendSecrets,
        SensitiveAction::AccessSecrets,
        SensitiveAction::DeleteLargeFile,
        SensitiveAction::GitPush,
    ];
}

fn base_approval(action: SensitiveAction) -> Approval {
    match action {
        SensitiveAction::ReadWorkspace => Approval::Auto,
        SensitiveAction::WriteWorkspace => Approval::Confirm,
        SensitiveAction::WriteOutsideWorkspace => Approval::Forbidden,
        SensitiveAction::NetworkEgress => Approval::Forbidden,
        SensitiveAction::ShellInteractive => Approval::Forbidden,
        SensitiveAction::ShellSandbox => Approval::Confirm,
        SensitiveAction::InstallOpenSkill => Approval::Confirm,
        SensitiveAction::InstallClosedSkillUnsigned => Approval::Forbidden,
        SensitiveAction::PulumiApply => Approval::Confirm,
        SensitiveAction::SendSecrets => Approval::Forbidden,
        SensitiveAction::AccessSecrets => Approval::Forbidden,
        SensitiveAction::DeleteLargeFile => Approval::Confirm,
        SensitiveAction::GitPush => Approval::Confirm,
    }
}

pub fn approval_for(level: SandboxLevel, action: SensitiveAction) -> Approval {
    match level {
        SandboxLevel::None => match action {
            SensitiveAction::ReadWorkspace => Approval::Auto,
            _ => Approval::Forbidden,
        },
        SandboxLevel::VuOnly | SandboxLevel::Container | SandboxLevel::Wasm => {
            base_approval(action)
        }
    }
}

pub fn approval_for_str(level: &str, action: &str) -> Approval {
    match (SandboxLevel::parse(level), SensitiveAction::parse(action)) {
        (Some(l), Some(a)) => approval_for(l, a),
        _ => Approval::Forbidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_level_round_trips_via_parse() {
        for l in SandboxLevel::ALL {
            assert_eq!(SandboxLevel::parse(l.as_str()), Some(l));
        }
        assert_eq!(SandboxLevel::parse("vuOnly"), Some(SandboxLevel::VuOnly));
        assert_eq!(SandboxLevel::parse("vu_only"), None);
        assert_eq!(SandboxLevel::parse(""), None);
        assert_eq!(SandboxLevel::parse("Docker"), None);
    }

    #[test]
    fn approval_and_action_round_trip() {
        for a in Approval::ALL {
            assert_eq!(Approval::parse(a.as_str()), Some(a));
        }
        assert_eq!(Approval::parse("allow"), None);
        for a in SensitiveAction::ALL {
            assert_eq!(SensitiveAction::parse(a.as_str()), Some(a));
        }
        assert_eq!(SensitiveAction::parse("npm_install"), None);
        assert_eq!(SensitiveAction::parse(""), None);
    }

    #[test]
    fn vu_only_matches_rfc18_approval_table() {
        use Approval::{Auto, Confirm, Forbidden};
        use SensitiveAction::*;
        let table: [(SensitiveAction, Approval); 13] = [
            (ReadWorkspace, Auto),
            (WriteWorkspace, Confirm),
            (WriteOutsideWorkspace, Forbidden),
            (NetworkEgress, Forbidden),
            (ShellInteractive, Forbidden),
            (ShellSandbox, Confirm),
            (InstallOpenSkill, Confirm),
            (InstallClosedSkillUnsigned, Forbidden),
            (PulumiApply, Confirm),
            (SendSecrets, Forbidden),
            (AccessSecrets, Forbidden),
            (DeleteLargeFile, Confirm),
            (GitPush, Confirm),
        ];
        for (action, expected) in table {
            assert_eq!(
                approval_for(SandboxLevel::VuOnly, action),
                expected,
                "action {}",
                action.as_str()
            );
            assert_eq!(approval_for(SandboxLevel::Container, action), expected);
            assert_eq!(approval_for(SandboxLevel::Wasm, action), expected);
        }
    }

    #[test]
    fn none_level_tightens_to_inner_core_only() {
        assert_eq!(
            approval_for(SandboxLevel::None, SensitiveAction::ReadWorkspace),
            Approval::Auto
        );
        for action in SensitiveAction::ALL {
            if action == SensitiveAction::ReadWorkspace {
                continue;
            }
            assert_eq!(
                approval_for(SandboxLevel::None, action),
                Approval::Forbidden,
                "action {}",
                action.as_str()
            );
        }
    }

    #[test]
    fn unknown_level_or_action_fails_safe_to_forbidden() {
        assert_eq!(
            approval_for_str("nope", "read_workspace"),
            Approval::Forbidden
        );
        assert_eq!(approval_for_str("none", "nope"), Approval::Forbidden);
        assert_eq!(approval_for_str("", ""), Approval::Forbidden);
        assert_eq!(
            approval_for_str("vu_only", "read_workspace"),
            Approval::Forbidden
        );
        assert_eq!(approval_for_str("vuOnly", "read_workspace"), Approval::Auto);
    }

    #[test]
    fn serde_wire_names_match_as_str() {
        for l in SandboxLevel::ALL {
            let json = serde_json::to_string(&l).unwrap();
            assert_eq!(json, format!("\"{}\"", l.as_str()), "{l:?}");
        }
        for a in Approval::ALL {
            let json = serde_json::to_string(&a).unwrap();
            assert_eq!(json, format!("\"{}\"", a.as_str()), "{a:?}");
        }
        for a in SensitiveAction::ALL {
            let json = serde_json::to_string(&a).unwrap();
            assert_eq!(json, format!("\"{}\"", a.as_str()), "{a:?}");
        }
    }
}
