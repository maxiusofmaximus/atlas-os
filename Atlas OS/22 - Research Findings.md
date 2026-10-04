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
- **Lecciones absorbidas en Atlas OS**:
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
| **Atlas OS (nuestro)** | AUTONOMOUS | `doom_loop` hard-deny + budget hard + goal_drift | los cuatro modos de `21` |

## 6. Cambios aplicados a los RFCs como consecuencia

- `research/34 - Phase 7 security + compliance.md` (nuevo plan): Phase 7 refinada en 4 sub-fases atómicas (7.0 Firmas → 7.1 Sandbox → 7.2 Compliance → 7.3 Supply-chain). Sub-fases 7.0 + 7.1 implementadas: `security::{signature (skill_checksum SHA-256 + ChecksumVerdict + .checksum sidecar + install_gate fail-safe Forbidden), sandbox (SandboxLevel none/vuOnly/container/wasm + approval_for tabla RFC 18 §2 + fail-safe)}`, gate integrado en `atlas skill install`. Sin crates nuevas (`sha2` + `hex` ya en deps, audit RFC 25 §11).
- `02 - Agent Operating System.md`: §5 Modos reescrito con cuatro `Execution Modes` ortogonales a `Resource Mode`.
- `03 - Engine Architecture.md`: §9 añadido con `DoomLoopDetector`, `GoalTracker`, `LanguageIdResolver`, `ProjectSymbolTable`, `SemanticEmbeddingIndex`, state machine del Execution Supervisor.
- `04 - Model Orchestrator.md`: registry de modelo ampliado con `protocol`, `capabilities`, `api_keys` (multi-key), `runtime_options`, `capability_tags`. Lección de AionUi escrita como nota.
- `17 - UI.md`: añadido el banner con cita de Nate Gentile; añádido Mission Control con cola de Review como ciudadano de primera clase; añadido Execution Mode selector con cuatro iconos.
- `19 - Execution Supervisor.md`: política anti-infinite-loop mecánica con `doom_loop` trip → `ask`/`hard_deny` según mode; `caps`, `goal_drift_decay`, `recovery`, `escape_hatch`, `audit`. State machine formal.
- `21 - Execution Modes.md` (nuevo, draft): los cuatro modos `MANUAL_CLASSIC` / `HUMAN_IN_LOOP` / `AUTOPILOT` / `AUTONOMOUS` cubren exactamente los tres pedidos por el usuario (Manual / Pedir-Permiso / IA-Hace-Todo) más Autopilot como punto medio. Defaults por tipo de archivo via `CapabilityResolver`. Cuadro comparativo con otros editores.
- `research/31 - Phase 4 swarm.md` (nuevo plan, 2026-09-24): Phase 4 refinada en 6 sub-fases atómicas (4.0 Foundation → 4.5 Swarm Console) con evidencia RFC 30 (agency-agents roles con personalidad, munder-difflin mailbox + floor 2D) y Conductor CN-001/CN-003/CN-004 (worktree por agente + auto-rebase + checks button). Sub-fase 4.0 implementada: `swarm::{roles (Role, 10 variantes RFC 05 §1), worktrees (WorktreeManager)}`, M29 (`swarm_agents` + `agent_mailbox`), registry `Journal::{register_swarm_agent, swarm_agents_for_mission, set_swarm_agent_state}`. Decisión: git CLI via `std::process::Command` (patrón `research::ingest`), `git2` NO se añade (audit single-binary RFC 25 §11).

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

TOP 10 lecciones absorbidas en Atlas OS:

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

1. **Event-sourced Journal** (Temporal, Airflow, Atlas OS, Letta) — único source of truth, replays any state.
2. **Worktrees own per subagent** (Droids, Cursor Cloud Agents, Atlas OS).
3. **Capability graph + skill picker** (AionUI, Atlas OS, Copilot Spaces).
4. **Anti-loop mecánico** (OpenCode CLI's `doom_loop`, Sakana Fugu, nuestro `19`).
5. **Budget hard + compaction limit** (Cursor Cloud Agents, Atlas OS AUTONOMOUS).
6. **Multi-key rotation** (AionUI `ApiKeyManager`).
7. **Chain-of-Thought trail persistente** (Manus, Magentic-One).
8. **Demos over diffs** (Cursor Cloud Agents).
9. **Sandbox + capability drop** (Hermes, OpenClaw attempts).
10. **Memory tiers** (Letta, MemoryTools).

TOP 10 features que colegimos y aplicamos:

1. Multi-model orchestration (CrewAI + AionUI).
2. Per-role model assignment (Swarm de Atlas OS, §3 de `05`).
3. Subagent tree fork (Cursor fork, Atlas OS §3.2).
4. Background isolate task (`Hermes /background`).
5. Cron / scheduled (Atlas OS Fase 9 Roadmap, AionUI Team Mode).
6. Steer while running (`Hermes /busy steer`).
7. Approvals pauserule (Hermes, propio en `24 §5`).
8. Vector KB semantic recall (Letta, DSPy, reflexion).
9. Cost audit live (Hermes HUD, Atlas OS `24` Cost view).
10. WebSocket health status (Hermes HUD).

### 8.2 Prompt Understanding & Refinement

Estudiados y citados en `23 - Prompt Understanding & Refinement.md` §2.1:

- **Self-Refine** (Madaan et al. 2023, https://arxiv.org/abs/2303.17651) — paso 7 de la pipeline.
- **Reflexion** (Shinn et al. 2023, https://arxiv.org/abs/2303.11366) — paso 4 recupera memorias verbales. **Contribución Atlas OS (sub-fase 2.2):** el paper original usa el mismo LLM para executor y reflexor; nosotros separamos roles y usamos un reflexor ~4× más barato (rate × 1/4 en `ReflexionCostGuard`). Generalización NO validada en el paper — anotada como contribution de Atlas OS. Implementado en `orchestrator/aggregation/reflexion.rs` con `detect_doom_loop()` (2 episodios consecutivos con `failure_signal` idéntico → abort mission, RFC 19 doom-loop guard).
- **AN-2.3-a `linfa` MLP deferral (sub-fase 2.3 auto-routing, 2026-08-12).** RFC 04 §7 (pre-2.3 wording), RFC 20 line 78 y `research/29` line 241 mencionaban "2-layer MLP `linfa`" como clasificador. Context7 verification (`npx ctx7 docs /rust-ml/linfa "neural network feed forward multilayer perceptron nn training backprop"`, 2026-08-12) retornó *no documentation match* — `linfa` NO es un MLP feed-forward module, sólo logistic regression + clustering + SVM. Una MLP hand-rolled sería ~200 LOC de matrix math para ganancias marginales: HybridLLM (arXiv:2404.14618 §4.3 Fig. 5) muestra que logistic regression con BGE-small ya alcanza ~94% de la accuracy del MLP con un peso-footprint un orden de magnitude menor. **Decisión**: la MLP queda deferred a Phase 2.5+ como optimización; la sub-fase 2.3 embarca `LogisticRegressionClassifier` (multi-class one-vs-rest multinomial softmax scratch-built en `orchestrator/classifier/log_reg.rs`, sin dep `linfa` — esto cumple AGENTS.md §6 boundary rule "no new external deps"). Cualquier MLP futura entra vía la misma `AutoRouterConfig.weights_path` (formato extendido sin romper la schema logreg JSON). Documentación completa in RFC 04 §7.1.
- **AN-9.1 candle/Laya audit DIFERIDO (sub-fase 9.1, 2026-09-25).** RFC 35 §7.1 + research/37 §A.1 decidían `ClassifierKind::Laya` (`laya = "0.1.1"`, `aovestdipaperino/laya-rust`, crates.io) como 4º backend. Audit RFC 25 §11 previo obligatorio vía crates.io API (verificación 2026-09-25): `laya 0.1.1` publicado 2026-09-20 (5 días), 61 descargas totales, `0.1.0` yanked, mantenedor único, sin docs, crate 178 KB (sólo código — los pesos ModernBERT-large + RL head se descargan en runtime, cientos de MB); deps non-optional `candle-core`/`candle-nn`/`candle-transformers ^0.9` + `tokenizers ^0.21` + `clap derive`. Bloqueadores: (1) `candle-core 0.9` exige `rand ^0.9` — Atlas OS pinnea `rand 0.8` (jitter `RetryPolicy`) → duplicaría `rand` en el árbol; (2) `tokenizers 0.21` NO está en deps (RFC 35 §7.1 asumía que sí) y arrastra build C++ (`esaxx-rs`) + lib precompilada (`spm_precompiled`) + ~18 crates; (3) el feature `serve` de laya pide `axum ^0.8` — el HUD pinnea `axum 0.7`; (4) doble runtime de inferencia (`ort-sys` vía `fastembed` + `candle`) contra el budget single-binary 30-45 MB; (5) madurez supply-chain insuficiente para bundlear (5 días, 61 descargas, yanked previo). **Decisión** (precedente Phase 8 8.2/8.3/8.4): DIFERIDO con audit documentado; la sub-fase 9.1 embarca `LayaClassifier` std-only MVP (`orchestrator/classifier/laya.rs`, cero deps nuevas, inferencia lexical-delegada determinista tagueada `Laya`, `load` con failure-path → fallback lexical), feature `laya` vacío default-off y M33 (schema 32, CHECK + `'laya'`). La inferencia candle real entra detrás del mismo gate cuando el crate madure (descargas + versión estable + pesos pinnables) sin cambiar callers. Follow-ups anotados: wiring compaction (5.3) + tool-result judging (winnow).
- **AN-9.2 tree-sitter/grammars audit APROBADO (sub-fase 9.2, 2026-09-25).** Research/37 §A.3 pedía auditar `tree-sitter` + grammars para el AST Context Engine. Verificación: el stack ya está vendored y auditado por RFC 22 §10 (Round 3) y RFC 28 §C (commit `cddcbc2`) — `tree-sitter 0.26` (C runtime link estático vía `cc`, ~3 MB) + `tree-sitter-rust 0.24` + `tree-sitter-svelte-next 0.1.1` (sustitución justificada de `tree-sitter-svelte 0.10`, que pinnaba tree-sitter 0.20), todos default-off tras `codebase-graph`, single-binary-safe (§10.4). **Decisión**: APROBADO con cero crates nuevas — la sub-fase 9.2 embarca `context/ast.rs` (`AstSymbol {kind, name, file, line}` + `validate` + feeds `presence_boost`/`confidence_for_symbol`) con doble modo (walk AST real tras `--features ast`/`codebase-graph`; heurístico std-only determinista en default, misma limitación `raw_text` en Svelte que §10.2), feature `ast` como alias de `codebase-graph` y M34 (schema 33, tabla `ast_symbols`). Follow-up: 9.4 consume `confidence_for_symbol` + `ast_symbols_for_file` para Confidence por símbolo en hover/diagnostics.
- **Tree of Thoughts** (Yao et al. 2023, https://arxiv.org/abs/2305.10601) — paso 2.
- **RePrompt** (Chen et al. 2024, https://arxiv.org/abs/2406.11132) — ajuste de prompts futuros.
- **ODUTQA-MDC** (Wang et al. ACL 2026, https://arxiv.org/abs/2604.10159) — etiquetado fino de underspecification.
- **LLM-Modulo** (Kambhampati et al. ICML 2024, https://arxiv.org/abs/2402.01817) — LLM no planifica solo; exige verifier externo.
- **LLM-as-judge** (Zheng et al. 2023, https://arxiv.org/abs/2306.05685) — confidence del verdict juzgado por rubric.
- **Lost in the Middle** (Liu et al. 2023, https://arxiv.org/abs/2307.03172) — reordenar contexto.
- **STORM** (Shao et al. NAACL 2024, https://arxiv.org/abs/2402.14207) — multi-perspective clarification questions.
- **Survey of Agent Architectures** (Masterman 2024, https://arxiv.org/abs/2404.11584) — orden fases: plan > execute > reflect.

Anti-patrones de usuario catalogados (10): capacity_hallucination, unknown_tool_dependency, underspecified, implicit_assumption, capacity_overreach, user_knowledge_gap, low_information_prompt, lost_in_the_middle, role_overload, model_omniscience_assumption. Ver `23 §1`.

Output aplicado: `PublicUnderstandingVerdict` y `MissionConsolidated` definidos como tipos canónicos consumidos por `12 - Planning Engine.md` y por `24 - HUD Mission Control.md` (badge de confidence y related mission panel).

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
- **NEW** `25 - Stack Técnico Multiplataforma.md` — Tauri 2 + Rust 1.84+ + SvelteKit 2 + SQLite/sqlite-vec + fastembed-rs + Tower-LSP + axum WebSocket + Docker/Podman/Firejail/Job-Object sandbox + OS keychain + 13 providers (3 locales + 10 free cloud + paid tier) + `atlas` CLI Rust.
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
- Mitigación: **ambas features default OFF**. Default build sin `codebase-graph` evita tree-sitter por completo. Para uso server-only el flag `--no-default-features` con `dag_mode` conserva planner DAG sin AST overhead.

### 10.4 Single-binary-safety audit

- ✅ `petgraph`: pure Rust, zero native deps.
- ✅ `tree-sitter`: C runtime linked estático via `cc` crate; no `.dll`/`.so` al runtime.
- ✅ Grammars `tree-sitter-rust` y `tree-sitter-svelte-next`: igual. **NO** dependemos de binarios Python (graphify runtime violaría §11).

### 10.5 Cambios aplicados a los RFCs como consecuencia de Round 3

- **NEW** `28 - External Tool Integration.md` §C — patrón graphify ported a Rust; **§C COMPLETE (Phase 1.5c)** — items 1–9 ✅ (migrations, traverse, AST, planner emitter, skill templates, learning graphs, HUD endpoint + `<GraphView>`). Last commit `cddcbc2`.
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

## 11. Investigación Round 4 (2026-07-29) - Phase 1.5e §E (Firecrawl web ingestion)

### 11.1 Motivación

RFC 28 §E introduce Firecrawl como superficie de web ingestion polyfacética (scrape, crawl, search, extract). Phase 1.5c §C ya introdujo `petgraph` + `tree-sitter` como bump binario; §E es otro bump pero elcrate oficial existe en Rust. La pregunta de research: ¿existe un SDK Rust oficial, qué crates competidoras hay, qué expansión binaria real, y cómo proteger la single-binary safety (RFC 25 §11)?

### 11.2 Decision arquitectónica: adapter facade

Adoptar **adapter facade** pattern: `src-tauri/src/firecrawl/{mod, facade, client, error}.rs`. La facade expone tipos canónicos `ScrapedDocument`, `SearchResult`, `CrawlBatch`, `ExtractResult` independientes de la librería subyacente. Beneficios:
- Permite swap del crate subyacente sin tocar callers (si `firecrawl` oficial deprecado, intercambiamos por facade impl alternativa).
- Centraliza redacción de API keys (callers nunca ven el raw client).
- Centraliza retry/backoff con `thiserror` taxonomía unificada (`FirecrawlFacadeError`).
- Gate feature `firecrawl` default OFF (single-binary safety; ver RFC 25 §11).
- callers (Research Engine, Skills, autoresearch) dependen de la facade, no del SDK.

### 11.3 Crates analizadas

| Crate | Version | Licencia | Maintainer | Decision |
|---|---|---|---|---|
| `firecrawl` | 2.12.1 | MIT | equipo Firecrawl (Mendable AI): `mogery`, `rafaelsideguide` | **adoptar** como dep opcional gated detras de feature `firecrawl` |
| `firecrawl-mcp` | 0.7.1 | MIT | `washanhanzi` (community) | **opcional/futuro**: solo si construimos MCP server nativo Rust |
| `firecrawl_rs` | 0.1.1 | ? | ? | **rechazar**: cubierta por `firecrawl` oficial |
| `firecrawl-sdk` | 0.5.1 | ? | ? | **rechazar**: cubierta por `firecrawl` oficial |

`firecrawl 2.12.1` es el SDK oficial Rust mantenido por el equipo Firecrawl (Mendable AI). MIT. Dependencies: `reqwest 0.12`, `serde`, `tokio 1`, `thiserror 1`, `serde_with 3`. API: `Firecrawl::new_http(api_key)`, `.scrape_url`, `.search`, `.crawl_url`, `.extract`. Async, returns `Result<...>`.

### 11.4 Binary-size budget

- Binario Atlas OS actual: ~30-45 MB (RFC 25).
- `firecrawl` crate: < 800 KB incremental (reqwest ya presente para axum). Aceptable.
- `firecrawl-mcp` (MCP SDK): ~1.5 MB. Postergado hasta que sea necesario (no en Phase 1.5).
- Budget total Phase 1.5 (§A+§C+§B+§E) < 5 MB incremental sobre el binario base.

### 11.5 Single-binary safety audit

RFC 25 §11 exige single binary. `firecrawl` crate: no requiere Python, no instala herramientas externas, no spawn processes. Todo HTTP via reqwest (async, dentro del proceso). Apto. `firecrawl-mcp` lo cumple igual pero postergamos.

### 11.6 Licencia y atribución

`firecrawl 2.12.1` MIT — compatible conAtlas OS (RFC 28 apéndice). Per-module attribution required en `src-tauri/src/firecrawl/mod.rs`: campo `// Ported from firecrawl 2.12.1 (MIT, Mendable AI, https://github.com/mendableai/firecrawl)` — no, no es porting, es uso directo del crate. Atribución distinta: declaración de uso de crate externo en `src-tauri/Cargo.toml` y `Atlas OS/28 - External Tool Integration.md` apéndice licencias.

### 11.7 Cambios aplicados

- ADDED §E a RFC 28 (lines 308-399): source items, adapter facade rationale, 6 objectives, Rust change plan, CLI commands plan, 10-item checklist (all ⏳), 7 risks, atribución.
- UPDATED `26 - Index & Cross-References.md` - +2 rows (Firecrawl adapter facade, `atlas research`).
- UPDATED este archivo con §11 round 4.
- UPDATED RFC 28 "Orden recomendado" para incluir §E quinto (post-graphify).

### 11.8 URLs nuevas (Round 4)

- firecrawl Rust SDK oficial: https://crates.io/crates/firecrawl
- firecrawl repo (Mendable AI): https://github.com/mendableai/firecrawl
- firecrawl docs API: https://docs.firecrawl.dev
- firecrawl-mcp: https://crates.io/crates/firecrawl-mcp
- firecrawl_rs (rechazada): https://crates.io/crates/firecrawl_rs
- firecrawl-sdk (rechazada): https://crates.io/crates/firecrawl-sdk
- adapter facade pattern: https://rust-unofficial.github.io/patterns/patterns/structural/facade.html
- reqwest 0.12: https://crates.io/crates/reqwest
- thiserror 1: https://crates.io/crates/thiserror
- serde_with 3: https://crates.io/crates/serde_with
- RFC 25 §11 single-binary: ./25%20-%20Stack%20T%C3%A9cnico%20Multiplataforma.md
- RFC 28 §E: ./28%20-%20External%20Tool%20Integration.md

## 12. Investigación Round 5 (2026-07-31) - Phase 1.5f/g/h §F/§G/§H (Toast, Calendar, Reset-window)

### 12.1 Motivación

RFC 28 §F (Windows Toast notifications), §G (Windows Calendar integration) y §H (Model API reset-window notifications) introducen tres superficies que tocan SOs nativos, autenticación federada y patrones UX de AI coding tools comerciales. La investigación exhaustiva buscó:

1. Para §F: una crate Rust maintained que exponga WinRT Toast con `on_activated`/`on_dismissed` callbacks y `register()` para AUMID desde apps desktop no-MSIX. `tauri-plugin-notification` fue considerado porque ya es Tauri-ecosystem.
2. Para §G: una solución a la escritura bidireccional al calendario nativo (Outlook/Apple/Google) sin depender de capabilities WinRT restringidas. WinRT `AppointmentManager` y `Microsoft.Graph.Calendar`fueron evaluados; la vía `.ics` (RFC 5545) servida vía el axum HUD server y la vía Microsoft Graph REST (`/me/calendarView`) resultaron complementarias.
3. Para §H: patrones provenientes de AI coding tools reales (Cline, Cursor, Aider, Copilot) en su manejo de 429/spend-limit, identificación del "discriminador competitivo" — qué feature NINGÚN tool ofrece hoy, y cómo Atlas OS puede adueñarse del espacio.

La investigación cubrió ~30 URLs entre crates.io, repos de crates, PRs de Cline/MIT-licensed AI tools y docs de providers LLM.

### 12.2 Toast notifications (Windows)

**Crate decision: `winrt-toast-reborn = "0.3.8"`** (crates.io ID `winrt-toast-reborn`, autor Md. Iftakhar Awal Chowdhury / `AtifChy`, repo `https://github.com/AtifChy/winrt-toast.git`, MIT, publicado 2025-09-01, ~16k downloads).

- Fork mantenido de `winrt-toast 0.1.1` (original `allenbenz/winrt-notification`, ya sin updates desde 2020).
- Expone `Toast::new(&AumId).with_title(...).show()?`, `register(&AumId, &DisplayName, &IconPath)?` (crea Start Menu shortcut + AUMID registry), `on_activated(|action| ...)`, `on_dismissed(|reason| ...)`, `on_failed(|error| ...)`. Soporta action buttons, deep-link via `activation_type=protocol` y argumento `opencode://...`.
- Pure Rust (windows-rs FFI bindings), zero native deps. Single-binary safe.
- MSRV: edition 2021, sin rust_version locked — compila con 1.84+.

`tauri-plugin-notification = "2.3.3"` RECHAZADO como primario en Windows: bug de heurística AUMID ([tauri-apps/plugins-workspace#1545](https://github.com/tauri-apps/plugins-workspace/issues/1545)) hace que `schedule` se vuelva silent no-op en desktop no-MSIX. No da callback de activación (sólo `show()`). Se retiene sólo para Linux/macOS como fallback trivial (no deep-link).

Scheduler: NO via `ScheduledToastNotification` (WinRT scheduling API) porque ninguna crate Rust lo expone. Driver propio sobre `tokio::time::sleep_until(expiry) + SQLite toast_queue` queue (RFC 28 §F.3). Sobrevive a crashes, es auditable, y reusa el `toast_queue` de §F para ambos `kind='model_ready'` (§H) y `kind='calendar_reminder'` (§G alternativo).

### 12.3 Calendar integration (Windows)

**Reject WinRT `AppointmentManager`**: requiere capability restringida `appointmentsSystem` (manifest). En apps desktop no-MSIX (Tauri 2 default, `.exe` instalado sin MSIX), `ShowAddAppointmentAsync`/`FindAppointmentsAsync`/etc. devuelven `E_ACCESSDENIED (0x80070005)` porque la capability está condicionada al ser UWP package sandboxed. Esto viola RFC 25 §11 (single-binary, sin capabilities restringidas) y AGENTS.md §6.

**WRITE side: `ics = "0.5.8"`** (crates.io ID `ics`, autor `hummingly`, repo `https://github.com/hummingly/ics`, MIT OR Apache-2.0). Pure Rust RFC 5545 generator. axum HUD server añade `GET /atlas-calendar.ics?token={base64url(16 bytes)}` que emite el feed por demanda. Subscription URL `webcal://127.0.0.1:{port}/atlas-calendar.ics?token=...`. Outlook/Apple Calendar soportan `webcal://` nativamente; Google Calendar vía "From URL" settings. RRULE soporta cadencias `FREQ=WEEKLY;BYDAY=MO;COUNT=8` para autoresearch recurring.

**READ side: `graph-rs-sdk = "3.0.1"`** (crates.io ID `graph-rs-sdk`, autor `sreeise`, repo `https://github.com/sreeise/graph-rs-sdk`, MIT, ~600 stars). `features = ["interactive-auth"]` abre popup wry webview (mismo engine que Tauri 2) para primer OAuth flow interactivo. Scopes `Calendars.Read` + `offline_access`. Refresh token en SQLite encrypted (AES-256-GCM via `aes-gcm 0.10`). Endpoint `GET /me/calendarView?startDateTime=...&endDateTime=...`, poller 60s desde `AppState`. Output: `AppState.context_busy_windows: Vec<BusyWindow>` consultado por Planning engine antes de encolar turn proactivo.

**Cross-platform parity argument**: `.ics` feed servido por axum HUD server funciona en Windows, macOS y Linux — cualquier cliente de calendario moderno (Outlook, Apple Calendar, Google Calendar) soporta `webcal://` subscription sin instalar nada. El READ side via Graph es sólo conversión Microsoft; en Linux/macOS el usuario puede simplemente subscribir su OpenCode calendar URL en el mismo calendario que Planifica y recibe los eventos. Conmutación bidireccional plena.

### 12.4 Model API reset-window notifications

**Pattern source: MIT-licensed `cline/cline` PRs**:

- **PR #10207** ([cline/cline#10207](https://github.com/cline/cline/pull/10207)) añade la `SpendLimitError` card con `resets_at`, botón "Request Increase" con 5-min localStorage cooldown, entry **exempt del auto-retry**. MIT license cubre el pattern port a Rust.
- **PR #10963** ([cline/cline#10963](https://github.com/cline/cline/pull/10963)) — retry middleware con jitter ±25% + parse `Retry-After` header + `x-ratelimit-reset` header.
- **PR #10141** ([cline/cline#10141](https://github.com/cline/cline/pull/10141)) — bail-out cuando `Retry-After > threshold` (default 60s): para el retry loop en lugar de esperar.

**OmniRoute envelope handling**: Atlas OS usa OmniRoute ([github.com/diegosouzapw/OmniRoute](https://github.com/diegosouzapw/OmniRoute), MIT, 35k stars, 290+ providers, 500+ models, default branch `release/v3.8.50`) como un único OpenAI-compatible provider. OmniRoute normaliza headers upstream (`x-ratelimit-reset`, `anthropic-ratelimit-*-reset`, `Retry-After`) a un único campo `error.resets_at` en su JSON envelope:

```json
{
  "error": {
    "type": "rate_limit" | "spend_limit",
    "message": "...",
    "status": 429 | 402 | 403,
    "provider": "anthropic",
    "model": "claude-3-5-sonnet",
    "resets_at": "2026-08-01T12:34:56Z",
    "request_id": "req_abc123"
  }
}
```

OpenCode NO parsea headers del provider upstream directamente cuando se usa OmniRoute — simplifica el código. Para providers directos (sin OmniRoute), sí se parsea el header específico y se mapea al mismo formato.

**Discriminador competitivo identificado**: NINGÚN AI coding tool comercial (Cline, Cursor, Aider, Copilot, Continue) **proactivamente notifica** al usuario cuando un model vuelve a estar disponible tras un rate-limit o spend-cap. Cline viene close con `SpendLimitError` card pero es reactiva — el usuario debe reintentar manualmente. Atlas OS capturando `reset_at`, persistiéndolo en SQLite, y disparando una Toast `kind='model_ready'` cuando el reset cumple es único. Ser PRIMERO en ofrecerlo es ventaja competitiva tangible.

**OpenRouter polling**: `X-RateLimit-Reset` header en 429 + `GET /api/v1/key` para tracking. No polling activo — sólo reacciona a 429. OpenRouter no expone webhook.

**Anthropic headers**: `anthropic-ratelimit-requests-reset`, `anthropic-ratelimit-tokens-reset`, `anthropic-ratelimit-tokens-reset` — formato RFC 3339. Parseable.

**OpenAI headers**: `x-ratelimit-reset-requests`, `x-ratelimit-reset-tokens` — formato duration string ("5s", "12m", "1h"). Parseable a `Duration`.

**LiteLLM defaults**: `cooldown_time=5s`, `allowed_fails=3` — referencias para thresholds propios.

### 12.5 Crates a añadir (todos opcionales, default OFF)

| Crate | Version | License | Phase | Cargo gate |
|---|---|---|---|---|
| `winrt-toast-reborn` | `0.3.8` | MIT | 1.5f §F | `features = ["toast-notifications"]`, target `cfg(windows)` |
| `tauri-plugin-notification` | `2.3.3` | Apache-2.0 OR MIT | 1.5f §F (fallback) | `features = ["toast-notifications"]`, target `cfg(not(windows))` |
| `ics` | `0.5.8` | MIT OR Apache-2.0 | 1.5g §G (WRITE) | `features = ["calendar"]` |
| `graph-rs-sdk` | `3.0.1` | MIT | 1.5g §G (READ) | `features = ["calendar"]`, `features = ["interactive-auth"]` |
| `aes-gcm` | `0.10` | MIT OR Apache-2.0 | 1.5g §G (token encryption) | `features = ["calendar"]` |

§H no introduce nuevas crates (usa `chrono`, `tokio`, `serde` ya presentes). Reusa el `toast_queue` de §F.

### 12.6 Binary-size budget

- `winrt-toast-reborn 0.3.8`: ~400KB incremental (windows-rs FFI a `Data_Xml_DOM`/`ToastNotificationManager`).
- `tauri-plugin-notification 2.3.3`: ~150KB incremental (nativo Tauri plugin).
- `ics 0.5.8`: ~30KB incremental (pure Rust parser/serializer).
- `graph-rs-sdk 3.0.1` + `interactive-auth` + `aes-gcm 0.10`: ~600KB incremental total (reqwest deps transitivos ya presentes; wry ya presente via Tauri).
- §H: sin aumento neto.

**Total §F+§G+§H incremental: ~1.2MB** (todas las features opcional default OFF; el usuario sólo paga si opt-in).

### 12.7 Single-binary safety audit

- `winrt-toast-reborn`: pure Rust · FFI windows-rs · sin DLL externo · sin build script native · single-binary safe.
- `tauri-plugin-notification`: Tauri plugin estándar · binario único.
- `ics`: pure Rust · cero deps nativas.
- `graph-rs-sdk`: reqwest + serde_json (ya en árbol) + wry (`interactive-auth`) — ya presente via Tauri.
- `aes-gcm`: pure Rust (Ring backend default) · sin native.

Todas las crates cumplen RFC 25 §11. WinRT lo usa sólo en `cfg(target_os="windows")` para features que son Windows-only por definición. En Linux/macOS los módulos `toast/` y `calendar/` reducen a no-ops o al fallback `tauri-plugin-notification` para Linux.

### 12.8 Atribución (per-module)

- `src-tauri/src/toast/*.rs` — `"Uses winrt-toast-reborn 0.3.8 (MIT) by Md. Iftakhar Awal Chowdhury (AtifChy) — maintained fork of winrt-toast 0.1.1. https://github.com/AtifChy/winrt-toast"`.
- `src-tauri/src/toast/fallback_unix.rs` — `"Uses tauri-plugin-notification 2.3.3 (Apache-2.0 OR MIT) by Tauri Apps. https://github.com/tauri-apps/plugins-workspace"`.
- `src-tauri/src/calendar/ics_writer.rs` — `"Uses ics 0.5.8 (MIT OR Apache-2.0) by hummingly. https://github.com/hummingly/ics"`.
- `src-tauri/src/calendar/graph_reader.rs` — `"Uses graph-rs-sdk 3.0.1 (MIT) by sreeise. https://github.com/sreeise/graph-rs-sdk"`.
- `src-tauri/src/calendar/auth.rs` — `"Interactive auth via graph-rs-sdk 3.0.1 (MIT). Encryption via aes-gcm 0.10 (MIT OR Apache-2.0)."`
- `src-tauri/src/orchestrator/{error,parse_error,retry}.rs` — `"Pattern adapted from Cline (Apache-2.0) PRs #10207 (SpendLimitError card, exempt auto-retry), #10963 (jitter ±25%, Retry-After parse), #10141 (bail-out threshold). https://github.com/cline/cline"`.

### 12.9 Cambios aplicados a los RFCs como consecuencia de Round 5

- UPDATED RFC 28 con §F (Toast notifications), §G (Calendar integration), §H (Model reset-window notifications) — completa documentación Phase 1.5 extensions hasta el roadmap Phase 1.5.
- UPDATED RFC 28 "Orden recomendado" con §F/§G/§H añadidos a ordering + status.
- UPDATED RFC 28 §B orden (de "en implementación" a "✅ COMPLETO", commits `7f8215e`-`4b4924e`).
- UPDATED RFC 26 (cross-references) con §F/§G/§H rows.
- UPDATED RFC 20 (Roadmap) con entradas Phase 1.5f/g/h post-Firecrawl-§E.
- Corregido el duplicado `## 11.` header (líneas 323/325 pre-existentes).

### 12.10 URLs nuevas (Round 5)

Toast notifications:
- winrt-toast-reborn crate: https://crates.io/crates/winrt-toast-reborn
- winrt-toast-reborn repo (AtifChy fork): https://github.com/AtifChy/winrt-toast
- winrt-toast original (allenbenz): https://github.com/allenbenz/winrt-toast
- tauri-plugin-notification crate: https://crates.io/crates/tauri-plugin-notification
- tauri-apps/plugins-workspace issue #1545 (AUMID bug): https://github.com/tauri-apps/plugins-workspace/issues/1545
- Microsoft Toasts docs (deep-link activation): https://learn.microsoft.com/en-us/windows/apps/design/shell/tiles-and-notifications/send-local-toast

Calendar integration:
- ics crate: https://crates.io/crates/ics
- ics repo (hummingly): https://github.com/hummingly/ics
- graph-rs-sdk crate: https://crates.io/crates/graph-rs-sdk
- graph-rs-sdk repo (sreeise): https://github.com/sreeise/graph-rs-sdk
- Microsoft Graph /me/calendarView: https://learn.microsoft.com/en-us/graph/api/calendar-list-calendarview
- WinRT AppointmentManager restricted capability: https://learn.microsoft.com/en-us/uwp/api/windows.applicationmodel.appointments.appointmentmanager
- RFC 5545 (iCalendar): https://datatracker.ietf.org/doc/html/rfc5545
- webcal:// URI scheme: https://en.wikipedia.org/wiki/Webcal
- AES-256-GCM crate: https://crates.io/crates/aes-gcm

Model reset-window notifications:
- OmniRoute repo: https://github.com/diegosouzapw/OmniRoute
- OmniRoute docs: https://diegosouzapw.github.io/OmniRoute/
- Cline PR #10207 (SpendLimitError): https://github.com/cline/cline/pull/10207
- Cline PR #10963 (retry with jitter): https://github.com/cline/cline/pull/10963
- Cline PR #10141 (bail-out threshold): https://github.com/cline/cline/pull/10141
- OpenRouter X-RateLimit-Reset: https://openrouter.ai/docs/api-reference/limits
- Anthropic rate limits: https://docs.anthropic.com/en/api/rate-limits
- OpenAI rate limits: https://platform.openai.com/docs/guides/rate-limits
- LiteLLM retries: https://docs.litellm.ai/docs/proxy/reliability

## 13. Investigación Round 6 (2026-09-23) - HydraFusion validación Phase 2 + ecosistema de referencia

### 13.1 Motivación

Con la Phase 2 (sub-fases 2.0→2.4) completada, la investigación buscó validación externa de la tesis de orquestación adaptativa y un catálogo del ecosistema de referencia (orquestadores, skills, estándares) para dirigir Phase 3+. El input incluye el anuncio de **Project HydraFusion** de GitHub Copilot y ~23 repos/sites aportados por el operador (lista "Jev" de Charlie Hills + skills de diseño + estándares). Detalle completo en RFC 30 (Ecosystem & Jev audit) y RFC 29 (Genspark).

### 13.2 HydraFusion (GitHub Copilot) — validación de la tesis

**Fuente:** [Project HydraFusion: Frontier quality via multi-model orchestration](https://github.blog/ai-and-ml/github-copilot/project-hydrafusion-frontier-quality-via-multi-model-orchestration/) (The GitHub Blog, Sep 2026). ⚠️ No confundir con `aicps/hydrafusion` (paper sensor-fusion para percepción de vehículos autónomos — proyecto distinto, sin relación).

- **Orquestación adaptativa**: "evalúa cada request y elige el **workflow menos complejo esperado** para cubrir sus necesidades, usando llamadas de modelo adicionales sólo cuando es probable que mejoren el resultado". Balancea quality/cost/latency.
- **Model pool evolutivo**: "cuando nuevos modelos estén disponibles, los evaluamos e incorporamos al pool, llevando sus fortalezas a las tareas mejor adecuadas" — coincide con el `Registry` + `atlas models refresh` de sub-fase 2.0.
- **Auto model selection** (precursor, early 2026): "revisa tu tarea y la empareja con el modelo mejor suited" — coincide con el `TaskTypeClassifier` de sub-fase 2.3.
- **Validación**: confirma que el modelo de routing adaptativo de Atlas OS (sub-fases 2.1–2.4: routing policy + aggregation + classifier + feedback loop) va alineado con el estado del arte. La diferencia: HydraFusion es cloud-first sobre el runtime de GitHub; Atlas OS es single-binary local-first con fallback cross-provider explícito y cost guard G11 como invariant.
- **Narrativa alineada** (RFC 04 §3, RFC 00 §7.2): `Single` = "our cost-aware default", `MoA` = "the Genspark default", orquestación adaptativa = "the HydraFusion way".

### 13.3 Ecosistema de referencia catalogado

23 referencias auditadas (Detalle + priorización en RFC 30):

- **Orquestadores**: OmniRoute (gateway 359 providers, ya en stack §H), orca (ADE fleet paralela, ya auditado RFC 27), herdr (runtime, ya auditado RFC 27), munder-difflin (office of clones — floor 2D + mailbox + memoria por agente, patrón divertido para Phase 4 Swarm), deepseek-harness (everything-is-a-plugin sobre Cordis, patrón de extensibilidad RFC 06/07), HydraFusion (ver §13.2).
- **Skills de diseño**: taste-skill (anti-slop design, 9 skills v2), awesome-design-md (DESIGN.md de marcas — UI matching), img2threejs (image→3D procedural), archify (diagramas de arquitectura self-contained HTML).
- **Skills de ingeniería**: mattpocock/skills (skills for real engineers), grill-me (stress-test de planes vía questioning — cabe en Phase 3 Planning).
- **Documentación**: anydoc (Word/PPT/Excel/PDF→Markdown en Rust — candidato Phase 3 ingestion), Context7Max (proyecto propio del operador — self-hosted docs gateway, capa de conocimiento del Research Engine Phase 3).
- **Estándares/periféricos**: zod (validación TS — ya en filosofía Validation), agents.md (ya adoptado), storybook (workshop UI — referencia HUD docs), omarchy (install Linux — Phase 8), OpenMontage (12 pipelines + 700 skills — patrón catálogo por dominio), MobiAI-Core (familia del operador), NavMeshPlus (referencia para el 2D office floor del Swarm).

### 13.4 Cambios aplicados a los RFCs como consecuencia

- RFC 04 §3: narrativa "MoA as default" (RFC 29 §3.E) — `Single` cost-aware default, `MoA` Genspark default, `AggregationMode::Auto` experimental anotado 2.5+.
- RFC 17 §11: "Modo builder" (RFC 29 §3.F) — narrativa non-technical builder.
- RFC 00 §7.1/§7.2: segmento non-technical builder + validación HydraFusion.
- RFC 20: sub-fase 2.4 COMPLETA (`302b170`) — Fase 2 cerrada.
- RFC 29: audit completo commiteado.

### 13.5 URLs citadas (Round 6)

- HydraFusion blog: https://github.blog/ai-and-ml/github-copilot/project-hydrafusion-frontier-quality-via-multi-model-orchestration/
- munder-difflin: https://github.com/chaitanyagiri/munder-difflin
- deepseek-harness: https://github.com/deepseek-ai/deepseek-harness
- archify: https://github.com/tt-a1i/archify
- agency-agents: https://github.com/msitarzewski/agency-agents
- orca: https://github.com/stablyai/orca
- herdr: https://github.com/herdrdev/herdr
- OmniRoute: https://github.com/diegosouzapw/OmniRoute
- taste-skill: https://github.com/leonxlnx/taste-skill
- tasteskill.dev: https://www.tasteskill.dev/
- awesome-design-md: https://github.com/VoltAgent/awesome-design-md
- img2threejs: https://img2threejs.io/
- mattpocock/skills: https://github.com/mattpocock/skills
- grill-me: https://www.skills.sh/mattpocock/skills/grill-me
- OpenMontage: https://github.com/calesthio/OpenMontage
- anydoc: https://github.com/firecrawl/anydoc
- omarchy: https://github.com/omacom/omarchy
- OpenMontage site: https://www.openmontage.video/
- storybook: https://github.com/storybookjs/storybook
- zod: https://zod.dev/
- agents.md: https://agents.md/
- MobiAI-Core: https://github.com/ArisGuimera/MobiAI-Core
- NavMeshPlus: https://github.com/h8man/NavMeshPlus

## 14. Investigación Round 7 (2026-09-25) - Symlink/Cloudflare-audit/Context-Mode/Zoxide + deep search multi-agente

### 14.1 Método

Las 4 referencias aportadas por el operador (Symlink toolset, cloudflare/security-audit-skill 21.4k★, mksglu/context-mode 24k★ — proyecto propio, zoxide) se auditaron con fetch directo + deep search multi-agente: 2 agentes opencode en paralelo (muse-spark-1.3-contributor-free para 12 hallazgos nuevos con 22 URLs verificadas; mimo-v2.6-flash-free degradado a reseña del gestor — 2 fallos de comprensión del prompt). **Lección de delegación**: los prompts a modelos free pequeños deben llevar las referencias inline cortas, no en archivos largos — Mimo no captó las referencias ni por archivo ni re-intento; el gestor sintetizó su parte. Detalle completo en RFC 35 (Ecosystem Round 7 Audit).

### 14.2 Hallazgos clave

- **Symlink toolset**: mklink /j (built-in Windows) + Junction (Sysinternals) + ln -s — sin herramientas externas para el MVP. Rutas: skills dir linking (junction del perfil → skills/ del repo), portable inventory, worktrees (git worktree ya usa junctions en Windows).
- **cloudflare/security-audit-skill**: 6 fases (reconnaissance → hunting → validation → output → verification → reporting) + 12 hunting classes + `report-schema.json` + `validate-findings.cjs` zero-dependency. Destino: formato machine-readable de findings para el AuditLog (Phase 9) + hunting classes enrichment de las compliance skills.
- **context-mode (propio)**: overlap alto con el core (Journal + compaction 5.3 + EvidenceGate); piezas nuevas: **FTS5 full-text search sobre journal_events** (rusqlite bundled FTS5, sin crates) + **ContextBudget por event kind** (el overflow indexado, el UI consulta por demanda).
- **zoxide**: frecency (aging + ranking) — **frecency interno propio** (port del algoritmo, determinista) para navegación missions/worktrees; crate RECHAZADA (22 deps, RFC 25 §11).
- **Deep search (12 hallazgos)**: ratatui (Sister TUI), tower-lsp-server fork, ast-grep, axum-oidc-layer, sysinfo+nvml-wrapper (VRAM/RAM), agentskills.io SDK, herdr PTY, semgrep/CodeQL, dependency-cruiser, RustDesk (AGPL — jamás bundling), tauri-plugin-axum, marketplaces skills.

### 14.3 Cambios aplicados

- RFC 35: catálogo completo + priorización Phase 8/9/10.
- RFC 26: filas RFC 35 + total 32 RFCs.
- README: (siguiente actualización con el cierre de Phase 8).

## 15. Investigación Round 8 (2026-10-03) — Calendar READ (v3.1.1 Graph + v3.1.A ICS)

### 15.1 Motivación

Cerrar la deuda del READ path de RFC 28 §G (calendario como contexto de busy
windows) para v3.1. Dos fuentes: Microsoft Graph (cuentas Microsoft 365/personales)
y una suscripción `.ics` (cualquier proveedor). El diseño original de Round 5 §G
asumía `graph-rs-sdk` con `interactive-auth` (webview) y no contemplaba parser ICS.

### 15.2 Decisión Graph READ: OAuth propio sobre `reqwest` (NO `graph-rs-sdk`)

- `graph-rs-sdk 3.0.1` con `interactive-auth` abre un webview wry para el primer
  OAuth; en la práctica, para una cuenta **personal** el **device-code** y el popup
  webview dieron fricción (redirect/logout). Se implementó **authorization-code +
  PKCE con loopback** (`redirect_uri=http://localhost:<puerto efímero>`, listener
  TCP local) usando `reqwest` (ya dependencia base) y `ring` para el RNG/nonce.
- `graph-rs-sdk` se **retira** del árbol (`calendar-graph = ["dep:aes-gcm", "dep:ring"]`).
  Menos peso, más control, y testable en las partes puras (PKCE, URL, cifrado).
- Refresh token cifrado AES-256-GCM (`aes-gcm 0.10`) con clave por-perfil
  `<profile_root>/calendar.key`.

### 15.3 Decisión ICS READ: `icalendar 0.17` (parser) — `ics` 0.5 es write-only

- **`ics 0.5.8` (ya en stack) NO parsea** — es "A library for creating ICalendar
  files". Para leer `.ics` se necesita un parser real.
- Evaluación: `ical` (Peltoche) está **ARCHIVADO** (2024) → descartado.
  `ical-rs` (pimalaya) es moderno y muy completo (RFC 5545 + 1.0 + 8984…) pero de
  adopción muy baja → riesgo a largo plazo. **`icalendar` (hoodie) v0.17.14** es el
  parser mantenido de industria: ~887k descargas, 28 contribuidores, releases
  continuos 2026, `Calendar: FromStr` + `Component::{get_start,get_end,get_summary,get_uid}`,
  MIT/Apache, Rust puro. **Elegido.**
- Integración: feature `calendar-ics`, módulo `calendar/ics_reader.rs`, CLI
  `atlas calendar sync-ics <url>`; busy windows `source=ics_local`.
- **Consolidación WRITE→`icalendar` evaluada y RECHAZADA** (ver §15.8): el
  writer de `icalendar` obliga a propiedades crudas para lo que aquí importa y
  cambiaría la salida; se mantiene `ics` para WRITE.

### 15.4 Crates (delta vs Round 5)

| Crate | Version | License | Uso | Gate |
|---|---|---|---|---|
| `icalendar` | `0.17.14` | MIT OR Apache-2.0 | ICS READ (parser) | `features = ["calendar-ics"]`, `default-features=false, features=["parser"]` |
| ~~`graph-rs-sdk`~~ | ~~3.0.1~~ | MIT | **RETIRADO** (sustituido por OAuth propio + reqwest) | — |

`aes-gcm 0.10` y `ring 0.17` permanecen para el cifrado del refresh token Graph.

### 15.5 Single-binary safety audit

- `icalendar`: pure Rust (nom + nom-language), sin native, sin build script. Single-binary safe.
- `reqwest`/`ring`/`aes-gcm`: ya presentes; sin native nuevo.
- Todas cumplen RFC 25 §11.

### 15.6 Atribución

- `src-tauri/src/calendar/ics_reader.rs` — `"Uses icalendar 0.17 (MIT OR Apache-2.0) by Hendrik Sollich (hoodie). https://github.com/hoodie/icalendar"`.
- `src-tauri/src/calendar/auth.rs` — `"OAuth 2.0 authorization-code + PKCE over reqwest; AES-256-GCM via aes-gcm 0.10; RNG via ring 0.17."`

### 15.7 URLs

- icalendar crate: https://crates.io/crates/icalendar
- icalendar repo: https://github.com/hoodie/icalendar
- ical (archivado): https://github.com/Peltoche/ical-rs
- ical-rs (pimalaya): https://github.com/pimalaya/ical
- RFC 5545 (iCalendar): https://datatracker.ietf.org/doc/html/rfc5545
- OAuth 2.0 device/loopback (RFC 8252): https://datatracker.ietf.org/doc/html/rfc8252

### 15.8 Decisión: NO consolidar el WRITE en `icalendar`

Se evaluó (docs.rs 0.17.14) retirar `ics` 0.5.8 del WRITE y usar `icalendar`
también para emitir (`ics_writer.rs`), dejando una sola crate iCalendar. **Se
rechaza.** Motivos concretos de la API del writer de `icalendar`:

- **`METHOD` y `CATEGORIES` no son propiedades tipadas**: `Calendar` sólo expone
  setters tipados para `NAME`/`DESCRIPTION`/`TIMEZONE`/`TTL`; `METHOD:PUBLISH` y
  `CATEGORIES:<status>` habría que emitirlos con `append_property(Property::new(..))`
  crudo → menos seguridad de tipos que `ics::properties::{Method, Categories}`.
- **`PRODID` fijo**: `Calendar::new()` prefija `VERSION`/`PRODID`/`CALSCALE` con su
  propio `PRODID`; para imponer `-//Atlas OS//Calendar//EN` hay que partir de
  `Calendar::empty()` y añadir a mano `VERSION`/`PRODID`/`CALSCALE`.
- **All-day cambia de forma**: `Event::all_day(NaiveDate)` emite **DTSTART y
  DTEND** con `VALUE=DATE`; nuestra salida actual (RFC 5545 §3.6.1) usa sólo
  `DTSTART;VALUE=DATE` sin `DTEND`. Aceptar el cambio obliga a reescribir los
  tests de forma de salida sin ganancia funcional.
- **Coste/beneficio**: `ics` es pure Rust (~30 KB) bajo el mismo feature opt-in
  `calendar-ics`; el ahorro es marginal y la reescritura es más código con
  propiedades crudas. Se mantiene el statu quo: `ics` = WRITE, `icalendar` = READ.
  Ambos son RFC 5545; la duplicación es de emisor/parser, no de responsabilidad.


## Anexo — Dependencias nuevas (Fase 27, RFC 63)

- **`reqwest` + feature `blocking`** (RFC 63 §4 `web.fetch`/`web.search`). Justificación:
  la crate ya está en el árbol con `json`/`stream`/`rustls-tls` (RFC 25 §3.8, cliente de
  proveedores). Habilitar `blocking` **no añade una crate nueva**: expone un
  `reqwest::blocking::Client` sobre el mismo runtime, necesario porque el trait `Tool`
  es síncrono (`fn execute`) y el registry no puede asumir un runtime tokio activo.
  Alternativa descartada: envolver cada tool async con `block_f_on`/`Runtime::new` dentro
  del tool (más frágil: panics si ya hay un reactor activo). No se bundlea nada: los
  tools de red son opt-in (`ToolRegistry::with_web_tools`) y sin backend configurado
  (`ATLAS_SEARCH_URL`) devuelven un error de tool claro en vez de panic.
- **Sin crates nuevas de parsing de diff.** `code.apply_diff` implementa el parser de
  unified-diff en Rust puro (`orchestrator/tools/code.rs`), para funcionar en el sandbox
  Linux sin depender del binario `git`.