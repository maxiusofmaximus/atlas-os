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
| Rust check | `cargo check --manifest-path src-tauri/Cargo.toml`                               |
| Rust lint  | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` |
| Rust fmt   | `cargo fmt --manifest-path src-tauri/Cargo.toml --check`                         |
| Rust test  | `cargo test --manifest-path src-tauri/Cargo.toml --lib`                          |
| Frontend   | `pnpm check && pnpm lint && pnpm test`                                           |

> **NOTA (M45):** `cargo check/test --all-features` no se usa en Windows con
> toolchain rustc 1.96: `ort-sys` (vía `fastembed`) provoca un ICE del
> compilador, no fixeable desde este repo (ver `docs/adr/0002-sqlite-and-sqlite-vec.md`).
> Cobertura de features: build default + `cargo check --no-default-features --features "tauri,cli"`.

## Status

**Phases 0–5 completas, Phases 2–5 con toda la spec de su RFC materializada:**

- **Phase 1** — engines (Prompt, Planning, Coding, Validation, Repair, Learning, Skills, Supervisor), HUD Mission Control (annotated diffs, step pills, model swaps, autoresearch card, graph view), headless CLI, multi-profile. ✅
- **Phase 1.5 — RFC 28 External Tool Integration**: §D AuditLog YAML export ✅, §A Karpathy autoresearch loop ✅, §C graphify-pattern mission graph ✅, §B Intelligent Terminal ACP server ✅, §E Firecrawl web ingestion ✅, §F Windows Toast ✅, §G Calendar WRITE ✅ (READ diferido), §H reset-window notifications ✅.
- **Phase 2 — RFC 04 Multi-model Orchestration** (sub-fases 2.0→2.4): Registry + Provider Normalization + Routing Policy + Aggregation + Auto-routing Classifier + **Feedback Loop** (`AffinityIndex` arc-swap + `RoutingStrategy::Mf` + `atlas models refresh`). ✅
- **Phase 3 — RFC 10 Research Engine** (sub-fases 3.0→3.5): Foundation M25 + Docs gateway (Context7Max adapter, `atlas research docs`) + Document ingestion (`atlas research ingest`) + Collective Engineering Intelligence (4 scorers + fail-safe, `atlas research query`) + Hands-on notes + ramas (`atlas research note/branches`) + `probe_feasibility` (M27 cache) + grill gate en `atlas plan`. ✅
- **Phase 4 — RFC 05 Swarm** (sub-fases 4.0→4.5): M29 registry + WorktreeManager (git CLI) + role presets agency-agents port (`atlas swarm presets/start`) + pool paralelo (FileLockRegistry + topología RFC 05) + mailbox + `agent_resume` (`atlas swarm send/inbox`) + auto-rebase CN-003 + **Swarm Console HUD** (floor 2D + mailbox drawer + checks button). ✅
- **Phase 5 — Learning + Compression** (sub-fases 5.0→5.4): M30 learned_rules + YAML `.opencode/rules/` + Reflection Engine formal (dedup + promote/deprecate, `atlas learn`) + compresión de skills Jaccard (`atlas skill compress`) + System One compaction (`atlas learn compact/summary`) + ajuste dinámico de prompts (hints desde reglas consultables). ✅
- **Phase 6 — Execution Supervisor completo** (sub-fases 6.0→6.2): `resume_state` reanudación desde cualquier estado (desde el último checkpoint) + **Observer web del Journal** (`GET /hud/journal` paginado + `<JournalObserver.svelte>`). El state machine + doom_loop + heartbeats + checkpoints ya existían de RFC 19. ✅
- **Phase 7 — Security & Compliance** (sub-fases 7.0→7.3): firmas de skills SHA-256 (+ `.checksum` sidecar + install gate fail-safe), sandbox levels (`SandboxLevel` + `approval_for` mapping RFC 18 §2/§6), compliance skills bundled (OWASP Top 10, GDPR, HIPAA — engine `security`), supply-chain install gate determinista (`atlas security gate` — typosquat + postinstall + env-var). ✅
- **Phase 8 — UI v2 accesible desde cualquier dispositivo** (sub-fases 8.0→8.5): FTS5 search sobre journal_events + frecency zoxide port (`atlas journal --query` / `atlas swarm jump`), Skill Picker iluminado/grisado (`atlas skill pick` ★Suggested/·Dimmed), VRAM/RAM/cost monitor (`atlas monitor` + `BusEventKind::HardwareSnapshot` + readers fail-safe), remote auth bearer + `GET /remote/status` (`atlas hud --auth-status/--rotate-token`, OIDC discovery fijado para swap futuro), Sister IDE-in-a-terminal (Document Model propio + `atlas sister` + binario `atlas-tui`), remote-live dual-PC RustDesk lateral AGPL jamás bundling (`atlas remote status/guide/serve/connect`). ✅
- **Phase 9 — Mejoras profundas** (sub-fases 9.0→9.4, M32-M36): findings.json schema + validator (`AuditReport`/`SecurityFinding` + `atlas audit validate`/`--json`, patrón Cloudflare), Laya como 4º backend del classifier (`ClassifierKind::Laya` std-only MVP + gate `laya`, el "System One" real diferido post-audit), AST Context Engine (`context/ast.rs` `AstSymbol` + `presence_boost` → Picker + `confidence_for_symbol` → LSP, Journal M34 tabla `ast_symbols`), dependency-cruiser límites de capas (`.dependency-cruiser.cjs` 4 reglas + `pnpm arch`), LSP Confidence por símbolo (`lsp/confidence.rs` MVP tower-lsp hover/diagnostics). ✅

- **Phase 10 — Plataforma abierta** (sub-fases 10.0→10.3, M37-M40): Skill SDK público (`atlas skill new` + `validate_skill_id` RFC 06 §1), marketplace local firmado (`atlas skill install` firma OBLIGATORIA + `atlas skill publish`), remixing de skills (`atlas skill fork` + `remixed_from` provenance), learning social (`atlas learn export/import` reglas firmadas + dedup). ✅

- **Phase 11 — Mobile testing** (sub-fases 11.0→11.1, M41-M42): `atlas mobile status/guide/run` (artemis lateral Python/uv, jamás bundling — RFC 38) + MCP template (`atlas mobile mcp-template [--write]` — 5 tools tipados, merge preserva servidores). Validación del operador: dispositivo físico Android. ✅
- **Phase 12 — Distribución y red** (sub-fases 12.0→12.2, M43-M45): Semgrep/CodeQL stages externos (`atlas validate --semgrep/--codeql` → `AuditReport`, fail-safe), marketplace git-based (`atlas skill install <name> --from <git-url>`, firma obligatoria intacta), cleanup preexistente (hud gating `--no-default-features` 37 errores → 0, flaky selfdiscover serializado, bundled gates, ort-sys ICE documentado). ✅
- **Phase 14 — Mobile companion + MaxAppsHub** (RFC 41, sub-fases 14.0→14.2, M46-M48): Tauri Android build (`com.opencode-os.app`, `mobile_entry_point` en `lib.rs`, HOME fix a storage interno) → GitHub Release `v0.1.0-android` (APK debug firmado) → MaxAppsHub entry (`AppRegistry.kt` packageId `com.opencode_os.app` + `<queries>` visibilidad) → validación física end-to-end en TECNO KI7 (instalación, arranque, HUD Mission Control WS connected, artemis completó "Abre la app Atlas OS" en ~106s/3 steps). ✅

- **Phase 15 — Release/Distribution Hardening**: Versionado coherente (source of truth `Cargo.toml`, tag `vX.Y.Z` parejo con `versionName`, `versionCode` = minor*1000+patch) → `.github/workflows/release-android.yml` con gates (versionName == tag, cert keystore == cert APK) → release `v0.1.1` publicada con firma consistente (`cf091fd2` = keystore como secret) → validación física del update en TECNO KI7 vía MaxAppsHub (0.1.0 → 0.1.1, "Actualizada", Atlas arranca, HUD WS connected, artemis smoke PASS). ✅

- **Phase 16 — Laya re-audit + cierre Roadmap v2**: re-audit periódico documentado con evidencia fechada (crates.io `laya 0.1.1`, GitHub 1 contributor, `tokenizers` 0.21, sin `rand` directo) → veredicto: criterio de desbloqueo NO cumplido → Phase 13 permanece BLOQUEADA. Roadmap v2 agotado: 11 ✅ / 12 ✅ / 14 ✅ / 15 ✅. Roadmap v3 requiere decisión del operador + RFC propio. ✅

- **Phase 17 — Baseline de verificación + higiene de lint**: `pnpm lint` recuperado a verde (prettier de `release-android.yml` + `.dependency-cruiser.cjs`, `.prettierignore` para archivos de trabajo del operador), header HUD evidencia `v0.1.1` en el bundle compilado, baseline `cargo test --lib` 1103 ok. ✅

- **Phase 18 — ACP host loop real (Phase 1.5d closeout)**: `session/prompt` ejecuta el dispatch real (`delegate::DelegateOutcome` → `PromptPlan`), cwd por sesión, `exec step` despacha al CLI real y cierra con `EndTurn`, refusal conservado solo para no-soportados. 42 tests acp verdes, clippy/fmt/check limpios. ✅

- **Phase 18.1 — ACP cancel in-flight**: `exec step` corre dentro de `RequestCancellation::run_until_cancelled`; un `$/cancel_request` aborta el CLI y responde `StopReason::Cancelled` (mandato ACP). 44 tests acp verdes. ✅

- **Phase 19 — LSP real v1**: `AtlasLspBackend` (tower-lsp 0.20) con initialize+hover+did_open sobre stdio cuando stdout es pipe/`ATLAS_LSP_STDIO=1`; desktop sigue parked. 9 tests lsp verdes, clippy/fmt/check limpios. ✅

- **Phase 20 — Journal refactor (steps 1-4, CERRADA)**: Section H, `dag_mode`, `ModelInvocationRow`/record/read y el split por dominio de `journal/tests.rs` (15 archivos en `journal/tests/`); 228 tests journal / 1106 lib verdes; check/default/dag_mode/fmt/clippy limpios. ✅

- **Phase 21 — Audit panic!/unwrap()**: cancelada como cambio; las coincidencias son todas test-only (`#[cfg(test)]`). Documentado. ✅

**ROADMAP v1 COMPLETO** (Phases 0-10, 33 RFCs, 1103 tests Rust + 67 frontend). **Roadmap v2**: Phase 11 (mobile testing ✅) + Phase 12 (distribución ✅) + Phase 14 (distribución Android ✅ físicamente validada) + Phase 15 (Release/Distribution Hardening ✅) — **Phase 13 (Laya real) BLOQUEO EN REVISIÓN**: re-audit 2026-10-03 (`scripts/laya_reaudit.ps1`) muestra `laya 0.1.1` pero **2 contributors**, cumpliendo el eje mantenedores ≥2 (pendiente confirmar write-access) → **requiere decisión del operador**. Roadmap v2 agotado; **Roadmap v3 iniciado** (`research/50`, decisión del operador 2026-10-03): prioridad **Windows Calendar real** (RFC 28 §G). Phase 13 sigue como **propuesta** (`research/49`) sin implementar.

## License

MIT — see [LICENSE](./LICENSE).
