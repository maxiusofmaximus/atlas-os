use rusqlite::Connection;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AstSymbolRow {
    pub id: String,
    pub file: String,
    pub name: String,
    pub kind: String,
    pub line: i64,
    pub lang: String,
    pub extracted_at: String,
}

pub fn upsert_ast_symbol(
    conn: &Connection,
    file: &str,
    name: &str,
    kind: &str,
    line: i64,
    lang: &str,
) -> anyhow::Result<String> {
    if name.trim().is_empty() {
        anyhow::bail!("ast symbol name is empty");
    }
    if line <= 0 {
        anyhow::bail!("ast symbol line must be > 0");
    }
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO ast_symbols
            (id, file, name, kind, line, lang, extracted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![id, file, name, kind, line, lang, now],
    )?;
    let resolved: String = conn.query_row(
        "SELECT id FROM ast_symbols
          WHERE file = ?1 AND name = ?2 AND kind = ?3 AND line = ?4",
        rusqlite::params![file, name, kind, line],
        |r| r.get(0),
    )?;
    Ok(resolved)
}

pub fn ast_symbols_for_file(conn: &Connection, file: &str) -> anyhow::Result<Vec<AstSymbolRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, file, name, kind, line, lang, extracted_at
           FROM ast_symbols
          WHERE file = ?1
          ORDER BY line ASC, name ASC",
    )?;
    let rows = stmt
        .query_map([file], |r| {
            Ok(AstSymbolRow {
                id: r.get(0)?,
                file: r.get(1)?,
                name: r.get(2)?,
                kind: r.get(3)?,
                line: r.get(4)?,
                lang: r.get(5)?,
                extracted_at: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn count_ast_symbols(conn: &Connection) -> anyhow::Result<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM ast_symbols", [], |r| r.get(0))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::Journal;

    fn fresh_conn() -> (tempfile::TempDir, Connection) {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let _journal = Journal::open(tmp.path()).expect("open runs migrate");
        let conn = Connection::open(tmp.path().join("journal.db")).expect("open raw conn");
        (tmp, conn)
    }

    #[test]
    fn m34_creates_ast_symbols_table() {
        let (_tmp, conn) = fresh_conn();
        let n: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='ast_symbols'",
                [],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(n, 1, "ast_symbols table should exist after M34");
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(v >= 33, "M34 must have run, got {v}");
    }

    #[test]
    fn m34_migration_is_idempotent() {
        let (tmp, _) = fresh_conn();
        let db_path = tmp.path().join("journal.db");
        let conn = Connection::open(&db_path).expect("reopen");
        crate::journal::schema::migrate(&conn).expect("second migrate ok");
        let v: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
            .expect("query");
        assert!(
            v >= crate::journal::schema::CURRENT_SCHEMA_VERSION,
            "expected schema version >= CURRENT_SCHEMA_VERSION after migrate(), got {v}"
        );
    }

    #[test]
    fn upsert_dedupes_on_file_name_kind_line() {
        let (_tmp, conn) = fresh_conn();
        let a = upsert_ast_symbol(&conn, "lib.rs", "alpha", "function", 3, "rust").expect("upsert");
        let b = upsert_ast_symbol(&conn, "lib.rs", "alpha", "function", 3, "rust").expect("upsert");
        assert_eq!(a, b, "replay must resolve to the existing row id");
        assert_eq!(count_ast_symbols(&conn).expect("count"), 1);
    }

    #[test]
    fn upsert_rejects_empty_name_and_zero_line() {
        let (_tmp, conn) = fresh_conn();
        assert!(upsert_ast_symbol(&conn, "lib.rs", "  ", "function", 1, "rust").is_err());
        assert!(upsert_ast_symbol(&conn, "lib.rs", "alpha", "function", 0, "rust").is_err());
    }

    #[test]
    fn upsert_rejects_unknown_kind() {
        let (_tmp, conn) = fresh_conn();
        let res = upsert_ast_symbol(&conn, "lib.rs", "alpha", "macro", 1, "rust");
        assert!(
            res.is_err(),
            "kind='macro' must be rejected by the CHECK constraint"
        );
    }

    #[test]
    fn for_file_returns_line_ordered_rows() {
        let (_tmp, conn) = fresh_conn();
        upsert_ast_symbol(&conn, "lib.rs", "zeta", "function", 30, "rust").expect("upsert");
        upsert_ast_symbol(&conn, "lib.rs", "alpha", "struct", 3, "rust").expect("upsert");
        upsert_ast_symbol(&conn, "other.rs", "beta", "enum", 1, "rust").expect("upsert");
        let rows = ast_symbols_for_file(&conn, "lib.rs").expect("query");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "alpha");
        assert_eq!(rows[1].name, "zeta");
        assert!(ast_symbols_for_file(&conn, "missing.rs")
            .expect("query")
            .is_empty());
    }
}
