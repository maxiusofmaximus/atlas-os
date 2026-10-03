// Atlas OS — Calendar background poller (RFC 28 §G READ, v3.1.A.3).
//
// A single `tokio::spawn` task, spawned from `AppState::bootstrap`, that
// periodically re-syncs every enabled ICS subscription (`sync_all_ics`) so
// the operational calendar context survives without a human running
// `atlas calendar sync-all`. Mirrors `toast::scheduler::ToastDriver`: it owns
// a `oneshot` shutdown channel and is torn down when the `AppState` drops.
//
// Cadence defaults to 15 minutes, overridable with `ATLAS_CALENDAR_POLL_SECS`
// (`0` disables the poller). The first run is delayed 30 s so it does not
// contend with app/UI startup.
//
// The journal lock is never held across the network `.await` (`sync_all_ics`
// takes it only for the short synchronous reads/writes), so the poller cannot
// stall the HUD or the CLI.

use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::journal::Journal;

/// Default sync cadence when `ATLAS_CALENDAR_POLL_SECS` is unset.
pub const DEFAULT_INTERVAL_SECS: u64 = 900;
/// Delay before the first sync, to avoid startup contention.
pub const DEFAULT_INITIAL_DELAY_SECS: u64 = 30;

pub struct CalendarPoller {
    handle: Mutex<Option<JoinHandle<()>>>,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl CalendarPoller {
    /// Spawn the poller over the shared journal. `interval` is the period
    /// between syncs; `initial_delay` defers the first sync.
    pub fn spawn(
        journal: Arc<Mutex<Journal>>,
        interval: Duration,
        initial_delay: Duration,
    ) -> Self {
        let (tx, mut rx) = oneshot::channel::<()>();
        let handle = tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(initial_delay) => {}
                _ = &mut rx => return,
            }
            loop {
                tokio::select! {
                    result = run_once(&journal) => {
                        if let Err(e) = result {
                            tracing::warn!(error = %e, "calendar poller tick failed");
                        }
                    }
                    _ = &mut rx => return,
                }
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = &mut rx => return,
                }
            }
        });
        Self {
            handle: Mutex::new(Some(handle)),
            shutdown_tx: Mutex::new(Some(tx)),
        }
    }

    /// Read the poll cadence from `ATLAS_CALENDAR_POLL_SECS`, falling back to
    /// `DEFAULT_INTERVAL_SECS`. `0` means "do not spawn the poller".
    pub fn configured_interval_secs() -> u64 {
        std::env::var("ATLAS_CALENDAR_POLL_SECS")
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_INTERVAL_SECS)
    }

    pub async fn shutdown(&self) {
        if let Some(tx) = self.shutdown_tx.lock().take() {
            let _ = tx.send(());
        }
        // Bind before awaiting: `if let Some(h) = self.handle.lock().take()`
        // would keep the guard temporary alive through the `.await`.
        let handle = self.handle.lock().take();
        if let Some(h) = handle {
            let _ = h.await;
        }
    }
}

impl Drop for CalendarPoller {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.lock().take() {
            let _ = tx.send(());
        }
    }
}

/// One sync pass over every enabled subscription. Returns the number of feeds
/// attempted (not a count of events).
pub async fn run_once(journal: &Arc<Mutex<Journal>>) -> anyhow::Result<usize> {
    let results = crate::calendar::ics_reader::sync_all_ics(journal)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    for r in &results {
        match &r.error {
            Some(e) => {
                tracing::warn!(sub = %r.name, error = %e, "calendar subscription sync failed");
            }
            None if r.not_modified => tracing::debug!(sub = %r.name, "calendar unchanged (304)"),
            None => {
                tracing::info!(sub = %r.name, written = r.written, "calendar subscription synced");
            }
        }
    }
    Ok(results.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> (tempfile::TempDir, Arc<Mutex<Journal>>) {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let journal = Journal::open(tmp.path()).expect("open");
        (tmp, Arc::new(Mutex::new(journal)))
    }

    #[tokio::test]
    async fn run_once_with_no_subscriptions_is_a_noop() {
        let (_dir, journal) = fresh();
        let n = run_once(&journal).await.expect("run");
        assert_eq!(n, 0);
        assert_eq!(journal.lock().busy_window_count().unwrap(), 0);
    }

    #[tokio::test]
    async fn spawn_and_shutdown_tears_down_cleanly() {
        let (_dir, journal) = fresh();
        let poller =
            CalendarPoller::spawn(journal, Duration::from_millis(50), Duration::from_millis(1));
        assert!(poller.handle.lock().is_some());
        tokio::time::sleep(Duration::from_millis(20)).await;
        poller.shutdown().await;
        assert!(poller.handle.lock().is_none());
    }
}
