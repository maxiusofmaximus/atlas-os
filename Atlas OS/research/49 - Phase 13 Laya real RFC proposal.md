# 49 - Phase 13 (Laya real) — RFC propuesta (SIN implementar)

**Estado:** PROPUESTA. Decisión del operador (2026-10-03): escribir el RFC, no
tocar código hasta confirmar write-access o un bump >0.2.x.

## Objetivo (cuando se autorice)

Integrar inferencia Laya real detrás de la gate `laya = []` (feature vacía hoy,
shape fijo) sin cambiar callers:

1. Inference real (candle) detrás de la gate `laya`.
2. `compaction` wiring (RFC 28 §5.3: `weak_model` → Laya).
3. `winnow` (judging de tool-results).

## Disparador — re-audit

- `scripts/laya_reaudit.ps1` + `research/laya_reaudit_2026-10-03.md`:
  - crates.io `laya 0.1.1` (updated 2026-09-20) — **sin cambio de versión**.
  - GitHub: **2 contributors** (`aovestdipaperino`, `enzinol`) → eje
    "mantenedores ≥2" **CUMPLIDO** por recuento de contributors.
  - `tokenizers = "0.21"` y sin dep directa de `rand` → sub-problema resuelto.
- **Caveat:** la API pública de contributors **no** confirma write-access. El
  operador ha decidido mantener esto como propuesta hasta confirmarlo, o hasta
  que la versión supere 0.2.x.

## Alcance propuesto (solo tras autorización)

- Feature `laya` real: swap de la gate vacía por `dep:laya` (o fork pinneado si
  el upstream rompe el contrato) **sin cambiar callers** (9.1).
- `orchestrator`/`compaction`: `weak_model` → Laya tras el criterio de calidad.
- `winnow`: judging de tool-results.
- Tests: happy-path (feature on) + failure-path (feature off / modelo ausente),
  fail-safe patrón lateral.

## Criterios de aceptación (futuros)

- [ ] `cargo check --features laya` compila.
- [ ] Gate off: comportamiento idéntico al actual (suite verde).
- [ ] Gate on: compaction/winnow usan Laya; KPI alucinaciones ≤1/100 diffs.
- [ ] Re-audit confirma write-access **o** versión >0.2.x.

## Fuera de alcance

- No implementar en v3 sin nueva autorización; no forkear sin decisión explícita.
- No bundling de runtimes externos (RFC 25 §11 single-binary sigue vigente).
