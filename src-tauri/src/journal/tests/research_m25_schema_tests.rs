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
fn m25_advances_schema_version_to_25() {
    let (_tmp, conn) = fresh_conn();
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(
        v >= crate::journal::schema::CURRENT_SCHEMA_VERSION,
        "expected schema version >= CURRENT_SCHEMA_VERSION after migrate(), got {v}"
    );
    assert!(v >= 25, "M25 must have run, got {v}");
}

#[test]
fn m25_migration_is_idempotent() {
    let (tmp, _) = fresh_conn();
    let db_path = tmp.path().join("journal.db");
    let conn = Connection::open(&db_path).expect("reopen");
    crate::journal::schema::migrate(&conn).expect("second migrate ok");
    crate::journal::schema::migrate(&conn).expect("third migrate ok");
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(v >= 25, "version must stay >= 25 after re-migrate, got {v}");
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table'
                 AND name IN ('research_sources','research_consensus')",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(count, 2, "both M25 tables must exist after re-migrate");
}

#[test]
fn m25_extends_research_runs_with_status_and_recommended() {
    let (_tmp, conn) = fresh_conn();
    let cols: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('research_runs')")
        .expect("prepare")
        .query_map([], |r| r.get(0))
        .expect("query")
        .flatten()
        .collect();
    for expected in ["id", "query", "status", "confidence", "recommended"] {
        assert!(
            cols.contains(&expected.to_string()),
            "research_runs must carry column {expected}, got {cols:?}"
        );
    }
}

#[test]
fn m25_consensus_rejects_unknown_dimension() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
            "INSERT INTO research_runs (id, query, created_at) VALUES ('rr-x','q','2026-01-01T00:00:00Z')",
            [],
        )
        .expect("seed run");
    let res = conn.execute(
        "INSERT INTO research_consensus (run_id, dimension, score) VALUES ('rr-x','press',50.0)",
        [],
    );
    assert!(
        res.is_err(),
        "dimension='press' must be rejected by the CHECK constraint"
    );
}

#[test]
fn m25_consensus_rejects_score_above_100() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
            "INSERT INTO research_runs (id, query, created_at) VALUES ('rr-y','q','2026-01-01T00:00:00Z')",
            [],
        )
        .expect("seed run");
    let res = conn.execute(
        "INSERT INTO research_consensus (run_id, dimension, score)
             VALUES ('rr-y','official',120.0)",
        [],
    );
    assert!(
        res.is_err(),
        "score=120 must be rejected by the 0..100 CHECK constraint"
    );
}
