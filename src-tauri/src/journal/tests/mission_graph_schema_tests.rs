use crate::journal::Journal;
use rusqlite::Connection;
use tempfile::TempDir;

fn fresh_conn() -> (TempDir, Connection) {
    let tmp = TempDir::new().expect("tmp");
    let _journal = Journal::open(tmp.path()).expect("open runs migrate");
    let conn = Connection::open(tmp.path().join("journal.db")).expect("open raw conn");
    (tmp, conn)
}

#[test]
fn m15_advances_schema_version_to_15() {
    let (_tmp, conn) = fresh_conn();
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(
        v >= 15,
        "expected schema version >= 15 (M16 runs after M15), got {v}"
    );
}

#[test]
fn m15_creates_mission_graph_nodes_and_edges_and_learning_graphs() {
    let (_tmp, conn) = fresh_conn();
    for table in [
        "mission_graph_nodes",
        "mission_graph_edges",
        "learning_graphs",
    ] {
        let exists: i64 = conn
            .query_row(
                &format!(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='{table}'"
                ),
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(exists, 1, "{table} should exist after M15");
    }
}

#[test]
fn m15_node_kind_check_rejects_unknown_value() {
    let (_tmp, conn) = fresh_conn();
    // No missions row exists so we expect FK violation BEFORE the
    // CHECK — guard around the FK first by inserting a mission.
    conn.execute(
            "INSERT INTO missions (id, label, status, created_at, updated_at)
             VALUES ('00000000-0000-0000-0000-000000000001', 'l', 'received', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
            [],
        )
        .expect("seed mission");
    let err = conn
            .execute(
                "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance) \
                 VALUES ('n1', '00000000-0000-0000-0000-000000000001', 'unknown_kind', 'x', 'EXTRACTED')",
                [],
            )
            .expect_err("CHECK should reject");
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("constraint") || msg.to_lowercase().contains("check"),
        "expected CHECK constraint failure, got: {msg}"
    );
}

#[test]
fn m15_provenance_check_accepts_extracted_inferred_ambiguous() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
            "INSERT INTO missions (id, label, status, created_at, updated_at)
             VALUES ('00000000-0000-0000-0000-000000000001', 'l', 'received', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
            [],
        )
        .expect("seed mission");
    for (idx, p) in ["EXTRACTED", "INFERRED", "AMBIGUOUS"].iter().enumerate() {
        conn.execute(
            "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance) \
                 VALUES (?1, '00000000-0000-0000-0000-000000000001', 'mission', ?2, ?3)",
            rusqlite::params![format!("n{idx}"), format!("lbl{idx}"), p],
        )
        .expect("valid provenance");
    }
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM mission_graph_nodes WHERE mission_id = ?1",
            rusqlite::params!["00000000-0000-0000-0000-000000000001"],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(count, 3);
}

#[test]
fn m15_edge_kind_check_rejects_unknown_value() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
            "INSERT INTO missions (id, label, status, created_at, updated_at)
             VALUES ('00000000-0000-0000-0000-000000000001', 'l', 'received', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
            [],
        )
        .expect("seed mission");
    conn.execute(
        "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance) \
             VALUES ('n1', '00000000-0000-0000-0000-000000000001', 'mission', 'a', 'EXTRACTED'),
                    ('n2', '00000000-0000-0000-0000-000000000001', 'mission', 'b', 'EXTRACTED')",
        [],
    )
    .expect("seed nodes");
    let err = conn
        .execute(
            "INSERT INTO mission_graph_edges (id, mission_id, src, dst, kind) \
                 VALUES ('e1', '00000000-0000-0000-0000-000000000001', 'n1', 'n2', 'teleports_to')",
            [],
        )
        .expect_err("CHECK should reject unknown edge kind");
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("constraint") || msg.to_lowercase().contains("check"),
        "expected CHECK constraint failure, got: {msg}"
    );
}

#[test]
fn m15_edge_visit_count_defaults_to_zero() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
            "INSERT INTO missions (id, label, status, created_at, updated_at)
             VALUES ('00000000-0000-0000-0000-000000000001', 'l', 'received', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
            [],
        )
        .expect("seed mission");
    conn.execute(
        "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance) \
             VALUES ('n1', '00000000-0000-0000-0000-000000000001', 'mission', 'a', 'EXTRACTED'),
                    ('n2', '00000000-0000-0000-0000-000000000001', 'mission', 'b', 'EXTRACTED')",
        [],
    )
    .expect("seed nodes");
    conn.execute(
        "INSERT INTO mission_graph_edges (id, mission_id, src, dst, kind) \
             VALUES ('e1', '00000000-0000-0000-0000-000000000001', 'n1', 'n2', 'calls')",
        [],
    )
    .expect("insert edge without visit_count");
    let count: i64 = conn
        .query_row(
            "SELECT visit_count FROM mission_graph_edges WHERE id = 'e1'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(count, 0);
}

#[test]
fn m15_learning_graphs_accepts_zero_one_success() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO learning_graphs (id, intent_signature, success, graph_json, created_ts) \
             VALUES ('g0', 'sig-0', 0, '{}'            , 100), \
                    ('g1', 'sig-1', 1, '{\"a\":1}'     , 200)",
        [],
    )
    .expect("insert both success variants");
    let err = conn
        .execute(
            "INSERT INTO learning_graphs (id, intent_signature, success, graph_json, created_ts) \
                 VALUES ('g2', 'sig-2', 2, '{}', 300)",
            [],
        )
        .expect_err("CHECK should reject success=2");
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("constraint") || msg.to_lowercase().contains("check"),
        "expected CHECK constraint failure, got: {msg}"
    );
}

#[test]
fn m15_cascade_delete_removes_edges_and_nodes_children() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
            "INSERT INTO missions (id, label, status, created_at, updated_at)
             VALUES ('00000000-0000-0000-0000-000000000001', 'l', 'received', '2026-07-26T00:00:00Z', '2026-07-26T00:00:00Z')",
            [],
        )
        .expect("seed mission");
    conn.execute(
        "INSERT INTO mission_graph_nodes (id, mission_id, kind, label, provenance) \
             VALUES ('n1', '00000000-0000-0000-0000-000000000001', 'mission', 'a', 'EXTRACTED'), \
                    ('n2', '00000000-0000-0000-0000-000000000001', 'mission', 'b', 'EXTRACTED')",
        [],
    )
    .expect("seed nodes");
    conn.execute(
        "INSERT INTO mission_graph_edges (id, mission_id, src, dst, kind) \
             VALUES ('e1', '00000000-0000-0000-0000-000000000001', 'n1', 'n2', 'calls')",
        [],
    )
    .expect("seed edge");
    conn.execute(
        "DELETE FROM missions WHERE id = '00000000-0000-0000-0000-000000000001'",
        [],
    )
    .expect("delete mission");
    let nodes: i64 = conn
        .query_row("SELECT count(*) FROM mission_graph_nodes", [], |r| r.get(0))
        .expect("query");
    let edges: i64 = conn
        .query_row("SELECT count(*) FROM mission_graph_edges", [], |r| r.get(0))
        .expect("query");
    assert_eq!(nodes, 0);
    assert_eq!(edges, 0);
}

#[test]
fn m16_advances_schema_version_to_at_least_16() {
    let (_tmp, conn) = fresh_conn();
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(v >= 16, "expected schema version >= 16 after M16, got {v}");
}

#[test]
fn m17_advances_schema_version_to_at_least_17() {
    // M17 (toast_queue + toast_history) runs unconditionally — the
    // tables exist whether or not the `toast` feature is on. This
    // keeps the schema idempotent across feature combos.
    let (_tmp, conn) = fresh_conn();
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(v >= 17, "expected schema version >= 17 after M17, got {v}");
}

#[test]
fn m17_creates_toast_queue_and_history_tables() {
    let (_tmp, conn) = fresh_conn();
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .expect("prepare")
        .query_map([], |r| r.get(0))
        .expect("query_map")
        .map(Result::unwrap)
        .collect();
    assert!(
        tables.iter().any(|t| t == "toast_queue"),
        "expected `toast_queue` table after M17, got: {tables:?}"
    );
    assert!(
        tables.iter().any(|t| t == "toast_history"),
        "expected `toast_history` table after M17, got: {tables:?}"
    );
}

#[test]
fn m16_adds_embedding_and_emb_model_columns_to_learning_graphs() {
    let (_tmp, conn) = fresh_conn();
    let cols: Vec<String> = conn
        .prepare("PRAGMA table_info(learning_graphs)")
        .expect("prepare")
        .query_map([], |r| {
            let name: String = r.get(1)?;
            Ok(name)
        })
        .expect("query_map")
        .map(Result::unwrap)
        .collect();
    assert!(
        cols.iter().any(|c| c == "embedding"),
        "expected `embedding` column after M16, got: {cols:?}"
    );
    assert!(
        cols.iter().any(|c| c == "emb_model"),
        "expected `emb_model` column after M16, got: {cols:?}"
    );
}

#[test]
fn m16_learning_graphs_accepts_nullable_embedding_and_emb_model() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO learning_graphs (id, intent_signature, success, graph_json, created_ts)
             VALUES ('g0', 'sig', 1, '{}', 1)",
        [],
    )
    .expect("insert without embedding should succeed (nullable)");
    let n: i64 = conn
        .query_row(
            "SELECT count(*) FROM learning_graphs WHERE embedding IS NULL AND emb_model IS NULL",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(n, 1);
}

#[test]
fn m16_learning_graphs_accepts_dense_embedding_blob() {
    let (_tmp, conn) = fresh_conn();
    let emb: [u8; 8] = [
        0x00, 0x00, 0x80, 0x3f, // 1.0f32 LE
        0x00, 0x00, 0x00, 0x40, // 2.0f32 LE
    ];
    conn.execute(
            "INSERT INTO learning_graphs (id, intent_signature, success, graph_json, created_ts, embedding, emb_model)
             VALUES ('g1', 'sig', 1, '{}', 1, ?1, 'fastembed-bge-small')",
            rusqlite::params![emb.to_vec()],
        )
        .expect("insert with embedding blob");
    let model: String = conn
        .query_row(
            "SELECT emb_model FROM learning_graphs WHERE id = 'g1'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(model, "fastembed-bge-small");
}

#[test]
fn m14_advances_schema_version_to_at_least_14() {
    let (_tmp, conn) = fresh_conn();
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(
        v >= 14,
        "expected schema version >= 14 (M14 runs before M15/M16), got {v}"
    );
}

#[test]
fn m14_creates_agent_session_events_table() {
    let (_tmp, conn) = fresh_conn();
    let exists: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='agent_session_events'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(exists, 1, "agent_session_events should exist after M14");
}

#[test]
fn m14_creates_ts_and_task_indexes() {
    let (_tmp, conn) = fresh_conn();
    for idx in ["idx_ase_ts", "idx_ase_task"] {
        let exists: i64 = conn
            .query_row(
                &format!("SELECT count(*) FROM sqlite_master WHERE type='index' AND name='{idx}'"),
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(exists, 1, "index {idx} should exist after M14");
    }
}

#[test]
fn m14_accepts_nullable_pane_id_and_task_id() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO agent_session_events
                (ts, pane_id, event_type, agent, task_id, payload_json)
             VALUES (1722000000, NULL, 'agent.idle', 'opencode', NULL, '{}')",
        [],
    )
    .expect("insert with NULL pane_id/task_id");
    let n: i64 = conn
        .query_row("SELECT count(*) FROM agent_session_events", [], |r| {
            r.get(0)
        })
        .expect("query");
    assert_eq!(n, 1);
}

#[test]
fn m14_roundtrips_envelope_payload_json() {
    let (_tmp, conn) = fresh_conn();
    let payload = r#"{"event":"agent.task.completed","hud_url":"http://127.0.0.1:57457"}"#;
    conn.execute(
        "INSERT INTO agent_session_events
                (ts, pane_id, event_type, agent, task_id, payload_json)
             VALUES (1722000001, 'pane-3', 'agent.task.completed', 'opencode', 'm-1', ?1)",
        rusqlite::params![payload],
    )
    .expect("insert");
    let stored: String = conn
        .query_row(
            "SELECT payload_json FROM agent_session_events WHERE ts = 1722000001",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(stored, payload);
}

#[test]
fn m18_advances_schema_version_to_at_least_18() {
    // M18 (calendar_busy_windows + calendar_auth) runs
    // unconditionally — the tables exist whether or not the
    // `calendar-*` features are on. This keeps the schema
    // idempotent across feature combos, same as M17.
    let (_tmp, conn) = fresh_conn();
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(v >= 18, "expected schema version >= 18 after M18, got {v}");
}

#[test]
fn m18_creates_calendar_busy_windows_and_auth_tables() {
    let (_tmp, conn) = fresh_conn();
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .expect("prepare")
        .query_map([], |r| r.get(0))
        .expect("query_map")
        .map(Result::unwrap)
        .collect();
    assert!(
        tables.iter().any(|t| t == "calendar_busy_windows"),
        "expected `calendar_busy_windows` table after M18, got: {tables:?}"
    );
    assert!(
        tables.iter().any(|t| t == "calendar_auth"),
        "expected `calendar_auth` table after M18, got: {tables:?}"
    );
}

#[test]
fn m18_calendar_busy_windows_source_check_constraint_rejects_unknown() {
    let (_tmp, conn) = fresh_conn();
    let ok = conn.execute(
        "INSERT INTO calendar_busy_windows
                (source, external_id, subject, starts_at, ends_at, weight)
             VALUES ('graph', 'evt-1', 'Sprint planning', 1_000, 2_000, 1.0)",
        [],
    );
    assert!(ok.is_ok(), "graph source should be accepted: {:?}", ok);
    let err = conn.execute(
        "INSERT INTO calendar_busy_windows
                (source, external_id, subject, starts_at, ends_at, weight)
             VALUES ('unknown', 'evt-2', 'Bad source', 1_000, 2_000, 1.0)",
        [],
    );
    assert!(
        err.is_err(),
        "unknown source should be rejected by CHECK constraint"
    );
}

#[test]
fn m18_calendar_busy_windows_unique_source_external_id_dedupe() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO calendar_busy_windows
                (source, external_id, subject, starts_at, ends_at, weight)
             VALUES ('graph', 'evt-7', 'First insert', 1_000, 2_000, 1.0)",
        [],
    )
    .expect("first insert");
    let dup = conn.execute(
        "INSERT INTO calendar_busy_windows
                (source, external_id, subject, starts_at, ends_at, weight)
             VALUES ('graph', 'evt-7', 'Dup insert', 3_000, 4_000, 1.0)",
        [],
    );
    assert!(
        dup.is_err(),
        "duplicate (source, external_id) should be rejected by UNIQUE constraint"
    );
}

#[test]
fn m18_calendar_busy_windows_accepts_soft_busy_weight() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO calendar_busy_windows
                (source, external_id, subject, starts_at, ends_at, weight)
             VALUES ('ics_local', 'mtg-soft', 'Maybe join', 5_000, 6_000, 0.5)",
        [],
    )
    .expect("insert with weight=0.5 (soft busy)");
    let w: f64 = conn
        .query_row(
            "SELECT weight FROM calendar_busy_windows WHERE external_id = 'mtg-soft'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert!((w - 0.5).abs() < 1e-6, "soft-busy weight should roundtrip");
}

#[test]
fn m18_calendar_auth_accepts_encrypted_token_blob() {
    let (_tmp, conn) = fresh_conn();
    let ciphertext: Vec<u8> = vec![0u8; 32];
    conn.execute(
        "INSERT INTO calendar_auth (account, token_ciphertext, key_hint)
             VALUES ('alice@contoso.com', ?1, 'host-key-1')",
        rusqlite::params![ciphertext],
    )
    .expect("insert");
    let stored_hint: String = conn
        .query_row(
            "SELECT key_hint FROM calendar_auth WHERE account = 'alice@contoso.com'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(stored_hint, "host-key-1");
}
