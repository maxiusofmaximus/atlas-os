// OpenCode OS — Toast scheduler driver (RFC 28 Section F).
//
// A single `tokio::spawn` task that polls the SQLite `toast_queue`
// for overdue `pending` rows, dispatches them via the configured
// `ToastDispatcher`, and writes the outcome back via `ToastQueue`.
//
// The cadence is 5 s when idle: the driver calls `next_pending(now)`,
// if there is no overdue row it sleeps 5 s; if there is one it
// dispatches immediately and loops back. The 5 s idle is a
// compromise — short enough that a `model_ready` toast firing from
// the §H reset-window is delivered within 5 s of the reset becoming
// active, long enough that an idle machine doesn't burn CPU polling
// an empty queue.
//
// Dispatch is **synchronous and blocking** (WinRT callbacks land on
// the calling thread; the `ToastManager` types are not `Send`). We
// therefore run the dispatch via `tokio::task::spawn_blocking` and
// keep the future `Send`-free of the dispatcher itself.
//
// `ToastDriver::spawn(conn, dispatcher)` returns a handle for
// graceful shutdown via the `shutdown` method / Drop.
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::journal::Journal;
use crate::toast::error::{Result as ToastResult, ToastFacadeError};
use crate::toast::manager::{outcome_to_status, ToastDispatcher};

pub struct ToastDriver {
    handle: Mutex<Option<JoinHandle<ToastResult<()>>>>,
    shutdown_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl ToastDriver {
    /// Spawn the driver over a shared `Journal`. The driver locks the
    /// journal briefly for each SQLite read/write — no long-held
    /// locks across the dispatch await.
    pub fn spawn(journal: Arc<Mutex<Journal>>, dispatcher: ToastDispatcher) -> Self {
        let (tx, mut rx) = oneshot::channel::<()>();
        let dispatcher = Arc::new(Mutex::new(dispatcher));
        let handle = tokio::spawn(async move {
            loop {
                let now_ms = chrono::Utc::now().timestamp_millis();
                let next = {
                    let journal = journal.lock();
                    journal.toast_next_pending(now_ms)?
                };
                match next {
                    Some(row) => {
                        // Dispatch is sync/blocking. Run in a spawn_blocking so the
                        // tokio task never touches the non-Send dispatcher.
                        let disp = dispatcher.clone();
                        let payload_arc = row.to_payload();
                        let outcome =
                            tokio::task::spawn_blocking(move || disp.lock().dispatch(&payload_arc))
                                .await
                                .map_err(|e| {
                                    ToastFacadeError::Io(std::io::Error::other(format!("{e:?}")))
                                })?;
                        let (status, stamp) = outcome_to_status(&outcome);
                        let reason = crate::toast::manager::outcome_to_reason(&outcome);
                        let journal = journal.lock();
                        match status {
                            "dismissed" => {
                                journal.toast_mark("dismissed", row.id, stamp, Some(reason))?;
                            }
                            "failed" => {
                                journal.toast_mark("failed", row.id, stamp, None)?;
                            }
                            "fired" => {
                                journal.toast_mark("fired", row.id, stamp, None)?;
                            }
                            _ => unreachable!(),
                        }
                    }
                    None => {
                        tokio::select! {
                            _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                            _ = &mut rx => return Ok(()),
                        }
                    }
                }
                tokio::select! {
                    _ = &mut rx => return Ok(()),
                    _ = tokio::task::yield_now() => {}
                }
            }
        });
        Self {
            handle: Mutex::new(Some(handle)),
            shutdown_tx: Mutex::new(Some(tx)),
        }
    }

    pub async fn shutdown(&self) -> ToastResult<()> {
        if let Some(tx) = self.shutdown_tx.lock().take() {
            let _ = tx.send(());
        }
        let join = self.handle.lock().take();
        if let Some(h) = join {
            h.await
                .map_err(|e| ToastFacadeError::Io(std::io::Error::other(format!("{e:?}"))))??;
        }
        Ok(())
    }
}

impl Drop for ToastDriver {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.lock().take() {
            let _ = tx.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_journal() -> (tempfile::TempDir, Arc<Mutex<Journal>>) {
        let tmp = tempfile::tempdir().expect("tmpdir");
        let journal = Journal::open(tmp.path()).expect("journal open");
        (tmp, Arc::new(Mutex::new(journal)))
    }

    #[tokio::test]
    #[cfg(not(windows))]
    async fn driver_fires_one_then_idles_to_shutdown() {
        let (_tmp, journal) = fresh_journal();
        journal
            .toast_enqueue(
                crate::toast::payload::ToastKind::Info,
                "hello",
                None,
                None,
                0,
            )
            .unwrap();
        let dispatcher = ToastDispatcher::Stub;
        let driver = ToastDriver::spawn(journal.clone(), dispatcher);
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(journal.toast_count_pending().unwrap(), 0);
        driver.shutdown().await.unwrap();
    }

    #[tokio::test]
    #[cfg(not(windows))]
    async fn driver_drains_backlog_on_startup() {
        let (_tmp, journal) = fresh_journal();
        journal
            .toast_enqueue(crate::toast::payload::ToastKind::Info, "a", None, None, 0)
            .unwrap();
        journal
            .toast_enqueue(crate::toast::payload::ToastKind::Info, "b", None, None, 0)
            .unwrap();
        let dispatcher = ToastDispatcher::Stub;
        let driver = ToastDriver::spawn(journal.clone(), dispatcher);
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert_eq!(journal.toast_count_pending().unwrap(), 0);
        driver.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn driver_handle_is_some_after_spawn() {
        let (_tmp, journal) = fresh_journal();
        let dispatcher = ToastDispatcher::Stub;
        let driver = ToastDriver::spawn(journal, dispatcher);
        assert!(driver.handle.lock().is_some());
        driver.shutdown().await.unwrap();
    }
}
