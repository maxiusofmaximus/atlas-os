// Reach into the crate root: tests live as a child mod of `journal`,
// so `super::Journal` would resolve to `journal::tests::Journal`.
use crate::core::bus::{BusEvent, BusEventKind};
use crate::journal::Journal;
use tempfile::TempDir;

#[test]
fn journal_happy_path_publish_and_tail() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path().to_path_buf();
    let journal = Journal::open(&root).expect("open");

    let evt = BusEvent::new(BusEventKind::TaskReceived {
        raw_prompt: "hello world".into(),
        session_id: uuid::Uuid::new_v4(),
    });
    journal.publish(&evt).expect("publish");

    let tail = journal.tail(10).expect("tail");
    assert_eq!(tail.len(), 1);
    assert_eq!(tail[0].kind, "task_received");
}

#[test]
fn journal_idempotent_replay() {
    let tmp = TempDir::new().expect("tmp");
    let root = tmp.path().to_path_buf();
    let journal = Journal::open(&root).expect("open");

    let evt = BusEvent::new(BusEventKind::AgentStatusChanged {
        agent_id: uuid::Uuid::new_v4(),
        status: crate::core::bus::AgentStatus::Queued,
    });
    journal.publish(&evt).expect("publish");
    // Replay the same event (same `id`) — must NOT duplicate.
    journal.publish(&evt).expect("publish replay");

    let tail = journal.tail(10).expect("tail");
    assert_eq!(
        tail.len(),
        1,
        "idempotency_key collision must drop the replay (RFC 02 §3.1.2)"
    );
}
