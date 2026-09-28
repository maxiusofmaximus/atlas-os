// Atlas OS — Learning Engine (RFC 16). Derives reusable
// `Pattern`s from `RepairReport`s so the kernel never repeats the same
// mistake twice (RFC 16 §2 Reflection Loop). Phase 1 is heuristic-only:
// the runner consumes a `RepairReport` and emits one `LearnOutcome`
// (which MAY carry a `Draft` pattern). Promotion to `Candidate` /
// `Active` and the Skill Compressor (RFC 16 §5) land with Phase 2
// alongside the vector store and the Model Orchestrator (RFC 04).

pub mod compaction;
pub mod compress;
pub mod rules;
pub mod runner;
pub mod share;
pub mod types;

pub use compaction::{
    needs_compaction, needs_compaction_with_threshold, summarize, CompactionSummary,
    COMPACTION_HEADLINES, COMPACTION_KEEP_LAST, COMPACTION_MODEL_ID, COMPACTION_THRESHOLD,
};
pub use compress::{
    apply_proposal, apply_proposals, collect_skills, find_compress_proposals, jaccard,
    merge_manifests, skill_tokens, AppliedCompress, CompressProposal, DEFAULT_COMPRESS_THRESHOLD,
};
pub use rules::{load_rule_file, pattern_from_yaml, pattern_to_yaml, write_rule_file};
pub use runner::{
    deprecate_stale, error_signature, promote_draft, promote_draft_with_threshold, reflect, run,
    should_deprecate, DEDUP_CONFIDENCE_BUMP, DEDUP_CONFIDENCE_CAP, PROMOTE_THRESHOLD,
};
pub use share::{
    export_rules, file_checksum, import_rules, read_file_sidecar, verify_file_against_sidecar,
    write_file_sidecar, ExportReport, ImportReport, SharedRule, SHARED_IMPORT_MODEL,
    SHARE_FORBIDDEN, SHARE_VERSION,
};
pub use types::{
    LearnInput, LearnOutcome, Pattern, PatternMetrics, RuleLifecycle, RuleThen, RuleWhen,
};
