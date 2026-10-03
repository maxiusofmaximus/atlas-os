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
fn m26_advances_schema_version_to_26() {
    let (_tmp, conn) = fresh_conn();
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(
        v >= crate::journal::schema::CURRENT_SCHEMA_VERSION,
        "expected schema version >= CURRENT_SCHEMA_VERSION after migrate(), got {v}"
    );
    assert!(v >= 26, "M26 must have run, got {v}");
}

#[test]
fn m26_migration_is_idempotent() {
    let (tmp, _) = fresh_conn();
    let db_path = tmp.path().join("journal.db");
    let conn = Connection::open(&db_path).expect("reopen");
    crate::journal::schema::migrate(&conn).expect("second migrate ok");
    crate::journal::schema::migrate(&conn).expect("third migrate ok");
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(v >= 26, "version must stay >= 26 after re-migrate, got {v}");
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table'
                 AND name IN ('research_notes')",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(count, 1, "M26 research_notes must exist after re-migrate");
}

#[test]
fn m26_notes_carry_hands_on_columns() {
    let (_tmp, conn) = fresh_conn();
    let cols: Vec<String> = conn
        .prepare("SELECT name FROM pragma_table_info('research_notes')")
        .expect("prepare")
        .query_map([], |r| r.get(0))
        .expect("query")
        .flatten()
        .collect();
    for expected in [
        "id",
        "title",
        "project",
        "decision",
        "outcome",
        "confidence",
        "tags_json",
        "attached_at",
        "signature",
    ] {
        assert!(
            cols.contains(&expected.to_string()),
            "research_notes must carry column {expected}, got {cols:?}"
        );
    }
}

#[test]
fn m26_notes_reject_confidence_above_one() {
    let (_tmp, conn) = fresh_conn();
    let res = conn.execute(
        "INSERT INTO research_notes
                (id, title, decision, confidence, attached_at, signature, created_at)
             VALUES ('rn-x','t','d',1.5,'2026-07-04','op','2026-07-04T00:00:00Z')",
        [],
    );
    assert!(
        res.is_err(),
        "confidence=1.5 must be rejected by the 0..1 CHECK constraint"
    );
}
