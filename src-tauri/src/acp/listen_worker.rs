// Atlas OS — ACP `wtcli listen --json` worker (RFC 28 §B item 5).
//
// The worker is the long-running task that spawns a `wtcli listen
// --event "agent.*" --json` subprocess, reads its stdout one JSON-line
// at a time, parses each envelope into a typed `AgentEventEnvelope`,
// and UPSERTs it into `agent_session_events` (M14) via
// `journal::agent_events::insert_agent_session_event`.
//
// Phase 1.5d ships the **pure** half of this loop: the JSON-line parser,
// the envelope type, and the persistence-decision function. The
// subprocess-spawning half (`spawn()` + `drive_loop`) is a typed stub
// returning `SpawnError::StubNotWired` — wiring actual stdin/stdout of
// `wtcli` lands with the host-loop effort of item 5+ (which also wires
// the `session/prompt` host loop from item 4's commands/delegate). This
// keeps the contract pinned, testable, and non-panicking today.
//
// The envelope parser is intentionally tolerant: any well-formed JSON
// envelope that matches the OSC 9001 schema is accepted; malformed
// entries return `ParseEnvelopeError` so the future subprocess loop
// can decide between drop-and-continue vs panic-on-corrupted-stream.

use serde::{Deserialize, Serialize};

use crate::journal::agent_events;

/// Top-level OSC 9001 envelope produced by `wtcli listen --json`. See
/// `src-tauri/specs/osc-9001.md` §"Event Schema". The shape is stable
/// across event types: `params` is the per-event payload (kept verbatim
/// by the persistence layer so any future structured query is possible
/// without a schema migration).
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AgentEventEnvelope {
    #[serde(rename = "type")]
    pub kind: String,
    pub method: String,
    pub params: AgentEventParams,
}

/// `params` sub-object of the OSC 9001 envelope. Only the fields the
/// persistence layer cares about are typed; the rest of the payload is
/// available via the original JSON the listener forwards to
/// `payload_json` verbatim.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct AgentEventParams {
    pub pane_id: Option<String>,
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
}

/// Why the parser rejected a JSON line. Used by the future subprocess
/// loop to drop-and-continue vs raise; the typed stub unit-tests assert
/// on the enum so the contract is pinned before the subprocess lands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseEnvelopeError {
    InvalidJson,
    WrongType {
        expected: &'static str,
        found: String,
    },
    WrongMethod {
        expected: &'static str,
        found: String,
    },
    MissingEvent,
}

impl std::fmt::Display for ParseEnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson => f.write_str("invalid JSON"),
            Self::WrongType { expected, found } => {
                write!(f, "type mismatch: expected `{expected}`, found `{found}`")
            }
            Self::WrongMethod { expected, found } => {
                write!(f, "method mismatch: expected `{expected}`, found `{found}`")
            }
            Self::MissingEvent => f.write_str("params.event missing"),
        }
    }
}

impl std::error::Error for ParseEnvelopeError {}

/// Parse one JSON line emitted by `wtcli listen --json` into the typed
/// envelope. Rejects anything that does not look like an OSC 9001
/// `agent_event` envelope (`type == "event"`, `method == "agent_event"`,
/// `params.event != ""`).
pub fn parse_envelope(line: &str) -> Result<AgentEventEnvelope, ParseEnvelopeError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(ParseEnvelopeError::InvalidJson);
    }
    let env: AgentEventEnvelope =
        serde_json::from_str(trimmed).map_err(|_| ParseEnvelopeError::InvalidJson)?;
    if env.kind != "event" {
        return Err(ParseEnvelopeError::WrongType {
            expected: "event",
            found: env.kind,
        });
    }
    if env.method != "agent_event" {
        return Err(ParseEnvelopeError::WrongMethod {
            expected: "agent_event",
            found: env.method,
        });
    }
    if env.params.event.is_empty() {
        return Err(ParseEnvelopeError::MissingEvent);
    }
    Ok(env)
}

/// Wall-clock seconds since the UNIX epoch. The OSC 9001 spec leaves the
/// timestamp format open (RFC 3339 strings are common but not mandated);
/// the listener worker normalises everything to `i64` so range queries
/// on `idx_ase_ts` stay cheap. Falls back to `0` when the timestamp
/// field is missing or unparseable — the actual `id` column still
/// preserves the receive order so HUD can fall back on `ORDER BY id`.
pub fn timestamp_to_epoch_seconds(ts: Option<&str>) -> i64 {
    let Some(s) = ts else { return 0 };
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(s) else {
        return 0;
    };
    parsed.timestamp()
}

/// Outcome of persisting one envelope. The future subprocess loop
/// tallies these so an operator can grep the journal for
/// `dropped_invalid_envelopes` metrics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PersistOutcome {
    Inserted {
        rowid: i64,
        ts: i64,
        event_type: String,
        pane_id: Option<String>,
    },
    Dropped {
        reason: ParseEnvelopeError,
    },
}

/// Parse + persist one envelope line into `agent_session_events` via
/// `&Connection`. Returns `Inserted` on success or `Dropped` on parse
/// failure (caller decides whether to terminate the loop). The fn is
/// pure-ish (state lives in SQLite) so the test-suite can drive it with
/// crafted JSON-lines and assert on the row that landed.
pub fn persist_envelope(conn: &rusqlite::Connection, line: &str) -> PersistOutcome {
    let env = match parse_envelope(line) {
        Ok(e) => e,
        Err(reason) => return PersistOutcome::Dropped { reason },
    };
    let ts = timestamp_to_epoch_seconds(env.params.timestamp.as_deref());
    let event_type = env.params.event.clone();
    let pane_id = env.params.pane_id.clone();
    let agent = env
        .params
        .agent
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    // Re-serialise a minimal payload_json so the table stays self-contained:
    // callers can `SELECT payload_json` and get the same fields they parsed.
    let payload = serde_json::to_string(&env.params).unwrap_or_else(|_| "{}".into());
    let rowid = agent_events::insert_agent_session_event(
        conn,
        ts,
        pane_id.as_deref(),
        &event_type,
        &agent,
        env.params.task_id.as_deref(),
        &payload,
    );
    match rowid {
        Ok(id) => PersistOutcome::Inserted {
            rowid: id,
            ts,
            event_type,
            pane_id,
        },
        // Persistence failures are fatal in Phase 1.5d — surface them as a
        // panic so the test suite catches bugs early. The future subprocess
        // loop will instead surface this as a `SpawnError::Persist` event.
        Err(e) => panic!("agent_session_events insert failed: {e:#}"),
    }
}

/// Typed result of `spawn_listener` in Phase 1.5d. The stub returns
/// `SpawnError::StubNotWired` so the binary's caller (`run_server` /
/// CLI plumbing) can decide whether to skip the worker or surface a
/// warning. Phase 2 swaps this for the real `tokio::process::Child`
/// handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpawnError {
    UnsupportedPlatform,
    NoWtComClsid,
    StubNotWired,
}

/// Phase 1.5d stub for spawning the `wtcli listen --json` subprocess.
/// Returns `Err(SpawnError)` for the same three reasons
/// `acp::delegate::capture_active_pane_scrollback` does: non-Windows
/// hosts, missing `WT_COM_CLSID` discovery env, or `StubNotWired` when
/// both preconditions pass but the host-loop wiring is still deferred.
pub fn spawn_listener() -> Result<(), SpawnError> {
    if cfg!(not(target_os = "windows")) {
        return Err(SpawnError::UnsupportedPlatform);
    }
    if std::env::var_os("WT_COM_CLSID").is_none() {
        return Err(SpawnError::NoWtComClsid);
    }
    Err(SpawnError::StubNotWired)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::schema::migrate;
    use tempfile::TempDir;

    fn fresh_conn() -> (TempDir, rusqlite::Connection) {
        let tmp = TempDir::new().expect("tmp");
        let conn = rusqlite::Connection::open(tmp.path().join("j.db")).expect("open");
        migrate(&conn).expect("migrate");
        (tmp, conn)
    }

    #[test]
    fn parses_canonical_task_started_envelope() {
        let line = r#"{"type":"event","method":"agent_event","params":{"pane_id":"3","event":"agent.task.started","timestamp":"2026-04-14T12:00:00Z","agent":"copilot-cli","task_id":"abc-123","description":"build auth"}}"#;
        let env = parse_envelope(line).expect("parse");
        assert_eq!(env.kind, "event");
        assert_eq!(env.method, "agent_event");
        assert_eq!(env.params.event, "agent.task.started");
        assert_eq!(env.params.pane_id.as_deref(), Some("3"));
        assert_eq!(env.params.agent.as_deref(), Some("copilot-cli"));
        assert_eq!(env.params.task_id.as_deref(), Some("abc-123"));
    }

    #[test]
    fn rejects_empty_line() {
        assert_eq!(
            parse_envelope("   ").unwrap_err(),
            ParseEnvelopeError::InvalidJson
        );
    }

    #[test]
    fn rejects_non_event_type() {
        let line =
            r#"{"type":"request","method":"agent_event","params":{"event":"agent.started"}}"#;
        let err = parse_envelope(line).unwrap_err();
        assert!(matches!(
            err,
            ParseEnvelopeError::WrongType {
                expected: "event",
                ..
            }
        ));
    }

    #[test]
    fn rejects_non_agent_event_method() {
        let line = r#"{"type":"event","method":"vt_sequence","params":{"event":"agent.started"}}"#;
        let err = parse_envelope(line).unwrap_err();
        assert!(matches!(
            err,
            ParseEnvelopeError::WrongMethod {
                expected: "agent_event",
                ..
            }
        ));
    }

    #[test]
    fn rejects_missing_event_field() {
        let line = r#"{"type":"event","method":"agent_event","params":{"pane_id":"3"}}"#;
        assert_eq!(
            parse_envelope(line).unwrap_err(),
            ParseEnvelopeError::MissingEvent
        );
    }

    #[test]
    fn timestamp_to_epoch_seconds_parses_rfc3339_to_seconds() {
        let secs = timestamp_to_epoch_seconds(Some("2026-04-14T12:00:00Z"));
        assert!(secs > 1_700_000_000);
    }

    #[test]
    fn timestamp_to_epoch_seconds_returns_zero_for_unparseable() {
        assert_eq!(timestamp_to_epoch_seconds(None), 0);
        assert_eq!(timestamp_to_epoch_seconds(Some("")), 0);
        assert_eq!(timestamp_to_epoch_seconds(Some("garbage")), 0);
    }

    #[test]
    fn persist_envelope_inserts_row_when_canonical() {
        let (_tmp, conn) = fresh_conn();
        let line = r#"{"type":"event","method":"agent_event","params":{"pane_id":"3","event":"agent.task.completed","timestamp":"2026-04-14T12:00:00Z","agent":"opencode","task_id":"abc","exit_code":0}}"#;
        let out = persist_envelope(&conn, line);
        match out {
            PersistOutcome::Inserted {
                rowid,
                event_type,
                pane_id,
                ..
            } => {
                assert!(rowid > 0);
                assert_eq!(event_type, "agent.task.completed");
                assert_eq!(pane_id.as_deref(), Some("3"));
            }
            other => panic!("expected Inserted, got {other:?}"),
        }
        let tail = agent_events::agent_session_events_tail(&conn, 10).expect("tail");
        assert_eq!(tail.len(), 1);
        let row = &tail[0];
        assert_eq!(row.event_type, "agent.task.completed");
        assert_eq!(row.agent, "opencode");
        assert_eq!(row.task_id.as_deref(), Some("abc"));
        assert!(row.payload_json.contains("task_id"));
    }

    #[test]
    fn persist_envelope_drops_lines_that_fail_parse() {
        let (_tmp, conn) = fresh_conn();
        let bad = r#"{"type":"event","method":"agent_event","params":{}}"#;
        let out = persist_envelope(&conn, bad);
        match out {
            PersistOutcome::Dropped { reason } => {
                assert_eq!(reason, ParseEnvelopeError::MissingEvent)
            }
            other => panic!("expected Dropped, got {other:?}"),
        }
        let tail = agent_events::agent_session_events_tail(&conn, 10).expect("tail");
        assert!(tail.is_empty());
    }

    #[test]
    fn spawn_listener_stub_returns_predictable_reason() {
        let err = spawn_listener().unwrap_err();
        assert!(
            matches!(
                err,
                SpawnError::UnsupportedPlatform
                    | SpawnError::NoWtComClsid
                    | SpawnError::StubNotWired
            ),
            "stub must return a typed SpawnError, got {err:?}"
        );
    }

    #[test]
    fn agent_event_params_serde_round_trips() {
        let params = AgentEventParams {
            pane_id: Some("3".into()),
            event: "agent.idle".into(),
            timestamp: None,
            agent: Some("opencode".into()),
            task_id: None,
        };
        let json = serde_json::to_string(&params).expect("ser");
        let back: AgentEventParams = serde_json::from_str(&json).expect("de");
        assert_eq!(back, params);
    }
}
