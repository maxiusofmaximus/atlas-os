// Atlas OS — Learning graphs store (RFC 28 §C item 6, RFC 16 §3
// "Por grafo de misión exitoso"). Persists successful mission-graph
// instances and retrieves the top-k most similar ones by cosine over
// a `fastembed-rs`-derived `intent_signature` embedding.
//
// Strategy:
//   * With the optional `fastembed` feature enabled, callers supply a
//     dense `f32` embedding (length = model dim) and we store it
//     little-endian in the `embedding` BLOB plus the `emb_model` tag.
//   * Without `fastembed`, callers pass `None` for both; we still
//     persist the row (success/failure outcome) and retrieve falls
//     back to exact `intent_signature` match / recency.
//   * Cosine is computed in-process over the cached rows. sqlite-vec's
//     `vec0` virtual table is intentionally NOT depended on: RFC 25
//     §3.4 marks the extension load as best-effort, so a portable
//     in-memory scan keeps retrieval correct on every host. Phase 2
//     may swap to `vec0` once a bundled sqlite-vec is wired in.
//
// The whole module is gated behind `dag_mode` because the persisted
// graph payload is a `MissionGraph` produced by the Planner DAG
// emitter (item 4) — there is nothing to learn from fornon-DAG
// missions.

use rusqlite::Connection;

/// One persisted learning-graph row projected from `learning_graphs`.
#[derive(Clone, Debug, PartialEq)]
pub struct LearningGraphRow {
    pub id: String,
    pub intent_signature: String,
    pub success: bool,
    pub graph_json: String,
    pub created_ts: i64,
    /// None when persisted without an embedding (fastembed feature off
    /// or pre-M16 row). Decoded back to `Vec<f32>` for cosine.
    pub embedding: Option<Vec<f32>>,
    pub emb_model: Option<String>,
}

/// Bundled inputs for [`persist_graph`] — keeps the call site readable
/// and stays under clippy's `too_many_arguments` threshold.
#[derive(Clone, Debug)]
pub struct PersistGraphRequest<'a> {
    pub id: &'a str,
    pub intent_signature: &'a str,
    pub success: bool,
    pub graph_json: &'a str,
    pub created_ts: i64,
    /// Unit-normalised dense embedding. `None` keeps the row portable to
    /// non-fastembed hosts (retrieve falls back to exact-match / recency).
    pub embedding: Option<&'a [f32]>,
    pub emb_model: Option<&'a str>,
}

/// Persist a mission-graph snapshot keyed by `intent_signature`.
///
/// `embedding` MUST already be normalised to unit length when `Some`;
/// the caller owns that contract (the fastembed layer normalises before
/// handing off). `None` keeps the row portable to non-fastembed hosts.
///
/// `id` is caller-supplied so the Supervisor can reuse a deterministic
/// UUID; passing `&Uuid::new_v4().to_string()` is fine for ad-hoc use.
pub fn persist_graph(conn: &Connection, req: &PersistGraphRequest<'_>) -> anyhow::Result<()> {
    let emb_blob: Option<Vec<u8>> = req.embedding.map(|v| {
        let mut bytes = Vec::with_capacity(v.len() * 4);
        for f in v {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        bytes
    });
    conn.execute(
        "INSERT INTO learning_graphs
            (id, intent_signature, success, graph_json, created_ts, embedding, emb_model)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET
            intent_signature = excluded.intent_signature,
            success          = excluded.success,
            graph_json        = excluded.graph_json,
            created_ts        = excluded.created_ts,
            embedding         = excluded.embedding,
            emb_model         = excluded.emb_model",
        rusqlite::params![
            req.id,
            req.intent_signature,
            if req.success { 1 } else { 0 },
            req.graph_json,
            req.created_ts,
            emb_blob,
            req.emb_model,
        ],
    )?;
    Ok(())
}

/// One hit returned by [`retrieve_similar_graphs`]. Carries the cosine
/// score (`0.0..=1.0` when an embedding was available, `1.0` for an
/// exact `intent_signature` hash fallback match, `0.0` for the recency
/// fallback so it sorts last).
#[derive(Clone, Debug, PartialEq)]
pub struct ScoredGraph {
    pub row: LearningGraphRow,
    pub score: f32,
}

/// Retrieve the top-k most similar successful learning graphs.
///
/// `query_embedding` (unit-normalised `f32` slice) drives cosine when
/// `Some`. When `None`, retrieval falls back in this order:
///   1. exact `intent_signature` match (score 1.0) when
///      `intent_signature` is `Some` and non-empty,
///   2. pure recency (score 0.0) otherwise.
///
/// Only rows with `success = 1` are returned (RFC 16 §3 — anti-patterns
/// are persisted for audit but never injected as Planner hints).
pub fn retrieve_similar_graphs(
    conn: &Connection,
    query_embedding: Option<&[f32]>,
    intent_signature: Option<&str>,
    top_k: usize,
) -> anyhow::Result<Vec<ScoredGraph>> {
    if top_k == 0 {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT id, intent_signature, success, graph_json, created_ts, embedding, emb_model
         FROM learning_graphs
         WHERE success = 1
         ORDER BY created_ts DESC",
    )?;
    let mut rows_iter = stmt.query_map([], parse_row)?;
    let mut scored: Vec<ScoredGraph> = Vec::new();
    while let Some(row) = rows_iter.next().transpose()? {
        let score = score_row(&row, query_embedding, intent_signature);
        scored.push(ScoredGraph { row, score });
    }
    // Highest score first; ties broken by recency (created_ts DESC).
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.row.created_ts.cmp(&a.row.created_ts))
    });
    scored.truncate(top_k);
    Ok(scored)
}

fn parse_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LearningGraphRow> {
    let id: String = row.get(0)?;
    let intent_signature: String = row.get(1)?;
    let success_i: i64 = row.get(2)?;
    let graph_json: String = row.get(3)?;
    let created_ts: i64 = row.get(4)?;
    let emb_blob: Option<Vec<u8>> = row.get(5)?;
    let emb_model: Option<String> = row.get(6)?;
    let embedding = emb_blob.as_ref().map(|b| decode_f32_le(b)).transpose()?;
    Ok(LearningGraphRow {
        id,
        intent_signature,
        success: success_i != 0,
        graph_json,
        created_ts,
        embedding,
        emb_model,
    })
}

fn decode_f32_le(bytes: &[u8]) -> rusqlite::Result<Vec<f32>> {
    if bytes.len() % 4 != 0 {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Blob,
            format!(
                "embedding blob length {} is not a multiple of 4",
                bytes.len()
            )
            .into(),
        ));
    }
    let mut out = Vec::with_capacity(bytes.len() / 4);
    let mut i = 0;
    while i < bytes.len() {
        let mut arr = [0u8; 4];
        arr.copy_from_slice(&bytes[i..i + 4]);
        out.push(f32::from_le_bytes(arr));
        i += 4;
    }
    Ok(out)
}

fn score_row(
    row: &LearningGraphRow,
    query_embedding: Option<&[f32]>,
    intent_signature: Option<&str>,
) -> f32 {
    if let (Some(q), Some(emb)) = (query_embedding, row.embedding.as_ref()) {
        return cosine(q, emb);
    }
    if let Some(sig) = intent_signature {
        if !sig.is_empty() && sig == row.intent_signature {
            return 1.0;
        }
    }
    0.0
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
    }
    let na = magnitude(a);
    let nb = magnitude(b);
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

fn magnitude(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;
    use crate::journal::schema::migrate;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn put(conn: &Connection, id: &str, sig: &str, success: bool, emb: Option<&[f32]>, ts: i64) {
        persist_graph(
            conn,
            &PersistGraphRequest {
                id,
                intent_signature: sig,
                success,
                graph_json: "{}",
                created_ts: ts,
                embedding: emb,
                emb_model: emb.map(|_| "test-model"),
            },
        )
        .unwrap();
    }

    #[test]
    fn persist_and_roundtrip_no_embedding() {
        let conn = fresh_conn();
        put(&conn, "g1", "sig-x", true, None, 100);
        let hits = retrieve_similar_graphs(&conn, None, Some("sig-x"), 10).unwrap();
        assert_eq!(hits.len(), 1);
        let row = &hits[0].row;
        assert_eq!(row.id, "g1");
        assert_eq!(row.intent_signature, "sig-x");
        assert!(row.success);
        assert_eq!(row.embedding, None);
        assert_eq!(row.emb_model, None);
    }

    #[test]
    fn persist_and_roundtrip_with_embedding() {
        let conn = fresh_conn();
        let emb = [0.1_f32, 0.2, 0.3, 0.4];
        put(&conn, "g2", "sig-y", true, Some(&emb), 100);
        let hits = retrieve_similar_graphs(&conn, Some(&emb), None, 10).unwrap();
        assert_eq!(hits.len(), 1);
        let recovered = hits[0].row.embedding.as_ref().unwrap();
        for (a, b) in emb.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
        assert_eq!(hits[0].row.emb_model.as_deref(), Some("test-model"));
        assert!((hits[0].score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn retrieve_only_returns_successful_rows() {
        let conn = fresh_conn();
        put(&conn, "ok", "sig", true, None, 1);
        put(&conn, "fail", "sig", false, None, 2);
        let hits = retrieve_similar_graphs(&conn, None, Some("sig"), 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].row.id, "ok");
    }

    #[test]
    fn retrieve_top_k_truncates() {
        let conn = fresh_conn();
        for i in 0..5 {
            put(&conn, &format!("g{i}"), "sig", true, None, i as i64);
        }
        let hits = retrieve_similar_graphs(&conn, None, Some("sig"), 3).unwrap();
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn retrieve_top_k_zero_returns_empty() {
        let conn = fresh_conn();
        put(&conn, "g", "sig", true, None, 1);
        assert!(retrieve_similar_graphs(&conn, None, Some("sig"), 0)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn exact_signature_match_scores_one() {
        let conn = fresh_conn();
        put(&conn, "g1", "sig-a", true, None, 1);
        put(&conn, "g2", "sig-b", true, None, 2);
        let hits = retrieve_similar_graphs(&conn, None, Some("sig-a"), 10).unwrap();
        assert_eq!(hits[0].row.id, "g1");
        assert!((hits[0].score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_ranks_higher_similarity_first() {
        let conn = fresh_conn();
        let q = [1.0_f32, 0.0];
        let near = [0.95_f32, 0.3_f32];
        let far = [0.0_f32, 1.0];
        put(&conn, "near", "a", true, Some(&near), 1);
        put(&conn, "far", "b", true, Some(&far), 2);
        let hits = retrieve_similar_graphs(&conn, Some(&q), None, 10).unwrap();
        assert_eq!(hits[0].row.id, "near");
        assert!(hits[0].score > hits[1].score);
    }

    #[test]
    fn cosine_zero_embedding_returns_zero_score() {
        let conn = fresh_conn();
        let zero = [0.0_f32; 4];
        put(&conn, "g", "sig", true, Some(&zero), 1);
        let hits = retrieve_similar_graphs(&conn, Some(&[1.0, 0.0, 0.0, 0.0]), None, 10).unwrap();
        assert!((hits[0].score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn mismatched_embedding_lengths_score_zero() {
        let conn = fresh_conn();
        put(&conn, "g", "sig", true, Some(&[1.0, 0.0, 0.0]), 1);
        let hits = retrieve_similar_graphs(&conn, Some(&[1.0, 0.0]), None, 10).unwrap();
        assert!((hits[0].score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn recency_fallback_when_no_embedding_and_no_sig() {
        let conn = fresh_conn();
        put(&conn, "old", "sig-a", true, None, 10);
        put(&conn, "new", "sig-b", true, None, 200);
        let hits = retrieve_similar_graphs(&conn, None, None, 10).unwrap();
        assert_eq!(hits[0].row.id, "new");
    }

    #[test]
    fn empty_sig_falls_back_to_recency_not_exact_match() {
        let conn = fresh_conn();
        put(&conn, "g", "sig-a", true, None, 1);
        let hits = retrieve_similar_graphs(&conn, None, Some(""), 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!((hits[0].score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn persist_is_idempotent_on_id() {
        let conn = fresh_conn();
        put(&conn, "g", "sig-a", true, None, 1);
        put(&conn, "g", "sig-b", true, None, 2);
        let hits = retrieve_similar_graphs(&conn, None, Some("sig-b"), 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].row.id, "g");
        assert_eq!(hits[0].row.intent_signature, "sig-b");
        assert_eq!(hits[0].row.created_ts, 2);
    }
}
