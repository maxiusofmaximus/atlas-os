# 22 - Research Findings (Hermes, AionUI, Cursor, editores clásicos y modos de loop)

> Documento de respaldo con los hallazgos de la investigación exhaustiva pedida por el usuario. Citamos fuentes primarias y los cambios que se hicieron sobre los docs `02`, `03`, `04`, `17`, `19` y `21` como consecuencia.

---

## 1. Hermes Agent

- **Origen**: Nous Research (los entrenadores de los modelos Hermes / Nomos / Psyche). Repo `github.com/NousResearch/hermes-agent`, licencia **MIT**. Diferencia con Claude Code (copiloto de terminal atado a Anthropic) y OpenCode (CLI agéntico): Hermes es un **agente autónomo persistente** que "vive donde tú lo pongas" — un VPS de 5$, un cluster GPU, Modal o Daytona.
- **Panel de administración**:
  - Nativo: CLI/TUI `hermes` con `/background <prompt>` para subagentes aislados (sesión propia, sin conocimiento de la principal), `/busy {queue|steer|interrupt}` para interactuar mientras trabaja, status bar con modelo, tokens, **coste $ de sesión**, contadores de compresiones de contexto y de background tasks activas (▶ N), badge ⚠ YOLO si está en auto-aprobación.
  - vía AionUi: tratado como uno de los 20+ backends CLI compatibles; hereda toda la capa GUI de AionUi (Desktop + WebUI + Mobile + Team Mode).
- **Pool swarm de Kimi K2.7**: no se pudo verificar documentación primaria. Lo que sí está documentado es soporte multi-modelo + delegación con subagentes aislados. Kimi CLI / Moonshot figuran como proveedor oficial de AionUi.
- **vs OpenClaw**: Hermes aporta **sandboxing real** (6 backends: local, Docker, SSH, Daytona, Singularity, Modal; root read-only; capabilities droppadas; namespace isolation) mientras que OpenClaw es criticado por ejecutar código arbitrario sin sandbox por defecto.
- **"Senior engineer 24/7"**: lo logra con (a) vida en VPS idle-casi-gratis, (b) cron en lenguaje natural, (c) gateway multi-canal (Telegram/Discord/Slack/WhatsApp/Teams/Signal/Matrix/Email), (d) memoria FTS5 cross-session + summarización + Honcho user modeling, (e) auto-skills.
- **Lecciones absorbidas en OpenCode OS**:
  - 19 - Execution Supervisor: status bar con coste y compresiones (ya estaba como telemetría; reforzado ahora).
  - 05 - Swarm: subagents `background` y patrones `queue|steer|interrupt` se incorporan al Swimming de roles.
  - 18 - Security: el principio "sandbox por defecto" de Hermes se confirma; ya estaba en nuestro doc.

## 2. AionUI — arquitectura de orquestación

- Repo: `iOfficeAI/AionUi` (Apache-2.0, 29.3k★ GitHub, v2.1.28 en julio 2026).
- App de escritorio **Electron** multiplataforma. No es un router smart de prompts a modelos: **la selección de modelo es manual por conversación o por assistant preset**. Eso convierte nuestra propuesta de routing dinámico automático en una **invención diferencial real**.
- **Layered architecture** (Main process alto-privilegio + Renderer UI + backend Rust bundled `aioncore`/Aionrs).
- **Provider pool**: 30+ proveedores directos (Gemini, Anthropic, OpenAI, AWS Bedrock, NewAPI/One API, OpenRouter, Ollama, LM Studio, DeepSeek, Qwen, Kimi, plus PRC ones). Llamadas **directas desde el cliente**, no proxyed via AionUI.
- **Multi-key rotation**: `ApiKeyManager` con arranque random + blacklist terminable + env-var sync — incorporado a nuestro `/04 §3`.
- **Team Mode (mult-agente)**: Leader + Teammates + Mailbox async + TaskManager + **TeamMcpServer TCP** (servidor MCP propio que expone tools `team_*` a los agentes) + **XML Fallback Adapter** para agentes sin MCP — ambos incorporados a nuestro `/05 §4` como mecanismo de compatibilidad.
- **State machine formal por Teammate**: `pending → idle → active → completed → failed` — incorporado a `/05 §8 Límites`.
- **Dynamic scaling**: añadir/eliminar Teammates en caliente — incorporado a `/05 §5` (Pool swim estilo Kimi).
- **Cron para equipo**: programar un swarm entero en cron.
- **Scheduled tasks 24/7**: keep-awake del OS, modos `continue in existing conversation` o `create new conversation each time`.
- **Canal remoto**: Telegram/Lark/DingTalk/WeChat/Slack/Discord para accionar agentes desde el móvil.
- **Skills marketplace**: https://skills.aionui.com.
- **Lecciones absorbidas en `/04 - Model Orchestrator.md`**:
  - Capability tag system (`text|vision|function_calling|image_generation`).
  - Protocol auto-detection (`gemini|anthropic|openai|bedrock|ollama|custom`).
  - Multi-key rotation con blacklist de 90s.
  - Runtime options (thought level, modes).
- **Invenciones diferenciales que mantenemos como novedosas y potencialmente patentables** (nadie en competencia las tiene):
  1. Routing dinámico por skill graph + tier + costo automático.
  2. Voting ponderado con confidence declarado por cada modelo.
  3. Cascade cross-provider con budget enforcement.
  4. Subagentes locales consumidos según VRAM/RAM libres (pool_size dinámico resource-aware).
  5. Telemetría `was_correct` retroalimentada al registry affinity.

## 3. Cursor multi-agente + OpenCode Desktop

### 3.1 Cursor Cloud Agents (https://cursor.com/cloud, https://cursor.com/agents)
- Spawneables desde Desktop, Mobile, Slack/Teams, Linear, Jira.
- Branch por agente → produce diff +262/-26 visible, botones `Review`/`Commit & Push`.
- Sandbox aislado con escritorio remoto: *"interact with the software from the agent's remote desktop"*.
- *"Iterate until they've validated their output"* — loop self-testing.
- **Demos over diffs**: el agente entrega videos, screenshots, logs — no solo texto.
- **Autoaciones (always-on agents)**: scheduled / triggered by GitHub/Slack/Linear/webhooks/cron, con `memory tool` que *"learns from past runs"*.
- **Bugbot Autofix**: spawn automático de cloud agents para testear PRs y abrir fixes.

### 3.2 OpenCode Desktop (https://opencode.ai)
- Arquitectura: primary agents (`build`, `plan`) + subagents (`general`, `explore`, `scout`, `compaction`, `title`, `summary`).
- `general` es explícitamente paralelo — orquestación tipo árbol de sesiones (no un solo hilo lineal).
- Sistema de permisos (https://opencode.ai/docs/permissions): `allow | ask | deny`; **incluye `doom_loop` triggered tras 3 tool calls idénticos**.
- Modo `--auto` approve-all by default salvo `deny`.

### 3.3 Principio "humano asistido por IA" (Nate Gentile)
- "No se trata de una IA asistida por humanos, sino de un humano asistido por IA" (min 12–14 video Cursor).
- "Varios agentes en paralelo; el humano sigue siendo quien coordina y recibe ayuda" (min 20–22 video OpenCode Desktop).
- Implicaciones UI: la cola de revisión es ciudadano de primera clase (no el editor); estado multi-agente visible; demos over diffs; el humano habla en objetivos, no en keystrokes.
- Esto justifica el Mission Control en `17 - UI.md` y el catalog Execution Modes en `21 - Execution Modes.md`.

## 4. Editores clásicos — autocompletado por extensión

Cuadro consolidado (luego emulado en `21 - Execution Modes.md §2.3`):

| Editor | Motor | Cómo detecta lenguaje | Tipo de completion | Symbol table | Inval. por save |
|---|---|---|---|---|---|
| Visual Studio (pre-AI) | Roslyn (C#/VB), IntelliSense C++ (EDB), FSharp.Compiler.Service; MEF por Content-Type | Extensión mapeada fija en `Tools > Options` | Estructural + sintáctica + semántica (Roslyn) | Roslyn `Workspace` incremental por delta edit | On-type; reindex total si cambia build config |
| Apache NetBeans | nb-javac (fork OpenJDK), PHP Parser propio, JS lexer, JLI | MIME type derivado de extensión vía `MIMESupport` (`text/x-java`, `text/x-php`, ...) | Semántica por AST + índice Java; camelHumps; Javadoc | Índice Lucene en `${userdir}/var/cache/index` | `DocumentEvent` on-type; save reindexa archivo + dirty de referencias |
| VS Code (LSP) | LSP servers externos (clangd, gopls, rust-analyzer, typescript-language-server, pylsp/pyright); built-in TS/CSS/HTML in-process | LanguageId por extensión (`files.associations`); ⌘K M override manual | Semántica vía LSP `textDocument/completion` + `resolve` | Delegada a cada server (clangd `~/.cache/clangd/`, rust-analyzer salsa, gopls GOCACHE) | `didChange` incremental; algunos servers reescanean en `didSave` |

Fuentes:
- https://learn.microsoft.com/en-us/visualstudio/ide/using-intellisense
- https://microsoft.github.io/language-server-protocol/overviews/lsp/overview/
- https://code.visualstudio.com/docs/languages/overview
- https://code.visualstudio.com/api/language-extensions/language-server-extension-guide
- https://netbeans.apache.org

## 5. `/loop`, `/goal` y modos autónomos

### 5.1 Claude Code `/loop`
Patrón comunitario: invoco `/loop "<objetivo>"` y el agente ejecuta → verifica → si falla diagnostica → edita de nuevo → hasta success. Límites comunes: `max_iterations`, `max_cost_usd`, `max_minutes`, `stop_condition` regex por output. Fallo conocido: **doom-loop** / **optimistic loop** donde reintroduce el mismo bug.

### 5.2 Claude `claude.ai /goal`
No hay documentación pública canónica de `/goal`. Lo equivalente es el patrón Projects + system prompt + memory como *goal tracker*;; en Claude Code sería un system-prompt permanente.

### 5.3 OpenCode `/loop`
OpenCode no expone `/loop` como comando builtin (están `/init`, `/undo`, `/redo`, `/share`, `/help`). Pero el runtime expone la política **`doom_loop`** como permiso que detona `ask` tras 3 llamadas idénticas de tool — principal innovación anti-loop del estado del arte.

### 5.4 Sakana AI / Fugu-like
Patrones evolutivos continuos (AI Scientist v2): el agente *muta su prompt* y *recompone su toolset* entre iteraciones. Está en la base de `21 - Execution Modes.md §5` con `GoalTracker.last_progress_step`.

### 5.5 Otros editores
| Editor | Loop | Anti-infinite-loop | Distintivo |
|---|---|---|---|
| Aider | `--auto-commits`, *architect mode* (dos fases) | `--verify` | architect+editor con dos modelos separados |
| Cline / Roo Code | `auto-approve` toggle, Yolo | checkpoints git por step | "Rest of code here" bug conocido |
| Continue.dev | "ambient" mode | no tiene detector | pasivo |
| Cursor Cloud Agents | *"iterate until validated"* | sandbox + time | demos over diffs |
| Cursor Automations | schedules/triggers; memory tool cross-run | cron + "Avoid racing other agents" | always-on |
| Hermes | `/background`, worktrees | - | VPS idle-casi-gratis |
| **OpenCode OS (nuestro)** | AUTONOMOUS | `doom_loop` hard-deny + budget hard + goal_drift | los cuatro modos de `21` |

## 6. Cambios aplicados a los RFCs como consecuencia

- `02 - Agent Operating System.md`: §5 Modos reescrito con cuatro `Execution Modes` ortogonales a `Resource Mode`.
- `03 - Engine Architecture.md`: §9 añadido con `DoomLoopDetector`, `GoalTracker`, `LanguageIdResolver`, `ProjectSymbolTable`, `SemanticEmbeddingIndex`, state machine del Execution Supervisor.
- `04 - Model Orchestrator.md`: registry de modelo ampliado con `protocol`, `capabilities`, `api_keys` (multi-key), `runtime_options`, `capability_tags`. Lección de AionUi escrita como nota.
- `17 - UI.md`: añadido el banner con cita de Nate Gentile; añádido Mission Control con cola de Review como ciudadano de primera clase; añadido Execution Mode selector con cuatro iconos.
- `19 - Execution Supervisor.md`: política anti-infinite-loop mecánica con `doom_loop` trip → `ask`/`hard_deny` según mode; `caps`, `goal_drift_decay`, `recovery`, `escape_hatch`, `audit`. State machine formal.
- `21 - Execution Modes.md` (nuevo, draft): los cuatro modos `MANUAL_CLASSIC` / `HUMAN_IN_LOOP` / `AUTOPILOT` / `AUTONOMOUS` cubren exactamente los tres pedidos por el usuario (Manual / Pedir-Permiso / IA-Hace-Todo) más Autopilot como punto medio. Defaults por tipo de archivo via `CapabilityResolver`. Cuadro comparativo con otros editores.

## 7. URLs citadas (resumen)

- Hermes Agent: `github.com/NousResearch/hermes-agent` (MIT).
- AionUI: https://github.com/iOfficeAI/AionUi — https://aionui.com — https://deepwiki.com/iOfficeAI/AionUi.
- AionUI Team Mode: https://deepwiki.com/iOfficeAI/AionUi/4.10-team-mode-(multi-agent-collaboration).
- AionUI API Key Rotation: https://deepwiki.com/iOfficeAI/AionUi/13.1-api-key-rotation.
- Cursor Cloud Agents: https://cursor.com/cloud — https://cursor.com/agents — https://cursor.com/blog/third-era — https://cursor.com/blog/agent-computer-use — https://cursor.com/blog/bugbot-autofix.
- Cursor Automations: https://cursor.com/automate.
- OpenCode home: https://opencode.ai — docs agents: https://opencode.ai/docs/agents — docs permissions (`doom_loop`): https://opencode.ai/docs/permissions — docs commands: https://opencode.ai/docs/commands.
- IntelliSense VS: https://learn.microsoft.com/en-us/visualstudio/ide/using-intellisense.
- LSP overview: https://microsoft.github.io/language-server-protocol/overviews/lsp/overview/.
- VS Code Programming Languages: https://code.visualstudio.com/docs/languages/overview.
- VS Code language-server extension guide: https://code.visualstudio.com/api/language-extensions/language-server-extension-guide.
- Apache NetBeans: https://netbeans.apache.org.
- Aider options: https://aider.app/docs/config/options.html.
- Julian Goldie Hermes+AionUi review: https://www.youtube.com/watch?v=vWxE6VO9TKo.
- Sakana AI Scientist (Fugu-like): https://sakana.ai/ai-scientist (verificar).

---

## 8. Investigación Round 2 (2026-07-13)

Ronda de investigación lanzada con 3 subagentes en paralelo. Hallazgos aplicados en `23`, `24` y `25`.

### 8.1 Orquestadores (30+ analizados)

Estudiados: **CrewAI**, **Microsoft AutoGen**, **LangGraph** (LangChain), **DSPy**, **Agno**, **Letta** (MemGPT), **Temporal**, **Apache Airflow**, **n8n**, **Windmill**, **Hermes**, **AionUI**, **Manus**, **Cognition Devin**, **Factory Droids**, **Sakana FuguCoT**, **Cline/Roo**, **Cursor**, **Claude Code**, **OpenCode CLI**, **Aider**, **Continue.dev**, **LangGraph-derived (LangChain Agents / Convergence)**, **Inferflow**, **Portkey + Devwrat/Krrish chat-arena**, **Pydantic AI**, **Magentic-One** (Microsoft), **Google AITaC / Agent Builder**, **Salesforce Agentforce**, **AWS Bedrock Agents**, **MuleSoft Tyestion**, **IBMAutomation**.

TOP 10 lecciones absorbidas en OpenCode OS:

1. **Temporal workflow engine** — Durable Execution con checkpointing event-sourced. Le aplicamos al `19 - Execution Supervisor.md` para persistir checkpoints en SQLite + reanudar desde cualquier estado.
2. **LangGraph state graph** — explícit conditional edges; reemplaza el prompt-implicitacio control flow de Claude Code. Lo aplicamos en `12 - Planning Engine.md` (output Plan se modela como DAG).
3. **CrewAI role hierarchies** — simétrico a nuestro swarm (Planner→Backend→Reviewer→Merger) pero CrewAI no es resource-aware. Nosotros nos quedamos con el `05 - Swarm.md`.
4. **Letta MemGPT memory tiers** — stairs of memory for long-running agents (core/archival/recall). Lo incorporamos al `09 - Vector Knowledge.md` y `16 - Learning Engine.md` como tiered storage.
5. **n8n node canvas + execution log per node** — aplicado al `24 - HUD Mission Control.md` §13 como Canvas view con reasoning trail por nodo.
6. **Windmill flows with typed inputs/outputs** — aplicado a Skills como TS/Rust type specifications (`06 - Skills.md`).
7. **Manus "delay tool call" pattern** — impone wait si el subagent intenta acción sensible sin sufficiente contexto. Lo aplicamos al `23 - Prompt Understanding & Refinement.md` (paso 2 aclarando el respeto).
8. **Cognition Devin persistent VPS** — aplicado como "the agent runs on a Rust core that survives webview crashes", §2 de `25`.
9. **Factory Droids worktree-per-task** — ya lo teníamos en `05 §3` (worktree own per subagent), confirmado como patrón dominante en producción.
10. **Magentic-One reflective loop** — generalist agent + multiagent reflection round. Lo aplicamos en `23 §1` (Self-Refine paso 7) y `16` Learning Engine.

TOP 10 patrones arquitecturales comunes:

1. **Event-sourced Journal** (Temporal, Airflow, OpenCode OS, Letta) — único source of truth, replays any state.
2. **Worktrees own per subagent** (Droids, Cursor Cloud Agents, OpenCode OS).
3. **Capability graph + skill picker** (AionUI, OpenCode OS, Copilot Spaces).
4. **Anti-loop mecánico** (OpenCode CLI's `doom_loop`, Sakana Fugu, nuestro `19`).
5. **Budget hard + compaction limit** (Cursor Cloud Agents, OpenCode OS AUTONOMOUS).
6. **Multi-key rotation** (AionUI `ApiKeyManager`).
7. **Chain-of-Thought trail persistente** (Manus, Magentic-One).
8. **Demos over diffs** (Cursor Cloud Agents).
9. **Sandbox + capability drop** (Hermes, OpenClaw attempts).
10. **Memory tiers** (Letta, MemoryTools).

TOP 10 features que colegimos y aplicamos:

1. Multi-model orchestration (CrewAI + AionUI).
2. Per-role model assignment (Swarm de OpenCode OS, §3 de `05`).
3. Subagent tree fork (Cursor fork, OpenCode OS §3.2).
4. Background isolate task (`Hermes /background`).
5. Cron / scheduled (OpenCode OS Fase 9 Roadmap, AionUI Team Mode).
6. Steer while running (`Hermes /busy steer`).
7. Approvals pauserule (Hermes, propio en `24 §5`).
8. Vector KB semantic recall (Letta, DSPy, reflexion).
9. Cost audit live (Hermes HUD, OpenCode OS `24` Cost view).
10. WebSocket health status (Hermes HUD).

### 8.2 Prompt Understanding & Refinement

Estudiados y citados en `23 - Prompt Understanding & Refinement.md` §2.1:

- **Self-Refine** (Madaan et al. 2023, https://arxiv.org/abs/2303.17651) — paso 7 de la pipeline.
- **Reflexion** (Shinn et al. 2023, https://arxiv.org/abs/2303.11366) — paso 4 recupera memorias verbales.
- **Tree of Thoughts** (Yao et al. 2023, https://arxiv.org/abs/2305.10601) — paso 2.
- **RePrompt** (Chen et al. 2024, https://arxiv.org/abs/2406.11132) — ajuste de prompts futuros.
- **ODUTQA-MDC** (Wang et al. ACL 2026, https://arxiv.org/abs/2604.10159) — etiquetado fino de underspecification.
- **LLM-Modulo** (Kambhampati et al. ICML 2024, https://arxiv.org/abs/2402.01817) — LLM no planifica solo; exige verifier externo.
- **LLM-as-judge** (Zheng et al. 2023, https://arxiv.org/abs/2306.05685) — confidence del verdict juzgado por rubric.
- **Lost in the Middle** (Liu et al. 2023, https://arxiv.org/abs/2307.03172) — reordenar contexto.
- **STORM** (Shao et al. NAACL 2024, https://arxiv.org/abs/2402.14207) — multi-perspective clarification questions.
- **Survey of Agent Architectures** (Masterman 2024, https://arxiv.org/abs/2404.11584) — orden fases: plan > execute > reflect.

Anti-patrones de usuario catalogados (10): capacity_hallucination, unknown_tool_dependency, underspecified, implicit_assumption, capacity_overreach, user_knowledge_gap, low_information_prompt, lost_in_the_middle, role_overload, model_omniscience_assumption. Ver `23 §1`.

Output aplicado: `PublicUnderstandingVerdict` y `MissionConsolidated` definidos como tipos canónicos consumidos por `12 - Planning Engine.md` y por `24 - HUD Mission Control.md` (badge de confidence y相关 mission panel).

### 8.3 UI admin patterns (Hermes HUD + Jira/Linear + Cursor + Notion + n8n)

Hermes cubre: jerarquía subagentes, health status WS, cost/token audit, skills/MCP en caliente. **No cubre**: audit timeline chained, mobile review queue, demos over diffs, block-level comments, canvas con execution log.

Lecciones absorbidas en `24 - HUD Mission Control.md`:

- **Jerarquía recursiva Mission→Objective→Task→Subtask** (Linear, Asana, sin límite).
- **Demos over diffs** (Cursor Cloud Agents, Loom for async mobile review).
- **Block-level comments** (Notion) en Outline view.
- **Execution log per node** (n8n / Windmill) en Canvas view como chain-of-thought trail.
- **Health status KPIs + cost/token audit** (Hermes) en Cost & Res view + Health KPIs view.
- **Approvals queue multi-dispositivo web OIDC** (Hermes + Loom mobile).
- **Audit timeline append-only hash-chained** (block explorer pattern).
- **Worktrees visuales** with mini-git-graph per Mission (GitKraken inspiration).
- **Execution Mode selector + Modo de uso badge** (reúne `21` + `23`).
- **Mission consolidated panel** que muestra el `PublicUnderstandingVerdict` antes de desbloquear Planning.

Output aplicado: `24 - HUD Mission Control.md` con 23 secciones, 8 views intercambiables (Kanban/Canvas/Outline/Timeline/Cost & Res/Health KPIs/Audit/Worktrees), 15+ campos por tarjeta, eventos WebSocket formalizados.

### 8.4 Cambios aplicados a los RFCs como consecuencia de Round 2

- **NEW** `23 - Prompt Understanding & Refinement.md` — pipeline 9 pasos, tipado de gap_types, PublicUnderstandingVerdict, MissionConsolidated, comando `/refine`, skill `prompt-clarify`, modos `ask`/`architect`/`code`/`context` ortogonales a Execution Modes.
- **NEW** `24 - HUD Mission Control.md` — panel HUD con 8 views, 15+ campos por tarjeta, WebSocket events, approvals queue con pauserule, cost & recursos view, health KPIs, skill/MCP drag-drop en caliente, audit timeline con hash chain, demos over diffs, worktrees visuales.
- **NEW** `25 - Stack Técnico Multiplataforma.md` — Tauri 2 + Rust 1.84+ + SvelteKit 2 + SQLite/sqlite-vec + fastembed-rs + Tower-LSP + axum WebSocket + Docker/Podman/Firejail/Job-Object sandbox + OS keychain + 13 providers (3 locales + 10 free cloud + paid tier) + `opencode` CLI Rust.
- UPDATED `12 - Planning Engine.md` §2 — input cambiado a `MissionConsolidated`; §7 reforzado con gate de `verdict.confidence`.
- UPDATED `10 - Research Engine.md` §11 — añadido `probe_feasibility` consumido por `23 §2.2`.
- UPDATED `21 - Execution Modes.md` §12 — añadido eje ortogonal `Modo de uso`.
- UPDATED `22 - Research Findings.md` (este archivo) — anexado §8 con la Round 2.
- UPDATED `02 - Agent Operating System.md` §3.1 y §4 — kernel ahora routea `task.received` por Prompt Understanding Pipeline; eventos `hud.*` publicados por Kernel Bus.

---

## 9. URL nueva (Round 2)

- Self-Refine: https://arxiv.org/abs/2303.17651
- Reflexion: https://arxiv.org/abs/2303.11366
- Tree of Thoughts: https://arxiv.org/abs/2305.10601 y código https://github.com/princeton-nlp/tree-of-thought-llm
- RePrompt: https://arxiv.org/abs/2406.11132
- ODUTQA-MDC: https://arxiv.org/abs/2604.10159
- LLM-Modulo: https://arxiv.org/abs/2402.01817
- LLM-as-judge: https://arxiv.org/abs/2306.05685
- Lost in the Middle: https://arxiv.org/abs/2307.03172
- STORM: https://arxiv.org/abs/2402.14207
- Survey of Agent Architectures (Masterman 2024): https://arxiv.org/abs/2404.11584
- sqlite-vec: https://github.com/asg017/sqlite-vec
- fastembed-rs: https://crates.io/crates/fastembed
- Tauri 2: https://tauri.app
- Tower-LSP: https://crates.io/crates/tower-lsp
- n8n: https://docs.n8n.io/workflows/
- Windmill: https://docs.windmill.dev/
- Temporal: https://docs.temporal.io/
- LangGraph: https://langchain-ai.github.io/langgraph/
- CrewAI: https://docs.crewai.com/
- AutoGen: https://microsoft.github.io/autogen/
- Letta: https://docs.letta.com/
- DSPy: https://dspy.ai/
- Manus: https://manus.im/
- Cognition Devin: https://devin.ai/
- Factory Droids: https://docs.factory.ai/droids
- Magentic-One: https://www.microsoft.com/en-us/research/blog/magentic-one-generalist-multi-agents-for-solving-complex-tasks/
- Sakana AI Scientist: https://sakana.ai/ai-scientist
- sqlite-vec examples: https://github.com/asg017/sqlite-vec/blob/main/examples.md

---

## 10. Investigación Round 3 (2026-07-27) — Phase 1.5c §C (graphify pattern)

### 10.1 Motivación

RFC 28 §C adopta el **patrón** graphify (no su código Python, que violaría §11 single-binary-safe). En su lugar portamos patrón a Rust con cuatro crates nuevos. Esta subsección es el *"entry registra"* que RFC 28 §C item 9 exige — justificación + single-binary-safety + licencia + binary-size budget.

### 10.2 Crates añadidos (todos opcionales, default OFF)

- **`petgraph = "0.8"`** (rust-version 1.64, MIT/Apache-2.0) — https://crates.io/crates/petgraph
  - Pure Rust graph library. Lo usamos en `src-tauri/src/graph/traverse.rs` (gated `dag_mode`): `shortest_path` (BFS), `god_nodes` (percentile), `community_partition` (union-find — **no** Leiden since no mature Rust impl exists; ver RFC 28 §C Riesgos).
  - Tamaño: ~200 KB.
  - Sustituye cualquier dependencia en `graphify/cluster.py` (Python, `graspologic`).
- **`tree-sitter = "0.26"`** (MIT) — https://crates.io/crates/tree-sitter
  - C runtime linked estáticamente; ya usado ecosistema Rust (LSP hosts, helix, lapce).
  - Lo usamos en `src-tauri/src/graph/ast.rs` (gated `codebase-graph`): AST walk on Rust source, `find_import_cycles` (Tarjan SCC), `report_to_graph` → `EXTRACTED` edges.
  - Tamaño: ~3 MB (incluye C runtime).
- **`tree-sitter-rust = "0.24"`** (MIT, rust-version 1.64+) — https://crates.io/crates/tree-sitter-rust
  - Grammar para Rust source. Exposes `LANGUAGE` const compatible con `tree-sitter 0.26`.
  - Tamaño: ~1.5 MB.
- **`tree-sitter-svelte-next = "0.1.1"`** (MIT/Apache-2.0, sin rust-version declarado) — https://crates.io/crates/tree-sitter-svelte-next
  - Grammar para Svelte source. Fork by PRRPCHT; el original `tree-sitter-svelte = "0.10"` pinnaba tree-sitter 0.20 y falta el const `LANGUAGE` que `tree-sitter 0.26` requiere — sustitución justificada.
  - Tamaño: ~1.5 MB.
  - **Limitación observada**: el grammar tags `<script>` content como `raw_text` (no embeds JS/TS parser), así que `ast::extract` para Svelte usa un **line-classifier heuristic** (regex-free, line-based) en vez de AST-walk. Aceptable para v1 — improvement deferred: si IBM o tree-sitter-js pinnable separado, reemplazar heuristic por sub-parse usando `tree-sitter-javascript`/`tree-sitter-typescript`.

### 10.3 Binary-size budget

- Tres crates suman **~6.2 MB** (petgraph 200KB + tree-sitter 3MB + 2 grammars 3MB). Tauri desktop binario se mueve de 30-45 MB (RFC 25) estimado a 36-51 MB. Aceptable para desktop; borderline para uso server.
- Mitigación: **ambas features default OFF**. Default build sin `codebase-graph` evita tree-sitter完全. Para uso server-only el flag `--no-default-features` con `dag_mode` conserva planner DAG sin AST overhead.

### 10.4 Single-binary-safety audit

- ✅ `petgraph`: pure Rust, zero native deps.
- ✅ `tree-sitter`: C runtime linked estático via `cc` crate; no `.dll`/`.so` al runtime.
- ✅ Grammars `tree-sitter-rust` y `tree-sitter-svelte-next`: igual. **NO** dependemos de binarios Python (graphify runtime violaría §11).

### 10.5 Cambios aplicados a los RFCs como consecuencia de Round 3

- **NEW** `28 - External Tool Integration.md` §C — patrón graphify ported a Rust; §C items 1, 2, 3, 9 completos.
- UPDATED `25 - Stack Técnico Multiplataforma.md` §3.2 — crates core list ahora incluye `petgraph`? **No**: §3.2 enumera required crates; los de §C son feature-gated default-off, por lo que permanecen fuera del "core" list. Esta nota clarifica la decisión.
- UPDATED `26 - Index & Cross-References.md` — +3 rows (Mission graph, traverse, AST extractor).
- UPDATED este archivo con §10 round 3.

### 10.6 URLs nuevas (Round 3)

- graphify (patrón, no runtime): https://github.com/safishamsi/graphify
- petgraph: https://crates.io/crates/petgraph
- tree-sitter: https://crates.io/crates/tree-sitter
- tree-sitter-rust: https://crates.io/crates/tree-sitter-rust
- tree-sitter-svelte-next: https://crates.io/crates/tree-sitter-svelte-next
- Tarjan's SCC reference: https://en.wikipedia.org/wiki/Tarjan%27s_strongly_connected_components_algorithm
