# 04 - Model Orchestrator

El motor que decide **qué cerebro** piensa cada paso. Es el pilar de la diferenciación frente a editores mono-modelo.

> No son mejores porque tengan un mejor prompt. Son mejores porque saben **cuándo cambiar de cerebro**.

---

> **Estado de implementación.** Phase 1 materializa sólo el hot-swap RFC 27 §B + reset-window management (RFC 28 §H items 1-13, "SpendLimitError" / "RetryPolicy" / "parse_omnroute" / `handle_spend_limit_error()`). Las secciones §1 (registry), §2 (selección routing), §3 (votación/fusión/debate), §5 (sub-modos local/free/mixto), §6 (presupuesto enforcement), §7 (skill-aware routing), §8 (feedback loop telemetría) son **PENDING Phase 2** — plan refinado con 6 sub-fases atómicas (2.0 Foundation → 2.0.5 Normalization → 2.1 Routing → 2.2 Aggregation → 2.3 Auto-routing → 2.4 Feedback) en `OpenCode OS/research/29 - Phase 2 model orchestrator.md`. Evidencia primaria: 11 papers arxiv cross-verified + LiteLLM + OpenRouter + Aider + async-openai + RouteLLM. Auditoría Round 3 detectó 9 gaps críticos (G1 prompt caching, G2 token counter pre-flight, G3 sampling params, G5 cooldown per-provider, G8 tool-call normalization, G11 cost guard pre-aggregation, G12 idempotency, G17 `Retry-After` headers, G18 back-pressure semáforo) que se materializan en sub-fase 2.0.5. Literales Rust existentes: `orchestrator::{mod, error, parse_error, retry}` + `journal::{model_swaps, model_resets}` + `BusEventKind::{ModelSwapped, SpendLimitObserved}`.

---

## 1. Registro de modelos `[IMPLEMENTADO Phase 2 — sub-fase 2.0 Foundation]`

El orquestador mantiene un registry de modelos. Cada entrada:

```json
{
  "id": "claude-sonnet-4.5",
  "provider": "anthropic",
  "protocol": "anthropic",            // gemini | anthropic | openai | bedrock | ollama | custom
  "kind": "remote",
  "tier": "frontier",
  "context_window": 200000,
  "capabilities": ["text", "vision", "function_calling", "image_generation"],
  "strengths": ["reasoning", "architecture", "long_context"],
  "weaknesses": ["cost", "rate_limits"],
  "cost_per_1k_tokens_in": 0.003,
  "cost_per_1k_tokens_out": 0.015,
  "latency_ms_p50": 900,
  "available": true,
  "credentials_env": "ANTHROPIC_API_KEY",
  "api_keys": ["ANTHROPIC_API_KEY", "ANTHROPIC_API_KEY_2"],   // multi-key pooling
  "runtime_options": {                                         // agent-defined runtime knobs
    "thought_level": ["minimal", "default", "extended"],
    "modes": ["code", "plan", "review"]
  },
  "capability_tags": ["reasoning", "long_context", "vision"]
}
```

> **Lecciones integradas de AionUI** (`iOfficeAI/AionUi`, Apache-2.0, 29.3k★): AionUi no rutea automáticamente por complejidad/costo/latencia — delega al usuario la selección manual de modelo por conversación o por *assistant preset*. OpenCode OS convierte ese routing en un motor automático, que es **la invención diferencial #1**. Hemos incorporado de AionUi: el *capability tag system* (`text|vision|function_calling|image_generation`), el *protocol auto-detection*, el *multi-key rotation con blacklist de 90s* (ver §3) y las *runtime options* (thought level / mode) que algunos backends exponen.

### Proveedores soportados por defecto
| Proveedor | Tier | Tipo |
|---|---|---|
| Anthropic (Claude) | Frontier / de pago | Remoto |
| OpenAI / o3 | Frontier / de pago | Remoto |
| Google AI (Gemini) | gratuito + de pago | Remoto |
| DeepSeek | pagado | Remoto |
| Mistral | gratuito + pagado | Remoto |
| OpenRouter (agregador) | página de cada modelo | Agregador |
| GitHub Models | gratuito | Remoto |
| Nvidia NIM | gratuito | Remoto |
| Cerebras | gratuito / ultra rápido | Remoto |
| Groq | gratuito / ultra rápido | Remoto |
| Sambanova | gratuito / ultra rápido | Remoto |
| Cloudflare Workers AI | gratuito | Edge |
| HuggingFace Inference | gratuito | Remoto |
| LM Studio / Ollama / llama-server | Local (7B–70B) | Local |
| Kimi K2.7 (en swarm) | Experimental | Remoto |

### Registry extensible
El usuario puede añadir proveedores de pago o cualquier otro endpoint compatible.

## 2. Política de selección `[PENDING Phase 2 — sub-fase 2.1 Routing Policy]`

El orquestador analiza la tarea y ejecuta:

```
Analiza la tarea
   ↓
Clasifica (dominio + riesgo + tamaño)
   ↓
Candidatos compatibles
   ↓
Filtra por restricciones del usuario
   (modo local-only / free-only / mixto)
   ↓
Estima cost / tiempo / confidence esperado
   ↓
Escoge el mejor cerebro
   ↓
Verifica resultado
   ↓
Si falla → escoge otro modelo
   ↓
Fusiona respuestas si procede
   ↓
Devuelve una sola decisión
```

### Mapa de afinidad (por defecto, configurable)

| Tipo de tarea | Modelo sugerido |
|---|---|
| Arquitectura / diseño | Claude (long context + reasoning) |
| Refactor gigante | Gemini (gran ventana) |
| Backend TypeScript | GPT |
| Bug difícil | o3 / DeepSeek (reasoning) |
| Búsqueda masiva | DeepSeek |
| Razonamiento matemático / lógica | DeepSeek |
| Comparación / votación | enjambre + votación |
| Comparación semántica | Qwen |
| Generación rápida y barata | Cerebras / Groq / Sambanova |
| Offline / privacidad | Local 7B–70B en GPU del usuario |
| Documentación / resumen | Local gratuito o Mistral |

## 3. Fusión y votación `[PENDING Phase 2 — sub-fase 2.2 Aggregation]`

Para decisiones críticas el orquestador puede:

1. Llamar a **N modelos** en paralelo (por ejemplo Claude, Gemini, GPT, DeepSeek).
2. Recoger sus respuestas estructuradas.
3. Aplicar uno de:
   - **Votación mayoritaria** con peso por Confidence.
   - **Fusión** (mezcla de outputs si son compatibles).
   - **Debate** (un modelo critica al otro, iteran, finalmente emiten).
4. Devolver **una** decisión al caller.

### Ejemplo
```
Decisión: ¿Strategy, Factory o Context para el patrón de estado?
  ↓
   Claude      → Strategy (Conf 0.91)
   Gemini      → Strategy (Conf 0.88)
   GPT         → Factory  (Conf 0.62)
   DeepSeek    → Strategy (Conf 0.90)
  ↓
Votación ponderada → Strategy
Confidence fuse → 0.89
```

## 4. Fail-over `[PARCIAL Phase 1 — Phase 2 sub-fase 2.1 completa cascade 3-buckets]`

> Phase 1: existen `SpendLimitError`/`ResetKind` (RFC 28 §H items 2,5,6) con parse OmniRoute envelope + `RetryPolicy::decide` con backoff exponencial + jitter ±25% + bail-out 60s threshold. `handle_spend_limit_error()` en `orchestrator/mod.rs` persiste `model_resets` (M19) y dispara Toast `kind='model_ready'` cuando reset cumple. **No** hay cascade cross-deployment ni múltiples buckets fallback aún — eso aterriza en sub-fase 2.1 (3 buckets LiteLLM: `fallbacks` + `context_window_fallbacks` + `content_policy_fallbacks`).


- Si un modelo devuelve timeout / rate-limit / hallucination detectada, se marca `available: false` temporalmente.
- Se reintenta con el siguiente candidato.
- Tras 3 fallos consecutivos se escapa hacia arriba (usuario o Planning Engine) para replanificar.

## 5. Restricciones de modo `[PENDING Phase 2 — sub-fase 2.1 routing filter]`

El orquestador **nunca** debe usar un modelo fuera del sub-modo activo:
- sub-modo `local-only` ⇒ filtra `kind == local`.
- sub-modo `free-only` ⇒ filtra `tier in {free, free_tier}`.
- sub-modo `mixto` ⇒ acepta cualquier `tier`, respetando el presupuesto definido.

## 6. Presupuesto y límites `[PENDING Phase 2 — sub-fase 2.0.5 back-pressure semáforo + 2.4 cost guard feedback]`

El usuario define:
- `max_tokens_per_minute`
- `max_cost_per_hour`
- `max_concurrent_remote_calls`
- `max_local_concurrency` (en función de RAM / VRAM libres)

El orquestador aplica back-pressure y cola si se superan.

## 7. Routing de inferencia por skill `[PENDING Phase 2 — sub-fase 2.3 auto-routing + MCP tool-capability-aware]`

El Skill Graph almacena `compatible_models` y `recommended_models`. El orquestador **prioriza** los recomendados al disparar una skill. Las Skills de diseño favorecen Gemini o Claude; las skills de tests priorizan modelos rápidos; las de lógica priorizan razonadores.

## 8. Telemetría `[PARCIAL Phase 1 — Phase 2 sub-fase 2.4 completa el loop affinity]`

> Phase 1 persiste `was_correct` en M7 `pattern_runs` (alcance: patrón/regla de repair, no invocación del modelo). Falta el reader que una `model_invocations` (M21 sub-fase 2.1) con `pattern_runs.was_correct` → alimenta affinity del `ModelRegistry`. Ese loop completa en sub-fase 2.4 Feedback Loop.

Cada invocación registra en el Journal:
- modelo, proveedor, latencia, tokens, coste,
- prompt template usada, skill disparadora,
- Confidence emitido, validación posterior,
- `was_correct` (booleano determinado por Learning Engine).

Esto retroalimenta la afinidad del orquestador con el tiempo → el sistema **aprende** qué modelo se porta mejor para cada clase de tarea en cada proyecto.

## 9. Frontends del orquestador (RFC 28 §B item 6)

El Orchestrator es agnóstico del transporte de UI. Hasta Phase 1 disponía de dos frontends:

1. **CLI pura** (`opencode` headless binary, RFC 25 §3.9 / RFC 08) — `mission new`, `plan`, `run`, `resume`, `fork`, `steer`, `swap_model`, `exec`, etc.
2. **HUD Mission Control** (WebView via Tauri, RFC 24) — graph cards, fork-tree, audit timeline, autoresearch card (RFC 28 §A), mission graph (RFC 28 §C), WebSockets over axum.

A partir de **RFC 28 §B** (Phase 1.5d) hay un tercer frontend: el **ACP server embebido en `opencode`** que Microsoft Intelligent Terminal auto-detecta vía `WT_COM_CLSID`. Esto NO convierte al Orchestrator en un proceso externo — el ACP server reusa la misma `crate::orchestrator` que los otros dos frontends, simplemente con un transporte JSON-RPC sobre stdio y un envelope de input distinto (`PromptRequest` en vez de `commands::dispatch`). El contrato es:

- `initialize` → `build_initialize_response` (sin `mcpCapabilities` en Phase 1.5d; ver RFC 28 §B).
- `session/new` + `available_commands_update` → los 6 slash commands del catalogue de `acp/commands.rs`.
- `session/prompt` con comando delegado → `acp::delegate::parse_delegate` enrutas a la lógica que el CLI ya conduce (`opencode exec step`, `/opencode fix` → Repair engine RFC 15, `/opencode restart` → `session/close` + `session/new`).
- `session/set_mode` → override de `SupervisorState` (ver RFC 19 §6.1.2). El Orchestrator aplica el override **antes** del siguiente `tick()` (no interrumpe un step en curso) y persiste en Journal M6 como audit.
- `$/cancel_request` → notification ack no-op (PIa 1.5d host loop no-oped; Phase 2 lo convertirá a `BusEvent::MissionSteered`).

Frontends **no tienen voto** sobre qué modelo usar — el Orchestrator wählt según el Skill Graph (§7), presupuesto (§6) y last `was_correct` (§8). Un frontend sólo puede sugerir via `profile swap <model>` (CLI) o `swap_model` (HUD) o `session/set_mode` (ACP, indirectamente via skill-compatible model del nuevo state). Esta separación mantiene el Orchestrator neutral sobre el transporte, exactamente como P3 (`cualquier modelo puede entrar`).

Single-binary safety (RFC 25 §11): el ACP frontend no añade subprocess externo; reusa tokio runtime + `agent-client-protocol::Stdio` builtin transport. Spawning `wtcli` (Channel 2 worker) es opt-in y gated `acp-server`; en macOS/Linux tanto `run_server` como `spawn_listener` son no-op.

---

## Apéndice — Plan refinado Phase 2 (sub-fases atómicas)

**Source:** `OpenCode OS/research/29 - Phase 2 model orchestrator.md`. Evidencia primaria: 11 papers arxiv cross-verified + LiteLLM/OpenRouter/Aider/async-openai/RouteLLM docs.

| Sub-fase | Título | Migración | Cobertura | Estado |
|---|---|---|---|---|
| 2.0 | Foundation (Registry + Tri-model) | M20 (`models`/`deployments`/`model_aliases`/`model_groups`) | §1 (Provider enum, ModelRegistry storage SQLite+JSON seed `ArcSwap<Registry>`, Aider tri-model en `Profile`) | **✅ IMPLEMENTADO** — `orchestrator::{provider, registry}` con `enum Provider` (20 builtin + `Custom(Arc<dyn Config>)`), `ProviderWire` serde-safe (rename explícito para PascalCase cortos), `ModelDescriptor`/`Deployment`/`Capability`/`Tier`, `Registry::{from_seed, from_bundled_seed, resolve, get, descriptors_for_provider, filter_by_resource_mode, filter_by_capability}` con seed JSON bundled (`assets/model_prices_and_context_window.json`, 18 modelos subset LiteLLM MIT), M20 migration idempotente (4 tablas + índices), `Profile` extendido con `main_model_id`/`architect_model_id`/`editor_model_id`/`weak_model_id` + `resource_mode` + `effective_{architect,editor,weak}_model()` fallback a `main_model_id`. 24 tests (13 `provider`, 11 `registry`). `parse()` en vez de `from_str()` para evitar colisión con `std::str::FromStr`. `Provider` sin `Eq/Hash` (Custom varía); cooldown maps usarán `ProviderWire`. |
| 2.0.5 | Provider Normalization Layer | — | Gaps críticos G1 prompt caching, G2 token pre-flight, G5 cooldown per-provider, G8 ToolCall enum cross-provider normalize/deserialize, G17 `Retry-After` header, G18 back-pressure `Arc<Semaphore>` | PENDING |
| 2.1 | Routing Policy | M21 (`model_invocations` con sampling_params/seed/cache_read) | §2 (RoutingStrategy enum 6 LiteLLM), §4 completa (3 buckets cascade fallback con `max_fallbacks=5` + exclusion set), §5 (sub-modos filter), §6 parcial (back-pressure), G12 idempotency `RequestFrame`, HUD WS DataParts reconciliation | PENDING |
| 2.2 | Aggregation (opt-in HighStakes) | M22 (`reflection_episodes`) + `council_votes` | §3 (`AggregationMode { Single, MajorityVote AgentForest N∈{1,3,5,9} stop-early 2/3, MoA 3×3 cost guard G11, Council 1-round, SelfRefine cap 2, Reflexion multi-model cap 3 anti-doom-loop }`). Papers: 2402.05120, 2406.04692, 2305.14325, 2303.11366, 2303.17651, 2402.03620 | PENDING |
| 2.3 | Auto-routing Classifier + MCP-aware | — | §7 (TaskTypeClassifier opt-in logistic regression + fastembed-rs BGE-small + linfa MLP, tag pre-filter hot-path `capability_tags` AND `tool_capabilities` MCP G19, Router selector via `model` field RouteLLM `router-mf-0.116`, threshold defaults `coding=0.116`) | PENDING |
| 2.4 | Feedback Loop + `mf` experimental | M20 affinity cache (in-memory) | §8 completa (reader `model_invocations` GROUP BY `(task_type, model_id)` → `AffinityRow` → `ArcSwap::store`, `RoutingStrategy::Mf { threshold }` A/B testing vs classifier) | PENDING |

**KPIs Phase 2:** coste LLM por mission ≤ Phase 1 × 0.6 (RouteLLM evidence >2× savings); routing overhead < 5ms p95 (ArcSwap lock-free); re-ingresos model crash 100%; feedback loop `was_correct` < 30s tras verdict usuario.

**Sub-pases deferrables (Future Work anotados, no en Phase 2):** G4 streaming partial aggregation, G6 circuit-breaker half-open state, G7 provider-level health HUD, G9 vision/multimodal routing capability mask, G10 Ollama `supports_tools` CSV, G13 HUD approval card aggregation, G15 `opencode calibrate-classifier` CLI, G16 offline eval harness `opencode eval`, G19 full MCP tool-capability-aware routing (en 2.3 parcial), G20 response cache separado de prompt cache provider.

