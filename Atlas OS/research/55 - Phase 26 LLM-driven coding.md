# research/55 — Fase 26: LLM-driven Coding en el execution loop

- **Fecha:** 2026-10-03
- **Estado:** aprobado; v26.0 **COMPLETA** (v26.1–v26.3 pendientes).
- **Contexto:** cierre de la Fase 25 (research/54). El loop `atlas execute` ya enruta
  (cascade/gate/coste/cancel) pero **no edita ni valida**: pide chat y registra el contenido.
  El loop heurístico `atlas run` (`cli/commands/mission.rs::run_single_step`) **sí** hace
  Coding → Validation → Repair, pero sin LLM.

## 1. Problema (por qué ahora)

Dos loops paralelos que no se tocan:

| Loop | Hace | LLM | Diff | Validation/Repair |
|---|---|---|---|---|
| `atlas run` (heurístico) | Coding heurístico → Diff → Validate → Repair | no | sí | sí |
| `atlas execute` (Fase 25) | `call_with_cascade` (chat) → registra contenido | sí | no | no |

Consecuencia para B: un benchmark externo corrido contra `atlas execute` mide "un LLM que
charla", no "un agente que edita código y se auto-valida". Y `atlas run` no puede medir el
harness LLM porque no enruta. **Falta el puente: que el LLM produzca un `Diff` que entre al
loop Validation→Repair ya existente.**

## 2. Decisión

Fase 26 = **LLM-driven Coding** (RFC 13 §2 — explícitamente la "Phase 2" del Engine: el campo
`Diff.model_id` documenta *"heuristic-v0 for Phase 1; model id for Phase 2"*). El orquestador
pide al modelo un `Diff` **estructurado** (JSON), lo parsea a `coding::types::Diff`, y ese
`Diff` fluye por los MISMOS motores puros que usa el path heurístico:

`Step → [LLM] → parse_diff_json → Diff → validation::runner::run → (Fail/Critical) → repair::runner::run`

Todos los motores ya existen y son **puros**:

- `validation::runner::run(&ValidationInput{ diff, mode, previous_critical_failure, model_id })`
  → `ValidationReport` (heurístico: inspecciona el texto del `Diff`; no shell-ea cargo).
- `repair::runner::run(RepairInput{ source_diff, triggered_by_report, previous_attempts,
  mission_failure_count, model_id })` → `RepairReport`.
- El supervisor ya modela `ValidationPassed` / `ValidationFailed` / `CriticalValidation` /
  `RepairAttempted` y las acciones `TriggerRepair{report_id}` / `TriggerRevalidation{diff_id}`.

## 3. Sub-fases

- **v26.0 (COMPLETA)** — codec puro `coding/llm.rs`: `DIFF_CONTRACT_PROMPT` (contrato JSON),
  `DiffMeta`, `parse_diff_json(content, meta) -> Result<Diff, DiffParseError>`. Tolerante a
  fences/prosa; normaliza `\`→`/`; clampa `old_end`; rechaza vacío / sin JSON / JSON inválido /
  sin files / file sin hunks. Golden task `coding.llm_diff_codec`. 11 tests.
- **v26.1** — `orchestrator`: pedir el `Diff` al modelo por step (`ChatRequest` con
  `DIFF_CONTRACT_PROMPT` en el system message) y parsear con `parse_diff_json`; tests con mock.
- **v26.2** — wiring del `Diff` por el loop puro: `validation::runner::run` + (en Fail/Critical)
  `repair::runner::run`, alimentando los eventos del supervisor; workspace inyectado (sin FS).
- **v26.3** — CLI/HUD: pill de step; `atlas execute` reporta diff/validación/repair; cierre.

## 4. No-goals

- **No** aplica el `Diff` a disco todavía (la Validation heurística no lo necesita; la
  aplicación real al workspace es del kernel, posterior).
- **No** toca el path heurístico (`atlas run`) — se reutiliza, no se reemplaza.
- Anthropic/Gemini siguen fuera (research/54).

## 5. Fuentes

- RFC 13 §2/§8 (Coding Engine, structured `Diff`s, Phase 2 provenance).
- RFC 14 §4/§8 (throttling, critical escalation) y RFC 15 §4/§6 (repair escalation).
- `coding/runner.rs::CodingInput` (*"el runner NO toca el filesystem"*), `coding/types.rs`.
- `cli/commands/mission.rs::run_single_step` (loop heurístico existente).
- research/53/54 (A→B; por qué el harness LLM debe cerrar el ciclo).
