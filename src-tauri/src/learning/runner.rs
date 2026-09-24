// Atlas OS — Learning Engine runner (RFC 16 §2 / §3 / §7).
//
// The runner is a pure function of a `RepairReport`. It does NOT call
// the model, does NOT touch disk, does NOT consult the Skill Graph —
// Phase 1 is pure pattern derivation. The Execution Supervisor (RFC
// 19) calls `learning::run` after every `repair::run` and persists the
// resulting `LearnOutcome` via `Journal::save_pattern` (M7).
//
// Heuristic derivation rules (RFC 16 §3 "Por error"):
//
//   * `RepairOutcome::Applied`  → emit a `Draft` pattern whose `when`
//     mirrors the stage + rule of the converged attempt and whose
//     `then` records the successful `RepairStrategy` plus the
//     literal hunk as `diff_hint`. Confidence starts at 0.7 (the
//     converged AutoFix is strong evidence).
//
//   * `RepairOutcome::Proposed` → emit a `Draft` with confidence 0.5;
//     the re-validation hasn't confirmed convergence yet.
//
//   * `RepairOutcome::EscalatedPlanning` → emit a `Draft` only when
//     the strategy was NOT `PlanningReplan` triggered by budget
//     exhaustion (we don't want to codify "give up" as a rule). The
//     confidence is 0.3 — weak evidence that the original strategy
//     would have worked.
//
//   * `RepairOutcome::EscalatedHuman`  → `NoPattern` (RFC 16 §8).
//
//   * Empty `attempts` → `NoPattern`.

use uuid::Uuid;

use crate::repair::types::{RepairOutcome, RepairStrategy};
use crate::validation::types::StageKind;

use super::types::{
    LearnInput, LearnOutcome, Pattern, PatternMetrics, RuleLifecycle, RuleThen, RuleWhen,
};

use crate::repair::types::RepairReport;

/// RFC 16 §2 entry point. Produces one `LearnOutcome` per
/// `RepairReport`. Never panics; malformed inputs surface as a
/// `NoPattern` outcome with a diagnostic note.
pub fn run(input: LearnInput) -> LearnOutcome {
    let repair = input.repair;

    let (pattern, note) = derive(input);
    let metrics = metrics_for(repair, &pattern);
    let evidence_diff = pattern
        .as_ref()
        .and_then(|p| {
            repair
                .attempts
                .iter()
                .find(|a| p.then.strategy == a.strategy && a.success)
                .and_then(|a| a.proposed_diff.clone())
        })
        .map(|b| Box::new(*b));

    LearnOutcome {
        learn_id: Uuid::new_v4(),
        source_repair_id: repair.repair_id,
        source_mission_id: repair.mission_id,
        generated_at: chrono::Utc::now().to_rfc3339(),
        pattern,
        metrics,
        note,
        evidence_diff,
    }
}

/// Pure helper — produces `(Option<Pattern>, note)` from the repair
/// run. Extracted so the public `run` can layer metrics + evidence
/// diff on top.
fn derive(input: LearnInput) -> (Option<Pattern>, String) {
    let repair = input.repair;

    if repair.attempts.is_empty() {
        return (None, "no attempts — nothing to learn from".into());
    }
    if matches!(repair.outcome, RepairOutcome::EscalatedHuman) {
        return (None, "RFC 16 §8 — escalated to human, no auto-rule".into());
    }

    // Pick the attempt we want to canonise. Prefer the successful one
    // (RFC 16 §3 "Por error"); fall back to the last attempt so a
    // Proposed/EscalatedPlanning run still yields a draft for the
    // reviewer to triage.
    let chosen = repair
        .attempts
        .iter()
        .find(|a| a.success)
        .or_else(|| repair.attempts.last())
        .expect("non-empty attempts checked above");

    // Don't codify "give up" as a rule (RFC 15 §4 budget escalation
    // surfaced through PlanningReplan is audit-only, never a Pattern).
    if chosen.strategy == RepairStrategy::PlanningReplan
        && matches!(
            repair.outcome,
            RepairOutcome::EscalatedPlanning | RepairOutcome::EscalatedHuman
        )
    {
        return (
            None,
            "budget-exhaustion PlanningReplan is not a learnable rule".into(),
        );
    }
    if chosen.strategy == RepairStrategy::HumanEscalate {
        return (None, "HumanEscalate is not a learnable rule".into());
    }

    let when = RuleWhen {
        stage: repair.triggering_stage,
        pattern: rule_tag_from_attempt(chosen, repair.triggering_stage),
        lang: lang_from_diff(repair).unwrap_or_else(|| "*".into()),
    };
    let then = RuleThen {
        strategy: chosen.strategy,
        skill: String::new(),
        diff_hint: diff_hint_from_attempt(chosen),
    };
    let confidence = match repair.outcome {
        RepairOutcome::Applied => 0.7,
        RepairOutcome::Proposed => 0.5,
        RepairOutcome::EscalatedPlanning => 0.3,
        RepairOutcome::EscalatedHuman => 0.0,
    };
    let lifecycle = RuleLifecycle::Draft;
    let evidence = vec![
        repair.repair_id,
        repair.triggered_by_report_id,
        repair.source_diff_id,
    ];
    let rule_id = build_rule_id(repair, input.mission_pattern_index);

    let note = format!(
        "draft rule `{rule_id}` from repair {} (outcome={}, strategy={})",
        repair.repair_id,
        repair.outcome.tag(),
        chosen.strategy.tag(),
    );

    let pattern = Pattern {
        pattern_id: Uuid::new_v4(),
        rule_id,
        when,
        then,
        confidence,
        lifecycle,
        priority: lifecycle.default_priority(),
        evidence,
        generated_at: chrono::Utc::now().to_rfc3339(),
        model_id: "heuristic-v0".into(),
        elapsed_ms: 0,
    };
    (Some(pattern), note)
}

/// Heuristic — look at the chosen attempt's proposed diff (if any) and
/// derive a `rule` tag back. When the runner already recorded a
/// `root_cause` mentioning a known sentinel (e.g. "no_println"), we
/// reuse it; otherwise we fall back to the StageKind tag so the `when`
/// block is never empty.
fn rule_tag_from_attempt(a: &crate::repair::types::RepairAttempt, stage: StageKind) -> String {
    // `root_cause` from `describe_root_cause` contains the rule tag in
    // backticks for Lint/Type findings, e.g. "lint rule `no_println`".
    if let Some(start) = a.root_cause.find('`') {
        if let Some(end) = a.root_cause[start + 1..].find('`') {
            return a.root_cause[start + 1..start + 1 + end].to_string();
        }
    }
    stage.tag().to_string()
}

/// Heuristic — pull the language out of the repair's source diff by
/// looking at the file extension of the first edit. `None` when the
/// diff carries no files.
fn lang_from_diff(repair: &crate::repair::types::RepairReport) -> Option<String> {
    // We don't have the diff inline here, so we infer from the strategy
    // + stage: TypeCheck+rust mission → "rust"; otherwise "*". This is
    // a conservative Phase-1 heuristic. Phase 2 will look at the real
    // `source_diff_id` via the Journal.
    if matches!(repair.triggering_stage, StageKind::TypeCheck)
        && matches!(
            repair.attempts.first()?.strategy,
            RepairStrategy::AutoFix | RepairStrategy::CodingAmendment
        )
    {
        return Some("rust".into());
    }
    None
}

/// Build the human-readable `rule_id`. Format: `r-YYYY-MM-DD-NNN`
/// where NNN is the per-mission counter passed by the Execution
/// Supervisor. Two patterns from the same mission thus share a date
/// prefix and increment the suffix.
fn build_rule_id(repair: &crate::repair::types::RepairReport, idx: u32) -> String {
    let date = repair.generated_at.get(..10).unwrap_or("unknown");
    format!("r-{date}-{:03}", idx)
}

/// Build the one-line `diff_hint` from the chosen attempt's proposed
/// diff (the first hunk's first new line, truncated). Empty string
/// when the attempt has no proposed diff.
fn diff_hint_from_attempt(a: &crate::repair::types::RepairAttempt) -> String {
    if let Some(diff) = &a.proposed_diff {
        if let Some(file) = diff.files.first() {
            if let Some(hunk) = file.hunks.first() {
                if let Some(line) = hunk.new_lines.first() {
                    let truncated = truncate(line, 80);
                    return format!("{}: {truncated}", file.path);
                }
            }
        }
    }
    String::new()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

/// RFC 16 §7 — fill the metric counters we can derive heuristically.
/// Phase 1 only fills `was_correct` (1 iff `Applied`) and
/// `tests_passed` (1 iff `Applied` AND the stage was `UnitTests`).
fn metrics_for(
    repair: &crate::repair::types::RepairReport,
    pattern: &Option<Pattern>,
) -> PatternMetrics {
    let was_correct = if matches!(repair.outcome, RepairOutcome::Applied) {
        1
    } else {
        0
    };
    let tests_passed = if matches!(repair.outcome, RepairOutcome::Applied)
        && matches!(repair.triggering_stage, StageKind::UnitTests)
    {
        1
    } else {
        0
    };
    let _ = pattern; // Phase 2 will fold the pattern's historical metrics here.
    PatternMetrics {
        was_correct,
        was_blocked: 0,
        tests_passed,
        approved_by_user: 0,
    }
}

// ——— Sub-fase 5.1 Reflection Engine formal (RFC 16 §2 loop, RFC 32 SECTOR B 5.1) ———
//
// Formalises the error → root cause → missing rule → draft → promote loop
// on top of the Phase-1 `run` heuristic:
//
//   * `error_signature` — deterministic dedup key per `RepairReport`
//     (`stage|error_class|rule_tag`, rule tag lowercased + trimmed).
//   * `reflect` — groups a batch of repairs by signature so N repetitions
//     of the same error collapse to ONE draft `Pattern` (evidence merged,
//     confidence bumped +0.05 per extra repetition, capped at 0.95).
//     Reports that `run` would reject (empty attempts, EscalatedHuman,
//     HumanEscalate, budget-exhaustion PlanningReplan) are skipped.
//   * `promote_draft` — advances `Draft → Candidate` iff the observed
//     `was_correct` (M7 `pattern_runs`, mirrored on `learned_rules`) meets
//     `PROMOTE_THRESHOLD`. Other lifecycles pass through untouched.
//   * `should_deprecate` / `deprecate_stale` — stale predicate (blocked
//     strictly more often than correct) + the lifecycle transition to
//     `Deprecated` (priority 0, never consulted).

/// Minimum `was_correct` observations before a draft is trusted enough
/// to become a `Candidate` (RFC 16 §2: draft priority 0 → candidate 30).
pub const PROMOTE_THRESHOLD: u32 = 2;

/// Confidence bump per extra repetition of the same signature inside one
/// `reflect` batch. A repeated error is stronger evidence than a single
/// sighting, but the cap keeps a heuristic draft below `Active` priors.
pub const DEDUP_CONFIDENCE_BUMP: f32 = 0.05;

/// Hard ceiling for the dedup bump so `reflect` never mints an
/// over-confident draft from repetition alone.
pub const DEDUP_CONFIDENCE_CAP: f32 = 0.95;

/// Deterministic dedup key for one repair run. Normalises the rule tag
/// (trim + lowercase) so `No_Println`, `no_println ` and `no_println`
/// collapse to the same bucket.
pub fn error_signature(report: &RepairReport) -> String {
    let chosen = report
        .attempts
        .iter()
        .find(|a| a.success)
        .or_else(|| report.attempts.last());
    let (class_tag, rule_tag) = match chosen {
        Some(a) => (
            a.error_class.tag().to_string(),
            rule_tag_from_attempt(a, report.triggering_stage)
                .trim()
                .to_lowercase(),
        ),
        None => (
            "none".to_string(),
            report.triggering_stage.tag().to_string(),
        ),
    };
    format!(
        "{}|{}|{}",
        report.triggering_stage.tag(),
        class_tag,
        rule_tag
    )
}

/// Group a batch of repairs by `error_signature` and emit ONE draft
/// `Pattern` per group. Deterministic: groups are visited in sorted
/// signature order and the output is sorted by `rule_id`.
pub fn reflect(repairs: &[RepairReport]) -> Vec<Pattern> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, Vec<&RepairReport>> = BTreeMap::new();
    for report in repairs {
        if report.attempts.is_empty() {
            continue;
        }
        if matches!(
            report.outcome,
            crate::repair::types::RepairOutcome::EscalatedHuman
        ) {
            continue;
        }
        let chosen = report
            .attempts
            .iter()
            .find(|a| a.success)
            .or_else(|| report.attempts.last());
        let Some(chosen) = chosen else { continue };
        if chosen.strategy == RepairStrategy::HumanEscalate {
            continue;
        }
        if chosen.strategy == RepairStrategy::PlanningReplan
            && matches!(
                report.outcome,
                crate::repair::types::RepairOutcome::EscalatedPlanning
                    | crate::repair::types::RepairOutcome::EscalatedHuman
            )
        {
            continue;
        }
        groups
            .entry(error_signature(report))
            .or_default()
            .push(report);
    }
    let mut out = Vec::with_capacity(groups.len());
    for (group_idx, (_sig, group)) in groups.into_iter().enumerate() {
        let rep = pick_representative(&group);
        let outcome = run(LearnInput::new(rep).with_pattern_index(group_idx as u32));
        let Some(mut pattern) = outcome.pattern else {
            continue;
        };
        let mut seen: std::collections::HashSet<uuid::Uuid> =
            pattern.evidence.iter().cloned().collect();
        for r in &group {
            for id in [r.repair_id, r.triggered_by_report_id, r.source_diff_id] {
                if seen.insert(id) {
                    pattern.evidence.push(id);
                }
            }
        }
        let extra = (group.len() as f32 - 1.0).max(0.0);
        pattern.confidence =
            (pattern.confidence + extra * DEDUP_CONFIDENCE_BUMP).min(DEDUP_CONFIDENCE_CAP);
        out.push(pattern);
    }
    out.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));
    out
}

fn pick_representative<'a>(group: &[&'a RepairReport]) -> &'a RepairReport {
    group
        .iter()
        .find(|r| matches!(r.outcome, crate::repair::types::RepairOutcome::Applied))
        .or_else(|| {
            group
                .iter()
                .find(|r| matches!(r.outcome, crate::repair::types::RepairOutcome::Proposed))
        })
        .or_else(|| group.first())
        .expect("non-empty group")
        .to_owned()
}

/// Advance a `Draft` to `Candidate` (priority 30) iff `was_correct >=
///
/// `PROMOTE_THRESHOLD`. Any other lifecycle passes through untouched so
/// the Journal's `promote_rule` stays the only path for
/// `Candidate → Active`.
pub fn promote_draft(pattern: Pattern, was_correct: u32) -> Pattern {
    promote_draft_with_threshold(pattern, was_correct, PROMOTE_THRESHOLD)
}

/// Same as `promote_draft` with an explicit threshold (the CLI
/// `--min-correct` flag threads through here).
pub fn promote_draft_with_threshold(
    mut pattern: Pattern,
    was_correct: u32,
    threshold: u32,
) -> Pattern {
    if pattern.lifecycle != RuleLifecycle::Draft {
        return pattern;
    }
    if was_correct >= threshold {
        pattern.lifecycle = RuleLifecycle::Candidate;
        pattern.priority = RuleLifecycle::Candidate.default_priority();
    }
    pattern
}

/// Stale predicate: the rule fired and was vetoed strictly more often
/// than it ran correctly. Callers feed `PatternMetrics` aggregated from
/// M7 `pattern_runs` (or `learned_rules.was_correct`).
pub fn should_deprecate(metrics: &PatternMetrics) -> bool {
    metrics.was_blocked > metrics.was_correct
}

/// Retire a pattern: any lifecycle → `Deprecated`, priority 0.
/// Idempotent; mirrors `Journal::deprecate_rule` for in-memory use.
pub fn deprecate_stale(mut pattern: Pattern) -> Pattern {
    pattern.lifecycle = RuleLifecycle::Deprecated;
    pattern.priority = RuleLifecycle::Deprecated.default_priority();
    pattern
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coding::types::{Diff, FileEdit, Hunk};
    use crate::repair::types::{ErrorClass, RepairAttempt, RepairReport, RepairStrategy};
    use crate::validation::types::StageKind;

    fn diff_with(lines: Vec<String>) -> Diff {
        Diff {
            diff_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            mission_id: Uuid::new_v4(),
            step_id: "s1".into(),
            agent_id: Uuid::new_v4(),
            generated_at: "2026-07-22T12:00:00Z".into(),
            files: vec![FileEdit {
                path: "src/lib.rs".into(),
                hunks: vec![Hunk {
                    old_start: 1,
                    old_end: 1,
                    new_lines: lines,
                    rationale: "fixture".into(),
                }],
                is_new_file: false,
                is_delete: false,
            }],
            narrative: "test".into(),
            research_refs: vec![],
            risk_decision: None,
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        }
    }

    fn successful_attempt(
        strategy: RepairStrategy,
        root_cause: &str,
        diff_lines: Vec<String>,
    ) -> RepairAttempt {
        RepairAttempt {
            error_class: ErrorClass::SyntaxFormat,
            strategy,
            success: true,
            root_cause: root_cause.into(),
            proposed_diff: Some(Box::new(diff_with(diff_lines))),
        }
    }

    fn report_with(
        outcome: RepairOutcome,
        stage: StageKind,
        attempts: Vec<RepairAttempt>,
    ) -> RepairReport {
        RepairReport {
            repair_id: Uuid::new_v4(),
            triggered_by_report_id: Uuid::new_v4(),
            source_diff_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            mission_id: Uuid::new_v4(),
            generated_at: "2026-07-22T12:00:00Z".into(),
            attempts,
            outcome,
            triggering_stage: stage,
            successful_attempt: Some(0),
            final_diff: None,
            model_id: "heuristic-v0".into(),
            elapsed_ms: 0,
        }
    }

    #[test]
    fn applied_repair_emits_draft_pattern_with_0_7_confidence() {
        let attempt = successful_attempt(
            RepairStrategy::AutoFix,
            "lint rule `no_println` fired",
            vec!["let _ = 42;".into()],
        );
        let report = report_with(RepairOutcome::Applied, StageKind::LintFormat, vec![attempt]);
        let outcome = run(LearnInput::new(&report));
        let pattern = outcome.pattern.expect("applied -> draft");
        assert_eq!(pattern.lifecycle, RuleLifecycle::Draft);
        assert!((pattern.confidence - 0.7).abs() < 1e-6);
        assert_eq!(pattern.when.stage, StageKind::LintFormat);
        assert_eq!(pattern.when.pattern, "no_println");
        assert_eq!(pattern.then.strategy, RepairStrategy::AutoFix);
        assert!(!pattern.then.diff_hint.is_empty());
        assert_eq!(pattern.evidence.len(), 3);
        assert!(pattern.rule_id.starts_with("r-2026-07-22-"));
        assert_eq!(outcome.metrics.was_correct, 1);
        assert!(outcome.evidence_diff.is_some());
    }

    #[test]
    fn proposed_repair_emits_draft_with_0_5_confidence() {
        let attempt = successful_attempt(
            RepairStrategy::CodingAmendment,
            "type-check rejected the heuristic parse: `missing_fn_keyword`",
            vec!["fn foo() {}".into()],
        );
        let report = report_with(RepairOutcome::Proposed, StageKind::TypeCheck, vec![attempt]);
        let outcome = run(LearnInput::new(&report));
        let pattern = outcome.pattern.expect("proposed -> draft");
        assert!((pattern.confidence - 0.5).abs() < 1e-6);
        assert_eq!(pattern.when.pattern, "missing_fn_keyword");
        assert_eq!(pattern.then.strategy, RepairStrategy::CodingAmendment);
        assert_eq!(pattern.when.lang, "rust");
    }

    #[test]
    fn escalated_human_emits_no_pattern() {
        let attempt = RepairAttempt {
            error_class: ErrorClass::SyntaxFormat,
            strategy: RepairStrategy::HumanEscalate,
            success: false,
            root_cause: "budget exhausted".into(),
            proposed_diff: None,
        };
        let report = report_with(
            RepairOutcome::EscalatedHuman,
            StageKind::LintFormat,
            vec![attempt],
        );
        let outcome = run(LearnInput::new(&report));
        assert!(outcome.pattern.is_none());
        assert!(outcome.note.contains("§8"));
        assert_eq!(outcome.metrics.was_correct, 0);
    }

    #[test]
    fn empty_attempts_emits_no_pattern() {
        let report = report_with(RepairOutcome::Proposed, StageKind::LintFormat, vec![]);
        let outcome = run(LearnInput::new(&report));
        assert!(outcome.pattern.is_none());
        assert!(outcome.note.contains("no attempts"));
    }

    #[test]
    fn planning_replan_from_budget_exhaustion_is_not_learnable() {
        // Even if outcome is EscalatedPlanning, a PlanningReplan attempt
        // triggered by budget exhaustion must NOT become a draft rule.
        let attempt = RepairAttempt {
            error_class: ErrorClass::TypeCheck,
            strategy: RepairStrategy::PlanningReplan,
            success: false,
            root_cause: "3 consecutive failures on stage type_check (RFC 15 §4)".into(),
            proposed_diff: None,
        };
        let report = report_with(
            RepairOutcome::EscalatedPlanning,
            StageKind::TypeCheck,
            vec![attempt],
        );
        let outcome = run(LearnInput::new(&report));
        assert!(outcome.pattern.is_none());
        assert!(outcome.note.contains("budget-exhaustion"));
    }

    #[test]
    fn escalated_planning_with_coding_amendment_still_emits_weak_draft() {
        let attempt = successful_attempt(
            RepairStrategy::CodingAmendment,
            "TDD red stub still asserts false",
            vec!["assert!(true)".into()],
        );
        let report = report_with(
            RepairOutcome::EscalatedPlanning,
            StageKind::UnitTests,
            vec![attempt],
        );
        let outcome = run(LearnInput::new(&report));
        let pattern = outcome.pattern.expect("weak draft for reviewer");
        assert!((pattern.confidence - 0.3).abs() < 1e-6);
        assert_eq!(pattern.lifecycle, RuleLifecycle::Draft);
        assert_eq!(outcome.metrics.tests_passed, 0); // EscalatedPlanning, not Applied
    }

    #[test]
    fn pattern_index_increments_rule_id_suffix() {
        let attempt =
            successful_attempt(RepairStrategy::AutoFix, "lint rule `no_dbg` fired", vec![]);
        let report = report_with(RepairOutcome::Applied, StageKind::LintFormat, vec![attempt]);
        let o0 = run(LearnInput::new(&report).with_pattern_index(0));
        let o1 = run(LearnInput::new(&report).with_pattern_index(1));
        let p0 = o0.pattern.unwrap();
        let p1 = o1.pattern.unwrap();
        assert!(p0.rule_id.ends_with("-000"));
        assert!(p1.rule_id.ends_with("-001"));
    }

    #[test]
    fn long_diff_hint_is_truncated() {
        let long_line = format!("let _ = {};", "x".repeat(200));
        let attempt = successful_attempt(
            RepairStrategy::AutoFix,
            "lint rule `no_println` fired",
            vec![long_line],
        );
        let report = report_with(RepairOutcome::Applied, StageKind::LintFormat, vec![attempt]);
        let outcome = run(LearnInput::new(&report));
        let pattern = outcome.pattern.unwrap();
        assert!(pattern.then.diff_hint.len() < 200);
        assert!(pattern.then.diff_hint.contains('…'));
    }

    fn applied_report(rule: &str, stage: StageKind) -> RepairReport {
        let attempt = successful_attempt(
            RepairStrategy::AutoFix,
            &format!("lint rule `{rule}` fired"),
            vec!["let _ = 42;".into()],
        );
        report_with(RepairOutcome::Applied, stage, vec![attempt])
    }

    #[test]
    fn error_signature_normalises_rule_tag() {
        let a = applied_report("no_println", StageKind::LintFormat);
        let b = applied_report("  NO_PRINTLN ", StageKind::LintFormat);
        assert_eq!(error_signature(&a), error_signature(&b));
        assert_eq!(error_signature(&a), "lint_format|syntax_format|no_println");
    }

    #[test]
    fn reflect_dedups_repeated_error_into_one_pattern() {
        let a = applied_report("no_println", StageKind::LintFormat);
        let b = applied_report("no_println", StageKind::LintFormat);
        let c = applied_report("no_dbg", StageKind::LintFormat);
        let patterns = reflect(&[a, b, c]);
        assert_eq!(patterns.len(), 2);
        let dup = patterns
            .iter()
            .find(|p| p.when.pattern == "no_println")
            .expect("dedup group");
        assert_eq!(dup.evidence.len(), 6);
        assert!((dup.confidence - 0.75).abs() < 1e-6);
        let single = patterns
            .iter()
            .find(|p| p.when.pattern == "no_dbg")
            .expect("single group");
        assert!((single.confidence - 0.7).abs() < 1e-6);
    }

    #[test]
    fn reflect_is_deterministic_across_input_order() {
        let a = applied_report("no_println", StageKind::LintFormat);
        let b = applied_report("no_dbg", StageKind::LintFormat);
        let fwd = reflect(&[a.clone(), b.clone()]);
        let rev = reflect(&[b, a]);
        assert_eq!(fwd.len(), rev.len());
        for (l, r) in fwd.iter().zip(rev.iter()) {
            assert_eq!(l.when, r.when);
            assert_eq!(l.then, r.then);
            assert_eq!(l.rule_id, r.rule_id);
        }
    }

    #[test]
    fn reflect_skips_non_learnable_reports() {
        let empty = report_with(RepairOutcome::Proposed, StageKind::LintFormat, vec![]);
        let human_attempt = RepairAttempt {
            error_class: ErrorClass::SyntaxFormat,
            strategy: RepairStrategy::HumanEscalate,
            success: false,
            root_cause: "budget exhausted".into(),
            proposed_diff: None,
        };
        let human = report_with(
            RepairOutcome::EscalatedHuman,
            StageKind::LintFormat,
            vec![human_attempt],
        );
        assert!(reflect(&[empty, human]).is_empty());
    }

    #[test]
    fn promote_draft_requires_threshold() {
        let report = applied_report("no_println", StageKind::LintFormat);
        let outcome = run(LearnInput::new(&report));
        let draft = outcome.pattern.unwrap();
        assert_eq!(draft.lifecycle, RuleLifecycle::Draft);
        let kept = promote_draft_with_threshold(draft.clone(), 1, 2);
        assert_eq!(kept.lifecycle, RuleLifecycle::Draft);
        assert_eq!(kept.priority, 0);
        let promoted = promote_draft_with_threshold(draft, 2, 2);
        assert_eq!(promoted.lifecycle, RuleLifecycle::Candidate);
        assert_eq!(promoted.priority, 30);
    }

    #[test]
    fn promote_draft_ignores_non_draft() {
        let report = applied_report("no_println", StageKind::LintFormat);
        let outcome = run(LearnInput::new(&report));
        let mut active = outcome.pattern.unwrap();
        active.lifecycle = RuleLifecycle::Active;
        active.priority = RuleLifecycle::Active.default_priority();
        let out = promote_draft(active.clone(), 99);
        assert_eq!(out.lifecycle, RuleLifecycle::Active);
        assert_eq!(out.priority, active.priority);
    }

    #[test]
    fn stale_predicate_and_deprecate_transition() {
        let blocked = PatternMetrics {
            was_correct: 1,
            was_blocked: 3,
            tests_passed: 0,
            approved_by_user: 0,
        };
        assert!(should_deprecate(&blocked));
        let healthy = PatternMetrics {
            was_correct: 3,
            was_blocked: 1,
            ..PatternMetrics::default()
        };
        assert!(!should_deprecate(&healthy));
        let report = applied_report("no_println", StageKind::LintFormat);
        let outcome = run(LearnInput::new(&report));
        let retired = deprecate_stale(outcome.pattern.unwrap());
        assert!(retired.is_deprecated());
        assert!(!retired.is_consultable());
        assert_eq!(retired.priority, 0);
    }
}
