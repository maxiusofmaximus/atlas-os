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

Cuatro superficies de integración opt-in sobre el core de Fase 1, orden `§D → §A → §C → §B` (justificación en RFC 28 §"Orden de implementación"). Cada sub-fase es **atomicable y demo-ready standalone**; el bloque entero no rompe el single-binary (RFC 25 §11) — las dependencias nuevas son feature-gated default-off.

- **1.5a — §D AuditLog YAML export (COMPLETO, commit `db25379`).** `opencode audit --export-posting -n 50 -o ./snapshots/` genera snapshots `.posting.yaml` compatibles con `darrenburns/posting`. `serde_yaml` reutilizado; sin runtime dep nueva.
- **1.5b — §A Karpathy autoresearch loop (COMPLETO, commit `853da30`).** M13 `autoresearch_runs` + `autoresearch_candidates`; FSM pura en `journal::autoresearch`; `POST /autoresearch/cancel` axum endpoint; `<AutoresearchCard.svelte>` HUD card. Host loop LLM-driven aterriza en Phase 2 (motores).
- **1.5c — §C graphify pattern adoption (COMPLETO, commit `cddcbc2`).** M15 `mission_graph` + M16 `learning_graphs`; `petgraph`+`tree-sitter` (feature-gated `dag_mode`/`codebase-graph` default-off); `graph/{mod, traverse, ast}` modules; Planner DAG emitter; skills `graph.toml` loader; cosine retrieval; `GET /hud/graph/:id` endpoint + `<GraphView.svelte>` HUD card. Items 1–9 ✅.
- **1.5d — §B Microsoft Intelligent Terminal ACP server (PLANIFICADO).** `agent-client-protocol = "=2.0.0"` + `sacp-tokio` (feature `acp-server` default off, sólo detectando `WT_COM_CLSID`); M14 `agent_session_events`; métodos ACP `initialize`/`session/new`/`session/prompt`/`session/cancel` + optativos `session/load`/`set_mode` (mapeo RFC 19); slash commands `/opencode fix`, `/opencode restart`, `/opencode exec step`, `/opencode mission new`; `wtcli listen --json` worker → SQLite. Windows-only target; en macOS/Linux el ACP server idles sin efecto.

Entregable: OpenCode OS detectable como ACP agent de primera clase por IT 0.1+ (autodetect PATH), slash commands en pane; HUD sigue siendo surface visual canonical. Validación manual en IT 0.1.1+ instalado (Windows).

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
