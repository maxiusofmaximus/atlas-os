# 51 — Phase 22 EVAL (Evaluación & Instrumentación — harness Layer 5)

- **Estado:** plan aprobado; EVAL.0–EVAL.1–EVAL.3 implementadas (EVAL.2/EVAL.4 pendientes).
- **Fecha:** 2026-10-03.
- **Motivación raíz:** investigación externa "harness engineering" (Round 9). La capa
  de mayor ROI que Atlas no tiene es **Evaluación/Instrumentación**, y ya estaba
  planificada y diferida (`eval_runs`, gap **G16**, `research/29` Phase 2.5;
  registrada como no implementada en `research/50` línea 88).

## 1. Evidencia externa (por qué esta fase, y por qué ahora)

Fuentes independientes convergen en 2026:

- **Harness Engineering — construir las 6 capas al revés, 6→1** (heyuan110,
  2026-04-18). De Contexto → Herramientas → Ejecución → Memoria → **Evaluación** →
  **Recovery**, las capas **5-6 concentran ~80% de la estabilidad**. Ganancias
  medidas: Eval **+22%** en 1 semana vs Contexto **+12%** en 2; Recovery **+18%**
  vs Memoria **+3%**. Cita: *«si el agente está atascado en la banda 60-70%, el
  problema no es la capa que tocas, es que no tienes la instrumentación para saber
  cuál está rota»*.
- **Mark Laursen, 18 meses de agentes en producción** (2026-05-06). 21 de 23
  post-mortems = 5 modos: compaction silenciosa corrupta, **tormenta de reintentos
  (coste)**, **cascada de tool-calls sin policy gate**, drift en handoffs
  multi-agente, desajuste de confianza. Remate: *«una vez tuvimos infraestructura de
  evaluación fiable… el 70% de las invocaciones migró a modelos más baratos»*.
  *«La arquitectura importa más que la elección de modelo»*.
- **T2D3 OS / OpenAI "Harness engineering" / Martin Fowler (Böckeler, 2026-04-02)**:
  el valor no está en el agente sino en el **harness** — guías (feed-forward) +
  **sensores** (feed-back), invariantes convertidos en checks de CI, *garbage
  collection* de drift. OpenAI: *«nuestros retos más difíciles ahora son diseñar
  entornos, feedback loops y sistemas de control»*.
- **Estándar de evaluación**: **Harbor** (harness oficial de **Terminal-Bench 2.0**;
  soporta agentes tipo OpenCode/Claude Code/OpenHands/Aider). La unidad de medida es
  el **par harness–modelo**. Paper **KDD 2026 "The Scaffold Effect"**: la elección de
  harness induce hasta **40× de diferencia de tokens** aunque el pass rate sólo varíe
  0-8 pp. Métricas obligatorias: **pass rate + tokens/tarea-resuelta + turnos sin
  acción + vector de categorías de fallo**.
- **Anécdotas (r/AI_Agents, r/ClaudeCode, r/agenticAI)**: el caos nº1 es el tree
  compartido (→ worktree **y** aislamiento de runtime: db/puertos/node_modules);
  *«bounds, not retries»* (3 intentos → para, revierte, reporta); multi-agente =
  **sistema distribuido**.

## 2. Diagnóstico Atlas

- **Tiene** capas 1-4 y buena parte de la 6: orquestador (Fase 2), swarm con
  worktrees (Fase 4), learning/compaction (Fase 5), **Execution Supervisor** —
  state machine + `doom_loop` + heartbeats + checkpoints + reanudación + observer
  (Fase 6), seguridad + supply gate (Fase 7).
- **Le falta la capa 5** (evaluación). No existe infra de benchmark en el código
  (grep `swe-bench|terminal-bench|benchmark|eval_harness` → sin resultados).
- Consecuencia: el clasificador `log_reg` (2.3), el **feedback loop** (2.4) y el
  **cost guard G11** operan **sin instrumento**; no se puede validar ni priorizar.
- **v3.1.2 (Planning proactivo)** está bloqueado en infraestructura (sin motor de
  turno proactivo). Construirlo sin eval = no poder medir si ayuda o estorba.

## 3. Diseño

1. **Persistencia** (`eval_runs` + `eval_cases`): una fila por corrida de suite
   (harness × modelo × suite) y una por caso, con las métricas del campo.
2. **Runner local determinista** (golden task set en-repo) sobre los motores
   existentes → pass/fail por caso.
3. **Instrumentación**: tokens/tarea-resuelta, turnos sin acción, validez de
   tool-calls, vector de categoría de fallo, coste/mission; visible en HUD y
   consumible por el feedback del orquestador.
4. **Adaptador Harbor**: registrar `atlas` como agente y correr Terminal-Bench 2.0 /
   SWE-bench Verified (subconjunto) → baseline público reproducible.
5. **Gate de CI**: la golden eval corre en cada cambio de motor.

## 4. Sub-fases atómicas (un commit por sub-fase)

### EVAL.0 — Foundation (schema + repositorio) — **EN IMPLEMENTACIÓN**
- Migración **M36 (schema v35)**: `eval_runs` + `eval_cases` (índices por suite y por
  run). Creadas incondicionalmente (misma regla que M18).
- `journal/eval_runs.rs`: `EvalRunRow`, `EvalCaseRow`, `EvalRunStart`,
  `EvalCaseInput`, `EvalTotals` + CRUD (`eval_run_start/record_case/finish/get/
  list/cases/delete`).
- Tests: ciclo start→casos→finish, listado, borrado, CHECK de status inválido.

### EVAL.1 — Runner local + golden suite — **COMPLETA**
- `eval/mod.rs`: `EvalTask` / `TaskResult` / `TaskStatus` + `run_suite`
  (in-process, `catch_unwind` por tarea → `error`/`REASON`, persiste en
  `eval_runs`/`eval_cases`).
- `eval/golden.rs`: 6 invariantes deterministas **offline** (schema idempotente,
  supply-gate ×4, calendario half-open).
- CLI `atlas eval run [suite] | list | report [id]` (`cli/commands/eval.rs`).
- Smoke: `atlas eval run golden` → **6/6**, persistido; `report`/`list` OK.

### EVAL.2 — Instrumentación + HUD + feedback
- Métricas normalizadas + card HUD (`<EvalCard.svelte>`).
- Cableado al feedback del Model Orchestrator (2.4) y al cost guard G11.

### EVAL.3 — Adaptador Harbor — **COMPLETA**
- **Importador** `eval/harbor.rs`: ingesta un `JobResult`/`TrialResult` de Harbor
  (`results/<job>/result.json`, un trial suelto o un dir de job) en
  `eval_runs`/`eval_cases`. Parseo tolerante por `serde_json::Value` contra el
  esquema pydantic real (`trial_name`, `source`, `agent_info.model_info.name`,
  `agent_result.{n_input_tokens,n_output_tokens,cost_usd}`,
  `verifier_result.rewards{}`, `exception_info`). Offline — sin Harbor/Docker.
- CLI `atlas eval import <path>`.
- **Scaffold del agente** `tools/harbor_atlas/` (`atlas_agent.py`
  `AtlasAgent(BaseInstalledAgent)` + `run-harbor.ps1` + README): dev-only, no
  bundleado (RFC 25 §11).
- 4 tests del importador (job con pass/fail/error, dir, trial suelto, fichero
  ausente).

### EVAL.4 — Gate CI + baseline público
- Job de CI que corre la golden eval; baseline en README.

## 5. Métricas (definición)

| Métrica | Definición | Fuente |
|---|---|---|
| `pass_rate` | casos `pass` / total | Harbor / KDD |
| `tokens_per_solved` | tokens totales / casos resueltos | KDD "Scaffold Effect" |
| `no_action_turns` | turnos sin efecto observable | KDD |
| `failure_kind` | REASON / VERIFY / MAX_TURNS / IDLE / TIME / SCOPE | KDD fingerprint |
| `cost_usd` | coste por misión/caso | Laursen (retención de coste) |

## 6. No-goals / riesgos

- **No** construir v3.1.2 (Planning proactivo) antes de EVAL: sin instrumento no es
  medible. Eval primero hace medible Planning, router y swarm.
- No reimplementar SWE-bench/Terminal-Bench: usar Harbor.
- Riesgo: eval se vuelva *coverage theater* (métricas que no discriminan) — mitigado
  con el vector de fallos + ablate del harness (KDD, Fowler).

## 7. Fuentes

- heyuan110 — Harness Engineering (6→1): https://www.heyuan110.com/posts/ai/2026-04-18-harness-six-layers-reverse-build/
- Mark Laursen — 18 Months of Production AI Agents: https://marklaursen.com/blog/18-months-production-ai-agents-lessons
- T2D3 OS — Six Months Shipping Software With Claude Code: https://www.t2d3.pro/learn/six-months-shipping-software-with-claude-code
- OpenAI — Harness engineering: https://openai.com/index/harness-engineering/
- Martin Fowler / B. Böckeler — Harness engineering for coding agent users: https://martinfowler.com/articles/harness-engineering.html
- Harbor (Terminal-Bench 2.0 official harness): https://github.com/harbor-framework/harbor
- The LLM Stack — Agent Evaluation & Benchmarks: https://prakashkagitha.github.io/llm-stack-book/08-agents-harness/08-agent-evaluation.html
- KDD 2026 — The Scaffold Effect in Coding Agents: https://kdd-eval-workshop.github.io/agenticai-evaluation-kdd2026/assets/papers/74_The_Scaffold_Effect_in_Codi.pdf
- Autonomous coding agents break at the seams: https://dev.to/zoetaka38/autonomous-coding-agents-dont-break-in-the-middle-they-break-at-the-seams-cb8
