# Registro de correcciones RFC/código (Builder-3)

- **Autor:** Builder-3 (rol de coordinación).
- **Fecha:** 2026-10-06.
- **Base:** auditoría `docs/audit/rfc-vs-code.md` (ACEPTADA) + correcciones del Todo Updater (`docs/TODO.md` §2–3).
- **Revisión:** HEAD `1292fe8` (no se commitea; integra el Project Lead).
- **Re-medida vigente [O]:** `cargo test --manifest-path src-tauri/Cargo.toml --lib` → **1392 passed / 1 ignored**; `pnpm test` → **116 passed**; `git log -1` de `0bf6b0e`, `0735daf`, `98ea6bc`, `e39edc6`, `db75839`, `4248293` (existen).
- **Alcance:** solo documentación. No tocados: `src/`, `src-tauri/`, RFC 66/67/62, `research/64`, `docs/design`, `docs/TODO.md`.

## Correcciones aplicadas

| # | Archivo | Línea | Antes → Después | Evidencia |
|---|---|---|---|---|
| 1 | `Atlas OS/07 - MCP.md` | 126 | "35 tests unitarios (`mcp::`)" → "50 tests unitarios (`mcp::`)" | `cargo test --lib mcp::` → `50 passed` [O] |
| 2 | `Atlas OS/23 - Prompt Understanding & Refinement.md` | 362 | "Status: Draft v1" → "Status: implementado (pipeline en `prompt/steps/` + `runner`; §2 9 pasos)" | `prompt/steps/{capture,parse,detect,similar,clarify,consolidate}.rs` + `prompt/runner.rs:21,64,118` [O] |
| 3 | `Atlas OS/24 - HUD Mission Control.md` | 282 | "Validation Engine (`13`)" → "Validation Engine (`14`)" | RFC 13 = Coding, RFC 14 = Validation (`14 - Validation Engine.md`) [O] |
| 4 | `Atlas OS/24 - HUD Mission Control.md` | 290 | "Drag-and-drop de skills/MCPs … sin restart." → "… de skills … (implementado, RFC 65 §10). El hot-swap de **MCP** NO está implementado (catálogo read + editor de allowlist + probe; `hud/mcp.rs`)." | `hud/mcp.rs:9` "hot-swap MCP runtime … is not implemented" [O] |
| 5 | `Atlas OS/24 - HUD Mission Control.md` | 23 | "Worktrees visuales con mini-git-graph por Mission." → "Worktrees visuales (tabla path/branch/state); el mini-git-graph queda **pendiente**." | `WorktreesView.svelte` (tabla, sin `graph/svg/node/edge`) [O] |
| 6 | `Atlas OS/24 - HUD Mission Control.md` | 418 | "… mini-git-graph local con subagentes como nodos." → "… (**pendiente**)." | ídem #5 [O] |
| 7 | `Atlas OS/63 - Agentic Capability Engine.md` | 4 | "12/13 completados; 1 ítem … bloqueado" → "11/13 completados; 2 ítems `[~]` (item 9 ≥0.10 e item 13 ≥0.50) bloqueados" | checklist líneas 160–172: 11 `[x]`, 2 `[~]` (9, 13) [O] |
| 8 | `Atlas OS/63 - Agentic Capability Engine.md` | 55, 161 | "read/write/edit/list/glob/grep" → "read/write/edit/list" | `tools/fs.rs` 4 `Tool` (líneas 60/99/143/196); `atlas agent --list-tools` = 6 tools sin glob/grep [O] |
| 9 | `Atlas OS/65 - HUD Mission Control v2.md` | 15 | "Las **8 views**: … Worktrees." → "Las **13 views**: … Worktrees, Settings, MCP (+ Overview, Agent)." | `src/lib/stores/views.ts:35-48` (13 entradas) [O] |
| 10 | `Atlas OS/65 - HUD Mission Control v2.md` | 22 | "Entregar las **8 views** …" → "Entregar las **13 views** …" | ídem #9 [O] |
| 11 | `Atlas OS/65 - HUD Mission Control v2.md` | 61 | "dark/light (selector ya en `+page.svelte`)" → "dark/light **pendiente** (sin `src/app.css` ni selector de tema en `+page.svelte`; Builder-2 `docs/audit/frontend-current-state.md` §(e))" | `+page.svelte` 0 menciones dark/light [O]; Builder-2 §(e) |
| 12 | `Atlas OS/65 - HUD Mission Control v2.md` | 96 | "(… 3 tests …). MCP sigue read-only (stage)." → "(… 2 tests …). MCP: catálogo read + editor de allowlist (`POST /hud/mcp/allowlist`) + probe (`POST /hud/mcp/probe`); el hot-swap sigue sin implementar (item 13)." | `hud/skills.rs` 2 tests (75, 81) [O]; `hud/server.rs:131-132` rutas write/probe [O] |
| 13 | `Atlas OS/research/63 …` | 19 | "Módulos Rust **32**" → "**36**" | `lib.rs` 36 `pub mod` [O] |
| 14 | `Atlas OS/research/63 …` | 20 | "Subcomandos CLI **32**" → "**34**" | `proto.rs` 34 variantes `Commands` [O] |
| 15 | `Atlas OS/research/63 …` | 23 | "Tests Rust **1383 ok**" → "**1392 ok**" (+ fila nueva "Tests frontend (vitest) **116 ok**") | `cargo test --lib` → 1392; `pnpm test` → 116 [O] |
| 16 | `Atlas OS/research/63 …` | 11 | "tests 1359→1383" → "tests 1359→1392" | ídem #15 [O] |
| 17 | `Atlas OS/research/63 …` | 40 | "RFC 65 (11 vistas …)" → "(13 vistas …)" | `views.ts` [O] |
| 18 | `Atlas OS/research/63 …` | 81 | "Motores (32 módulos) … validation (12 stages) … HUD (11 vistas) … CLI (32 subcomandos)" → "36 … 11 stages … 13 vistas … 34 subcomandos" | módulos 36, `StageKind` 11 variantes (`validation/types.rs:29-40`), vistas 13, CLI 34 [O] |
| 19 | `Atlas OS/research/63 …` | 85–97 (§E) | Quita "frente #1 = A2" (hecho), quita B duplicado (#6), mueve puente `ToolRegistry` a hecho, frente #1 → A1 | A2 ✅; §C fila 07 "puente ✅" [O] |
| 20 | `PLAYBOOK.md` | tras intro | (nueva) "## Estado (reconciliado con `20 - Roadmap.md` y git, 2026-10-06)" | commits `86b844e`,`0735daf`,`98ea6bc`,`e39edc6`,`db75839`,`4248293`,`0bf6b0e` [O] |
| 21 | `Atlas OS/20 - Roadmap.md` | 579 | "**Siguiente:** re-correr Terminal-Bench con `--agent` (F38) …" → "**F38 hecho:** corrida agent-mode registrada **0/11** en `0bf6b0e` …" | `git log -1 0bf6b0e` [O] |

## PENDIENTE / fuera de alcance

- **`Atlas OS/67 - UI Specification.md`** (Todo Updater §3 item 5): su encabezado afirma que `research/64` "AÚN NO EXISTE", pero existe (~43 KB). **NO aplicado**: RFC 67 es de otro rol (prohibido en mi alcance). → **PENDIENTE (rol UI/UX Designer)**.
- **`research/63 §E #2` "B — decidir default build"**: se mantiene como **decisión de operador** (no es una corrección factual). Evidencia ya aportada: ADR 0002 (commit `3475d25`) — `fastembed` ICEa → no puede ser default; `dag_mode`/`codebase-graph` activables. → **PENDIENTE-DECISIÓN (operador)**.
- **`research/63 §E` #4 "A3"**: núcleo + seam hechos (commits `90f18b5`, `eaaad4b`); adaptador `teloxide` → **PENDIENTE-DECISIÓN (operador)**.

## Verificación posterior

- `git diff --stat` de los `.md` modificados (abajo).
- Re-medida tras editar: no aplica (solo documentación; sin cambios en `src`/`src-tauri`).
