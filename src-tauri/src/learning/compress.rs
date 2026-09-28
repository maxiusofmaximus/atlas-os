// Atlas OS — Skill Compressor (RFC 06 §5, RFC 32 Phase 5 sub-fase 5.2).
//
// Detects redundant skills by deterministic token-overlap (Jaccard) over
// `description + summary + domain/language/framework`, proposes merges for
// pairs above threshold, and — only under `--apply` — writes the fused
// `skill.toml` plus a `DEPRECATED` marker on the absorbed profile skill.
//
// No embeddings required (fastembed feature-gated follow-up, RFC 32 §C).
// The filesystem (`<profile>/skills/<id>/skill.toml`) is the source of
// truth; the Journal `skill_manifests` mirror is best-effort
// (`(skill_id, version)` PK is `DO NOTHING`, so re-saving an unchanged
// version is a no-op). Bundled skills are compiled in and can never be
// deprecated on disk: absorbing one records a skip reason instead.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::skills::SkillManifest;

/// Default similarity threshold for `atlas skills compress` (RFC 32 B 5.2).
pub const DEFAULT_COMPRESS_THRESHOLD: f64 = 0.7;

/// One dry-run merge proposal: `merge` is absorbed into `keep`.
/// `keep` is the higher-priority skill (ties broken by smaller id) so the
/// surviving manifest preserves the strongest routing signal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompressProposal {
    pub keep: String,
    pub merge: String,
    pub similarity: f64,
}

/// Outcome of applying one proposal under `--apply`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppliedCompress {
    pub keep: String,
    pub merge: String,
    pub similarity: f64,
    pub kept_path: PathBuf,
    pub deprecated_path: Option<PathBuf>,
    pub skipped_reason: Option<String>,
}

/// Deterministic token set for one skill: lowercase alphanumeric tokens
/// from `description` + `summary`, plus the `domain` / `language` /
/// `framework` tags when set.
pub fn skill_tokens(skill: &SkillManifest) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut feed = |text: &str| {
        for token in text
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
        {
            out.insert(token.to_string());
        }
    };
    feed(&skill.description);
    if let Some(summary) = &skill.summary {
        feed(summary);
    }
    for tag in [&skill.domain, &skill.language, &skill.framework]
        .into_iter()
        .flatten()
    {
        feed(tag);
    }
    out
}

/// Jaccard similarity over `skill_tokens`: `|A∩B| / |A∪B|`.
/// Two token-less skills compare as identical (1.0); one token-less
/// skill against a non-empty one compares as disjoint (0.0).
pub fn jaccard(a: &SkillManifest, b: &SkillManifest) -> f64 {
    let ta = skill_tokens(a);
    let tb = skill_tokens(b);
    if ta.is_empty() && tb.is_empty() {
        return 1.0;
    }
    let inter = ta.intersection(&tb).count() as f64;
    let union = ta.union(&tb).count() as f64;
    if union == 0.0 {
        1.0
    } else {
        inter / union
    }
}

/// Pairwise proposals for every skill pair with `similarity >= threshold`.
/// Pure and deterministic: pairs visited in id order, output sorted by
/// similarity descending, then `keep` / `merge` ascending.
pub fn find_compress_proposals(skills: &[SkillManifest], threshold: f64) -> Vec<CompressProposal> {
    let mut ordered: Vec<&SkillManifest> = skills.iter().collect();
    ordered.sort_by(|a, b| a.id.cmp(&b.id));
    let mut out = Vec::new();
    for (i, a) in ordered.iter().enumerate() {
        for b in ordered.iter().skip(i + 1) {
            let similarity = jaccard(a, b);
            if similarity >= threshold {
                let b_wins = b.priority > a.priority || (b.priority == a.priority && b.id < a.id);
                let (keep, merge) = if b_wins {
                    (&b.id, &a.id)
                } else {
                    (&a.id, &b.id)
                };
                out.push(CompressProposal {
                    keep: keep.clone(),
                    merge: merge.clone(),
                    similarity,
                });
            }
        }
    }
    out.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.keep.cmp(&b.keep))
            .then(a.merge.cmp(&b.merge))
    });
    out
}

fn union_sorted(mut base: Vec<String>, extra: &[String], exclude: &[&str]) -> Vec<String> {
    base.extend(extra.iter().cloned());
    base.sort();
    base.dedup();
    base.retain(|s| !exclude.contains(&s.as_str()));
    base
}

fn pick_or(keep: &Option<String>, absorb: &Option<String>) -> Option<String> {
    keep.clone().or_else(|| absorb.clone())
}

/// Fuse `absorb` into `keep` (RFC 06 §5: the hybrid inherits dependencies,
/// compatible models and a summarised description; redundancy is auditable
/// via the surviving `description / summary` text). Pure: touches no disk.
/// The fused manifest ships `verified = false` so the user-approved diff
/// (RFC 06 §5) gates it before it routes automatically.
pub fn merge_manifests(keep: &SkillManifest, absorb: &SkillManifest) -> SkillManifest {
    let exclude = [keep.id.as_str(), absorb.id.as_str()];
    let description = if keep.description == absorb.description {
        keep.description.clone()
    } else {
        format!("{} / {}", keep.description, absorb.description)
    };
    let summary = match (&keep.summary, &absorb.summary) {
        (Some(k), Some(a)) if k == a => Some(k.clone()),
        (Some(k), Some(a)) => Some(format!("{k} / {a}")),
        (Some(k), None) => Some(k.clone()),
        (None, Some(a)) => Some(a.clone()),
        (None, None) => None,
    };
    SkillManifest {
        id: keep.id.clone(),
        version: keep.version.clone(),
        description,
        engine: if keep.engine != crate::skills::Engine::Unspecified {
            keep.engine
        } else {
            absorb.engine
        },
        priority: keep.priority.max(absorb.priority),
        domain: pick_or(&keep.domain, &absorb.domain),
        language: pick_or(&keep.language, &absorb.language),
        framework: pick_or(&keep.framework, &absorb.framework),
        confidence: keep.confidence.max(absorb.confidence),
        estimated_tokens: keep.estimated_tokens.max(absorb.estimated_tokens),
        estimated_time: keep.estimated_time.max(absorb.estimated_time),
        dependencies: union_sorted(keep.dependencies.clone(), &absorb.dependencies, &exclude),
        compatible_models: union_sorted(
            keep.compatible_models.clone(),
            &absorb.compatible_models,
            &[],
        ),
        vector_embedding: keep.vector_embedding.clone(),
        summary,
        conflicts: union_sorted(keep.conflicts.clone(), &absorb.conflicts, &exclude),
        auto_generated: keep.auto_generated && absorb.auto_generated,
        verified: false,
        capabilities: union_sorted(keep.capabilities.clone(), &absorb.capabilities, &[]),
        requires_sandbox: keep.requires_sandbox || absorb.requires_sandbox,
        author: pick_or(&keep.author, &absorb.author),
        license: pick_or(&keep.license, &absorb.license),
        home: keep.home.clone(),
        remixed_from: keep.remixed_from.clone(),
    }
}

/// Load every candidate skill: bundled catalog first, then the profile
/// `<profile>/skills/` dir via `SkillGraph::scan_directory` (RFC 06 §6).
/// Profile manifests carry `home`; bundled ones keep whatever the TOML
/// declares (usually unset).
pub fn collect_skills(skills_dir: &Path) -> anyhow::Result<Vec<SkillManifest>> {
    let mut graph = crate::skills::SkillGraph::new();
    if let Err(e) = crate::skills::bundled::bundled_manifests().map(|manifests| {
        for m in manifests {
            graph.register(m);
        }
    }) {
        tracing::warn!(error = %e, "bundled skill catalog failed to parse");
    }
    let _ = graph.scan_directory(skills_dir);
    let mut out: Vec<SkillManifest> = graph.iter().cloned().collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

/// Apply one proposal: write the fused manifest to
/// `<skills_dir>/<keep>/skill.toml` (created when the keep side is a
/// bundled skill), mark `<skills_dir>/<merge>/DEPRECATED` when the
/// absorbed side lives on disk, and best-effort mirror the fused
/// manifest into the Journal.
pub fn apply_proposal(
    proposal: &CompressProposal,
    all: &[SkillManifest],
    skills_dir: &Path,
    journal: Option<&crate::journal::Journal>,
) -> anyhow::Result<AppliedCompress> {
    let by_id: HashMap<&str, &SkillManifest> = all.iter().map(|m| (m.id.as_str(), m)).collect();
    let keep = by_id
        .get(proposal.keep.as_str())
        .ok_or_else(|| anyhow::anyhow!("unknown skill `{}`", proposal.keep))?;
    let absorb = by_id
        .get(proposal.merge.as_str())
        .ok_or_else(|| anyhow::anyhow!("unknown skill `{}`", proposal.merge))?;
    let merged = merge_manifests(keep, absorb);

    let keep_dir = skills_dir.join(&proposal.keep);
    std::fs::create_dir_all(&keep_dir)?;
    let kept_path = keep_dir.join("skill.toml");
    std::fs::write(&kept_path, toml::to_string(&merged)?)?;

    let merge_dir = skills_dir.join(&proposal.merge);
    let deprecated_path;
    let skipped_reason;
    if merge_dir.is_dir() {
        let marker = merge_dir.join("DEPRECATED");
        let already = marker.exists();
        std::fs::write(
            &marker,
            format!(
                "merged_into = \"{}\"\nsimilarity = {:.4}\n",
                proposal.keep, proposal.similarity
            ),
        )?;
        deprecated_path = Some(marker);
        skipped_reason = if already {
            Some("already deprecated — merge re-written".to_string())
        } else {
            None
        };
    } else {
        deprecated_path = None;
        skipped_reason = Some(
            "absorbed skill is bundled (compiled in) — copy it to the profile skills dir first to deprecate it".to_string(),
        );
    }

    if let Some(j) = journal {
        j.save_skill(&merged)?;
    }

    Ok(AppliedCompress {
        keep: proposal.keep.clone(),
        merge: proposal.merge.clone(),
        similarity: proposal.similarity,
        kept_path,
        deprecated_path,
        skipped_reason,
    })
}

/// Apply every proposal in order, skipping ones whose `merge` side was
/// already absorbed by an earlier proposal in the same batch.
pub fn apply_proposals(
    proposals: &[CompressProposal],
    all: &[SkillManifest],
    skills_dir: &Path,
    journal: Option<&crate::journal::Journal>,
) -> anyhow::Result<Vec<AppliedCompress>> {
    let mut absorbed: HashSet<&str> = HashSet::new();
    let mut out = Vec::with_capacity(proposals.len());
    for proposal in proposals {
        if absorbed.contains(proposal.merge.as_str()) || absorbed.contains(proposal.keep.as_str()) {
            out.push(AppliedCompress {
                keep: proposal.keep.clone(),
                merge: proposal.merge.clone(),
                similarity: proposal.similarity,
                kept_path: skills_dir.join(&proposal.keep).join("skill.toml"),
                deprecated_path: None,
                skipped_reason: Some(
                    "skipped — id already absorbed by an earlier proposal in this batch"
                        .to_string(),
                ),
            });
            continue;
        }
        let applied = apply_proposal(proposal, all, skills_dir, journal)?;
        if applied.deprecated_path.is_some() {
            absorbed.insert(proposal.merge.as_str());
        }
        out.push(applied);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::Engine;

    fn skill(id: &str, priority: u32, description: &str) -> SkillManifest {
        SkillManifest {
            id: id.into(),
            version: "1.0.0".into(),
            description: description.into(),
            engine: Engine::Coding,
            priority,
            ..Default::default()
        }
    }

    #[test]
    fn jaccard_is_deterministic_and_bounded() {
        let a = skill("a", 50, "Fix broken Rust code with compiler errors");
        let b = skill("b", 50, "Fix broken Rust code with compiler errors");
        assert!((jaccard(&a, &b) - 1.0).abs() < 1e-9);
        assert!((jaccard(&a, &a) - 1.0).abs() < 1e-9);

        let c = skill("c", 50, "Bake sourdough bread at home");
        assert!((jaccard(&a, &c) - 0.0).abs() < 1e-9);

        let d = skill("d", 50, "Fix broken Rust code");
        let partial = jaccard(&a, &d);
        assert!(partial > 0.0 && partial < 1.0);
        assert!((jaccard(&a, &d) - partial).abs() < 1e-9);

        let empty_a = skill("e1", 50, "");
        let empty_b = skill("e2", 50, "");
        assert!((jaccard(&empty_a, &empty_b) - 1.0).abs() < 1e-9);
        assert!((jaccard(&empty_a, &a) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn threshold_filters_pairs() {
        let skills = vec![
            skill("fix-a", 50, "Fix broken Rust code with compiler errors"),
            skill("fix-b", 50, "Fix broken Rust code with compiler errors"),
            skill("bread", 50, "Bake sourdough bread at home"),
        ];
        let proposals = find_compress_proposals(&skills, 0.7);
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].keep, "fix-a");
        assert_eq!(proposals[0].merge, "fix-b");
        assert!((proposals[0].similarity - 1.0).abs() < 1e-9);

        assert!(find_compress_proposals(&skills, 1.01).is_empty());

        let all = find_compress_proposals(&skills, 0.0);
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn keep_wins_by_priority_then_id() {
        let low = skill("aaa", 10, "Fix broken Rust code with compiler errors");
        let high = skill("zzz", 90, "Fix broken Rust code with compiler errors");
        let proposals = find_compress_proposals(&[low, high], 0.7);
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].keep, "zzz");
        assert_eq!(proposals[0].merge, "aaa");
    }

    #[test]
    fn merge_inherits_union_fields() {
        let mut keep = skill("keep", 80, "Fix Rust code");
        keep.dependencies = vec!["dep-a".into(), "keep".into()];
        keep.compatible_models = vec!["model-a".into()];
        keep.conflicts = vec!["vue".into()];
        keep.capabilities = vec!["fmt".into()];
        keep.summary = Some("Rust fixer".into());
        keep.verified = true;
        let mut absorb = skill("absorb", 90, "Fix Rust code and format it");
        absorb.dependencies = vec!["dep-b".into(), "dep-a".into()];
        absorb.compatible_models = vec!["model-b".into(), "model-a".into()];
        absorb.conflicts = vec!["svelte".into(), "keep".into()];
        absorb.capabilities = vec!["lint".into()];
        absorb.summary = Some("Rust formatter".into());
        absorb.verified = true;
        absorb.requires_sandbox = true;

        let merged = merge_manifests(&keep, &absorb);
        assert_eq!(merged.id, "keep");
        assert_eq!(merged.priority, 90);
        assert_eq!(merged.dependencies, vec!["dep-a", "dep-b"]);
        assert_eq!(merged.compatible_models, vec!["model-a", "model-b"]);
        assert_eq!(merged.conflicts, vec!["svelte", "vue"]);
        assert_eq!(merged.capabilities, vec!["fmt", "lint"]);
        assert_eq!(merged.summary, Some("Rust fixer / Rust formatter".into()));
        assert!(!merged.verified);
        assert!(merged.requires_sandbox);
        assert!(merged.description.contains("Fix Rust code"));
    }

    #[test]
    fn dry_run_does_not_touch_disk() {
        let dir = tempfile::TempDir::new().unwrap();
        let skills = vec![
            skill("fix-a", 50, "Fix broken Rust code with compiler errors"),
            skill("fix-b", 50, "Fix broken Rust code with compiler errors"),
        ];
        let proposals = find_compress_proposals(&skills, 0.7);
        assert_eq!(proposals.len(), 1);
        let merged = merge_manifests(&skills[0], &skills[1]);
        assert_eq!(merged.id, "fix-a");
        assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
    }

    fn write_profile_skill(dir: &Path, manifest: &SkillManifest) {
        let skill_dir = dir.join(&manifest.id);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("skill.toml"),
            toml::to_string(manifest).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn apply_writes_merged_and_deprecates_absorbed() {
        let dir = tempfile::TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        let a = skill("fix-a", 50, "Fix broken Rust code with compiler errors");
        let b = skill("fix-b", 50, "Fix broken Rust code with compiler errors");
        write_profile_skill(&skills_dir, &a);
        write_profile_skill(&skills_dir, &b);

        let all = vec![a, b];
        let proposals = find_compress_proposals(&all, 0.7);
        assert_eq!(proposals.len(), 1);
        let applied = apply_proposal(&proposals[0], &all, &skills_dir, None).unwrap();
        assert_eq!(
            applied.kept_path,
            skills_dir.join("fix-a").join("skill.toml")
        );
        assert!(applied.skipped_reason.is_none());

        let raw = std::fs::read_to_string(&applied.kept_path).unwrap();
        let back: SkillManifest = toml::from_str(&raw).unwrap();
        assert_eq!(back.id, "fix-a");
        assert!(!back.verified);

        let marker = applied.deprecated_path.expect("deprecated marker");
        assert_eq!(marker, skills_dir.join("fix-b").join("DEPRECATED"));
        let note = std::fs::read_to_string(&marker).unwrap();
        assert!(note.contains("fix-a"));

        let again = apply_proposal(&proposals[0], &all, &skills_dir, None).unwrap();
        assert!(again
            .skipped_reason
            .as_deref()
            .unwrap_or("")
            .contains("already deprecated"));
    }

    #[test]
    fn apply_skips_bundled_absorb_without_deprecated_marker() {
        let dir = tempfile::TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        let bundled = crate::skills::bundled::bundled_manifests().unwrap();
        let pair = vec![bundled[0].clone(), bundled[1].clone()];
        let proposal = CompressProposal {
            keep: pair[0].id.clone(),
            merge: pair[1].id.clone(),
            similarity: 1.0,
        };
        let applied = apply_proposal(&proposal, &pair, &skills_dir, None).unwrap();
        assert!(applied.kept_path.exists());
        assert!(applied.deprecated_path.is_none());
        assert!(applied.skipped_reason.is_some());
    }
}
