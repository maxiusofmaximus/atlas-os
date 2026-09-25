// Atlas OS — frecency navigation (research 36 sub-fase 8.0, RFC 35 §5).
// Deterministic port of the zoxide aging + ranking algorithm, dependency-free:
// score = access_count × recency_decay(elapsed_since_last_access).
// zoxide is rejected as a crate (22 deps, RFC 25 §11); the CLI-external
// `zoxide query` remains an opt-in convenience, never a dependency.

use anyhow::{Context, Result};
use rusqlite::Connection;

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 86_400_000;
const WEEK_MS: i64 = 604_800_000;

/// zoxide rank multiplier by recency: fresh dirs outrank stale ones at
/// equal access counts so `swarm jump` converges on current work.
pub fn frecency_score(access_count: i64, last_access_ms: i64, now_ms: i64) -> f64 {
    let elapsed = now_ms.saturating_sub(last_access_ms);
    let decay = if elapsed < HOUR_MS {
        4.0
    } else if elapsed < DAY_MS {
        2.0
    } else if elapsed < WEEK_MS {
        0.5
    } else {
        0.25
    };
    access_count.max(0) as f64 * decay
}

fn parse_ms(ts: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|t| t.timestamp_millis())
        .unwrap_or(0)
}

/// Record one access to `dir`. UPSERT: the counter increments and
/// `last_access` moves forward (live state like `step_states` M11 —
/// deliberately NOT first-write-wins, the count must track real use).
pub fn record_dir_access(conn: &Connection, dir: &str, now_rfc3339: &str) -> Result<()> {
    let dir = dir.trim();
    if dir.is_empty() {
        anyhow::bail!("dir must not be empty");
    }
    conn.execute(
        "INSERT INTO dir_access (dir, access_count, last_access, created_at)
         VALUES (?1, 1, ?2, ?2)
         ON CONFLICT(dir) DO UPDATE SET
            access_count = dir_access.access_count + 1,
            last_access  = excluded.last_access",
        rusqlite::params![dir, now_rfc3339],
    )
    .with_context(|| format!("recording dir access for '{dir}'"))?;
    Ok(())
}

/// Ranked `(dir, score)` pairs whose dir contains `prefix` (empty prefix
/// matches all), ordered by score DESC then dir ASC for determinism.
pub fn frecency(
    conn: &Connection,
    prefix: &str,
    now_ms: i64,
    limit: i64,
) -> Result<Vec<(String, f64)>> {
    let limit = limit.clamp(1, 200);
    let like = format!("%{prefix}%");
    let mut stmt = conn.prepare(
        "SELECT dir, access_count, last_access FROM dir_access
         WHERE dir LIKE ?1 ORDER BY dir ASC",
    )?;
    let rows = stmt.query_map(rusqlite::params![like], |row| {
        let dir: String = row.get(0)?;
        let count: i64 = row.get(1)?;
        let last: String = row.get(2)?;
        Ok((dir, count, last))
    })?;
    let mut out: Vec<(String, f64)> = Vec::new();
    for row in rows {
        let (dir, count, last) = row?;
        out.push((dir, frecency_score(count, parse_ms(&last), now_ms)));
    }
    out.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    out.truncate(limit as usize);
    Ok(out)
}

#[cfg(test)]
mod frecency_score_tests {
    use super::*;

    #[test]
    fn decay_buckets_match_zoxide_multipliers() {
        assert!((frecency_score(10, 0, 1_000) - 40.0).abs() < 1e-9);
        assert!((frecency_score(10, 0, HOUR_MS + 1) - 20.0).abs() < 1e-9);
        assert!((frecency_score(10, 0, DAY_MS + 1) - 5.0).abs() < 1e-9);
        assert!((frecency_score(10, 0, WEEK_MS + 1) - 2.5).abs() < 1e-9);
    }

    #[test]
    fn fresh_single_access_outranks_stale_heavy_use() {
        let now = 10_000_000_000i64;
        let fresh = frecency_score(1, now - 1_000, now);
        let stale = frecency_score(4, 0, now);
        assert!(
            fresh > stale,
            "recency must beat raw count ({fresh} vs {stale})"
        );
    }
}
