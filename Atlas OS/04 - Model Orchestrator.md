# 04 - Model Orchestrator

El motor que decide **qué cerebro** piensa cada paso. Es el pilar de la diferenciación frente a editores mono-modelo.

> No son mejores porque tengan un mejor prompt. Son mejores porque saben **cuándo cambiar de cerebro**.

---

> **Estado de implementación.** Phase 1 materializa sólo el hot-swap RFC 27 §B + reset-window management (RFC 28 §H items 1-13, "SpendLimitError" / "RetryPolicy" / "parse_omnroute" / `handle_spend_limit_error()`). Las secciones §1 (registry), §2 (selección routing), §3 (votación/fusión/debate), §5 (sub-modos local/free/mixto), §6 (presupuesto enforcement), §7 (skill-aware + auto-routing classifier), §8 (feedback loop telemetría) son **PENDING Phase 2 — §1-§8 IMPLEMENTADO (Phase 2 sub-fases 2.0→2.4 completas)** — plan refinado con 6 sub-fases atómicas (2.0 Foundation → 2.0.5 Normalization → 2.1 Routing → 2.2 Aggregation → 2.3 Auto-routing → 2.4 Feedback) en `Atlas OS/research/29 - Phase 2 model orchestrator.md`. Evidencia primaria: 11 papers arxiv cross-verified + LiteLLM + OpenRouter + Aider + async-openai + RouteLLM + Context7 (linfa/fastembed-rs API verification). Auditoría Round 3 detectó 9 gaps críticos (G1 prompt caching, G2 token counter pre-flight, G3 sampling params, G5 cooldown per-provider, G8 tool-call normalization, G11 cost guard pre-aggregation, G12 idempotency, G17 `Retry-After` headers, G18 back-pressure semáforo) que se materializan en sub-fase 2.0.5. Literales Rust existentes: `orchestrator::{mod, error, parse_error, retry}` + `journal::{model_swaps, model_resets}` + `BusEventKind::{ModelSwapped, SpendLimitObserved}`.

---

## 1. Registro de modelos `[IMPLEMENTADO Phase 2 — sub-fases 2.0 Foundation ✅ + 2.0.5 Provider Normalization ✅]`

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

> **Lecciones integradas de AionUI** (`iOfficeAI/AionUi`, Apache-2.0, 29.3k★): AionUi no rutea automáticamente por complejidad/costo/latencia — delega al usuario la selección manual de modelo por conversación o por *assistant preset*. Atlas OS convierte ese routing en un motor automático, que es **la invención diferencial #1**. Hemos incorporado de AionUi: el *capability tag system* (`text|vision|function_calling|image_generation`), el *protocol auto-detection*, el *multi-key rotation con blacklist de 90s* (ver §3) y las *runtime options* (thought level / mode) que algunos backends exponen.

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

## 2. Política de selección `[IMPLEMENTADO Phase 2 — sub-fase 2.1 Routing Policy ✅]`

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

**Implementación sub-fase 2.1.** La capa de routing vive en `orchestrator/routing.rs` (~20 tests) con `enum RoutingStrategy { SimpleShuffle, LatencyBased { ttl_secs, buffer_ms }, UsageBasedV2, LeastBusy, CostBased, Hybrid { branches, default }, Custom(Arc<dyn Router>) }` — las cinco estrategias de producción LiteLLM (MIT, BerriAI) más `Hybrid` para estrategias condicionales (p.ej. HighStakes → LeastBusy, bulk → CostBased) y `Custom` para routers in-process (`#[serde(skip)]` por la `Arc<dyn>`). El trait `Router::route(ctx) -> RouteDecision` y el `RouteContext<'a>` (snapshot pura: healthy deployments, in-flight, p50 latency, error rate, tokens/min, budget, prompt tokens, capabilities, pre-cost, has_tool_calls, high_stakes, exclusion set) hacen la selección deterministamente testeable sin runtime. `Condition` enum (`TokensAbove`, `Requires`, `CostAbove`, `HasToolCalls`, `HighStakes`) drive el `Hybrid` branch matching.

`RoutingConfig` (serializable, en `Profile.routing_config`) porta: `strategy: RoutingStrategy`, `fallback: FallbackMap` (3 buckets LiteLLM: `fallbacks`/`context_window_fallbacks`/`content_policy_fallbacks`), `default_fallbacks: Vec<String>`, `max_fallbacks: u8=5` (cap), `default_cooldown_secs=60`, `allowed_fails=3`, `fails_window_secs=60`.

`orchestrator/cascade.rs` (~13 tests) implementa la escalation: `Cascade::next_target(FailureMode, healthy_for)` hace (1) weighted failover en same-group para `RateLimited` (exclusion set `HashSet<String>` acumula IDs ya probados), (2) escalation al bucket correspondiente (`FallbackBucket::Generic|ContextWindow|ContentPolicy`), (3) `default_fallbacks` catch-all, (4) cap en `max_fallbacks` (default 5). `FailureMode` discrimina 401/404/408/network vs 429 vs context overflow vs content-policy refusal. `CascadeStep::{TryNext { deployment, model_id, bucket, attempt_index }, Exhausted { reason: NoFallbackConfigured|MaxFallbacksReached|AllFallbacksUnhealthy, attempts }}`.

`orchestrator/idempotency.rs` (7 tests, G12): `RequestFrame { idempotency_key: Uuid v4, executed_tool_calls: Vec<String>, tool_calls_complete: bool }` — `can_cascade()` bloquea cascade cuando hay tool calls pending (preserva consistencia del dispatcher de tools). `mark_executed()` dedupe--append. `filter_unexecuted(&[OpShape])` parcha prompts del fallback.

`orchestrator/cost_guard.rs` (8 tests, G11): trait `AggregationPolicy { pre_cost_estimate(ctx) -> f64, aggregate_cost_breakdown(ctx) }` shape definido aquí; `LinearCostGuard` + `NoAggregation` defaults. Implementación concreta en sub-fase 2.2.

`orchestrator/data_parts.rs` (11 tests): `DataPartBuffer` con `DataPart { id, payload, is_transient, seq }` — patrón DataParts Vercel AI SDK (MIT): mismo id actualiza in-place; `persistent_only()` descarta partes transient tras cascade (preserva durables: tool-call deltas, plan diffs, commit hashes).

M21 migration `journal/schema.rs`: `model_invocations` table (id PK, mission_id FK, model_id FK weak, deployment_id, provider, idempotency_key, started_at, finished_at, latency_ms, tokens_in, tokens_out, cache_read_input_tokens, cost_usd, seed, temperature, sampling_params_json, route_taken_json, was_correct NULL, error_kind, error_message) + 3 indexes. 134 journal tests pasan.

`Profile` extendido con `routing_config: RoutingConfig` (default SimpleShuffle + buckets vacíos). `Profile` pierde `Eq` (RoutingConfig contiene `f64` en `Condition::CostAbove`), retiene `PartialEq` — ningún sitio usa `Eq` sobre `Profile`. `impl Default for Profile` añadido. 6 profile tests adaptados con `..Default::default()`.

39 tests nuevos. 65 tests combined (routing + cascade + idempotency + cost_guard + data_parts + profiles) pasan. 542 tests totales verde.

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

## 3. Fusión y votación `[IMPLEMENTADO Phase 2 — sub-fase 2.2 Aggregation ✅]`

> **Sub-fase 2.2 ✅:** `orchestrator/aggregation/` materializa los 6 modos
> (Single, MajorityVote, MoA, Council, Reflexion, SelfRefine, SelfDiscover)
> como `enum AggregationMode` + `trait Aggregator` (`#[async_trait]`). Cada
> modo tiene su struct (`MajorityVoteAggregator`, `MoAAggregator`,
> `CouncilAggregator`, `ReflexionAggregator`, `SelfRefineAggregator`,
> `SelfDiscoverAggregator`) y su `AggregationPolicy` cost guard en
> `orchestrator/cost_guard.rs` (`MajorityVoteCostGuard`, `MoACostGuard`,
> `CouncilCostGuard`, `ReflexionCostGuard`, `SelfRefineCostGuard`,
> `SelfDiscoverCostGuard`). `aggregator_for(&mode)` despacha con unit
> struct constructors. M22 migration añade `reflection_episodes` +
> `council_votes` al schema. `Profile.aggregation: AggregationMode` field
> (default = `Single`). Papers cross-verified: 2402.05120 (AgentForest
> majority vote), 2406.04692 (MoA), 2305.14325 (Council debate),
> 2303.11366 (Reflexion), 2303.17651 (Self-Refine), 2402.03620
> (Self-Discover).

Para decisiones críticas el orquestador puede:

1. Llamar a **N modelos** en paralelo (por ejemplo Claude, Gemini, GPT, DeepSeek).
2. Recoger sus respuestas estructuradas.
3. Aplicar uno de 6 modos (seleccionado por `Profile.aggregation`):
   - **`Single`** (default, pass-through — sin aggregation, el orquestador
     enruta una inferencia y devuelve la respuesta intacta).
   - **`MajorityVote { n_samples, stop_early_threshold }`** (Agent Forest
     arxiv 2402.05120) — N paralelo con stop-early tras cada batch de 3
     votos si `agreement ≥ 2/3` Y `shannon_entropy < 0.5`. `n_samples`
     difficulty-aware (1 easy, 3 medium, 5/9 hard). Canonicaliza
     respuestas antes de votar (strip whitespace, lowercase, trim
     trailing punctuation).
   - **`MoA { layers: Vec<Vec<ModelId>> }`** (arxiv 2406.04692) — layered
     architecture 3×3 por defecto. Cada capa-N consume todos los
     outputs de capa-(N-1) como auxiliary input. Restringido a
     `ExecutionMode::HighStakes` por cost guard G11 (`MoACostGuard`
     escala cuadráticamente con `layers.len()`).
   - **`Council { debaters, rounds }`** (arxiv 2305.14325) — 2-3 debaters
     + 1 round default. Schema SQLite `council_votes` (M22). Un
     coordinador sintetiza la fused response.
   - **`Reflexion { memory_buffer_size }`** (arxiv 2303.11366) —
     multi-model: executor caro + reflexor barato. M22 migration
     `reflection_episodes`. Anti-doom-loop: `detect_doom_loop()` aborta
     la mission si `failure_signal` idéntico aparece en 2 episodios
     consecutivos. **Contribución Atlas OS:** paper original usa el
     mismo modelo para executor y reflexor; nosotros separamos roles y
     usamos un reflexor ~4× más barato (anotado en RFC 22 §Research
     Findings).
   - **`SelfRefine { max_iterations, stop_condition }`** (arxiv
     2303.17651) — single-LLM generator→feedback→refiner. Cap 2 iter
     por defecto; aborta si `line_delta() < threshold` (stall
     detection). `StopCondition::{Converged,MaxIterations,Stalled}`.
   - **`SelfDiscover { cache_ttl_secs }`** (arxiv 2402.03620) — planning
     engine pre-decode selecciona modules de razonamiento y genera JSON
     skeleton reuseado. Cachea por `SHA-256(url + prompt)` con TTL
     configurable. Feature-gated `fastembed` para prompt embeddings.
4. Devolver **una** `FusedResponse` al caller.

### Narrativa "MoA as default" (RFC 29 §3.E)

> Genspark (Super Agent; blog del CTO Kay, "Less Control, More Tools") vende
> multi-model como default para todo. Atlas OS mantiene la posición
> competitiva opuesta: **`MoA` es "the Genspark default"; `Single` es "our
> cost-aware default"** — habilitar MoA global rompe el budget (G11 es
> crítico, ver RFC 22 §10). Un `AggregationMode::Auto` experimental que
> decide por task difficulty (usando el classifier de sub-fase 2.3 + la
> affinity de sub-fase 2.4) queda anotado como Future Work 2.5+; el cost
> guard (G11) permanece como invariant en cualquier modo.

### Cost guard pre-aggregation (G11)

Antes de despachar aggregation, el orquestador consulta
`AggregationPolicy::pre_cost_estimate(&ctx) -> f64`. Si el estimado
excede `profile.budget_per_turn`, el orquestador **fallback a `Single`**
y loguea `BusEventKind::AggregationCostBudgetExceeded`. Cada modo tiene
su cost guard específico en `orchestrator/cost_guard.rs`:

| Modo | CostGuard | Fórmula |
|------|-----------|---------|
| Single | `NoAggregation` | 0.0 siempre |
| MajorityVote | `MajorityVoteCostGuard` | `parallel_samples × tokens × blended / 1M` |
| MoA | `MoACostGuard` | `parallel_samples × layers × tokens × blended / 1M` |
| Council | `CouncilCostGuard` | `debater_cost + coordinator_cost` (200 tok/synth) |
| Reflexion | `ReflexionCostGuard` | `executor_cost + reflexor_cost` (rate × 1/4) |
| SelfRefine | `SelfRefineCostGuard` | `iterations × tokens × blended / 1M` |
| SelfDiscover | `SelfDiscoverCostGuard` | `calls × tokens × blended / 1M` (cache miss = 2 calls) |

### Ejemplo
```
Decisión: ¿Strategy, Factory o Context para el patrón de estado?
  ↓
   Claude      → Strategy (Conf 0.91)
   Gemini      → Strategy (Conf 0.88)
   GPT         → Factory  (Conf 0.62)
   DeepSeek    → Strategy (Conf 0.90)
  ↓
Votación mayoritaria (MajorityVote, n=4, stop_early_threshold=0.66) → Strategy
agreement_ratio = 0.75 ≥ 0.66, shannon_entropy ≈ 0.56
Stop-early: batch 1/1 (3 votos Strategy) → fuse → Strategy
Confidence fuse → 0.89
```

## 4. Fail-over `[IMPLEMENTADO Phase 2 — sub-fase 2.1 ✅ completa cascade 3-buckets]`

> Phase 1: existen `SpendLimitError`/`ResetKind` (RFC 28 §H items 2,5,6) con parse OmniRoute envelope + `RetryPolicy::decide` con backoff exponencial + jitter ±25% + bail-out 60s threshold. `handle_spend_limit_error()` en `orchestrator/mod.rs` persiste `model_resets` (M19) y dispara Toast `kind='model_ready'` cuando reset cumple.
>
> **Phase 2 sub-fase 2.1 ✅:** `Cascade::next_target(FailureMode, healthy_for)` en `orchestrator/cascade.rs` materializa el cascade completo con los 3 buckets LiteLLM + `default_fallbacks` + `max_fallbacks=5` cap + weighted failover en same-group + exclusion set `HashSet<String>` acumulativo. `FailureMode` discrimina 401/404/408/network vs 429 vs context overflow vs content-policy refusal. `ExhaustionReason::{NoFallbackConfigured|MaxFallbacksReached|AllFallbacksUnhealthy}` surface el HUD card paused-mission.

- Si un modelo devuelve timeout / rate-limit / hallucination detectada, se marca `available: false` temporalmente.
- Se reintenta con el siguiente candidato.
- Tras 3 fallos consecutivos se escapa hacia arriba (usuario o Planning Engine) para replanificar.

## 5. Restricciones de modo `[PARCIAL — sub-fase 2.0 ResourceMode enum ✅ + sub-fase 2.1 routing filter ✅]`

El orquestador **nunca** debe usar un modelo fuera del sub-modo activo:
- sub-modo `local-only` ⇒ filtra `kind == local`.
- sub-modo `free-only` ⇒ filtra `tier in {free, free_tier}`.
- sub-modo `mixto` ⇒ acepta cualquier `tier`, respetando el presupuesto definido.

## 6. Presupuesto y límites `[Phase 2 — sub-fase 2.0.5 back-pressure semáforo ✅ IMPLEMENTADO + sub-fase 2.1 cost-guard trait shape ✅ + 2.4 cost guard feedback ✅ IMPLEMENTADO (AggregationCostContext::from_journal)]`

> Cost guard pre-aggregation (G11): trait `AggregationPolicy::pre_cost_estimate(ctx) -> f64` shape definido en `orchestrator/cost_guard.rs` (sub-fase 2.1, 8 tests) con defaults `NoAggregation` (0.0 siempre) y `LinearCostGuard` (`parallel_samples × rounds × tokens_per_sample × blended_cost_per_1m / 1M`). **Sub-fase 2.2 ✅:** implementación concreta de los 6 mode-specific cost guards (`MajorityVoteCostGuard`, `MoACostGuard`, `CouncilCostGuard`, `ReflexionCostGuard`, `SelfRefineCostGuard`, `SelfDiscoverCostGuard`) — 6 tests nuevos en `cost_guard.rs` (14 total).

El usuario define:
- `max_tokens_per_minute`
- `max_cost_per_hour`
- `max_concurrent_remote_calls`
- `max_local_concurrency` (en función de RAM / VRAM libres)

El orquestador aplica back-pressure y cola si se superan.

## 7. Routing de inferencia por skill `[IMPLEMENTADO Phase 2 — sub-fase 2.3 auto-routing + MCP tool-capability-aware ✅]`

El orquestador clasifica cada prompt en uno de 12 `TaskType` concretos (`coding`/`test`/`refactor`/`fix`/`build`/`plan`/`review`/`explain`/`translate`/`tidy`/`exec`/`chat`) más `Unknown` como fallback (RFC 22 §7 AN-2.3, Phase 2.3).

Tres backends detrás del trait `TaskTypeClassifier` (`#[async_trait]`) — cuatro desde Phase 9 sub-fase 9.1 (M33):

- **`LexicalClassifier`** (default): pure-Rust regex-counts sobre el prompt. Casí cero deps, ningún feature flag, ningún weights-file. Corre en todo profile con `Profile.auto_router.enabled = true`. Feature vectors de 12 elementos (uno por bucket) alimentan también `LogisticRegressionClassifier`.
- **`LogisticRegressionClassifier`** (opt-in): multi-class one-vs-rest logistic regression scratch-built — **no se usa `linfa`**, ver §7.1 abajo. Carga pesos desde `AutoRouterConfig.weights_path` (JSON shape `[[f64;12];12]` para `weights`, `[f64;12]` para `intercept`, `[String;12]` para `classes`). Cuando el archivo falta o está malformado, `classifier_for()` cae silenciosamente al `LexicalClassifier`.
- **`EmbeddingClassifier`** (`#[cfg(feature = "fastembed")]`, opt-in): fastembed-rs `TextEmbedding::try_new(Default::default())` carga `BGE-small-en-v1.5` (384 dim, ONNX, ~50 MB en `.fastembed_cache/`). Cosine-similarity contra 12 vectores prototipo pre-calibrados (formato JSON `[[f64;384];12]`). Cuando `fastembed` feature-off, cae al `LexicalClassifier`.
- **`LayaClassifier`** (Phase 9 sub-fase 9.1, M33 — el "System One" real, RFC 35 §7.1): `laya = "0.1.1"` (ModernBERT-large + RL decision head). Audit RFC 25 §11 **DIFERIDO** (RFC 22 §7 AN-9.1: `rand 0.8` vs `0.9`, `tokenizers` no en deps, `axum 0.7` vs `0.8`, weights runtime, crate de 5 días) → std-only MVP sin crates nuevas: inferencia determinista lexical-delegada tagueada `ClassifierKind::Laya`, feature `laya` vacío default-off. `LayaClassifier::load(path)` valida el model dir; si falta, `classifier_for()` cae al `LexicalClassifier` (mismo contrato que logreg). La inferencia candle real entra detrás del mismo gate sin cambiar callers. Follow-ups: wiring compaction (5.3) + tool-result judging (winnow).

Hot-path contract (research/29 line 242 "tag pre-filter hot-path", G19):

1. **Pre-filter**: `McpToolFilter::pre_filter(registry, &required_capabilities, &required_tool_names, catalog)` descarta deployments antes de invocar el classifier. Filtra por `Capability` (`Vision`, `ToolUse`, `FunctionCalling`, …) Y los tool-names disponibles en los MCP servers del workspace (vía trait `McpServerCatalog`). Cuando no hay servers registrados se usa `NoMcpCatalog` default — el pre-filter se reduce al intersection de `capability_tags`. Phase 2.5+ puede inyectar un catalog backed por un MCP server registry real (cuando `rmcp` se adopte en single-binary, ver RFC 07 §7).
2. **Classify**: `classifier.classify(ctx)` retorna `TaskVerdict { task_type, confidence, backend }`. La confianza es `max_prob(softmax(W·x+b))` para `LogisticRegressionClassifier`, `bucket_count/total_count` para `LexicalClassifier`, `max_softmax(cosine_sims)` para `EmbeddingClassifier`.
3. **Threshold routing**: `AutoRouterConfig.threshold_for(task)` compara contra la confidence. Defaults RouteLLM-`mf` calibrated (research/29 line 244): `coding=0.116, plan=0.05, chat=0.20, fix=0.10`. Si `confidence >= threshold` → `strong_model_id`, sino → `weak_model_id` (fallback a `Profile.main_model_id` cuando unset).

Router selector via `model` field ("non-obvious pattern #1", research/29 line 243): el caller pasa `model="gpt-5"` literal o `model="router-auto-0.5"` / `model="router-mf-0.116"` pseudo-id. `RouterId::parse(s)` discrimina entre `Literal` / `Auto{strong_pct}` / `Mf{threshold}`. El branch runtime en el Orchestrator decide si routing-rún strategy-only o routing-auto con classifier — **la request envelope caller-side queda OpenAI-shape, sin tagadura del router**.

M23 migration aporta dos tablas:
- `task_classifier_decisions`: append-only audit (`prompt_hash`, `classifier_kind`, `predicted_task_type`, `confidence`, `features_json`). UNIQUE sobre `(prompt_hash, classifier_kind)` — un prompt reploteado con un mismo backend no mienta nueva fila. CHECK `classifier_kind IN ('lexical','logreg','embedding','main','mf_ab','laya')` (M24 añade `main`/`mf_ab`, M33 añade `laya`) y `confidence BETWEEN 0.0 AND 1.0`.
- `model_affinity_cache`: PK compuesta `(task_type, model_id)` con `success_rate`, `p95_latency_ms`, `mean_cost_usd`, `n_samples`, `updated_at`. `INSERT OR REPLACE` idempotente — sub-fase 2.4 reader lo activa via `ArcSwap::store` para el in-memory cache.

### §7.1 AN-2.3-a — `linfa` MLP deferral (Future Work)

RFC 04 §7 (pre-2.3 wording) y RFC 20 line 78 mencionaban "2-layer MLP `linfa`". Context7 verification (`npx ctx7 docs /rust-ml/linfa "neural network feed forward multilayer perceptron nn training backprop"`, 2026-08-12) retornó *no documentation match* — `linfa` no es un MLP feed-forward module, sólo logistic regression + clustering + SVM. Una MLP hand-rolled sería ~200 LOC de matrix math para ganancias marginales: HybridLLM (arXiv:2404.14618 §4.3 Fig. 5) muestra que logistic regression con BGE-small ya alcanza ~94% de la accuracy del MLP con un peso-footprint un orden de magnitude menor. **Deferral decision (documented in RFC 22 §7 AN-2.3-a)**: la MLP queda deferred a Phase 2.5+ como optimización, ES `LinearCostGuard` budget-safe: cualquier MLP futura entra vía la misma `AutoRouterConfig.weights_path` (formato estended, sin romper logreg JSON).

Legacy: el Skill Graph (`Atlas OS/06 - Skills.md`) almacena `compatible_models` y `recommended_models`. El orquestador **prioriza** los recomendados al disparar una skill (preserva P3 — bất model puede entrar). Las Skills de diseño favorecen Gemini o Claude; las skills de tests priorizan modelos rápidos; las de lógica priorizan razonadores. Phase-2.3 NO rompe eso — la auto-routing layer se situé antes del Skill Graph lookup, mejor: clasifica TASK, despacha al skill compatible, y el skill aporta el MODEL HINT.

## 8. Telemetría `[✅ IMPLEMENTADO Phase 2 — sub-fase 2.4 completa el loop affinity]`

> Phase 1 persiste `was_correct` en M7 `pattern_runs` (alcance: patrón/regla de repair, no invocación del modelo). El reader que une `model_invocations` (M21 sub-fase 2.1) con `task_classifier_decisions` → alimenta la affinity del `ModelRegistry` via `ArcSwap::store` **IMPLEMENTADO en sub-fase 2.4 Feedback Loop**: `journal::read_affinity` + `Registry::refresh_affinity_from_journal` + `AffinityIndex` (arc-swap lock-free), brazo manual `atlas models refresh`.

Cada invocación registra en el Journal:
- modelo, proveedor, latencia, tokens, coste,
- prompt template usada, skill disparadora,
- Confidence emitido, validación posterior,
- `was_correct` (booleano determinado por Learning Engine).

Esto retroalimenta la afinidad del orquestador con el tiempo → el sistema **aprende** qué modelo se porta mejor para cada clase de tarea en cada proyecto.

## 9. Frontends del orquestador (RFC 28 §B item 6)

El Orchestrator es agnóstico del transporte de UI. Hasta Phase 1 disponía de dos frontends:

1. **CLI pura** (`atlas` headless binary, RFC 25 §3.9 / RFC 08) — `mission new`, `plan`, `run`, `resume`, `fork`, `steer`, `swap_model`, `exec`, etc.
2. **HUD Mission Control** (WebView via Tauri, RFC 24) — graph cards, fork-tree, audit timeline, autoresearch card (RFC 28 §A), mission graph (RFC 28 §C), WebSockets over axum.

A partir de **RFC 28 §B** (Phase 1.5d) hay un tercer frontend: el **ACP server embebido en `atlas`** que Microsoft Intelligent Terminal auto-detecta vía `WT_COM_CLSID`. Esto NO convierte al Orchestrator en un proceso externo — el ACP server reusa la misma `crate::orchestrator` que los otros dos frontends, simplemente con un transporte JSON-RPC sobre stdio y un envelope de input distinto (`PromptRequest` en vez de `commands::dispatch`). El contrato es:

- `initialize` → `build_initialize_response` (sin `mcpCapabilities` en Phase 1.5d; ver RFC 28 §B).
- `session/new` + `available_commands_update` → los 6 slash commands del catalogue de `acp/commands.rs`.
- `session/prompt` con comando delegado → `acp::delegate::parse_delegate` enrutas a la lógica que el CLI ya conduce (`opencode exec step`, `/opencode fix` → Repair engine RFC 15, `/opencode restart` → `session/close` + `session/new`).
- `session/set_mode` → override de `SupervisorState` (ver RFC 19 §6.1.2). El Orchestrator aplica el override **antes** del siguiente `tick()` (no interrumpe un step en curso) y persiste en Journal M6 como audit.
- `$/cancel_request` → notification ack no-op (PIa 1.5d host loop no-oped; Phase 2 lo convertirá a `BusEvent::MissionSteered`).

Frontends **no tienen voto** sobre qué modelo usar — el Orchestrator wählt según el Skill Graph (§7), presupuesto (§6) y last `was_correct` (§8). Un frontend sólo puede sugerir via `profile swap <model>` (CLI) o `swap_model` (HUD) o `session/set_mode` (ACP, indirectamente via skill-compatible model del nuevo state). Esta separación mantiene el Orchestrator neutral sobre el transporte, exactamente como P3 (`cualquier modelo puede entrar`).

Single-binary safety (RFC 25 §11): el ACP frontend no añade subprocess externo; reusa tokio runtime + `agent-client-protocol::Stdio` builtin transport. Spawning `wtcli` (Channel 2 worker) es opt-in y gated `acp-server`; en macOS/Linux tanto `run_server` como `spawn_listener` son no-op.

---

## Apéndice — Plan refinado Phase 2 (sub-fases atómicas)

**Source:** `Atlas OS/research/29 - Phase 2 model orchestrator.md`. Evidencia primaria: 11 papers arxiv cross-verified + LiteLLM/OpenRouter/Aider/async-openai/RouteLLM docs.

| Sub-fase | Título | Migración | Cobertura | Estado |
|---|---|---|---|---|
| 2.0 | Foundation (Registry + Tri-model) | M20 (`models`/`deployments`/`model_aliases`/`model_groups`) | §1 (Provider enum, ModelRegistry storage SQLite+JSON seed `ArcSwap<Registry>`, Aider tri-model en `Profile`) | **✅ IMPLEMENTADO** — `orchestrator::{provider, registry}` con `enum Provider` (20 builtin + `Custom(Arc<dyn Config>)`), `ProviderWire` serde-safe (rename explícito para PascalCase cortos), `ModelDescriptor`/`Deployment`/`Capability`/`Tier`, `Registry::{from_seed, from_bundled_seed, resolve, get, descriptors_for_provider, filter_by_resource_mode, filter_by_capability}` con seed JSON bundled (`assets/model_prices_and_context_window.json`, 18 modelos subset LiteLLM MIT), M20 migration idempotente (4 tablas + índices), `Profile` extendido con `main_model_id`/`architect_model_id`/`editor_model_id`/`weak_model_id` + `resource_mode` + `effective_{architect,editor,weak}_model()` fallback a `main_model_id`. 24 tests (13 `provider`, 11 `registry`). `parse()` en vez de `from_str()` para evitar colisión con `std::str::FromStr`. `Provider` sin `Eq/Hash` (Custom varía); cooldown maps usarán `ProviderWire`. |
| 2.0.5 | Provider Normalization Layer | — | Gaps críticos G1 prompt caching, G2 token pre-flight, G5 cooldown per-provider, G8 ToolCall enum cross-provider normalize/deserialize, G17 `Retry-After` header, G18 back-pressure `Arc<Semaphore>` | **✅ IMPLEMENTADO** — `orchestrator::{wire, tokenizer, cache_control, cooldown, backpressure}`. `wire.rs` (~24 tests): `OpShape` neutro + `ToolCall` enum (OpenAI/Anthropic/Gemini/Local dialects) + `normalize()`/`denormalize()` + `prefix_id()`/`next_id()` atomic. `tokenizer.rs` (~17 tests): `trait Tokenizer` + `OpenAITokenizer` (tiktoken-rs 0.6 `o200k_base` BPE via `OnceLock`) + `CharRatioTokenizer` fallback + `tokenizer_for(ProviderWire)`. `cache_control.rs` (25 tests): `CachePolicy`/`CacheTtl` + `inject_breakpoints()`/`extract_cache_read()`/`extract_cache_creation()` (estilo Anthropic ephemeral). `cooldown.rs` (21 tests): `CooldownConfig::default_for(ProviderWire)` + `resolve()`/`resolve_with_retry_after()` con `RetryAfterSource` trait + `CooldownOutcome { duration, origin }` + `DurationClampExt`. `backpressure.rs` (~20 tests): `BackPressureConfig` con `HashMap<ProviderWire, u32>` overrides + `BackPressure` con `parking_lot::Mutex<HashMap<ProviderWire, Arc<Semaphore>>>` lazy-init + `try_acquire()`/`acquire()` async + `available_permits()` + `reset_for()` + `plan_reconfigure()` (no `unsafe`). Cargo: `tiktoken-rs = "0.6"`. 109 tests nuevos (483 total). clippy + fmt + svelte-check + vitest verdes. |
| 2.1 | Routing Policy | M21 (`model_invocations` con sampling_params/seed/cache_read) | §2 (RoutingStrategy enum 6 LiteLLM), §4 completa (3 buckets cascade fallback con `max_fallbacks=5` + exclusion set), §5 (sub-modos filter), §6 parcial (back-pressure), G12 idempotency `RequestFrame`, HUD WS DataParts reconciliation | **✅ IMPLEMENTADO** — `orchestrator::{routing, cascade, idempotency, cost_guard, data_parts}`. `routing.rs` (~20 tests): `RoutingStrategy` enum 6 variantes (`SimpleShuffle`/`LatencyBased{ttl_secs,buffer_ms}`/`UsageBasedV2`/`LeastBusy`/`CostBased`/`Hybrid{branches,default}`/`Custom(Arc<dyn Router>)`), `RoutingConfig` (3 buckets fallback + default_fallbacks + max_fallbacks=5 + cooldown/allowed_fails defaults), `Router` trait, `Condition` enum (TokensAbove/Requires/CostAbove/HasToolCalls/HighStakes), `RouteContext<'a>` snapshot pura, `RouteDecision::{Deploy/NoHealthy}`. `cascade.rs` (~13 tests): `Cascade::next_target(FailureMode, healthy_for)` con weighted failover same-group → bucket escalation → default_fallbacks → max_fallbacks cap. `FailureMode::{RateLimited,BadConfigOrNetwork,ContextWindowOverflow,ContentPolicyRefusal}` + `ExhaustionReason`. `idempotency.rs` (7 tests, G12): `RequestFrame { idempotency_key: Uuid v4, executed_tool_calls, tool_calls_complete }` con `can_cascade()`, `filter_unexecuted(&[OpShape])`. `cost_guard.rs` (8 tests, G11): `AggregationPolicy` trait shape + `LinearCostGuard`/`NoAggregation`. `data_parts.rs` (11 tests): `DataPartBuffer` Vercel AI SDK pattern (mismo id update in-place, transient vs persistent guard, `persistent_only()` post-cascade). M21 migration `journal/schema.rs`: `model_invocations` (19 columns + 3 indexes). `Profile.routing_config: RoutingConfig` añadido (pierde `Eq`, retiene `PartialEq`, `impl Default`). 39 tests nuevos (542 total). clippy + fmt verdes. |
| 2.2 | Aggregation (opt-in HighStakes) | M22 (`reflection_episodes` + `council_votes`) | §3 (`AggregationMode { Single, MajorityVote AgentForest N∈{1,3,5,9} stop-early 2/3, MoA 3×3 cost guard G11, Council 1-round, SelfRefine cap 2, Reflexion multi-model cap 3 anti-doom-loop, SelfDiscover skeleton cache }`). Papers: 2402.05120, 2406.04692, 2305.14325, 2303.11366, 2303.17651, 2402.03620 | **✅ IMPLEMENTADO** — `orchestrator/aggregation/` (mod + 6 submodules: `majority_vote`, `moa`, `council`, `reflexion`, `self_refine`, `self_discover`). `enum AggregationMode` (`#[derive(Default)]` con `Single` default + `serde(tag="type", rename_all="snake_case")`) con 7 variantes. `trait Aggregator` (`#[async_trait]`): `async fn aggregate(ctx: AggregationContext) -> Result<FusedResponse, AggregationError>`. `aggregator_for(&mode) -> Option<Arc<dyn Aggregator>>` despacha con unit struct constructors (clippy `default_constructed_unit_structs`). `MajorityVoteAggregator` con `canonicalise()`, `agreement_ratio()`, `shannon_entropy()`, `mode_winner()`, stop-early tras batches de 3. `MoAAggregator` 3×3 layer loop. `CouncilAggregator` con `mode_winner_str()` + schema `council_votes`. `ReflexionAggregator` con `detect_doom_loop()` (2 episodios consecutivos idénticos → abort) + schema `reflection_episodes`. `SelfRefineAggregator` con `line_delta()` + `StopCondition::{Converged,MaxIterations,Stalled}`. `SelfDiscoverAggregator` con `CachedSkeleton` + `SKELETON_CACHE` (`OnceLock<Mutex<HashMap>>`) + `cache_key()` (SHA-256 de URL+prompt) + `reset_cache_for_tests()`. `cost_guard.rs`: 6 mode-specific `AggregationPolicy` impls (`MajorityVoteCostGuard`/`MoACostGuard`/`CouncilCostGuard`/`ReflexionCostGuard`/`SelfRefineCostGuard`/`SelfDiscoverCostGuard`) + 6 tests (14 total). M22 migration `schema.rs`: `reflection_episodes` (id, mission_id, episode_idx, attempt, executor_model, reflexor_model, response, self_reflection, failure_signal, created_at + index) + `council_votes` (id, mission_id, round, debater_model, vote_text, vote_confidence, rationale, created_at + index). `Profile.aggregation: AggregationMode` field añadido (default `Single`). `orchestrator/mod.rs` re-exports `AggregationContext`/`AggregationError`/`AggregationMode`/`AggregationModeSnapshot`/`Aggregator`/`FusedResponse`/`ReflectionEpisodeOut`/`StopCondition` + 6 cost guards. 39 tests nuevos (581 total). clippy + fmt verdes. |
| 2.3 | Auto-routing Classifier + MCP-aware | M23 (`task_classifier_decisions` + `model_affinity_cache`) | §7 completa (TaskTypeClassifier trait async + `enum TaskType` 12 concretos + `AutoRouterConfig` opt-in off-by-default), `McpToolFilter::pre_filter` G19 (capability_tags AND tool_capabilities con `McpServerCatalog` trait + `NoMcpCatalog`/`StaticMcpCatalog` defaults), `RouterId::parse` "router-auto-0.5"/"router-mf-0.116" / Literal fallback, threshold defaults `coding=0.116` RouteLLM-mf calibrated | **✅ IMPLEMENTADO** — `orchestrator/classifier/` (mod + 5 submódulos). `mod.rs`: `AutoRouterConfig` (off-by-default + `ClassifierKind::{Lexical,LogReg,Embedding}` + per-task thresholds default RouteLLM-mf `coding=0.116/plan=0.05/chat=0.20/fix=0.10`, retries idempotentes via `upsert_threshold`), `TaskVerdict{task_type,confidence,backend}`, `ClassifierError`, `classifier_for(&cfg) -> Arc<dyn TaskTypeClassifier>` dispatcher con fallback silencioso Lexical cuando logreg weights-missing o `fastembed` feature-off, `TaskType::ALL` const array de 12 concretos + round-trip `as_str`/`parse`, `Default::default()` = `Unknown`. `lexical.rs` (`LexicalClassifier`, 12 tests): regex counts con word-boundary (`count_occurrences` no substring false-positivos: `fix` no match`infix`), `extract_features()` reutiliza `LogisticRegressionClassifier`, BUCKETS const [TaskType,&[&str]] con keywords léxicas (write/test/refactor/fix/build/plan/review/explain/translate/tidy/exec/chat). `log_reg.rs` (`LogisticRegressionClassifier`, 11 tests): multi-class one-vs-rest softmax con numeric stability (`max(logit)` subtraction), load JSON weights con shape validation (12×12 + 12 intercepts + 12 class-labels parse TaskType::parse), `forward()` acceleration async-via-Ready. **`linfa` MLP deferral documentado** (RFC 22 §7 AN-2.3-a: linfa no tiene MLP feed-forward per Context7 verification, scratch ~200 LOC para +6% accuracy marginal vs `LogisticRegressionClassifier` que ya alcanza ~94% del MLP vía arXiv:2404.14618 §4.3). `embedding.rs` (`EmbeddingClassifier` cfg `fastembed`): fastembed-rs `BGESmallENV15` default (384 dim, ONNX), cosine vs 12 prototypes, load JSON `[[f64;384];12]`, fast init fallback al default Lexical. `router_id.rs` (`RouterId::Literal/Auto{strong_pct}/Mf{threshold}`, 12 tests): parser `router-auto-0.5`/`router-mf-0.116` con range check [0.0,1.0], fallback Literal en no-numeric/out-of-range/trailing-garbage. `mcp_filter.rs` (`McpToolFilter::pre_filter`, 8 tests): `McpServerCatalog` trait + `NoMcpCatalog` default + `StaticMcpCatalog` test helper, intersection capability_required ⊆ model.capabilities AND tool_names_required ⊆ catalog.available_tool_names (G19). M23 migration `journal/schema.rs`: `task_classifier_decisions` (id, mission_id, prompt_hash, predicted_task_type, confidence, features_json, classifier_kind CHECK IN ('lexical','logreg','embedding'), UNIQUE (prompt_hash,classifier_kind), 2 índices), `model_affinity_cache` (PK compuesta task_type+model_id, success_rate/p95_latency_ms/mean_cost_usd/n_samples/updated_at, INSERT OR REPLACE idempotente para 2.4 reader). `Profile.auto_router: AutoRouterConfig` añadido (serde default, off-by-default). 63 tests nuevos (638 total). clippy + fmt verdes. |
| 2.4 | Feedback Loop + `mf` experimental | M24 (`model_affinity_cache` mirror + `task_classifier_decisions` CHECK fix, schema 23→24) | §8 completa (reader `model_invocations` GROUP BY `(task_type, model_id)` → `AffinityRow` → `ArcSwap::store`, `RoutingStrategy::Mf { threshold }` strong-vs-weak por classifier confidence) | **✅ IMPLEMENTADO** — `orchestrator::affinity` (6 tests): `AffinityRow {task_type, model_id, success_rate, p95_latency_ms, mean_cost_usd, n_samples}` + `MIN_SAMPLES=3` confidence floor + `AffinityIndex` (ArcSwap lock-free, `store_all`/`load()` Guard + `get()` con filtro below-min-samples; clones comparten el snapshot subyacente). `journal::{read_affinity, upsert_affinity_rows}`: reader GROUP BY ventana + persistencia M24 mirror idempotente (`INSERT OR REPLACE`). `registry.rs`: `Registry.affinity: AffinityIndex` + `refresh_affinity_from_journal(journal, window)` — re-crunch + swap atómico. `routing.rs`: `RoutingStrategy::Mf { threshold }` (4 tests) — strong-vs-weak binary routing (`classifier_confidence >= threshold` → restringe a `strong_ids`, `<` → resto) + weighted-shuffle fallback graceful cuando el split vacía (evita `NoHealthy` si el caller olvidó poblar `strong_ids`). `cost_guard.rs`: `AggregationCostContext::from_journal(journal, model_id, ...)` — "real cost_guard impl" (research/29 line 252): rellena `tokens_per_sample`/`blended_cost_per_1m` desde medias rolling-window de `model_invocations`; el orchestrator degrada a single-model si excede el budget. CLI `atlas models refresh -n <window> --list` (`cli/commands/models.rs`): brazo manual del loop. Cargo: `arc-swap = "1.7"` (MIT OR Apache-2.0, zero transitive deps, single-binary safe RFC 25 §11). Desviación documentada: Mf sin `include_bytes!("assets/mf_weights.bin")` pre-trained (G14) — flavour experimental implementado sobre classifier confidence; weights pre-trained + A/B testing postergado a 2.5+. |

**KPIs Phase 2:** coste LLM por mission ≤ Phase 1 × 0.6 (RouteLLM evidence >2× savings); routing overhead < 5ms p95 (ArcSwap lock-free); re-ingresos model crash 100%; feedback loop `was_correct` < 30s tras verdict usuario.

**Sub-pases deferrables (Future Work anotados, no en Phase 2):** G4 streaming partial aggregation, G6 circuit-breaker half-open state, G7 provider-level health HUD, G9 vision/multimodal routing capability mask, G10 Ollama `supports_tools` CSV, G13 HUD approval card aggregation, G15 `opencode calibrate-classifier` CLI, G16 offline eval harness `atlas eval`, G19 MCP tool-capability-aware routing (`pre_filter` skeleton + `McpServerCatalog` trait en 2.3, integración con `rmcp` real registry deferred a 2.5+), G20 response cache separado de prompt cache provider, AN-2.3-a linfa MLP feed-forward (deferral — `linfa` no lo tiene, scratch impl deferred a Phase 2.5+).

---

## Apéndice — Catálogo de providers y routers (research/62)

`research/62` Capa 1c/1e fija el universo de providers que el Registry (§1) debe cubrir. El default build ya soporta HTTP OpenAI-compatible (RFC 25 §3.8); este apéndice es el objetivo de cobertura.

- **Cloud comerciales:** OpenAI, Anthropic, Google (Gemini), xAI (Grok), Mistral, Cohere, AI21.
- **Cloud CN / open-weight:** Zhipu **GLM**, **MiniMax**, **DeepSeek**, Alibaba **Qwen**, Moonshot **Kimi**, ByteDance **Seed/Doubao**.
- **Free-tier / inference:** **NVIDIA NIM**, **Groq**, **Cerebras**, **SambaNova**, Cloudflare Workers AI, GitHub Models, HuggingFace Inference, OpenRouter.
- **Locales:** Ollama, LM Studio, llama.cpp, vLLM, SGLang.
- **Gateways/routers (patrón):** LiteLLM, OpenRouter, Portkey, Helicone, ZenMux.
- **Evals/leaderboards:** llm-stats, Artificial Analysis, arena.ai, BridgeBench.

El routing (`orchestrator/routing.rs`) prioriza por tier/coste/capability; los free-tier de la lista alimentan el modo `free-only` (§6). Cross-ref: RFC 26 (`providers`), RFC 25 §15.

