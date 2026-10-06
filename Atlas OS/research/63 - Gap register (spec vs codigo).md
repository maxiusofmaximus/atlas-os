# 63 — Gap register (estado real spec vs código)

> **Tipo:** auditoría de brechas (research), no plan aspiracional. **Fecha:** 2026-10-05.
> **Método:** medición directa del repo (módulos, subcomandos CLI, features, checklists de RFC, tablas del journal) + síntesis de `research/61` (auditoría de nivel de producto) y `20 - Roadmap`.
> **Pregunta que responde:** *¿cuánto falta de verdad para que Atlas OS sea el entorno de desarrollo que describe la documentación?*
>
> **Conclusión de una línea:** los **mecanismos** existen casi todos (los checklists de RFC están verdes), pero faltan **capacidad end-to-end, configuración de modelo, y varias features apagadas por defecto**. El hueco #1 es que **el agente aún no resuelve tareas reales** — y eso está atado al **modelo**, no al andamiaje.
>
> **Reconciliado contra código (2026-10-05):** A2 (claves/llavero), A4 (user modeling) y A5 (skills empaquetadas) están **implementados**; A3 (multicanal) **sigue ausente**. Verificado por búsqueda directa en `src-tauri/src`.
>
> **Reconciliado (2026-10-06):** RFC 07 §10 cerrado salvo infra externa — puente `ToolRegistry` (`mcp/bridge.rs`, **sin cambiar el trait `Tool`**), latencia §8 (`ToolResult.duration_ms`), rotación §9, vista HUD MCP + editor de allowlist (RFC 65 §10). M23 `eval_runs` ya **completada** (Phase 22). Vistas HUD 11→13, tests 1359→1383.

---

## 0. Inventario medido (ground truth)

| Métrica | Valor |
|---|---|
| Módulos Rust (`src-tauri/src/*`) | **32** |
| Subcomandos CLI (`atlas <cmd>`) | **32** |
| Features Cargo | **22** (5 en el build default; 17 opt-in) |
| Vistas HUD | **13** (`overview, agent, kanban, approvals, cost, health, audit, canvas, outline, timeline, worktrees, settings, mcp`) |
| Tests Rust (`cargo test --lib`) | **1383 ok** |
| Checklists de RFC | **casi todos ✅**; solo RFC 63 tiene 2 parciales (por modelo) |

Esto confirma la tesis del registro: **los checklists verdes ≠ entorno usable.** Abajo están los huecos que los checklists NO capturan.

---

## A. Bloqueadores de producto (los que impiden "usarlo de verdad")

| # | Hueco | Evidencia | Bloqueo | Acción |
|---|---|---|---|---|
| **A1** | **Capacidad agéntica ≈ 0**: el agente no resuelve tareas reales | `research/61 §3/§8bis` (Terminal-Bench 2 `pass_rate` 0.000 con modelos de nube; **>0** con Qwen 125B local) | **Modelo** (el harness mide bien: oracle 0.88) | Correr el harness con un modelo de frontera y subir el baseline (RFC 63 §9/§13) |
| **A2** | **Configuración de una clave/modelo** (antes: solo env vars) | `secrets/` + `atlas secrets set\|get\|list\|delete` + `orchestrator::client::resolve_api_key` (env → keychain); backend nativo por SO en `Cargo.toml` | ✅ **Implementado (2026-10)** | Hecho (roundtrip verificado con `tools/secrets-smoke.ps1`). Opcional a futuro: input en el HUD |
| **A3** | **Gateway multicanal** (steer desde Telegram/Slack/…); antes solo HUD web | RFC 29 §3.B | **A3.0 ✅ (2026-10-06)** núcleo channel-agnóstico `channels/` (`parse_command` + `format_event` + CLI `atlas channels`); adaptador `teloxide` (feature `multi-channel`, default off) + token de bot | Núcleo hecho; adaptador pendiente |
| **A4** | ~~Sin user modeling~~ ✅ **Implementado** | `journal/user_profile.rs` (`user_profile` table + `knowledge_state.gaps_identified`) consumido por `prompt/steps/detect.rs::apply_user_profile` (RFC 23) | ✅ Hecho | — |
| **A5** | ~~Bundled skills catalog ausente~~ ✅ **Implementado** | `skills/bundled.rs` compila **16** skills vía `include_str!` (`bundled-skills`, default on) | ✅ Hecho | — |

**Nota:** la brecha "HUD = panel de debug" de `research/61 §5.B` **ya está cerrada** por RFC 65 (11 vistas + approvals + agent cards).
La brecha "runtime muere con el desktop" (`§5.C`) **ya está cerrada** por `atlas serve` (RFC 29 §3.A).

---

## B. Capacidades que NO entran en el build por defecto

Build default = `tauri, hud, lsp, cli, bundled-skills`. Estas **no están** salvo que las actives:

| Feature | Qué activa | Por qué importa |
|---|---|---|
| `fastembed` | **Vector Knowledge (RFC 09)** — memoria semántica | Sin esto, **el build default no tiene memoria vectorial** |
| `dag_mode` / `codebase-graph` / `ast` | Grafo de misión + grafo de código (tree-sitter) | Sin esto, no hay navegación de código por grafo |
| `acp-server` | Host ACP de Microsoft IT (RFC 28 §B) | Integración con el terminal |
| `firecrawl` | Ingestión web (RFC 28 §E) | Research con web |
| `toast` / `calendar-ics` / `calendar-graph` | Notificaciones / Calendar (RFC 28 §F/G) | Superficies de producto |
| `laya` | Clasificador "System One" (RFC 35) | Routing barato |
| `remote-ui` / `tui` / `doc-ingest` | UI remota / TUI / ingesta de docs | Superficies |

**Decisión pendiente:** activarlas por defecto (peso/ICE en Windows, ADR 0002) **o** mantenerlas opt-in (hoy: documentado en README §"Default build").

**Evidencia medida (2026-10-06, ADR 0002 §"Reproducción del ICE"):** con `rustc 1.96.0` + `ort-sys 2.0.0-rc.9`, `fastembed` **sigue ICEando en `cargo build`** (no en `cargo check`) → **no puede ser default**. `dag_mode` + `codebase-graph` (Rust puro) **sí** compilan, con **+1.1 MB** (dev). **Recomendación:** mantener `fastembed` opt-in; `dag_mode`/`codebase-graph` son activables por defecto (coste bajo) **cuando el operador lo decida** (el comentario de `Cargo.toml` los gatea a "GraphView toggle" y a la preferencia de tamaño).

---

## C. Sub-fases diferidas (enumerables, ya especificadas)

| RFC | Diferido | Estado |
|---|---|---|
| 63 | items 9/13 — gates de capacidad `≥0.10` / `≥0.50` | **Bloqueado por modelo** (A1) |
| 07 | Fase 29.1+ — isolation real de sandbox, supply-chain MCP, puente `ToolRegistry`, telemetría, rotación | ✅ **puente `ToolRegistry`** (`mcp/bridge.rs`: hilo worker + runtime propio + nombres leaked; **sin cambiar el trait `Tool`**, contra lo que asumía este registro), ✅ latencia §8 (`ToolResult.duration_ms` rellenada en `ToolRegistry::call`), ✅ rotación §9 (3 fallos consecutivos → tool desactivada), ✅ vista HUD MCP + editor de allowlist (RFC 65 §10). Pendiente: isolation real §2, firma/checksum + Socket §3, `confidence_delta` §8, Learning Engine §9 |
| 28 | §E items 9-10 (MCP server nativo; graphify closure); §I checklist 2-7 (terminal-browser) | post-MVP |
| 65 | video TTS/Loom, MCP hot-swap, E2E Playwright, auditoría móvil | diferido explícito |
| 20 | M23 `eval_runs` (gap G16) — **RESUCITADA y COMPLETA** (Phase 22, M36/v35: `journal/eval_runs.rs` + CLI `atlas eval run/list/report/metrics/import` + gate CI); routing diferido (Phase 2.5); auditorías de deps | eval_runs ✅, resto diferido |
| 13 | Laya real | **bloqueado por upstream** (`research/42`: criterio no cumplido) |
| 25 §6 | **OS keychain para secretos** | = A2 |

---

## D. Lo que SÍ existe (para no ser alarmista)

Motores (32 módulos): orchestrator completo (routing/cascade/MoA/cost/backpressure/affinity), supervisor (doom-loop/budget/checkpoints), coding loop (LLM→Diff→apply→verify→repair), validation (12 stages), swarm (worktrees/pool/mailbox/merge), MCP runtime, calendar, toast, domain packs, RFC 65 HUD (11 vistas), journal SQLite (58 tablas), CLI (32 subcomandos), `atlas serve` headless, seguridad (firmas + supply-chain + sandbox types).

---

## E. Plan para terminar (ordenado; sin re-loop)

**Camino crítico (desbloquea "entorno de desarrollo usable"):**
1. ✅ **A2 — Claves/secretos seguros** (keychain) + resolución de key con fallback env. **Implementado (2026-10).**
2. **A1 — Correr el harness con un modelo real** y subir el baseline (RFC 63 §9 → §13). Requiere un endpoint OpenAI-compatible; la key ya se puede guardar con `atlas secrets set`, así que el bloqueo ahora es **elegir/levantar el modelo**, no Atlas.
3. **B — Decidir el default build** (activar `fastembed` + `dag_mode`/`codebase-graph`, o dejarlo documentado).

**Ampliación de producto (resto):**
4. **C — sub-fases diferidas**: RFC 07 (supply-chain MCP §3 + `ToolRegistry` bridge), RFC 28 §E items 9-10, RFC 65 diferidos (video/Playwright).
5. **A3 — gateway multicanal** (requiere decisión de crates/tokens).
6. **B — decidir el default build** (activar `fastembed`/`dag_mode`, o dejarlo documentado).

> Regla anti-loop: no se abre un frente nuevo hasta cerrar el anterior. El frente #1 es **A2**.
