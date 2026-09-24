// Atlas OS — Learned rules persistence (RFC 16 §2 / §4, RFC 32 Phase 5 sub-fase 5.0,
// M30 `learned_rules`).
//
// Same JSON-blob + duplicated-index pattern as the other Journal writers: the
// `Pattern`'s `RuleWhen` / `RuleThen` are stored as JSON text (identical serde
// shape to the `learning::rules` YAML writer, so YAML ↔ Journal needs no
// translation layer); `lifecycle` / `priority` are denormalised for the HUD
// tail and the consultable-rules query without a JSON parse. `ON CONFLICT DO
// NOTHING` honours RFC 02 §3.1.2 at-least-once idempotency (first write wins).

use crate::journal::store::LearnedRuleRow;
use crate::learning::types::{Pattern, RuleLifecycle, RuleThen, RuleWhen};

fn row_from(
    id: String,
    priority: i64,
    lifecycle: String,
    was_correct: Option<i64>,
    n_applied: i64,
    created_at: String,
    updated_at: String,
) -> LearnedRuleRow {
    LearnedRuleRow {
        id,
        priority,
        lifecycle,
        was_correct,
        n_applied,
        created_at,
        updated_at,
    }
}

impl crate::journal::Journal {
    pub fn save_learned_rule(&self, pattern: &Pattern) -> anyhow::Result<()> {
        let when_json = serde_json::to_string(&pattern.when)?;
        let then_json = serde_json::to_string(&pattern.then)?;
        let conn = self.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO learned_rules
                (id, when_trigger, then_action, priority, lifecycle,
                 was_correct, n_applied, created_at, updated_at)
              VALUES (?1, ?2, ?3, ?4, ?5, NULL, 0, ?6, ?6)
              ON CONFLICT(id) DO NOTHING",
            rusqlite::params![
                pattern.rule_id,
                when_json,
                then_json,
                pattern.priority as i64,
                pattern.lifecycle.tag(),
                now,
            ],
        )?;
        Ok(())
    }

    pub fn get_learned_rule(&self, id: &str) -> anyhow::Result<Option<LearnedRuleRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, priority, lifecycle, was_correct, n_applied, created_at, updated_at
              FROM learned_rules WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(rusqlite::params![id], |row| {
            Ok(row_from(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })?;
        Ok(rows.next().transpose()?)
    }

    /// Decode the stored `when_trigger` / `then_action` blobs back into the
    /// canonical structs. Returns `None` when the id is unknown.
    pub fn learned_rule_when_then(&self, id: &str) -> anyhow::Result<Option<(RuleWhen, RuleThen)>> {
        let conn = self.conn.lock();
        let mut stmt =
            conn.prepare("SELECT when_trigger, then_action FROM learned_rules WHERE id = ?1")?;
        let mut rows = stmt.query_map(rusqlite::params![id], |row| {
            let when_json: String = row.get(0)?;
            let then_json: String = row.get(1)?;
            Ok((when_json, then_json))
        })?;
        match rows.next().transpose()? {
            Some((when_json, then_json)) => Ok(Some((
                serde_json::from_str(&when_json)?,
                serde_json::from_str(&then_json)?,
            ))),
            None => Ok(None),
        }
    }

    pub fn list_learned_rules(&self, limit: i64) -> anyhow::Result<Vec<LearnedRuleRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, priority, lifecycle, was_correct, n_applied, created_at, updated_at
              FROM learned_rules ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![limit], |row| {
            Ok(row_from(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Rules the 5.4 prompt hook may consult (`candidate` / `active`,
    /// mirroring `Pattern::is_consultable`), highest priority first.
    pub fn list_consultable_rules(&self, limit: i64) -> anyhow::Result<Vec<LearnedRuleRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, priority, lifecycle, was_correct, n_applied, created_at, updated_at
              FROM learned_rules
              WHERE lifecycle IN ('candidate','active')
              ORDER BY priority DESC, updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(rusqlite::params![limit], |row| {
            Ok(row_from(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Advance one lifecycle step: `draft → candidate` (priority 30),
    /// `candidate → active` (priority 60). `active` is idempotent;
    /// `deprecated` and unknown ids are errors.
    pub fn promote_rule(&self, id: &str) -> anyhow::Result<LearnedRuleRow> {
        let current = self
            .get_learned_rule(id)?
            .ok_or_else(|| anyhow::anyhow!("unknown learned rule `{id}`"))?;
        let (lifecycle, priority) = match current.lifecycle.as_str() {
            "draft" => (
                RuleLifecycle::Candidate,
                RuleLifecycle::Candidate.default_priority(),
            ),
            "candidate" => (
                RuleLifecycle::Active,
                RuleLifecycle::Active.default_priority(),
            ),
            "active" => return Ok(current),
            other => anyhow::bail!("cannot promote rule `{id}` from lifecycle `{other}`"),
        };
        {
            let conn = self.conn.lock();
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE learned_rules SET lifecycle = ?1, priority = ?2, updated_at = ?3
                  WHERE id = ?4",
                rusqlite::params![lifecycle.tag(), priority as i64, now, id],
            )?;
        }
        Ok(self
            .get_learned_rule(id)?
            .expect("row existed a moment ago"))
    }

    /// Retire a rule: any lifecycle → `deprecated`, priority 0.
    /// Idempotent; unknown ids are errors.
    pub fn deprecate_rule(&self, id: &str) -> anyhow::Result<LearnedRuleRow> {
        if self.get_learned_rule(id)?.is_none() {
            anyhow::bail!("unknown learned rule `{id}`");
        }
        {
            let conn = self.conn.lock();
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE learned_rules SET lifecycle = 'deprecated', priority = 0, updated_at = ?1
                  WHERE id = ?2",
                rusqlite::params![now, id],
            )?;
        }
        Ok(self
            .get_learned_rule(id)?
            .expect("row existed a moment ago"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::Journal;
    use crate::repair::types::RepairStrategy;
    use crate::validation::types::StageKind;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn fresh() -> (TempDir, Journal) {
        let dir = TempDir::new().unwrap();
        let j = Journal::open(dir.path()).unwrap();
        (dir, j)
    }

    fn fixture(rule_id: &str) -> Pattern {
        Pattern {
            pattern_id: Uuid::new_v4(),
            rule_id: rule_id.into(),
            when: RuleWhen {
                stage: StageKind::LintFormat,
                pattern: "no_println".into(),
                lang: "rust".into(),
            },
            then: RuleThen {
                strategy: RepairStrategy::AutoFix,
                skill: String::new(),
                diff_hint: "remove println".into(),
            },
            confidence: 0.7,
            lifecycle: RuleLifecycle::Draft,
            priority: RuleLifecycle::Draft.default_priority(),
            evidence: vec![Uuid::new_v4()],
            generated_at: "2026-07-04T12:00:00Z".into(),
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        }
    }

    #[test]
    fn save_and_get_roundtrip() {
        let (_d, j) = fresh();
        j.save_learned_rule(&fixture("r-2026-07-04-001")).unwrap();
        let row = j
            .get_learned_rule("r-2026-07-04-001")
            .unwrap()
            .expect("row");
        assert_eq!(row.lifecycle, "draft");
        assert_eq!(row.priority, 0);
        assert_eq!(row.was_correct, None);
        assert_eq!(row.n_applied, 0);
        let (when, then) = j
            .learned_rule_when_then("r-2026-07-04-001")
            .unwrap()
            .expect("blobs");
        assert_eq!(when.stage, StageKind::LintFormat);
        assert_eq!(when.pattern, "no_println");
        assert_eq!(then.strategy, RepairStrategy::AutoFix);
    }

    #[test]
    fn save_is_idempotent_on_replay() {
        let (_d, j) = fresh();
        j.save_learned_rule(&fixture("r-dup")).unwrap();
        j.save_learned_rule(&fixture("r-dup")).unwrap();
        assert_eq!(j.list_learned_rules(10).unwrap().len(), 1);
    }

    #[test]
    fn unknown_rule_returns_none() {
        let (_d, j) = fresh();
        assert!(j.get_learned_rule("r-missing").unwrap().is_none());
        assert!(j.learned_rule_when_then("r-missing").unwrap().is_none());
    }

    #[test]
    fn promote_advances_draft_to_candidate_to_active() {
        let (_d, j) = fresh();
        j.save_learned_rule(&fixture("r-promo")).unwrap();
        let c = j.promote_rule("r-promo").unwrap();
        assert_eq!(c.lifecycle, "candidate");
        assert_eq!(c.priority, 30);
        let a = j.promote_rule("r-promo").unwrap();
        assert_eq!(a.lifecycle, "active");
        assert_eq!(a.priority, 60);
        let again = j.promote_rule("r-promo").unwrap();
        assert_eq!(again.lifecycle, "active");
    }

    #[test]
    fn promote_unknown_or_deprecated_errors() {
        let (_d, j) = fresh();
        assert!(j.promote_rule("r-missing").is_err());
        j.save_learned_rule(&fixture("r-stale")).unwrap();
        j.deprecate_rule("r-stale").unwrap();
        assert!(j.promote_rule("r-stale").is_err());
    }

    #[test]
    fn deprecate_retires_and_zeroes_priority() {
        let (_d, j) = fresh();
        j.save_learned_rule(&fixture("r-old")).unwrap();
        j.promote_rule("r-old").unwrap();
        let d = j.deprecate_rule("r-old").unwrap();
        assert_eq!(d.lifecycle, "deprecated");
        assert_eq!(d.priority, 0);
        assert!(j.list_consultable_rules(10).unwrap().is_empty());
        assert!(j.deprecate_rule("r-missing").is_err());
    }

    #[test]
    fn consultable_lists_only_candidate_and_active() {
        let (_d, j) = fresh();
        j.save_learned_rule(&fixture("r-draft")).unwrap();
        j.save_learned_rule(&fixture("r-cand")).unwrap();
        j.save_learned_rule(&fixture("r-act")).unwrap();
        j.promote_rule("r-cand").unwrap();
        j.promote_rule("r-act").unwrap();
        j.promote_rule("r-act").unwrap();
        let rows = j.list_consultable_rules(10).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "r-act");
        assert_eq!(rows[1].id, "r-cand");
    }

    #[test]
    fn m30_migration_is_idempotent_and_preserves_rows() {
        let dir = TempDir::new().unwrap();
        let j = Journal::open(dir.path()).unwrap();
        j.save_learned_rule(&fixture("r-keep")).unwrap();
        drop(j);
        let j2 = Journal::open(dir.path()).unwrap();
        assert!(j2.get_learned_rule("r-keep").unwrap().is_some());
        let version: i64 = j2
            .conn
            .lock()
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .unwrap();
        assert!(
            version >= crate::journal::schema::CURRENT_SCHEMA_VERSION,
            "expected version >= CURRENT after re-migrate, got {version}"
        );
    }
}
