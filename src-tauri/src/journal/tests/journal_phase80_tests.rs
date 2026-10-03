use crate::core::bus::{BusEvent, BusEventKind};
use crate::journal::Journal;
use rusqlite::Connection;
use tempfile::TempDir;

fn open() -> (TempDir, Journal) {
    let tmp = TempDir::new().expect("tmp");
    let journal = Journal::open(tmp.path()).expect("open runs migrate");
    (tmp, journal)
}

fn publish(journal: &Journal, prompt: &str) {
    let evt = BusEvent::new(BusEventKind::TaskReceived {
        raw_prompt: prompt.into(),
        session_id: uuid::Uuid::new_v4(),
    });
    journal.publish(&evt).expect("publish");
}

#[test]
fn m31_advances_schema_version_and_creates_dir_access() {
    let (tmp, _) = open();
    let conn = Connection::open(tmp.path().join("journal.db")).expect("raw conn");
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(
        v >= crate::journal::schema::CURRENT_SCHEMA_VERSION,
        "expected version >= CURRENT after migrate(), got {v}"
    );
    assert!(v >= 31, "M31 must have run, got {v}");
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='dir_access'",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(count, 1, "M31 dir_access must exist");
}

#[test]
fn m31_migration_is_idempotent() {
    let (tmp, _) = open();
    let db_path = tmp.path().join("journal.db");
    let conn = Connection::open(&db_path).expect("reopen");
    crate::journal::schema::migrate(&conn).expect("second migrate ok");
    crate::journal::schema::migrate(&conn).expect("third migrate ok");
    let v: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_version", [], |r| r.get(0))
        .expect("query");
    assert!(v >= 31, "version must stay >= 31 after re-migrate, got {v}");
}

#[test]
fn search_round_trip_finds_published_payload() {
    let (_tmp, journal) = open();
    publish(&journal, "migrate the unicorn billing pipeline");
    publish(&journal, "something entirely unrelated here");
    let hits = journal
        .search_events("unicorn billing", 10)
        .expect("search");
    assert_eq!(hits.len(), 1, "only the matching event must hit");
    assert_eq!(hits[0].kind, "task_received");
}

#[test]
fn search_with_fts_operators_falls_back_to_like_without_error() {
    let (_tmp, journal) = open();
    publish(&journal, "plain event about caching");
    let hits = journal.search_events("*** (((", 10).expect("search");
    assert!(
        hits.is_empty(),
        "operator-only input must degrade to LIKE with no hits, not an error"
    );
    let hits = journal.search_events("caching", 10).expect("search");
    assert_eq!(hits.len(), 1, "LIKE fallback must still match 'caching'");
}

#[test]
fn search_empty_query_returns_empty_without_error() {
    let (_tmp, journal) = open();
    publish(&journal, "anything");
    let hits = journal.search_events("   ", 10).expect("search");
    assert!(hits.is_empty());
}

#[test]
fn record_dir_access_upserts_counter_and_frecency_ranks() {
    let (_tmp, journal) = open();
    journal
        .record_dir_access("/wts/mission-a/backend")
        .expect("rec");
    journal
        .record_dir_access("/wts/mission-a/backend")
        .expect("rec");
    journal
        .record_dir_access("/wts/mission-a/backend")
        .expect("rec");
    journal
        .record_dir_access("/wts/mission-b/frontend")
        .expect("rec");
    let hits = journal.frecency("", 10).expect("frecency");
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].0, "/wts/mission-a/backend");
    assert!(
        hits[0].1 > hits[1].1,
        "3 accesses must outrank 1 at equal recency"
    );
}

#[test]
fn frecency_prefix_filters_and_aging_decay_applies() {
    let (_tmp, journal) = open();
    journal.record_dir_access("/wts/alpha").expect("rec");
    journal.record_dir_access("/wts/beta").expect("rec");
    let hits = journal.frecency("alpha", 10).expect("frecency");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].0, "/wts/alpha");

    let stale = crate::journal::frecency::frecency_score(4, 0, 20_000_000_000);
    let fresh = crate::journal::frecency::frecency_score(1, 19_999_999_000, 20_000_000_000);
    assert!(
        fresh > stale,
        "aging decay must let a fresh dir beat a stale 4x-used one (4.0 vs 1.0)"
    );
}

#[test]
fn record_dir_access_rejects_empty_dir() {
    let (_tmp, journal) = open();
    assert!(journal.record_dir_access("   ").is_err());
    let hits = journal.frecency("", 10).expect("frecency");
    assert!(hits.is_empty());
}
