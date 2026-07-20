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
