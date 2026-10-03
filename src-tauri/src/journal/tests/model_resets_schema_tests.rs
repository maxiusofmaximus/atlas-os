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
fn m19_advances_schema_version_to_at_least_19() {
    let (_tmp, conn) = fresh_conn();
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(v >= 19, "expected schema version >= 19 after M19, got {v}");
}

#[test]
fn m19_creates_model_resets_table() {
    let (_tmp, conn) = fresh_conn();
    let exists: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='model_resets'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(exists, 1, "model_resets table should exist after M19");
}

#[test]
fn m19_unique_constraint_dedupes_same_provider_model_resets_at() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
            "INSERT INTO model_resets (provider, model, status_code, error_type, resets_at, request_id)
             VALUES ('anthropic', 'claude-3-5-sonnet', 429, 'rate_limit', 1_700_000_000_000, 'req_1')",
            [],
        )
        .expect("first insert");
    let dup = conn.execute(
            "INSERT INTO model_resets (provider, model, status_code, error_type, resets_at, request_id)
             VALUES ('anthropic', 'claude-3-5-sonnet', 429, 'rate_limit', 1_700_000_000_000, 'req_2')",
            [],
        );
    assert!(
        dup.is_err(),
        "duplicate (provider, model, resets_at) should be rejected by UNIQUE index"
    );
}

#[test]
fn m19_roundtrips_all_columns() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
            "INSERT INTO model_resets
                (provider, model, status_code, error_type, resets_at, request_id, observed_at, toast_id, toast_dismissed_at)
             VALUES ('openai', 'gpt-5', 402, 'spend_limit', 1_700_000_001_000, 'req_3', 1_700_000_000_500, 42, 1_700_000_002_000)",
            [],
        )
        .expect("insert full row");
    let (
            provider,
            model,
            status_code,
            error_type,
            resets_at,
            request_id,
            observed_at,
            toast_id,
            toast_dismissed_at,
        ): (String, String, i64, String, i64, String, i64, i64, i64) = conn
            .query_row(
                "SELECT provider, model, status_code, error_type, resets_at, request_id, observed_at, toast_id, toast_dismissed_at
                 FROM model_resets WHERE provider = 'openai'",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                    ))
                },
            )
            .expect("query");
    assert_eq!(provider, "openai");
    assert_eq!(model, "gpt-5");
    assert_eq!(status_code, 402);
    assert_eq!(error_type, "spend_limit");
    assert_eq!(resets_at, 1_700_000_001_000);
    assert_eq!(request_id, "req_3");
    assert_eq!(observed_at, 1_700_000_000_500);
    assert_eq!(toast_id, 42);
    assert_eq!(toast_dismissed_at, 1_700_000_002_000);
}

#[test]
fn m19_accepts_nullable_error_type_and_request_id() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO model_resets (provider, model, status_code, error_type, resets_at, request_id)
             VALUES ('omniroute', 'claude-3-5-sonnet', 429, NULL, 1_700_000_000_000, NULL)",
        [],
    )
    .expect("insert with NULL error_type/request_id");
    let n: i64 = conn
        .query_row("SELECT count(*) FROM model_resets", [], |r| r.get(0))
        .expect("query");
    assert_eq!(n, 1);
}

#[test]
fn m19_pending_partial_index_lists_undismissed_rows() {
    // The `model_resets_pending_idx` partial index covers rows
    // where `toast_dismissed_at IS NULL` — the scheduler's working
    // set. Sanity-check that rows with `toast_dismissed_at = NULL`
    // are listed by the index (alias via SELECT FROM the table with
    // same filter).
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO model_resets (provider, model, status_code, error_type, resets_at)
             VALUES ('anthropic', 'claude-3-5-sonnet', 429, 'rate_limit', 1_700_000_000_000)",
        [],
    )
    .expect("pending row");
    conn.execute(
            "INSERT INTO model_resets (provider, model, status_code, error_type, resets_at, toast_dismissed_at)
             VALUES ('openai', 'gpt-5', 402, 'spend_limit', 1_700_000_001_000, 1_800_000_000_000)",
            [],
        )
        .expect("dismissed row");
    let pending_count: i64 = conn
        .query_row(
            "SELECT count(*) FROM model_resets WHERE toast_dismissed_at IS NULL",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(
        pending_count, 1,
        "only the undismissed row should be pending"
    );
}

#[test]
fn m22_advances_schema_version_to_at_least_22() {
    let (_tmp, conn) = fresh_conn();
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(v >= 22, "expected schema version >= 22 after M22, got {v}");
}

#[test]
fn m22_creates_reflection_episodes_table() {
    let (_tmp, conn) = fresh_conn();
    let exists: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='reflection_episodes'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(
        exists, 1,
        "reflection_episodes table should exist after M22"
    );
}

#[test]
fn m22_creates_council_votes_table() {
    let (_tmp, conn) = fresh_conn();
    let exists: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='council_votes'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(exists, 1, "council_votes table should exist after M22");
}

#[test]
fn m22_reflection_episodes_rejects_attempt_no_above_3() {
    let (_tmp, conn) = fresh_conn();
    let res = conn.execute(
            "INSERT INTO reflection_episodes (episode_id, mission_id, attempt_no, executor_model,
                reflexor_model, failure_signal, verbal_reflection, injected_prompt_delta, created_at)
             VALUES ('ep1', 'm1', 4, 'gpt-4o', 'haiku', 'sig', 'reflection', 'delta', '2026-01-01T00:00:00Z')",
            [],
        );
    assert!(
        res.is_err(),
        "attempt_no=4 must be rejected by the CHECK constraint"
    );
}

#[test]
fn m22_reflection_episodes_unique_mission_attempt() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
            "INSERT INTO reflection_episodes (episode_id, mission_id, attempt_no, executor_model,
                reflexor_model, failure_signal, verbal_reflection, injected_prompt_delta, created_at)
             VALUES ('ep1', 'm1', 1, 'gpt-4o', 'haiku', 'sig', 'r1', 'd1', '2026-01-01T00:00:00Z')",
            [],
        )
        .expect("first insert ok");
    let dup = conn.execute(
            "INSERT INTO reflection_episodes (episode_id, mission_id, attempt_no, executor_model,
                reflexor_model, failure_signal, verbal_reflection, injected_prompt_delta, created_at)
             VALUES ('ep2', 'm1', 1, 'gpt-4o', 'haiku', 'sig2', 'r2', 'd2', '2026-01-01T01:00:00Z')",
            [],
        );
    assert!(
        dup.is_err(),
        "duplicate (mission_id, attempt_no) must be rejected by the UNIQUE constraint"
    );
}

#[test]
fn m23_advances_schema_version_to_at_least_23() {
    let (_tmp, conn) = fresh_conn();
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(
        v >= crate::journal::schema::CURRENT_SCHEMA_VERSION,
        "expected schema version >= CURRENT_SCHEMA_VERSION after migrate(), got {v}"
    );
}

#[test]
fn m23_creates_task_classifier_decisions_table() {
    let (_tmp, conn) = fresh_conn();
    let exists: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='task_classifier_decisions'",
                [],
                |r| r.get(0),
            )
            .expect("query");
    assert_eq!(
        exists, 1,
        "task_classifier_decisions table should exist after M23"
    );
}

#[test]
fn m23_creates_model_affinity_cache_table() {
    let (_tmp, conn) = fresh_conn();
    let exists: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='model_affinity_cache'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(
        exists, 1,
        "model_affinity_cache table should exist after M23"
    );
}

#[test]
fn m23_task_classifier_decisions_rejects_unknown_kind() {
    let (_tmp, conn) = fresh_conn();
    let res = conn.execute(
        "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                confidence, classifier_kind, created_at)
             VALUES ('d1', 'h1', 'coding', 0.9, 'unknown_kind', '2026-01-01T00:00:00Z')",
        [],
    );
    assert!(
        res.is_err(),
        "classifier_kind='unknown_kind' must be rejected by the CHECK constraint"
    );
}

#[test]
fn m23_task_classifier_decisions_dedupes_prompt_hash_per_kind() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                confidence, classifier_kind, created_at)
             VALUES ('d1', 'h1', 'coding', 0.9, 'lexical', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("first insert ok");
    let dup = conn.execute(
        "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                confidence, classifier_kind, created_at)
             VALUES ('d2', 'h1', 'plan', 0.8, 'lexical', '2026-01-02T00:00:00Z')",
        [],
    );
    assert!(
        dup.is_err(),
        "duplicate (prompt_hash, classifier_kind) must be rejected by the UNIQUE constraint"
    );
    // Different classifier_kind on same prompt_hash is allowed.
    conn.execute(
        "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                confidence, classifier_kind, created_at)
             VALUES ('d3', 'h1', 'plan', 0.85, 'logreg', '2026-01-03T00:00:00Z')",
        [],
    )
    .expect("different kind allowed for same prompt_hash");
}

#[test]
fn m23_model_affinity_cache_upsert_replaces_existing_row() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO model_affinity_cache (task_type, model_id, success_rate, p95_latency_ms,
                mean_cost_usd, n_samples, updated_at)
              VALUES ('coding', 'gpt-5', 0.85, 1200, 0.01, 12, '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("insert ok");
    conn.execute(
        "INSERT OR REPLACE INTO model_affinity_cache (task_type, model_id, success_rate,
                p95_latency_ms, mean_cost_usd, n_samples, updated_at)
              VALUES ('coding', 'gpt-5', 0.9, 1100, 0.011, 18, '2026-01-02T00:00:00Z')",
        [],
    )
    .expect("upsert ok");
    let (rate, n): (f64, i64) = conn
        .query_row(
            "SELECT success_rate, n_samples FROM model_affinity_cache
                 WHERE task_type='coding' AND model_id='gpt-5'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("query");
    assert!(
        (rate - 0.9).abs() < 1e-6,
        "upsert should overwrite success_rate"
    );
    assert_eq!(n, 18, "upsert should overwrite n_samples");
}

#[test]
fn m33_task_classifier_decisions_accepts_laya_kind() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                confidence, classifier_kind, created_at)
             VALUES ('l1', 'hlaya', 'coding', 0.9, 'laya', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("classifier_kind='laya' must be accepted after M33");
    let kind: String = conn
        .query_row(
            "SELECT classifier_kind FROM task_classifier_decisions WHERE id='l1'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(kind, "laya");
}

#[test]
fn m33_task_classifier_decisions_still_rejects_unknown_kind() {
    let (_tmp, conn) = fresh_conn();
    let res = conn.execute(
        "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                confidence, classifier_kind, created_at)
             VALUES ('x1', 'hx', 'coding', 0.9, 'jev', '2026-01-01T00:00:00Z')",
        [],
    );
    assert!(
        res.is_err(),
        "classifier_kind='jev' must still be rejected by the CHECK constraint"
    );
}

#[test]
fn m33_task_classifier_decisions_preserves_preexisting_rows() {
    let (_tmp, conn) = fresh_conn();
    for (id, kind) in [
        ('a', "lexical"),
        ('b', "logreg"),
        ('c', "embedding"),
        ('d', "main"),
        ('e', "mf_ab"),
    ] {
        conn.execute(
            "INSERT INTO task_classifier_decisions (id, prompt_hash, predicted_task_type,
                    confidence, classifier_kind, created_at)
                 VALUES (?1, ?2, 'coding', 0.8, ?3, '2026-01-01T00:00:00Z')",
            rusqlite::params![format!("p{id}"), format!("h{id}"), kind],
        )
        .expect("pre-existing kind insert ok");
    }
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM task_classifier_decisions", [], |r| {
            r.get(0)
        })
        .expect("query");
    assert_eq!(
        n, 5,
        "all pre-M33 kinds must survive the M33 recreate-and-copy"
    );
}
