# Phase 2 — Multi-Model Orchestration: Plan Refinado + Research Findings

**Fecha:** 2026-08-05 · **Orquestador:** opencode (z-ai/glm-5.2)
**Source primaria:** arxiv 11 papers (cross-verified IDs), LiteLLM docs, OpenRouter docs, Aider README/changelog, async-openai README+MIDDLEWARE, async-anthropic docs.rs, RouteLLM GitHub, docs.rs (tower/governor/arc-swap/notify/rusqlite/dashmap), Anthropic prompt-caching docs, LangSmith invocation schema.
**Propósito:** Refinar la Fase 2 RFC 20 ("Multi-model Orchestration") con evidencia primaria externa + auditoría iterativa gaps vs proyectos comparables. Output: 6 sub-fases atómicas commiteables (no PRs, per AGENTS.md §9).

---

## SECTOR A — Investigación Round 1: Orquestadores líderes

### A.1 LiteLLM (BerriAI, 55.6k★, MIT)

Fuente: `docs.litellm.ai/docs/routing`, `docs.litellm.ai/docs/proxy/reliability`, repo root.

**Concepto clave: `Deployment` ≠ `Model`.** Un `model_name` agrupa N `litellm_params` (cada uno = 1 deployment con `api_key`/`api_base`/region distinta). Health/cooldown **per-deployment** (`model_id = hash(litellm_params)` determinista), no por grupo. → OpenCodeOS: separar `ModelDescriptor` (qué modelo lógico) de `Deployment` (dónde/cómo se sirve).

**5 routing strategies como enum explícito** (`simple-shuffle | latency-based-routing | usage-based-routing-v2 | least-busy | cost-based-routing`), cada una con bucket de estado y TTL configurable. Default prod = `simple-shuffle` — LiteLLM advierte explícitamente "Usage-based routing is not recommended for production due to performance impacts".

**Routing Groups per-model** (novedad 2024-2025). Una sola instancia de Router aplica `latency-based` para `gpt-4o` y `simple-shuffle` para el resto. Conflicto (model_name en 2 grupos) → `ValueError` en init.

**Cooldown triggers discriminados**: 429 inmediato, >50% fallos/min, errores no-reintentables (401/404/408). Defaults `allowed_fails=3`, `cooldown_time=5s` — pero **recomendación explicita: per-model override** (bedrock 1s, anthropic 30s, openai 60s).

**3 buckets de fallback tipados**: `fallbacks` (generic 429/500), `context_window_fallbacks`, `content_policy_fallbacks` + `default_fallbacks`. Per-request `disable_fallbacks: true`.

**`RetryPolicy` + `AllowedFailsPolicy` discriminan por tipo de excepción**: `ContentPolicyViolationErrorRetries=3, AuthenticationErrorRetries=0, BadRequestErrorRetries=1, TimeoutErrorRetries=2, RateLimitErrorRetries=3`.

**Pricing en JSON versionado**, no hardcoded: `model_prices_and_context_window.json` + `.schema.json` (repo root). MIT-licensed.

### A.2 OpenRouter (openrouter.ai)

**Auto Beta = task-type classifier + spend-share ranking**: (1) clasificador ligero asigna ~30 task types; (2) lookup de qué modelos gastan los devs en ese task type en ventana 7 días (`rankings.ai` Share of Spend); (3) dial `cost_quality_tradeoff` 0-10 filtra por percentil; (4) fallbacks en orden de spend-share.

**Latest-alias (`~openai/gpt-latest`)** resuelve al flagship actual sin redeploy → patrón "mutable alias" en el registry del OS.

**Session Stickiness 2 niveles**: implícita (hash del primer system+user message) o explícita (`session_id`). Ambas expiran a 5 min de inactividad; cada request resetea el timer. Si el provider cacheado falla, **no** se actualiza el cache (permite re-route).

**Auto Beta degrada con grace**: si clasificador o rankings caen, usa default set — request nunca falla por infra de routing.

### A.3 RouteLLM (lm-sys, 5.3k★, Apache-2.0)

Fuente: `github.com/lm-sys/RouteLLM`, paper `arxiv:2406.18665` (corregido del originalmente citado `2406.06585` que resultó ser "Symbolic Regression").

**Routers 4 tipos out-of-the-box**: `mf` (matrix factorization), `sw_ranking` (softmax-weighted ranking), `bert`, `causal_llm`. **MF supera a BERT en cost/perf** en sus benchmarks: >2× cost reduction manteniendo calidad; MF logra misma calidad que BERT a >40% lower cost.

**Threshold calibration script**: `python -m routellm.calibrate_threshold --routers mf --strong-model-pct 0.5` (dataset Chatbot Arena inference). → patrón portable: `opencode calibrate-classifier`.

**API surface "drop-in"**: `Controller(routers=["mf"], strong_model, weak_model)` + servidor OpenAI-compatible. El `model` field admite `router-mf-0.116` o `gpt-4o` indistintamente — caller no sabe si es router-agregado o modelo-directo. **Patrón elegante**: elimina branches en el caller.

### A.4 Aider (Aider-AI, 48k★, Apache-2.0)

Fuente: `aider.chat/docs/config/options.html`, changelog HISTORY.md.

**Tri-model**: `--model` (main/architect) + `--editor-model` (ejecuta edits con `--editor-edit-format`) + `--weak-model` (commit messages AND **chat history summarization** cuando `--max-chat-history-tokens` se excede). El weak-model role es **no-obvio**: hace compactación de history, no commits.

**`--architect` mode**: main propone cambios en natural语言/diff, editor los aplica al código.

**Prompt caching Anthropic gestionado desde v0.42**: Aider injecta `prompt_cache_control` markers y `caching=true` en Anthropic SDK, logs `cache_read tokens` para cost accounting. **Sin esto, el feedback loop reporta cost inflated para modelos Anthropic** — contaminaría el affinity learning (gap crítico detectado en Round 3).

**`--check-model-accepts-settings`**: pre-flight que valida `reasoning_effort`/`thinking_tokens` aceptado por el modelo.

**`--alias ALIAS:MODEL`** acumulable — equivalente a OpenRouter `~openai/gpt-latest` pero a nivel CLI.

### A.5 Vercel AI SDK v7 (`ai` npm)

Fuente: `ai-sdk.dev/docs/foundations/providers-and-models`, `ai-sdk.dev/docs/ai-sdk-ui/streaming-data`.

**`LanguageModelSpecification v3`** publicada como package OSS — contratos formales versionados (`v1`/`v2`/`v3`) para no romper compat atrás.

**`UIMessageStream` con Data Parts tipados con ID reconciliation**: mismo `id` → cliente actualiza in-place. Matpea WS bus actual (RFC 24): puedo enviar `data-card-transition` con IDs estables que se reconcilian.

**Transient parts (`transient: true`)**: sólo llegan al cliente vía `onData`, no se persisten al `message.parts`. **Distinción crítica**: lo que ve el usuario vs lo que se journaliza.

**`Language Model Middleware`** para interceptar/cachear/rate-limit sin tocar el provider → composabilidad: budget enforcement sin tocar el adapter.

---

## SECTOR B — Papers SOTA (IDs cross-verified)

| Paper (arXiv ID verificado) | Hipótesis | Top resultado | Patrón portable |
|---|---|---|---|
| **FrugalGPT** `2305.05176` | cascade de LLMs por cost-aware adapter | Hasta 98% cost reduction matching best single LLM; +4% acc sobre GPT-4 a mismo cost | `cost-aware cascade` en Kernel Bus: prompt → barato → verifier → escalar si score<threshold. Schema `model_cascade`. |
| **HybridLLM** `2404.14618` (Microsoft, no Meta) | router predicting (Q_large−Q_small) usando BERT/MLP + uncertainty | Hasta 85% cost reduction manteniendo calidad strong | `QueryDifficultyRouter` MLP con features léxicos + embeddings → score `gap_q`. Privacy gate léxico → forced local. |
| **RouteLLM** `2406.18665` | MF router sobre Chatbot Arena preference data | >2× cost reduction sin quality loss; MF > BERT a 40% lower cost; transfer entre model pairs | `CollaborativeFilterRouter` (matrix factorization <100KB). Threshold default `coding=0.116`. |
| **LLM-Blender** `2306.02561` | PairRanker (cross-attention) + GenFuser top-K | PairRanker highest correlation w/ ChatGPT ranking; supera single+ensemble baselines | `PairwiseCouncil` para plan-final/refactor: N candidates → PairRanker (PairRM 0.4B vía fastembed-rs ONNX) → GenFuser merge top-3. |
| **More Agents Is All You Need** `2402.05120` (TMLR) | sampling+vote escala con N instancias, gain correlaciona con difficulty | Scaling power-law; gain se satura >10 agents; usual N∈{3,5,8}; ceiling collapse en tasks demasiado difíciles | `AgentForest` difficulty-aware N. Stop-early: tras batch de 3 votos, si agreement ≥2/3 Y entropy <0.5 → cortar. Max N=9. |
| **MoA (Together AI)** `2406.04692` | capas: cada agente en capa i toma outputs capa i-1 | OSS-only MoA = 65.1% AlpacaEval-2 vs GPT-4o 57.5% | `MoACouncil` 3×3 — reservar para high-blast-radius: plan-final, breaking-change-detect. Coste 9×. |
| **LLM-Debate** `2305.14325` | N LLM proponen+debatan+refinan → convergencia | Mejora math+strategic reasoning; reduce hallucinations | `DebateValidator` para plan correctness & API contract mismatch: 2-3 debaters + 1 round. Use sparingly. |
| **Self-Discover** `2402.03620` | LLM auto-compone reasoning structure de modulos atómicos | +32% sobre CoT en BigBench-Hard; +20% sobre CoT-Self-Consistency a 10-40× fewer FLOPs | `SelfDiscoverPlanner` (RFC 12): pre-decode selecciona modules (`step_by_step, contrast_cases, decomp, abstraction`) → JSON skeleton reusado. Cachea por URL+prompt_embedding. |
| **Reflexion** `2303.11366` | verbal reflection en episodic memory sin weight updates | 91% pass@1 HumanEval vs GPT-4 80% | `JournalReflexionRing`: fallo → crítica verbal → `reflection_episodes` SQLite → inject en siguiente intent. Cap 3 (saturation). |
| **Self-Refine** `2303.17651` | same-LLM generator→feedback→refiner | +20% abs promedio en 7 tasks; GPT-4 mejorable a test-time | `TestTimeRefiner` (RFC 13): codegen → self-critique (compile/tests/lint) → regen delta. Cap 2 iter; abort si `delta_lines<10`. |
| **Constitutional AI** `2212.08073` (Anthropic blog) | self-critique + revision contra principles list + RLAIF | Harmless non-evasive; SL+RL ambos benefit de CoT | `ConstitutionalGate` (RFC 23): `principles.md` por profile → critique → revise. NO runtime filter — offline mission-brief synthesis only. |

---

## SECTOR C — Rust architecture best practices

### C.1 Trait design para LLM providers

Fuente: `github.com/64bit/async-openai` (README+MIDDLEWARE), `docs.rs/async-anthropic/latest` (v0.6.0, MIT, bosun-ai).

**`async-openai`** define `trait Config` + `OpenAIConfig` default + `Box<dyn Config>` para polimorfismo (`let config = Box::new(OpenAIConfig::default()) as Box<dyn Config>;`). Permite match exhaustivo sobre variantes conocidas (zero-cost Static dispatch) Y Box para Custom en runtime.

**API group pattern**: `client.chat()` / `client.responses()` / etc., cada uno con `.path()/.query()/.header()` aditivos. Boundary del middleware NO es `reqwest::Request` sino `HttpRequestFactory` (cheaply clonable) porque `reqwest::Request` no es `Clone` una vez tiene streaming body — retry necesita replay.

**`OpenAIRetryLayer` distingue 429 retryable vs quota-permanent**: consume el body para clasificar; respeta `Retry-After`. Tower `BoxError` → `OpenAIError::Boxed`.

**`async-anthropic`** existe (MIT, bosun-ai): `reqwest` + `reqwest-eventsource` para SSE + `backoff` + `secrecy` para API keys. **No hay crate Gemini nativo de uso general** — práctica estándar es HTTP directo con `reqwest` + `reqwest-eventsource`, modelando tus propios tipos con `serde`.

### C.2 Storage pattern single-binary

Fuente: LiteLLM root, `docs.rs/rusqlite`, `docs.rs/notify`, `docs.rs/arc-swap`.

**`include_str!("model_prices_and_context_window.json")`**: se compila dentro del binario (const-evaluable en `Lazy`/`OnceLock` vía `serde_json::from_str`). SQLite guarda overrides por deployment/profile. Single-binary intact (RFC 25 §11).

**`rusqlite::Transaction`** envuelve BEGIN/COMMIT/ROLLBACK automáticamente en drop; combinado con `CREATE TABLE IF NOT EXISTS` en `Batch::new(&conn).iterate(sql)` se producen seeds idempotentes sin framework externo.

**`notify::recommended_watcher(EventHandler)`** (v8.2.0, CC0) para hot-reload opcional del JSON sidecar. Caveat: NFS/WSL2 paths no emiten eventos → fallback `PollWatcher`. **Opt-in, no critical path** — JSON embebido en binario es source of truth.

**`ArcSwap<Registry>`**: la docs literalmente muestra `ROUTING_TABLE: Lazy<ArcSwap<RoutingTable>>` + `let table = ROUTING_TABLE.load(); table.route(packet);`. Cuando watcher recarga, `registry.store(Arc::new(new_registry))` y lecturas en vuelo usan versión vieja hasta que dropan el `Guard`. Lock-free reads.

### C.3 Routing policy algorithms

Fuente: LiteLLM routing docs, `docs.rs/governor` (GCRA), `docs.rs/tower/limit`, `docs.rs/arc-swap`.

**SimpleShuffle = `rand::distributions::WeightedIndex`** sampleado tras filtrar deployments enfriados. Cada deployment aporta `weight`; sin weight → fallback a `rpm` o uniforme.

**EWMA latency tracking**: `HashMap<DeploymentId, Histogram<u64>>` (hdrhistogram). EWMA = `α * nueva + (1-α) * vieja` con `α = 2/(N+1)`, `N = ttl_secs / bucket`. `lowest_latency_buffer=0.5` considera deployments dentro del 50% del mínimo (evita saturar el recién-llegado).

**Cooldown per-deployment = `DashMap<DeploymentId, (Instant, u32)>`** checked en pre-call hook. `tokio::time::Instant` para no depender de wall clock (tests deterministas).

**Rate limiting con `governor` 0.10.4** (GCRA, más eficiente que token bucket clásico): `Quota::per_second(nonzero!(50u32))` + `DefaultKeyedRateLimiter` por deployment ID. Alternativa `tower::limit::RateLimitLayer` si transporte es Tower.

**Budget stream cancel con `tokio_util::sync::CancellationToken`**: envolver `Stream<Item = Result<Event>>` de `reqwest-eventsource` con `take_until(token)`. Pre-flight estimate + stream counter + future abort.

### C.4 Anti-patrones async Rust

Fuente: Tokio tutorial, arc-swap docs.

- **No `block_on` dentro de async** — runtime panics/deadlocks. `block_in_place` (solo multi-thread) o spawn task + canales.
- **No sostener `MutexGuard` a través de `.await`** — std ni tokio. Clone datos, drop guard, luego await.
- **No `Rc`/`RefCell` en tasks spawned** — requiere `Send + 'static`. Para state compartido: `Arc<T>` + `ArcSwap`/`DashMap`/`Semaphore`.

---

## SECTOR D — Auditoría iterativa (Round 3) — Gaps detectados

20 gaps detectados vs proyectos comparables. Categorizados:

### D.1 Críticos — incorporar antes de cerrar Phase 2

| # | Gap | Por qué crítico | Patrón portable | Fuente |
|---|---|---|---|---|
| **G1** | **Prompt caching accounting** — sin `cache_read_tokens`, feedback loop reporta modelos Anthropic 3-5× más caros de lo real → affinity aprenderá mal | `cache_control: { type: "ephemeral" }` marker injection + `cache_read_input_tokens` field en `model_invocations` | Anthropic prompt-caching docs; Aider v0.42 |
| **G2** | **Token counter pre-flight** — cascade `context_window_fallbacks` reacciona post-error en vez de pre-flight | `tiktoken-rs` (OpenAI), `tokenizers` (HF/Ollama), Anthropic `count_tokens` endpoint. `Tokenizer::estimate(payload) -> u32` pre-routing | docs.rs crates |
| **G3** | **Reproducibility params** — `model_invocations` no captura `seed`, `temperature`, `top_p`, `top_k`, `response_format` | Añadir columnas `seed INTEGER NULL`, `temperature REAL`, `sampling_params JSON` | LangSmith invocation schema |
| **G5** | **Cooldown time hardcoded 5s** — rate-limits reales 30-60s, `allowed_fails=3` reabre en frío | Per-`Provider` override: `{anthropic: 30s, openai: 60s, ollama: 5s}`. Leer `Retry-After` header 429 → override dinámico | LiteLLM docs; RFC 7231 |
| **G8** | **Tool-call normalization cross-provider** — cascade Anthropic→OpenAI→Gemini romperá con tools | `ToolCall` enum `OpenAI(name, args_json) | Anthropic(name, input) | Gemini(name, args)` + `normalize(call) -> OpShape` + `deserialize(shape, target_provider)`. **Capa explícita antes de 2.1** | LiteLLM `litellm.utils.function_call` normalization |
| **G11** | **Cost guard pre-aggregation** — Council involuntario puede vaciar presupuesto en un turn | `AggregationPolicy::pre_cost_estimate(n_models, est_tokens)` → if > budget → fallback a `Single`. Patrón `max_budget` per virtual key de LiteLLM | LiteLLM virtual_keys |
| **G12** | **Idempotency de tool-calls en fallback** — reintentar con otro deployment duplicaría side-effects (filesystem write) | `RequestFrame { idempotency_key, executed_tool_calls: Vec<ToolCallId> }` — fallback solo reenvia si `tool_calls_complete==true` | RFC 02 §3.1.2 |
| **G17** | **`Retry-After` header parsing** en 429 — plan original sólo cooldown fijo, no respeta provider hint | `reqwest::Response::headers().get("Retry-After")` → override dinámico de `cooldown_until`. Gratis. | LiteLLM routing docs |
| **G18** | **Back-pressure semáforo por deployment** — `max_concurrent_remote_calls` (RFC 04 §6) sin implementación explícita | `Arc<Semaphore>` per `ProviderId` con `acquire_owned()`. Effort S | `tokio::sync::Semaphore` |

### D.2 Deferrables (Future Work, anotar en RFC 20)

- **G4** Streaming partial aggregation (MoA/Vote sobre streams, no esperar full completion). Future Work 2.4+.
- **G6** Circuit breaker con `half-open` probes. OK sin half-open en 2.1; añadir en 2.4+.
- **G7** Provider-level health aggregation para HUD. Future Work HUD.
- **G9** Vision/multimodal routing — capability mask `vision` filter. Future Work 2.5+.
- **G10** Local Ollama capability mapping CSV (`supports_tools` por modelo). Phase 2.3.
- **G13** HUD approval card para aggregation cost estimate. Future Work HUD card.
- **G14** `mf` router como first-class `RoutingStrategy::Mf { threshold }` con `include_bytes!` weights pre-trained. Phase 2.4+ (evaluable).
- **G15** `opencode calibrate-classifier` CLI. Phase 2.5+.
- **G16** Offline eval harness (`opencode eval --router R --benchmark B`). Future Work.
- **G19** MCP tool-capability-aware routing — pre-filter deployments por `capability_tags` AND `tool_capabilities` de MCP servers disponibles. Phase 2.3+.
- **G20** Response cache keyed por `hash(prompt_payload + model_id + temperature)` — distinto del prompt cache provider. Phase 3 feature.

### D.3 Patrones no-obvios adicionales (incorporar)

1. **Router selector via `model` field** (RouteLLM `router-mf-0.116`): caller emite OpenAI shape sin saber si es router-agregado o modelo-directo. **Elimina branches en el caller**.
2. **LiteLLM fallback cache**: si fallback sucedió correctamente, cachear response keyed por `prompt_hash` para no recorrer cascade la siguiente vez. Distinto de prompt cache provider.
3. **Tag pre-filter hot-path**: antes de invocar classifier, filtrear deployments por `capability_tags` obligatorias (`vision`, `tools`) — search space shrink.
4. **RouteLLM `mf` ablation evidence**: MF > BERT en MMLU/GSM8k al mismo cost. Justifica `mf` como `RoutingStrategy` experimental en 2.4 con baseline comparativo — mínimo effort.
5. **`cooldown_time` per-provider override**: bedrock 1s, anthropic 30s, openai 60s. 5s global es demasiado corto para 429s reales de anthropic.

---

## SECTOR E — Plan refinado Phase 2 (6 sub-fases atómicas)

Per AGENTS.md §9: commits atómicos, no PRs. Cada sub-fase produce cargo check + clippy + tests verdes y es self-contained.

### Sub-fase 2.0 — Foundation (Registry + Tri-model)

- `enum Provider { OpenAI, Anthropic, Gemini, VertexAI, Bedrock, Azure, Ollama, LmStudio, OpenRouter, Custom(Arc<dyn Config>) }` en `src-tauri/src/orchestrator/provider.rs`.
- `trait Config` async-openai-style: `Box<dyn Config>` para Custom variant, match exhaustivo para hardcoded.
- M20 migration: SQLite tables `models`, `deployments`, `model_aliases`, `model_groups`.
- JSON seed: `src-tauri/src/orchestrator/assets/model_prices_and_context_window.json` (bundle LiteLLM MIT) cargado en `OnceLock` vía `include_str!`. `opencode models refresh` re-sincroniza desde el JSON sin rebuild.
  - **License audit**: LiteLLM `model_prices_and_context_window.json` MIT. Atribución en module-level prose.
- `ArcSwap<Registry>` para lock-free reads. `DashMap<DeploymentId, (Instant, u32)>` para cooldown.
- `Profile` extendido: `architect_model`, `editor_model`, `weak_model` (todos `Option<ModelId>`, default = `main_model`).
- Tests: happy-path enum exhaustive match, ArcSwap atomic swap, JSON seed parsing roundtrip, `Profile` tri-model defaults.

### Sub-fase 2.0.5 — Provider Normalization Layer (gap crítico G8, G1, G5, G17, G18)

**Nueva sub-fase tras Round 3 — sin ella el cascade cross-provider y el feedback loop son incorrectos.**

- `ToolCall` enum cross-provider: `OpenAI { name, args_json } | Anthropic { name, input } | Gemini { name, args } | Local`. `normalize(call) -> OpShape` y `deserialize(shape, target_provider)`.
- `Tokenizer` trait: `tiktoken-rs` para OpenAI, `tokenizers` para HF/Ollama, `count_tokens` endpoint para Anthropic. `estimate(payload) -> u32` pre-routing (G2).
- Prompt cache control (G1): `cache_control` marker injection en `prompt_payload::*`. `model_invocations.cache_read_input_tokens INTEGER` tracking.
- Cooldown configurable per-provider (G5): `Profile.cooldown_overrides: HashMap<Provider, Duration>`. Defaults: `{anthropic: 30s, openai: 60s, ollama: 5s, bedrock: 1s}`.
- `Retry-After` header parsing (G17): `reqwest::Response::headers().get("Retry-After")` → override dinámico de `cooldown_until`.
- Back-pressure semáforo (G18): `Arc<Semaphore>` per `ProviderId` con `acquire_owned()`. `max_concurrent_remote_calls` leído desde `Profile`.
- Tests: tool-call normalization roundtrips (Anthropic↔OpenAI, Gemini↔Anthropic), tokenizer accuracy ±2 tokens, cache_control marker injection, cooldown override con `Retry-After` mock, semáforo back-pressure.

### Sub-fase 2.1 — Routing Policy

- `enum RoutingStrategy { SimpleShuffle, LatencyBased { ttl_secs: u64, buffer: f64 }, UsageBasedV2, LeastBusy, CostBased, Hybrid(Vec<(Condition, RoutingStrategy)>), Custom(Arc<dyn Router>) }`. Sin `Mf` aún (esa aterriza en 2.4 como experimental).
- M21 migration: `model_invocations` table (latency_ms, tokens_in, tokens_out, cost, model_id, provider, was_correct NULL, cache_read_input_tokens, seed INTEGER NULL, temperature REAL, sampling_params JSON, route_taken_json ARRAY).
- `RoutingConfig`: 3 buckets fallback (LiteLLM) — `fallbacks`, `context_window_fallbacks`, `content_policy_fallbacks` + `default_fallbacks: Option<Vec<ModelId>>` + `max_fallbacks: u8` (5).
- Cascade failover impl: `enable_weighted_failover` dentro del mismo model_group primero (exclusion set `HashSet<DeploymentId>`); escalar a fallbacks cross-group tras agotar peers. Cap 5.
- 429 cooldown inmediato; >50% fails/min trigger; 401/404/408 → fallbacks no reintento. `RetryPolicy` + `AllowedFailsPolicy` discriminados por error type (per Aider-LiteLLM-RFC 28 §H).
- Cost guard pre-aggregation (G11): `AggregationPolicy::pre_cost_estimate()` — no en esta sub-fase pero el trait shape se define aquí para que 2.2 lo implemente.
- Idempotency (G12): `RequestFrame { idempotency_key: Ulid, executed_tool_calls: Vec<ToolCallId>, tool_calls_complete: bool }` — fallback solo si `complete`.
- HUD WS `Data Parts` con ID reconciliation (Vercel AI SDK pattern) — mismo ID actualiza in-place. Transient vs persistent guard.
- Tests: SimpleShuffle weighted random distribution, cooldown inmediato 429, 3 bucket routing discriminados, fallback cascade con exclusion set acumulativo, idempotency skip en fallback, HUD DataParts reconciliation.

### Sub-fase 2.2 — Aggregation (opt-in HighStakes)

- `trait Aggregator { async fn aggregate(prompts, responses) -> Result<fused_response>; }`.
- `enum AggregationMode { Single, MajorityVote { n_samples: u8, stop_early_threshold: f32 }, MoA { layers: Vec<Vec<ModelId>> }, Council { debaters: Vec<ModelId>, rounds: u8 }, SelfRefine { max_iterations: u8, stop_condition }, Reflexion { memory_buffer_size: u8 } }`.
- `MajorityVote` (AgentForest `2402.05120`): difficulty-aware N (easy=1, medium=3, hard=5/9). Stop-early tras cada batch de 3 votos si agreement ≥2/3 Y entropy <0.5. Canonicalize respuestas antes de votar (strip whitespace, lowercase).
- `MoA` (`2406.04692`): solo en `ExecutionMode::HighStakes` (RFC 19). 3 capas × 3 modelos. Cost guard pre-aggregation (G11) — rechazar si `pre_cost_estimate > profile.budget_per_turn`.
- `Council` (`2305.14325`): 2-3 debaters + 1 round (cost control). Schema SQLite `council_votes(episode_id, round, agent_id, claim, critique_of_prev)`.
- `Reflexion` (`2303.11366`) multi-model: M22 migration `reflection_episodes(episode_id ULID PK, mission_id, attempt_no ≤ 3, executor_model, reflexor_model, failure_signal, failure_trace, verbal_reflection, injected_prompt_delta, tokens_in, tokens_out, created_at)`. Multi-model: executor caro + reflexor barato (generalización NO validada en paper — anotar en RFC 22 como contribution). Anti-doom-loop: failure_signal idéntico en 2 episodios consecutivos → abort mission + escalar humano (RFC 19).
- `SelfRefine` (`2303.17651`): single-LLM generator→feedback→refiner para Coding Engine (RFC 13). Cap 2 iter; abort si `delta_lines<10`.
- `SelfDiscover` (`2402.03620`): Planning engine (RFC 12) pre-decode selecciona modules → JSON skeleton reusado. Cachea por URL+prompt_embedding.
- HUD card (deferred G13): texto "Council mode: engaged, est. cost $X" — anotar como Future Work HUD card 2.2 follow-up.
- Tests: MajorityVote stop-early 2/3 canonicalized, MoA cost guard rejection > budget, Reflexion saturation cap 3 anti-doom-loop, SelfDiscover skeleton cache hit.

### Sub-fase 2.3 — Auto-routing Classifier + MCP-aware

- `TaskTypeClassifier` opt-in (sub-modo `auto` off-by-default). Logistic regression sobre TF-IDF léxicos (regex counts de tokens: test/refactor/fix/build/architecture) + `fastembed-rs` BGE-small (384 dim) → 2-layer MLP `linfa`. **No usar BERT/DeBERTa** — demasiado para single-binary.
- **Tag pre-filter hot-path** (patrón no-obvio #3): pre-filter deployments por `capability_tags` obligatorias (vision, tools) AND `tool_capabilities` de MCP servers disponibles (G19) antes de invocar classifier.
- **Router selector via `model` field** (patrón no-obvio #1): `ModelId` admite `"gpt-4o"` o `"router-auto-0.5"` — caller emite OpenAI shape sin saber si es router-agregado o directo. Internal branch en Orchestrator runtime.
- `opencode router calibrate --task-set <bench> --strong-pct 0.5` (G15 deferred a 2.5+ pero trait shape en 2.3): offline calibration threshold per task-type. Defaults: `coding=0.116, plan=0.05, chat=0.2` (RouteLLM calibrated).
- HUD: switch `auto-router` opt-in en `Profile`.
- Tests: classifier 10 task types emission, tag pre-filter reduce candidates, router-selector string parsing, offline threshold calibration idempotente.

### Sub-fase 2.4 — Feedback Loop + `mf` experimental

- Reader que agrupa `model_invocations` por `(task_type, model_id)` → emite `AffinityRow { success_rate, p95_latency, mean_cost, n_samples }`. Alimenta `ModelRegistry.affinity` (in-memory cache invalidado por `ArcSwap::store` on update).
- `RoutingStrategy::Mf { threshold }` experimental (G14): `include_bytes!("assets/mf_weights_<provider_pair>.bin")` con pesos pre-trained (matriz <100KB). A/B testing contra `TaskTypeClassifier` log-loss: ambos emitidos ambos días durante una semana.
- `aggregation_cost_estimate()` impl real del trait definido en 2.1: usa historico `model_invocations.mean_tokens_in / mean_tokens_out` para predecir cost.
- M23 `eval_runs` table (G16 deferred a 2.5+): `opencode eval --router R --benchmark B` — cached results offline.
- Tests: affinity reader GROUP BY, mf weights loading, A/B emission, cost_estimate budget guard integration con 2.2.

---

## SECTOR F — Atribución obligatoria por módulo (RFC 28 apéndice)

| Módulo Rust | Origen | Atribución module-level prose |
|---|---|---|
| `src-tauri/src/orchestrator/provider.rs` | `async-openai` `Config` trait pattern (MIT) | "Pattern ported from 64bit/async-openai (MIT). Copyright 64bit." |
| `src-tauri/src/orchestrator/registry.rs` | LiteLLM `model_prices_*.json` (MIT) | "Cost map bundled from BerriAI/litellm model_prices_and_context_window.json (MIT). Copyright BerriAI." |
| `src-tauri/src/orchestrator/normalizer.rs` (2.0.5) | LiteLLM `litellm.utils.function_call` (MIT) | "Tool-call normalization pattern ported from BerriAI/litellm (MIT). Copyright BerriAI." |
| `src-tauri/src/orchestrator/aggregation.rs` (2.2) | Papers papers (no code port) | "Algorithms modeled on Du et al. LLM-Debate (arXiv:2305.14325), Wang et al. MoA (arXiv:2406.04692), Shinn et al. Reflexion (arXiv:2303.11366), Madaan et al. Self-Refine (arXiv:2303.17651), Li et al. AgentForest (arXiv:2402.05120)." |
| `src-tauri/src/orchestrator/classifier.rs` (2.3) | RouteLLM `mf` (Apache-2.0); HybridLLM (arXiv:2404.14618) | "Pattern modeled on lm-sys/RouteLLM (Apache-2.0) and Ding et al. HybridLLM (arXiv:2404.14618)." |

---

## SECTOR G — Métricas de éxito Phase 2 (RFC 20)

- Confidence medio de decisiones ≥ 0.75.
- Alucinaciones por cada 100 diffs ≤ 1.
- **Coste LLM por mission ≤ baseline Phase 1 × 0.6** (objective: routing ahorra ≥40% versus single-model-only, evidence RouteLLM >2×).
- Latencia routing overhead < 5ms p95 (ArcSwap reads + DashMap cooldown check).
- Re-ingresos de model crash sin perder trabajo: 100% (cadena cascade cubre todos los 429/402/403).
- Telemetría loop: `was_correct` alimentado en < 30s tras verdict del usuario (HUD approval).

---

## SECTOR H — Dependencias RFC + siguientes pasos

- **RFC 04** (este plan materializa §1-§8 enteras).
- **RFC 03 §9** (state common `status: idle|running|blocked|failed|done`).
- **RFC 19 §6.1** (override ACP `session/set_mode` → aplica antes del siguiente `tick()`).
- **RFC 21 §12** (modo_uso ask/architect/code/context — cambia prompt template + roles de modelo, NO routing directo).
- **RFC 16** (Loop feedback: `pattern_runs.was_correct` ya persiste — falta reader affinity M21→M7 join).
- **RFC 22** (Research Findings): añadir contribución "multi-model Reflexion generalization" (executor caro + reflexor barato — NO validado en paper).
- **RFC 25 §3.8** (model providers enum).
- **RFC 26** (catálogo): añadir entrada research `29 - Phase 2 model orchestrator`.

Siguiente paso operativo: actualizar RFC 20 Roadmap con 6 sub-fases detalladas + RFC 04 con IMPLEMENTED/PENDING markers + RFC 26 catálogo.
