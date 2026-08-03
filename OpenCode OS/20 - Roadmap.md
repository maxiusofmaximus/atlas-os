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
- **1.5d — §B Microsoft Intelligent Terminal ACP server (COMPLETO, commits `7f8215e`–`4b4924e`).** `agent-client-protocol = "2.0"` (feature `acp-server` default off); M14 `agent_session_events`; ACP methods `initialize`/`session/new`/`session/prompt`/`session/cancel` + opt-in `session/load`/`set_mode` (mapeo RFC 19); slash commands `/opencode fix`, `/opencode restart`, `/opencode exec step`, `/opencode mission new`; `acp/listen_worker.rs` → SQLite. CLI binario `opencode` con subcommands `acp` (serve) + auxiliares. 40 ACP tests + `tools/acp-smoke.ps1` (15/15 assertions). Windows-only target; en macOS/Linux el ACP server idles sin efecto.
- **1.5e — §E Firecrawl web ingestion (✅ items 1-8, commit `c228e4a`; items 9-10 post-MVP).** `firecrawl = "2.12.1"` oficial Rust SDK (MIT) via adapter facade `src-tauri/src/firecrawl/{mod, facade, client, error}.rs` (feature `firecrawl` default off). `opencode research` CLI subcommand (gated). Tests: 38 firecrawl+research (default+firecrawl build 306+1 ignored vs default 268). `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo clippy --features firecrawl -- -D warnings`: todos limpios. Items 9 (MCP server nativo, decisión post-MVP) y 10 (graphify closure, RFC 16 §3 spec) postergados. Ver RFC 28 §E (lines 308–399) + Round 4 research en RFC 22 §11.
- **1.5f — §F Windows Toast Notifications (✅ items 1-8, commit `d6e6e23`; item 9 post-MVP).** `winrt-toast-reborn = "0.3.8"` (MIT) en `cfg(windows)` via `[target.'cfg(windows)'.dependencies]`, gated behind feature `toast` default off. `ToastDispatcher::Win` usa std::thread dedicado + mpsc channel (WinRT `ToastManager` no-Send); `ToastDispatcher::Stub` en non-Windows logea via `tracing::info!`. SQLite-driven scheduler (M17: `toast_queue` + `toast_history` tables) pollado por `ToastDriver::spawn(Arc<Mutex<Journal>>)` con 5-s idle cadencia. `on_activated`/`on_dismissed`/`on_failed` callbacks a WinRT. `register_aumid()` idempotente corre en `AppState::bootstrap()`. Tests: 32 toast+cli (default+toast build 301 vs default 270). Clippy limpio default + `--features toast`. `cargo check --features "firecrawl,toast"` OK (combo verify). Item 9 (`tools/toast-smoke.ps1`) post-MVP. Ver RFC 28 §F (lines 400–528) + Round 5 research en RFC 22 §12.
- **1.5g — §G Windows Calendar Integration (DOCUMENTADO, items 1-4 IMPLEMENTADOS, items 5-8 diferidos a Phase 2).** Bidireccional: WRITE sirve `GET /opencode-calendar.ics` desde axum HUD server con `ics = "0.5"` (RFC 5545 + RFC 7986 propiedad `REFRESH-INTERVAL`), subscribible via `webcal://`; READ consume Microsoft Graph `/me/calendarView` pollando 60s via `graph-rs-sdk = "3.0.1"` con `features = ["interactive-auth"]` (wry popup OAuth, refresh token SQLite encrypted AES-256-GCM). `BusyWindow` injectado al Planning engine. WinRT `AppointmentManager` RECHAZADO (capability `appointmentsSystem` viola RFC 25 §11). M18 migration (schema 17→18). **Items 1-4 (WRITE)**: `CalendarWriter::render`, `BusyWindowQueue`, `IcsMission`, axum route `/opencode-calendar.ics`. **Items 5-8 (READ)**: diferidos a Phase 2 (§G READ es infraestructura commodity; §H es el diferencial). `calendar` (umbrella), `calendar-ics`, `calendar-graph` features (default OFF). Ver RFC 28 §G (lines 529–686) + Round 5 en RFC 22 §12.
- **1.5h — §H Model API Reset-Window Notifications (items 1-11 IMPLEMENTADOS).** **Feature diferencial frente a competencia**: captura `resets_at` desde OmniRoute envelope error JSON (no parse upstream headers directos), persiste en `model_resets` SQLite, y dispara Toast `kind='model_ready'` desde el scheduler de §F cuando el reset cumple. `SpendLimitError` card (HUD) port de Cline PR #10207 con `Request Increase` + `Switch Provider` botones + 5-min localStorage cooldown. Retry middleware con jitter ±25 % + parse `Retry-After` + bail-out threshold 60 s (ports de Cline PRs #10963, #10141). Sin crates nuevas (chrono + tokio + serde ya presentes; `rand = "0.8"` añadida para jitter). M19 migration (schema 18→19). Items 1-10 + 11 (RFC docs) completos; items 12-13 (user docs + smoke test) pending. Ver RFC 28 §H (lines 687–837) + Round 5 en RFC 22 §12.

Entregable: OpenCode OS detectable como ACP agent de primera clase por IT 0.1+ (autodetect PATH), slash commands en pane; HUD sigue siendo surface visual canonical. Toast notifications asyncronous via AUMID-registered deep-links. Calendar WRITE path (ICS feed) — bidireccional completa pendiente Phase 2. Reset-window notifica al usuario cuando el model vuelve a estar disponible — advantage competitiva. Validación manual en IT 0.1.1+ instalado (Windows) + `tools/toast-smoke.ps1`, `tools/calendar-smoke.ps1`, `tools/reset-window-smoke.ps1`.

## Fase 2 — Multi-model Orchestration
- Model Orchestrator completo (votación + debate + fail-over).
- Sub-modos local / free / mixto.
- Telemetría del Learning Engine para afinidad de modelos.
- Capability Resolver (lenguaje/framework).

Entregable: el sistema sabe **cuándo cambiar de cerebro**.

## Fase 3 — Research Engine
- Fuentes: GitHub, arXiv, SO, blogs, docs vía Context7 MCP.
- Collective Engineering Intelligence.
- Cache Vector KB.
- Output reports referenciados en el Journal.

Entregable: la IA decide con evidencia social y académica, no con suposición.

## Fase 4 — Swarm
- Roles: Planner, Researcher, Architect, Backend, Frontend, DB, Security, Testing, Reviewer, Merger.
- Pool swarm estilo Kimi (paralelismo por rol).
- Worktrees Git por agente.
- Locks de archivos.
- Swarm Console en la UI.

Entregable: 10 agentes cooperan en una mission.

## Fase 5 — Learning + Compression
- Reflection Engine formal (de errores → reglas).
- Compresión de Skills.
- Auto-reglas `.opencode/rules/`.
- Ajuste dinámico de prompts.

Entregable: el editor **mejora solo** según el uso.

## Fase 6 — Execution Supervisor completo
- Heartbeats y checkpoints en SQLite.
- Reanudación desde cualquier estado.
- Anti-bucles.
- Observer web del Journal.

Entregable: sistema durable, recuperable de fallos sin scripts externos.

## Fase 7 — Seguridad & Compliance
- Sandbox levels: vuOnly, container, wasm (futuro).
- Firmas de skills cerradas.
- Socket + Snyk integrados en cada install.
- OWASP / GDPR/ HIPAA skills.

Entregable: extensible como OpenClaw pero seguro.

## Fase 8 — UI v2 accesible desde cualquier dispositivo
- Command Center web remoto (SSO/OIDC).
- Agent Console en vivo.
- Skill Picker iluminado/grisado.
- VRAM/RAM/cost monitor.

Entregable: ve el swarm desde el móvil/tablet.

## Fase 9 — Mejoras profundas
- Tree-sitter como lectura principal del Context Engine.
- Semgrep + CodeQL como Validation stages.
- Dependency-cruiser para límites de capas.
- LSPs propios que expongan Confidence por símbolo.

Entregable: razonamiento sobre AST y reglas de seguridad profundas.

## Fase 10 — Plataforma abierta
- SDK público para escribir Skills.
- Marketplace con firma obligatoria.
- Remixing de Skills entre usuarios.
- Plugin de Learning social: compartir reglas verificado entre usuarios.

Entregable: OpenCode OS como **plataforma**.

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
