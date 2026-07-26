# 19 - Execution Supervisor

Mantiene al sistema **vivo**. Reemplaza el script .ps1 externo por un supervisor interno completo, robusto y observable.

> La idea de usar un `TODO.md` como "memoria externa" es ingeniosa y funciona, pero yo la evolucionaría hacia un **Execution Journal** estructurado (JSON o SQLite) con checkpoints, decisiones, errores y estado de cada fase.

---

## 1. Funciones

- **Heartbeat periódico** para detectar si un agente dejó de progresar.
- Detección de estancamiento basada en **ausencia de cambios reales** (no solo tiempo).
- **Persistencia del estado**: plan, contexto, tareas pendientes.
- **Reanudación exacta** desde el último checkpoint sin depender de un TODO.md.
- **Límites** para evitar bucles infinitos de reinicio cuando el problema es estructural.

## 2. Execution Journal

Formato SQLite/JSON. Estructura mínima:

```sql
CREATE TABLE missions ( id, mission_text, status, created, updated, … );
CREATE TABLE objectives ( id, mission_id, text, status, owner_agent, … );
CREATE TABLE steps ( id, objective_id, text, status, ..., resume_point );
CREATE TABLE decisions ( id, step_id, rationale, evidence_json, confidence, … );
CREATE TABLE research_runs ( id, step_id, report_json, confidence );
CREATE TABLE files_changed ( id, step_id, path, diff );
CREATE TABLE pending ( id, mission_id, blocker_reason );
CREATE TABLE blocked ( id, mission_id, reason, awaiting );
CREATE TABLE resume_points ( id, mission_id, step_id, snapshot_json );
CREATE TABLE errors ( id, step_id, error_kind, root_cause, fix_applied );
CREATE TABLE events ( id, ts, source, type, payload_json );
```

El usuario (o cualquier modelo) puede hacer `SELECT` en el Journal para entender el estado completo del proyecto en segundos.

## 3. Schema del Journal de alto nivel

```
Mission
  ↓
Objectives
  ↓
Current Step
  ↓
Reasoning
  ↓
Evidence
  ↓
Research
  ↓
Decisions
  ↓
Confidence
  ↓
Architecture
  ↓
Files Changed
  ↓
Pending
  ↓
Blocked
  ↓
Resume Point
```

Cualquier modelo puede entrar, leer y continuar sin perder contexto.

## 4. Heartbeat

Cada N segundos:
- el supervisor pregunta al agente activo `alive?`,
- mide delta en `files_changed` y `events` desde último beat,
- si delta = 0 durante M beats → considerado stalled.

Stall en realidad != reinicia ciegamente: primero intenta **kick** (enviarle "continue according to plan", recordándole el `resume_point`). Si tras K kicks sigue sin progresar → reinicia el agente desde el último checkpoint.

## 5. Reanudación

Snapshot del resume_point contiene:
- Plan completo,
- contexto activo,
- archivos modificados,
- archivos bloqueados,
- skills activadas,
- modelo activo.

`Supervisor.resume(mission_id)` carga el snapshot y dispatcha al Planner.

## 6. Anti-infinite-loop policy (mecánica)

> OpenCode ya expone el detector `doom_loop` que dispara `ask` tras 3 repeticiones idénticas de un mismo tool call. Ni Aider, ni Cline/Roo, ni Continue.dev ni los Cursor Cloud Agents publican un detector mecánico equivalente. Aquí lo elevamos a **policia hard configurable** y añadimos los otros guard-gates que faltan.

```yaml
policies:
  anti_infinite_loop:
    detector:
      type: rolling_window
      window_seconds: 60
      min_repeats: 3
      key: "{tool_name}|{normalized_input_hash}"
      on_match:
        MANUAL_CLASSIC:    N/A
        HUMAN_IN_LOOP:     trigger_permission(doom_loop)        # ask
        AUTOPILOT:         trigger_permission(doom_loop)        # ask
        AUTONOMOUS:        hard_deny + downgrade_to_AUTOPILOT   # hard-deny

    caps:
      max_iterations: 25                # por sesión
      max_minutes: 30                   # por sesión
      max_cost_usd: 1.50                # presupuesto hard (AUTONOMOUS exige definir)
      max_consecutive_failures: 5       # 5 tests rojos seguidos -> ask
      max_context_compactions: 2        # 2 compactaciones -> escalar al humano

    goal_drift_decay:
      predicate: "errors_ast_diff != prev_errors_ast_diff AND files_touched != prev_files_touched"
      after_n_steps_without_progress: 5
      on_violation: trigger_permission(goal_drift)

    recovery:
      on_doom_loop:
        - inject_prompt: "You're repeating the same tool call. Summarize the state, list current blockers, propose 3 alternative approaches."
        - downgrade_mode: AUTONOMOUS -> AUTOPILOT
        - max_recovery_attempts: 2
      on_goal_drift:
        - inject_prompt: "Re-articulate the original goal in one sentence and identify what's blocking it."
      on_budget_hit:
        - halt_session(reason: budget_exceeded)
        - produce_summary(subagent: summary)
        - request_human_decision

    escape_hatch:
      always: [Ctrl+C, ESC]
      interrupts_within_ms: 50

    audit:
      log: every_iteration_with { iteration_n, tool, input_hash, latency_ms, cost_cents, mode_before, mode_after }
```

### 6.1 State machine del supervisor

No es un `while true` — es un state machine explícito:

```
idle → planning → executing → verifying → recovering → halted → done
```

Transiciones documentadas; cada una deja su traza en el Journal.

### 6.2 Políticas heredadas
- Tras 2 reinicios consecutivos en el mismo step → marca `blocked`.
- Tras 4 reinicios en la mission → escalar al usuario.
- Nunca entra en `while true` estilo `supervisor.ps1` sin suelo.

## 7. supervisor.ps1 deprecado

El script original:
```powershell
while ($true) {
  opencode "Lee el archivo TODO.md..."
  Start-Sleep -Seconds 3
}
```
queda reemplazado por el Execution Supervisor integrado. Elimina el `supervisor.ps1` externo y la dependencia en `TODO.md` explícito.

## 8. Otros monitores

- Heartbeat para modelos remotos (rate-limit / timeout): retry con backoff.
- Monitor de presupuesto (`max_cost_per_hour`): pausa y avisa al usuario.
- Monitor de VRAM local: si baja y un modelo local intentó swap → aborta y reasigna a modelo remoto rápido (Groq/Cerebras free tier).

## 9. Checkpoints de seguridad

Snapshots extra en:
- antes de `Pulumi apply`,
- antes de migrar schema DB,
- antes de borrar archivo >50KB,
- antes de push remoto.

Si algo sale mal, hay rollback limpio.

## 10. Observabilidad

El Journal expone logs en un panel web:
- missions activas,
- checkpoints recientes,
- stall events,
- restarts,
- aprobaciones pendientes.

Es la columna vertebral de la transparencia total mencionada en `01 - Core Principles.md`.

## 11. Modo Autoresearch (RFC 28 §A)

Karpathy autoresearch loop — hill-climbing greedy sobre commits de git, con keep verdict del supervisor (no del modelo). Añadido por RFC 28 §A; el host loop completo aterriza en Phase 2 (motores LLM-driven), pero el state machine, la persistencia en SQLite y la telemetría HUD ya son canónicas.

### 11.1 Modo de mission

`opencode mission new --autoresearch --metric "<cmd>" [--max-steps N] [--timebox Ss] "<prompt>"`:

- Inicializa una row en `autoresearch_runs` con `outcome = 'running'`, `metric_command`, `max_steps` (clampado a `HARD_MAX_STEPS = 200`), `timebox_seconds` y `git_sha_start`.
- El host supervisor itera: genera diff candidato, mide con `metric_command`, decide keep (estricto `<` vs `best_metric`), persiste en `autoresearch_candidates`, emite `autoresearch:<run_id>` telemetry a la HUD.
- Termina cuando: `step_count ≥ max_steps` (`Outcome::Timeout`), 3 resets consecutivos (`Outcome::Plateau`), `timebox_seconds` agotado (también `Timeout`), o `Outcome::Aborted` desde la HUD.

### 11.2 State machine pura

`journal::autoresearch::tick(state, event) -> AutoresearchOutput` — FSM pura, sin IO, alineada con §6.1:

```
running ──Start──> recording ──CandidateRecorded──> (kept | reset)
                                                          │
                                                          ├── plateau (3 resets)
                                                          ├── timeout (step budget)
                                                          └── aborted (HUD cancel)
```

`Outcome` (terminal): `Improved | Plateau | Timeout | Aborted`. `kept` es estricto `<` (empate → reset, ver RFC 28 §A Riesgos). `HARD_MAX_STEPS = 200` es techo anti-doom-loop.

### 11.3 Cancel desde la HUD

`POST /autoresearch/cancel { run_id, outcome }` → publica `BusEventKind::AutoresearchCancelled` en el Kernel Bus. El supervisor host escucha y transiciona a `Outcome::Aborted`, persiste, emite `JournalCheckpoint`. En Phase 1.5b el endpoint sólo es transporte: persiste el evento pero no corre el loop (todavía no hay host).

### 11.4 Atribución

El algoritmo deriva de `karpathy/autoresearch program.md` (MIT, Copyright (c) 2025-2026). Texto verbatim en `skills/autoresearch/program.md`; invariantes en `skills/autoresearch/NEVER_STOP.md`. Ver apéndice de atribución obligatoria en RFC 28.

