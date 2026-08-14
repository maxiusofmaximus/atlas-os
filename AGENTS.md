# AGENTS.md — Atlas OS project guide for AI coding agents

This file documents conventions and commands for AI agents working on the Atlas OS codebase. opencode reads it auto-magically as part of every boot so agents know what to do here.

## 1. Project layout

```
/
├── Atlas OS/              # All RFCs (00 - 26). READ THESE BEFORE CHANGES.
├── src-tauri/                # Rust core (Tauri 2 + axum HUD + LSP + CLI)
│   ├── src/
│   │   ├── core/             # AppState, Kernel Bus, IPC
│   │   ├── journal/          # SQLite + schema
│   │   ├── hud/              # axum WebSocket server
│   │   ├── lsp/              # Tower-LSP host (Phase 2)
│   │   ├── skills/           # Skill loader
│   │   ├── profiles/         # Hermes-style multi-profile
│   │   ├── cli/bin/opencode.rs # Headless CLI entrypoint
│   │   ├── main.rs           # Tauri desktop entrypoint
│   │   └── lib.rs            # Re-exported public API
│   ├── capabilities/         # Tauri 2 permission files
│   ├── Cargo.toml            # Rust manifest (workspace root)
│   ├── tauri.conf.json       # Tauri 2 config (pkg name, CSP, icons, bundle)
│   └── build.rs
├── src/                      # SvelteKit frontend (CSR-only, Svelte 5 runes)
│   ├── routes/+page.svelte   # HUD Mission Control landing
│   ├── routes/+layout.svelte # Root layout (SSR disabled)
│   ├── lib/stores/hud.ts     # Kernel Bus WS store (Svelte writable)
│   └── app.d.ts              # Ambient types
├── skills/prompt-clarify/    # Sample skill (RFC 23 §7.2)
├── package.json              # pnpm@9 manifest
├── pnpm-lock.yaml            # generated — never commit lockfiles manually
├── svelte.config.js          # adapter-static, Svelte 5 runes
├── vite.config.ts            # dev host/port baked in for Tauri
├── tsconfig.json             # strict
├── eslint.config.js          # flat config (Svelte + TS strict)
└── .prettierrc
```

## 2. Stack (RFC 25)

- **Rust 1.84+**, edition 2021.
- **Tauri 2** desktop shell; webview = WebView2 / WebKitGTK / WKWebView by OS.
- **SQLite** + `sqlite-vec` for embeddings (bundled-in via `rusqlite`).
- **`fastembed-rs`** (ONNX, no Python) for embeddings.
- **axum 0.7** + WebSocket listener on `127.0.0.1:0` (ephemeral) — survives webview crashes.
- **tower-lsp 0.20** for the LSP host (proxy multiplexer).
- **SvelteKit 2 / Svelte 5 runes**, `adapter-static` — CSR only.
- **pnpm 9+** (NEVER npm). See `package.json` `packageManager`.

## 3. Commands

Always run these from the project root (`C:\Users\Max\Desktop\Harness Ing del Editor de Código`).

### Install dependencies (first time after clone)

```bash
pnpm install
```

For the Rust side, dependencies are fetched automatically on first `cargo build`/`pnpm tauri dev` build. They live in `~/.cargo/registry`.

### Lint

Rust:

```bash
pnpm tauri build --no-bundle        # builds Rust + frontend, also type-checks Rust
# Or:
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --check
```

Frontend:

```bash
pnpm lint
pnpm format --check
```

### Typecheck

Frontend TypeScript:

```bash
pnpm check        # svelte-check + svelte-kit sync
```

Rust typecheck / strict build:

```bash
cargo check --manifest-path src-tauri/Cargo.toml --all-features
```

### Test

Frontend (vitest):

```bash
pnpm test
pnpm test:watch
```

Rust (when sources exist):

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-features
```

### Run / develop

```bash
pnpm tauri:dev    # desktop app with hot reload (Tauri + Vite)
pnpm dev          # SvelteKit frontend only (no Tauri shell)
pnpm tauri:build  # production build for the current OS
```

Headless CLI quick sanity checks:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin opencode -- mission new "test prompt"
cargo run --manifest-path src-tauri/Cargo.toml --bin opencode -- hud
cargo run --manifest-path src-tauri/Cargo.toml --bin opencode -- profile list
cargo run --manifest-path src-tauri/Cargo.toml --bin opencode -- audit -n 10
cargo run --manifest-path src-tauri/Cargo.toml --bin opencode -- journal -n 10
```

## 4. Critical conventions

- **No comments in code unless explicitly requested by the user.** Module-level prose explaining intent is OK; no `// TODO` noise.
- **Use pnpm, never npm.** Never invoke `npm install`, `npm ci`, `npm run`. Pre-commit hook rejects package-lock.json. Use `pnpm` for script invocation too.
- **Never commit lockfiles manually.** Run `pnpm install` and let `pnpm-lock.yaml` regenerate.
- **Never introduce a new dependency without checking the RFCs first.** Particularly: avoid Electron, Node-only APIs, Python, Redis, Postgres. State justifications in `22 - Research Findings.md` when you add something.
- **Read RFCs before editing an engine.** Each engine section (`12` Planning, `13` Coding, `14` Validation, `15` Repair, `16` Learning, `19` Execution Supervisor, `23` Prompt Understanding, `24` HUD) is the spec. Code that contradicts the RFC will be rejected in review.
- **Update `26 - Index & Cross-References.md` when adding/modifying concepts.** See `26 §6` editing rules.
- **Keep git history clean.** We follow Conventional Commits. Examples:
  - `feat(hud): add card action buttons for fork/steer`
  - `fix(journal): correct idempotency_key collision on replays`
  - `docs(rfc-23): clarify gap_type taxonomy in §1`
- **Tests**: write `*.test.ts` for the frontend (vitest) and `#[cfg(test)]` for Rust. New engine code requires at least one happy-path and one failure-path test.

## 5. Context7 MCP — must-use for any library question

We have registered Context7 MCP (`.opencode/mcp.json`). Whenever an agent needs to:
- Add a new crate version to `Cargo.toml`,
- Use a new Tauri plugin,
- Add an axum extractor,
- Use a new tower-lsp method,
- Verify a `fastembed-rs` API,
…etc., **it MUST query Context7 first**:

```
ctx7 library <library name> "<your intent>"
ctx7 docs /org/library "<your specific question>"
```

If Context7 is unreacheable, fall back to `find-docs` skill + `webfetch` over official docs.

## 6. Boundary rules

- **No adding módulos nuevos de monolithic features.** New engines go in their own RFC first (`Atlas OS/NN - …md`), then code.
- **No cargo features beyond what's already in `Cargo.toml`.** If you must add one, update `26 §5` and `Cargo.toml` together.
- **No new external tools bundled** (no Conda, noasdf, no pyinstaller). We are a single-binary distribution (`25 §11`).

## 7. Where RFCs live when something is unclear

See `Atlas OS/26 - Index & Cross-References.md` for a complete map. Quick pointers:
- Adding a new subagent role? `Atlas OS/05 - Swarm.md`.
- Editing IDs (state machine) for doom loop? `Atlas OS/19 - Execution Supervisor.md`.
- Editing HUD card anatomy? `Atlas OS/24 - HUD Mission Control.md` §3.
- New prompt understanding step? `Atlas OS/23 - Prompt Understanding & Refinement.md`.
- Adding a new model provider? `Atlas OS/04 - Model Orchestrator.md` (and `25 §3.8`).
- Changing CLI commands? `Atlas OS/08 - CLI.md` and `25 §3.9`.

## 8. Agent bootstrap reminder

After cloning:
1. `pnpm install` (will populate `node_modules` and pull Rust deps on next build).
2. Install Rust toolchain if missing: `rustup show` triggers the prompt.
3. Run `pnpm tauri:dev` for the first time — this will download Rust crates (~slow first time, ~3 minutes).
4. Open the HUD at the URL printed by `atlas hud` once the desktop app boots.

## 9. We do not commit unless asked

Per the global opencode rule: never commit changes without explicit user request.
