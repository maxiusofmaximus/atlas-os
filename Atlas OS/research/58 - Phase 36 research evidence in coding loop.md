# research/58 — Fase 36: research evidence en el coding loop

- **Fecha:** 2026-10-03
- **Estado:** **COMPLETA** (v36.0).
- **Contexto:** Fase 26 dejó el coding loop (routed `Diff` → Validation → Repair). En el test
  `coding_step_parses_a_diff_and_verifies_it` se observó que un `Diff` de código **siempre**
  va a repair: la EvidenceGate (RFC 30 §2.1) exige evidencia además de narrativa.

## 1. Hallazgo (el gap era menor de lo previsto)

El **motor de Research ya está construido** (RFC 10, Fases 3.0–3.5): `research::collective`
(gather/score/build_report), `docs_gateway`, `consensus`, `feasibility`, `hands_on`, y el CLI
`atlas research query|note|branches|docs|search|feasibility|ingest`. El roadmap Fase 3 estaba
más avanzado de lo que el playbook asumía.

Lo que faltaba no era el motor, sino **conectar** la evidencia al coding loop. La EvidenceGate
pasa un `Diff` de código si: `narrative ≥ MIN_NARRATIVE_LEN` **y** (toca test **o**
`research_refs` no vacío **o** `risk_decision`). El coding loop de F26 no ponía ninguno.

## 2. Decisión

- Añadir `ResearchContext { run_id, refs, notes }` al coding loop.
- El system prompt pide al modelo citar las evidencias por **índice** (`"cites": [0,1]`); el
  codec resuelve índices → UUIDs **reales** del contexto y los adjunta al `Diff`. El modelo
  no puede inventar un UUID: solo elige índices.
- El prompt pide además **tocar/crear un test** cuando el cambio afecta comportamiento (la vía
  de evidencia por test, que no depende de refs externas).

## 3. Sub-fase

- **v36.0 COMPLETA** — `orchestrator/code.rs`: `CODING_DIFF_PROMPT`, `ResearchContext`
  (con `is_empty`), `parse_cited_diff` (segundo parse tolerante que captura `cites`),
  resolución `cites`→refs; `StepDiffOutcome.narrative_missing`. `call_diff_with_cascade_and_denied`
  gana un parámetro `research`; wrappers pasan `&ResearchContext::default()` → comportamiento
  intacto. CLI `atlas execute --coding [--research-ref ...] [--research-run ...]`
  (`research_context_from`; texto libre → UUID determinista `uuid_from_text` sin la feature
  `v5`). 2 tests (`research_refs_are_attached_from_citations`,
  `invalid_citation_indexes_are_ignored`).

## 4. Diferido (deliberado)

- Auto-descubrir el run de research desde el journal (el journal no tiene un
  `list_research_runs`; añadirlo es otra sub-fase). Hoy el operador pasa los refs, que es
  explícito y determinista.
- `atlas research query` → `execute --coding` encadenados: script/composición posterior.

## 5. Fuentes

- RFC 10 (Research Engine), RFC 30 §2.1 (EvidenceGate), RFC 13 §6 (`research_refs`).
- `validation/stages/evidence_gate.rs` (condición exacta de paso).
- research/55 (Fase 26), research/57 (Fase 35).
