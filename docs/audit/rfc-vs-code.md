# RFC vs código — auditoría de discrepancias documentales

- **Autor:** Builder-3 (rol de coordinación).
- **Fecha:** 2026-10-06.
- **Revisión auditada:** git `1292fe8` (HEAD al iniciar la ronda).
- **Alcance:** RFC 07, 12, 13, 14, 15, 19, 23, 24, 63, 64, 65.
- **Método:** para cada RFC se extraen las afirmaciones marcadas como implementadas / `Status` / `✅` / `[x]` y se contrastan contra el código (`src-tauri/src`, `src`) y los tests (`cargo test --manifest-path src-tauri/Cargo.toml --lib <filtro>`, `pnpm test`). Cada hallazgo cita `fichero:línea` o el nombre del test.
- **Clasificación:** `DOC-ERRONEO` (afirma algo falso), `DOC-OBSOLETO` (fue cierto, ya no), `CODIGO-SIN-DOC` (existe en código y el RFC no lo recoge), `GAP-REAL` (el RFC promete algo que no existe).
- **Severidad:** `P0` (rompe la construcción), `P1` (induce a error grave), `P2` (imprecisión que desvía), `P3` (cosmético / desactualizado).
- **Procedencia:** `[O]` observado en este repo esta sesión (comando/fichero:línea), `[Os]` observado en vivo en fuente citada, `[I]` inferido/memoria, `[P]` pendiente.
- **Restricciones cumplidas:** solo lectura de RFC y código; salida única en este fichero; sin dependencias nuevas; sin commit. Ninguna afirmación sin evidencia ejecutada o `fichero:línea`.
- **Nota:** `Atlas OS/research/63 - Gap register` se **verifica**, no se repite (ver §Verificación de research/63).

---

## Resumen de discrepancias

| # | RFC | Clase | Sev | Afirmación (doc) | Realidad (código) |
|---|---|---|---|---|---|
| 1 | 63 | DOC-ERRONEO | **P1** | `fs.rs`: read/write/edit/list/**glob/grep** | solo `fs.read/write/edit/list` (4 tools) |
| 2 | 24 | DOC-ERRONEO | P2 | "Drag-and-drop de skills/**MCPs** a agentes activos, sin restart" | MCP hot-swap **no implementado** |
| 3 | 24 | GAP-REAL | P2 | "Worktrees visuales con **mini-git-graph** por Mission" | `WorktreesView` = tabla, sin grafo |
| 4 | 63 | DOC-ERRONEO | P2 | Status "**12/13** completados; **1** ítem bloqueado" | 11 `[x]`; **2** `[~]` (9 y 13) |
| 5 | 23 | DOC-OBSOLETO | P2 | "Status: **Draft v1**" | pipeline implementado en `prompt/steps/` |
| 6 | 24 | DOC-ERRONEO | P3 | "Validation Engine (`13`)" | Validation Engine = RFC **14** |
| 7 | 65 | DOC-OBSOLETO | P3 | "MCP sigue **read-only** (stage)" | write (`/hud/mcp/allowlist`) + probe |
| 8 | 65 | DOC-ERRONEO | P3 | skills activate "**3 tests**" | `hud/skills.rs` = **2** tests |
| 9 | 65 | DOC-OBSOLETO | P3 | "Las **8 views**: …" | **13** views hoy |
| 10 | 07 | DOC-OBSOLETO | P3 | "**35** tests unitarios (`mcp::`)" | **50** tests (`mcp::`) |
| R1 | reg-63 | DOC-OBSOLETO | P3 | "Módulos Rust **32**" | 36 (`lib.rs` `pub mod`) |
| R2 | reg-63 | DOC-OBSOLETO | P3 | "Subcomandos CLI **32**" | 34 variantes `Commands` |
| R3 | reg-63 | DOC-OBSOLETO | P3 | "Tests Rust **1383 ok**" | **1392 ok** |

**Total: 10 discrepancias RFC↔código + 3 de drift del gap register.**

---

## Detalle por hallazgo

### 1 — RFC 63 item 2: `fs.*` promete `glob`/`grep` que no existen · DOC-ERRONEO · P1
- **Doc** (`63 - Agentic Capability Engine.md:161`, y el diagrama de la línea 55): `Tools fs.* (orchestrator/tools/fs.rs: read/write/edit/list/glob/grep)`.
- **Código** [O]: `src-tauri/src/orchestrator/tools/fs.rs` implementa **4** `Tool` (`fn name()` en líneas 60, 99, 143, 196 → `fs.read`, `fs.write`, `fs.edit`, `fs.list`). No hay `fs.glob` ni `fs.grep`.
- **Test/ejecución** [O]: `atlas agent --list-tools` lista 6 tools (`code.apply_diff`, `exec.run`, `fs.edit`, `fs.list`, `fs.read`, `fs.write`) — sin glob/grep.
- **Corrección:** `… fs.rs: read/write/edit/list` (quitar `glob/grep`).

### 2 — RFC 24 §8: MCP drag-and-drop "sin restart" · DOC-ERRONEO · P2
- **Doc** (`24 - HUD Mission Control.md:290`): `Drag-and-drop de skills/MCPs a agentes activos, sin restart.`
- **Código** [O]: `src-tauri/src/hud/mcp.rs:9` — *"Atlas's own hot-swap MCP runtime (RFC 07 / RFC 24 §8) is not implemented; this endpoint is the honest read side of the catalog."* Skills sí tienen activación real (`hud/skills.rs:44 pub async fn activate`, publica `SkillActivated`); **MCP no**.
- **Corrección:** `Drag-and-drop de skills a agentes activos, sin restart (implementado, RFC 65 §10). El hot-swap de MCP NO está implementado: hoy es catálogo read + editor de allowlist + probe.`

### 3 — RFC 24 §12: "mini-git-graph" no implementado · GAP-REAL · P2
- **Doc** (`24 - HUD Mission Control.md:23`, y §12 línea 418): `8. Worktrees visuales con mini-git-graph por Mission.`
- **Código** [O]: `src/lib/components/WorktreesView.svelte` renderiza una **tabla** `Path/Branch/State` (líneas 64-81); no hay `svg`/`node`/`edge`/grafo (búsqueda `graph|svg|node|edge` → 0 coincidencias).
- **Corrección:** `Worktrees visuales (tabla path/branch/state); el mini-git-graph queda pendiente.`

### 4 — RFC 63 Status: recuento y nº de bloqueados incorrectos · DOC-ERRONEO · P2
- **Doc** (`63 - Agentic Capability Engine.md:4`): `Status: In progress — 12/13 completados; 1 ítem (el gate de capacidad 9/13, model-bound) bloqueado…`.
- **Checklist** [O] (líneas 160-172): `[x]` en 1,2,3,4,5,6,7,8,10,11,12 → **11**; `[~]` en **9** y **13** → **2**. No 12/13 ni 1 bloqueado.
- **Corrección:** `Status: In progress — 11/13 completados; 2 ítems [~] (item 9 gate ≥0.10 e item 13 gate ≥0.50) bloqueados por capacidad del modelo.`

### 5 — RFC 23 §12: "Draft v1" sobre un pipeline implementado · DOC-OBSOLETO · P2
- **Doc** (`23 - Prompt Understanding & Refinement.md:362`): `- Status: Draft v1`.
- **Código** [O]: `src-tauri/src/prompt/steps/{capture,parse,detect,similar,clarify,consolidate}.rs` + `prompt/runner.rs` (import línea 21; `step_7_consolidate` línea 64; `run_with_profile_and_rules` línea 118). El pipeline (9 pasos) está implementado.
- **Corrección:** `- Status: implementado (pipeline en prompt/steps/ + runner; §2 9 pasos)`.

### 6 — RFC 24 §7: cross-ref erróneo "Validation Engine (`13`)" · DOC-ERRONEO · P3
- **Doc** (`24 - HUD Mission Control.md:282`): `Checker status: verde/amarillo/rojo por cada tool del Validation Engine (13).`
- **Realidad** [O]: RFC **13 = Coding Engine**, RFC **14 = Validation Engine** (`14 - Validation Engine.md`). El motor de validación es el 14.
- **Corrección:** `… del Validation Engine (14).`

### 7 — RFC 65 item 10: "MCP sigue read-only" ya no es cierto · DOC-OBSOLETO · P3
- **Doc** (`65 - HUD Mission Control v2.md:96`): `… MCP sigue read-only (stage).`
- **Código** [O]: `src-tauri/src/hud/server.rs:131-132` — `POST /hud/mcp/allowlist` (`hud/mcp.rs::set_allowlist`) y `POST /hud/mcp/probe` (`hud/mcp.rs::probe`); item 13 del propio RFC 65 documenta el editor. Contradice su propio checklist.
- **Corrección:** `MCP: catálogo read + editor de allowlist (POST /hud/mcp/allowlist) + probe (POST /hud/mcp/probe); el hot-swap sigue sin implementar (item 13).`

### 8 — RFC 65 item 10: "3 tests" pero hay 2 · DOC-ERRONEO · P3
- **Doc** (`65 - HUD Mission Control v2.md:96`): `(hud/skills.rs publica SkillActivated en el Kernel Bus; 3 tests + svelte-check verde)`.
- **Código** [O]: `src-tauri/src/hud/skills.rs` tiene **2** tests (`body_defaults_to_no_agent` línea 75, `body_parses_agent_id` línea 81).
- **Corrección:** `… 2 tests (body_defaults_to_no_agent, body_parses_agent_id) + svelte-check verde`.

### 9 — RFC 65 §1: "8 views" desactualizado · DOC-OBSOLETO · P3
- **Doc** (`65 - HUD Mission Control v2.md:15`, y línea 22): `- Las 8 views: Kanban, Canvas, Outline, Timeline, Cost & Res, Health KPIs, Audit, Worktrees.`
- **Código** [O]: `src/lib/stores/views.ts:35-48` (`VIEWS`) tiene **13** entradas: `overview, agent, kanban, approvals, cost, health, audit, canvas, outline, timeline, worktrees, settings, mcp`. El propio §3 del RFC 65 (tabla) ya lista Settings + MCP.
- **Corrección:** `- Las 13 views: … + Settings + MCP (y Overview/Agent).`

### 10 — RFC 07 §10: "35 tests (`mcp::`)" desactualizado · DOC-OBSOLETO · P3
- **Doc** (`07 - MCP.md:126`): `35 tests unitarios (mcp::) con transporte en memoria + timeout + allowlist`.
- **Ejecución** [O]: `cargo test --manifest-path src-tauri/Cargo.toml --lib mcp::` → **`50 passed; 0 failed`** (el puente, la rotación y el editor añadieron tests).
- **Corrección:** `50 tests unitarios (mcp::) …` (o retirar el número para que no derive).

---

## Verificación de `research/63` (no repetida, comprobada)

El gap register es **sustancialmente correcto** en sus tesis (mecanismos presentes, capacidad end-to-end/modelo como brecha #1; A2/A4/A5 hechos). Se detecta **drift numérico** tras los cambios recientes [O]:

| Claim (`research/63`) | Medido 2026-10-06 | Veredicto |
|---|---|---|
| `Módulos Rust 32` (línea 19) | `lib.rs` tiene **36** `pub mod` | DOC-OBSOLETO (R1) |
| `Subcomandos CLI 32` (línea 20) | **34** variantes `Commands` (`proto.rs`) | DOC-OBSOLETO (R2) |
| `Tests Rust 1383 ok` (línea 23) | **1392 ok** (`cargo test --lib`) | DOC-OBSOLETO (R3) |
| `Vistas HUD 13` (línea 22) | 13 (`views.ts`) | ✅ correcto |
| `Features Cargo 22 (5 default)` (línea 21) | 5 en `default` | ✅ correcto |
| `RFC 63 tiene 2 parciales` (línea 22) | items 9 y 13 `[~]` | ✅ correcto |

Observación adicional: el register §D dice `validation (12 stages)`; `StageKind` tiene **11** variantes (`validation/types.rs:29-40`) y `static_analysis` es un **módulo** usado por `atlas validate` (`stages/mod.rs:14`), no una etapa del pipeline. Precisión recomendada: `11 etapas + static_analysis (CLI)`.

---

## RFCs revisados sin discrepancia (claims verificados)

- **RFC 07** (salvo #10): `mcp/bridge.rs` + `McpBridge`/`McpTool`/`register_mcp`/`set_allowed_tools_in_file`/`MAX_CONSECUTIVE_FAILURES` existen [O]; CLI `atlas mcp list|add|remove|probe|call` existe (`commands/mcp.rs:29-63`) [O].
- **RFC 12**: `Strategy` enum = `Tdd, Strangler, BigBang, Incremental, Pair` (`planning/types.rs:27-32`) = las 5 estrategias de §6 [O]; `dag_mode`/`MissionGraph` en `graph/mod.rs` (§3.1) [O].
- **RFC 13**: spec sin checklist; `coding/{apply,llm,runner,types}.rs` existen [O].
- **RFC 14**: `evaluate_done_claim` en `validation/evidence.rs:178` [O]; `StageKind::pipeline_order()` referenciado correctamente [O]; `static_analysis.rs` para Semgrep/CodeQL [O].
- **RFC 15**: `repair/types.rs::ErrorClass` (línea 26) + `repair/runner.rs` existen [O].
- **RFC 19**: `supervisor/resume.rs::resume_state` coincide con lo citado; **7 tests** (`cargo test --lib supervisor::resume` → `7 passed`) [O]; `hud/observer.rs` + `GET /hud/journal?limit&offset&kind` (`hud/server.rs:93`) [O].
- **RFC 63** (salvo #1 y #4): items 3/4 (`sandbox/{mod,local,bridge,daytona,e2b}.rs`), 5 (`tools/{code,web,browse}.rs`), 6 (`artifacts.rs::verify_artifacts` línea 97), 7 (`journal/agent_runs.rs`), 10/11 (`AgentCard.svelte`), 12 (`baseline.json`, `scripts/agent-bench-ratchet.mjs`, `.github/workflows/{agent-bench,eval-gate}.yml`) verificados [O].
- **RFC 64** (11/11): `domain/{manifest,registry,lateral}.rs`, `journal/domain.rs`, `StageKind::Domain`, **7** packs `.toml`, **7** `docs/domain-*.md`, `install --sha256` (`commands/domain.rs:51`) [O].
- **RFC 65** (salvo #7/#8/#9): rutas `GET /hud/{cost,health,audit,demos}` y `/remote/status` (`hud/server.rs:121-129`) [O]; `@media (max-width: 720px)` (`+page.svelte:1215`) [O]; `AgentCard.svelte`/`DemoPane.svelte` existen [O].
- **RFC 24** (salvo #2/#3/#6): `AgentStatus` enum = los 10 estados listados en §(línea 115) [O]; §10.1 posting (`cli/commands/audit.rs` `--export-posting` / `.posting.yaml`) [O]; OIDC backend en `remote_auth/mod.rs` (bearer + discovery; capa real diferida) [O].

---

## Límites y pendientes

- **No verificado a fondo** (fuera del alcance de "claims implementados"): RFC 23 §2 mapea 9 pasos a 6 módulos de `prompt/steps/` + `runner` — los pasos 8 (loop con usuario) y 9 (locked mission) no tienen módulo propio; **no confirmado** si viven en `runner`/CLI (requiere lectura adicional) → **[P]**, no se afirma discrepancia.
- **Specs sin checklist** (12, 13, 15, y §16 de 24): sus requisitos son aspiracionales; se verificó que los **artefactos citados** existen, no que cada requisito esté cubierto (fuera de "implementado/status").
- **Recuentos**: todos medidos con `cargo test --lib` y `Select-String`; HEAD `1292fe8`. Con el árbol cambiando (features opt-in), los recuentos derivan rápido → se recomienda **no fijar números de tests en los RFC** (hallazgos #8/#10/R3).
- **Comandos usados** (evidencia): `cargo test --manifest-path src-tauri/Cargo.toml --lib mcp::` (50), `… --lib supervisor::resume` (7), `… --lib` (1392); `Select-String`/`Get-ChildItem` sobre `src-tauri/src/**`, `src/**`, `Atlas OS/**`; `atlas agent --list-tools`.

## Criterio de terminado

- Cada RFC listado (07, 12, 13, 14, 15, 19, 23, 24, 63, 64, 65) revisado y con veredicto. ✅
- Cuadro de discrepancias con clase y severidad, **10 hallazgos RFC↔código + 3 de drift del register**, cada uno con `fichero:línea`/test. ✅
- Correcciones concretas propuestas (frase actual → frase correcta). ✅
- `research/63` verificado, no repetido. ✅
