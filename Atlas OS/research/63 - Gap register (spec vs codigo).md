# 63 — Gap register (estado real spec vs código)

> **Tipo:** auditoría de brechas (research), no plan aspiracional. **Fecha:** 2026-10-05.
> **Método:** medición directa del repo (módulos, subcomandos CLI, features, checklists de RFC, tablas del journal) + síntesis de `research/61` (auditoría de nivel de producto) y `20 - Roadmap`.
> **Pregunta que responde:** *¿cuánto falta de verdad para que Atlas OS sea el entorno de desarrollo que describe la documentación?*
>
> **Conclusión de una línea:** los **mecanismos** existen casi todos (los checklists de RFC están verdes), pero faltan **capacidad end-to-end, configuración de modelo, y varias features apagadas por defecto**. El hueco #1 es que **el agente aún no resuelve tareas reales** — y eso está atado al **modelo**, no al andamiaje.

---

## 0. Inventario medido (ground truth)

| Métrica | Valor |
|---|---|
| Módulos Rust (`src-tauri/src/*`) | **32** |
| Subcomandos CLI (`atlas <cmd>`) | **32** |
| Features Cargo | **22** (5 en el build default; 17 opt-in) |
| Vistas HUD | **11** (`overview, agent, kanban, approvals, cost, health, audit, canvas, outline, timeline, worktrees`) |
| Tests Rust (`cargo test --lib`) | **1359 ok** |
| Checklists de RFC | **casi todos ✅**; solo RFC 63 tiene 2 parciales (por modelo) |

Esto confirma la tesis del registro: **los checklists verdes ≠ entorno usable.** Abajo están los huecos que los checklists NO capturan.

---

## A. Bloqueadores de producto (los que impiden "usarlo de verdad")

| # | Hueco | Evidencia | Bloqueo | Acción |
|---|---|---|---|---|
| **A1** | **Capacidad agéntica ≈ 0**: el agente no resuelve tareas reales | `research/61 §3/§8bis` (Terminal-Bench 2 `pass_rate` 0.000 con modelos de nube; **>0** con Qwen 125B local) | **Modelo** (el harness mide bien: oracle 0.88) | Correr el harness con un modelo de frontera y subir el baseline (RFC 63 §9/§13) |
| **A2** | **No hay forma segura de configurar un modelo/clave** en el producto: las claves se leen **solo de env vars** | `orchestrator/provider.rs:435-438` (*"encrypting at-rest is a Phase 3 concern (RFC 25 §6 OS keychain)"*); `keyring` declarado en `Cargo.toml` con **0 usos**; no existe `atlas secrets` (RFC 25 §3.9) | Especificado, **no implementado** | `atlas secrets set\|get\|list\|delete` sobre `keyring-rs` (servicio `OpenCodeOS`) + resolución de key con fallback a env. **Camino crítico** |
| **A3** | **Sin gateway multicanal** (steer desde Telegram/Slack/…); el operador solo abre el HUD web | RFC 29 §3.B | Faltan crates (`teloxide`/`serenity`/`slack-morphism`) + tokens de bot → **decisión de operador** | Diferido hasta decisión |
| **A4** | **Sin user modeling** (el sistema no distingue "qué sabe el usuario") | RFC 29 §3.C | Tabla `user_profile` (M20+) no existe | Implementable (SQLite + consulta en RFC 23) |
| **A5** | **Bundled skills catalog ausente** (Genspark trae 80+; Atlas obliga a instalar) | RFC 29 §3.D | `bundled-skills` existe como feature pero sin catálogo curado de ~30 skills | Implementable |

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

---

## C. Sub-fases diferidas (enumerables, ya especificadas)

| RFC | Diferido | Estado |
|---|---|---|
| 63 | items 9/13 — gates de capacidad `≥0.10` / `≥0.50` | **Bloqueado por modelo** (A1) |
| 07 | Fase 29.1+ — isolation real de sandbox, supply-chain MCP, puente `ToolRegistry`, telemetría, rotación | §2 (política) ✅; el resto pendiente. El puente `ToolRegistry` requiere cambiar la firma del trait `Tool` (async + nombres dinámicos) |
| 28 | §E items 9-10 (MCP server nativo; graphify closure); §I checklist 2-7 (terminal-browser) | post-MVP |
| 65 | video TTS/Loom, MCP hot-swap, E2E Playwright, auditoría móvil | diferido explícito |
| 20 | M23 `eval_runs` (gap G16); routing diferido (Phase 2.5); auditorías de deps (sysinfo/nvml, oidc, ratatui, candle) | diferido |
| 13 | Laya real | **bloqueado por upstream** (`research/42`: criterio no cumplido) |
| 25 §6 | **OS keychain para secretos** | = A2 |

---

## D. Lo que SÍ existe (para no ser alarmista)

Motores (32 módulos): orchestrator completo (routing/cascade/MoA/cost/backpressure/affinity), supervisor (doom-loop/budget/checkpoints), coding loop (LLM→Diff→apply→verify→repair), validation (12 stages), swarm (worktrees/pool/mailbox/merge), MCP runtime, calendar, toast, domain packs, RFC 65 HUD (11 vistas), journal SQLite (58 tablas), CLI (32 subcomandos), `atlas serve` headless, seguridad (firmas + supply-chain + sandbox types).

---

## E. Plan para terminar (ordenado; sin re-loop)

**Camino crítico (desbloquea "entorno de desarrollo usable"):**
1. **A2 — Claves/secretos seguros** (keychain) + resolución de key con fallback env. → permite enchufar **tu modelo**.
2. **A1 — Correr el harness con un modelo real** y subir el baseline (RFC 63 §9 → §13). Requiere un endpoint OpenAI-compatible.
3. **B — Decidir el default build** (activar `fastembed` + `dag_mode`/`codebase-graph`, o dejarlo documentado).

**Ampliación de producto (tras el camino crítico):**
4. **A4 — user modeling** (tabla `user_profile` + integración con RFC 23).
5. **A5 — bundled skills catalog** (~30 skills).
6. **C — sub-fases diferidas de RFC 07** (supply-chain MCP, `ToolRegistry` bridge).
7. **A3 — gateway multicanal** (requiere decisión de crates/tokens).

> Regla anti-loop: no se abre un frente nuevo hasta cerrar el anterior. El frente #1 es **A2**.
