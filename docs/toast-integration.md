# Windows Toast Notifications

> **Operator guide.** This document is the **user-facing** recipe for RFC 28 §F — the database-driven Windows Toast surface. Atlas queues a notification in SQLite and a background driver (owned by the desktop process) dispatches it through WinRT, with `on_activated` / `on_dismissed` / `on_failed` callbacks so a clicked toast deep-links back into the HUD.
>
> The Rust contract lives in `src-tauri/src/toast/*` (`manager`, `queue`, `scheduler`, `payload`), the SQLite tables are M17 (`toast_queue`, `toast_history`), and the CLI is `atlas toast …`. The RFC spec is RFC 28 §F (lines 402-532).

---

## 1. Prerequisites

1. **Windows 10 build 19041+ or Windows 11.** The dispatcher wraps WinRT toasts via `winrt-toast-reborn 0.3.8`; on other platforms a stub logs the payload instead.

2. **Build with the `toast` feature** (OFF by default):

   ```powershell
   pnpm install
   pnpm tauri:build -- --features toast
   ```

   The M17 schema migration runs **unconditionally**, so `atlas toast …` can enqueue rows on any build — but only a `toast`-enabled **desktop** build dispatches them to the OS. Without the feature, `atlas toast` is absent (clap reports "no such command").

3. **One-time AUMID registration.** On first boot the desktop app calls `register_aumid()`, which creates a Start Menu shortcut for AUMID `dev.opencode.OpenCodeOS.HUD`. Without that shortcut a no-MSIX desktop app can still _show_ toasts, but clicks cannot route back to the process.

---

## 2. How it works

```
atlas toast queue …          AppState::bootstrap() (desktop process)
      │                              │
      ▼                              ▼
┌─────────────┐  5-s poll   ┌──────────────┐  dispatch  ┌───────────────┐
│ toast_queue │ ──────────▶ │ ToastDriver  │ ─────────▶ │ WinRT Toast   │
│ (SQLite M17)│             │ (tokio task) │            │ (AUMID-routed)│
└─────────────┘ ◀────────── └──────────────┘            └───────┬───────┘
      │            persist outcome                              │ click
      ▼                                                         ▼
┌──────────────┐                                    deep-link back into HUD
│toast_history │                                    (opencode:// …)
└──────────────┘
```

Key point: the **CLI never dispatches a toast**. It only reads/writes the queue for manual edits and introspection. The 5-second driver, the WinRT object, and the deep-link callbacks all live in the desktop process.

---

## 3. CLI

```powershell
# Enqueue now (in 0 s) or schedule a delay.
atlas toast queue model_ready --title "Model ready" --body "gpt-x reset window elapsed" --in 0
atlas toast queue info --title "Queued" --deep-link "opencode://mission/42"

# Inspect (newest first; one JSON object per row, or "[]").
atlas toast list -n 20

# Cancel a still-pending row by id (idempotent).
atlas toast cancel 7
```

**Kinds:** `model_ready`, `turn_end`, `validation_failed`, `calendar_reminder`, `critical`, `info`.

Output is JSON line-delimited so pipes and skills can consume it:

| Verb     | Output                                                                         |
| -------- | ------------------------------------------------------------------------------ |
| `queue`  | `{"queued":true,"id":7,"kind":"model_ready","fire_at":1759…,"deep_link":null}` |
| `list`   | one `{"id":…,"kind":…,"title":…,"status":…,"attempts":…}` per row              |
| `cancel` | `{"cancelled":true,"id":7}` (second call → `false`)                            |

---

## 4. Deep-links and history

- **Deep-link** — pass `--deep-link opencode://…`. On click the driver hands the URL to WinRT `Action::new("Open", "open", link)`; the registered AUMID routes it back into the desktop process, which deep-links the HUD.
- **Dedupe / cooldown** — every `mark_fired` / `mark_dismissed` / `mark_failed` writes a `toast_history` audit row. The reset-window subsystem (RFC 28 §H) uses a 10-minute per-`(provider, model)` cooldown so a flapping provider cannot spam `model_ready`.
- **Crash safety** — the queue is durable SQLite; on boot the driver drains every `pending` row whose `fire_at <= now`, so notifications survive a process restart.

---

## 5. Troubleshooting

| Symptom                               | Cause / fix                                                                                                                                                                           |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `atlas toast …` → "no such command"   | Build lacks `toast`. Rebuild with `--features toast`.                                                                                                                                 |
| Row queued but no toast appears       | The **desktop** process is not running (the CLI does not dispatch). Launch the desktop app; the driver polls every 5 s. On non-Windows, expect the stub log instead of a real toast.  |
| Toast shows but clicking does nothing | AUMID not registered (Start Menu shortcut missing, often a permissions issue). Check the boot log for `register_aumid` warnings; re-run the desktop app with Start Menu write access. |
| Toasts route to the wrong app         | Another app reused `dev.opencode.OpenCodeOS.HUD` (unlikely). WinRT delivers callbacks to the last registrant; the namespace is unique to Atlas OS.                                    |
| Stale/again-firing toast              | Delayed rows whose `fire_at` passed while the process was down fire immediately on next boot — cancel with `atlas toast cancel <id>` if unwanted.                                     |

---

## 6. Attribution

- `src-tauri/src/toast/*.rs` — `Uses winrt-toast-reborn 0.3.8 (MIT) by Md. Iftakhar Awal Chowdhury (AtifChy), fork maintained of winrt-toast 0.1.1. https://github.com/AtifChy/winrt-toast`
- `src-tauri/Cargo.toml` — the `winrt-toast-reborn` entry under `[target.'cfg(windows)'.dependencies]` carries the crate name, version, and MIT license in an inline comment.
