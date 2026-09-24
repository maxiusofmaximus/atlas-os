# Model API Reset-Window Notifications

> **Operator guide.** This document is the **user-facing** recipe for RFC 28 §H — the "feature diferencial" that surfaces a HUD card when a model API returns a `SpendLimitError` (rate-limit 429 or spend-cap 402/403), and fires a `model_ready` Toast notification when the reset window elapses.
>
> The Rust-side contract lives in `src-tauri/src/orchestrator/{error,parse_error,retry,mod}.rs`, `src-tauri/src/journal/model_resets.rs`, and `src-tauri/src/hud/cards.rs`; the Svelte cards are in `src/lib/components/SpendLimitErrorCard.svelte` and `src/lib/components/ModelReadyCard.svelte`. The RFC spec is RFC 28 §H (lines 687-837).

---

## 1. Prerequisites

1. **Atlas OS** built with the `toast` feature (default OFF):

   ```powershell
   pnpm install
   pnpm tauri:build -- --features toast
   ```

   Without `toast`, the `model_resets` SQLite table is still maintained (the schema migration runs unconditionally — M19), but the `model_ready` Toast cards are not enqueued. The HUD still surfaces the `SpendLimitErrorCard` (it's driven by the kernel bus, not the Toast queue).

2. **OmniRoute** gateway (free MIT, OpenAI-compatible) recommended but not required. OmniRoute normalizes upstream provider error envelopes into a single `{ error: { type, status, provider, model, resets_at, ... } }` shape — the parser (`orchestrator::parse_error::parse_omniroute`) handles this format first-class. When OmniRoute is bypassed, the parser falls back to HTTP `Retry-After` header parsing and `x-ratelimit-reset-requests`/`x-ratelimit-reset-tokens` (OpenAI-specific).

3. **Windows 10 build 19041+ or Windows 11** if you want the native Toast surface (the `winrt-toast-reborn` crate uses the AUMID-registered Start Menu shortcut to route deep-links back into the desktop shell — see RFC 28 §F for the Toast subsystem itself).

---

## 2. How it works

```
┌────────────────┐     429 / 402      ┌──────────────────┐
│ Orchestrator   │ ─────────────────▶ │ parse_omniroute() │
│ (turn loop)    │                    │  or Retry-After   │
└────────────────┘                    └────────┬─────────┘
                                               │
                                               ▼
                                       ┌───────────────────┐
                                       │ SpendLimitError   │
                                       │ (provider, model, │
                                       │  resets_at, kind) │
                                       └────────┬──────────┘
                                                │
                    ┌───────────────────────────┼────────────────────────────────┐
                    ▼                            ▼                                ▼
       ┌─────────────────────────┐  ┌──────────────────────────┐    ┌────────────────────────────┐
       │ Journal model_resets    │  │ BusEventKind::           │    │ Toast queue (§F)           │
       │ (M19, idempotent upsert)│  │ SpendLimitObserved       │    │ kind='model_ready'         │
       └─────────────────────────┘  │ → HUD SpendLimitErrorCard│    │ fire_at_ms = resets_at    │
                                     └──────────────────────────┘    └────────────┬───────────────┘
                                                                                │ resets_at arrivals
                                                                                ▼
                                                                    ┌────────────────────────────┐
                                                                    │ Toast fired                │
                                                                    │ → model_ready Toast        │
                                                                    │ → HUD ModelReadyCard       │
                                                                    └────────────────────────────┘
```

The subsystem is **idempotent**: the same `(provider, model, resets_at)` triple persisted twice (e.g. on replay after a crash) returns the same `reset_id` and does not enqueue a second Toast — `Journal::model_reset_upsert` uses an `INSERT OR IGNORE` keyed on the table's UNIQUE index.

A **stale-reset guard** rejects envelopes whose `resets_at` is more than 60 seconds in the past at parse time: the parser yields `Ok(None)` and the orchestrator proceeds with the normal retry loop. This handles clock-drift (RFC 28 §H.8 risk 2).

---

## 3. Configuring thresholds

### 3.1 Retry policy defaults (compiled in `orchestrator/retry.rs`)

| Field                | Default | Description                                                                       |
| -------------------- | ------- | --------------------------------------------------------------------------------- |
| `base_delay`         | 1 s     | Initial exponential backoff delay.                                                |
| `max_delay`          | 30 s    | Backoff cap.                                                                      |
| `bail_out_threshold` | 60 s    | Total wait time after which `RetryPolicy::decide` returns `BailDecision::GiveUp`. |
| `max_attempts`       | 5       | Hard retry limit before giving up.                                                |
| `jitter`             | 0.25    | ±25 % jitter around each backoff delay (uniform via `rand::Rng`).                 |

### 3.2 Per-profile overrides (`<profile_root>/profile.toml`)

```toml
id = "work"
bail_out_threshold_secs = 120   # override the 60 s default
backup_profile_id = "personal"  # clicked by the "Switch Provider" button
```

When the profile file is missing or malformed, the runtime falls back to the in-code defaults (`bail_out_threshold_secs = 60`, `backup_profile_id = None`). The "Switch Provider" button on the `SpendLimitErrorCard` is **disabled** when `backup_profile_id` is `None` — a one-profile setup simply doesn't surface the button.

Edit `profile.toml` by hand — there's no CLI yet (planned: `atlas profile edit`). Reload by restarting the desktop app (the file is read once at boot).

### 3.3 Toast cooldown (10 min per `(provider, model)`)

The scheduler (§F) refuses to enqueue a second `model_ready` Toast for the same `(provider, model)` within 10 minutes of the most recent one. This is RFC 28 §H.8 risk 3 (Toast spam guard).

### 3.4 "Request Increase" cooldown (5 min per provider)

The `SpendLimitErrorCard`'s "Request Increase" button writes a `localStorage` key `opencode.llm.cooldown.request_increase.{provider}` on every click. The button is disabled for 5 minutes (300 000 ms) afterwards — it can't be re-clicked in a panic while the provider dashboard opens in a browser tab. localStorage is best-effort; private-mode sandboxes silently skip the cooldown (the button stays enabled).

---

## 4. Troubleshooting

### 4.1 No Toast appears when `resets_at` elapses

- Confirm `--features toast` was passed to `pnpm tauri:build`. The `toast` feature is default OFF.
- Check the journal:
  ```powershell
  cargo run --manifest-path src-tauri/Cargo.toml --bin opencode -- journal -n 50 |
    Select-String "model_ready|spend_limit_observed"
  ```
  If you see `spend_limit_observed` entries but no `model_ready`, the row never reached `fire_at_ms` — either the desktop app exited before the reset, or `toast_id` is `NULL` (the toast enqueue step was skipped).
- Open the HUD (`atlas hud`) and check the Card column. The `model_ready` Toast payload itself is best verified via the kernel-bus WS stream — `tail -f /tail/model_swaps` does NOT show Toasts; use the WS subscription on `ws://127.0.0.1:<port>/ws`.

### 4.2 The card never appears on rate-limit

- The OmniRoute envelope parser requires `error.resets_at` in RFC-3339 format. If your OmniRoute version omits it, the parser returns `Ok(None)` and the orchestrator falls back to plain retry — no card is enqueued.
- If you're hitting the upstream provider directly (no OmniRoute), confirm the response carries either an HTTP `Retry-After` header (RFC 7231) or the OpenAI-style `x-ratelimit-reset-requests` / `x-ratelimit-reset-tokens` header. OpenAI's Anthropic equivalent is `anthropic-ratelimit-*-reset` — OmniRoute normalizes these to a single `resets_at`; the direct path parses them per-header (see `parse_error.rs::parse_x_ratelimit_reset`).

### 4.3 Stale-reset guard triggered (silent `Ok(None)`)

If `parse_omniroute` logs nothing and the orchestrator proceeds without persisting a `model_resets` row, the stale-reset guard likely fired. The guard rejects any envelope whose `resets_at` is more than 60 s in the past relative to `Utc::now()`. Verify your system clock (`Get-Date` on Windows) and confirm the OmniRoute response's `resets_at` is actually a future timestamp; if your host clock is ahead of UTC, consider `w32tm /resync`.

### 4.4 "Switch Provider" disabled

- The button is enabled only when `profile.toml` declares a non-empty `backup_profile_id`. Edit the file (`<profile_root>/profile.toml`), save, and restart the desktop app.
- The CLI verification:
  ```powershell
  cargo run --manifest-path src-tauri/Cargo.toml --bin opencode -- profile list
  ```
  lists every profile by id. The backup must already exist on disk under `~/.opencode/profiles/<id>/`; the HUD does NOT create it on the fly.

### 4.5 Duplicate Toasts on rapid re-replays

If you see two Toasts back-to-back, the Toast cooldown logic (10 min) likely didn't kick in. Confirm the `model_resets` table's `toast_id` column was linked after the first enqueue:

```sql
SELECT id, provider, model, resets_at, toast_id, toast_dismissed_at
FROM model_resets
WHERE toast_dismissed_at IS NULL
ORDER BY resets_at DESC
LIMIT 10;
```

Rows with `toast_id IS NOT NULL` are linked; rows without one will re-enter the enqueue queue on the next scheduler tick. If both rows exist with the same `(provider, model, resets_at)` triple, the UNIQUE index failed (consider running `PRAGMA integrity_check` on the journal file).

---

## 5. Smoke test

See `tools/reset-window-smoke.ps1` (Item 13). The script mocks a provider response with `Retry-After: 5`, fires a turn through the orchestrator, and validates that:

1. A `model_resets` row was persisted.
2. A `model_ready` Toast was enqueued (with `toast` feature).
3. After 5 seconds, the Toast transitioned to `fired`.
4. Re-running the mock within 10 minutes does NOT enqueue a second Toast.

---

## 6. References

- RFC 28 §H (lines 687-837) — the canonical spec.
- RFC 20 §1.5h — roadmap entry.
- RFC 26 §6 — index entry for `OmniRoute`, `SpendLimitError`, `ModelReady Toast`, `Retry policy`, `Bail-out threshold`, `Backup profile`.
- RFC 24 §3.3 (addendum) — HUD card anatomy for the SpendLimit / ModelReady cards.
- Cline PRs #10207 / #10963 / #10141 (Apache-2.0) — attribution: the j±25% jitter, the SpendLimitError card exempt from auto-retry, the bail-out threshold pattern. See `src-tauri/src/orchestrator/{error,parse_error,retry}.rs` per-module attribution notes.
