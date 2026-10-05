// Atlas OS — `GET /hud/demos` (RFC 65 §9, Demos over diffs; RFC 24 §9).
//
// Powers `<DemoPane>`: the demo artefacts produced by agent runs (`artifacts`,
// M50) — screenshots / files / logs / preview URLs. The full Loom-style TTS
// video capture (RFC 24 §9) is not implemented; this endpoint surfaces the
// grounded subset the capability layer actually persists. Read-only.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::core::state::AppState;

#[derive(Debug, Deserialize)]
pub struct DemosQuery {
    /// Number of most-recent artefacts to return (1..=500, default 50).
    pub last: Option<i64>,
}

/// Best-effort preview URL from an artefact's `evidence_json`. Looks for the
/// common URL-ish keys and only accepts `http(s)` values, so a stray string is
/// never rendered as a link.
fn extract_preview_url(evidence_json: Option<&str>) -> Option<String> {
    let value: Value = serde_json::from_str(evidence_json?).ok()?;
    for key in ["preview_url", "url", "demo_url", "preview", "href"] {
        if let Some(s) = value.get(key).and_then(Value::as_str) {
            if s.starts_with("http://") || s.starts_with("https://") {
                return Some(s.to_string());
            }
        }
    }
    None
}

pub async fn get_demos(
    State(state): State<Arc<AppState>>,
    Query(q): Query<DemosQuery>,
) -> Json<Value> {
    let last = q.last.unwrap_or(50).clamp(1, 500);
    let journal = state.journal();
    let artifacts = journal.artifact_tail(last).unwrap_or_default();
    drop(journal);

    let items: Vec<Value> = artifacts
        .iter()
        .map(|a| {
            json!({
                "id": a.id,
                "run_id": a.run_id,
                "kind": a.kind,
                "path": a.path,
                "sha256": a.sha256,
                "verified": a.verified,
                "preview_url": extract_preview_url(a.evidence_json.as_deref()),
            })
        })
        .collect();

    Json(json!({ "artifacts": items, "count": items.len() }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{AgentRunRow, ArtifactRow};

    fn run(id: &str) -> AgentRunRow {
        AgentRunRow {
            id: id.into(),
            mission_id: None,
            goal: "g".into(),
            success_predicate: "[]".into(),
            sandbox: "local".into(),
            status: "done".into(),
            steps: 0,
            tokens_in: 0,
            tokens_out: 0,
            cost_usd: 0.0,
            ts_started: 1,
            ts_ended: Some(2),
        }
    }

    #[test]
    fn extract_preview_url_only_accepts_http() {
        assert_eq!(
            extract_preview_url(Some(r#"{"preview_url":"http://localhost:3000"}"#)).as_deref(),
            Some("http://localhost:3000")
        );
        assert_eq!(extract_preview_url(Some(r#"{"url":"ftp://x"}"#)), None);
        assert_eq!(extract_preview_url(Some("not json")), None);
        assert_eq!(extract_preview_url(None), None);
    }

    #[tokio::test]
    async fn lists_artifacts_with_preview_url() {
        let dir = tempfile::TempDir::new().unwrap();
        let journal = crate::journal::Journal::open(dir.path()).unwrap();
        journal.create_agent_run(&run("r1")).unwrap();
        journal
            .record_artifact(&ArtifactRow {
                id: "a1".into(),
                run_id: "r1".into(),
                kind: "screenshot".into(),
                path: Some("shot.png".into()),
                sha256: Some("abc".into()),
                verified: true,
                evidence_json: Some(r#"{"preview_url":"https://cdn.example/shot.png"}"#.into()),
            })
            .unwrap();
        journal
            .record_artifact(&ArtifactRow {
                id: "a2".into(),
                run_id: "r1".into(),
                kind: "file".into(),
                path: Some("out.txt".into()),
                sha256: None,
                verified: false,
                evidence_json: None,
            })
            .unwrap();
        let state = Arc::new(AppState::from_journal_in(journal, dir.path().to_path_buf()));

        let Json(v) = get_demos(State(state), Query(DemosQuery { last: Some(10) })).await;
        assert_eq!(v["count"], 2);
        let items = v["artifacts"].as_array().unwrap();
        let with_url = items
            .iter()
            .find(|a| a["kind"] == "screenshot")
            .expect("screenshot");
        assert_eq!(with_url["preview_url"], "https://cdn.example/shot.png");
        assert_eq!(with_url["verified"], true);
    }
}
