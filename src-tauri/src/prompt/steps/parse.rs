// Atlas OS — Step 2: Parse Intent (RFC 23 §2, step 2).
//
// Heuristic implementation: no LLM. Native code only.
//
// What it does:
//   - Splits the normalised prompt into tokens / phrase chunks.
//   - Surfaces a fixed set of *implicit_signals* tokens that downstream
//     detectors (step 3) use to fire gap rules (RFC 23 §1 table).
//   - Extracts *keys*: the noun-ish words quoted in backticks, the leading
//     identifier-like tokens, words following action verbs. These are the
//     candidates Journal embedding similarity will be computed against in
//     step 4.
//   - Extracts *named entities* of types: crate, file path, URL, GitHub
//     `org/repo`. The `in_kb` flag is `false` at parse time — step 6
//     (Architecture Memory lookup) flips it to `true` when the entity is
//     observed in the live Project Map.
//   - guesses *domain* by keyword intersection (frontend / backend / devops /
//     data / ml / docs).
//   - guesses *desired_action* by leading verb.
//   - guesses *scope* by quantifier tokens (file / module / feature / project).
//   - emits 1..=3 *intent_hypotheses* ranked by a deterministic
//     feasibility_score (no LLM yet).
//
// All heuristics here are deliberately keyword/regex-driven so the tests are
// reproducible. The cost isrecall: complex prompts may yield `Unknown`
// domain and only the loosest hypothesis — that's intentional, the
// downstream Self-Refine step (Fase 1.7) collapses weak intent hypotheses.

use crate::prompt::types::{DesiredAction, Domain, IntentHypothesis, NamedEntity, Scope};

#[derive(Clone, Debug)]
pub struct ParsedIntent {
    pub intent: String,
    pub intent_hypotheses: Vec<IntentHypothesis>,
    pub keys: Vec<String>,
    pub named_entities: Vec<NamedEntity>,
    pub domain: Domain,
    pub technology: Vec<String>,
    pub desired_action: DesiredAction,
    pub scope: Scope,
    pub implicit_signals: Vec<String>,
}

pub fn run(normalised: &str) -> ParsedIntent {
    let lower = normalised.to_lowercase();
    let tokens: Vec<&str> = normalised.split_whitespace().collect();

    let mut keys: Vec<String> = Vec::new();
    let mut named_entities: Vec<NamedEntity> = Vec::new();
    let mut implicit_signals: Vec<String> = Vec::new();
    let mut technology: Vec<String> = Vec::new();

    extract_backtick_keys(normalised, &mut keys);
    extract_paths_and_urls(normalised, &mut named_entities);
    extract_tech_keywords(&lower, &mut technology, &mut named_entities);
    detect_implicit_signals(&lower, &mut implicit_signals);

    let desired_action = guess_desired_action(&lower);
    let domain = guess_domain(&lower);
    let scope = guess_scope(&lower, tokens.len());

    let intent = build_intent(normalised, desired_action, &keys);
    let intent_hypotheses = build_hypotheses(&intent, &keys, &named_entities, &implicit_signals);

    ParsedIntent {
        intent,
        intent_hypotheses,
        keys,
        named_entities,
        domain,
        technology,
        desired_action,
        scope,
        implicit_signals,
    }
}

fn extract_backtick_keys(normalised: &str, keys: &mut Vec<String>) {
    let bytes = normalised.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let start = i + 1;
            let mut j = start;
            while j < bytes.len() && bytes[j] != b'`' {
                j += 1;
            }
            if j < bytes.len() {
                if let Ok(s) = std::str::from_utf8(&bytes[start..j]) {
                    if !s.is_empty() {
                        keys.push(s.to_string());
                    }
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    keys.sort();
    keys.dedup();
}

fn extract_paths_and_urls(normalised: &str, out: &mut Vec<NamedEntity>) {
    for tok in normalised.split_whitespace() {
        let t = tok.trim_matches(|c: char| c == ',' || c == '.');
        if t.starts_with("http://") || t.starts_with("https://") {
            out.push(NamedEntity {
                text: t.to_string(),
                r#type: "url".into(),
                in_kb: false,
            });
        } else if parse_github_ref(t).is_some() && !looks_like_file_path(t) {
            // RFC 23 §2: `org/repo` GitHub references must be classified
            // before the file_path fallback — `vercel/next.js` would
            // otherwise be mis-detectado as a local file because
            // `is_pathlike` only checks for a separator + length. We still
            // cede to file_path when the trailing segment is a known file
            // extension (`src/lib.rs` is a path, not a repo).
            out.push(NamedEntity {
                text: t.to_string(),
                r#type: "github_ref".into(),
                in_kb: false,
            });
        } else if is_pathlike(t) {
            out.push(NamedEntity {
                text: t.to_string(),
                r#type: "file_path".into(),
                in_kb: false,
            });
        }
    }
    out.sort_by(|a, b| a.text.cmp(&b.text));
    out.dedup_by(|a, b| a.text == b.text);
}

/// Heuristic to split a `org/repo` GitHub reference from a `dir/file.ext`
/// local path. Returns `true` when the token looks like a source-tree path
/// rather than a GitHub `org/repo` reference.
///
/// Rules (in priority order):
///   1. multiple `/` segments → unambiguously a path
///      (`src/lib/utf8.rs` is not `org/repo`).
///   2. the first segment is a reserved path prefix (`src`, `lib`, `tests`,
///      `bin`, `cmd`, `app`, `crate`, `docs`, `pkg`, `internal`, `examples`)
///      → treat as path even with a single slash.
///   3. otherwise: if the trailing segment ends with a known file extension
///      *and* the head segment is not a known GitHub org handle pattern
///      (alnum, hyphen, 2..=39 chars, not all-numeric) → treat as path.
///
/// `vercel/next.js` is the canonical case that passes rule 3: `vercel`
/// matches the GitHub org pattern and `.js` doubles as a build artefact
/// extension; we still classify it as a github_ref because the head looks
/// like an org, not a reserved directory.
fn looks_like_file_path(s: &str) -> bool {
    if !s.contains('/') && !s.contains('\\') {
        return false;
    }
    let norm = s.replace('\\', "/");
    let segs: Vec<&str> = norm.split('/').filter(|s| !s.is_empty()).collect();
    if segs.len() != 2 {
        return true;
    }
    const RESERVED_DIRS: &[&str] = &[
        "src",
        "lib",
        "libs",
        "tests",
        "test",
        "bin",
        "cmd",
        "app",
        "apps",
        "crate",
        "crates",
        "docs",
        "doc",
        "pkg",
        "packages",
        "internal",
        "examples",
        "example",
        "scripts",
        "tools",
        "vendor",
        "build",
        "dist",
        "target",
        "node_modules",
        ".cargo",
        ".config",
    ];
    if RESERVED_DIRS.contains(&segs[0]) {
        return true;
    }
    // Head must look like a GitHub org handle: alnum + hyphen, 2..=39 chars,
    // not all-numeric. Otherwise it's a path-like directory.
    let head = segs[0];
    let head_is_org = (2..=39).contains(&head.len())
        && head.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !head.chars().all(|c| c.is_ascii_digit());
    const EXT: &[&str] = &[
        ".rs", ".ts", ".tsx", ".js", ".jsx", ".svelte", ".json", ".toml", ".md", ".py", ".go",
        ".java", ".kt", ".c", ".h", ".cpp", ".cc", ".hpp", ".css", ".html", ".vue", ".astro",
        ".yaml", ".yml", ".lock", ".txt", ".sh", ".ps1", ".bat",
    ];
    let last = segs[1].to_lowercase();
    let has_ext = EXT.iter().any(|e| last.ends_with(e));
    // If the head looks like an org AND last segment has a file extension,
    // we cannot disambiguate structurally. Classify as `org/repo` (github_ref)
    // because the user's *intent* is more likely to cite a repo than a
    // 2-segment file path; deeper file paths are already caught by rule 1.
    if head_is_org && has_ext {
        return false;
    }
    if !head_is_org {
        return true;
    }
    false
}

fn is_pathlike(s: &str) -> bool {
    (s.contains('/') || s.contains('\\')) && !s.starts_with("http") && s.len() < 256
}

fn parse_github_ref(s: &str) -> Option<()> {
    let mut parts = s.split('/');
    let org = parts.next()?;
    let repo = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if org.is_empty() || repo.is_empty() {
        return None;
    }
    if !org
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    if !repo
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return None;
    }
    Some(())
}

fn extract_tech_keywords(lower: &str, tech: &mut Vec<String>, ents: &mut Vec<NamedEntity>) {
    const TECH: &[&str] = &[
        "rust",
        "tauri",
        "svelte",
        "sveltekit",
        "typescript",
        "javascript",
        "node",
        "npm",
        "pnpm",
        "cargo",
        "axum",
        "tower-lsp",
        "sqlx",
        "rusqlite",
        "sqlite",
        "react",
        "vue",
        "next.js",
        "prisma",
        "tailwind",
        "python",
        "django",
        "fastapi",
        "go",
        "kubernetes",
        "docker",
        "terraform",
        "aws",
        "azure",
        "gcp",
        "postgres",
        "mysql",
        "redis",
        "kafka",
    ];
    for kw in TECH {
        if lower.contains(kw) {
            tech.push((*kw).to_string());
            ents.push(NamedEntity {
                text: (*kw).to_string(),
                r#type: "technology".into(),
                in_kb: false,
            });
        }
    }
    tech.sort();
    tech.dedup();
}

fn detect_implicit_signals(lower: &str, out: &mut Vec<String>) {
    const SIGNALS: &[(&str, &str)] = &[
        ("should ", "should"),
        ("i think", "i_think"),
        ("maybe ", "maybe"),
        ("probably", "probably"),
        ("i guess", "i_guess"),
        ("asap", "urgency"),
        ("urgent", "urgency"),
        ("quick", "urgency"),
        ("you already know", "model_omniscience"),
        ("you know", "model_omniscience"),
        ("as an ai", "role_signal"),
        ("act as", "role_signal"),
        ("pretend you", "role_signal"),
    ];
    for (needle, label) in SIGNALS {
        if lower.contains(needle) {
            out.push((*label).to_string());
        }
    }
    out.sort();
    out.dedup();
}

fn guess_desired_action(lower: &str) -> DesiredAction {
    let token_count = lower.split_whitespace().count();
    if token_count <= 1
        && !matches!(
            lower,
            "create" | "modify" | "debug" | "test" | "explain" | "research" | "refactor"
        )
    {
        return DesiredAction::Unknown;
    }
    if lower.starts_with("create ") || lower.starts_with("add ") || lower.starts_with("new ") {
        DesiredAction::Create
    } else if lower.starts_with("fix ")
        || lower.starts_with("debug ")
        || lower.starts_with("repair ")
        || lower.contains("stack trace")
        || lower.contains("error ")
    {
        DesiredAction::Debug
    } else if lower.starts_with("explain ")
        || lower.starts_with("what ")
        || lower.starts_with("why ")
        || lower.starts_with("how ")
    {
        DesiredAction::Explain
    } else if lower.starts_with("test ") || lower.contains("write tests") {
        DesiredAction::Test
    } else if lower.starts_with("research ") || lower.contains("find ") {
        DesiredAction::Research
    } else if lower.starts_with("refactor ") || lower.contains("migrate ") {
        DesiredAction::Refactor
    } else if !lower.is_empty() {
        DesiredAction::Modify
    } else {
        DesiredAction::Unknown
    }
}

fn guess_domain(lower: &str) -> Domain {
    const FE: &[&str] = &[
        "react", "vue", "svelte", "frontend", "css", "tailwind", "ui", "ux",
    ];
    const BE: &[&str] = &[
        "backend", "api", "endpoint", "rest", "graphql", "rpc", "server",
    ];
    const DO: &[&str] = &[
        "docker",
        "kubernetes",
        "terraform",
        "ci/cd",
        "github actions",
        "deploy",
        "helm",
        "ansible",
    ];
    const DA: &[&str] = &[
        "sql",
        "etl",
        "pipeline",
        "dataframe",
        "pandas",
        "spark",
        "dbt",
    ];
    const ML: &[&str] = &[
        "model",
        "training",
        "inference",
        "embedding",
        "llm",
        "vector",
        "agent",
        "rag",
    ];
    const DC: &[&str] = &[
        "README",
        "readme",
        "docs",
        "documentation",
        "tutorial",
        "guide",
    ];

    let any = |kws: &[&str]| kws.iter().any(|k| lower.contains(k));
    if any(FE) {
        Domain::Frontend
    } else if any(BE) {
        Domain::Backend
    } else if any(DO) {
        Domain::Devops
    } else if any(DA) {
        Domain::Data
    } else if any(ML) {
        Domain::Ml
    } else if any(DC) {
        Domain::Docs
    } else {
        Domain::Unknown
    }
}

fn guess_scope(lower: &str, token_count: usize) -> Scope {
    if lower.contains("whole project")
        || lower.contains("entire repo")
        || lower.contains("repo-wide")
        || lower.contains("everything")
    {
        Scope::Project
    } else if lower.contains("module") || lower.contains("service") || lower.contains("crate") {
        Scope::Module
    } else if lower.contains("feature") || lower.contains("epic") {
        Scope::Feature
    } else if token_count < 12 || lower.contains("this file") || lower.contains("in this file") {
        Scope::File
    } else {
        Scope::Unknown
    }
}

fn build_intent(normalised: &str, desired_action: DesiredAction, keys: &[String]) -> String {
    let verb = match desired_action {
        DesiredAction::Create => "Create",
        DesiredAction::Modify => "Modify",
        DesiredAction::Debug => "Debug",
        DesiredAction::Explain => "Explain",
        DesiredAction::Test => "Test",
        DesiredAction::Research => "Research",
        DesiredAction::Refactor => "Refactor",
        DesiredAction::Unknown => "Investigate",
    };
    // RFC 23 §1 C7 — low_information_prompt catalogue. A bare token from this
    // set carries no real intent target; fall back to `<missing>` so the
    // verdict intent statement reads `Investigate <missing>` and step 7's
    // Self-Refine + rubric treat the verdict as Block.
    const LOW_INFO_TOKENS: &[&str] = &[
        "go", "ok", "okay", "fix", "yes", "no", "y", "n", "continue", "next",
    ];
    let bare_low_info = normalised
        .split_whitespace()
        .next()
        .map(|t| {
            let lower = t.to_lowercase();
            normalised.split_whitespace().count() == 1 && LOW_INFO_TOKENS.contains(&lower.as_str())
        })
        .unwrap_or(false);
    let target = keys.first().cloned().unwrap_or_else(|| {
        if normalised.is_empty() || bare_low_info {
            "<missing>".to_string()
        } else {
            normalised.chars().take(80).collect::<String>()
        }
    });
    let mut s = format!("{verb} {target}");
    if normalised.chars().count() > 80 {
        s.push_str(" …(truncated)");
    }
    s
}

fn build_hypotheses(
    intent: &str,
    keys: &[String],
    named: &[NamedEntity],
    signals: &[String],
) -> Vec<IntentHypothesis> {
    let mut hyps: Vec<IntentHypothesis> = Vec::new();

    // Rank 1: the canonical interpretation.
    let mut feasibility: f32 = 0.7;
    if !signals.is_empty() {
        feasibility -= 0.15;
    }
    if keys.is_empty() && named.is_empty() {
        feasibility -= 0.2;
    }
    hyps.push(IntentHypothesis {
        rank: 1,
        text: intent.to_string(),
        feasibility_score: feasibility.clamp(0.0, 1.0),
        rejection_reason: None,
    });

    // Rank 2 (optional) — the user really wanted to debug, even if they
    // said "fix"/"better". Produced when a key + the words "always" or
    // "again" appear — proxy for recurrence.
    let lowered = intent.to_lowercase();
    if lowered.contains("again") || lowered.contains("always") || lowered.contains("still") {
        hyps.push(IntentHypothesis {
            rank: 2,
            text: format!("Debug recurring issue in {intent}"),
            feasibility_score: (feasibility - 0.1).max(0.0),
            rejection_reason: None,
        });
    }

    hyps.sort_by_key(|a| a.rank);
    hyps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::types::DesiredAction;

    #[test]
    fn backtick_keys_are_extracted_dedup_sorted() {
        let p = run("fix `bug` in `rating` and `bug`");
        assert_eq!(p.keys, vec!["bug", "rating"]);
    }

    #[test]
    fn url_path_github_entities_detected() {
        let p = run("see https://example.com and src/lib.rs and vercel/next.js");
        let types: Vec<&str> = p.named_entities.iter().map(|e| e.r#type.as_str()).collect();
        assert!(types.contains(&"url"));
        assert!(types.contains(&"file_path"));
        assert!(types.contains(&"github_ref"));
    }

    #[test]
    fn implicit_signals_gas_model_omniscience_and_role() {
        let p = run("you already know this, act as a chef and maybe fix it");
        assert!(p
            .implicit_signals
            .contains(&"model_omniscience".to_string()));
        assert!(p.implicit_signals.contains(&"role_signal".to_string()));
        assert!(p.implicit_signals.contains(&"maybe".to_string()));
    }

    #[test]
    fn desired_action_is_guessed_by_leading_verb() {
        assert_eq!(run("fix it").desired_action, DesiredAction::Debug);
        assert_eq!(
            run("explain the file").desired_action,
            DesiredAction::Explain
        );
        assert_eq!(
            run("create a new file").desired_action,
            DesiredAction::Create
        );
        assert_eq!(
            run("write tests for fmt").desired_action,
            DesiredAction::Test
        );
    }

    #[test]
    fn domain_detection_matches_keyword_groups() {
        use crate::prompt::types::Domain;
        assert_eq!(run("add a React component").domain, Domain::Frontend);
        assert_eq!(run("modify the axum endpoint").domain, Domain::Backend);
        assert_eq!(run("deploy on kubernetes").domain, Domain::Devops);
        assert_eq!(run("train the embedding model").domain, Domain::Ml);
    }

    #[test]
    fn low_information_prompt_yields_unknown_intent_target() {
        let p = run("go");
        assert_eq!(p.intent, "Investigate <missing>");
    }
}
