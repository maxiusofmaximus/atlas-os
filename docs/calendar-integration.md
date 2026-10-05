# Calendar Integration

> **Operator guide.** This document is the **user-facing** recipe for RFC 28 §G — the Windows/multi-platform calendar bridge. It covers the two independent surfaces: the **WRITE path** (publish Atlas missions as an iCalendar feed your calendar app subscribes to) and the **READ path** (pull your busy windows so the Planning engine only queues proactive turns when you are actually free).
>
> The Rust contract lives in `src-tauri/src/calendar/*` (`ics_writer`, `ics_route`, `graph_reader`, `auth`, `token`, `queue`, `poller`), the SQLite tables are M18 (`calendar_busy_windows`, `calendar_auth`), and the CLI is `atlas calendar …`. The RFC spec is RFC 28 §G (lines 533-691).

---

## 1. Prerequisites

Atlas OS splits the calendar surface into two Cargo features so you only pay for what you use:

| Feature          | What it adds                                                                     | Deps                                                    |
| ---------------- | -------------------------------------------------------------------------------- | ------------------------------------------------------- |
| `calendar-ics`   | **WRITE** feed (`GET /atlas-calendar.ics`) + ad-hoc/durable `.ics` subscriptions | pure-Rust `ics 0.5` + `icalendar 0.17` (no native deps) |
| `calendar-graph` | **READ** path: Microsoft Graph poller + encrypted refresh-token storage          | `graph-rs-sdk`, `aes-gcm`, `ring`                       |
| `calendar`       | umbrella = `calendar-ics` + `calendar-graph`                                     | both                                                    |

Build with the surfaces you need (default is neither):

```powershell
pnpm install
# WRITE only (cheapest — no auth, no Graph):
pnpm tauri:build -- --features calendar-ics
# Both surfaces:
pnpm tauri:build -- --features calendar
```

The M18 schema migration runs **unconditionally** (schema version is fixed regardless of feature), so `calendar busy list` works even on a build without any calendar feature — only the Graph/ICS _operations_ are gated.

---

## 2. WRITE path — subscribe your calendar to Atlas missions

Atlas publishes its mission/step timeline as an RFC 5545 feed, served from the HUD's axum server (the same `127.0.0.1:<port>` you open for the Mission Control HUD).

```powershell
# 1. Start the HUD (persists its ephemeral port to <profile_root>/hud_port.txt).
atlas hud

# 2. Print the subscription URL. The first call mints an opaque token and
#    persists it to <profile_root>/calendar_ics_token.txt.
atlas calendar feed
# → webcal://127.0.0.1:54321/atlas-calendar.ics?token=6Z0k…Q
```

Paste that `webcal://` URL into your calendar client:

- **Outlook** — _Add calendar ▸ Subscribe from web_, paste the `webcal://` URL, name it "Atlas OS".
- **Google Calendar** — _Other calendars ▸ + ▸ From URL_; replace the `webcal://` prefix with `https://` (Google will not accept `webcal://`; the `http://` form works because the host is `127.0.0.1`).
- **Apple Calendar** — _File ▸ New Calendar Subscription_, paste the URL.

Notes:

- The `token` query parameter gates the feed so another local process cannot scrape your mission titles. It is **base64url of 16 random bytes** (22 URL-safe chars) and is stable across restarts.
- If `atlas calendar feed` prints a "start the HUD" hint instead of a URL, `atlas hud` is not running (no `hud_port.txt`) — start it and re-run.
- Rotate the token by deleting `<profile_root>/calendar_ics_token.txt`; the next `atlas calendar feed` mints a new one, and old subscriptions stop working.

### Ad-hoc and durable `.ics` subscriptions (READ of _other_ calendars)

The WRITE feed is one direction. To pull **someone else's** `.ics` (Outlook/Google/Apple "secret address", a holiday feed, a team calendar) into the busy-window queue:

```powershell
# One-shot: fetch a URL now and replace its busy windows.
atlas calendar sync-ics "https://outlook.office365.com/owa/calendar/…/calendar.ics" -n work

# Durable: register a named subscription, then sync all of them.
atlas calendar subscribe work "https://outlook.office365.com/owa/calendar/…/calendar.ics"
atlas calendar subscriptions          # list with last-sync state
atlas calendar sync-all               # conditional GET; HTTP 304 = no work
atlas calendar unsubscribe work       # removes the subscription (windows stay)
```

---

## 3. READ path — let Atlas see your busy windows (Microsoft Graph)

The Graph poller fills `calendar_busy_windows` with your real events so the Planning engine can hold proactive turns while you are in a meeting.

```powershell
# 1. Authenticate. OAuth device-code flow: the CLI prints a code + URL,
#    you approve it in the browser, and the refresh token is stored
#    AES-256-GCM-encrypted in the M18 `calendar_auth` table.
atlas calendar login

# 2. Pull the next week of events into `graph` busy windows.
atlas calendar sync

# 3. Inspect the stored token claims (aud / scp / tid) — diagnostics only.
atlas calendar status
```

> **Deviation from the RFC:** RFC 28 §G.3.1 originally specified an interactive **wry popup**. The implementation uses the **OAuth device-code** flow instead — it needs no embedded browser, works on every platform, and keeps the single-binary invariant. The RFC checklist is annotated accordingly (G.6 item 6).

---

## 4. Inspecting and hand-authoring busy windows

```powershell
atlas calendar busy list -n 20      # newest first
atlas calendar busy count
atlas calendar busy add <start_ms> <end_ms> -l "focus block" -w 1.0
atlas calendar busy rm <id>
```

Windows are half-open `[start, end)` epoch-milliseconds. `weight` is the planning pressure (default `1.0`); a window with `weight >= threshold` blocks a proactive turn.

## 5. Availability and proactive turns

```powershell
# Can a proactive turn of ~15 min (900_000 ms) start right now?
atlas calendar availability -e 900000 -w 1.0
# → availability: RUN NOW (free for N ms) | WaitUntil(ts) | BLOCKED

# Persisted proactive-turn policy (eta / weight / horizon).
atlas calendar policy --eta-ms 900000 --weight 1.0 --horizon-ms 86400000 --enable

# One probe: availability + backlog → supervisor action.
atlas calendar proactive
```

---

## 6. Troubleshooting

| Symptom                                                    | Cause / fix                                                                                                                                                                                                         |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `calendar feed` prints the "start the HUD" hint            | `atlas hud` is not running, or the profile has no `hud_port.txt`. Start `atlas hud`.                                                                                                                                |
| `GET /atlas-calendar.ics` returns **404**                  | The build lacks `calendar-ics`. Rebuild with `--features calendar-ics`.                                                                                                                                             |
| Feed returns **401 `missing or invalid token`**            | The `token=` query param is missing or stale. Re-run `atlas calendar feed`; delete the token file to rotate.                                                                                                        |
| `calendar login` / `sync` / `status` are "no such command" | The build lacks `calendar-graph`. Rebuild with `--features calendar` (or `calendar-graph`).                                                                                                                         |
| Graph events missing after `sync`                          | The heuristic filter keeps events whose `subject` mentions "Atlas"/"OpenCode"; other events are ignored by design. Verify `atlas calendar busy list` shows `graph` rows, then check `atlas calendar status` claims. |
| Wrong/blank busy windows after moving machines             | The refresh token is encrypted to the device; it is **not** portable. Run `atlas calendar login` again on the new machine.                                                                                          |

**Security note (RFC 28 §G.7):** the Graph refresh token is AES-256-GCM encrypted in SQLite and is **not recoverable** if you lose the device. Run `atlas calendar login` again after moving machines; there is no logout in this revision.

**Rate limits:** the Graph poller runs every 60 s (`24 h = 1440 reqs/day`), far under Graph's default `10 000 reqs / 10 min` app limit.

---

## 7. Attribution

- `src-tauri/src/calendar/ics_writer.rs` — `Uses ics 0.5.8 (MIT OR Apache-2.0) by hummingly. https://github.com/hummingly/ics`
- `src-tauri/src/calendar/ics_route.rs` — ICS READ uses `icalendar 0.17 (MIT) by hoodie. https://github.com/hoodie/icalendar-rs`
- `src-tauri/src/calendar/graph_reader.rs` — `Uses graph-rs-sdk 3.0.1 (MIT) by sreeise. https://github.com/sreeise/graph-rs-sdk`
- `src-tauri/src/calendar/auth.rs` — `Interactive auth via graph-rs-sdk 3.0.1 (MIT). Encryption via aes-gcm 0.10 (MIT OR Apache-2.0).`
