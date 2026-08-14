# ADR 0003 — HUD as a localhost axum WebSocket server (separate from the Tauri webview)

- **Status**: Accepted
- **Date**: 2026-07-20
- **Decision owner**: RFC 24 §1, RFC 25 §2
- **Supersedes**: —
- **Superseded by**: —

## Context

The HUD Mission Control (RFC 24) shows live agent state from the Kernel Bus.
Two failure modes the architecture must tolerate:

1. **Webview crash**: the user-facing Tauri webview dies (GPU reset, OOM,
   extension-induced segfault). The agent loop MUST NOT die.
2. **Mobile remote access** (RFC 24 §16): a phone on the same LAN should
   be able to open Mission Control while the desktop app is the source of
   truth.

If the HUD lived inside the Tauri webview's JS context, a webview crash
would drop all HUD state and we could not point a mobile browser at it.

## Decision

Ship the HUD as a local **axum** HTTP + WebSocket server bound to
`127.0.0.1:0` (ephemeral port) on a dedicated OS thread with its own Tokio
multi-thread runtime. The Tauri webview is just _one_ client of this server;
a mobile phone or another local browser can connect to the same WS stream.

- `src-tauri/src/hud/server.rs` — axum router with `/health` and `/ws`.
- The desktop webview loads the SvelteKit page that opens the WS via the
  `hud` store (`src/lib/stores/hud.ts`).
- The exact port is published by `AppState::set_hud_port` after bind and
  persisted to `~/.opencode/profiles/<id>/hud_port.txt` so the headless
  `atlas hud` CLI can recover it later.
- Graceful shutdown: `main.rs` owns a `CancellationToken`; the
  `on_window_event(CloseRequested)` handler cancels it, axum stops accepting,
  active requests drain, and `hud_handle.join()` returns cleanly before
  process exit.

## Consequences

- **Webview crash resiliency**: even if WebView2/WKGTK crashes mid-flight,
  the HUD keeps running. Reopening the webview reconnects to the same WS
  stream and the rolling 200-event buffer repopulates Mission Control.
- **Two runtimes**: the HUD runtime is a sibling of the LSP runtime — they
  never share a single Tokio scheduler, so a misbehaving LSP server cannot
  starve the HUD.
- **No cross-origin restrictions** from the Tauri side: axum enables
  `CorsLayer::permissive()` because the only listener is on `127.0.0.1`.
  Remote access (Phase 16) will tighten this with a pairing token.
- **COOP/COEP headers**: removing `Cross-Origin-Embedder-Policy: require-corp`
  from `tauri.conf.json` was necessary because Vite HMR is cross-origin and
  died under `require-corp` on WebView2 (process exit `0xffffffff`). Re-add
  via Vite `server.headers` once the desktop shell is the only HUD client
  (Phase 8 — UI v2). Tracked as debt in RFC 25.

## Alternatives rejected

- **Tauri-only IPC** (no separate server): forces Mission Control to die
  with the webview. Blocks mobile remote access.
- **gRPC instead of WS**: heavier; WebSocket is enough for a thin event
  stream.
- **Embedding axum inside the same Tokio runtime as Tauri**: simpler, but
  the Tauri event loop is single-threaded and a slow HTTP handler could
  block window events.
