# research/56 — Fase 33: Reliability gate en el coding loop

- **Fecha:** 2026-10-03
- **Estado:** **COMPLETA** (v33.0).
- **Contexto:** Fase 26 dejó el coding loop LLM (routed `Diff` → Validation → Repair). La
  compuerta de fiabilidad (Fase 24) existe pero es *opt-in* y **no toca el coding loop**: un
  `Diff` puede enrutarse a un modelo cuya tasa de acierto histórica es baja.

## 1. Investigación (por qué y qué evitar)

Routing de modelos en producción (rodeos documentados):

- **`Model Router Was Trained on Your Eval Set, Not Your Traffic`** — los fallos del router
  se **agrupan** en el long tail; el error no aparece como "error del router". Hay que
  instrumentar la decisión, no solo el resultado.
- **`The Cascade Router Reliability Trap`** — la latencia p95 sube; hay que medir
  kept/escalated por separado. Corolario para Atlas: registrar la ruta tomada.
- **`Multi-LLM routing: the failure modes nobody warns about`** — los peores fallos
  devuelven **HTTP 200** con cuerpo bien formado; instrumentar `cost-per-successful-task`,
  `fallback rate` y **drift** de calidad.
- **`OpenRouter in Production`** — *"no fijes una lista corta de providers con
  `allow_fallbacks:false`"*: pinchar proveedores sin fallback es un punto único de fallo;
  los fallos silenciosos (`content:null`) exigen validar la respuesta, no el status.
- **MLflow / RASER** — router basado en **eval**: puntuar cada proveedor contra una
  definición medible; **no** entrenar un clasificador con muestras pequeñas.

**Conclusiones que gobiernan Atlas:**

1. **No decidir con muestras pequeñas.** `ReliabilityGate.min_samples` (Fase 24) ya lo
   honra: con pocas samples, el modelo se deja "unknown" y `allow_unknown` lo permite.
2. **Fail-safe, jamás "0 candidatos".** Si el gate denegara todo, se usa el set original
   (el usuario ya lo tenía; no empeora). Ver `gate_refs`.
3. **Auditar la decisión.** Loggear `gate=on/off`, modelo elegido y denegados — el fallo
   invisible es el peor.

## 2. Decisión

Cablear la Fase 24 al coding loop (Fase 26): el gate filtra los modelos candidatos
**antes** de `call_diff_with_cascade` — y también durante el failover.

## 3. Sub-fase

- **v33.0 COMPLETA** — `orchestrator/call.rs::call_with_cascade_and_denied` (excluye
  `denied` del primario y de cada grupo de failover; el wrapper `call_with_cascade` pasa
  `&[]` → comportamiento intacto); `orchestrator/code.rs::call_diff_with_cascade_and_denied`;
  `orchestrator/execute.rs::execute_coding_step_denied`. CLI `atlas execute --coding`:
  `load_reliability_gate()` → `reliabilities_from_journal(&journal, &models, 200)` →
  `filter_deployments` → `denied` → `execute_coding_step_denied`; imprime
  `reliability_gate=on/off denied=[...]`. 1 test nuevo (`denied_primary_is_skipped_for_the_failover_model`).

## 4. Diferido (deliberado)

- `route_taken_json` con la lista de denegados por step: **F34** (observabilidad en
  `model_invocations`, hoy el `--coding` no persiste `model_invocations`).
- Un golden task dedicado: la cobertura ya existe vía `orchestrator.reliability_gate` +
  el test de ruta denegada; no se añade ruido.

## 5. Fuentes

- research/53 (Fase 24) y research/55 (Fase 26).
- Blogs 2026: tianpan.co (router eval/traffic, cascade reliability trap),
  willianpinho.com (multi-LLM failure modes), pinggy.io (OpenRouter pitfalls), mlflow.org
  (LLM routing stages), arxiv RASER (recoverability-aware routing).
- Harbor docs (agent adapter + `TrialResult`/`job/result.json`) — schema para F32.2.
