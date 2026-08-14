// Atlas OS — Step 4: Search Similar Missions (RFC 23 §2, step 4).
//
// Phase 1: placeholder. The real sqlite-vec cosine lookup lives behind a
// trait (`SimilarMissionsLookup`) that the Journal will implement once the
// `fastembed` feature gates the embedding producer. The runner calls
// `lookup` with the parsed intent — Fase 1.5 will return real neighbours,
// Fase 1 returns an empty list and a tracing note so downstream steps see
// a well-formed `similar_missions: vec![]` in the verdict.

use crate::prompt::steps::parse::ParsedIntent;
use crate::prompt::types::SimilarMission;

pub trait SimilarMissionsLookup {
    fn similar(&self, intent: &ParsedIntent, k: usize) -> anyhow::Result<Vec<SimilarMission>>;
}

/// Phase 1 placeholder: always returns an empty list. Kept as a real
/// function so the runner's `similar::lookup` reference forces an import,
/// which in turn forces Fase 1.5 to update this file (not the runner)
/// when the sqlite-vec lookup lands.
pub fn lookup(_: &ParsedIntent) -> Vec<SimilarMission> {
    tracing::debug!("similar::lookup called (Phase 1 stub — returns empty)");
    Vec::new()
}
