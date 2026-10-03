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
fn m27_advances_schema_version_to_27() {
    let (_tmp, conn) = fresh_conn();
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(
        v >= crate::journal::schema::CURRENT_SCHEMA_VERSION,
        "expected schema version >= CURRENT_SCHEMA_VERSION after migrate(), got {v}"
    );
    assert!(v >= 27, "M27 must have run, got {v}");
}

#[test]
fn m27_migration_is_idempotent() {
    let (tmp, _) = fresh_conn();
    let db_path = tmp.path().join("journal.db");
    let conn = Connection::open(&db_path).expect("reopen");
    crate::journal::schema::migrate(&conn).expect("second migrate ok");
    crate::journal::schema::migrate(&conn).expect("third migrate ok");
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(v >= 27, "version must stay >= 27 after re-migrate, got {v}");
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table'
                 AND name IN ('feasibility_cache')",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(
        count, 1,
        "M27 feasibility_cache must exist after re-migrate"
    );
}

#[test]
fn m27_cache_carries_probe_columns() {
    let (_tmp, conn) = fresh_conn();
    let cols: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('feasibility_cache')")
        .expect("prepare")
        .query_map([], |r| r.get(0))
        .expect("query")
        .flatten()
        .collect();
    for expected in ["cache_key", "topic", "domains", "report_json", "created_at"] {
        assert!(
            cols.contains(&expected.to_string()),
            "feasibility_cache must carry column {expected}, got {cols:?}"
        );
    }
}

#[test]
fn m27_cache_refresh_overwrites_same_key() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO feasibility_cache
                (cache_key, topic, domains, report_json, created_at)
             VALUES ('k','t','software','{}','2026-07-13T14:02:00+00:00')",
        [],
    )
    .expect("first insert");
    conn.execute(
        "INSERT OR REPLACE INTO feasibility_cache
                (cache_key, topic, domains, report_json, created_at)
             VALUES ('k','t','software','{\"found\":true}','2026-07-14T14:02:00+00:00')",
        [],
    )
    .expect("refresh");
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM feasibility_cache WHERE cache_key = 'k'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(count, 1, "cache refresh must replace, never duplicate");
}
