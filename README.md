# Atlas OS

> **Agent Engineering Operating System — a single-binary, Tauri 2 + Rust + SvelteKit desktop shell for orchestrating swarms of AI coding agents.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Stack: Rust 1.84](https://img.shields.io/badge/Rust-1.84%2B-orange.svg)](https://www.rust-lang.org/)
[![Stack: Tauri 2](https://img.shields.io/badge/Tauri-2-orange.svg)](https://tauri.app/)
[![Stack: SvelteKit 2](https://img.shields.io/badge/SvelteKit-2-FF3E00.svg)](https://kit.svelte.dev/)

Atlas OS is an Agent Engineering Operating System (AEOS) where the Rust core
outlives the webview. The HUD Mission Control is a local axum WebSocket
server: when the Tauri webview crashes, the agent swarm keeps running; when
the user reopens the window, the live event stream resumes from where it
left off.

## Why Atlas OS

The current landscape of AI coding CLIs (OpenCode, Codex CLI, Antigravity,
Gemini CLI, Claude Code, aionUI, Aider…) is fragmented:

- Each one reissues the same model-bridge, file-watcher, audit-journal and
  approval-flow code.
- Most are Node-only and die when the renderer or terminal dies.
- Multi-agent swarms (Sakana AI's Fugu, AI-Scientist) and
  model-fusion routers (OpenRouter Fusion) typically live in research
  scripts, not production shells.

Atlas OS takes the opposite stance: a **single-binary, cross-platform,
AEOS-grade runtime** that any agent engine — present or future — can plug
into via the shared Kernel Bus, Journal and HUD contract.

## Architecture at a glance

```
┌──────────────────────────────────────────────────────────────────┐
│  Tauri 2 desktop shell  (WebView2 / WKWebView / WebKitGTK)        │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │   SvelteKit 2 — HUD Mission Control (RFC 24)             │    │
│  │   Renders a live Kanban/cards view streamed via WS.       │    │
│  └────────────────────┬─────────────────────────────────────┘    │
│         invoke()      │      ws://127.0.0.1:<ephemeral>/ws        │
│                       v                                            │
│  ┌──────────────────────────────────────────────────────────┐    │
│  │   Rust core (atlas-os lib crate, RFC 25 §2)          │    │
│  │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │    │
│  │   │ AppState    │  │ Journal     │  │ Kernel Bus  │       │    │
│  │   │ (RFC 25 §3)│  │ SQLite+vec  │  │ broadcast   │       │    │
│  │   └─────────────┘  └─────────────┘  └─────────────┘       │    │
│  │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │    │
│  │   │ HUD axum    │  │ LSP host    │  │ Profiles    │       │    │
│  │   │ server      │  │ (tower-lsp) │  │ (Hermes)    │       │    │
│  │   └─────────────┘  └─────────────┘  └─────────────┘       │    │
│  │   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │    │
│  │   │ Skills      │  │ MCP client  │  │ Orchestrator│       │    │
│  │   │ (RFC 06)    │  │ (RFC 07)    │  │ (RFC 04)    │       │    │
│  │   └─────────────┘  └─────────────┘  └─────────────┘       │    │
│  └──────────────────────────────────────────────────────────┘    │
│                                                                   │
│  Headless CLI (`atlas` binary, RFC 08) shares the same lib.    │
└──────────────────────────────────────────────────────────────────┘
```

See `docs/adr/` for the four architectural decision records that pin the
choices behind Atlas OS:

- [ADR 0001 — Tauri 2 as the desktop shell](./docs/adr/0001-tauri-2-desktop-shell.md)
- [ADR 0002 — SQLite + sqlite-vec as the journal and vector store](./docs/adr/0002-sqlite-and-sqlite-vec.md)
- [ADR 0003 — HUD as a localhost axum WebSocket server](./docs/adr/0003-hud-localhost-axum-websocket.md)
- [ADR 0004 — Profiles layout under ~/.opencode/profiles/<id>/](./docs/adr/0004-profiles-layout.md)

## Project layout

```
atlas-os/
├── Atlas OS/              27 RFCs (00–26) — the spec
├── docs/adr/                 Architectural Decision Records
├── src-tauri/                Rust core (Tauri 2 + axum HUD + LSP + journal + CLI)
│   ├── src/
│   │   ├── core/             AppState, Kernel Bus, IPC
│   │   ├── journal/          SQLite + sqlite-vec + schema
│   │   ├── hud/              axum WebSocket server (RFC 24)
│   │   ├── lsp/              Tower-LSP host (Phase 2)
│   │   ├── skills/           Skill loader (RFC 06)
│   │   ├── profiles/         Hermes-style multi-profile (RFC 25 §4)
│   │   ├── cli/              Headless CLI entrypoint (RFC 08)
│   │   ├── main.rs           Tauri desktop entrypoint
│   │   └── lib.rs            Re-exported public API
│   ├── capabilities/         Tauri 2 permission files
│   ├── Cargo.toml            Rust manifest (workspace root)
│   └── tauri.conf.json       Tauri 2 config
├── src/                      SvelteKit frontend (CSR-only, Svelte 5 runes)
│   ├── routes/+page.svelte   HUD Mission Control landing
│   ├── lib/stores/hud.ts     Kernel Bus WS store
│   └── app.d.ts
├── skills/prompt-clarify/     Sample skill (RFC 23 §7.2)
├── docs/adr/                  ADRs
├── package.json               pnpm 9 manifest
├── svelte.config.js           adapter-static, Svelte 5 runes
├── vite.config.ts             dev host/port baked in for Tauri
└── AGENTS.md                  AI agent conventions for this repo
```

## Stack

- **Rust 1.84+**, edition 2021.
- **Tauri 2** desktop shell; WebView2 / WebKitGTK / WKWebView by OS.
- **SQLite** + `sqlite-vec` (bundled via `rusqlite`) for journal + embeddings.
- **`fastembed-rs`** (ONNX, no Python) for embeddings — optional feature.
- **axum 0.7** + WebSocket listener on `127.0.0.1:0` — survives webview
  crashes (ADR 0003).
- **tower-lsp 0.20** for the LSP host (proxy multiplexer).
- **SvelteKit 2 / Svelte 5 runes**, `adapter-static` — CSR only.
- **pnpm 9+** (NEVER npm).

## Install & run

Requires Rust 1.84+, Node 20+, pnpm 9+.

```bash
pnpm install
pnpm tauri:dev    # desktop app with hot reload (Tauri + Vite)
```

The HUD Mission Control URL is printed in the Rust logs once axum binds
(look for `HUD Mission Control listening`). The SvelteKit UI auto-connects
to it via Tauri IPC.

Headless CLI sanity checks:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin opencode -- mission new "test prompt"
cargo run --manifest-path src-tauri/Cargo.toml --bin opencode -- hud
cargo run --manifest-path src-tauri/Cargo.toml --bin opencode -- profile list
cargo run --manifest-path src-tauri/Cargo.toml --bin opencode -- audit -n 10
cargo run --manifest-path src-tauri/Cargo.toml --bin opencode -- journal -n 10
```

## Lint / test

| Target     | Command                                                                          |
| ---------- | -------------------------------------------------------------------------------- |
| Rust check | `cargo check --manifest-path src-tauri/Cargo.toml --all-features`                |
| Rust lint  | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` |
| Rust fmt   | `cargo fmt --manifest-path src-tauri/Cargo.toml --check`                         |
| Rust test  | `cargo test --manifest-path src-tauri/Cargo.toml --lib`                          |
| Frontend   | `pnpm check && pnpm lint && pnpm test`                                           |

## Status

**Phase 1 ~95% complete** — engines (Prompt, Planning, Coding, Validation, Repair,
Learning, Skills, Supervisor), HUD Mission Control (annotated diffs, step pills,
model swaps, autoresearch card, graph view), headless CLI with `exec/audit/journal`
namespaces, multi-profile. **Phase 1.5 — RFC 28 External Tool Integration**:
§D AuditLog YAML export ✅, §A Karpathy autoresearch loop ✅, §C graphify-pattern
mission graph (`petgraph` + `tree-sitter` feature-gated) ✅, §B Microsoft Intelligent
Terminal ACP server planned (Phase 1.5d). The 29 RFCs (00–28) in `Atlas OS/`
describe the roadmap through Phase 7.

## License

MIT — see [LICENSE](./LICENSE).
