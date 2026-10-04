# 54 — Phase 25: Orchestrator execution loop (v25)

- **Estado:** **COMPLETA** (v25.0–v25.3).
- **Fecha:** 2026-10-03.
- **Decisión:** hacer **A antes que B** (research/53 §… y conversación 2026-10-03). B (Harbor)
  mide el *harness*; un harness que no enruta produce un artefacto del "Scaffold Effect"
  (KDD). A activa el routing/cascade/gate/cost-guard y hace que B mida el sistema **real**.

## 1. Diagnóstico (verificado en código 2026-10-03)

- `cli/commands/run.rs`: `atlas run` = **un solo pase** Coding→Validation→Repair y sale
  ("Phase 1 — no Execution Supervisor loop"). No pasa por el routing del orquestador.
- **Ningún caller de producción** de `Cascade::new`/`next_target`/`Router::route`/
  `CostGuard` (grep): son núcleos **puros y testeados, sin loop que los despierte**.
- `orchestrator/wire.rs` tiene las *shapes* (OpenAI/Anthropic/Gemini/Local) pero **no existe
  un cliente HTTP de provider** (`impl Provider`). `reqwest 0.12` ya es dependencia.
- Consecuencia: el gate de fiabilidad (Fase 24), el cost guard (G11) y el cascade **nunca se
  ejecutan**; y un benchmark externo mediría el tramo Phase-1, no el orquestado.

## 2. Diseño (scope acotado)

Una **misión → un plan → un paso → una llamada real** por `gate → router → cascade →
provider client → cost guard`, journaled, cancelable, con budgets. **No** swarm/paralelo,
**no** wire multi-provider completo todavía. OpenAI-compatible primero (el dominante).

### v25.0 — Provider client — **COMPLETA**
- `orchestrator/client.rs`: trait `ProviderClient` (AFIT/RPITIT, sin `dyn`) +
  `HttpProviderClient` (reqwest, `POST {api_base}/chat/completions`, bearer desde
  `Deployment::api_key_env`).
- Codecs **puros** testables: `build_chat_body`, `parse_chat_response`, `chat_endpoint`.
- `ChatMessage`/`ChatRequest`/`ChatResponse`/`Usage`/`ClientError`.
- Mock (`ScriptedClient`) en tests → el loop y el cascade podrán testearse **offline**.

### v25.1 — Call-with-cascade — **COMPLETA**
- `orchestrator/call.rs`: `call_with_cascade(client, config, primary_model_id, deployments,
  request, max_attempts)`. Hallazgo: `Cascade::next_target` **no** elige el primario (arranca
  en failover/escalado), así que el caller elige el primario y luego cede cada `FailureMode`
  al cascade (same-group → buckets) excluyendo los ya intentados.
- `mode_of(ClientError) -> FailureMode` (429/5xx→RateLimited; 401/403/404/408→BadConfig;
  400/413/422 con contexto→ContextWindowOverflow, con "content/policy"→ContentPolicyRefusal).
- 4 tests con mock (primario OK; 429→fallback a otro modelo; agotamiento; `mode_of`).

### v25.2 — Loop integration — **COMPLETA**
- `orchestrator/execute.rs`: `execute_steps(client, deployments, routing, mission_id,
  plan_id, steps, system_prompt, cfg, cancel)`. Por cada paso: rutea vía
  `call_with_cascade`, alimenta el supervisor (`ToolCall` → sube el `BudgetTally` y puede
  disparar `HaltSession`), respeta `cancel` (`AtomicBool`) y los `BudgetCaps`, y registra
  el `StepOutcome`. Devuelve `ExecuteReport` (nunca panickea).
- CLI **`atlas execute <mission_id> [--max-attempts N]`**: carga el Plan, aplana
  `Registry.deployments`, proyecta los `plan.steps` a `ExecuteStep` y corre el loop con
  `HttpProviderClient`.
- 4 tests con mock (todos OK; cancelación; budget cap; fallo de paso).
- **Diferido a v25.3:** escritura de `model_invocations` + eventos de bus + wiring de
  señal de cancelación + cost guard con precios (hoy `cost_usd = 0.0`).

### v25.3 — Observabilidad — **COMPLETA**
- **Coste real**: `cost_of(usage, price)` (input/output por 1M) + `ModelPrice`; el loop lo
  pasa al `ToolCall` → `BudgetCaps.max_cost_usd` deja de ser inerte.
- **Journal + bus**: el CLI `atlas execute` persiste una fila `model_invocations` por paso
  (modelo, deployment, tokens, coste, latencia, idempotency_key) y publica `AgentTokens`.
- **Cancelación por señal**: `tokio::signal::ctrl_c()` setea el `AtomicBool` compartido.
- 2 tests nuevos (coste desde usage+precio; `cost_of` con entradas ausentes).
- **Diferido:** card/endpoint HUD dedicado (el dato ya está en `model_invocations` y en el
  stdout del CLI).

## 3. No-goals / riesgos

- **No** cambiar el default de routing ni activar el gate de fiabilidad sin opt-in.
- **No** tocar la red en tests (mock); el HTTP real se cubre en un smoke manual.
- **No** wire Anthropic/Gemini en v25.0 (follow-up); el payload layer (`wire.rs`) ya existe.
- Riesgo: scope balloon → el RFC fija el corte (una llamada real, no swarm).

## 4. B-prep (adaptador Harbor)

- `tools/harbor_atlas/atlas_agent.py` ahora conduce el **pipeline real**
  (`atlas --profile harbor mission new --force → plan → execute`), de modo que el número
  externo mide el **harness orquestado** (routing + cascade + gate + coste), no el pase
  Phase-1.
- Falta la **corrida real**: requiere Harbor + Docker + creds de modelo + perfil `harbor`.
  La ingestión ya está lista (`atlas eval import jobs/<job-id>`).

## 5. Fuentes

- Conversación 2026-10-03 y `research/53` (A→B). `research/51` (EVAL), `research/29`
  (Phase 2 orchestrator), KDD 2026 (Scaffold Effect), `Atlas OS/04` (RFC 04 §2/§6).
