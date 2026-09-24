// Atlas OS — Swarm role presets (RFC 05 §1/§3, Phase 4 sub-fase 4.1, research/31 §A.1).
// agency-agents port: every role ships personality/processes/deliverables so the
// operator picks an installable preset instead of hand-wiring prompts. Three
// bundled presets: `atlas-team` (full 10-role RFC 05 §1 roster), `pair-programming`
// (driver + navigator), `solo-plus` (single executor + reviewer critic).

use serde::{Deserialize, Serialize};

use crate::swarm::roles::{ModelSlot, Role};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RolePreset {
    pub role: Role,
    pub personality: String,
    pub processes: Vec<String>,
    pub deliverables: Vec<String>,
    pub model_slot: ModelSlot,
}

impl RolePreset {
    pub fn new(role: Role, personality: &str, processes: &[&str], deliverables: &[&str]) -> Self {
        Self {
            model_slot: role.model_slot(),
            role,
            personality: personality.to_string(),
            processes: processes.iter().map(|s| s.to_string()).collect(),
            deliverables: deliverables.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn personality_json(&self) -> String {
        serde_json::json!({
            "personality": self.personality,
            "processes": self.processes,
            "deliverables": self.deliverables,
        })
        .to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    pub description: String,
    pub members: Vec<RolePreset>,
}

impl Preset {
    pub fn roles(&self) -> Vec<Role> {
        self.members.iter().map(|m| m.role).collect()
    }

    pub fn preset_for_role(&self, role: Role) -> Option<&RolePreset> {
        self.members.iter().find(|m| m.role == role)
    }
}

fn full_team() -> Vec<RolePreset> {
    vec![
        RolePreset::new(
            Role::Planner,
            "Decomposes the mission into ordered sub-goals. Decides scope, never writes code.",
            &[
                "split mission into sub-goals",
                "order by dependency",
                "hand off to architect",
            ],
            &["sub-goal list with acceptance criteria"],
        ),
        RolePreset::new(
            Role::Researcher,
            "Collects evidence from docs and code. Reports facts, never decides.",
            &[
                "gather sources",
                "cite file paths and URLs",
                "hand evidence to architect",
            ],
            &["evidence pack with references"],
        ),
        RolePreset::new(
            Role::Architect,
            "Turns sub-goals plus evidence into a buildable design. Designs, never implements.",
            &[
                "draft module boundaries",
                "define diff-level steps",
                "hand plan to executors",
            ],
            &["design doc with file-level steps"],
        ),
        RolePreset::new(
            Role::Backend,
            "Implements server-side logic on an isolated worktree. Never touches frontend files.",
            &[
                "implement backend steps",
                "keep diffs scoped",
                "emit diff for review",
            ],
            &["backend diff on own branch"],
        ),
        RolePreset::new(
            Role::Frontend,
            "Implements UI on an isolated worktree. Never touches backend files.",
            &[
                "implement UI steps",
                "keep diffs scoped",
                "emit diff for review",
            ],
            &["frontend diff on own branch"],
        ),
        RolePreset::new(
            Role::Database,
            "Owns schema alterations. Never touches business logic.",
            &[
                "draft migration",
                "verify backwards compatibility",
                "emit diff for review",
            ],
            &["schema migration diff"],
        ),
        RolePreset::new(
            Role::Security,
            "Audits diffs for vulnerabilities. Audits, never adds features.",
            &[
                "review diff for vulns",
                "flag secrets and injection",
                "approve or block",
            ],
            &["security verdict per diff"],
        ),
        RolePreset::new(
            Role::Testing,
            "Writes and runs tests. Tests, never refactors logic.",
            &["cover new paths", "run suite", "report failures with logs"],
            &["test run report"],
        ),
        RolePreset::new(
            Role::Reviewer,
            "Critiques every diff for correctness and style. Reviews, never edits files.",
            &[
                "check correctness",
                "check style",
                "approve or request changes",
            ],
            &["review verdict per diff"],
        ),
        RolePreset::new(
            Role::Merger,
            "Merges approved diffs in deterministic order. Merges, never generates content.",
            &[
                "order approved diffs",
                "apply batch merge",
                "hand off to validation",
            ],
            &["merged branch ready for validation"],
        ),
    ]
}

pub fn bundled_presets() -> Vec<Preset> {
    let team = full_team();
    let by_role = |team: &[RolePreset], r: Role| {
        team.iter()
            .find(|m| m.role == r)
            .cloned()
            .expect("full team covers every role")
    };
    let pair = vec![
        by_role(&team, Role::Backend),
        by_role(&team, Role::Frontend),
    ];
    let solo = vec![
        by_role(&team, Role::Backend),
        by_role(&team, Role::Reviewer),
    ];
    vec![
        Preset {
            id: "atlas-team".to_string(),
            description: "Full 10-role RFC 05 roster: plan, research, design, parallel executors, review, merge.".to_string(),
            members: team,
        },
        Preset {
            id: "pair-programming".to_string(),
            description: "Driver plus navigator: backend implements, frontend mirrors the surface.".to_string(),
            members: pair,
        },
        Preset {
            id: "solo-plus".to_string(),
            description: "Single executor plus critic: backend implements, reviewer validates.".to_string(),
            members: solo,
        },
    ]
}

pub fn find_preset(id: &str) -> Option<Preset> {
    bundled_presets().into_iter().find(|p| p.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_presets_cover_expected_ids_and_sizes() {
        let presets = bundled_presets();
        assert_eq!(presets.len(), 3);
        let team = presets.iter().find(|p| p.id == "atlas-team").unwrap();
        assert_eq!(team.members.len(), 10);
        assert_eq!(team.roles().len(), 10);
        let pair = presets.iter().find(|p| p.id == "pair-programming").unwrap();
        assert_eq!(pair.roles(), vec![Role::Backend, Role::Frontend]);
        let solo = presets.iter().find(|p| p.id == "solo-plus").unwrap();
        assert_eq!(solo.roles(), vec![Role::Backend, Role::Reviewer]);
    }

    #[test]
    fn every_member_carries_personality_processes_deliverables() {
        for preset in bundled_presets() {
            assert!(
                !preset.description.is_empty(),
                "preset {} needs a description",
                preset.id
            );
            for m in &preset.members {
                assert!(!m.personality.is_empty(), "{:?} needs personality", m.role);
                assert!(!m.processes.is_empty(), "{:?} needs processes", m.role);
                assert!(
                    !m.deliverables.is_empty(),
                    "{:?} needs deliverables",
                    m.role
                );
                assert_eq!(m.model_slot, m.role.model_slot());
            }
        }
    }

    #[test]
    fn find_preset_returns_none_for_unknown_id() {
        assert!(find_preset("ghost-team").is_none());
        assert!(find_preset("").is_none());
        assert_eq!(find_preset("solo-plus").unwrap().id, "solo-plus");
    }

    #[test]
    fn preset_for_role_locates_member_or_none() {
        let solo = find_preset("solo-plus").unwrap();
        assert_eq!(
            solo.preset_for_role(Role::Reviewer).unwrap().role,
            Role::Reviewer
        );
        assert!(solo.preset_for_role(Role::Planner).is_none());
    }

    #[test]
    fn model_slot_mapping_routes_roles_to_tri_model_tiers() {
        for r in [Role::Planner, Role::Architect, Role::Reviewer] {
            assert_eq!(r.model_slot(), ModelSlot::Architect, "{r:?}");
        }
        for r in [
            Role::Backend,
            Role::Frontend,
            Role::Database,
            Role::Security,
        ] {
            assert_eq!(r.model_slot(), ModelSlot::Editor, "{r:?}");
        }
        for r in [Role::Researcher, Role::Testing, Role::Merger] {
            assert_eq!(r.model_slot(), ModelSlot::Weak, "{r:?}");
        }
    }

    #[test]
    fn resolve_model_uses_tri_model_overrides_per_slot() {
        use crate::profiles::{Profile, ProfileId};
        let profile = Profile {
            id: ProfileId::new("t"),
            bail_out_threshold_secs: 60,
            backup_profile_id: None,
            main_model_id: Some("main".into()),
            architect_model_id: Some("strong".into()),
            editor_model_id: Some("mid".into()),
            weak_model_id: Some("cheap".into()),
            resource_mode: "mixed".into(),
            ..Default::default()
        };
        assert_eq!(
            Role::Planner.resolve_model(&profile).as_deref(),
            Some("strong")
        );
        assert_eq!(
            Role::Backend.resolve_model(&profile).as_deref(),
            Some("mid")
        );
        assert_eq!(
            Role::Testing.resolve_model(&profile).as_deref(),
            Some("cheap")
        );
    }

    #[test]
    fn resolve_model_falls_back_to_main_then_none() {
        use crate::profiles::{Profile, ProfileId};
        let with_main = Profile {
            id: ProfileId::new("t"),
            bail_out_threshold_secs: 60,
            backup_profile_id: None,
            main_model_id: Some("main".into()),
            architect_model_id: None,
            editor_model_id: None,
            weak_model_id: None,
            resource_mode: "mixed".into(),
            ..Default::default()
        };
        assert_eq!(with_main.effective_architect_model(), Some("main"));
        assert_eq!(
            Role::Planner.resolve_model(&with_main).as_deref(),
            Some("main")
        );
        assert_eq!(
            Role::Backend.resolve_model(&with_main).as_deref(),
            Some("main")
        );
        assert_eq!(
            Role::Merger.resolve_model(&with_main).as_deref(),
            Some("main")
        );

        let empty = Profile::default_for(ProfileId::new("e"));
        assert_eq!(Role::Planner.resolve_model(&empty), None);
        assert_eq!(Role::Backend.resolve_model(&empty), None);
    }

    #[test]
    fn personality_json_round_trips_through_serde() {
        let preset = find_preset("solo-plus").unwrap();
        let member = preset.preset_for_role(Role::Backend).unwrap();
        let raw = member.personality_json();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            v["personality"],
            serde_json::Value::String(member.personality.clone())
        );
        assert!(v["processes"].is_array());
        assert!(v["deliverables"].is_array());
    }
}
