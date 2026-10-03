use crate::journal::Journal;
use crate::skills::{Engine, SkillManifest};
use tempfile::TempDir;

fn skill(id: &str, version: &str, priority: u32, engine: Engine) -> SkillManifest {
    SkillManifest {
        id: id.into(),
        version: version.into(),
        description: format!("{id} v{version}"),
        engine,
        priority,
        domain: Some("frontend".into()),
        language: Some("typescript".into()),
        framework: Some("react".into()),
        confidence: 0.85,
        conflicts: vec!["vue-ui-expert".into()],
        verified: true,
        ..Default::default()
    }
}

#[test]
fn save_skill_roundtrip_preserves_payload() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let s = skill("react-ui-expert", "1.4.2", 90, Engine::Coding);
    journal.save_skill(&s).expect("save");

    let raw = journal
        .skill_payload("react-ui-expert", "1.4.2")
        .expect("payload")
        .unwrap();
    let back: SkillManifest = serde_json::from_str(&raw).unwrap();
    assert_eq!(back.id, "react-ui-expert");
    assert_eq!(back.version, "1.4.2");
    assert_eq!(back.engine, Engine::Coding);
    assert_eq!(back.priority, 90);
    assert_eq!(back.domain.as_deref(), Some("frontend"));
    assert_eq!(back.conflicts, vec!["vue-ui-expert"]);

    let row = journal.skill_tail(10).expect("tail")[0].clone();
    assert_eq!(row.skill_id, "react-ui-expert");
    assert_eq!(row.version, "1.4.2");
    assert_eq!(row.engine, "coding");
    assert_eq!(row.priority, 90);
    assert!((row.confidence - 0.85_f64).abs() < 1e-6);
    assert!(row.verified);
    assert!(!row.auto_generated);
}

#[test]
fn save_skill_idempotent_replay_does_not_overwrite() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let s = skill("tailwind-expert", "0.2.0", 80, Engine::Coding);
    journal.save_skill(&s).expect("first");
    journal.save_skill(&s).expect("replay");
    let tail = journal.skill_tail(10).expect("tail");
    assert_eq!(tail.len(), 1, "idempotent replay (RFC 02 §3.1.2)");
}

#[test]
fn new_version_keeps_old_history() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    // Same skill id, two versions.
    journal
        .save_skill(&skill("react-ui-expert", "1.4.0", 70, Engine::Coding))
        .expect("v1");
    std::thread::sleep(std::time::Duration::from_millis(10));
    journal
        .save_skill(&skill("react-ui-expert", "1.4.2", 90, Engine::Coding))
        .expect("v2");
    let tail = journal.skill_tail(10).expect("tail");
    assert_eq!(tail.len(), 2, "both versions persisted");
    // Newest first.
    assert_eq!(tail[0].version, "1.4.2");
    assert_eq!(tail[1].version, "1.4.0");
    // Latest lookup returns the newest.
    let latest = journal
        .latest_skill("react-ui-expert")
        .expect("latest")
        .unwrap();
    assert_eq!(latest.version, "1.4.2");
    // And the old version's payload is still recoverable.
    let raw = journal
        .skill_payload("react-ui-expert", "1.4.0")
        .expect("payload")
        .unwrap();
    let old: SkillManifest = serde_json::from_str(&raw).unwrap();
    assert_eq!(old.priority, 70);
}

#[test]
fn unknown_skill_returns_none() {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open");
    let payload = journal.skill_payload("ghost", "1.0.0").expect("query");
    assert!(payload.is_none());
    let latest = journal.latest_skill("ghost").expect("query");
    assert!(latest.is_none());
}
