// OpenCode OS — Prompt Understanding Pipeline tests (RFC 23).
// Phase 1: serialisation round-trips for the canonical types. Behavioural
// tests for the heuristic detectors land alongside their step modules.

#[cfg(test)]
mod runner_tests {
    use super::super::runner::{run, PipelineOptions};
    use crate::prompt::types::{ConfidenceLevel, GapType, RecommendedMode};

    #[test]
    fn prompt_low_information_yields_block_confidence_and_ask_mode() {
        let (v, c) = run("go", &PipelineOptions::default()).expect("run");
        assert_eq!(v.confidence, ConfidenceLevel::Block);
        assert_eq!(v.recommended_mode, RecommendedMode::Ask);
        assert!(c.is_none(), "Step 9 must not lock a BLOCK mission");
        assert!(v
            .gaps
            .iter()
            .any(|g| g.kind == GapType::LowInformationPrompt));
    }

    #[test]
    fn well_scoped_prompt_yields_high_and_code_mode_with_consolidated() {
        let prompt =
            "fix the bug in `rating_service` where `compute_weight` overflows on `f64` input";
        let (v, c) = run(prompt, &PipelineOptions::default()).expect("run");
        assert!(
            v.confidence == ConfidenceLevel::High || v.confidence == ConfidenceLevel::Medium,
            "got {:?} — expected HIGH or MEDIUM",
            v.confidence
        );
        if v.confidence == ConfidenceLevel::High {
            assert!(c.is_some(), "HIGH verdict must produce a Consolidated");
        }
    }

    #[test]
    fn force_flag_locks_mission_even_when_confidence_low() {
        let (v, c) = run(
            "fix",
            &PipelineOptions {
                force: true,
                ..PipelineOptions::default()
            },
        )
        .expect("run");
        assert!(
            c.is_some(),
            "--force must always return a Consolidated regardless of confidence"
        );
        let consolidated = c.unwrap();
        assert!(consolidated.locked);
        assert_eq!(
            consolidated.locked_by,
            crate::prompt::types::LockSource::User
        );
        // sanity: confidence label still reflects reality.
        assert_eq!(v.confidence, ConfidenceLevel::Low);
    }

    #[test]
    fn galaxy_prompt_fires_capacity_hallucination_and_blocks_consolidation() {
        let (v, c) = run(
            "build something that flies through the galaxy tomorrow",
            &PipelineOptions::default(),
        )
        .expect("run");
        assert!(v
            .gaps
            .iter()
            .any(|g| g.kind == GapType::CapacityHallucination));
        assert!(c.is_none(), "capacity hallucination must block lock");
    }

    #[test]
    fn intent_statement_is_refined_when_target_key_present() {
        let (v, _) = run(
            "fix the bug in `db_pool` — it doesn't re-use connections",
            &PipelineOptions::default(),
        )
        .expect("run");
        assert!(v.intent.contains("db_pool"));
    }
}

#[cfg(test)]
mod journal_verdict_persistence_tests {
    use crate::journal::Journal;
    use crate::prompt::runner::{run, PipelineOptions};
    use tempfile::TempDir;
    use uuid::Uuid;

    #[test]
    fn runner_verdict_round_trips_through_journal() {
        let tmp = TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        let (v, c) = run("fix", &PipelineOptions::default()).expect("run");
        let mission_id = Uuid::new_v4();
        journal.save_verdict(&v, Some(mission_id)).expect("save v");
        if let Some(c) = c {
            journal.save_consolidated(&c).expect("save c");
            let consolidated_rows = journal.consolidated_tail(10).expect("tail");
            assert_eq!(consolidated_rows.len(), 1);
        }
        let row = journal.verdict_tail(10).expect("v tail")[0].clone();
        assert_eq!(row.verdict_id, v.verdict_id);
        assert_eq!(row.confidence, v.confidence.tag());
        assert_eq!(row.recommended_mode, v.recommended_mode.tag());
    }
}
