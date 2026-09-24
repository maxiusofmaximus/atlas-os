// Atlas OS — Swarm agent roles (RFC 05 §1, Phase 4 sub-fase 4.0).
// Closed-purpose roles: planners/researchers/architects never touch code,
// executors run in parallel on isolated worktrees, reviewer/merger
// consolidate. Model-per-role assignment (RFC 05 §3) and installable
// personality presets (agency-agents port) land in sub-fase 4.1.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelSlot {
    Architect,
    Editor,
    Weak,
}

impl ModelSlot {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelSlot::Architect => "architect",
            ModelSlot::Editor => "editor",
            ModelSlot::Weak => "weak",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "architect" => Some(ModelSlot::Architect),
            "editor" => Some(ModelSlot::Editor),
            "weak" => Some(ModelSlot::Weak),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Planner,
    Researcher,
    Architect,
    Backend,
    Frontend,
    Database,
    Security,
    Testing,
    Reviewer,
    Merger,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Planner => "planner",
            Role::Researcher => "researcher",
            Role::Architect => "architect",
            Role::Backend => "backend",
            Role::Frontend => "frontend",
            Role::Database => "database",
            Role::Security => "security",
            Role::Testing => "testing",
            Role::Reviewer => "reviewer",
            Role::Merger => "merger",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "planner" => Some(Role::Planner),
            "researcher" => Some(Role::Researcher),
            "architect" => Some(Role::Architect),
            "backend" => Some(Role::Backend),
            "frontend" => Some(Role::Frontend),
            "database" => Some(Role::Database),
            "security" => Some(Role::Security),
            "testing" => Some(Role::Testing),
            "reviewer" => Some(Role::Reviewer),
            "merger" => Some(Role::Merger),
            _ => None,
        }
    }

    pub const ALL: [Role; 10] = [
        Role::Planner,
        Role::Researcher,
        Role::Architect,
        Role::Backend,
        Role::Frontend,
        Role::Database,
        Role::Security,
        Role::Testing,
        Role::Reviewer,
        Role::Merger,
    ];

    pub fn model_slot(&self) -> ModelSlot {
        match self {
            Role::Planner | Role::Architect | Role::Reviewer => ModelSlot::Architect,
            Role::Backend | Role::Frontend | Role::Database | Role::Security => ModelSlot::Editor,
            Role::Researcher | Role::Testing | Role::Merger => ModelSlot::Weak,
        }
    }

    pub fn resolve_model(&self, profile: &crate::profiles::Profile) -> Option<String> {
        let slot = match self.model_slot() {
            ModelSlot::Architect => profile.effective_architect_model(),
            ModelSlot::Editor => profile.effective_editor_model(),
            ModelSlot::Weak => profile.effective_weak_model(),
        };
        slot.map(str::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_as_str_round_trips_via_parse() {
        for r in Role::ALL {
            assert_eq!(Role::parse(r.as_str()), Some(r));
        }
        assert_eq!(Role::parse("planner"), Some(Role::Planner));
        assert_eq!(Role::parse("merger"), Some(Role::Merger));
    }

    #[test]
    fn role_all_enumerates_ten_rfc05_roles() {
        assert_eq!(Role::ALL.len(), 10);
        assert!(Role::ALL.contains(&Role::Planner));
        assert!(Role::ALL.contains(&Role::Merger));
    }

    #[test]
    fn role_serde_wire_names_match_as_str() {
        for r in Role::ALL {
            let json = serde_json::to_string(&r).unwrap();
            assert_eq!(json, format!("\"{}\"", r.as_str()));
        }
    }

    #[test]
    fn role_parse_rejects_unknown_strings() {
        assert_eq!(Role::parse("lead"), None);
        assert_eq!(Role::parse(""), None);
        assert_eq!(Role::parse("Backend"), None);
    }

    #[test]
    fn model_slot_as_str_round_trips_via_parse() {
        for s in [ModelSlot::Architect, ModelSlot::Editor, ModelSlot::Weak] {
            assert_eq!(ModelSlot::parse(s.as_str()), Some(s));
        }
        assert_eq!(ModelSlot::parse("strong"), None);
        assert_eq!(ModelSlot::parse(""), None);
    }
}
