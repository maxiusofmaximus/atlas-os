# 62 — Genealogía de proyectos base y arquitectura de Atlas OS

> **Tipo:** mapa de referencia (research). **Fecha:** 2026-10-04.
> **Para:** recordar en qué proyectos/librerías/papers se basó Atlas OS, cuál es la arquitectura base, y hacia dónde apunta el roadmap.
> **Fuentes:** RFC 00–41 + `research/27–60` + `README.md` + `Cargo.toml`.

Atlas OS se apoya en **cinco capas** de influencia. No todo es "fuente de código": hay **competidores a igualar** (sin copiar), **repos portados** (con licencia y atribución), **ecosistema auditado** (features), **stack técnico** (crates), y **teoría** (papers).

---

## Capa 1 — Referentes objetivo (competidores, NO fuente de código)

Son los "para igualar o superar". Atlas se posiciona contra ellos, no los copia.

| Proyecto | Qué se tomó como referencia |
|---|---|
| **opencode** (`opencode.ai`, el runtime del que hereda el nombre del binario) | `doom_loop` (detector mecánico), agente `plan`/`build`, sessions, permissions (`03`, `19`) |
| **Claude Code** | loop `/loop` en user-mode, skills estilo `~/.claude/skills` (`06`) |
| **Codex CLI** (OpenAI) | per-turn routing (vía `0xNatoshi/jev-codex-router`), terminal agent loop (`30`) |
| **Cursor / Cursor Cloud Agents** | "iterate until validated", demos over diffs, automations, worktrees (`17`, `21`, `22`) |
| **Windsurf** | editor + flujo agéntico (`00`) |
| **Cline / Roo** | "YOLO mode", bug "Rest of code here", auto-approve (`21`, `22`) |
| **Aider** | tri-model (architect/editor/weak), `/architect`, `/auto`, modo ask/code (`04`, `12`, `23`) |
| **Continue.dev** | "ambient" mode (`21`, `22`) |
| **Hermes** (NousResearch `nousresearch/hermes-agent`) | profiles (`?profile=`), HUD web, `/background`, memoria (`05`, `24`, `25`) |
| **OpenClaw** | extensibilidad (skills/MCP/CLI) — criticado por inseguro; Atlas lo hace seguro (`01`, `07`, `18`) |
| **AionUI** (`iofficeai/aionui`) | multi-key rotation, user tags, fail-over (`04`, `22`, `25`) |
| **Genspark** | "AI employee", super-app, MoA default (`29`) |
| **Antigravity / Gemini CLI** (Google) | agentic IDE de referencia (README) |
| **GitHub Copilot Project HydraFusion** | orquestación multi-modelo adaptativa (valida la tesis de routing) (`00 §7.2`) |
| **OpenRouter Fusion / Sakana Fugu** | "sabe cuándo cambiar de cerebro" → Model Orchestrator (`03`, `22`) |
| **Kimi** (K2.7 / k3) | pool swarm / subagentes locales ociosos (`05`) |

### Referentes añadidos (2026) — model providers, plataformas agénticas y verticales

| Producto | Tipo | Por qué es referente | Mapeo a Atlas OS |
|---|---|---|---|
| **MiniMax** (`M2`/`M2.5`/`M2.7`/`M3`; MiniMax Agent / MiniMax Code) | Model provider + harness agéntico | MoE 230B totales / 10B activos; SWE-bench Verified 69.4, Terminal-Bench fuerte; agent propio (Lightning/Pro), pesos abiertos; NVIDIA **NemoClaw** para OpenClaw always-on | Provider del Model Orchestrator; su harness compite con el coding loop (F26); NemoClaw = referencia para la brecha A de Genspark (agente persistente) |
| **GLM / Zhipu AI (Z.ai)** | Model provider + coding plan | **GLM Coding Plan** multi-herramienta (Claude Code, Cursor, Cline, OpenCode, Kilo) con MCP propios (Vision/Web Search/Web Reader/Zread); GLM-4.6→5.x; `glm-acp-agent` (agente **ACP**) | Provider; el `glm-acp-agent` usa **el mismo protocolo ACP** que RFC 28 §B — referencia directa del contrato de agente externo |
| **Blackbox AI** | Plataforma de agentes (router + multi-agente) | Router de 300+ modelos; modo **`/multi-agent`** (Blackbox/Claude Code/Codex/Gemini en **worktrees paralelos**) + **Chairman LLM** que elige la mejor implementación; ACP support; Agents API | Referente directo de **swarm + worktrees + aggregation/MoA + reliability gate**; "Chairman LLM" ≈ `orchestrator/aggregation` + `swarm/merge` |
| **Qoder** (Alibaba / Bright Zenith) | Agentic coding IDE + CLI + cloud | **Quest mode** (delegación autónoma), **Experts mode** (pipeline planning/research/coding/review/testing), **RepoWiki** (contexto hasta 100k archivos), Cloud Agents, Agent SDK, mobile | Referente del **HUD/swarm** (pipeline multi-agente) y del **Context Engine** (RepoWiki ≈ Project Map RFC 11) |
| **Manus** (Monica) | Agente general autónomo | Multi-agente Planner/Execution/Verification; sandbox cloud con FS persistente; SOTA reportado en GAIA | Referente de **Reasoning + Swarm + sandbox** (`03`, `05`, `18`) |
| **Abacus.AI (ChatLLM / AI Agent)** | Super-asistente + agent platform | 100+ modelos en uno; agente general (apps, **computer/browser use**, slides); **agent swarms**; **AutoBots self-improving** always-on sin HITL; desktop + CLI + coding agent | Referente de **Model Orchestrator multi-proveedor + Swarm + Learning autoevolutivo + remote/desktop** |
| **Vida** (`vida.io`) | **"AI Agent Operating System"** empresarial | Omnicanal (voz/SMS/email/chat/browser control), **OpenClaw-compatible**, white-label/reseller, Agent Builder/Runtime/Monitoring, multi-LLM orchestration | Colisión de posicionamiento con Atlas ("AI Agent OS"); referente de **multi-canal + cloud + reseller + compat OpenClaw** (brechas A/B de RFC 29) |
| **MecAgent** (`mecagent.com`) | Vertical: AI CAD copilot | Specs-to-CAD, generación de macros, **background drawing agent**, integración con SolidWorks/Inventor/Fusion/CATIA/Creo | Referente de **harness vertical por dominio** (skill/engine dedicado) y del patrón "background agent" |
| **Parakeet AI** (`parakeet-ai.com`) | Vertical: asistente de llamadas en tiempo real | Transcripción + respuesta en vivo, soporte de coding con captura de pantalla, knowledge base, AI notes, 52 idiomas, "invisible" | Referente de **latencia ultra-baja + input de audio/screen + memoria de sesión** (fuera del core de coding, pero modelo del copiloto en vivo) |

> **Diferenciación de posicionamiento:** Vida y Abacus ya ocupan comercialmente el término "AI Agent OS" (y Vida es además OpenClaw-compatible). Atlas OS debe defender su diferencial declarado: **local-first, single-binary, Journal auditable y seguro** — no multi-canal cloud ni reseller.

---

## Capa 1c — Catálogo extendido de referentes objetivo (130+)

Base ampliada para no ser ignorantes del estado del arte. Organizado por **función**, no por fama. Cada entrada es un objetivo a igualar/superar o una fuente de patrón. (Marcadas `★` las más directamente relevantes a la tesis de Atlas: orquestación, journal, swarm, sandbox, agent-loop.)

### A. Agentes de terminal open-source
| Proyecto | Qué es | Relevancia |
|---|---|---|
| **OpenHands** (All Hands, ex-OpenDevin) ★ | 90k★, platform GUI+CLI+SDK, ACP, Agent Canvas, sandbox Docker | Competidor directo de la tesis "plataforma de agentes" |
| **SWE-agent** (Princeton) | Agente ACI para issues reales, base de SWE-bench | Referente de interfaz agente-computadora |
| **Goose** (Block) | Agente local extensible, MCP | Referente de agente on-device con MCP |
| **Aider** | Pair-programmer diff-based, tri-model | Ya fuente de `architect/editor/weak` |
| **Plandex** | Plan-first, sandbox de diff review, 2M contexto | Referente de plan-first + isolation |
| **gptme** | Agente terminal con loops persistentes | Referente de persistencia |
| **Devon** (entropy-research) | Pair-programmer OSS con TUI + git | Referente de git-workflow agéntico |
| **AutoCodeRover** | Patch de issues con code-search | Referente de reparación automática |
| **Octomind** | Runtime de agente Rust, tap registry, 13+ providers | Referente de runtime multi-provider en Rust |
| **Coro Code / Kode CLI / QQCode / Ferrum / zot / g3 / Zap / Coro** | CLIs OSS (varios Rust) con skills/MCP/guardrails | Referentes de diseño CLI single-binary |
| **jcode** | TUI Rust, RAM-optimizado, swarm mode, 40+ providers | Referente de eficiencia + swarm + OAuth |
| **Coro Code** | Alternativa libre a Claude Code | Referente de paridad OSS |
| **OpenInterpreter / GPT-Engineer / MetaGPT / Smol Developer / Devika / AutoGPT / BabyAGI / SuperAGI** | Linaje histórico de agentes autónomos | Referente de loops autónomos tempranos |
| **Pi / Pi Agent IDE** | Harness minimal + IDE agent-native | Referente de harness minimal |

### B. Agentes de plataforma (cerrados) y CLIs corporativos
| Proyecto | Qué es | Relevancia |
|---|---|---|
| **Claude Code** (Anthropic) ★ | CLI+IDE, skills, `/loop`, plugins, tool-use | Objetivo #1 de capacidad agéntica |
| **Codex CLI** (OpenAI) ★ | Agente terminal OSS Apache-2.0, GPT-5.x-Codex | Objetivo de capacidad + open-source |
| **Gemini CLI** (Google) ★ | Agente terminal Apache-2.0, 107k★ | Objetivo + integración Google |
| **Cursor + Cursor CLI + Bugbot** ★ | IDE agéntico + `agent` CLI + review | Objetivo de producto/UI |
| **Windsurf** (Cognition) | IDE agéntico | Objetivo de producto |
| **Devin** (Cognition) | SWE autónomo shell+browser, self-healing | Objetivo de agente full-stack |
| **Amp** (Sourcegraph) | Agente CLI sobre codebases reales | Referente de code-search + agent |
| **Junie CLI** (JetBrains) | CLI LLM-agnóstico, plan mode | Referente de plan mode |
| **Cortex Code** (Snowflake) / **Tabnine CLI** / **Mentat CLI** / **Amazon Q Developer** / **GitHub Copilot CLI** | Agentes CLI enterprise | Referentes de gobernanza enterprise |
| **Warp** | Terminal agéntico | Referente de terminal AI-native |
| **Trae** (ByteDance) | IDE agéntico (SOLO) | Referente chino de IDE autónomo |
| **FetchCoder** (Fetch.ai) | Agente con ASI1, server+API | Referente de agente-as-server |

### C. Ecosistema de coding chino
| Proyecto | Qué es | Relevancia |
|---|---|---|
| **Qwen Code / Qwen-Coder** (Alibaba) | CLI OSS + modelo de código | Provider + harness |
| **Qoder + Tongyi Lingma** (Alibaba) | IDE agéntico (Quest/Experts, RepoWiki, Cloud Agents) | Referente de context-engine + multi-agente |
| **CodeGeeX** (Zhipu) | Asistente free sin límites | Referente free-tier |
| **CodeBuddy** (Tencent) | IDE+CLI+MCP, Unix-philosophy pipes | Referente de CLI composable |
| **Baidu Comate / Zulu** | IDE enterprise Baidu Cloud | Referente enterprise CN |
| **Kimi Code + K2.7/K3** (Moonshot) | Modelo + CLI de coding | Provider + harness |
| **Huawei CodeArts** / **iFlytek Spark** | Plataformas coding CN | Referentes CN |
| **DeepSeek (Coder/V3.2/V4)** | Modelos open-weight | Provider clave |
| **GLM / MiniMax** | Providers CN | Ya en Capa 1b |

### D. Harnesses, runners paralelos y orquestadores
| Proyecto | Qué es | Relevancia |
|---|---|---|
| **claude-flow** (ruvnet) | Swarms multi-agente coordinados | Referente de swarm |
| **DeerFlow** (ByteDance) | Super-agent long-horizon, sub-agentes+skills+memory+sandbox | Referente de orquestador + sandbox |
| **Symphony** (OpenAI) | Issues→runs autónomos con proof-of-work (CI/PR/video) | Referente de evidence-gated done |
| **Omnigent** (Databricks) | Meta-harness sobre Claude Code/Codex/Cursor/OpenCode/Hermes/Kiro | Referente de multi-harness |
| **claude-flow / gastown / OMK / kodo / wreckit / OpenCastle / 5dive** | Orquestadores DAG/autonomous loops; 19 especialistas; scheduling cron+heartbeat | Referente de orchestration + verify-before-done |
| **CliDeck / GridBash / tlbx / ADHDev / cmux** | Dashboards/grids para N agentes en worktrees; control remoto móvil | Referente de HUD multi-agente (RFC 24) |
| **Conductor** | worktrees por subagente | Ya fuente CN-001..008 |
| **tmux / Zellij / Overmind** | Multiplexers/supervisores | Base del "agent pane" |
| **Vibe Kanban** (BloopAI) | Kanban para agentes de coding | Referente de Kanban HUD |
| **AgentSwarms / Overbrilliant OB-1 / darce / Forge / CLAII / Nausicaa / Jazz / Smelt** | Harnesses con lanes, A2A, memoria, permisos | Referentes de infraestructura |

### E. Frameworks y SDKs de agentes
| Proyecto | Qué es | Relevancia |
|---|---|---|
| **LangGraph / LangChain** ★ | Grafo de estado durable, checkpointer, HITL | Referente de durable execution |
| **CrewAI** | Multi-agente role-based (Crews + Flows) | Referente de roles |
| **AutoGen / AG2** (Microsoft) | Multi-agente conversacional | Referente de debate |
| **Microsoft Agent Framework** | Sucesor de AutoGen+Semantic Kernel | Referente .NET/enterprise |
| **Semantic Kernel** | Planners+plugins enterprise | Referente de plugins |
| **LlamaIndex Workflows** | Event-driven document-centric | Referente de RAG/workflows |
| **Google ADK** | Runtime de agentes GCP, 5 lenguajes | Referente de runtime portable |
| **OpenAI Agents SDK** | Handoffs, guardrails, tracing | Referente de handoff chain |
| **Mastra** | Framework TS de agentes | Referente de stack TS |
| **Pydantic AI** | Agentes tipados con validación | Referente de type-safety |
| **AWS Strands Agents** | Agentes model-driven AWS | Referente cloud |
| **Dify / Flowise / n8n** | Low-code agent/workflow builders | Referente de builder (RFC 24 Canvas) |
| **Temporal** | Durable execution | Base del patrón idempotency_key (RFC 02) |

### F. Infraestructura de agentes (memoria, sandbox, vector)
| Proyecto | Qué es | Relevancia |
|---|---|---|
| **Mem0** | Memoria de largo plazo para agentes | Referente de memory tiers (RFC 09) |
| **Letta (MemGPT)** | Memoria por tiers core/archival/recall | Ya fuente del patrón |
| **Zep** | Memoria temporal con grafo | Referente de memory graph |
| **Qdrant / Weaviate / Chroma / pgvector** | Vector DBs | Referentes de Vector KB |
| **Daytona** | Sandboxes sub-90ms para código IA | Referente de sandbox (RFC 18) |
| **E2B / Modal** | Sandboxes de código en cloud | Referentes de ejecución aislada |
| **Firecracker / gVisor** | MicroVM/sandbox runtime | Base potencial de sandbox |
| **mksglu/context-mode** | FTS5/BM25 + session memory | Ya fuente RFC 35 |

### G. Cloud dev environments
| Proyecto | Qué es | Relevancia |
|---|---|---|
| **Coder** | CDE self-hosted enterprise | Referente de remote workspace |
| **Gitpod / Ona** | CDE ephemeral; Gitpod→Ona (mission control) | Referente de remote + agentes |
| **GitHub Codespaces** | CDE gestionado | Referente de UX cloud |
| **DevPod** | CDE client-only devcontainer | Referente de portabilidad |
| **code-server / openvscode-server** | VS Code en browser | Referente de IDE remoto (RFC 17/24) |
| **Devcontainers spec** | Spec de entorno reproducible | Base de sandbox reproducible |
| **Bunnyshell** | Entornos efímeros | Referente de preview envs |

### H. Observabilidad, evals y gateways LLM
| Proyecto | Qué es | Relevancia |
|---|---|---|
| **Langfuse** (DE, MIT) ★ | Tracing + evals + prompts self-hosted | Referente de observabilidad (Journal/HUD) |
| **LangSmith** | Plataforma de agent engineering (observe/eval/deploy) | Referente de evaluación |
| **Arize Phoenix** | OSS tracing OpenTelemetry/OpenInference | Referente de tracing estándar |
| **Braintrust** | Evals colaborativas + datasets | Referente de golden datasets (EVAL) |
| **MLflow** | Plataforma AI engineering (Apache-2.0) | Referente de tracking |
| **Helicone / Portkey / LiteLLM / OpenRouter / ZenMux** ★ | Gateways multi-provider, cache, fallback, cost | Referente directo del Model Orchestrator |
| **TruLens / Comet Opik / Lunary / W&B Weave** | Evals/instrumentación | Referentes de eval |
| **OpenTelemetry GenAI** | Convenciones semánticas de telemetría | Base de trazas agénticas |
| **AgentOps** | Sesiones de agentes autónomos | Referente de replay |

### I. Code review y calidad
| Proyecto | Qué es | Relevancia |
|---|---|---|
| **Qodo** (ex-CodiumAI) | Review + governance + rules system | Referente de quality gate (RFC 14) |
| **CodeRabbit** | Review agentic en PR/IDE/CLI | Referente de review |
| **Greptile** | Indexa repo→grafo y revisa con contexto total | Referente de full-context review |
| **Graphite** | Stacked PRs + review (adquirido por Cursor) | Referente de stacked diffs |
| **Cursor Bugbot / GitHub Copilot Code Review** | Review nativo | Referentes |
| **Snyk / SonarQube / Codacy / DeepSource** | SAST/quality | Referentes de security_scan/lint stages |
| **Sourcegraph** | Code intelligence + Amp | Referente de code-search |
| **Qodex** (≠ Qodo) | Review con probes en runtime | Referente de runtime evidence |

### J. Model providers y modelos de coding
| Proyecto | Relevancia |
|---|---|
| **Mistral** (Devstral/Codestral/Magistral/Le Chat) | Provider EU + modelo de coding |
| **DeepSeek (Coder)** | Provider open-weight |
| **Qwen (Coder)** | Provider open-weight |
| **Meta Llama / Google Gemma / Ai2 OLMo** | Open-weight base |
| **Cohere (Command/North)** | Enterprise, se fusionó con Aleph Alpha |
| **Moonshot Kimi / Zhipu GLM / MiniMax / ByteDance Seed (Doubao)** | Providers CN |
| **Poolside** | Startup de coding AI (FR) |
| **Cursor Composer / SWE-1 (Cognition)** | Modelos verticales de coding |
| **Nvidia NIM** | Runtime de inferencia (operador ya lo usa) |

### K. Ecosistema europeo (🇪🇺)
| Proyecto | País | Relevancia |
|---|---|---|
| **Mistral AI** | FR | Campeón EU, open-weight, soberanía |
| **Aleph Alpha** (+ PhariaAI) | DE | Soberanía enterprise/regulada (fusionada con Cohere) |
| **DeepL** | DE | Traducción GDPR-native |
| **Hugging Face** | FR | Hub open-source |
| **Dust** | FR | Plataforma de agentes enterprise |
| **LightOn** | FR | LLM soberano (Paradigm RAG) |
| **Nabla** | FR | Copiloto médico |
| **H Company** | FR | Agentes + computer-use |
| **Kyutai** | FR | Research OSS (voz Moshi) |
| **Photoroom** | FR | Edición de imagen |
| **Stability AI** | UK | Stable Diffusion |
| **Synthesia** | UK | Video con avatares |
| **Silo AI (AMD)** | FI | LLMs nórdicos (Poro/Viking) |
| **Helsing / Wayve** | DE/UK | Defensa / conducción autónoma |
| **Qdrant / Langfuse / n8n** | DE | Vector DB / observabilidad / automatización |
| **Weaviate** | NL | Vector DB |

### L. Agentes verticales
| Proyecto | Dominio | Relevancia |
|---|---|---|
| **Harvey** | Legal | "Operating system for legal" + Agent Builder (500+ agentes) |
| **Rogo (Felix)** | Finanzas | Agente que produce deliverables (decks/models) |
| **Sierra (Horizon)** | Customer service | Agentes proactivos multi-canal |
| **Hebbia** | Finanzas | Análisis documental |
| **Abridge / OpenEvidence / Ambience / Hippocratic AI** | Salud | Agentes clínicos |
| **Decagon / Parloa** | CX | Agentes de atención + voz (Parloa DE) |
| **Legora** | Legal (SE) | Agente legal EU |
| **Cognition Devin (vertical SWE)** | Ingeniería | Ya en B |

### M. CAD / 3D / ingeniería (vertical de referencia del operador)
| Proyecto | Qué es | Relevancia |
|---|---|---|
| **OpenCADStudio** (HakanSeven12) ★ | CAD open-source para ingeniería civil/topografía (.NET) | Ejemplo del usuario: vertical CAD OSS |
| **FreeCAD** | CAD paramétrico OSS | Base para harness CAD (MCP) |
| **OpenSCAD** | CAD programático | Salida de text-to-CAD |
| **CadQuery** | CAD como Python | Representación ejecutable para LLMs |
| **Zoo (ex-KittyCAD, KCL)** | Text-to-CAD con kernel B-Rep propio (STEP) | Referente de text-to-CAD serio |
| **Backflip AI** | Scan/STL→CAD paramétrico (feature trees) | Referente de reversión a CAD |
| **Adam CAD (CADAM)** | Web app OSS text-to-CAD (OpenSCAD + Claude) | Referente OSS text-to-CAD |
| **MecAgent** | Copiloto CAD en SolidWorks/Inventor/Fusion | Ya en Capa 1b |
| **Onshape / Shapr3D / Spline** | CAD/3D modernos | Referentes de UX 3D |
| **Physna** | Búsqueda geométrica 3D | Referente de part search |
| **Text-to-CadQuery** (arxiv 2505.06507) | Dataset+modelo text→CadQuery | Referente académico |

### N. Protocolos de interoperabilidad de agentes
| Proyecto | Qué es | Relevancia |
|---|---|---|
| **MCP** (Anthropic) ★ | Agente↔herramientas/datos | RFC 07, ya base |
| **A2A** (Google, Linux Foundation) ★ | Agente↔agente (JSON-RPC/HTTPS) | Futuro de swarm distribuido |
| **ACP** (Zed/Block) ★ | Agente↔cliente de editor (stdio) | RFC 28 §B, ya implementado |
| **ACP→A2A** (IBM BeeAI) | Protocolo agente↔agente REST | Consolidado bajo Linux Foundation |
| **AG-UI** (CopilotKit) | Agente↔usuario (event-based, SSE/WS) | Referente para HUD en vivo |
| **ANP / Agora / LMOS** | Protocolos emergentes de agentes | Vigilar |
| **A2UI / MCP-UI / Open JSON UI** | Specs de UI generativa | Referente de HUD data-driven (RFC 24) |

### O. Benchmarks y harnesses de evaluación
| Proyecto | Relevancia |
|---|---|
| **SWE-bench / SWE-bench Verified / Multi-SWE-bench** | Métrica de reparación real |
| **Terminal-Bench 2** | Métrica de agente terminal (Atlas: 0.000 hoy) |
| **GAIA / AgentCompany** | Agentes generalistas |
| **WebArena / OSWorld / BrowseComp** | Navegación y computer-use |
| **SWE-Lancer** | Tareas de ingeniería con valor económico |
| **Harbor** | Harness de eval (ya en `tools/harbor_atlas/`) |

**Total capa 1c: ~150 referentes** (A–O). Se suman a los ~25 de Capas 1/1b → **~175 proyectos objetivo** catalogados.

### Lectura estratégica — los 12 patrones a adoptar/observar primero

1. **Durable execution con checkpointer** (LangGraph, Temporal, Symphony) → ya tenemos Journal; falta el "resume exacto" formal end-to-end.
2. **Sandbox de ejecución** (Daytona, E2B, Modal, gVisor/Firecracker) → RFC 18 está en tipos; falta runtime real.
3. **Chairman/aggregation multi-agente** (Blackbox, Qoder Experts, claude-flow) → `orchestrator/aggregation` + `swarm/merge` es el análogo.
4. **Per-turn routing + cost gateway** (OpenRouter, LiteLLM, Portkey, Helicone) → Phase 2 ya lo cubre; falta streaming/partial.
5. **Evidence-gated done** (Symphony proof-of-work, Qodo, Qodex) → RFC 14 `EvidenceGate` existe; falta rigor de "proof".
6. **Memory graph / tiers** (Letta, Mem0, Zep) → RFC 09/16; hoy `fastembed` apagado por defecto.
7. **ACP/A2A/AG-UI** (Zed/Block, Google, CopilotKit) → ACP ✅; **A2A y AG-UI son el próximo salto de interop**.
8. **Text-to-CAD / harness vertical** (Zoo, CADAM, Backflip, OpenCADStudio, MecAgent) → modelo de "engine por dominio" (Skills + MCP).
9. **Observabilidad estándar** (Langfuse, Phoenix OpenInference, OTel GenAI) → emitir trazas OTel haría el Journal interoperable.
10. **Multi-canal + cloud persistence** (Vida, Sierra, Abacus) → brechas A/B de RFC 29.
11. **Worktree isolation + remote control móvil** (CliDeck, GridBash, tlbx, Conductor) → RFC 05/24; falta el panel.
12. **European sovereign stack** (Mistral, Aleph Alpha, Qdrant, Langfuse, n8n) → alineación natural con *local-first/single-binary/GDPR* de Atlas.

---

## Capa 1d — Catálogo creativo, de producto y "mindful" (+120)

Ampliación pedida por el operador: proyectos hermosos y valiosos que **no se deben menospreciar**, documentados por siembra. Se organizan por función.

### Los seis añadidos directos (identificación corregida por el operador)

| Proyecto | Qué es | Relevancia para Atlas |
|---|---|---|
| **Higgsfield** (higgsfield.ai) ★ | Suite creativa AI-native: 50+ modelos de video/imagen/audio, Cinema Studio, Agentic Engine (canvas nodal), **MCP + CLI**, agente "Supercomputer" (GPT-6). Fundada 2023; ~$4B val. | Referente de plataforma creativa multi-modelo + MCP/CLI + agente propio |
| **mr-mak-workspace** (`witnesstodark`, MIT, 295★) ★ | Workspace de escritorio **local (Tauri)** para Codex/Claude Code/OpenCode: reports, proyectos, **20 skills creativas** (Blender, Three.js, fal.ai, **Higgsfield workflows**, character sheets, motion), voice coordinator opcional (OpenAI Live + Codex), panel MCP. Windows x64 + Linux. | Primo directo de Atlas: Tauri + workspace local + skills creativas + MCP + multi-CLI |
| **Flova AI** (flova.ai) ★ | Plataforma **Agente de video** (Agent-Native Video Production): planifica, storyboard, genera assets, ensambla timeline, project memory, Skills reutilizables, Agent Canvas, **CLI que conecta Codex/Claude Code**. | Referente de agente vertical creativo + skills + canvas + CLI-puente a CLIs de código |
| **Tesana** (tesana.ai) | **AI game builder**: describe el juego → agente construye juego jugable real (engine propio "Atomos"), assets 3D/audio, multijugador, estudio, código propio exportable. | Referente de "vertical engine + agente" (texto→producto ejecutable) |
| **Cave** = **Cave Engine** (Uniday Studio — Guilherme Teres Nunes) ★ | Motor de juegos 3D de escritorio, **C++ (~200k LOC) + scripting Python** (pybind11), editor completo, **Logic Bricks** (visual scripting), terrain/water, timeline de cutscenes; pago, **sin royalties**, Windows/Linux. **NO está vibe-coded** (10+ años de un solo dev). Importa Blender vía FBX/OBJ/Assimp y `.blend`→level descriptor. | Referente del modelo **"un engine vertical por dominio"** (C++ core + capa scripting + editor propio) y de **integración DCC→engine** (Blender como content tool). *(Homónimos reales y distintos, NO este: `withcave.ai` orquestador self-hosted y **CaveAgent** académico arXiv 2601.01569.)* |
| **Stove** = **Stove 3D** (mismo dev de Cave Engine, Uniday Studio) ★ | **Vibe-coded** 3D modeling software, **alternativa ligera a Blender** para assets de juego low-poly/retro/hand-painted: workflow de 3 pasos **View · Model · Paint**, **sin UV mapping**, exporta game-ready; Windows/Linux (+Mac nativo en camino). | Caso de estudio ideal de **"vibe-codear un producto real"** (un DCC tool completo) por un dev de engines; referente de **herramienta enfocada** (menos features, menos fricción) — la tesis "haz una cosa muy bien". *(Homónimos: Stove Health, Stove Finance, Stove test-harness — no son este.)* |

### P. Plataformas de generación de video/imagen/audio (creativas)
> Sora (OpenAI), Veo (Google), Kling (Kuaishou), Runway (Gen-4), Pika, Luma AI (Dream Machine/Ray), Hailuo (MiniMax), Wan (Alibaba), Seedance/Seedream (ByteDance), Vidu (Shengshu), LTX Studio (Lightricks), HeyGen, Synthesia, Colossyan, D-ID, Hedra, Captions, InVideo AI, Descript, Opus Clip, Midjourney, Black Forest Labs FLUX, Ideogram, Recraft, Leonardo AI, Adobe Firefly, Freepik AI.
**Relevancia:** providers multimodales del Model Orchestrator; patrón de "AI-native creative suite" (Higgsfield) aplicable a HUD/artifacts.

### Q. Generación 3D y game builders
> Tesana (ya), **Cave Engine** (Uniday — engine 3D C++/Python, ya arriba), **Rosebud AI** (games por prompt), **Roblox Cube**, **Unity Muse/Sentis**, **Unreal + AI**, **Scenario** (assets de juego), **Layer.ai**, **Meshy**, **Tripo AI**, **Hyper3D Rodin**, **Hunyuan3D**, **Luma Genie**, **Spline AI**, **Sloyd**.
**Relevancia:** `img2threejs`/Blender MCP de mr-mak; patrón de "engine vertical + agente" (como un engine de Atlas por dominio).

### R. Workspaces y app-builders con agente
> **mr-mak-workspace** (ya) ★, **Bolt.new / bolt.diy**, **v0** (Vercel), **Lovable**, **Replit Agent**, **Dyad** (local), **Onlook** (visual editor), **Same.dev**, **Pythagora/GPT-Pilot**, **Create.xyz**, **Firebase Studio** (Google), **Glide/Softr** (no-code+AI).
**Relevancia:** competidores directos del "HUD/workspace local" y del app-builder agéntico.

### S. Memoria, compañeros y agentes "mindful"
> **Cave** (`incave.io`, compañero con memoria — homónimo, NO Cave Engine), **Character.AI**, **Replika**, **Pi** (Inflection), **Nomi**, **Kindroid**, **Dot**, **Personal.ai**, **Limitless/Rewind**, **Mem (mem.ai)**, **Notion AI**, **Reflect**, **Tana**.
**Relevancia:** memoria personalizada (RFC 09/16) y el ángulo "AI que recuerda y acompaña" (complementa brecha C de RFC 29).

### T. Salud, bienestar y cuidado (mindful/vertical)
> **Stove Health** (ya), **Nabla**, **Abridge**, **OpenEvidence**, **Ambience**, **Woebot/Wysa** (salud mental), **Ada Health**, **K Health**, **Summer Health**, **Function Health**, **Whoop Coach**, **Hippocratic AI**.
**Relevancia:** verticales regulados; patrón de agente con guardrails de seguridad (RFC 18) y skills OWASP/GDPR/HIPAA ya bundled.

### U. Agentes verticales legales/financieros
> **Harvey**, **Rogo**, **Hebbia**, **EvenUp**, **Spellbook**, **Ironclad**, **Klarity** (legal), **Ramp Intelligence**, **Sierra**, **Legora** (EU).
**Relevancia:** patrón "domain agent + Agent Builder + workflows" (500+ agentes preconstruidos).

### V. Tooling local / self-hosted open-source
> **Ollama**, **LM Studio**, **llama.cpp**, **vLLM**, **SGLang**, **Open WebUI**, **LibreChat**, **Jan**, **GPT4All**, **AnythingLLM**, **LocalAI**, **text-generation-webui**, **SillyTavern**, **ComfyUI**, **AUTOMATIC1111 (SD WebUI)**, **Fooocus**, **InvokeAI**.
**Relevancia:** los **3 providers locales** de RFC 25 §3.8 y el pilar *local-first*; ComfyUI/SD como pipelines creativos.

### W. Motor 3D / game engines / assets / DCC
> **Blender** (+MCP), **Stove 3D** (Uniday — alternativa Blender vibe-coded, ya arriba), **UPBGE** (fork de Blender Game Engine), **Armory3D** (engine para Blender), **Cave Engine** (ya), **Godot**, **Bevy** (Rust), **Lumix Engine**, **GDevelop**, **Rogue Engine** (Three.js), **Game Pencil Engine**, **Blockbench**, **MagicaVoxel**, **Cascadeur**, **Mixamo**, **Sketchfab**, **Poly Haven**, **Poly Pizza**, **Vibe4D** (asistente IA para Blender), **CSM Coder** (text-to-code bpy).
**Relevancia:** cluster **Blender/DCC + engines** directamente ligado al harness creativo del operador (Blender MCP, Three.js); Stove 3D es el caso de estudio de "vibe-codear una herramienta de creación" y Cave Engine del "engine vertical".

### X. Audio, voz y música
> **ElevenLabs**, **Suno**, **Udio**, **Stable Audio**, **Deepgram**, **fal.ai**, **Replicate**, **Together AI**.
**Relevancia:** providers de las capacidades voz/audio del orquestador y del voice coordinator (mr-mak).

**Total capa 1d: ~125 referentes.** Sumado a Capas 1/1b/1c → **~305 proyectos objetivo catalogados**.

### Capa 1e — Puntos de apoyo desde favoritos (URL + mapeo a engine/RFC)

> Fuente: `Edge/User Data/Default/Bookmarks` → **Personal** (2.513 URLs). Curados los relevantes; excluido el grueso personal (manga/novelas/juegos/adulto/empleo/educación). `Mapeo Atlas` = engine (RFC) donde encaja.

**Descubrimiento clave:** los favoritos confirman el **toolchain real del operador** (Blender/Unity/Unreal/Godot, NIM/Groq/Cerebras/OpenRouter, Meshy/Tripo/Kaedim, Warp/Kiro/Amp/MecAgent) — alineado con la tesis de Atlas.

**AI coding / agentes / IDE**

| Punto de apoyo | URL | Mapeo Atlas |
|---|---|---|
| Warp | https://www.warp.dev/ | Coding (13), CLI (08) |
| Kiro (AWS) | https://kiro.dev/ | Coding (13) |
| Monica | https://monica.im/ | Lateral (28) asistente |
| Amp | https://ampcode.com/ | Coding (13) |
| fx.sh | https://fx.sh/ | Coding (13) — agente nativo tiny |
| Rocket.new | https://www.rocket.new/ | HUD/app-builder (17/24) |
| Base44 | https://base44.com/ | HUD/app-builder (24) |
| Lovable | https://lovable.dev/ | HUD/app-builder (24) |
| v0 (Vercel) | https://v0.app/ | HUD/app-builder (24) |
| Replit | https://replit.com/ | HUD/cloud (24/25) |
| InsForge | https://insforge.dev/ | Lateral backend (28), Skills (06) |
| Seenode | https://seenode.com/ | Lateral deploy (28) |
| PortKiller | https://portkiller.app/ · https://github.com/productdevbook/port-killer | CLI util (08) |
| Cursor MCP Directory | https://docs.cursor.com/tools | MCP (07) |
| GitHub Copilot quickstart | https://docs.github.com/en/copilot/quickstart | Lateral (28) |
| GitHub HydraFusion | https://github.blog/ai-and-ml/github-copilot/ | Orchestrator (04), RFC 00 §7.2 |
| Status Claude | https://status.claude.com/ | Provider health (04, G7) |
| MecAgent | https://mecagent.com/ | Vertical CAD (06/28) |

**MCP / skills / spec**

| Punto de apoyo | URL | Mapeo Atlas |
|---|---|---|
| skills.sh | https://skills.sh/ | Skills (06) |
| Pieces MCP | https://pieces.app/features/mcp | MCP (07) |
| obra/superpowers | https://github.com/obra/superpowers | Skills/Swarm (06/05) |
| wshobson/agents | https://github.com/wshobson/agents | Swarm (05) |
| agents.md | https://agents.md/ | Skills/CLI (06/08) |
| OpenSpec | https://openspec.dev/ | Planning (12), Prompt (23) |
| Pencil | https://www.pencil.dev/ | UI/Coding (17/13) |
| PicoBerry | https://teaser.picoberry.ai/ | Lateral 3D (28) |
| MeshTailor | https://meshtailor.github.io/ | Lateral 3D (28) |
| MobiAI.dev | https://mobiai.dev/ | Mobile (38) |

**Providers / infra IA**

| Punto de apoyo | URL | Mapeo Atlas |
|---|---|---|
| Cerebras | https://www.cerebras.ai/ | Model Orchestrator (04) |
| Groq | https://groq.com/ | (04) |
| OpenRouter | https://openrouter.ai/ | (04) |
| NVIDIA NIM | https://build.nvidia.com/models | (04) |
| SambaNova | https://cloud.sambanova.ai/ | (04) |
| Mistral | https://mistral.ai/ | (04) |
| GooseAI | https://goose.ai/ | (04) |
| Unsloth | https://unsloth.ai/docs | Learning/fine-tuning (16) |
| CanIRun.ai | https://www.canirun.ai/ | Stack/local (25) |
| llmfit | https://llmfit.org/ | Stack/local (25) |
| llm-stats | https://llm-stats.com/ | Evals (22/EVAL) |
| Artificial Analysis | https://artificialanalysis.ai/ | Evals (22) |
| arena.ai | https://arena.ai/ | Evals (22) |
| Hugging Face | https://huggingface.co/ | Models/Vector (04/09) |
| Decodo | https://decodo.com/ | Research/scraping (10) |
| AIsa | https://aisa.one/ | Lateral agentic (28) |
| Hunyuan3D | https://3d.hunyuan.tencent.com/ | Lateral 3D (28) |
| Stable Diffusion | https://stablediffusionweb.com/ | Lateral imagen (28) |
| Yeri AI | https://yeri.ai/ | Lateral imagen (28) |
| Midjourney | https://www.midjourney.com/ | Lateral imagen (28) |
| Freepik AI | https://www.freepik.com/ai | Lateral gen (28) |
| Suno | https://suno.com/ | Lateral audio (28) |
| Mureka | https://mureka.ai/ | Lateral audio (28) |
| Fish Audio | https://fish.audio/ | Lateral TTS (28) |
| Pika | https://pika.art/ | Lateral video (28) |
| Luma | https://lumalabs.ai/ | Lateral video/3D (28) |
| Topview | https://topview.ai/ | Lateral video agent (28) |

**3D / game dev / DCC**

| Punto de apoyo | URL | Mapeo Atlas |
|---|---|---|
| Superhive (Blender Market) | https://superhivemarket.com/ | Lateral Blender (28) |
| Quixel Mixer | https://quixel.com/products/mixer | Lateral 3D (28) |
| ReShade | https://reshade.me/ | Lateral 3D |
| Unity | https://unity.com/ | Vertical engine (06/28) |
| Unity Asset Store | https://assetstore.unity.com/ | Assets |
| Unity Gaming Services | https://dashboard.unity3d.com/ | Infra |
| Bolt Visual Scripting | https://docs.unity3d.com/Packages/com.unity.bolt@1.4/manual/bolt-installation.html | Vertical |
| Code Monkey (cursos Unity) | https://unitycodemonkey.com/freecourses.php | Learning (16) |
| Unreal Marketplace | https://unrealengine.com/marketplace/ | Assets |
| MetaHuman | https://www.metahuman.com/ | Lateral 3D (28) |
| Spine | https://es.esotericsoftware.com/ | Lateral animación 2D (28) |
| Modddif | https://modddif.com/ | Lateral 3D (28) |
| OpenPencil | https://openpencil.dev/ | UI/design (17) |
| drawDB | https://www.drawdb.app/ | Planning/modelado (12) |
| Story Engine | https://storyengine.live/ | Content/world (16) |

**3D / assets**

| Punto de apoyo | URL | Mapeo Atlas |
|---|---|---|
| Kenney | https://kenney.nl/ | Assets |
| OpenGameArt | https://opengameart.org/ | Assets |
| Poly Pizza | https://poly.pizza/ | Assets (ya en MCP Blender) |
| ambientCG | https://ambientcg.com/ | Assets |
| Poly Haven | https://polyhaven.com/ | Assets |
| freepbr | https://freepbr.com/ | Assets |
| textures.com | https://www.textures.com/ | Assets |
| Poliigon | https://www.poliigon.com/ | Assets |
| ShareTextures | https://www.sharetextures.com/ | Assets |
| Sketchfab | https://sketchfab.com/ | Assets |
| CGTrader | https://www.cgtrader.com/ | Assets |
| GameBanana | https://gamebanana.com/ | Assets |
| S2V (Source 2 viewer) | https://s2v.app/ | Herramienta |
| Pixabay | https://pixabay.com/ | Assets |
| Meshy | https://www.meshy.ai/ | Gen 3D |
| Tripo | https://studio.tripo3d.ai/ | Gen 3D |
| Kaedim | https://www.kaedim3d.com/ | Gen 3D |
| 3D AI Studio | https://www.3daistudio.com/ | Gen 3D |
| Top3D.ai | https://www.top3d.ai/ | Comparador gen 3D |
| 80.lv (retopología IA) | https://80.lv/articles/new-ai-powered-tool-to-automate-retopology-for-3d-artists | Gen 3D/ref |
| NeuralFur | https://neuralfur.is.tue.mpg.de/ | Gen 3D/ref |
| TRELLIS.2 | https://microsoft.github.io/TRELLIS.2/ | Gen 3D |

**Infra / backend / deploy**

| Punto de apoyo | URL | Mapeo Atlas |
|---|---|---|
| Laragon | https://laragon.org/download | Dev env (25) |
| XAMPP / Bitnami | https://www.apachefriends.org/ · https://bitnami.com/ | Dev env |
| Contabo | https://contabo.com/ | VPS/sandbox (18/25) |
| Cloudways | https://www.cloudways.com/ | VPS |
| Hostinger | https://www.hostinger.com/ | VPS |
| alwaysdata | https://www.alwaysdata.com/ | Hosting |
| Railway | https://railway.app/ | Deploy |
| Render | https://render.com/ | Deploy |
| Neon | https://neon.tech/ | Postgres serverless |
| Vercel | https://vercel.com/ | Deploy |
| Netlify | https://www.netlify.com/ | Deploy |
| Cloudinary | https://cloudinary.com/ | Media |
| Supabase | https://supabase.com/ | Backend/DB |
| MongoDB Atlas | https://www.mongodb.com/atlas | DB |
| Turso | https://github.com/tursodatabase/turso | DB SQLite — análogo |
| DuckDNS | https://www.duckdns.org/ | Remote (25) |
| Termius | https://termius.com/ | SSH/remote (25) |
| Synergy | https://symless.com/synergy | Remote-live (8.5) |
| Radmin VPN / Hamachi | https://www.radmin-vpn.com/ · https://vpn.net/ | Remote |
| Tailscale | https://tailscale.com/ | Remote/VPN (25) |
| Shadow.tech | https://shadow.tech/ | Remote-live (8.5) |
| MSYS2 | https://www.msys2.org/ | Toolchain |
| Zeal | https://zealdocs.org/ | Docs offline (11) |
| DevDocs | https://devdocs.io/ | Docs (11) |
| JSON Crack | https://jsoncrack.com/ | Tooling |
| n8n-workflows | https://github.com/Zie619/n8n-workflows | Automations (24) |

**Learning / benchmarks / FOSS**

| Punto de apoyo | URL | Mapeo Atlas |
|---|---|---|
| roadmap.sh | https://roadmap.sh/ | Skills (06) |
| Boot.dev | https://www.boot.dev/ | Skills (06) |
| Frontend Mentor | https://www.frontendmentor.io/ | Evals/bench (22) |
| freeCodeCamp | https://www.freecodecamp.org/ | Learning (16) |
| VisuAlgo | https://visualgo.net/ | Learning (16) |
| BridgeBench | https://www.bridgebench.ai/ | Evals (22/EVAL) |
| OSSU / TeachYourselfCS | https://ossu.firebaseapp.com/ · https://teachyourselfcs.com/ | Learning |
| adventJS | https://adventjs.dev/ | Evals |
| codewars / leetcode / hackerrank | https://www.codewars.com/ · https://leetcode.com/ · https://www.hackerrank.com/ | Evals |
| fullstackopen | https://fullstackopen.com/ | Skills |
| Open Bootcamp / Codely / EDteam | https://open-bootcamp.com/ · https://codely.com/ · https://ed.team/ | Learning |
| it's free* | https://itsfree.dev/ | FOSS catalog (22) |
| NoSubscription.org | https://nosubscription.org/ | FOSS catalog |
| LearnXinYMinutes | https://learnxinyminutes.com/ | Learning |

**Ya presentes en el catálogo (overlap):** Higgsfield · MecAgent · Manus · Abacus · Vida · Parakeet · Blackbox · taste-skill · grill-me · img2threejs · Cave Engine · Luma · Suno · Freepik · Mistral · OpenRouter · Groq · Cerebras · NVIDIA NIM · Poly Haven · Poly Pizza · Sketchfab · Supabase.

**Total capa 1e: ~120 puntos de apoyo** desde favoritos → **~425 proyectos/recursos objetivo catalogados** en total.

---

## Capa 2 — Fuentes portadas (RFC 28 §A–§I + research)

Aquí sí hay código/patrón portado, con licencia y atribución obligatoria.

| § | Fuente | Licencia | Qué se portó | Estado |
|---|---|---|---|---|
| §A | `karpathy/autoresearch` | MIT | Loop hill-climbing (prepare/train/program.md, `results.tsv`, NEVER_STOP) | ✅ Fase 1.5b |
| §B | `microsoft/intelligent-terminal` | MIT | ACP server (agent-client-protocol), `wtcli`, OSC 9001, slash commands | ✅ Fase 1.5d/18 |
| §C | `safishamsi/graphify` → `graphify-labs/graphify` | Apache-2.0 | Knowledge graph: Leiden(→petgraph), tree-sitter extractores, report, atomic write | ✅ Fase 1.5c |
| §D | `darrenburns/posting` | Apache-2.0 | Formato YAML `.posting.yaml` para export del AuditLog | ✅ Fase 1.5a |
| §E | `firecrawl/firecrawl` (crate `firecrawl`, MIT) + `firecrawl/anydoc` + `firecrawl-mcp-server` | MIT | Web ingestion polyfacética (adapter facade) | ✅ Fase 1.5e |
| §F | `atifchy/winrt-toast` (`winrt-toast-reborn`) | MIT | Windows Toast + AUMID + deep-link | ✅ Fase 1.5f |
| §G | `hummingly/ics` (write) + `sreeise/graph-rs-sdk` (read, luego retirado a reqwest) | MIT | Calendar `.ics` + Microsoft Graph | ✅ v3.1.x |
| §H | `cline/cline` PRs #10207 / #10963 / #10141 | Apache-2.0 | `SpendLimitError` card, Retry-After, bail-out threshold | ✅ Fase 1.5h |
| §I | `zenbu-labs/terminal-browser` | MIT | Navegador en el pane del terminal (kitty graphics) — integración lateral | 📄 documentado |
| — | **Conductor** (`conductor.build`) | — | 8 patrones CN-001..008 (worktrees por subagente, auto-rebase) | ✅ Fase 4 |

`research/28 - conductor & alt surfaces.md` define además el **modelo lateral** ("patrón 8.5"): tools externas por proceso, **jamás bundleadas** (RustDesk, artemis, terminal-browser).

---

## Capa 3 — Ecosistema auditado (rondas de feature-mining)

Features propuestas/adoptadas a partir de repos externos.

### RFC 27 — Orquestadores de terminal
`bufanoc/tmux-orchestrator-ai-code`, `stablyai/orca` (27.4k★), `ogulcancelik/herdr` (Rust puro, 20k★), `traycerai/traycer` (spec-first). → 6 principios (worktree isolation, at-a-glance, remote attach, reflexivity, BYOA, spec-first) + 8 brechas A–H.

### RFC 29 — Genspark
3 brechas estructurales: A cloud persistence ("AI employee"), B gateway multi-canal, C user modeling (Honcho-style).

### RFC 30 — Ecosystem & Jev Audit (20 repos "Jev")
`browser-use/jev-ultrafast` (routing por coste), `tamaratran/fast-jev-compaction` (compaction con modelo barato), `vercel-labs/json-render` (UI generativa), `jarrodwatts/jev-trader`, `lahfir/agent-desktop` (computer-use Rust), `devagrawal09/jev-review` (reviewer+dashboard), `fhshaik/typesafe-mario`, `jkudish/jev-mcp`, `itsmostafa/typesafe-mcp`, **`0xNatoshi/jev-codex-router`** (per-turn routing), `RomanSlack/jev-drone`, `monteduro/killmyidea`. + referencias: **Context7Max** (propio), **anydoc**, **grill-me** (`mattpocock/skills`), **taste-skill** (`leonxlnx`), `voltagent/awesome-design-md`, **img2threejs**, **archify** (`tt-a1i`), **agency-agents** (`msitarzewski`, 154k★), **munder-difflin** (`chaitanyagiri`), `calesthio/openmontage`, `deepseek-ai/deepseek-harness`, `arisguimera/mobiai-core`, `h8man/navmeshplus`, `omacom/omarchy`.

### RFC 35 — Round 7
`cloudflare/security-audit-skill` (findings.json + validator), `mksglu/context-mode` (FTS5/BM25 + session memory), `ajeetdsouza/zoxide` (frecency), `aovestdipaperino/laya-rust` ("System One" — ModernBERT + RL head).

### RFC 38 — Round 8 (el proyecto de Google)
**`google/artemis`** — automatización Android por lenguaje natural, MCP nativo (`mobile_run_task`), CLI `--profile flash/pro`, Apache-2.0. Integración **lateral** (`atlas mobile`), jamás bundling. + `minitap-ai/mobile-use`.

### RFC 41 — Round 9
`maxiusofmaximus/MaxAppsHub` (launcher Android del operador — AppRegistry, updates vía GitHub Releases) + toolchain Tauri Android.

### research/ — fuentes extras
`elbruno/graphify-dotnet` (alternativa .NET a graphify), `orange-opensource/hurl` (rechazada: DSL > YAML), `lizardbyte/sunshine` (remote-live), `lm-sys/routellm`, `64bit/async-openai`, `harbor-framework/harbor` (eval harness).

---

## Capa 4 — Stack técnico y dependencias (RFC 25 + `Cargo.toml`)

| Área | Tecnología |
|---|---|
| Shell desktop | **Tauri 2** (WebView2/WKWebView/WebKitGTK) |
| Frontend | **SvelteKit 2 + Svelte 5 runes**, `adapter-static` (CSR) |
| Lenguaje core | **Rust 1.84+**, edition 2021 |
| Persistencia | **SQLite** (`rusqlite` bundled) + **`sqlite-vec`** (`asg017/sqlite-vec`) |
| Embeddings | **`fastembed-rs`** (ONNX, sin Python) — *feature-gated, default off* |
| HUD/API | **axum 0.7** + WebSocket en `127.0.0.1:0` (sobrevive al webview) |
| LSP | **tower-lsp 0.20** (proxy/multiplexer) |
| HTTP modelos | **reqwest 0.12** (client propio OpenAI-compatible) |
| ACP | **`agent-client-protocol 2.0`** (feature `acp-server`) |
| Grafos/AST | **petgraph**, **tree-sitter** + grammars (features `dag_mode`/`codebase-graph`) |
| Varios | serde/serde_yaml, chrono, uuid, sha2, rand, notify, clap |
| **Referencias de diseño** | **LiteLLM** (seed de precios/modelos, MIT), **OpenRouter**, **Aider tri-model**, **RouteLLM**, **async-openai** |
| Laterales (no bundled) | **RustDesk** (remote-live, AGPL), **terminal-kit** (sister IDE), **artemis** (mobile), **terminal-browser**, **Harbor** (eval) |

Rechazados/deferidos: `ratatui` (monitor/sister → std-only MVP), `sysinfo`+`nvml-wrapper`, `axum-oidc-layer`+`openidconnect`, `hurl`, `graphify` runtime (Python), `laya` real (candle stack).

---

## Capa 5 — Teoría y papers

- **Tree of Thoughts** (`princeton-nlp/tree-of-thought-llm`), Self-Refine, Reflexion, RePrompt, ODUTQA-MDC, LLM-Modulo, LLM-as-judge, Lost in the Middle, STORM, Masterman — pipeline de Prompt Understanding (`23`).
- **Letta / MemGPT** — memoria por tiers (core/archival/recall) (`09`, `16`).
- **Jev / "System One"** — modelo pequeño/barato para decisiones rápidas (routing/eval/compaction); `laya` como backend real (bloqueado por audit).
- **MoA (Mixture-of-Agents)** + majority vote — aggregación (`orchestrator/aggregation`).

---

## Arquitectura base

**10 motores** (`03`) + subsistemas transversales:

```
Context · Research · Planning · Reasoning · Coding · Validation · Repair · Learning · Model Orchestrator · Execution Supervisor
```

Transversales: **Swarm Coordinator**, **Skill Graph**, **MCP/CLI/Vector KB**, **Security Layer**, **UI/HUD**.

Contrato de motor: `{ id, status, inputs, outputs, policy, journal, api }`; eventos `engine.start/progress/need_input/blocked/success/failed/handoff`.

Flujo: `Mission → Planning → Research → Reasoning → Orchestrator → Swarm → Coding → Validation → (Repair ↺ | Learning) → Journal`.

Pilares de infraestructura: **Kernel Bus** (event bus), **Journal** (SQLite, 58 tablas, at-least-once idempotente), **HUD** (axum WS), **CLI `atlas`**, **Profiles** (multi-perfil).

> Nota honesta (de la auditoría `61`): el "Reasoning Engine" no existe como módulo propio; vive disperso en `orchestrator/{aggregation,council}` y `prompt/steps/self_refine.rs`. Los demás motores sí tienen módulo (algunos feature-gated).

---

## Estado de cada capa

| Capa | Estado |
|---|---|
| 1 Referentes | Comparación explícita en RFCs; paridad declarada en gobernanza/routing, brecha en capacidad agéntica |
| 2 Fuentes portadas | §A–§H ✅ implementadas (con atribución); §I y Conductor = patrones |
| 3 Ecosistema | Features mayormente adoptadas en fases 3/4/5/7/9/10/11/12/14 |
| 4 Stack | Núcleo ✅; varias features clave (embeddings/grafos/ACP/calendar) **apagadas por defecto** |
| 5 Teoría | Prompt pipeline ✅; "System One" (Laya real) bloqueado por audit |

---

## Hacia dónde va (futuro)

Del roadmap (`20`) y planes (`research/40` v2, `research/50` v3) + brechas:
1. **Cerrar capacidad agéntica** (Terminal-Bench 0.000 → >0.1 → >0.5) — prioridad #1 de la auditoría `61`.
2. **HUD Mission Control real** (8 views de RFC 24; hoy panel de debug).
3. **Laya real** (`aovestdipaperino/laya-rust`) — 4º backend "System One", bloqueado hasta que mantenedores/estabilidad cumplan.
4. **Cloud persistence + multi-canal** (brechas A/B de RFC 29 — "AI employee" estilo Genspark/Hermes).
5. **UI generativa** (patrón `vercel-labs/json-render`), **context sieve** y **compaction con modelo barato** (brechas de RFC 30).
6. **Terminal-browser §I** y **artemis mobile** ya documentados como integraciones laterales.
7. **Plugin architecture** (patrón `deepseek-ai/deepseek-harness`) para fases 5+.
8. **Paridad con plataformas agénticas 2026** (Qoder Experts/RepoWiki, Blackbox Chairman LLM/`/multi-agent`, Abacus agent swarms + AutoBots, Manus Planner/Execution/Verification, Vida multi-canal): valida la dirección de swarm/aggregation/APC, y fija el listón a superar en la brecha de capacidad agéntica (`61` §3).
9. **Compatibilidad ACP/OpenClaw** como superficie de ecosistema (GLM `glm-acp-agent`, Vida "OpenClaw compatible", MiniMax/NVIDIA NemoClaw): Atlas ya habla ACP (§B); considerar exponer un contrato compatible con el ecosistema OpenClaw sin bundling.

---

## Apéndice — URLs y licencias (fuentes portadas)

- `karpathy/autoresearch` (MIT), `microsoft/intelligent-terminal` (MIT), `safishamsi/graphify` (Apache-2.0), `darrenburns/posting` (Apache-2.0), `mendableai/firecrawl` (MIT), `atifchy/winrt-toast` (MIT), `hummingly/ics` (MIT OR Apache-2.0), `sreeise/graph-rs-sdk` (MIT), `cline/cline` (Apache-2.0), `zenbu-labs/terminal-browser` (MIT), `google/artemis` (Apache-2.0), `maxiusofmaximus/MaxAppsHub` (—), `lizardbyte/sunshine` (GPL — lateral), RustDesk (AGPL — lateral), `asg017/sqlite-vec` (MIT/Apache-2.0), `harbor-framework/harbor` (—).
- Atribución obligatoria por módulo: ver `28 - External Tool Integration.md` §"Atribución obligatoria por módulo".
