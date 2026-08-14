// Atlas OS — Toast dispatcher (RFC 28 Section F).
//
// Two implementations of the dispatch protocol:
//
// * `ToastDispatcher::Win` — wraps `winrt_toast_reborn::ToastManager`
//   and produces WinRT Toasts with `on_activated`, `on_dismissed`,
//   `on_failed` callbacks. WinRT `ToastManager` is **not `Send`** —
//   Toast activation happens via COM apartment-threaded callbacks,
//   which can only fire on the thread that registered the AUMID.
//   We therefore dispatch on a dedicated std::thread; the `Win`
//   variant hides an mpsc channel + worker thread, so the scheduler
//   driver (a `tokio::spawn` task on a different thread) never
//   touches the WinRT object directly.
//
// * `ToastDispatcher::Stub` — fallback used when the binary is built
//   with `--features toast` on a non-Windows target (e.g., CI for
//   Linux). The stub logs the payload via `tracing::info!` and
//   pretends the toast fired and was dismissed with `TimedOut`. That
//   lets us assert the driver loop end-to-end without WinRT.
//
// Both variants implement the inherent `dispatch(&payload) ->
// DispatchOutcome` method with the same signature, so callers do not
// need conditional compilation. AUMID registration is separately
// idempotent via `register_aumid()` (Windows-only).

use std::path::Path;
use std::sync::mpsc;
use std::thread;

use chrono::Utc;

use crate::toast::error::{Result as ToastResult, ToastFacadeError};
use crate::toast::payload::ToastPayload;

#[derive(Clone, Debug)]
pub enum DispatchOutcome {
    Activated { action_arg: Option<String> },
    Dismissed { reason: String },
    Failed { error: String },
}

#[cfg(windows)]
pub const AUMID: &str = "dev.opencode.OpenCodeOS.HUD";

#[cfg(windows)]
pub const DISPLAY_NAME: &str = "Atlas OS — Mission Control";

#[cfg(windows)]
pub fn register_aumid(icon_path: Option<&Path>) -> ToastResult<()> {
    use winrt_toast_reborn as wtr;
    match wtr::register(AUMID, DISPLAY_NAME, icon_path) {
        Ok(()) => Ok(()),
        Err(e) => Err(ToastFacadeError::AumidRegister(format!("{e:?}"))),
    }
}

#[cfg(not(windows))]
pub fn register_aumid(_icon_path: Option<&Path>) -> ToastResult<()> {
    Ok(())
}
/// Channelled handle into the WinRT worker thread. Dropping the
/// handle closes the channel and the worker exits.
#[cfg(windows)]
pub struct WinToastHandle {
    tx: std::sync::Mutex<mpsc::Sender<(ToastPayload, mpsc::Sender<DispatchOutcome>)>>,
    worker: Option<thread::JoinHandle<()>>,
}

#[cfg(windows)]
impl WinToastHandle {
    fn new(aumid: &'static str) -> Self {
        let (tx, rx) = mpsc::channel::<(ToastPayload, mpsc::Sender<DispatchOutcome>)>();
        let worker = thread::Builder::new()
            .name("opencode-toast-win".into())
            .spawn(move || {
                let manager = winrt_toast_reborn::ToastManager::new(aumid);
                for (payload, response_tx) in rx {
                    if payload.id == -1 {
                        // Poison pill — drain and exit.
                        continue;
                    }
                    let outcome = winrt_dispatch(&manager, &payload);
                    let _ = response_tx.send(outcome);
                }
            })
            .expect("spawn toast worker thread");
        Self {
            tx: std::sync::Mutex::new(tx),
            worker: Some(worker),
        }
    }

    fn dispatch(&self, p: &ToastPayload) -> DispatchOutcome {
        let (resp_tx, resp_rx) = mpsc::channel::<DispatchOutcome>();
        if let Ok(guard) = self.tx.lock() {
            if let Err(_e) = guard.send((p.clone(), resp_tx)) {
                return DispatchOutcome::Failed {
                    error: "worker thread exited".into(),
                };
            }
        } else {
            return DispatchOutcome::Failed {
                error: "worker lock poisoned".into(),
            };
        }
        resp_rx.recv().unwrap_or_else(|_| DispatchOutcome::Failed {
            error: "worker dropped response".into(),
        })
    }
}

#[cfg(windows)]
impl Drop for WinToastHandle {
    fn drop(&mut self) {
        // Drop the sender to close the channel. The worker drains its
        // rx iterator and exits. We then join the worker thread to
        // ensure clean shutdown (no panics leak into the host).
        if let Ok(guard) = self.tx.lock() {
            let _ = guard.send((
                ToastPayload {
                    id: -1,
                    kind: crate::toast::payload::ToastKind::Info,
                    title: String::new(),
                    body: None,
                    deep_link: None,
                    fire_at: 0,
                },
                mpsc::channel().0,
            ));
        }
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

#[cfg(windows)]
fn winrt_dispatch(manager: &winrt_toast_reborn::ToastManager, p: &ToastPayload) -> DispatchOutcome {
    use winrt_toast_reborn as wtr;
    let mut toast = wtr::Toast::new();
    toast.text1(&p.title);
    if let Some(body) = &p.body {
        toast.text2(wtr::Text::new(body));
    }
    if let Some(link) = &p.deep_link {
        toast.action(wtr::Action::new("Open", "open", link.as_str()));
    }
    let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let slot_activated = slot.clone();
    let slot_dismissed = slot.clone();
    let slot_failed = slot.clone();
    let m = manager.clone();
    let _ = m.on_activated(None, move |act| {
        *slot_activated.lock().unwrap() = Some(match act {
            Some(a) => DispatchOutcome::Activated {
                action_arg: Some(a.arg.clone()),
            },
            None => DispatchOutcome::Activated { action_arg: None },
        });
    });
    let m = manager.clone();
    let _ = m.on_dismissed(move |reason| {
        let reason_str = match reason {
            Ok(r) => match r.reason {
                wtr::DismissalReason::UserCanceled => "userCanceled",
                wtr::DismissalReason::ApplicationHidden => "applicationHidden",
                wtr::DismissalReason::TimedOut => "timedOut",
            }
            .to_string(),
            Err(e) => format!("error:{e:?}"),
        };
        *slot_dismissed.lock().unwrap() = Some(DispatchOutcome::Dismissed { reason: reason_str });
    });
    let m = manager.clone();
    let _ = m.on_failed(move |e| {
        *slot_failed.lock().unwrap() = Some(DispatchOutcome::Failed {
            error: format!("{e:?}"),
        });
    });
    let m = manager.clone();
    match m.show(&toast) {
        Ok(()) => {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(25);
            loop {
                if let Some(o) = slot.lock().unwrap().take() {
                    return o;
                }
                if std::time::Instant::now() > deadline {
                    return DispatchOutcome::Dismissed {
                        reason: "timedOut".to_string(),
                    };
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
        Err(e) => DispatchOutcome::Failed {
            error: format!("{e:?}"),
        },
    }
}

/// Agnostic dispatcher. On Windows we keep a channelled handle so the
/// rest of the code stays `Send`; on non-Windows we use a trivial
/// logging stub.
pub enum ToastDispatcher {
    #[cfg(windows)]
    Win(WinToastHandle),
    Stub,
}

impl ToastDispatcher {
    pub fn new_default() -> Self {
        #[cfg(windows)]
        {
            ToastDispatcher::Win(WinToastHandle::new(AUMID))
        }
        #[cfg(not(windows))]
        {
            ToastDispatcher::Stub
        }
    }

    #[cfg(windows)]
    pub fn new_powershell_for_tests() -> Self {
        ToastDispatcher::Win(WinToastHandle::new(
            winrt_toast_reborn::ToastManager::POWERSHELL_AUM_ID,
        ))
    }

    pub fn dispatch(&self, p: &ToastPayload) -> DispatchOutcome {
        match self {
            #[cfg(windows)]
            ToastDispatcher::Win(h) => h.dispatch(p),
            ToastDispatcher::Stub => {
                tracing::info!(
                    target: "opencode::toast",
                    kind = %p.kind,
                    title = %p.title,
                    "stub dispatch"
                );
                DispatchOutcome::Dismissed {
                    reason: "timedOut".to_string(),
                }
            }
        }
    }
}

pub fn outcome_to_reason(o: &DispatchOutcome) -> &'static str {
    match o {
        DispatchOutcome::Activated { .. } => "activated",
        DispatchOutcome::Dismissed { reason } => match reason.as_str() {
            "userCanceled" => "userCanceled",
            "applicationHidden" => "applicationHidden",
            "timedOut" => "timedOut",
            _ => "unknown",
        },
        DispatchOutcome::Failed { .. } => "failed",
    }
}

pub fn outcome_to_status(o: &DispatchOutcome) -> (&'static str, i64) {
    let now = Utc::now().timestamp_millis();
    match o {
        DispatchOutcome::Activated { .. } | DispatchOutcome::Dismissed { .. } => ("dismissed", now),
        DispatchOutcome::Failed { .. } => ("failed", now),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_to_reason_recognises_known_strings() {
        assert_eq!(
            outcome_to_reason(&DispatchOutcome::Dismissed {
                reason: "userCanceled".into()
            }),
            "userCanceled"
        );
        assert_eq!(
            outcome_to_reason(&DispatchOutcome::Dismissed {
                reason: "applicationHidden".into()
            }),
            "applicationHidden"
        );
        assert_eq!(
            outcome_to_reason(&DispatchOutcome::Dismissed {
                reason: "timedOut".into()
            }),
            "timedOut"
        );
        assert_eq!(
            outcome_to_reason(&DispatchOutcome::Dismissed {
                reason: "exotic".into()
            }),
            "unknown"
        );
        assert_eq!(
            outcome_to_reason(&DispatchOutcome::Activated { action_arg: None }),
            "activated"
        );
        assert_eq!(
            outcome_to_reason(&DispatchOutcome::Failed {
                error: "boom".into()
            }),
            "failed"
        );
    }

    #[test]
    fn outcome_to_status_routes_correctly() {
        let (s, _) = outcome_to_status(&DispatchOutcome::Activated { action_arg: None });
        assert_eq!(s, "dismissed");
        let (s, _) = outcome_to_status(&DispatchOutcome::Dismissed {
            reason: "userCanceled".into(),
        });
        assert_eq!(s, "dismissed");
        let (s, _) = outcome_to_status(&DispatchOutcome::Failed {
            error: "boom".into(),
        });
        assert_eq!(s, "failed");
    }

    #[test]
    #[cfg(not(windows))]
    fn stub_dispatch_returns_timed_out() {
        let d = ToastDispatcher::Stub;
        let p = ToastPayload {
            id: 1,
            kind: crate::toast::payload::ToastKind::Info,
            title: "x".into(),
            body: None,
            deep_link: None,
            fire_at: 0,
        };
        match d.dispatch(&p) {
            DispatchOutcome::Dismissed { reason } => assert_eq!(reason, "timedOut"),
            other => panic!("expected dismissed-timeout, got {other:?}"),
        }
    }

    #[test]
    #[cfg(not(windows))]
    fn register_aumid_is_noop_on_non_windows() {
        register_aumid(None).unwrap();
    }
}
