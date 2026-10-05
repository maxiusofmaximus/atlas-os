# 61 — Nivel de implementación vs proyectos objetivo (opencode, Claude Code, Codex, Hermes, Cursor…)

> **Tipo:** auditoría de estado (research), no plan. **Fecha:** 2026-10-04.
> **Método:** medición directa del repo (LOC, tests, migraciones, features, stubs) + lectura de la spec (`Atlas OS/00`, `20`, `21`, `24`, `26`, `28`, `30`) + baseline externo publicado en `README.md`.
> **Pregunta:** ¿en qué nivel está Atlas OS frente a los proyectos que se propuso igualar/superar (opencode, Claude Code, Codex CLI, Cursor, Windsurf, Hermes, OpenClaw, Cline/Roo, Aider, OpenRouter Fusion / Sakana Fugu / AionUI)?

---

## 1. Objetivos declarados (dónde están)

| Objetivo | Declarado en |
|---|---|
| Igualar/superar Cursor, Windsurf, Claude Code, Hermes, OpenClaw | `00 - Vision.md` §2, §7 |
| Superar a Hermes **haciéndolo distribuido** (swarm) | `05 - Swarm.md` §1, §5 |
| Paridad de gobernanza vs Aider / Cline / Cursor Cloud | `21 - Execution Modes.md` tabla comparativa |
| HUD tipo Hermes + Cursor + Notion + n8n + Jira | `24 - HUD Mission Control.md` §1, tablas §3 |
| Orquestación "sabe cuándo cambiar de cerebro" (OpenRouter Fusion / Fugu / AionUI) | `Contexto de lo que se quiere conseguir.txt`, `00 §7.2` |
| Igualar opencode / Codex CLI / Antigravity / Gemini CLI / aionUI | `README.md` §"Why Atlas OS" |

Los targets nombrados explícitamente: **opencode**, **Claude Code**, **Codex CLI**, **Cursor**, **Windsurf**, **Hermes**, **OpenClaw**, **Cline/Roo**, **Aider**, **Continue.dev**, **Kimi** (swarm pool), **OpenRouter Fusion / Sakana Fugu / AionUI** (orquestación).

---

## 2. Qué hay realmente implementado (medido)

| Métrica | Valor | Fuente |
|---|---|---|
| Rust LOC | **76.662** en **268** archivos | `src-tauri/src/**/*.rs` |
| Svelte LOC | **3.200** en **10** componentes | `src/**/*.svelte` |
| Tests Rust (`#[test]`/`#[tokio::test]`) | **1.468** (baseline `cargo test --lib` = **1103 ok**, README) | grep |
| Tests frontend | 67 (README) | — |
| Commits | **203** | `git log` |
| Schema SQLite | versión **37**, **58** tablas | `journal/schema.rs` |
| Rutas HUD (axum) | **32** | `hud/*.rs` |
| Módulos CLI | **30** subcomandos | `cli/commands/` |
| Stubs/TODO activos | 43/268 archivos (0 `todo!()`/`unimplemented!()`); 56 "stub", 9 `StubNotWired`, 10 "deferred" | grep |
| Fases cerradas | Roadmap v1 (0–10) completo; v2 (11/12/14/15); v3 (calendar); Fases 16–26 + F33–F39 | `20 - Roadmap.md` |

**Features del default build:** `tauri, hud, lsp, cli, bundled-skills`.
**Apagadas por defecto:** `fastembed` (embeddings/Vector KB), `dag_mode` (grafo de misión), `codebase-graph`/`ast` (tree-sitter), `acp-server`, `calendar-ics`, `laya`.

### Infraestructura real (no scaffold)

- **Kernel Bus + Journal** SQLite (58 tablas) at-least-once idempotente — real.
- **Model Orchestrator Phase 2 completo:** `registry`, `provider`, `wire` (normalización), `tokenizer`, `cache_control`, `cooldown`, `backpressure`, `routing` (6 estrategias), `cascade`, `idempotency`, `cost_guard`, `aggregation` (MoA + majority vote), `classifier`, `affinity` (feedback), `verdict`/`reliability_gate`. **Llama HTTP real** (`orchestrator/client.rs`, reqwest, OpenAI-compatible).
- **Supervisor/estado:** doom-loop hard-deny, budget caps, checkpoints, reanudación, heartbeats — implementado.
- **Coding loop LLM:** `llm`→`Diff` codec, `apply_diff`, `verify_diff`, repair, terminal agent loop (`orchestrator/agent.rs`) — implementado.
- **Validation:** 12 stages (type_check, unit_tests, lint_format, dead_code, layer_boundary, security_scan, supply_chain, iac, e2e, static_analysis, evidence_gate…).
- **Swarm:** worktrees (git), presets, pool, mailbox, rebase, merge de diffs.
- **Seguridad:** firmas SHA-256 de skills, `SandboxLevel`, supply-chain gate.
- **Integraciones laterales:** firecrawl, calendar (ICS+Graph), toast, ACP, LSP, mobile/artemis.

### Stubs principales (lo que aún no cierra)

- `acp/listen_worker.rs` (12), `skills/sdk.rs` (11), `toast/manager.rs` (8), `acp/delegate.rs` (7), `repair/runner.rs` (7), `validation/stages/lint_format.rs` (7).
- **No existe un motor `reasoning/`** propio: CoT/ToT/debate/self-refine viven dispersos en `orchestrator/aggregation`, `prompt/steps/self_refine.rs`, `orchestrator/council.rs` (RFC 03 lista "Reasoning Engine" como motor propio — no materializado como tal).
- **MCP (RFC 07)** es mínimo: casi todo el soporte está en `cli/commands/mcp.rs`; no hay bridge stdio JSON-RPC robusto.
- **Vector Knowledge (RFC 09)** depende de `fastembed` + `sqlite-vec`, **apagado por defecto** y con `load_extension` best-effort.

---

## 3. La brecha crítica: competencia agéntica end-to-end ≈ 0

Evidencia publicada por el propio proyecto (`README.md` §Evaluation baseline):

- **Terminal-Bench 2.0** (`AtlasAgent` × `kimi-k3`, NIM): **89/89 trials, pass_rate 0.000**. El solver de referencia `oracle` da **≈0.88** en el mismo harness → el instrumento mide bien.
- **F38 agent-mode:** **0/11**.
- Conclusión textual del README: *"el 0.000 es la capacidad actual de Atlas (aún no ejecuta comandos de terminal ni verifica artefactos por sí mismo)"*.
- `orchestrator/agent.rs` (F39) es el primer bucle de terminal, verificado solo contra tareas triviales (Groq).
- El gate local `atlas eval run golden` da **6/6**, pero son invariantes triviales (idempotencia de migración, supply-chain, solape de calendario) — **no miden capacidad de programar**.

Esto significa: **en la tarea que define a los objetivos (resolver problemas de código/terminal de forma fiable), Atlas está en ~0, mientras Claude Code / Codex CLI / opencode / Cursor son producto de nivel 4–5.**

---

## 4. Scorecard por objetivo

Escala: **0** no existe · **1** spec · **2** scaffold · **3** funcional interno · **4** producto usable · **5** competitivo.

| Objetivo | Dimensión comparada | Nivel Atlas | Evidencia |
|---|---|---|---|
| **opencode / Claude Code / Codex CLI** | Resolver tareas de código de punta a punta | **2–3** | Plumbing completo (diff/apply/verify/repair/terminal), pero Terminal-Bench 0.000 |
| **Cursor / Windsurf** | IDE + UX (editor, review queue, demos) | **1–2** | Sin editor (RFC 17 declarado reemplazado); HUD = panel de debug |
| **Hermes** | HUD local + profiles + worktrees | **3** | HUD WS, profiles, worktrees reales |
| **Hermes** | Agente persistente 24/7 multi-canal + cloud | **1–2** | Runtime headless real: `atlas serve` (RFC 29 §3.A) sobrevive al desktop; falta el gateway multi-canal (§3.B) y user modeling (§3.C) |
| **OpenClaw** | Extensible + **seguro** | **3** | Firmas SHA-256, sandbox types, supply-chain gate (scaffold sólido) |
| **Cline / Roo / Aider** | Gobernanza (doom-loop, budget) | **4** | `DoomLoopDetector` hard-deny + budget caps + 4 modos (spec *y* código) |
| **OpenRouter Fusion / Fugu / AionUI** | Orquestación multi-modelo adaptativa | **4** | Phase 2 completa: routing/cascade/normalización/coste/backpressure/affinity |
| **Kimi (pool swarm)** | Swarm distribuido paralelo | **3** | Pool + mailbox + worktrees + merge; falta validación end-to-end a escala |

---

## 5. Tres brechas estructurales

**A. Capacidad agéntica ≈ 0 (la brecha #1).**
La infraestructura de órquestes es de las más completas del mercado OSS, pero el último tramo (entender el repo, ejecutar, verificar artefactos, iterar hasta "done") no funciona en tareas reales. Es donde opencode/Codex/Claude Code/Cursor ganan y donde Atlas pierde.

**B. Producto/UI: el HUD es un panel de debug, no el Mission Control de RFC 24.**
Real implementado: 8 cards (Autoresearch, Availability, Eval, GraphView, JournalObserver, ModelReady, SpendLimitError, SwarmConsole) + 11 "tail boxes". **No existen** las 8 views prometidas (Kanban/Canvas/Outline/Timeline/Cost&Res/Health KPIs/Audit/Worktrees), ni la approvals queue UI, ni las agent cards de 15+ campos, ni un editor de texto. Frente a Cursor/Windsurf/Hermes, la capa de producto está a años-luz.

**C. Persistencia cloud / multi-canal ausente.**
**Actualizado:** el runtime ya no muere con el desktop — `atlas serve` (Fase 30.0, RFC 29 §3.A) es un daemon headless (Kernel Bus + HUD axum sin webview, bind configurable). Sigue pendiente el gateway multicanal (§3.B) y el user modeling (§3.C). Antes: el sistema moría al cerrar el desktop Tauri. No hay "AI employee" 24/7 ni canales (WhatsApp/Slack/Teams…). Es exactamente la brecha que `29` reconoce (§3.A/B).

---

## 6. Dónde Atlas ya iguala o supera (honestidad bidireccional)

- **Gobernanza de loops:** `doom_loop` hard-deny + budget hard + 4 modos + checkpoints es más estricto que Aider, Cline/Roo y Cursor Cloud (que no exponen detector mecánico).
- **Orquestación multi-modelo:** la profundidad de Phase 2 (normalización de wire, cache control, cooldown, backpressure, cost guard, MoA, affinity) supera a lo publicado por muchos competidores CLI.
- **Observabilidad:** Journal SQLite de 58 tablas + 32 rutas HUD + export posting es más auditable que la media.
- **Seguridad de supply-chain:** firmas + gate determinista (typosquat/postinstall/env) resuelve la crítica a OpenClaw.
- **Plataforma:** single-binary multi-OS, swarm con worktrees reales, mobile (artemis) validado en dispositivo físico.

---

## 7. Veredicto

Atlas OS es hoy un **sistema de orquestación y observabilidad de amplitud inusualmente alta con competencia agéntica casi nula**. Su perfil no es "CLI de código que compite con Claude Code"; es "SO de agentes" cuya *maquinaria* está por delante de varios competidores, pero cuyo *resultado de tarea* está por detrás de todos los objetivos nombrados.

- **Dónde brilla:** arquitectura, routing, journal, gobernanza, seguridad, single-binary.
- **Dónde está a 0:** resolver tareas reales (Terminal-Bench 0.000), UI/producto, persistencia cloud.
- **Riesgo central:** mucha superficie (48 migraciones, 30 subcomandos, ~20 features) y poca profundidad end-to-end → riesgo de "teatro de ingeniería" si no se cierra la brecha A.

---

## 8. Recomendaciones priorizadas

1. **Cerrar la brecha A antes que añadir superficie.** Congelar features nuevas; invertir todo en el bucle agéntico (herramientas de filesystem/exec, verificación de artefactos, reparación) hasta mover Terminal-Bench de 0.000 a >0.1 → luego >0.5. Es el único KPI que decide si el proyecto "iguala/supera" a los objetivos.
2. **Re-fundar el HUD** sobre las 8 views de RFC 24 (empezar por Kanban + approvals queue + agent cards), o declarar explícitamente el recorte.
3. **Hacer honesto el default build:** o activar `fastembed`/`ast` por defecto, o documentar que el producto por defecto **no** tiene memoria vectorial ni grafo.
4. **Reconciliar spec vs código:** añadir un `REASONING ENGINE` real o corregir RFC 03 (9 motores, no 10); robustecer MCP (RFC 07) que hoy es casi nominal. **→ MCP: ✅ Fase 29.0** — runtime real (`mcp/`: registry dual-shape opencode/RFC07 + JSON-RPC 2.0 + stdio con `cmd /C` en Windows + allowlist §4; CLI `atlas mcp list|add|remove|probe|call`), verificado en vivo contra Context7 (`probe` + `call` reales). Reasoning Engine **resuelto por reconciliación (2026-10): RFC 03 corregido a 9 motores + Reasoning como capacidad transversal**, materializado en `orchestrator/aggregation/` (`moa.rs`/`majority_vote.rs`/`council.rs`/`self_refine.rs`/`reflexion.rs`/`self_discover.rs`) + `orchestrator/reliability_gate.rs`. No se añade un módulo `reasoning/` (evitaría superficie sin profundidad).
5. **Mantener el baseline externo en CI** (no solo el golden 6/6) para que la capacidad no pueda fingirse con tests triviales.

---

## 8bis. Consolidado de benchmarks (2026-10-04, actualiza §3)

La brecha A (§3, "Terminal-Bench 0.000") **se movió ese mismo día**. Tres corridas reales,
todas bajo Harbor 0.23 (Docker vía WSL2):

| # | Agente × modelo | Modo | Trials | pass_rate | coste |
|---|---|---|---|---|---|
| 1 | `AtlasAgent` × `kimi-k3` (NIM, nube) | `--coding` | 89/89 | **0.000** | API |
| 2 | `AtlasAgent` × `gpt-oss-120b` (Groq, nube) | `--agent` | 11/11 | **0.000** | API |
| 3 | `AtlasAgent` × `qwen3.8-flash-next-iq2_xs` (**125B local**, Strata) | `--agent` | 89/89 | **>0** (`fix-git`, `prove-plus-comm`) | **$0** |

**Control:** `oracle` ≈ **0.88** en el mismo harness → el instrumento mide bien.

**Lectura por corrida:**
- **#1** el loop `--coding` edita ficheros pero **no ejecuta comandos** → 0.
- **#2** el `--agent` (F39) **sí ejecuta comandos**, pero el modelo free-tier no resuelve → 0.
- **#3** el modelo de frontera **local** rompe el 0.000. **La barrera no era Atlas ni la
  infra: era el modelo.**

**Causas raíz resueltas para llegar a #3 (todas infra real):**
1. **GLIBC mismatch** — el binario se compiló con glibc 2.39 (Ubuntu 24.04) y las tareas usan
   Debian 12 (glibc 2.36) → no arrancaba. Fix: **binario estático musl**
   (`x86_64-unknown-linux-musl`).
2. **CLI no aislado de Tauri** — los plugins de Tauri eran deps no-opcionales → metían
   `glib`/`gtk` en `--features cli`, imposibilitando el build estático. Fix: plugins
   opcionales tras el feature `tauri` + `build.rs`/`run_app`/`core::ipc` gateados.
3. **Red contenedor↔modelo** — Strata escucha en el host; activado `networkingMode=mirrored`
   en WSL + **relay `socat`** `WSL :8099 → Strata` para que los contenedores de Harbor
   alcancen el modelo local. Firewall de Windows abierto para 8080.
4. **Qwen local** — Strata (motor de offload MoE, MIT) corre Qwen3.8-Flash-Next 125B en una
   RTX 5070 (12 GB) + 64 GB RAM a 70-80 tok/s, expuesto OpenAI-compatible en `:8080`.

**Impacto en §3 y §8.1:** la brecha ya no es "0" sino ">0 con modelo de frontera local",
lo que **valida la recomendación 1** (invertir en el bucle agéntico — RFC 63 — es la vía
correcta) y confirma el RFC 63 como plan vigente para subir `pass_rate ≥0.10 → ≥0.50`.

---

## Apéndice — evidencia cruda

- `README.md` §Status / §Evaluation baseline (Terminal-Bench 2.0 0.000, oracle 0.88; golden 6/6; 1103 tests).
- `Atlas OS/20 - Roadmap.md` (fases 0–26 + F33–F39; estados).
- `Atlas OS/26 - Index & Cross-References.md` (catálogo 00–60).
- `src-tauri/src/{orchestrator, superviso, coding, validation, journal, swarm, hud}` (medido).
- `git log --oneline` (203 commits; último 2026-10-04).
