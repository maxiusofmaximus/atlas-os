# RFC 63 — Agentic Capability Engine (Fase 27)

**Author:** opencode architect agent · **Date:** 2026-10-04
**Status:** Proposed — 0 de los 13 items implementados.
**Depends on:** RFC 03 (Engines), RFC 04 (Orchestrator), RFC 05 (Swarm), RFC 13 (Coding), RFC 14 (Validation), RFC 15 (Repair), RFC 18 (Security), RFC 19 (Supervisor), RFC 25 (Stack), RFC 28 §I (terminal-browser), `research/61` (audit), `research/62` (genealogy).
**Scope:** Cierra la **brecha #1** de la auditoría `61` — la capacidad agéntica end-to-end (Terminal-Bench 2 = **0.000**, F38 agent-mode **0/11**). Añade una **Capa de Capacidad** (tool registry + sandbox de ejecución + verificación de artefactos + instrumentación de tokens/coste) y endurece el bucle agéntico hasta un success-predicate con evidencia. Sin crates obligatorias nuevas; sandbox local por defecto, backends laterales (WSL2/Daytona/E2B) opcionales.

> **Honestidad (regla RFC 28 §Riesgos 6):** este RFC no promete puntuaciones. Fija **mecanismos** y **KPIs medibles**; el primer gate real es mover Terminal-Bench de `0.000` a `>0.10`, y el objetivo de fase a `>0.50` (oracle de referencia ≈ `0.88`).

---

## 1. Contexto y motivación

`README.md` §Evaluation baseline documenta el baseline externo: `AtlasAgent × moonshotai/kimi-k3` sobre Terminal-Bench 2.0 → **89/89 trials, pass_rate 0.000** (oracle ≈ 0.88). El propio README concluye: *"el 0.000 es la capacidad actual de Atlas (aún no ejecuta comandos de terminal ni verifica artefactos por sí mismo)"*.

La infraestructura existe (orquestación, journal, diffs, validación), pero **el último tramo no cierra**: el agente no opera un entorno de forma fiable, no verifica el artefacto, no itera hasta éxito y no instrumenta lo que gastó. Este RFC ataca exactamente eso, sin tocar la tesis de plataforma.

## 2. Estado actual (qué ya existe / qué falta)

**Ya existe:**
- `orchestrator/agent.rs` — bucle `run_command` + `atlas agent <task>` (F39, verificado vs Groq en tareas triviales).
- `orchestrator/{client,call,execute}.rs` — cascade multi-provider + budgets + Ctrl-C (F25).
- `coding/llm.rs` (`Diff` codec) · `coding/apply.rs` (`apply_diff`) · `orchestrator/verify.rs` (`verify_diff`) · `repair/` (F26/F35).
- `validation/stages/*` (12 stages) + `EvidenceGate` (RFC 14 §10).
- `eval/` (golden 6/6 + Harbor importer, `tools/harbor_atlas/`).
- `supervisor/` (doom-loop, checkpoints, budgets) y `journal/` (schema v37, 58 tablas).

**Falta (las 5 causas raíz):**
1. **Tool layer pobre** — sólo `run_command`; sin fs read/write/edit, search, diff-apply tipado, ni browser.
2. **Sin sandbox real** — ejecución en el host; RFC 18 tiene los *tipos* (`SandboxLevel`) pero no un runtime.
3. **Sin verificación de artefactos** — el agente no comprueba el resultado (tests/archivos/salida) antes de "done".
4. **Sin instrumentación** — `AgentContext` va sin tokens/coste (`populate_context_post_run` sin rellenar).
5. **Loop no grounded** — no hay success-predicate explícito ni re-observación del entorno entre pasos.

## 3. Objetivos y KPIs

1. **Tool registry** tipado con tools de fs/exec/search/web/browse/verify y permisos por modo (RFC 21).
2. **Sandbox de ejecución** con backend local por defecto + `wsl2`/`daytona`/`e2b` laterales (jamás bundling).
3. **Verificación de artefactos** evidence-gated antes de declarar `done` (extiende RFC 14 §10).
4. **Instrumentación completa** de tokens/coste/herramientas por step; poblar `AgentContext`.
5. **Bucle agéntico robusto** con `success_predicate`, re-observación y repair hasta N intentos (respeta doom-loop/budgets).

**KPIs (gate de fase):**
- Terminal-Bench 2 `pass_rate`: `0.000 → ≥0.10` (item 9) `→ ≥0.50` (item 13).
- F38 agent-mode: `0/11 → ≥5/11`.
- Golden suite: +6 tareas nuevas que ejerzan tools/verify (0 regresiones en las 6 actuales).

## 4. Arquitectura — Capa de Capacidad

```
AgentLoop (supervisor FSM)
   │  decide acción
   ▼
ToolRegistry ──► Tool { name, schema, execute(ctx) -> ToolResult }
   │   ├── fs.read/write/edit/list/glob/grep
   │   ├── exec.run(command, timeout, cwd)   ──► Sandbox
   │   ├── code.apply_diff (coding/apply)
   │   ├── web.fetch (RFC 28 §E firecrawl / webfetch) · web.search
   │   ├── browse.open/snapshot (RFC 28 §I terminal-browser, lateral)
   │   └── verify.artifacts (evidence)
   ▼
ArtifactVerifier  ──► EvidenceGate (RFC 14 §10)
   ▼
Journal (agent_runs/agent_steps/tool_invocations/artifacts) + Bus events
```

- **Sandbox** — trait `Sandbox { exec, read_file, write_file, snapshot }`; backends: `Local` (default, con allowlist de red RFC 28 §I-style), `Wsl2`, `Daytona`/`E2B` (lateral, proceso externo). Selección por `ATLAS_SANDBOX` + `SandboxLevel` (RFC 18).
- **Permisos** — cada Tool declara `sensitivity`; `MANUAL_CLASSIC`/`HUMAN_IN_LOOP`/`AUTOPILOT`/`AUTONOMOUS` (RFC 21) mapean a `allow/ask/deny` (RFC 02 Capability Resolver).
- **Tools externas** — nada se bundlea: `browse` invoca `terminal-browser` si está en PATH (RFC 28 §I); `web.fetch` usa la facade firecrawl o `webfetch`.

## 5. Bucle agéntico robusto

```
goal(mission) → plan(steps) →
  loop (máx N, respeta BudgetCaps/doom_loop):
     act(tool) → observe(result) → verify(artifacts)
       ├─ pass  → step done
       ├─ fail  → repair (RFC 15) → reintenta
       └─ stall → recovery (RFC 19) o abort con evidencia
  → success_predicate(goal) == true  ⇒ done (EvidenceGate)
  → false ⇒ handoff a HUMAN_IN_LOOP con resumen + artefactos
```

Reglas: (a) `done` **sólo** con evidencia (tests/archivos/salida); (b) re-observar el entorno antes de cada decisión (no confiar en memoria); (c) todo tool call es idempotente o marcado (RFC 02 at-least-once); (d) tokens/coste por step, siempre.

## 6. Esquema SQL — M49/M50 (schema v38+)

```sql
-- M49 (v38): agent_runs / agent_steps
CREATE TABLE IF NOT EXISTS agent_runs (
  id TEXT PRIMARY KEY, mission_id TEXT NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
  goal TEXT NOT NULL, success_predicate TEXT NOT NULL,
  sandbox TEXT NOT NULL, status TEXT NOT NULL CHECK (status IN ('running','done','failed','aborted')),
  steps INTEGER NOT NULL DEFAULT 0, tokens_in INTEGER NOT NULL DEFAULT 0,
  tokens_out INTEGER NOT NULL DEFAULT 0, cost_usd REAL NOT NULL DEFAULT 0,
  ts_started INTEGER NOT NULL, ts_ended INTEGER
);
CREATE TABLE IF NOT EXISTS agent_steps (
  id TEXT PRIMARY KEY, run_id TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
  step INTEGER NOT NULL, thought TEXT, action TEXT, observation TEXT,
  evidence_json TEXT, verdict TEXT CHECK (verdict IN ('pass','fail','unknown')), ts INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_agent_steps_run ON agent_steps(run_id, step);

-- M50 (v39): tool_invocations / artifacts
CREATE TABLE IF NOT EXISTS tool_invocations (
  id TEXT PRIMARY KEY, run_id TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
  step INTEGER NOT NULL, tool TEXT NOT NULL, args_json TEXT NOT NULL,
  result_json TEXT, exit_code INTEGER, duration_ms INTEGER, tokens INTEGER, cost_usd REAL, ts INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY, run_id TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
  kind TEXT NOT NULL, path TEXT, sha256 TEXT, verified INTEGER NOT NULL DEFAULT 0, evidence_json TEXT
);
```

## 7. Cambios Rust (plan, no implementación)

- `orchestrator/tools/{mod,registry,fs,exec,search,web,browse,verify}.rs` (nuevo) — `Tool` trait + `ToolRegistry` + tools.
- `orchestrator/sandbox/{mod,local,wsl2,daytona}.rs` (nuevo) — trait `Sandbox` + backends (laterales gated por feature).
- `orchestrator/agent.rs` (rework) — usar `ToolRegistry` + `Sandbox`; añadir `success_predicate`, re-observación, `repair`, instrumentación; emitir `BusEventKind::AgentStep`.
- `journal/agent_runs.rs` (nuevo, M49/M50) — writers/readers tipados + wrappers en `Journal`.
- `orchestrator/execute.rs` — poblar `AgentContext` (tokens/coste) en `populate_context_post_run`.
- `eval/` — `AtlasAgent` adapter del Harbor consume `ToolRegistry`/`Sandbox`; nuevo `eval/tasks` descentralizado.
- `lib.rs` — re-exports.

Sin crates obligatorias. Backends laterales (`daytona`, `e2b`) tras features opt-in + proceso externo (RFC 25 §11).

## 8. CLI

```bash
atlas agent "<task>" --sandbox local --verify --max-steps 40 --timebox 10m
atlas agent tools                       # lista el ToolRegistry
atlas agent run --from-file task.json    # reproducible
atlas bench terminal-bench --agent atlas --limit 20   # harness de capacidad
```

## 9. HUD

`AgentCard.svelte` (nuevo): steps en vivo, tool calls, evidencia, tokens/coste, botones Pause/Stop/Handoff. Canal `agent:<run_id>` sobre Kernel Bus. (Enlaza con RFC 65.)

## 10. Tests y CI de capacidad

- Unit: cada tool (fs/exec/verify) happy + failure; registry de permisos por modo.
- Integración: golden task `agent.loop_closes` (task con 3 steps y evidencia) + `agent.evidence_blocks_done`.
- **CI de capacidad**: `.github/workflows/agent-bench.yml` corre `atlas bench terminal-bench --limit 20` y **falla si `pass_rate` baja** (ratchet, no umbral fijo al inicio).
- Baseline publicado en README (sección ya existente).

## 11. Actualizaciones a RFCs existentes

- **RFC 04 §9** — ToolRegistry/Sandbox como capability del Orchestrator.
- **RFC 14 §10** — `EvidenceGate` gana el consumidor `ArtifactVerifier` (agent).
- **RFC 19** — `success_predicate` + re-observación formalizadas en el FSM.
- **RFC 18** — backends de sandbox reales (Local/WSL2/Daytona) + SensitiveActions por tool.
- **RFC 24 §3** — `agent` card con steps/tool-calls/evidencia.
- **RFC 26** — cross-refs (`ToolRegistry`, `Sandbox`, `atlas bench`).

## 12. Checklist

1. ⏳ `Tool` trait + `ToolRegistry` (`orchestrator/tools/`).
2. ⏳ Tools `fs.*` (read/write/edit/list/glob/grep).
3. ⏳ Tool `exec.run` + trait `Sandbox` + backend `Local`.
4. ⏳ Backend `Wsl2` (lateral) + `Daytona`/`E2B` (features opt-in).
5. ⏳ `code.apply_diff` + `web.fetch`/`web.search` + `browse.*` (RFC 28 §I lateral).
6. ⏳ `ArtifactVerifier` + wiring a `EvidenceGate`.
7. ⏳ M49/M50 (`agent_runs`/`agent_steps`/`tool_invocations`/`artifacts`).
8. ⏳ Rework de `orchestrator/agent.rs` (success_predicate + repair + re-observación).
9. ⏳ Instrumentación `AgentContext` (tokens/coste) + `AgentStep` bus event; **gate Terminal-Bench ≥0.10**.
10. ⏳ `atlas agent tools` + `--verify` + `--sandbox`.
11. ⏳ `AgentCard.svelte` (live) — coordina con RFC 65.
12. ⏳ CI `agent-bench.yml` (ratchet).
13. ⏳ Subir capacidad: `pass_rate ≥0.50`, F38 `≥5/11`.
