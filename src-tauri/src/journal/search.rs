// Atlas OS — full-text search over `journal_events` (research 36
// sub-fase 8.0, RFC 35 §4 context-mode pattern). FTS5 virtual table
// `journal_events_fts` (M31, content-synced via triggers) served with
// BM25 `rank` ordering; any FTS5 failure (missing module, bad MATCH
// syntax from operator input) falls back to LIKE so search never errors.

use anyhow::Result;
use rusqlite::Connection;

use super::store::JournalEntry;

fn fts_table_present(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'journal_events_fts'",
        [],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n > 0)
    .unwrap_or(false)
}

/// Build a safe FTS5 MATCH expression from raw operator input: keep
/// alphanumeric-led tokens, drop FTS5 operators, phrase-quote each and
/// OR them. Returns `None` when nothing searchable remains (caller
/// falls back to LIKE).
fn match_expr(query: &str) -> Option<String> {
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(|t| {
            t.trim_matches(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
                .to_string()
        })
        .filter(|t| t.chars().any(|c| c.is_alphanumeric()))
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect();
    if tokens.is_empty() {
        return None;
    }
    Some(tokens.join(" OR "))
}

fn row_from(row: &rusqlite::Row) -> rusqlite::Result<JournalEntry> {
    Ok(JournalEntry {
        id: row.get(0)?,
        ts: row.get(1)?,
        kind: row.get(2)?,
        payload: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
    })
}

fn search_fts(conn: &Connection, expr: &str, limit: i64) -> Result<Vec<JournalEntry>> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.ts, e.kind, e.payload
         FROM journal_events_fts f
         JOIN journal_events e ON e.id = f.rowid
         WHERE journal_events_fts MATCH ?1
         ORDER BY rank LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![expr, limit], row_from)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn search_like(conn: &Connection, query: &str, limit: i64) -> Result<Vec<JournalEntry>> {
    let like = format!("%{query}%");
    let mut stmt = conn.prepare(
        "SELECT id, ts, kind, payload FROM journal_events
         WHERE kind LIKE ?1 OR payload LIKE ?1 OR event_id LIKE ?1
         ORDER BY id DESC LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![like, limit], row_from)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Full-text search over `journal_events`, newest-relevant first via
/// BM25 when FTS5 is available, LIKE fallback otherwise. Never fails
/// on operator input: unparseable MATCH falls back to LIKE.
pub fn search_events(conn: &Connection, query: &str, limit: i64) -> Result<Vec<JournalEntry>> {
    let limit = limit.clamp(1, 200);
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    if fts_table_present(conn) {
        if let Some(expr) = match_expr(query) {
            if let Ok(rows) = search_fts(conn, &expr, limit) {
                return Ok(rows);
            }
        }
    }
    search_like(conn, query.trim(), limit)
}

#[cfg(test)]
mod search_expr_tests {
    use super::*;

    #[test]
    fn tokens_are_phrase_quoted_and_ored() {
        assert_eq!(
            match_expr("hello world").as_deref(),
            Some("\"hello\" OR \"world\"")
        );
    }

    #[test]
    fn pure_operators_yield_none_so_caller_uses_like() {
        assert!(match_expr("*** (((").is_none());
        assert!(match_expr("   ").is_none());
    }
}
