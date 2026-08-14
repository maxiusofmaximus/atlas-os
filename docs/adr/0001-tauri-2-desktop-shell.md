# ADR 0001 — Adopt Tauri 2 as the desktop shell

- **Status**: Accepted
- **Date**: 2026-07-20
- **Decision owner**: RFC 25 §2
- **Supersedes**: —
- **Superseded by**: —

## Context

Atlas OS is an Agent Engineering Operating System whose Rust core must
outlive a webview crash (the agent loop cannot die because the renderer the
user happens to be looking at died). We need a desktop shell that:

1. Lets the Rust core own the lifecycle of the HUD axum server, LSP host,
   SQLite journal, and Tauri IPC handlers.
2. Keep native dependencies minimal (single-binary distribution, RFC 25 §11).
3. Bundle a small, web-standards-compliant webview for the Mission Control UI.
4. Ship on Windows, macOS, and Linux from the same codebase.

Options considered: Electron, WebView2-only, neutralino, Sciter, Tauri 2.

## Decision

Use **Tauri 2** as the desktop shell.

- Rust hosts the entire kernel (AppState, Journal, Kernel Bus, HUD axum
  server, LSP host, etc.).
- The webview is WebView2 (Win), WKWebView (macOS), WebKitGTK (Linux) — no
  Chromium bundled.
- The desktop binary and the headless `atlas` CLI share the same
  `atlas-os` library crate (RFC 25 §3.9): no Node, no Electron
  main-process split.

## Consequences

- **App bundle size**: tens of MB instead of hundreds (no Chromium).
- **Boot**: HUD axum runs on its own OS thread + Tokio runtime, so a webview
  crash does NOT kill the mission. The user can close and reopen the webview
  without losing agent state.
- **IPC**: frontend talks to Rust via `invoke()` (`core::ipc`).
- **Capability system**: filesystem and shell-open are not on the default
  capability set (ADRs 0003 / 0004 add them per-phase as engines come online).

## Alternatives rejected

- **Electron**: requires Node runtime for the main process + a duplicate JS
  core; loses 1.5–3× memory and adds packaging complexity.
- **WebView2-only**: Windows-only; no Linux/macOS story.
- **neutralino**: smaller, but the Rust ecosystem around it is immature; no
  strong story for a Rust-owned, webview-crash-resilient kernel.
- **Sciter**: proprietary; poor Rust bindings.
