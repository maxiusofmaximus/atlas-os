# 26 - Index & Cross-References

> Mapa maestro de la especificación OpenCode OS (AEOS v1). Lista los 27 RFCs (`00`–`26`) con un resumen de una línea, dependencias (estrictas y suaves), two-way cross-references, rutas de lectura recomendada según rol, y un grafo de dependencias en ASCII para navigate. Pensado para que un recién-llegado encuentre qué RFC responder a qué pregunta, y para que cualquier modificación futura de un RFC identifique en segundos qué otros RFCs se ven afectados.

---

## 1. Catálogo completo

| # | RFC | Tamaño | Resume |
|---|---|---|---|
| 00 | [Vision](./00%20-%20Vision.md) | 3.1 KB | Tesis fundacional: OpenCode OS como plataforma para agentes, no editor con IA. Define Nombre (OpenCode OS), nombre técnico (AEOS), versión (v1.0), licencia, diferencia fundamental vs Cursor/Windsurf/Claude Code/Hermes/OpenClaw. |
| 01 | [Core Principles](./01%20-%20Core%20Principles.md) | 3.4 KB | 10 principios rectores: transparencia > magia, multi-modelo, distribución, verificabilidad, memoria persistente, auto-evolución, seguridad sandbox, of-architecture, agnostic de provider, open-source. |
| 02 | [Agent Operating System](./02%20-%20Agent%20Operating%20System.md) | 10.3 KB | Kernel: Kernel Bus, Scheduler, Capability Resolver, Política de ejecución (auto/confirm/forbidden), 4 Execution Modes × 3 Resource Modes, evento flow, SOP con 4 checkpoints, patrón at-least-once idempotente con `idempotency_key` (patrón Temporal). |
| 03 | [Engine Architecture](./03%20-%20Engine%20Architecture.md) | 8 KB | Topología de 10 motores: Context, Planning, Research, Reasoning, Coding, Validation, Repair, Learning, Model Orchestrator, Swarm. State machine formal del Execution Supervisor en §9 con DoomLoopDetector, GoalTracker, LanguageIdResolver, ProjectSymbolTable, SemanticEmbeddingIndex. |
| 04 | [Model Orchestrator](./04%20-%20Model%20Orchestrator.md) | 6.1 KB | Routing por skill graph + tier + costo. Registry de modelos con protocol, capabilities, api_keys multi-key (rotación rigurosa AionUI-style), runtime_options, capability_tags. Cascada cross-provider con budget enforcement. Lección AionUI anotada. |
| 05 | [Swarm](./05%20-%20Swarm.md) | 4.9 KB | Roles (Planner, Researcher, Architect, Backend, Frontend, Database, Security, Testing, Reviewer, Merger). Topología distribuida, asignación de modelo por rol, worktree own por subagent, pool swarm estilo Kimi, subagentes locales ociosos consumidos automáticamente. |
| 06 | [Skills](./06%20-%20Skills.md) | 5.3 KB | Skill Graph, iluminación/grisado desde Learning, skills auto-compresoras, marketplace con firma obligatoria. Resolución de conflictos. Soporta shell, TS, Rust, Python, Go. |
| 07 | [MCP](./07%20-%20MCP.md) | 3.7 KB | Bridge stdio JSON-RPC para MCPs externos. Multi-MCP con streaming y verificación de firma. |
| 08 | [CLI](./08%20-%20CLI.md) | 2.6 KB | `opencode` CLI: `mission new`, `run`, `resume`, `fork`, `steer`, `swarm` family, `research query`, `research feasibility`, `audit tail`, `audit verify-hash-chain`, `profile switch`, `mcp refresh`. |
| 09 | [Vector Knowledge](./09%20-%20Vector%20Knowledge.md) | 2.9 KB | Vector KB con tiered storage (core/archival/recall, patrón Letta MemGPT). |
| 10 | [Research Engine](./10%20-%20Research%20Engine.md) | 9.2 KB | Research pregunta en internet antes de decisiones críticas. Fuentes 11 categorías. Collective Engineering Intelligence. §11: `probe_feasibility(topic, domain)` para capacity_hallucination / unknown_tool_dependency, consume los 10 `gap_type` de `23`. |
| 11 | [Context Engine](./11%20-%20Context%20Engine.md) | 3.8 KB | Project Map, Tree-sitter, reordenación "Lost in the middle" (`23 §8 context mode`). |
| 12 | [Planning Engine](./12%20-%20Planning%20Engine.md) | 5.1 KB | Input: `MissionConsolidated` (no raw prompt). Output: Plan con milestones, strategies (TDD/strangler/big bang/incremental/pair). Anti-gate: no Coding Engine si `Plan.confidence < 0.7` o si `verdict.confidence < HIGH` sin lock. |
| 13 | [Coding Engine](./13%20-%20Coding%20Engine.md) | 3 KB | Diff single/multi-hunk, herramientas de edit, file locks. |
| 14 | [Validation Engine](./14%20-%20Validation%20Engine.md) | 3.1 KB | Stages incremental: tsc, Biome, Vitest, Knip, Semgrep optional. |
| 15 | [Repair Engine](./15%20-%20Repair%20Engine.md) | 2.2 KB | Diagnóstico automático a partir de `validation.failed`. |
| 16 | [Learning Engine](./16%20-%20Learning%20Engine.md) | 2.9 KB | Reflexión formal, extracción de reglas, `was_correct` por decision. Alimenta el affinity del Model Registry (`04`) y el iluminado del Skill Picker (`17`). Memories tiers (Letta pattern). |
| 17 | [UI](./17%20-%20UI.md) | 7.1 KB | Layout base (ModeBar + Editor + Command Center + Agent Console). Modo Manual vs IA. Mission Control referente Cursor Desktop + OpenCode sessions. Execution Mode selector 🕊/🤝/🛫/🚀. Acceso web remoto. Comments in-editor. Modo noob. Atajos `:c :p :s :r :v :o`. *Es remplazado por el HUD Mission Control (`24`), pero queda como layout base del editor de texto.* |
| 18 | [Security](./18%20-%20Security.md) | 4 KB | Sandbox levels: vuOnly/container/wasm. Firmas de skills con SHA-256 + minisign. SensibleActions. OWASP/GDPR/HIPAA skills. |
| 19 | [Execution Supervisor](./19%20-%20Execution%20Supervisor.md) | 6.3 KB | Heartbeats, checkpoints SQLite, reanudación desde cualquier estado. Política anti-infinite-loop mecánica con `doom_loop` hard-deny, `caps`, `goal_drift_decay`, `recovery`, `escape_hatch`, `audit`. State machine formal. |
| 20 | [Roadmap](./20%20-%20Roadmap.md) | 3.9 KB | Fase 0 → 10. Cada fase produce sistema funcional más ambicioso. KPIs: confidence ≥ 0.75, alucinaciones ≤ 1 / 100 diffs, latencia HUD < 100ms. |
| 21 | [Execution Modes](./21%20-%20Execution%20Modes.md) | 15.9 KB | Los 4 modos: `MANUAL_CLASSIC` (LSP IntelliSense), `HUMAN_IN_LOOP` (difure que el humano apruebe), `AUTOPILOT` (checkpoints cada N), `AUTONOMOUS` (loop con budget hard). Defaults por tipo de archivo. §12 añade eje ortogonal `modo de uso` (ask/architect/code/context). |
| 22 | [Research Findings](./22%20-%20Research%20Findings.md) | 22.8 KB | Backup documental: Hermes (Nous), AionUI (Apache 29.3k★), Cursor Cloud Agents, OpenCode Desktop, editores clásicos (VS/NetBeans/VS Code LSP), `/loop /goal`, anti-infinite-loop, 30+ orquestadores analizados (Round 2), 10 papers de Prompt Understanding, 10 lecciones UI. |
| 23 | [Prompt Understanding & Refinement](./23%20-%20Prompt%20Understanding%20&%20Refinement.md) | 17.6 KB | Pipeline de 9 pasos (capture → parse ToT → detect gaps → search similar → K STORM → auto-resolve → Self-Refine → loop → lock). 10 `gap_type` catalogados. `PublicUnderstandingVerdict` + `MissionConsolidated` tipos canónicos. Política de confidence. Comando `/refine`, skill `prompt-clarify`. Modos `ask`/`architect`/`code`/`context`. Citas académicas (Self-Refine, Reflexion, ToT, RePrompt, ODUTQA-MDC, LLM-Modulo, LLM-as-judge, Lost in the Middle, STORM, Masterman). |
| 24 | [HUD Mission Control](./24%20-%20HUD%20Mission%20Control.md) | 35.9 KB | Panel HUD. Layout maestro + 8 views (Kanban/Canvas/Outline/Timeline/Cost&Res/Health KPIs/Audit/Worktrees). Tarjeta por subagent con 15+ campos. WebSocket events tipados. Approvals queue con batch/scope/pauserule. Skill/MCP drag-drop en caliente. Audit timeline append-only hash-chained. Demos over diffs estilo Loom. Worktrees visuales. Mobile OIDC. Reporter de approve. Integra Hermes HUD + Jira/Linear + Cursor + Notion + n8n. |
| 25 | [Stack Técnico Multiplataforma](./25%20-%20Stack%20T%C3%A9cnico%20Multiplataforma.md) | 25.9 KB | Tauri 2 + Rust 1.84+ + SvelteKit 2 + SQLite/`sqlite-vec` + `fastembed-rs` + Tower-LSP + axum WebSocket + sandbox Docker/Podman/Firejail/Job-Object + OS keychain + 13 model providers HTTP (3 locales + 10 free cloud + paid tier) + `opencode` CLI Rust. Multi-OS binario único 30-45 MB. Justificación vs Electron/AionUI. Tamaños y performance budgets. |
| 26 | [Index & Cross-References](./26%20-%20Index%20&%20Cross-References.md) | este archivo | Mapa maestro. |
| 27 | [Orchestration Fundamentals](./27%20-%20Orchestration%20Fundamentals.md) | 13.5 KB | Audit comparativo vs `tmux-orchestrator`, `orca`, `herdr`, `traycer`. Extrae 6 principios fundacionales (worktree isolation, at-a-glance state, remote attach, reflexivity, BYOA, spec-first) y cataloga 8 brechas (A–H) contra los fundamentos, con priorización Phase 2+. |
| 28 | [External Tool Integration](./28%20-%20External%20Tool%20Integration.md) | 98.7 KB | 8 superficies: §A Karpathy autoresearch loop (Hill-climbing con métrica medible, M13 `autoresearch_runs`); §B Microsoft Intelligent Terminal ACP server (M14 `agent_session_events`, slash commands `/opencode fix` `/restart`); §C graphify pattern adoption (M15 `mission_graph` + `petgraph`+`tree-sitter`, `<GraphView>` HUD, skills como graph templates, Planner DAG); §D AuditLog YAML-on-disk export formato posting; §E Firecrawl: web ingestion polyfacética (adapter facade, post-graphify); §F Windows Toast Notifications (AUMID + Start Menu shortcut via `winrt-toast-reborn`, `tokio::sleep_until` SQLite-driven scheduler, `on_activated` deep-link); §G Windows Calendar (`.ics` feed via axum HUD `GET /opencode-calendar.ics` + Microsoft Graph `/me/calendarView` reader via `graph-rs-sdk` `interactive-auth`, refresh token SQLite encrypted); §H Model API reset-window notifications (OmniRoute envelope parsing, `SpendLimitError` card port de Cline PR #10207, `model_ready` Toast — feature diferencial frente a competencia). Fase 1.5 orden §D→§A→§C→§B→§E→§F→§G→§H. **Status Phase 1.5: §D ✅ (`db25379`), §A ✅ (`853da30`), §C ✅ (`cddcbc2`), §B ✅ (`7f8215e`–`4b4924e` items 1-8 todos ✅), §E ✅ items 1-8 (`c228e4a` — adapter facade + CLI research subcommand, items 9-10 post-MVP), §F ✅ items 1-8 (`d6e6e23` — Toast queue + driver + CLI subcommand + M17 schema + AppState integration, item 9 smoke post-MVP), §G ✅ items 1-4 (`8d26528` — WRITE path; items 5-8 READ diferidos a Phase 2), §H ✅ items 1-11 (M19 + `SpendLimitError`/`ResetKind`/`parse_omniroute`/`RetryPolicy`/`handle_spend_limit_error`/`cards.rs`/`SpendLimitErrorCard.svelte`/`ModelReadyCard.svelte`/`Profile` bail-out + backup; items 12-13 pending docs/smoke).** Research internos en `OpenCode OS/research/`. |

**Total: 29 RFCs, ~272 KB** de especificación.

---

## 2. Grafo de dependencias estrictas

```
                                    00 Vision
                                       │
                                       ▼
                                    01 Principles
                                       │
                            ┌──────────┼──────────┐
                            ▼          ▼          ▼
                         18 Security   02 AOS     20 Roadmap
                                       │
                                       ▼
                                    03 Engines ─────────┐
              ┌────────────────────┬─────────────────┐  │
              ▼                    ▼                 ▼  ▼
          11 Context            04 Model Orch        14 Validation
              │                    │                    │
              │                    ▼                    │
              │                 05 Swarm                 │
              │                    │                    │
              ▼                    ▼                    ▼
          12 Planning ◀──── 23 Prompt Understanding  15 Repair
              │                                           │
              ▼                                           ▼
          10 Research ──▶ 13 Coding ◀────────────── 16 Learning
              │
              ▼
       Vector KB ─────▶ 09 Vector Knowledge
                            │
                            ▼
                       06 Skills
                            │
                            ▼
                       07 MCP
                            │
                            ▼
                       08 CLI
                            │
                            ▼
                       17 UI ──────▶ 24 HUD ──▶ 25 Stack
                            │           │           │
                            └─► 19 Execution Superv ◄┘
                            │
                  └─► 21 Execution Modes
                            │
                            └─► 22 Research Findings (backup)
```

### 2.1 Dependencias estrictas (`must exist before`)

| RFC | Depende estrictamente de |
|---|---|
| 02 | 03, 18, 19, 23, 24, 25 |
| 03 | 09, 19 |
| 04 | — (puede leerse aislado) |
| 05 | 03, 04, 24 |
| 06 | 14, 16, 18 |
| 07 | 18 |
| 08 | 02, 10, 18, 23, 25 |
| 09 | — (puede leerse aislado) |
| 10 | 02, 11, 23 (vía `probe_feasibility`) |
| 11 | 03, 09 |
| 12 | 02, 10, 23 |
| 13 | 03, 14 |
| 14 | 03 |
| 15 | 03, 14 |
| 16 | 03, 14, 05 |
| 17 | 02, 21, 24 |
| 18 | 01 |
| 19 | 02, 03, 21 |
| 20 | 02 (implícitamente todos) |
| 21 | 02, 17, 19, 23, 24 |
| 22 | — (es backup documental) |
| 23 | 02, 10, 12, 14, 19, 21, 24 |
| 24 | 02, 05, 10, 12, 16, 17, 18, 19, 21, 23, 25 |
| 25 | 02, 05, 07, 08, 09, 10, 11, 17, 18, 19, 23, 24 |
| 26 | todos |

### 2.2 Dependencias suaves (referencia no estricta)

| RFC | Referencia suave (no bloqueante) |
|---|---|
| 04 | 06 (sugiere skills), 22 (lecciones AionUI) |
| 05 | 06, 07, 16, 22, 24 |
| 06 | 08 (CLI instala skills) |
| 08 | 23 (comando `/refine`) |
| 11 | 03, 25 (Tower-LSP) |
| 16 | 09 (Vector KB persiste reglas) |
| 17 | 02 (modos), 21 (selector), 23 (mission panel), 24 (HUD reemplazo) |
| 19 | 21 (modo AUTONOMOUS), 25 (SQLite persistencia) |
| 20 | todos (es el plan de implementación) |
| 22 | todos (es el respaldo documental) |

---

## 3. Two-way cross-references

Tabla bidireccional. Si el RFC A referencia al RFC B, B también aparece listado en A. Útil para rastrear el impacto de un cambio.

| RFC A | Referencias hacia | RFC B |
|---|---|---|
| 02 | → | 03 |
| 02 | → | 05 |
| 02 | → | 18 |
| 02 | → | 21 |
| 02 | → | 23 |
| 02 | → | 24 |
| 02 | → | 25 |
| 03 | → | 19 |
| 03 | → | 09 |
| 03 | → | 02 |
| 04 | → | 22 |
| 04 | → | 06 |
| 04 | → | 16 |
| 05 | → | 03 |
| 05 | → | 04 |
| 05 | → | 24 |
| 06 | → | 14 |
| 06 | → | 16 |
| 06 | → | 18 |
| 07 | → | 18 |
| 08 | → | 02 |
| 08 | → | 23 |
| 10 | → | 02 |
| 10 | → | 11 |
| 10 | → | 23 |
| 11 | → | 03 |
| 11 | → | 09 |
| 11 | → | 25 |
| 12 | → | 02 |
| 12 | → | 10 |
| 12 | → | 23 |
| 13 | → | 03 |
| 13 | → | 14 |
| 14 | → | 03 |
| 15 | → | 03 |
| 15 | → | 14 |
| 16 | → | 03 |
| 16 | → | 05 |
| 16 | → | 14 |
| 17 | → | 02 |
| 17 | → | 21 |
| 17 | → | 24 |
| 19 | → | 02 |
| 19 | → | 21 |
| 19 | → | 25 |
| 20 | → | 02 |
| 21 | → | 02 |
| 21 | → | 17 |
| 21 | → | 19 |
| 21 | → | 23 |
| 21 | → | 24 |
| 22 | → | (todos los referenciados en §1, §8, §9) |
| 23 | → | 02 |
| 23 | → | 10 |
| 23 | → | 12 |
| 23 | → | 14 |
| 23 | → | 19 |
| 23 | → | 21 |
| 23 | → | 24 |
| 24 | → | 02 |
| 24 | → | 05 |
| 24 | → | 10 |
| 24 | → | 12 |
| 24 | → | 16 |
| 24 | → | 17 |
| 24 | → | 18 |
| 24 | → | 19 |
| 24 | → | 21 |
| 24 | → | 23 |
| 24 | → | 25 |
| 25 | → | 02 |
| 25 | → | 05 |
| 25 | → | 07 |
| 25 | → | 08 |
| 25 | → | 09 |
| 25 | → | 10 |
| 25 | → | 11 |
| 25 | → | 17 |
| 25 | → | 18 |
| 25 | → | 19 |
| 25 | → | 22 |
| 25 | → | 23 |
| 25 | → | 24 |
| 28 | → | 04 |
| 28 | → | 06 |
| 28 | → | 19 |
| 28 | → | 20 |
| 28 | → | 22 |
| 28 | → | 24 |
| 28 | → | 25 |

---

## 4. Rutas de lectura por rol

Cada rol debe leer los RFCs en este orden recomendado:

### 4.1 Arquitecto / Tech Lead
Lectura completa, en profundidad: `00 → 01 → 02 → 03 → 04 → 05 → 12 → 23 → 24 → 19 → 21 → 20`.

### 4.2 Ingeniero Backend (model orchestration)
`02 (§3 Kernel, §3.1 SOP) → 03 → 04 → 25 → 08` para levantar el core Rust.

### 4.3 Ingeniero Frontend (HUD)
`24 → 17 → 25 §3.3 (SvelteKit) → 02 §3.1.1 (eventos hud.*)` para levantar el HUD Mission Control.

### 4.4 DevSecOps / SRE
`01 → 18 → 19 → 25 §3.7 (Sandbox) → 02 §6 (Casos de fallo)`.

### 4.5 Investigador
`22 (todo el documento) → 10 → 23 → 03 §9 (state machine) → 19`.

### 4.6 Product manager
`00 → 20 → 21 → 02 §5 (Modes matrix) → 24 §1 (Layout)`.

### 4.7 Recién-llegado general
`00 → 02 → 20 → 21 → 24 → 25` (ruiseñor + tesis: por qué sistema, cómo funciona, cuál es el plan, qué modos, cómo se ve, sobre qué stack). En segundo pase, `23 → 19 → 05 → 04`.

---

## 5. Lista de conceptos y su RFC

Para ubicar dónde definir/clavar un concepto sin buscar desde cero.

| Concepto | RFC |
|---|---|
| `Agent Operating System` | 02 |
| `ACP server frontends the Orchestrator` (CLI / HUD / ACP trio) | 04 §9; 28 §B item 6 |
| `Approvals queue` pauserule | 24 §5 |
| `Architecture Memory` | 02 §3.4 |
| `Audit timeline hash chain` | 24 §10 |
| `Audit YAML export (posting-format)` | 24 §10.1; 28 §D |
| `Autoresearch loop` (Karpathy hill-climbing) | 19 §11; 28 §A |
| `Autoresearch cancel HUD endpoint` | 28 §A (Phase 1.5b §A-3) |
| `BgeBaseEnV15` embeddings | 25 §3.5 |
| `Capabilities` del Registry | 04 §3 |
| `Capability Resolver` | 02 §3.3 |
| `Card types (agent/autoresearch/graph)` | 24 §3.3; 28 §A (autoresearch live), 28 §C (graph live) |
| `Checkpoints SOP` | 02 §3.1.1 |
| `Clarification Questions` (STORM) | 23 §5 |
| `Command Center` (legacy) | 17 §1 (remplazado por HUD) |
| `Compaction` | 21 §4.4 |
| `Confidence Score` | 03 |
| `Confidence thresholds (HIGH/MED/LOW/BLOCK)` | 23 §5 |
| `Consolidated mission` | 23 §4 |
| `Context Engine` | 11 |
| `Cosine vs 内积` | 09 |
| `Cross-provider cascade` | 04 |
| `Cube mode` (CodeMirror) | 25 §3.3 |
| `Demos over diffs` | 24 §9 |
| `DoomLoopDetector` | 19; 03 §9.1; 21 §5.3 |
| `dag_mode` feature flag (Planner DAG + State DAG + skills graph templates) | 12 §3.1; 19 §6.1.1; 23 §7.3; 28 §C |
| `Edge-case tool` feasibility probe | 10 §11 |
| `Execution Modes` (4) | 21 |
| `Execution Journal` | 02 §3.4; 19 |
| `Fastembed-rs` | 25 §3.5 |
| `Firecrawl` (web ingestion polyfacética, adapter facade) | 28 §E; 22 §11 (Round 4) |
| `fork` (subagent) | 24 §3.2 |
| `Free providers` | 25 §3.8; 21 §2.1 |
| `Gap types` catalog | 23 §1 |
| `GoalTracker` / `goal_drift` | 19; 03 §9 |
| `Hash-chained audit` | 24 §10 |
| `Health KPIs` | 24 §7 |
| `Hermes HUD` | 24 §0, §4.3 (cite); 22 §1 |
| `HUD Mission Control` | 24 |
| `Idempotency key` | 02 §3.1.2 |
| `Job Object` sandbox Windows | 25 §3.7 |
| `Kernel Bus` | 02 §3.1 |
| `LanguageIdResolver` | 03 §9.3; 21 §2.2 |
| `Launcher CLI` (`opencode`) | 08; 25 §3.9 |
| `Learning graphs` (cosine retrieval of past successful `mission_graph`) | 16 §3 "Por grafo de misión exitoso"; 28 §C |
| `Local models` (Ollama/LM Studio/llama-server) | 25 §3.8 |
| `Lost in the Middle` | 11 (reordenar); 23 §1 C8 |
| `MCP bridge stdio` | 07; 25 §5 |
| `Memory tiers` (Letta) | 09; 16 |
| `Mission Consolidated` | 23 §4; 12 §2 |
| `Mission graph` (graphify pattern) | 28 §C (GR-001/003/004/006/007/009) |
| `MissionGraph::traverse` (`shortest_path`, `god_nodes`, `community_partition`) | 28 §C; 28 §C item 3 (Phase 1.5c) |
| `AST extractor` (`tree-sitter` + heuristic Svelte line-classifier) | 28 §C (GR-006); `src-tauri/src/graph/ast.rs` |
| `Mobile remote access` | 24 §16 |
| `Multi-key rotation` | 04 §3.1; 22 §2 |
| `N8n canvas stile reasoning trail` | 24 §13 |
| `Noob mode` | 17 §11 |
| `opencode research` (CLI subcommand, gated `firecrawl` feature) | 28 §E; 08 (surface) |
| `Outline view Notion` | 24 §14 |
| `Pauserule` | 24 §5 |
| `Plan` (estrutura) | 12 §3 |
| `Planning mode (architect/code/ask/context)` | 23 §8; 21 §12 |
| `Pool swarm` | 05 §5 |
| `Posting format` (audit export) | 24 §10.1; 28 §D |
| `AUMID registration` (Start Menu shortcut for non-MSIX Toast activation) | 28 §F.2 |
| `Toast queue` (SQLite-driven scheduler, survives crashes) | 28 §F.3 |
| `Toast activation deep-link` (`opencode://mission/{id}/...`) | 28 §F.4 |
| `Toast history` (append-only audit + dedupe) | 28 §F.5 |
| `Calendar ICS feed` (axum HUD `GET /opencode-calendar.ics`) | 28 §G.2 |
| `Microsoft Graph /me/calendarView` reader (`graph-rs-sdk`) | 28 §G.3; 22 §12 (Round 5) |
| `BusyWindow` (`AppState.context_busy_windows`, Planning engine hook) | 28 §G.4; 12 §3 |
| `OmniRoute` (free MIT AI gateway, OpenAI-compatible) | 28 §H.2; 22 §12 (Round 5) |
| `SpendLimitError` card (port de Cline PR #10207) | 28 §H.3; 22 §12 (Round 5); 24 §3.3 |
| `ModelReady` Toast (`kind='model_ready'`, feature diferencial) | 28 §H.4; 22 §12 (Round 5) |
| `Retry policy` (jitter ±25%, exponential, bail-out threshold) | 28 §H.5; 04 §9 addendum |
| `Bail-out threshold` (`Profile::bail_out_threshold_secs`, default 60s) | 28 §H.5; 06 (profile schema) |
| `Backup profile` (`Profile::backup_profile_id`, auto-switch on spend-limit) | 28 §H.3; 06 (profile schema) |
| `WinRT AppointmentManager rejected` (capability `appointmentsSystem`) | 28 §G.1; 22 §12 (Round 5) |
| `ProjectSymbolTable` | 03 §9.4 |
| `probe_feasibility` | 10 §11; 23 §2.2 |
| `Prompt Understanding Pipeline` | 23 §2 |
| `PublicUnderstandingVerdict` | 23 §3 |
| `Reasoning trails` (n8n-stile) | 24 §13 |
| `Reflexion` | 16; 23 §2.1 |
| `Resource Mode` (local/free/mixto) | 02 §5.2; 21 §10 |
| `Roadmap` (fases) | 20 |
| `Sandbox` levels | 18; 25 §3.7 |
| `Scope approval` | 24 §5 |
| `Self-Refine` | 23 §2.1 |
| `Semantic Embedding Index` | 03 §9.5 |
| `SenseActions / policy` | 18 §2; 02 §3.5 |
| `Single source of truth` (SQLite Journal) | 02 §3.4 |
| `Skill Graph` | 06 |
| `Skill Picker` iluminado | 17 §4 |
| `Skill refresh en caliente` | 24 §8.1 |
| `Skills as graph templates` (`graph.toml`, 4th skill file) | 23 §7.3; 28 §C |
| `State DAG` (RFC 19 supervisor as DAG) | 19 §6.1.1; 28 §C |
| `session/set_mode` override (ACP → SupervisorState) | 19 §6.1.2; 28 §B item 6 |
| `SvelteKit CSR` | 25 §3.3 |
| `sqlite-vec` | 25 §3.4; 09 |
| `Stack technical` | 25 |
| `Steer` (en caliente) | 05 §7; 24 §3.2 |
| `Swarm roles` | 05 §1 |
| `Tauri 2` | 25 §3.1 |
| `Team Mode` (AionUI) | 22 §2 (cited); 05 |
| `Tiered memory` | 09; 16 |
| `Tower-LSP` | 25 §3.6 |
| `Worktree own per subagent` | 05 §3; 24 §12; 25 §3.2 |
| `Worktrees view` | 24 §12 |
| `Zag Nano Stores` | 25 §3.3 |

---

## 6. Reglas para editar estos RFCs

1. Al editar cualquier RFC, **anotar el cambio en `22 - Research Findings.md` §6/§8.4** si el cambio se origina en investigación. Si no, dejarlo documentado en el propio RFC, sección "Estado".
2. Si se añade o renombrar un concepto referenciado desde otros RFCs, **actualizar la tabla del §5 de este RFC `26`**.
3. Si se añade/borra una dependencia estricta, **actualizar §2.1** y rehacer el grafo ASCII del §2.
4. Si se añade un RFC nuevo (p.ej. `27 - Something.md`), **anotarlo en §1 catálogo, §2 dependencias, §3 cross-references, §4 ruta de lectura, §5 concepts**.
5. Mantener **cada RFC autónomo** — un humano que abre solo ese archivo debe poder entenderlo sin otro. La dependencia en la cabecera enumera solo "qué RFCs tienen algo que añadir sobre este tema", no "qué RFCs son prerrequisito intelectual".
6. **No romper linkage**: cualquier referencia tipo \`nn - Title.md\` debe resolver a un archivo existente. Verificado en el §3.

---

## 7. Conversión futura mkdocs (out-of-scope)

Para un futuro si se quiere servir el RFC como sitio navegable:

- Cada RFC se vuelve una página.
- Mkdocs material permite Mermaid diagram (grafo de dependencias §2).
- Search bar.
- Tags por dominio (Backend, Frontend, Security, …).

Mientras tanto, este archivo `26 - Index & Cross-References.md` sirve como índice navegable.

---

## 8. Estado

- Status: Draft v1
- Depends on: todos los demás RFCs
- Realiza: compendio y map navigateable. En una single página, un recién-llegado puede encontrar qué RFC responde su pregunta.
