# 20 - Roadmap

Plan de implementación por fases. Cada fase produce un sistema funcional más ambicioso que el anterior.

---

## Fase 0 — Foundation (supervivencia)
- Kernel Bus mínimo.
- Execution Journal SQLite.
- Modo Manual + Editor (sin IA todavía).
- Validation Engine Biome + tsc + Vitest + Knip.
- Skill loader abierto (YAML/Markdown).
- Context Engine mínimo (Project Map).

Entregable: editor que ya reduce alucinaciones aunque no orqueste modelos.

## Fase 1 — Single Agent IA
- Planning Engine v1 (mission → steps).
- Reasoning Engine CoT + Self-Reflection + Confidence.
- Coding Engine con diff.
- Repair Engine con conexión a Validation.
- Model Orchestrator con 5 proveedores (Claude, OpenAI, Gemini, OpenRouter free, Local Ollama).

Entregable: Modo IA funcional con un agente mono-modelo por step.

## Fase 1.5 — External Integration Surfaces (RFC 28)

Cuatro superficies de integración opt-in sobre el core de Fase 1, orden `§D → §A → §C → §B → §E → §F → §G → §H` (justificación en RFC 28 §"Orden recomendado"). Cada sub-fase es **atomicable y demo-ready standalone**; el bloque entero no rompe el single-binary (RFC 25 §11) — las dependencias nuevas son feature-gated default-off.

- **1.5a — §D AuditLog YAML export (COMPLETO, commit `db25379`).** `opencode audit --export-posting -n 50 -o ./snapshots/` genera snapshots `.posting.yaml` compatibles con `darrenburns/posting`. `serde_yaml` reutilizado; sin runtime dep nueva.
- **1.5b — §A Karpathy autoresearch loop (COMPLETO, commit `853da30`).** M13 `autoresearch_runs` + `autoresearch_candidates`; FSM pura en `journal::autoresearch`; `POST /autoresearch/cancel` axum endpoint; `<AutoresearchCard.svelte>` HUD card. Host loop LLM-driven aterriza en Phase 2 (motores).
- **1.5c — §C graphify pattern adoption (COMPLETO, commit `cddcbc2`).** M15 `mission_graph` + M16 `learning_graphs`; `petgraph`+`tree-sitter` (feature-gated `dag_mode`/`codebase-graph` default-off); `graph/{mod, traverse, ast}` modules; Planner DAG emitter; skills `graph.toml` loader; cosine retrieval; `GET /hud/graph/:id` endpoint + `<GraphView.svelte>` HUD card. Items 1–9 ✅.
- **1.5d — §B Microsoft Intelligent Terminal ACP server (COMPLETO, commits `7f8215e`–`4b4924e`).** `agent-client-protocol = "2.0"` (feature `acp-server` default off); M14 `agent_session_events`; ACP methods `initialize`/`session/new`/`session/prompt`/`session/cancel` + opt-in `session/load`/`set_mode` (mapeo RFC 19); slash commands `/opencode fix`, `/opencode restart`, `/opencode exec step`, `/opencode mission new`; `acp/listen_worker.rs` → SQLite. CLI binario `atlas` con subcommands `acp` (serve) + auxiliares. 40 ACP tests + `tools/acp-smoke.ps1` (15/15 assertions). Windows-only target; en macOS/Linux el ACP server idles sin efecto.
- **1.5e — §E Firecrawl web ingestion (✅ items 1-8, commit `c228e4a`; items 9-10 post-MVP).** `firecrawl = "2.12.1"` oficial Rust SDK (MIT) via adapter facade `src-tauri/src/firecrawl/{mod, facade, client, error}.rs` (feature `firecrawl` default off). `atlas research` CLI subcommand (gated). Tests: 38 firecrawl+research (default+firecrawl build 306+1 ignored vs default 268). `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo clippy --features firecrawl -- -D warnings`: todos limpios. Items 9 (MCP server nativo, decisión post-MVP) y 10 (graphify closure, RFC 16 §3 spec) postergados. Ver RFC 28 §E (lines 308–399) + Round 4 research en RFC 22 §11.
- **1.5f — §F Windows Toast Notifications (✅ items 1-8, commit `d6e6e23`; item 9 post-MVP).** `winrt-toast-reborn = "0.3.8"` (MIT) en `cfg(windows)` via `[target.'cfg(windows)'.dependencies]`, gated behind feature `toast` default off. `ToastDispatcher::Win` usa std::thread dedicado + mpsc channel (WinRT `ToastManager` no-Send); `ToastDispatcher::Stub` en non-Windows logea via `tracing::info!`. SQLite-driven scheduler (M17: `toast_queue` + `toast_history` tables) pollado por `ToastDriver::spawn(Arc<Mutex<Journal>>)` con 5-s idle cadencia. `on_activated`/`on_dismissed`/`on_failed` callbacks a WinRT. `register_aumid()` idempotente corre en `AppState::bootstrap()`. Tests: 32 toast+cli (default+toast build 301 vs default 270). Clippy limpio default + `--features toast`. `cargo check --features "firecrawl,toast"` OK (combo verify). Item 9 (`tools/toast-smoke.ps1`) post-MVP. Ver RFC 28 §F (lines 400–528) + Round 5 research en RFC 22 §12.
- **1.5g — §G Windows Calendar Integration (DOCUMENTADO, items 1-4 IMPLEMENTADOS, items 5-8 diferidos a Phase 2).** Bidireccional: WRITE sirve `GET /atlas-calendar.ics` desde axum HUD server con `ics = "0.5"` (RFC 5545 + RFC 7986 propiedad `REFRESH-INTERVAL`), subscribible via `webcal://`; READ consume Microsoft Graph `/me/calendarView` pollando 60s via `graph-rs-sdk = "3.0.1"` con `features = ["interactive-auth"]` (wry popup OAuth, refresh token SQLite encrypted AES-256-GCM). `BusyWindow` injectado al Planning engine. WinRT `AppointmentManager` RECHAZADO (capability `appointmentsSystem` viola RFC 25 §11). M18 migration (schema 17→18). **Items 1-4 (WRITE)**: `CalendarWriter::render`, `BusyWindowQueue`, `IcsMission`, axum route `/atlas-calendar.ics`. **Items 5-8 (READ)**: diferidos a Phase 2 (§G READ es infraestructura commodity; §H es el diferencial). `calendar` (umbrella), `calendar-ics`, `calendar-graph` features (default OFF). Ver RFC 28 §G (lines 529–686) + Round 5 en RFC 22 §12.
- **1.5h — §H Model API Reset-Window Notifications (items 1-11 IMPLEMENTADOS).** **Feature diferencial frente a competencia**: captura `resets_at` desde OmniRoute envelope error JSON (no parse upstream headers directos), persiste en `model_resets` SQLite, y dispara Toast `kind='model_ready'` desde el scheduler de §F cuando el reset cumple. `SpendLimitError` card (HUD) port de Cline PR #10207 con `Request Increase` + `Switch Provider` botones + 5-min localStorage cooldown. Retry middleware con jitter ±25 % + parse `Retry-After` + bail-out threshold 60 s (ports de Cline PRs #10963, #10141). Sin crates nuevas (chrono + tokio + serde ya presentes; `rand = "0.8"` añadida para jitter). M19 migration (schema 18→19). Items 1-10 + 11 (RFC docs) completos; items 12-13 (user docs + smoke test) pending. Ver RFC 28 §H (lines 687–837) + Round 5 en RFC 22 §12.

Entregable: Atlas OS detectable como ACP agent de primera clase por IT 0.1+ (autodetect PATH), slash commands en pane; HUD sigue siendo surface visual canonical. Toast notifications asyncronous via AUMID-registered deep-links. Calendar WRITE path (ICS feed) — bidireccional completa pendiente Phase 2. Reset-window notifica al usuario cuando el model vuelve a estar disponible — advantage competitiva. Validación manual en IT 0.1.1+ instalado (Windows) + `tools/toast-smoke.ps1`, `tools/calendar-smoke.ps1`, `tools/reset-window-smoke.ps1`.

## Fase 2 — Multi-model Orchestration (COMPLETA, sub-fases 2.0 → 2.4)

Plan refinado en `Atlas OS/research/29 - Phase 2 model orchestrator.md` (6 sub-fases atómicas, commits no PRs). Evidencia primaria: 11 papers arxiv cross-verified, LiteLLM docs, OpenRouter docs, Aider README/changelog, async-openai MIDDLEWARE, async-anthropic docs.rs, RouteLLM GitHub, docs.rs (tower/governor/arc-swap/notify/rusqlite/dashmap). Auditoría iterativa Round 3 detectó 9 gaps críticos (G1/G2/G3/G5/G8/G11/G12/G17/G18) que se incorporan en sub-fase 2.0.5 — sin ellos el cascade cross-provider y feedback loop son incorrectos.

### Sub-fase 2.0 — Foundation (Registry + Tri-model)
- `enum Provider` (variantes OpenAI/Anthropic/Gemini/VertexAI/Bedrock/Azure/Ollama/LmStudio/OpenRouter + `Custom(Arc<dyn Config>)`) en `orchestrator/provider.rs` — async-openai `Box<dyn Config>` pattern.
- M20 migration: SQLite tables `models`/`deployments`/`model_aliases`/`model_groups`.
- JSON seed `assets/model_prices_and_context_window.json` (LiteLLM MIT bundled `include_str!`) + `atlas models refresh`.
- `ArcSwap<Registry>` lock-free reads + `DashMap<DeploymentId, (Instant, u32)>` cooldown.
- `Profile` extendido: `architect_model`/`editor_model`/`weak_model` (Aider port — weak hace compactación de history, no commits).

### Sub-fase 2.0.5 — Provider Normalization Layer (gaps G1/G2/G5/G8/G17/G18)

**Sub-fase nueva tras Round 3 — sin ella cascade cross-provider y feedback loop son incorrectos.**
- `ToolCall` enum cross-provider + `normalize()`/`deserialize()` (Anthropic tool_use / OpenAI function_calling / Gemini function_call).
- `Tokenizer` trait pre-flight (`tiktoken-rs` / `tokenizers` / Anthropic `count_tokens` endpoint) → `estimate(payload)->u32` pre-routing (G2).
- Prompt cache accounting (G1): `cache_control` marker injection + `model_invocations.cache_read_input_tokens` tracking.
- Cooldown per-provider overrideable (G5): defaults `{anthropic:30s, openai:60s, ollama:5s, bedrock:1s}`. `Retry-After` header override (G17).
- Back-pressure `Arc<Semaphore>` per `ProviderId` (G18) — `max_concurrent_remote_calls` de RFC 04 §6.

### Sub-fase 2.1 — Routing Policy
- `enum RoutingStrategy` (LiteLLM 6 strategies: SimpleShuffle default, LatencyBased, UsageBasedV2, LeastBusy, CostBased, Hybrid, Custom). Per-ModelGroup override.
- M21 migration `model_invocations` (latency_ms, tokens_in/out, cost, cache_read, seed NULL, temperature, sampling_params JSON, route_taken JSON, was_correct NULL).
- 3-bucket cascade fallback (LiteLLM: `fallbacks` + `context_window_fallbacks` + `content_policy_fallbacks` + `default_fallbacks` + `max_fallbacks=5`).
- Idempotency (G12): `RequestFrame { idempotency_key Ulid, executed_tool_calls, tool_calls_complete bool }` — fallback solo si `complete`.
- HUD WS `Data Parts` con ID reconciliation (Vercel AI SDK pattern).

### Sub-fase 2.2 — Aggregation (opt-in HighStakes)
- `trait Aggregator` + `enum AggregationMode { Single, MajorityVote, MoA, Council, SelfRefine, Reflexion }`.
- `MajorityVote` (AgentForest `2402.05120`): N∈{1,3,5,9} por task difficulty, stop-early 2/3 agreement + entropy <0.5.
- `MoA` (`2406.04692`): solo `ExecutionMode::HighStakes` (RFC 19), 3×3 modelos, cost guard (G11).
- `Reflexion` (`2303.11366`) multi-model: M22 `reflection_episodes` con `executor_model` + `reflexor_model` barato. Cap 3, anti-doom-loop (RFC 19).
- `SelfRefine` (`2303.17651`) Coding Engine: cap 2 iter; abort si `delta_lines<10`.
- `SelfDiscover` (`2402.03620`) Planning Engine: skeleton JSON cacheado por prompt_embedding.
- Cost guard pre-aggregation (G11): `pre_cost_estimate > profile.budget_per_turn` → fallback a `Single`.

### Sub-fase 2.3 — Auto-routing Classifier + MCP-aware
- `TaskTypeClassifier` opt-in (sub-modo `auto` off-by-default): logistic regression (TF-IDF léxicos) + `fastembed-rs` BGE-small (384 dim) + `linfa` MLP. **No BERT/DeBERTa** (single-binary).
- Tag pre-filter hot-path: pre-filter deployments por `capability_tags` AND `tool_capabilities` de MCP servers disponibles (G19).
- Router selector via `model` field (RouteLLM `router-mf-0.116`): caller emite OpenAI shape sin saber si es router-agregado o directo.
- Threshold defaults RouteLLM: `coding=0.116, plan=0.05, chat=0.2`.

### Sub-fase 2.4 — Feedback Loop + `mf` experimental (COMPLETO, commit `302b170`)
- Reader `model_invocations` GROUP BY `(task_type, model_id)` → `AffinityRow` alimenta `ModelRegistry.affinity` vía `ArcSwap::store` ✅ — `orchestrator::affinity` (`AffinityIndex` arc-swap lock-free + `AffinityRow` + `MIN_SAMPLES=3`) + `journal::{read_affinity, upsert_affinity_rows}` + M24 mirror (`model_affinity_cache` + `task_classifier_decisions` CHECK fix, schema 23→24).
- `RoutingStrategy::Mf { threshold }` experimental (RouteLLM `2406.18665`): strong-vs-weak binary routing sobre `classifier_confidence` + `strong_ids` con fallback graceful. **Desviación documentada**: sin `include_bytes!("assets/mf_weights.bin")` pre-trained (G14) — el flavour experimental usa classifier confidence; weights pre-trained + A/B testing contra classifier log-loss postergados a 2.5+.
- `aggregation_cost_estimate()` impl real ✅: `AggregationCostContext::from_journal()` usando medias histórico `mean_tokens_in/out` — budget guard integra con 2.2.
- Brazo manual del loop ✅: CLI `atlas models refresh -n <window> --list` (`cli/commands/models.rs`). Cargo: `arc-swap = "1.7"`.
- M23 `eval_runs` (G16 deferred a 2.5+): no implementada, como estaba planificado.

Entregable: el sistema sabe **cuándo cambiar de cerebro**. KPI: coste LLM por mission ≤ baseline Phase 1 × 0.6 (evidence RouteLLM >2× savings).

## Fase 3 — Research Engine

Plan refinado en `Atlas OS/research/30 - Phase 3 research engine.md` (6 sub-fases atómicas, commits no PRs). Evidencia: audit RFC 30 (ecosistema + Jev — "System One judgments" para el weak-model pre-filter), HydraFusion (RFC 22 §13), decisiones A.1–A.4 (Context7Max fuente primaria, anydoc ingestion, grill-me gate).

### Sub-fase 3.0 — Foundation (COMPLETO, delegado a muse-spark-1.3-contributor-free)
- M25 migration `research_runs` (status/recommended via PRAGMA, idempotente)/`research_sources`/`research_consensus` (PK compuesta + CHECKs) ✅ + `ResearchRunReport` tipos canónicos (`research/report.rs`, validate + round-trip JSON/YAML) + `trait ConsensusScorer` + `enum ConsensusDimension` + `ConsensusScore` (0–100 + refs) + `combined_confidence()` (hands-on ×1.5) + `MIN_SOURCES=3` + `enum ResearchRunKind {Full,Targeted,Mega}` + `ResearchRunStatus` fail-safe. Persistencia `journal/research.rs` (create/complete/get_run, add/list_sources, save/list_consensus idempotente). 20 tests nuevos (678 total). clippy + fmt verdes.

### Sub-fase 3.1 — Docs gateway (Context7Max adapter)
- Adapter facade `research/docs_gateway.rs` (estilo firecrawl RFC 28 §E): ctx7max primaria → Context7 MCP → webfetch fallback. `atlas research docs <library> "<question>"`. Sin crate nueva.

### Sub-fase 3.2 — Document ingestion (anydoc, feature-gated)
- Crate `anydoc` (audit single-binary-safety primero) o parser mínimo propio. Feature `doc-ingest` default off. `atlas research ingest <file>`.

### Sub-fase 3.3 — Collective Engineering Intelligence (COMPLETO, delegado a muse-spark-1.3-contributor-free)
- `research/collective.rs`: pre-filtro weak-model determinista `prefilter_sources` (A.4) + `classify_source_kind`, 4 scorers (`Community`/`Enterprise`/`Academic`/`Official`, kind-relevance 0–100 + refs, `score_all`/`top_reference`), `HANDS_ON_WEIGHT=1.5`, fail-safe `FAIL_SAFE_CONFIDENCE=0.6` → `NeedingHuman` (`apply_fail_safe`), `build_report` + `ReportSections` al YAML RFC 10 §7, colectores best-effort sin crate nueva (`gh` CLI, arXiv API, docs vía 3.1). CLI `atlas research query "<pregunta>" [--source/--hands-on/--library/--gh-repo/--min-confidence]` con run persistido en M25. 17 tests nuevos + demo YAML verificada. clippy + fmt verdes.

### Sub-fase 3.4 — Hands-on + ramas de aplicación
- `research_notes` (evidencia experta firmada) + ramas Opción A/B/C con coste estimado en este código + journal_ref auditable.

### Sub-fase 3.5 — probe_feasibility + fail-safe + grill gate
- `probe_feasibility` (RFC 10 §11) + fail-safe anti-alucinación duro + grill-me como gate de confidence antes de `Plan.lock`.

Entregable: la IA decide con evidencia social y académica, no con suposición.

## Fase 4 — Swarm

Plan refinado en `Atlas OS/research/31 - Phase 4 swarm.md` (6 sub-fases atómicas, commits no PRs). Evidencia: RFC 30 (agency-agents roles con personalidad + munder-difflin office floor/mailbox), RFC 22 `28 - conductor & alt surfaces.md` (worktrees CN-001/CN-003/CN-004). Worktrees via git CLI — sin crate nueva.

### Sub-fase 4.0 — Foundation (COMPLETO, commit `0d583b0`)
- M29 migration `swarm_agents`/`agent_mailbox` ✅ + `enum Role` (10 roles RFC 05, `as_str`/`parse` round-trip) ✅ + `WorktreeManager` via git CLI (fail-safe si git no está, path-traversal protegido) ✅ + registry `Journal::{register_swarm_agent, set_swarm_agent_state, swarm_agents_for_mission}` ✅. Decisión: git CLI via `std::process::Command`, `git2` NO se añade (RFC 25 §11).

### Sub-fase 4.1 — Role presets (agency-agents port) (COMPLETO, commit `bd75864`)
- Presets con personality/processes/deliverables (`atlas-team` 10 roles, `pair-programming`, `solo-plus`) ✅ + `ModelSlot` + `Role::model_slot()` + `Role::resolve_model(&Profile)` (Aider tri-model via `effective_*`) ✅ + CLI `atlas swarm presets/start` (registra agentes con model resuelto + personality JSON) ✅.

### Sub-fase 4.2 — Pool swarm (paralelismo por rol) (COMPLETO, commit `c0b06e9`)
- `FileLockRegistry` (acquire atómico todo-o-nada, orden determinístico anti-deadlock RFC 05 §8) ✅ + `SwarmRunner` (`run_one`/`run_parallel` tokio tasks + `BackPressure::acquire` per-provider reusado del orchestrator / `run_topology` planner→research/architect→executors paralelos→reviewer→merger RFC 05 §2) ✅ + checkpoints RFC 19 §5 por agente + estados done/failed. Fallo de un agente no cancela el pool.

### Sub-fase 4.3 — Mailbox + memoria por agente (COMPLETO, commit `018e7b9`)
- `MailboxMessage` + `Journal::{send_message, inbox_for, unread_inbox_for, unread_count, mark_read}` (idempotente, first-write-wins RFC 02) ✅ + memoria por agente `swarm_agent(id)` + `agent_resume(id) -> AgentResume {agent, checkpoint}` (une el último checkpoint RFC 19 para reanudación exacta) ✅ + CLI `atlas swarm send/inbox [--unread-only] [--mark-read]` ✅.

### Sub-fase 4.4 — Auto-rebase post-merge (CN-003) (COMPLETO, commit `aa62ba5`)
- `swarm/rebase.rs` via git CLI: `rebase_worktree`/`rebase_many` (no short-circuit)/`rebase_after_merge` (descubre worktrees vivos via `manager.list()`)/`abort_rebase` ✅ + `RebaseError` tipado fail-safe (conflicto → abort + fail-safe a manual, fetch tolera "no remote") ✅.

### Sub-fase 4.5 — Swarm Console HUD (COMPLETO, commit `3f87e2e`)
- `<SwarmConsole.svelte>` floor 2D (munder-difflin) con desk por agente (rol, state pill, modelo, worktree, badge no-leídos) + drawer mailbox con formulario de envío (`postSwarmSend`) + checks button por worktree (`fetchSwarmChecks` CN-004) ✅ + proyecciones puras del tail WS (`projectSwarmAgents`/`projectSwarmInbox`/`countUnread`/`swarmStateColor`) en `hud.ts` + sección montada en `+page.svelte` ✅. Nota: test de card es compilación+estructura (no `mount` — el vitest resuelve svelte a server build; el alias requeriría config/dépendance nueva, descartado RFC 25 §11/AGENTS.md §4).

Entregable: 10 agentes cooperan en una mission. **Phase 4 COMPLETA** — M29 registry + presets + pool paralelo + mailbox + rebase + Swarm Console. KPI: latencia end-to-end UI <100ms (RFC 20); re-ingresos sin perder trabajo 100%.

## Fase 5 — Learning + Compression (COMPLETA, sub-fases 5.0 → 5.4)

Plan refinado en `Atlas OS/research/32 - Phase 5 learning + compression.md` (5 sub-fases atómicas, commits no PRs). Evidencia: RFC 30 (System One compaction — fast-jev-compaction pattern), RFC 16 §4/§5 (YAML rules + ajuste dinámico). Gestión: trabajo mecánico pesado delegado a muse-spark-1.3; judgment en el gestor.

### Sub-fase 5.0 — Foundation (COMPLETO, commit `9405ce9`)
- M30 `learned_rules` (lifecycle CHECK `draft`/`candidate`/`active`/`deprecated`)/`compaction_events` ✅ + writer/loader YAML `.opencode/rules/` (RFC 16 §4, envelope `rule:`, serde_yaml ya existente) ✅ + `Journal::{save/get/promote/deprecate/list_consultable}_rule` (idempotente, promote draft→candidate(30)→active(60)) ✅.

### Sub-fase 5.1 — Reflection Engine formal (COMPLETO, commit `b4a8bcb`)
- `reflect()` dedup por firma (`stage|error_class|rule_tag`, confianza +0.05/repetición cap 0.95) ✅ + `promote_draft_with_threshold` (PROMOTE_THRESHOLD=2, was_correct) ✅ + `should_deprecate`/`deprecate_stale` (was_blocked > was_correct) ✅ + `Journal::record_rule_feedback` ✅ + CLI `atlas learn rules/promote/deprecate` ✅.

### Sub-fase 5.2 — Compresión de Skills (COMPLETO, commit `9803298`)
- `learning/compress.rs`: Jaccard determinista (sin embeddings obligatorios) ✅ + `find_compress_proposals` (umbral 0.7, greedy keep=mayor prioridad) ✅ + `merge_manifests` (unión ordenada, verified=false) ✅ + `apply_proposals` (escribe skill.toml fusionada + marcador DEPRECATED; bundled absorbida = skip) ✅ + CLI `atlas skill compress [--threshold] [--apply]` ✅.

### Sub-fase 5.3 — System One compaction (COMPLETO, commit `fbc3896`)
- `learning/compaction.rs`: COMPACTION_THRESHOLD=100 + `summarize()` determinista (conteo por kind + headlines newest-first) ✅ + `Journal::{save_compaction_event, compacted_summary, compaction_history, mission_entry_count}` (reúsa tabla M30) ✅ + CLI `atlas learn compact/summary` ✅. Wiring `weak_model` real documentado como follow-up (RFC 32 §C).

### Sub-fase 5.4 — Ajuste dinámico de prompts (COMPLETO, commit `fbc3896`)
- `LearnedHint` + `apply_learned_hints` (match case-insensitive de `RuleWhen.pattern`, inyecta `[rule <id>] <hint>` en observations) ✅ + `run_with_profile_and_rules` (firmas existentes intactas) ✅ + `core/pipeline.rs::run_mission` carga consultable rules best-effort (degrada si Journal falla) ✅.

Entregable: el editor **mejora solo** según el uso. **Phase 5 COMPLETA** — KPI (RFC 20): skills redundantes reducidas -30% en 3 meses; errores repetitivos → reglas automáticas.

## Fase 6 — Execution Supervisor completo

Plan refinado en `Atlas OS/research/33 - Phase 6 execution supervisor.md` (3 sub-fases atómicas). Estado RFC 19 ya materializado (state machine + doom_loop + heartbeats + checkpoints); lo faltante: reanudación desde cualquier estado + observer web del Journal.

### Sub-fase 6.0 — Reanudación desde cualquier estado (COMPLETO, commit `9c3ab6a`)
- `supervisor/resume.rs`: `resume_state(journal, mission_id)` — reconstruye `SupervisorState` desde el último checkpoint (phase via `MissionPhase::parse` + caps/mode persistidos en `MissionCheckpoint`, legacy → defaults) + `continue_from()` wrapper de `tick()` ✅ + integrado en CLI `atlas resume` ✅. 7 tests.

### Sub-fase 6.1 — Observer web del Journal (COMPLETO, commits `9181d66` + cierre `6.2`)
- axum `GET /hud/journal?limit=N&offset=M&kind=K` (`hud/observer.rs`, paginación newest-first + payload íntegro + `total`, validación 4xx) ✅ + `<JournalObserver.svelte>` (payload expandible + filtro + paginación) ✅ + `fetchJournalPage` ✅ + montado en `+page.svelte` ✅. 9 tests Rust + 5 vitest.

### Sub-fase 6.2 — Cierre (COMPLETO)
- RFC 20/RFC 19 §5/§10 markers + RFC 26 índice + README status.

Entregable: sistema durable, recuperable de fallos sin scripts externos. **Phase 6 COMPLETA** — state machine + doom_loop + heartbeats + checkpoints + reanudación desde cualquier estado + observer web. KPI: re-ingresos de model crash sin perder trabajo 100% (RFC 20).

## Fase 7 — Seguridad & Compliance

Plan refinado en `Atlas OS/research/34 - Phase 7 security + compliance.md` (4 sub-fases atómicas). Evidencia RFC 18 (spec completa). Firmas: SHA-256 MVP (`sha2`/`hex` ya en deps, ed25519 diferido); sandbox levels: tipos + policy (ejecución real diferida); supply gate determinista (Socket/Snyk APIs follow-up).

### Sub-fase 7.0 — Firmas de skills (SHA-256 MVP) (COMPLETO, commit `c374322`)
- `security/signature.rs`: `skill_checksum` (SHA-256 canónico skill.toml + README, orden estable, `sha2`+`hex` ya en deps — cero crates nuevas) ✅ + `verify_skill` → `ChecksumVerdict {Ok, Mismatch, Missing}` ✅ + `.checksum` sidecar writer/reader ✅ + `install_gate` fail-safe Forbidden en `atlas skill install` (RFC 18 §5) ✅ + `catalog_checksum` (catálogo bundled estable). 7 tests. ed25519/minisign diferido (RFC 34 §C — audit RFC 25 §11 previo).

### Sub-fase 7.1 — Sandbox levels (tipos + policy) (COMPLETO, commit `c374322`)
- `security/sandbox.rs`: `SandboxLevel {None, VuOnly, Container, Wasm}` (wire `vuOnly` verbatim RFC 18 §6) ✅ + `Approval {Auto, Confirm, Forbidden}` + `SensitiveAction` (13 acciones tabla §2) + `approval_for`/`approval_for_str` (tabla §2 exacta + overrides por nivel + fail-safe Forbidden) ✅. 6 tests. Ejecución real en Docker/Podman diferida (RFC 34 §C).

### Sub-fase 7.2 — Compliance skills (OWASP/GDPR/HIPAA) (COMPLETO, commit `edb0066`)
- 3 skills bundled (engine `security`, manifests RFC 06 §1 completos): `atlas-owasp-check` (Top 10 2021), `atlas-gdpr-check` (Art 5/25/32/33), `atlas-hipaa-check` (45 CFR §160/164) — checklists accionables con leyes citadas verbatim ✅ + catálogo bundled 12→15 ✅. Test `compliance_skills_route_to_security_engine`.

### Sub-fase 7.3 — Supply-chain install gate (COMPLETO, commit `edb0066`)
- `security/supply_gate.rs`: `evaluate_package(name, manifest_json)` → `SupplyReport {Pass, Warn, Block, reasons}` — typosquat por Levenshtein ≤2 contra `KNOWN_PACKAGES` (d1=Block, d2=Warn, respeta scope `@x/`) + `preinstall/install/postinstall` = Block + env-var access = Warn + fail-safe nombre vacío = Block (sin red en MVP; Socket/Snyk APIs follow-up RFC 34 §C) ✅ + CLI `atlas security gate <name> [--manifest]` (exit ≠ 0 en Block) ✅ + stage `supply_chain` referencia el gate ✅. 11 tests.

Entregable: extensible como OpenClaw pero seguro. **Phase 7 COMPLETA** — KPI: acciones bloqueadas ≥99% antes de impacto (RFC 20).

## Fase 8 — UI v2 accesible desde cualquier dispositivo

Plan refinado en `Atlas OS/research/36 - Phase 8 UI v2 + ecosistema.md` (5 sub-fases atómicas). Evidencia RFC 35 (frecency zoxide, FTS5 context-mode, sysinfo/nvml, ratatui, axum-oidc). Crates nuevas: audit RFC 25 §11 previo; RustDesk AGPL lateral — jamás bundling.

### Sub-fase 8.0 — Frecency + FTS5 (quick wins RFC 35, sin crates)
- M31 FTS5 virtual table + `Journal::search_events` + frecency interno propio (port zoxide aging+ranking) + CLI `atlas journal --query` / `atlas swarm jump`.

### Sub-fase 8.1 — Skill Picker iluminado/grisado (RFC 17 §4)
- `skills/picker.rs` scoring determinista (priority + engine/domain match + lifecycle) + CLI `atlas skill pick` iluminada vs grisada.

### Sub-fase 8.2 — VRAM/RAM/cost monitor (IMPLEMENTADO, std-only MVP)
- Audit `sysinfo`+`nvml-wrapper` DIFERIDO (RFC 25 §11: transitive deps + peso binario pendientes) → monitor propio sin crates: `src-tauri/src/monitor/` (`HardwareSnapshot`, consts `MONITOR_RAM_WARN/CRIT_PRESSURE`, `MONITOR_COST_WARN/CRIT_USD`, `MONITOR_POLL_SECS=5`) + `BusEventKind::HardwareSnapshot` (`hardware_snapshot`) + `Journal::total_model_cost_usd()` + CLI `atlas monitor [--ram-warn/--ram-crit/--cost-warn/--cost-crit/--no-publish]` + proyección HUD (`projectHardwareSnapshot`/`monitorPressureOf` en `hud.ts`). VRAM vía `nvidia-smi` best-effort (`None` = fail-safe). Feature `hardware-monitor` (sysinfo/nvml) queda como follow-up opt-in default-off.

### Sub-fase 8.3 — Command Center web remoto (SSO/OIDC) (IMPLEMENTADO, std-only MVP)
- Audit `axum-oidc-layer`+`openidconnect` DIFERIDO (RFC 25 §11: oauth2/reqwest-blocking transitive + peso binario pendientes) → auth propia sin crates: `src-tauri/src/remote_auth/` (bearer `ATLAS_REMOTE_TOKEN`/`remote_token.txt` con `ensure/read/rotate`, `verify_bearer` xor-fold, `extract_request_token` header `Authorization` + cookie `atlas_session`, `discovery_url` OIDC shape fijo, `RemoteAccessStatus` sin filtrar el secreto) + `GET /remote/status` siempre montado (informativo; con feature `remote-ui` default-off exige bearer y responde 401) + CLI `atlas hud [--auth-status/--rotate-token]`. Postura default local-only; remoto solo sobre tunnel autenticado (Tailscale/SSH/Cloudflare, RFC 24 §16). Feature `remote-ui` (capa OIDC real) queda como follow-up opt-in default-off.

### Sub-fase 8.4 — Sister IDE-in-a-terminal (ratatui) (IMPLEMENTADO, std-only MVP)
- Audit `ratatui` DIFERIDO (RFC 25 §11: crossterm/unicode-width/widgets + peso binario pendientes; research 28 Tier-1 es terminal-kit Node, no crate Rust) → Document Model propio sin crates: `src-tauri/src/sister/` (`SisterSnapshot` desde journal tails + `HardwareSnapshot`, `SisterFeed` cap 20 sobre `BusEvent::tag`, `render_frame` puro determinista 72 cols, `ws_url` al mismo Kernel Bus WS del HUD) + CLI `atlas sister [--watch/--ticks]` + binario `atlas-tui` (feature `tui` default-off, `required-features`). Renderer alternate-screen futuro reusa el mismo shape.

### Sub-fase 8.5 — Remote-live dual-PC (RustDesk lateral, AGPL) (IMPLEMENTADO)
- RustDesk como proceso externo lateral (jamas link/bundle AGPL): `src-tauri/src/remote/` (`RemoteRole` server/client, `find_rustdesk_in_path` patron docs_gateway, `ATLAS_RUSTDESK_BIN` override, `launch_args`/`spawn_session` via `std::process::Command`) + `atlas remote status/guide/serve [--launch]/connect [--peer-id] [--launch]` + modelo Nate Gentile (PC servidor potente + PC thin cliente). Sin RustDesk → mensaje util con download link. Sin crates nuevas.

Entregable: ve el swarm desde el móvil/tablet, programa en vivo desde un PC thin accediendo a los recursos del servidor. **Phase 8 COMPLETA** — KPI: latencia end-to-end UI <100ms (RFC 20).

**Lateral 8.6 (opcional, post-Phase 10 — RFC 38 §5):** `atlas mobile` (patrón 8.5) — detecta `google/artemis` (VERIFICADO: Apache-2.0, Python/uv, 99%+ AndroidWorld) en PATH y lanza la sesión de testing Android real (HUD mobile probado en dispositivo/emulador, MCP `mobile_run_task`/`mobile_diagnose` Logcat + screenshots); jamás bundling Python RFC 25 §11.

## Fase 9 — Mejoras profundas (plan refinado: research `37`)
- Tree-sitter como lectura principal del Context Engine.
- Semgrep + CodeQL como Validation stages.
- Dependency-cruiser para límites de capas.
- LSPs propios que expongan Confidence por símbolo.
- findings schema + validator (Cloudflare, RFC 35 §3) + Laya 4º backend (RFC 35 §7.1).

Entregable: razonamiento sobre AST y reglas de seguridad profundas.

### Sub-fase 9.0 — findings.json schema + validator (Cloudflare, sin crates) (M32) (COMPLETO)
- `validation/report.rs`: `AuditReport` + `SecurityFinding` + `validate_report` (patrón findings.json Cloudflare) + puente `from_evidence` (EvidenceGate 2.5). CLI: `atlas audit validate` / `atlas audit --json`.

### Sub-fase 9.1 — Laya classifier backend (4º backend — el "System One" real) (M33) (COMPLETO)
- Audit candle stack DIFERIDO (RFC 25 §11, RFC 22 §7 AN-9.1: `rand 0.8` vs `0.9`, `tokenizers` no en deps, `axum 0.7` vs `0.8`, weights runtime, crate de 5 días/61 descargas) → feature `laya` vacío default-off + `ClassifierKind::Laya` + `LayaClassifier` std-only MVP (lexical-delegado determinista, `load` con failure-path → fallback lexical) + M33 (schema 32, CHECK + `'laya'`). Follow-ups: compaction wiring (5.3) + winnow.

### Sub-fase 9.2 — Tree-sitter AST Context Engine (M34) (COMPLETO)
- Audit tree-sitter + grammars APROBADO (RFC 22 §7 AN-9.2: reutiliza el stack vendored de `codebase-graph`, cero crates nuevas) → feature `ast` default off (alias de `codebase-graph`) + `context/ast.rs` (`AstSymbol {kind, name, file, line}` + `validate` + `presence_boost`/`confidence_for_symbol` → Skill Picker 8.1 + LSP 9.4) + M34 (schema 33, tabla `ast_symbols` + `Journal::record_ast_symbol`/`ast_symbols_for_file`). AST real con `codebase-graph`; heurístico std-only sin el feature.

### Sub-fase 9.3 — Dependency-cruiser límites de capas (M35) (COMPLETO)
- `dependency-cruiser 18.4.0` dev-dep pnpm + `.dependency-cruiser.cjs` (4 reglas FORBIDDEN: `lib/` no importa `routes/`, `stores/` no importa `components/`, `components/` no importa `routes/`, `lib/` no importa core Node-only — tests `*.test.ts` exentos) + script `pnpm arch`. Verificado: 0 violaciones (28 módulos, 40 deps), suite frontend verde. Rust: check propio defer (documentado).

### Sub-fase 9.4 — LSP Confidence por símbolo (M36) (COMPLETO)
- `lsp/confidence.rs` (`SymbolConfidence {name, file, line, confidence, present}` + `hover_for_symbol`/`diagnostic_for_symbol` + `who_owns`/`affects_where` sobre `ast_symbols`): hover/diagnostics exponen `Confidence` (fuente: Skill Picker relevance 8.1 como base + `confidence_for_symbol` 9.2 por presencia; sin DB nueva — consume M34). Tests: hover determinista, wiring relevance→confidence, failure-path sin datos (símbolo inválido, base no finita).

## Fase 10 — Plataforma abierta (plan refinado: research `39`)
- SDK público para escribir Skills.
- Marketplace con firma obligatoria.
- Remixing de Skills entre usuarios.
- Plugin de Learning social: compartir reglas verificado entre usuarios.

Entregable: Atlas OS como **plataforma** (plan refinado: research `39`).

### Sub-fase 10.0 — Skill SDK público (scaffold) (M37)
- `skills/sdk.rs`: `scaffold_skill` (template validado + manifest + skill.md stub). CLI: `atlas skill new <name>`.

### Sub-fase 10.1 — Marketplace con firma obligatoria (M38)
- `skills/marketplace.rs`: `install_skill` (firma OBLIGATORIA 7.0, sin `.checksum` rechaza) + `publish_skill`. CLI: `atlas skill install`/`publish`.

### Sub-fase 10.2 — Remixing de Skills (M39)
- `skills/remix.rs`: `fork_skill` (nueva id/versión + provenance `remixed_from` patrón graph/). CLI: `atlas skill fork`.

### Sub-fase 10.3 — Learning social (M40)
- `learning/share.rs`: `export_rules` (verificado 5.1) + `import_rules` (valida firma + dedup). CLI: `atlas learn export/import`.

---

## Métricas de éxito

| KPI | Meta |
|---|---|
| Confidence medio de decisiones | ≥ 0.75 |
| Alucinaciones por cada 100 diffs | ≤ 1 |
| Re-ingresos de model crash sin perder trabajo | 100% |
| Skills redundantes reducidas | -30% en 3 meses |
| Acciones bloqueadas | ≥99% antes de impacto |
| Latencia end-to-end UI | <100ms |

## Principio rector por fase

> Ninguna fase se libera si su Validation Engine falla. Avanzamos cuando el editor puede construirse a sí mismo de forma estable.

## Out of scope (por ahora)
- IDE distribuido multi-usuario simultáneo en el mismo archivo.
- Modelos propios entrenados desde cero.
- Hardware de inferencia dedicado.

Estos items pasan a Roadmap v2 una vez v1 esté en uso productivo.
