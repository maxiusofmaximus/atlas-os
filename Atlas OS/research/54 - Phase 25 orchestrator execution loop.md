# 54 — Phase 25: Orchestrator execution loop (v25)

- **Estado:** plan aprobado; v25.0 **COMPLETA** (v25.1–v25.3 pendientes).
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

### v25.1 — Call-with-cascade
- `call_with_cascade(client, candidates, request, cascade, retry, cooldown)`: intenta los
  deployments **gateados** en orden por `FailureMode`, devuelve respuesta o `Exhausted`.
  Testeado con el mock (éxito, 429→fallback, agotamiento).

### v25.2 — Loop integration
- `atlas execute <mission_id>` (o extender `run`): integra supervisor (`tick`) + routing +
  client + cost guard + budgets + journal/bus; un pase controlado, cancelable.

### v25.3 — Observabilidad
- Telemetría por run (modelo elegido, tokens, coste, fallbacks) → HUD.

## 3. No-goals / riesgos

- **No** cambiar el default de routing ni activar el gate de fiabilidad sin opt-in.
- **No** tocar la red en tests (mock); el HTTP real se cubre en un smoke manual.
- **No** wire Anthropic/Gemini en v25.0 (follow-up); el payload layer (`wire.rs`) ya existe.
- Riesgo: scope balloon → el RFC fija el corte (una llamada real, no swarm).

## 4. Fuentes

- Conversación 2026-10-03 y `research/53` (A→B). `research/51` (EVAL), `research/29`
  (Phase 2 orchestrator), KDD 2026 (Scaffold Effect), `Atlas OS/04` (RFC 04 §2/§6).
