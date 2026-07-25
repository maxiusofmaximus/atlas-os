// OpenCode OS — AppState.
// Single source of truth shared by:
//  - Tauri desktop frontend (via IPC commands — `core::ipc`)
//  - Local HUD WebSocket server (`hud::serve`)
//  - LSP host subprocess pool (`lsp::host`)
//  - `opencode` CLI binary (`cli/bin/opencode.rs`)
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
    pub journal: Mutex<Journal>,
    /// Kernel Bus broadcast channel — multi-consumer event stream
    /// (RFC 02 §3.1). HUD, LSP, CLI, swarm all subscribe.
    pub bus_tx: broadcast::Sender<BusEvent>,
    /// HUD port where the axum server is bound. Published by `hud::serve`
    /// after bind succeeds. Atomic because every IPC `hud_url` call reads
    /// it; locking the journal mutex just to read an int is wasteful and
    /// can induce async stalls.
    pub hud_port: AtomicU16,
}

impl AppState {
    pub fn bootstrap() -> anyhow::Result<Self> {
        let profile_id = ProfileId::default();
        let profile_root = profiles::resolve_root(&profile_id)?;
        let journal = Journal::open(&profile_root)?;
        let (bus_tx, _) = broadcast::channel(1024);

        Ok(Self {
            inner: Arc::new(Inner {
                profile_id: RwLock::new(profile_id),
                profile_root: RwLock::new(profile_root),
                journal: Mutex::new(journal),
                bus_tx,
                hud_port: AtomicU16::new(0),
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

    pub fn journal(&self) -> parking_lot::MutexGuard<'_, Journal> {
        self.inner.journal.lock()
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
        let (bus_tx, _) = broadcast::channel(1024);
        Self {
            inner: Arc::new(Inner {
                profile_id: RwLock::new(ProfileId::default()),
                profile_root: RwLock::new(PathBuf::new()),
                journal: Mutex::new(journal),
                bus_tx,
                hud_port: AtomicU16::new(0),
            }),
        }
    }
}

// Re-exports
pub use super::bus::BusEvent;
