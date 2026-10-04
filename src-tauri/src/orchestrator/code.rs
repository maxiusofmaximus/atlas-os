// Atlas OS — LLM-driven Coding: request + parse a `Diff` from a model (Fase 26 v26.1).
//
// Completes the bridge started in v26.0 (`coding::llm::parse_diff_json`): this
// routes the step to a model through `call_with_cascade` (routing + failover +
// reliability gate upstream) and turns the reply into a canonical `Diff`. The
// `Diff` is what the pure Validation/Repair engines consume (v26.2), so a code
// step stops being "a chat message" and becomes an edit that gets verified.

use serde::Deserialize;
use uuid::Uuid;

use crate::coding::llm::{parse_diff_json, DiffMeta, DiffParseError};
use crate::coding::types::Diff;
use crate::orchestrator::call::{call_with_cascade_and_denied, CallError};
use crate::orchestrator::client::{ChatMessage, ChatRequest, ProviderClient, Usage};
use crate::orchestrator::provider::Deployment;
use crate::orchestrator::routing::RoutingConfig;

/// A routed step whose model reply was successfully parsed into a `Diff`.
#[derive(Clone, Debug)]
pub struct StepDiffOutcome {
    pub diff: Diff,
    pub deployment_id: String,
    pub attempts: u8,
    pub usage: Option<Usage>,
    /// `true` when the model's reply dropped the `narrative`, so the caller can
    /// surface that the gate would warn even on an otherwise clean diff.
    pub narrative_missing: bool,
}

/// Workspace context fed to the model so it can produce a GROUNDED `Diff`
/// instead of asking for context it cannot see (a real-LLM finding: with only
/// the bare step statement, a model legitimately refuses). `files` is the tree
/// (relative paths) and `snippets` is `path -> content` for the files most
/// likely relevant to the step, already truncated to a token budget by the host.
#[derive(Clone, Debug, Default)]
pub struct WorkspaceContext {
    pub files: Vec<String>,
    pub snippets: Vec<(String, String)>,
}

impl WorkspaceContext {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.snippets.is_empty()
    }

    /// Render as a prompt section. `snippets` are the highest-signal part, so
    /// they come first; the file list follows as a short map.
    pub fn render(&self) -> String {
        let mut out = String::new();
        if !self.files.is_empty() {
            out.push_str("\nWorkspace files (relative to repo root):\n");
            for f in self.files.iter().take(200) {
                out.push_str("  - ");
                out.push_str(f);
                out.push('\n');
            }
        }
        if !self.snippets.is_empty() {
            out.push_str("\nRelevant file contents (ground your hunks on these line numbers):\n");
            for (path, body) in &self.snippets {
                out.push_str(&format!("\n--- {path} ---\n{body}\n"));
            }
        }
        out
    }
}

/// The code prompt asks the model for a structured diff whose `narrative`
/// states why, keeping the EvidenceGate (RFC 30 §2.1) satisfiable.
pub const CODING_DIFF_PROMPT: &str = r#"You are the Atlas Coding Engine. Produce the code change for the step below as ONE JSON object and nothing else — no prose, no markdown fences.

Schema:
{
  "narrative": "<what changed, what it does now, why — required>",
  "files": [
    { "path": "relative/path.ext", "is_new_file": false, "is_delete": false,
      "hunks": [ { "old_start": 10, "old_end": 12, "new_lines": ["line"], "rationale": "why" } ] }
  ]
}

Rules:
- The `narrative` is REQUIRED and must state the change and its rationale.
- Emit structured hunks, never a unified-diff blob. `old_start`..`old_end` is the half-open 0-based range being replaced; `0,0` inserts at the top; empty `new_lines` deletes.
- Paths are relative to the repo root with `/`.
- If the change affects behaviour, also edit or add a test file so the change carries check evidence.
"#;

/// Optional Research Engine evidence (Fase 36). When present the model is asked
/// to cite which research refs justify the change, and those refs are attached
/// to the `Diff` verbatim — the EvidenceGate (RFC 30 §2.1) is what consumes
/// them. Citations are matched by INDEX so the model cannot coin a UUID.
#[derive(Clone, Debug, Default)]
pub struct ResearchContext {
    pub run_id: String,
    pub refs: Vec<Uuid>,
    /// Short summaries parallel to `refs` (what each ref found).
    pub notes: Vec<String>,
}

impl ResearchContext {
    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }
}

#[derive(Deserialize)]
struct RawCitedDiff {
    #[serde(default)]
    narrative: String,
    #[serde(default)]
    cites: Vec<usize>,
}

/// Either the cascade could not reach any model, or the reply was not a usable
/// diff. Kept structured so the caller can branch (a `Parse` error is worth a
/// re-ask; a `Call` error is worth a cascade/cooldown reaction).
#[derive(Debug, thiserror::Error)]
pub enum StepDiffError {
    #[error("cascade failed: {0}")]
    Call(#[from] CallError),
    #[error("model reply was not a usable diff: {0}")]
    Parse(#[from] DiffParseError),
}

/// Ask the routed model for a structured `Diff` for `step_statement` and parse
/// it. `meta` carries the kernel-side identity the model must not supply.
/// `denied` lists model ids the reliability gate has excluded (Fase 24/F33):
/// they are skipped for both the primary and the failover groups. `research`,
/// when present, adds the refs so the model cites them (Fase 36).
#[allow(clippy::too_many_arguments)]
pub async fn call_diff_with_cascade_and_denied<C: ProviderClient>(
    client: &C,
    routing: RoutingConfig,
    model: &str,
    deployments: &[Deployment],
    step_statement: &str,
    meta: DiffMeta,
    max_attempts: u8,
    denied: &[String],
    research: &ResearchContext,
    workspace: &WorkspaceContext,
) -> Result<StepDiffOutcome, StepDiffError> {
    let mut system = CODING_DIFF_PROMPT.to_string();
    if !workspace.is_empty() {
        system.push_str(&workspace.render());
    }
    if !research.is_empty() {
        let listing = research
            .refs
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let note = research.notes.get(i).map(String::as_str).unwrap_or("");
                format!("[{i}] ({r}) {note}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        system.push_str(&format!(
            "\nResearch evidence (cite the indexes you rely on via a top-level \"cites\": [n, ...] field):\n{listing}"
        ));
    }
    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage::system(system.clone()),
            ChatMessage::user(step_statement),
        ],
        temperature: None,
        max_tokens: None,
    };

    let outcome = call_with_cascade_and_denied(
        client,
        routing.clone(),
        model,
        deployments,
        &request,
        max_attempts,
        denied,
    )
    .await?;

    let content = outcome.response.content.clone();
    // Models sometimes answer in prose or omit `files`. One firm re-ask before
    // giving up: the retry is offline-safe (deterministic mock replies are
    // consumed in order) and materially improves real-LLM success.
    let (mut diff, content) = match parse_diff_json(&content, meta.clone()) {
        Ok(d) => (d, content),
        Err(first_err) => {
            let repair_request = ChatRequest {
                model: model.to_string(),
                messages: vec![
                    ChatMessage::system(system),
                    ChatMessage::user(step_statement),
                    ChatMessage::assistant(content.clone()),
                    ChatMessage::user(
                        "Your reply was not valid diff JSON. Reply with ONLY the JSON object \
                         (keys: narrative, files[].path, files[].hunks[].{old_start,old_end,new_lines,rationale}). \
                         No prose, no markdown."
                            .to_string(),
                    ),
                ],
                temperature: None,
                max_tokens: None,
            };
            let retry = call_with_cascade_and_denied(
                client,
                routing,
                model,
                deployments,
                &repair_request,
                max_attempts,
                denied,
            )
            .await?;
            let retry_content = retry.response.content.clone();
            match parse_diff_json(&retry_content, meta.clone()) {
                Ok(d) => (d, retry_content),
                Err(second_err) => {
                    tracing::warn!(
                        first = %first_err,
                        second = %second_err,
                        raw = %retry_content.chars().take(600).collect::<String>(),
                        "coding model did not produce a usable diff after retry"
                    );
                    return Err(StepDiffError::Parse(first_err));
                }
            }
        }
    };
    let narrative_missing = diff.narrative.trim().is_empty();
    if !research.is_empty() {
        if let Some(raw) = parse_cited_diff(&content) {
            if diff.narrative.trim().is_empty() && !raw.narrative.trim().is_empty() {
                diff.narrative = raw.narrative;
            }
            let mut seen = std::collections::BTreeSet::new();
            for idx in raw.cites {
                if let Some(r) = research.refs.get(idx) {
                    if seen.insert(*r) && !diff.research_refs.contains(r) {
                        diff.research_refs.push(*r);
                    }
                }
            }
        }
    }

    Ok(StepDiffOutcome {
        diff,
        deployment_id: outcome.deployment_id,
        attempts: outcome.attempts,
        usage: outcome.response.usage,
        narrative_missing,
    })
}

/// Best-effort second parse that also captures the optional `cites` array. The
/// strict `parse_diff_json` owns validation; this only extracts extra fields.
fn parse_cited_diff(content: &str) -> Option<RawCitedDiff> {
    let s = content.trim();
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str(&s[start..=end]).ok()
}

/// Convenience wrapper with no denied models and no research (behaviour
/// identical to before F33/F36).
#[allow(clippy::too_many_arguments)]
pub async fn call_diff_with_cascade<C: ProviderClient>(
    client: &C,
    routing: RoutingConfig,
    model: &str,
    deployments: &[Deployment],
    step_statement: &str,
    meta: DiffMeta,
    max_attempts: u8,
) -> Result<StepDiffOutcome, StepDiffError> {
    call_diff_with_cascade_and_denied(
        client,
        routing,
        model,
        deployments,
        step_statement,
        meta,
        max_attempts,
        &[],
        &ResearchContext::default(),
        &WorkspaceContext::default(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::client::{ChatResponse, ClientResult};
    use std::sync::Mutex;
    use uuid::Uuid;

    struct Scripted {
        replies: Mutex<Vec<ClientResult<ChatResponse>>>,
    }

    impl Scripted {
        fn new(replies: Vec<ClientResult<ChatResponse>>) -> Self {
            Self {
                replies: Mutex::new(replies),
            }
        }
    }

    impl ProviderClient for Scripted {
        async fn chat(
            &self,
            _deployment: &Deployment,
            _request: &ChatRequest,
        ) -> ClientResult<ChatResponse> {
            self.replies.lock().unwrap().remove(0)
        }
    }

    fn dep(model: &str, id: &str) -> Deployment {
        let mut d = Deployment::new(model, format!("http://{id}"));
        d.id = id.into();
        d
    }

    fn meta() -> DiffMeta {
        DiffMeta {
            mission_id: Uuid::new_v4(),
            plan_id: Uuid::new_v4(),
            step_id: "S1".into(),
            agent_id: Uuid::new_v4(),
            model_id: "m1".into(),
        }
    }

    fn json_reply() -> ChatResponse {
        ChatResponse {
            model: "m1".into(),
            content: r#"{"files":[{"path":"src/lib.rs","hunks":[{"old_start":0,"old_end":0,"new_lines":["pub fn hi() {}"],"rationale":"r"}]}]}"#.into(),
            usage: None,
        }
    }

    #[tokio::test]
    async fn parses_a_routed_diff() {
        let deployments = vec![dep("m1", "d1")];
        let client = Scripted::new(vec![Ok(json_reply())]);
        let out = call_diff_with_cascade(
            &client,
            RoutingConfig::default(),
            "m1",
            &deployments,
            "add a hi() function",
            meta(),
            5,
        )
        .await
        .expect("routed diff parses");
        assert_eq!(out.diff.files[0].path, "src/lib.rs");
        assert_eq!(
            out.diff.files[0].hunks[0].new_lines,
            vec!["pub fn hi() {}".to_string()]
        );
        assert_eq!(out.deployment_id, "d1");
    }

    #[tokio::test]
    async fn non_diff_reply_is_a_parse_error() {
        let deployments = vec![dep("m1", "d1")];
        let client = Scripted::new(vec![
            Ok(ChatResponse {
                model: "m1".into(),
                content: "Sure! I would add a function.".into(),
                usage: None,
            }),
            Ok(ChatResponse {
                model: "m1".into(),
                content: "Still prose, no JSON here either.".into(),
                usage: None,
            }),
        ]);
        let err = call_diff_with_cascade(
            &client,
            RoutingConfig::default(),
            "m1",
            &deployments,
            "add a hi() function",
            meta(),
            5,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(err, StepDiffError::Parse(DiffParseError::NoJson)),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn denied_primary_is_skipped_for_the_failover_model() {
        use std::collections::HashMap;
        let deployments = vec![dep("m1", "d1"), dep("m2", "d2")];
        let client = Scripted::new(vec![Ok(json_reply())]);
        // m1 denied → the cascade fails over to m2 via the configured bucket.
        let mut fallback = HashMap::new();
        fallback.insert("m1".to_string(), vec!["m2".to_string()]);
        let routing = RoutingConfig {
            fallback: crate::orchestrator::routing::FallbackMap {
                fallbacks: fallback,
                ..Default::default()
            },
            ..RoutingConfig::default()
        };
        let out = call_diff_with_cascade_and_denied(
            &client,
            routing,
            "m1",
            &deployments,
            "add a hi() function",
            meta(),
            5,
            &["m1".to_string()],
            &ResearchContext::default(),
            &WorkspaceContext::default(),
        )
        .await
        .expect("failover to m2");
        assert_eq!(
            out.deployment_id, "d2",
            "the denied primary must not be used"
        );
    }

    #[tokio::test]
    async fn research_refs_are_attached_from_citations() {
        let deployments = vec![dep("m1", "d1")];
        let r0 = uuid::Uuid::new_v4();
        let r1 = uuid::Uuid::new_v4();
        let client = Scripted::new(vec![Ok(ChatResponse {
            model: "m1".into(),
            content: r#"{"narrative":"adds add() per research","cites":[0],"files":[{"path":"src/lib.rs","hunks":[{"old_start":0,"old_end":0,"new_lines":["pub fn add(a: u32, b: u32) -> u32 { a + b }"],"rationale":"r"}]}]}"#.into(),
            usage: None,
        })]);
        let research = ResearchContext {
            run_id: "rr-x".into(),
            refs: vec![r0, r1],
            notes: vec!["tokio spawn docs".into(), "unused".into()],
        };
        let out = call_diff_with_cascade_and_denied(
            &client,
            RoutingConfig::default(),
            "m1",
            &deployments,
            "add add()",
            meta(),
            5,
            &[],
            &research,
            &WorkspaceContext::default(),
        )
        .await
        .expect("cites parsed");
        assert_eq!(out.diff.research_refs, vec![r0], "only cited refs attach");
        assert!(!out.narrative_missing);
    }

    #[tokio::test]
    async fn invalid_citation_indexes_are_ignored() {
        let deployments = vec![dep("m1", "d1")];
        let r0 = uuid::Uuid::new_v4();
        let client = Scripted::new(vec![Ok(ChatResponse {
            model: "m1".into(),
            content: r#"{"narrative":"n","cites":[99],"files":[{"path":"a.rs","hunks":[{"old_start":0,"old_end":0,"new_lines":["x"]}]}]}"#.into(),
            usage: None,
        })]);
        let research = ResearchContext {
            run_id: "rr-x".into(),
            refs: vec![r0],
            notes: vec![],
        };
        let out = call_diff_with_cascade_and_denied(
            &client,
            RoutingConfig::default(),
            "m1",
            &deployments,
            "s",
            meta(),
            5,
            &[],
            &research,
            &WorkspaceContext::default(),
        )
        .await
        .expect("out-of-range cite is ignored, not fatal");
        assert!(out.diff.research_refs.is_empty());
    }
}
