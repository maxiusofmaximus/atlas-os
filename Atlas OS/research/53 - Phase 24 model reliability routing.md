# 53 — Phase 24: Model reliability → routing (EVAL-informed routing)

- **Estado:** **COMPLETA** (v24.0–v24.2).
- **Fecha:** 2026-10-03.
- **Motivación:** cerrar el bucle **EVAL → routing**. Hoy `eval::metrics::model_reliability`
  existe (Fase 22) pero **nadie lo consume**: el router elige por latencia/coste/uso,
  nunca por fiabilidad histórica. La investigación (research/51 §1) es explícita:
  *«una vez tuvimos infraestructura de evaluación fiable… el 70% de las invocaciones
  migró a modelos más baratos»*, y el fallo *trust calibration mismatch* (Laursen)
  se corrige actuando **solo sobre historia estadísticamente significativa**.

## 1. Por qué un gate y no un score

- El routing (`orchestrator/routing.rs`) elige un `Deployment` por grupo `model_id`
  con una `RoutingStrategy` pura. La fiabilidad es una señal **por modelo**, no por
  deployment → se aplica como **filtro previo** (gate), no como estrategia nueva.
- Regla anti-ruido: actuar solo cuando `n >= min_samples` (research/51: medir antes
  de decidir; no degradar por 2-3 casos). Modelos sin historial → permitidos por
  defecto (`allow_unknown`).
- **Opt-in**: el gate NO cambia el default del routing (Fase 24.0 entrega el núcleo
  puro; el cableado en el host es 24.1 y la política/HUD 24.2).

## 2. Diseño

### 2.1 Núcleo puro (v24.0) — `orchestrator/reliability_gate.rs`

```rust
pub struct ReliabilityGate { pub min_samples: i64, pub min_pass_rate: f64, pub allow_unknown: bool }
pub enum GateDecision { Allow, Deny(&'static str) }

pub fn gate_model(model_id, reliabilities: &HashMap<String, Reliability>, gate) -> GateDecision
pub fn filter_deployments<'a>(deployments: &'a [Deployment], reliabilities, gate)
    -> (Vec<&'a Deployment>, Vec<(&'a Deployment, &'static str)>)
```

Default: `min_samples = 20`, `min_pass_rate = 0.5`, `allow_unknown = true`.

### 2.2 Host (v24.1)

- El host construye `HashMap<model_id, Reliability>` desde
  `eval::metrics::model_reliability` para los modelos del `Registry` y aplica
  `filter_deployments` **antes** de construir el `RouteContext` (así el trait
  `Router` sigue puro). Opt-in por política de perfil.
- Si el filtro deja 0 deployments, se cae al set original (fail-safe: nunca dejar
  el sistema sin ruta).

### 2.3 Política + HUD (v24.2)

- `ReliabilityGate` persistida por perfil (patrón `proactive_policy`), CLI
  `atlas models reliability-gate [--min-samples] [--min-pass-rate] [--allow-unknown|--strict]`,
  y exposición en el HUD.

## 3. KPI

- Con el gate activo, el pass rate agregado (harness×modelo) sube o iguala, sin
  caer el `tokens/solved` (KDD) más de un margen acordado.
- 0 degradaciones sobre modelos con `n < min_samples` (guard de ruido).

## 4. No-goals / riesgos

- **No** cambiar el default de routing sin evidencia (opt-in; gate apagado por defecto).
- **No** degradar con muestras pequeñas ni dejar el sistema sin ruta (fallback al set
  original).
- Riesgo: bucles de retroalimentación (un modelo penalizado no recibe tráfico → no
  acumula muestras que lo rehabiliten). Mitigación: `allow_unknown` + un pequeño
  cupo de exploración (v24.1+).

## 5. Sub-fases atómicas

- **v24.0 — Gate puro** (`reliability_gate.rs` + tests + golden task). **EN IMPLEMENTACIÓN.**
- **v24.1 — Host glue** — **COMPLETA**:
  - `reliabilities_from_journal(journal, models, limit)` — puebla el mapa desde
    el store EVAL (`eval::metrics::model_reliability`); modelos sin historial no
    aparecen.
  - `gate_refs` / `gate_refs_with_denied` — la forma que consume el closure
    `healthy_for` del cascade; **fail-safe**: si el gate dejaría 0 candidatos,
    devuelve el set original (nunca sin ruta).
  - 3 tests (fallback total, parcial, poblado desde EVAL).
- **v24.2 — Política persistida + CLI + HUD** — **COMPLETA**:
  - Migración **M38 (schema v37)** `reliability_gate` (fila única) +
    `journal/reliability_policy.rs` (`load/save_reliability_gate`; default
    **disabled** = opt-in) + `Journal::eval_models` (modelos con historial EVAL).
  - CLI `atlas models reliability-gate [--min-samples] [--min-pass-rate]
    [--allow-unknown|--strict] [--enable|--disable]`.
  - HUD `GET /hud/reliability` (política + mapa `model → Reliability`).
  - 2 tests (roundtrip de política, handler HUD).

## 6. Fuentes

- `research/51` (EVAL; trust calibration, unidad harness×modelo) y `Atlas OS/22 §15`.
- Laursen, "18 Months of Production AI Agents" — trust calibration mismatch.
- KDD 2026 "The Scaffold Effect" — tokens/solved como métrica de eficiencia.
- `Atlas OS/04 - Model Orchestrator.md` (RFC 04 §2/§6/§7) y `research/29`.
