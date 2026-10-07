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
fn m52_advances_schema_version_to_current() {
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
fn m52_creates_task_annotations_table() {
    let (_tmp, conn) = fresh_conn();
    let exists: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='task_annotations'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(exists, 1, "task_annotations table should exist after M52");
}

#[test]
fn m52_roundtrips_all_columns() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO task_annotations (id, task_id, file_path, line_no, body, author, created_at)
         VALUES ('a1', 'mission-1', 'src/lib.rs', 7, 'comment', 'max', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("insert full row");
    let (task_id, file_path, line_no, body, author): (String, String, i64, String, String) = conn
        .query_row(
            "SELECT task_id, file_path, line_no, body, author FROM task_annotations WHERE id='a1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .expect("query");
    assert_eq!(task_id, "mission-1");
    assert_eq!(file_path, "src/lib.rs");
    assert_eq!(line_no, 7);
    assert_eq!(body, "comment");
    assert_eq!(author, "max");
}

#[test]
fn m52_accepts_nullable_file_path_and_line_no() {
    let (_tmp, conn) = fresh_conn();
    conn.execute(
        "INSERT INTO task_annotations (id, task_id, file_path, line_no, body, author, created_at)
         VALUES ('a2', 'mission-1', NULL, NULL, 'task-level comment', 'max', '2026-01-01T00:00:00Z')",
        [],
    )
    .expect("insert with NULL file_path/line_no");
    let n: i64 = conn
        .query_row("SELECT count(*) FROM task_annotations", [], |r| r.get(0))
        .expect("query");
    assert_eq!(n, 1);
}
