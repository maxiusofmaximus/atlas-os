// Atlas OS — AppState.
// Single source of truth shared by:
//  - Tauri desktop frontend (via IPC commands — `core::ipc`)
//  - Local HUD WebSocket server (`hud::serve`)
//  - LSP host subprocess pool (`lsp::host`)
//  - `atlas` CLI binary (`cli/bin/opencode.rs`)
//
// Designed to **survive webview crashes** (RFC 25 §2): the inner state is
// an Arc around a small bundle of long-lived handles; the webview holds a
// clone but cannot tear down the kernel when it dies.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use tokio::sync::broadcast;

use crate::journal::Journal;
use crate::profiles::{self, ProfileId};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    pub profile_id: RwLock<ProfileId>,
    pub profile_root: RwLock<PathBuf>,
    pub journal: Arc<Mutex<Journal>>,
    /// Kernel Bus broadcast channel — multi-consumer event stream
    /// (RFC 02 §3.1). HUD, LSP, CLI, swarm all subscribe.
    pub bus_tx: broadcast::Sender<BusEvent>,
    /// HUD port where the axum server is bound. Published by `hud::serve`
    /// after bind succeeds. Atomic because every IPC `hud_url` call reads
    /// it; locking the journal mutex just to read an int is wasteful and
    /// can induce async stalls.
    pub hud_port: AtomicU16,
    /// Toast driver task handle (RFC 28 §F). Present when the `toast`
    /// feature is enabled, regardless of platform. Owned by `Inner`
    /// so dropping the `AppState` triggers a graceful shutdown.
    /// Currently no public accessor is needed — `Drop` does the
    /// teardown.
    #[cfg(feature = "toast")]
    #[allow(dead_code)]
    pub toast_driver: Option<crate::toast::scheduler::ToastDriver>,
    /// Calendar poller task handle (RFC 28 §G READ, v3.1.A.3). Present when
    /// the `calendar-ics` feature is enabled and `ATLAS_CALENDAR_POLL_SECS`
    /// is non-zero. Owned here so dropping the `AppState` tears it down.
    #[cfg(feature = "calendar-ics")]
    #[allow(dead_code)]
    pub calendar_poller: Option<crate::calendar::poller::CalendarPoller>,
}

impl AppState {
    pub fn bootstrap() -> anyhow::Result<Self> {
        let profile_id = ProfileId::default();
        let profile_root = profiles::resolve_root(&profile_id)?;
        let journal = Journal::open(&profile_root)?;
        let journal = Arc::new(Mutex::new(journal));
        let (bus_tx, _) = broadcast::channel(1024);

        #[cfg(feature = "toast")]
        let toast_driver = {
            // Register AUMID + spawn Toast driver (RFC 28 §F.2/§F.3).
            // Errors here are non-fatal — Toast is a convenience
            // surface, not a core dependency. We log and proceed
            // without the driver if registration or spawn fails.
            let icon_path = None; // Bundled icon path resolved post-§F MVP.
            if let Err(e) = crate::toast::manager::register_aumid(icon_path) {
                tracing::warn!(error = %e, "toast AUMID registration failed — toasts will not receive activations");
            }
            let dispatcher = crate::toast::manager::ToastDispatcher::new_default();
            Some(crate::toast::scheduler::ToastDriver::spawn(
                Arc::clone(&journal),
                dispatcher,
            ))
        };

        #[cfg(feature = "calendar-ics")]
        let calendar_poller = {
            // Background ICS-subscription refresh (v3.1.A.3). Errors are
            // non-fatal; `0` disables the poller entirely.
            let secs = crate::calendar::poller::CalendarPoller::configured_interval_secs();
            if secs == 0 {
                None
            } else {
                Some(crate::calendar::poller::CalendarPoller::spawn(
                    Arc::clone(&journal),
                    std::time::Duration::from_secs(secs),
                    std::time::Duration::from_secs(
                        crate::calendar::poller::DEFAULT_INITIAL_DELAY_SECS,
                    ),
                ))
            }
        };

        Ok(Self {
            inner: Arc::new(Inner {
                profile_id: RwLock::new(profile_id),
                profile_root: RwLock::new(profile_root),
                journal,
                bus_tx,
                hud_port: AtomicU16::new(0),
                #[cfg(feature = "toast")]
                toast_driver,
                #[cfg(feature = "calendar-ics")]
                calendar_poller,
            }),
        })
    }

    /// HTTP URL where the HUD Mission Control is reachable (RFC 24 §1).
    /// For Phase 0 we bind dynamically; the real port is published on the
    /// Kernel Bus after `hud::serve` succeeds. Reads an atomic — no lock.
    pub fn hud_url(&self) -> String {
        format!(
            "http://localhost:{}/",
            self.inner.hud_port.load(Ordering::Relaxed)
        )
    }

    /// Set the HUD port. Called once by `hud::server::serve` after the axum
    /// listener binds successfully.
    pub fn set_hud_port(&self, port: u16) {
        self.inner.hud_port.store(port, Ordering::Relaxed);
    }

    /// Raw HUD port (0 = not serving yet). Read by `/remote/status`
    /// (RFC 20 Phase 8.3) so the route reports the port without
    /// string-parsing `hud_url()`.
    pub fn hud_port(&self) -> u16 {
        self.inner.hud_port.load(Ordering::Relaxed)
    }

    pub fn journal(&self) -> parking_lot::MutexGuard<'_, Journal> {
        self.inner.journal.lock()
    }

    /// RFC 20 Fase 23 v3.1.2.1 — current operator availability for a
    /// proactive turn, computed from the M18 busy windows (research/52).
    /// This is the `context_availability` the repo lacked.
    pub fn context_availability(
        &self,
        policy: &crate::planning::availability::TurnPolicy,
    ) -> anyhow::Result<crate::planning::availability::Availability> {
        let now = chrono::Utc::now().timestamp_millis();
        let journal = self.inner.journal.lock();
        crate::planning::availability::availability_now(&journal, now, policy)
    }

    pub fn bus(&self) -> broadcast::Sender<BusEvent> {
        self.inner.bus_tx.clone()
    }

    /// Persist a Kernel Bus event to the journal AND publish it to the
    /// in-process broadcast channel, in that order.
    ///
    /// Ordering invariant (RFC 02 §3.1): any subscriber that observes an
    /// event on the broadcast channel is guaranteed to find it persisted in
    /// the journal. We hold the journal mutex across the publish so a
    /// concurrent `journal::tail()` reader cannot return a state older than
    /// what the bus already announced.
    ///
    /// Best-effort delivery: if no subscriber is listening the broadcast
    /// send returns a `SendError`, which we treat as `Ok(())` (the journal
    /// is still the durable source of truth).
    pub fn publish_and_broadcast(&self, event: &BusEvent) -> anyhow::Result<()> {
        let journal = self.inner.journal.lock();
        journal.publish(event)?;
        drop(journal);
        let _ = self.inner.bus_tx.send(event.clone());
        Ok(())
    }

    pub fn profile_root(&self) -> PathBuf {
        self.inner.profile_root.read().clone()
    }

    pub fn profile_id(&self) -> ProfileId {
        self.inner.profile_id.read().clone()
    }

    pub fn switch_profile(&self, new: ProfileId) -> anyhow::Result<()> {
        let path = profiles::resolve_root(&new)?;
        let new_journal = Journal::open(&path)?;
        // Atomic-ish swap. Profile metadata fields are independent RwLocks;
        // the journal lives behind a single Mutex so readers never observe a
        // half-swapped state.
        *self.inner.profile_id.write() = new;
        *self.inner.profile_root.write() = path;
        *self.inner.journal.lock() = new_journal;
        Ok(())
    }

    /// Test-and-HUD-integration-only constructor that wraps a pre-built
    /// Journal in an AppState without going through profile bootstrap.
    /// The profile id is `default` and the profile_root is a sentinel —
    /// only the journal matters for HUD route handlers that read the
    /// journal. Non-production callers should prefer `bootstrap()`.
    #[allow(dead_code)]
    pub(crate) fn from_journal(journal: Journal) -> Self {
        Self::from_journal_in(journal, PathBuf::new())
    }

    /// Test-only variant of `from_journal` that pins the profile root, so
    /// route tests can stage per-profile files (tokens, `hud_port.txt`)
    /// in a tempdir instead of racing on process env vars.
    #[allow(dead_code)]
    pub(crate) fn from_journal_in(journal: Journal, profile_root: PathBuf) -> Self {
        let (bus_tx, _) = broadcast::channel(1024);
        Self {
            inner: Arc::new(Inner {
                profile_id: RwLock::new(ProfileId::default()),
                profile_root: RwLock::new(profile_root),
                journal: Arc::new(Mutex::new(journal)),
                bus_tx,
                hud_port: AtomicU16::new(0),
                #[cfg(feature = "toast")]
                toast_driver: None,
                #[cfg(feature = "calendar-ics")]
                calendar_poller: None,
            }),
        }
    }
}

// Re-exports
pub use super::bus::BusEvent;
