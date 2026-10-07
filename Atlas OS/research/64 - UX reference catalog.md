# 64 — UX reference catalog (atlas-os)

**Autor:** Researcher · **Fecha:** 2026-10-06 · **Tarea (Project Lead):** catálogo UX completo de `Atlas OS/research/62` (capas 1, 1b, 1c, 1d, 1e) + deep-dive de las prioridades A.
**Alcance:** solo documentación. No toca `src/`, `src-tauri/`, RFC 62, RFC 66 ni `docs/design`.

> **Procedencia (regla PLAN-docs-hardening §22):** **[O]** observado en el repo esta sesión · **[Os]** observado en vivo en la fuente (URL citada) esta sesión · **[I]** inferido/conocimiento previo (no inspeccionado en vivo) · **[P]** pendiente de verificar.

## 0. Método y recuento EXACTO

- **Fuente única:** `Atlas OS/research/62 - Project genealogy & architecture base.md` (leído completo; sin modificar). [O]
- **Regla de fila:** cada fila de tabla de RFC 62 = 1 entrada; cada ítem separado por comas en las listas inline de la Capa 1d (P–X) = 1 entrada. Se excluyen 7 filas divisorias "Punto de apoyo" (1e), las cabeceras de tabla y las capas 2/3/4/5 (fuera de alcance).
- **Extracción:** parser Node sobre RFC 62 (ver §8 comandos). Las URLs solo se citan si están en RFC 62 **[O]** o si se verificaron en vivo **[Os]**; si no, **[P]**. No se inventan URLs.

### 0.1 Recuento exacto por capa

| Capa | Entradas |
|---|---:|
| 1 | 16 |
| 1b | 9 |
| 1b-alwayson | 3 |
| 1c:A | 14 |
| 1c:B | 12 |
| 1c:C | 9 |
| 1c:D | 10 |
| 1c:E | 13 |
| 1c:F | 8 |
| 1c:G | 7 |
| 1c:H | 9 |
| 1c:I | 8 |
| 1c:J | 9 |
| 1c:K | 16 |
| 1c:L | 8 |
| 1c:M | 11 |
| 1c:N | 7 |
| 1c:O | 6 |
| 1d:directos | 6 |
| 1d:P | 27 |
| 1d:Q | 13 |
| 1d:R | 11 |
| 1d:S | 13 |
| 1d:T | 11 |
| 1d:U | 10 |
| 1d:V | 17 |
| 1d:W | 18 |
| 1d:X | 8 |
| 1e | 132 |
| **TOTAL** | **441** |

**Nota de honestidad:** RFC 62 declara "~425 proyectos"; el conteo exacto por fila/ítem es **441** [O]. La diferencia es de granularidad (comas vs fila) y de las 7 divisorias excluidas. **No coincide con ningún "320+".**

### 0.2 Distribución

| tiene_UI | # | | Prioridad | # | | Categoría UX | # |
|---|---:|---|---|---:|---|---|---:|

| si | 376 | | A | 93 | | otro | 251 |
| no | 38 | | B | 117 | | chat/workspace | 84 |
| parcial | 27 | | C | 231 | | IDE | 29 |
|  |  | |  |  | | terminal | 21 |
|  |  | |  |  | | ADE/orquestador | 20 |
|  |  | |  |  | | observabilidad | 18 |
|  |  | |  |  | | chat | 12 |
|  |  | |  |  | | canvas | 5 |
|  |  | |  |  | | kanban/PM | 1 |

## 1. Duplicados entre capas

Grupos con el mismo nombre normalizado entre secciones (33): un producto aparece en varias capas (p. ej. `Claude Code` en Capa 1 y 1c:B). No son error de RFC 62 — es solapamiento intencional capa "referente" ↔ capa "catálogo".

- **claude code**: 1 `Claude Code` · 1c:B `Claude Code (Anthropic)`
- **codex cli**: 1 `Codex CLI (OpenAI)` · 1c:B `Codex CLI (OpenAI)`
- **cursor cursor**: 1 `Cursor / Cursor Cloud Agents` · 1c:B `Cursor + Cursor CLI + Bugbot`
- **windsurf**: 1 `Windsurf` · 1c:B `Windsurf (Cognition)`
- **aider**: 1 `Aider` · 1c:A `Aider`
- **github copilot**: 1 `GitHub Copilot Project HydraFusion` · 1e `GitHub Copilot quickstart`
- **mecagent**: 1b `MecAgent (mecagent.com)` · 1c:M `MecAgent` · 1e `MecAgent`
- **coro code**: 1c:A `Coro Code / Kode CLI / QQCode / Ferrum / zot / g3 / Zap / Coro` · 1c:A `Coro Code`
- **amp**: 1c:B `Amp (Sourcegraph)` · 1e `Amp`
- **warp**: 1c:B `Warp` · 1e `Warp`
- **deepseek**: 1c:C `DeepSeek (Coder/V3.2/V4)` · 1c:J `DeepSeek (Coder)`
- **claude flow**: 1c:D `claude-flow (ruvnet)` · 1c:D `claude-flow / gastown / OMK / kodo / wreckit / OpenCastle / 5dive`
- **mistral**: 1c:J `Mistral (Devstral/Codestral/Magistral/Le Chat)` · 1e `Mistral`
- **nvidia nim**: 1c:J `Nvidia NIM` · 1e `NVIDIA NIM`
- **hugging face**: 1c:K `Hugging Face` · 1e `Hugging Face`
- **nabla**: 1c:K `Nabla` · 1d:T `Nabla`
- **synthesia**: 1c:K `Synthesia` · 1d:P `Synthesia`
- **harvey**: 1c:L `Harvey` · 1d:U `Harvey`
- **rogo**: 1c:L `Rogo (Felix)` · 1d:U `Rogo`
- **sierra**: 1c:L `Sierra (Horizon)` · 1d:U `Sierra`
- **hebbia**: 1c:L `Hebbia` · 1d:U `Hebbia`
- **legora**: 1c:L `Legora` · 1d:U `Legora`
- **lovable**: 1e `Lovable` · 1d:R `Lovable`
- **v0**: 1e `v0 (Vercel)` · 1d:R `v0 (Vercel)`
- **hunyuan3d**: 1e `Hunyuan3D` · 1d:Q `Hunyuan3D`
- **midjourney**: 1e `Midjourney` · 1d:P `Midjourney`
- **freepik ai**: 1e `Freepik AI` · 1d:P `Freepik AI`
- **suno**: 1e `Suno` · 1d:X `Suno`
- **pika**: 1e `Pika` · 1d:P `Pika`
- **poly pizza**: 1e `Poly Pizza` · 1d:W `Poly Pizza`
- **poly haven**: 1e `Poly Haven` · 1d:W `Poly Haven`
- **sketchfab**: 1e `Sketchfab` · 1d:W `Sketchfab`
- **meshy**: 1e `Meshy` · 1d:Q `Meshy`

## 2. Catálogo completo (una fila por entrada)


### Capa 1 — Referentes objetivo — 16 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 1 | opencode (opencode.ai, el runtime del que hereda el nombre del binario) | 1 | — [P] | si | ADE/orquestador |  | A |
| 2 | Claude Code | 1 | — [P] | si | terminal |  | A |
| 3 | Codex CLI (OpenAI) | 1 | — [P] | si | terminal |  | A |
| 4 | Cursor / Cursor Cloud Agents | 1 | — [P] | si | IDE |  | A |
| 5 | Windsurf | 1 | — [P] | si | IDE |  | A |
| 6 | Cline / Roo | 1 | — [P] | si | IDE |  | A |
| 7 | Aider | 1 | — [P] | si | terminal |  | A |
| 8 | Continue.dev | 1 | — [P] | si | IDE |  | A |
| 9 | Hermes (NousResearch nousresearch/hermes-agent) | 1 | — [P] | si | ADE/orquestador |  | A |
| 10 | OpenClaw | 1 | — [P] | si | ADE/orquestador |  | A |
| 11 | AionUI (iofficeai/aionui) | 1 | — [P] | si | chat/workspace |  | A |
| 12 | Genspark | 1 | — [P] | si | chat/workspace |  | A |
| 13 | Antigravity / Gemini CLI (Google) | 1 | — [P] | si | terminal |  | A |
| 14 | GitHub Copilot Project HydraFusion | 1 | — [P] | si | IDE |  | A |
| 15 | OpenRouter Fusion / Sakana Fugu | 1 | — [P] | si | observabilidad |  | A |
| 16 | Kimi (K2.7 / k3) | 1 | — [P] | si | chat/workspace |  | A |

### Capa 1b — Añadidos 2026 — 9 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 17 | MiniMax (M2/M2.5/M2.7/M3; MiniMax Agent / MiniMax Code) | 1b | — [P] | si | chat/workspace |  | A |
| 18 | GLM / Zhipu AI (Z.ai) | 1b | — [P] | si | otro |  | B |
| 19 | Blackbox AI | 1b | — [P] | si | IDE |  | A |
| 20 | Qoder (Alibaba / Bright Zenith) | 1b | — [P] | si | IDE |  | A |
| 21 | Manus (Monica) | 1b | — [P] | si | chat/workspace |  | A |
| 22 | Abacus.AI (ChatLLM / AI Agent) | 1b | — [P] | si | chat/workspace |  | A |
| 23 | Vida (vida.io) | 1b | — [P] | si | ADE/orquestador |  | B |
| 24 | MecAgent (mecagent.com) | 1b | — [P] | si | otro |  | B |
| 25 | Parakeet AI (parakeet-ai.com) | 1b | — [P] | si | otro |  | B |

### Capa 1b — Always-on cloud-computer — 3 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 26 | Dots (OpenAI, Sep 29 2026, DevDay) | 1b-alwayson | — [P] | si | chat/workspace |  | A |
| 27 | Grok Bot (xAI / SpaceXAI, beta Ago 11 2026) | 1b-alwayson | — [P] | si | chat/workspace |  | A |
| 28 | Meta Muse · Google Gemini Spark · Instinct | 1b-alwayson | — [P] | si | chat/workspace |  | A |

### A. Agentes de terminal open-source — 14 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 29 | OpenHands (All Hands, ex-OpenDevin) | 1c:A | — [P] | si | ADE/orquestador |  | A |
| 30 | SWE-agent (Princeton) | 1c:A | — [P] | si | terminal |  | B |
| 31 | Goose (Block) | 1c:A | — [P] | si | terminal |  | A |
| 32 | Aider | 1c:A | — [P] | si | terminal |  | A |
| 33 | Plandex | 1c:A | — [P] | si | terminal |  | A |
| 34 | gptme | 1c:A | — [P] | si | terminal |  | A |
| 35 | Devon (entropy-research) | 1c:A | — [P] | si | terminal |  | A |
| 36 | AutoCodeRover | 1c:A | — [P] | si | terminal |  | B |
| 37 | Octomind | 1c:A | — [P] | si | terminal |  | B |
| 38 | Coro Code / Kode CLI / QQCode / Ferrum / zot / g3 / Zap / Coro | 1c:A | — [P] | si | terminal |  | A |
| 39 | jcode | 1c:A | — [P] | si | terminal |  | A |
| 40 | Coro Code | 1c:A | — [P] | si | terminal |  | A |
| 41 | OpenInterpreter / GPT-Engineer / MetaGPT / Smol Developer / Devika / AutoGPT / BabyAGI / SuperAGI | 1c:A | — [P] | si | terminal |  | B |
| 42 | Pi / Pi Agent IDE | 1c:A | — [P] | si | IDE |  | A |

### B. Agentes de plataforma y CLIs — 12 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 43 | Claude Code (Anthropic) | 1c:B | — [P] | si | terminal |  | A |
| 44 | Codex CLI (OpenAI) | 1c:B | — [P] | si | terminal |  | A |
| 45 | Gemini CLI (Google) | 1c:B | — [P] | si | ADE/orquestador |  | A |
| 46 | Cursor + Cursor CLI + Bugbot | 1c:B | — [P] | si | IDE |  | A |
| 47 | Windsurf (Cognition) | 1c:B | — [P] | si | IDE |  | A |
| 48 | Devin (Cognition) | 1c:B | — [P] | si | ADE/orquestador |  | A |
| 49 | Amp (Sourcegraph) | 1c:B | — [P] | si | ADE/orquestador |  | A |
| 50 | Junie CLI (JetBrains) | 1c:B | — [P] | si | IDE |  | B |
| 51 | Cortex Code (Snowflake) / Tabnine CLI / Mentat CLI / Amazon Q Developer / GitHub Copilot CLI | 1c:B | — [P] | si | ADE/orquestador |  | A |
| 52 | Warp | 1c:B | — [P] | si | terminal |  | A |
| 53 | Trae (ByteDance) | 1c:B | — [P] | si | IDE |  | B |
| 54 | FetchCoder (Fetch.ai) | 1c:B | — [P] | si | ADE/orquestador |  | A |

### C. Ecosistema de coding chino — 9 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 55 | Qwen Code / Qwen-Coder (Alibaba) | 1c:C | — [P] | parcial | IDE |  | B |
| 56 | Qoder + Tongyi Lingma (Alibaba) | 1c:C | — [P] | si | IDE |  | A |
| 57 | CodeGeeX (Zhipu) | 1c:C | — [P] | parcial | IDE |  | B |
| 58 | CodeBuddy (Tencent) | 1c:C | — [P] | parcial | IDE |  | B |
| 59 | Baidu Comate / Zulu | 1c:C | — [P] | parcial | IDE |  | B |
| 60 | Kimi Code + K2.7/K3 (Moonshot) | 1c:C | — [P] | si | chat/workspace |  | A |
| 61 | Huawei CodeArts / iFlytek Spark | 1c:C | — [P] | parcial | IDE |  | B |
| 62 | DeepSeek (Coder/V3.2/V4) | 1c:C | — [P] | parcial | IDE |  | B |
| 63 | GLM / MiniMax | 1c:C | — [P] | si | otro |  | B |

### D. Harnesses y orquestadores — 10 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 64 | claude-flow (ruvnet) | 1c:D | — [P] | si | ADE/orquestador |  | A |
| 65 | DeerFlow (ByteDance) | 1c:D | — [P] | si | ADE/orquestador |  | A |
| 66 | Symphony (OpenAI) | 1c:D | — [P] | si | ADE/orquestador |  | A |
| 67 | Omnigent (Databricks) | 1c:D | — [P] | si | ADE/orquestador |  | A |
| 68 | claude-flow / gastown / OMK / kodo / wreckit / OpenCastle / 5dive | 1c:D | — [P] | si | ADE/orquestador |  | A |
| 69 | CliDeck / GridBash / tlbx / ADHDev / cmux | 1c:D | — [P] | si | ADE/orquestador |  | A |
| 70 | Conductor | 1c:D | — [P] | si | ADE/orquestador |  | A |
| 71 | tmux / Zellij / Overmind | 1c:D | — [P] | si | terminal |  | A |
| 72 | Vibe Kanban (BloopAI) | 1c:D | — [P] | si | kanban/PM |  | A |
| 73 | AgentSwarms / Overbrilliant OB-1 / darce / Forge / CLAII / Nausicaa / Jazz / Smelt | 1c:D | — [P] | si | ADE/orquestador |  | A |

### E. Frameworks y SDKs — 13 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 74 | LangGraph / LangChain | 1c:E | — [P] | no | otro | framework/SDK (librería, sin UI propia) | C |
| 75 | CrewAI | 1c:E | — [P] | no | otro | framework/SDK de agentes (librería, sin UI de usuario) | C |
| 76 | AutoGen / AG2 (Microsoft) | 1c:E | — [P] | no | otro | framework/SDK de agentes (librería, sin UI de usuario) | C |
| 77 | Microsoft Agent Framework | 1c:E | — [P] | no | otro | framework/SDK de agentes (librería, sin UI de usuario) | C |
| 78 | Semantic Kernel | 1c:E | — [P] | no | otro | framework/SDK de agentes (librería, sin UI de usuario) | C |
| 79 | LlamaIndex Workflows | 1c:E | — [P] | no | otro | framework/SDK de agentes (librería, sin UI de usuario) | C |
| 80 | Google ADK | 1c:E | — [P] | no | otro | framework/SDK de agentes (librería, sin UI de usuario) | C |
| 81 | OpenAI Agents SDK | 1c:E | — [P] | no | otro | framework/SDK de agentes (librería, sin UI de usuario) | C |
| 82 | Mastra | 1c:E | — [P] | no | otro | framework/SDK de agentes (librería, sin UI de usuario) | C |
| 83 | Pydantic AI | 1c:E | — [P] | no | otro | framework/SDK de agentes (librería, sin UI de usuario) | C |
| 84 | AWS Strands Agents | 1c:E | — [P] | no | otro | framework/SDK de agentes (librería, sin UI de usuario) | C |
| 85 | Dify / Flowise / n8n | 1c:E | — [P] | si | canvas |  | A |
| 86 | Temporal | 1c:E | — [P] | no | otro | infra de orquestación durable, sin UI de agentes propia | B |

### F. Infraestructura de agentes — 8 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 87 | Mem0 | 1c:F | — [P] | no | otro | infraestructura/servicio (memoria, sandbox, vector) sin UI de orquestación propia | C |
| 88 | Letta (MemGPT) | 1c:F | — [P] | no | otro | infraestructura/servicio (memoria, sandbox, vector) sin UI de orquestación propia | C |
| 89 | Zep | 1c:F | — [P] | no | otro | infraestructura/servicio (memoria, sandbox, vector) sin UI de orquestación propia | C |
| 90 | Qdrant / Weaviate / Chroma / pgvector | 1c:F | — [P] | no | otro | infraestructura/servicio (memoria, sandbox, vector) sin UI de orquestación propia | C |
| 91 | Daytona | 1c:F | — [P] | no | otro | infraestructura/servicio (memoria, sandbox, vector) sin UI de orquestación propia | C |
| 92 | E2B / Modal | 1c:F | — [P] | no | otro | infraestructura/servicio (memoria, sandbox, vector) sin UI de orquestación propia | C |
| 93 | Firecracker / gVisor | 1c:F | — [P] | no | otro | infraestructura/servicio (memoria, sandbox, vector) sin UI de orquestación propia | C |
| 94 | mksglu/context-mode | 1c:F | — [P] | no | otro | infraestructura/servicio (memoria, sandbox, vector) sin UI de orquestación propia | C |

### G. Cloud dev environments — 7 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 95 | Coder | 1c:G | — [P] | si | IDE |  | A |
| 96 | Gitpod / Ona | 1c:G | — [P] | si | IDE |  | A |
| 97 | GitHub Codespaces | 1c:G | — [P] | si | IDE |  | A |
| 98 | DevPod | 1c:G | — [P] | si | IDE |  | A |
| 99 | code-server / openvscode-server | 1c:G | — [P] | si | IDE |  | A |
| 100 | Devcontainers spec | 1c:G | — [P] | no | otro | especificación (sin UI) | C |
| 101 | Bunnyshell | 1c:G | — [P] | si | IDE |  | A |

### H. Observabilidad y gateways — 9 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 102 | Langfuse (DE, MIT) | 1c:H | — [P] | si | observabilidad |  | A |
| 103 | LangSmith | 1c:H | — [P] | si | observabilidad |  | A |
| 104 | Arize Phoenix | 1c:H | — [P] | si | observabilidad |  | A |
| 105 | Braintrust | 1c:H | — [P] | si | observabilidad |  | A |
| 106 | MLflow | 1c:H | — [P] | si | observabilidad |  | A |
| 107 | Helicone / Portkey / LiteLLM / OpenRouter / ZenMux | 1c:H | — [P] | si | observabilidad |  | A |
| 108 | TruLens / Comet Opik / Lunary / W&B Weave | 1c:H | — [P] | si | observabilidad |  | A |
| 109 | OpenTelemetry GenAI | 1c:H | — [P] | no | otro | estándar de telemetría (sin UI) | C |
| 110 | AgentOps | 1c:H | — [P] | si | observabilidad |  | A |

### I. Code review y calidad — 8 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 111 | Qodo (ex-CodiumAI) | 1c:I | — [P] | si | observabilidad |  | B |
| 112 | CodeRabbit | 1c:I | — [P] | si | observabilidad |  | B |
| 113 | Greptile | 1c:I | — [P] | si | observabilidad |  | B |
| 114 | Graphite | 1c:I | — [P] | si | observabilidad |  | B |
| 115 | Cursor Bugbot / GitHub Copilot Code Review | 1c:I | — [P] | si | observabilidad |  | B |
| 116 | Snyk / SonarQube / Codacy / DeepSource | 1c:I | — [P] | si | observabilidad |  | B |
| 117 | Sourcegraph | 1c:I | — [P] | si | observabilidad |  | B |
| 118 | Qodex (≠ Qodo) | 1c:I | — [P] | si | observabilidad |  | B |

### J. Model providers — 9 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 119 | Mistral (Devstral/Codestral/Magistral/Le Chat) | 1c:J | — [P] | parcial | chat/workspace | playground/Le Chat + API | B |
| 120 | DeepSeek (Coder) | 1c:J | — [P] | parcial | otro |  | C |
| 121 | Qwen (Coder) | 1c:J | — [P] | parcial | otro |  | C |
| 122 | Meta Llama / Google Gemma / Ai2 OLMo | 1c:J | — [P] | parcial | otro |  | C |
| 123 | Cohere (Command/North) | 1c:J | — [P] | parcial | otro |  | C |
| 124 | Moonshot Kimi / Zhipu GLM / MiniMax / ByteDance Seed (Doubao) | 1c:J | — [P] | si | chat/workspace |  | A |
| 125 | Poolside | 1c:J | — [P] | parcial | otro |  | C |
| 126 | Cursor Composer / SWE-1 (Cognition) | 1c:J | — [P] | si | IDE |  | A |
| 127 | Nvidia NIM | 1c:J | — [P] | parcial | otro |  | C |

### K. Ecosistema europeo — 16 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 128 | Mistral AI | 1c:K | — [P] | parcial | chat/workspace | playground/Le Chat + API | B |
| 129 | Aleph Alpha (+ PhariaAI) | 1c:K | — [P] | parcial | otro |  | B |
| 130 | DeepL | 1c:K | — [P] | parcial | otro |  | B |
| 131 | Hugging Face | 1c:K | — [P] | si | otro | hub con UI de modelos/datasets/spaces | B |
| 132 | Dust | 1c:K | — [P] | si | chat/workspace |  | A |
| 133 | LightOn | 1c:K | — [P] | parcial | otro |  | B |
| 134 | Nabla | 1c:K | — [P] | parcial | otro |  | B |
| 135 | H Company | 1c:K | — [P] | parcial | otro |  | B |
| 136 | Kyutai | 1c:K | — [P] | parcial | otro |  | B |
| 137 | Photoroom | 1c:K | — [P] | parcial | otro |  | B |
| 138 | Stability AI | 1c:K | — [P] | parcial | otro |  | B |
| 139 | Synthesia | 1c:K | — [P] | parcial | otro |  | B |
| 140 | Silo AI (AMD) | 1c:K | — [P] | parcial | otro |  | B |
| 141 | Helsing / Wayve | 1c:K | — [P] | parcial | otro |  | B |
| 142 | Qdrant / Langfuse / n8n | 1c:K | — [P] | si | observabilidad |  | A |
| 143 | Weaviate | 1c:K | — [P] | parcial | otro |  | B |

### L. Agentes verticales — 8 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 144 | Harvey | 1c:L | — [P] | si | chat/workspace |  | B |
| 145 | Rogo (Felix) | 1c:L | — [P] | si | chat/workspace |  | B |
| 146 | Sierra (Horizon) | 1c:L | — [P] | si | chat/workspace |  | B |
| 147 | Hebbia | 1c:L | — [P] | si | chat/workspace |  | B |
| 148 | Abridge / OpenEvidence / Ambience / Hippocratic AI | 1c:L | — [P] | si | chat/workspace |  | B |
| 149 | Decagon / Parloa | 1c:L | — [P] | si | chat/workspace |  | B |
| 150 | Legora | 1c:L | — [P] | si | chat/workspace |  | B |
| 151 | Cognition Devin (vertical SWE) | 1c:L | — [P] | si | ADE/orquestador |  | A |

### M. CAD / 3D / ingeniería — 11 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 152 | OpenCADStudio (HakanSeven12) | 1c:M | — [P] | si | otro |  | A |
| 153 | FreeCAD | 1c:M | — [P] | si | otro |  | B |
| 154 | OpenSCAD | 1c:M | — [P] | si | otro |  | B |
| 155 | CadQuery | 1c:M | — [P] | si | otro |  | B |
| 156 | Zoo (ex-KittyCAD, KCL) | 1c:M | — [P] | si | otro |  | B |
| 157 | Backflip AI | 1c:M | — [P] | si | otro |  | B |
| 158 | Adam CAD (CADAM) | 1c:M | — [P] | si | otro |  | B |
| 159 | MecAgent | 1c:M | — [P] | si | otro |  | B |
| 160 | Onshape / Shapr3D / Spline | 1c:M | — [P] | si | otro |  | B |
| 161 | Physna | 1c:M | — [P] | si | otro |  | B |
| 162 | Text-to-CadQuery (arxiv 2505.06507) | 1c:M | — [P] | si | otro |  | B |

### N. Protocolos de interoperabilidad — 7 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 163 | MCP (Anthropic) | 1c:N | — [P] | no | otro | protocolo/especificación de interoperabilidad (sin UI) | C |
| 164 | A2A (Google, Linux Foundation) | 1c:N | — [P] | no | otro | protocolo/especificación de interoperabilidad (sin UI) | C |
| 165 | ACP (Zed/Block) | 1c:N | — [P] | no | otro | protocolo/especificación de interoperabilidad (sin UI) | C |
| 166 | ACP→A2A (IBM BeeAI) | 1c:N | — [P] | no | otro | protocolo/especificación de interoperabilidad (sin UI) | C |
| 167 | AG-UI (CopilotKit) | 1c:N | — [P] | no | otro | protocolo/especificación de interoperabilidad (sin UI) | C |
| 168 | ANP / Agora / LMOS | 1c:N | — [P] | no | otro | protocolo/especificación de interoperabilidad (sin UI) | C |
| 169 | A2UI / MCP-UI / Open JSON UI | 1c:N | — [P] | no | otro | protocolo/especificación de interoperabilidad (sin UI) | C |

### O. Benchmarks — 6 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 170 | SWE-bench / SWE-bench Verified / Multi-SWE-bench | 1c:O | — [P] | no | otro | benchmark/harness de evaluación (CLI, sin UI de usuario) | C |
| 171 | Terminal-Bench 2 | 1c:O | — [P] | no | otro | benchmark/harness de evaluación (CLI, sin UI de usuario) | C |
| 172 | GAIA / AgentCompany | 1c:O | — [P] | no | otro | benchmark/harness de evaluación (CLI, sin UI de usuario) | C |
| 173 | WebArena / OSWorld / BrowseComp | 1c:O | — [P] | no | otro | benchmark/harness de evaluación (CLI, sin UI de usuario) | C |
| 174 | SWE-Lancer | 1c:O | — [P] | no | otro | benchmark/harness de evaluación (CLI, sin UI de usuario) | C |
| 175 | Harbor | 1c:O | — [P] | no | otro | benchmark/harness de evaluación (CLI, sin UI de usuario) | C |

### Capa 1d — Seis añadidos directos — 6 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 176 | Higgsfield (higgsfield.ai) | 1d:directos | — [P] | si | otro |  | B |
| 177 | mr-mak-workspace (witnesstodark, MIT, 295★) | 1d:directos | — [P] | si | chat/workspace |  | A |
| 178 | Flova AI (flova.ai) | 1d:directos | — [P] | si | chat/workspace |  | A |
| 179 | Tesana (tesana.ai) | 1d:directos | — [P] | si | otro |  | B |
| 180 | Cave = Cave Engine (Uniday Studio — Guilherme Teres Nunes) | 1d:directos | — [P] | si | otro |  | B |
| 181 | Stove = Stove 3D (mismo dev de Cave Engine, Uniday Studio) | 1d:directos | — [P] | si | otro |  | B |

### P. Generación de video/imagen/audio — 27 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 182 | Sora (OpenAI) | 1d:P | — [P] | si | chat/workspace |  | C |
| 183 | Veo (Google) | 1d:P | — [P] | si | chat/workspace |  | C |
| 184 | Kling (Kuaishou) | 1d:P | — [P] | si | chat/workspace |  | C |
| 185 | Runway (Gen-4) | 1d:P | — [P] | si | chat/workspace |  | C |
| 186 | Pika | 1d:P | — [P] | si | chat/workspace |  | C |
| 187 | Luma AI (Dream Machine/Ray) | 1d:P | — [P] | si | chat/workspace |  | C |
| 188 | Hailuo (MiniMax) | 1d:P | — [P] | si | chat/workspace |  | C |
| 189 | Wan (Alibaba) | 1d:P | — [P] | si | chat/workspace |  | C |
| 190 | Seedance/Seedream (ByteDance) | 1d:P | — [P] | si | chat/workspace |  | C |
| 191 | Vidu (Shengshu) | 1d:P | — [P] | si | chat/workspace |  | C |
| 192 | LTX Studio (Lightricks) | 1d:P | — [P] | si | chat/workspace |  | C |
| 193 | HeyGen | 1d:P | — [P] | si | chat/workspace |  | C |
| 194 | Synthesia | 1d:P | — [P] | si | chat/workspace |  | C |
| 195 | Colossyan | 1d:P | — [P] | si | chat/workspace |  | C |
| 196 | D-ID | 1d:P | — [P] | si | chat/workspace |  | C |
| 197 | Hedra | 1d:P | — [P] | si | chat/workspace |  | C |
| 198 | Captions | 1d:P | — [P] | si | chat/workspace |  | C |
| 199 | InVideo AI | 1d:P | — [P] | si | chat/workspace |  | C |
| 200 | Descript | 1d:P | — [P] | si | chat/workspace |  | C |
| 201 | Opus Clip | 1d:P | — [P] | si | chat/workspace |  | C |
| 202 | Midjourney | 1d:P | — [P] | si | chat/workspace |  | C |
| 203 | Black Forest Labs FLUX | 1d:P | — [P] | si | chat/workspace |  | C |
| 204 | Ideogram | 1d:P | — [P] | si | chat/workspace |  | C |
| 205 | Recraft | 1d:P | — [P] | si | chat/workspace |  | C |
| 206 | Leonardo AI | 1d:P | — [P] | si | chat/workspace |  | C |
| 207 | Adobe Firefly | 1d:P | — [P] | si | chat/workspace |  | C |
| 208 | Freepik AI | 1d:P | — [P] | si | chat/workspace |  | C |

### Q. Generación 3D y game builders — 13 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 209 | Rosebud AI | 1d:Q | — [P] | si | otro |  | C |
| 210 | Roblox Cube | 1d:Q | — [P] | si | otro |  | C |
| 211 | Unity Muse/Sentis | 1d:Q | — [P] | si | otro |  | C |
| 212 | Unreal + AI | 1d:Q | — [P] | si | otro |  | C |
| 213 | Scenario | 1d:Q | — [P] | si | otro |  | C |
| 214 | Layer.ai | 1d:Q | https://Layer.ai [O] | si | otro |  | C |
| 215 | Meshy | 1d:Q | — [P] | si | otro |  | C |
| 216 | Tripo AI | 1d:Q | — [P] | si | otro |  | C |
| 217 | Hyper3D Rodin | 1d:Q | — [P] | si | otro |  | C |
| 218 | Hunyuan3D | 1d:Q | — [P] | si | otro |  | C |
| 219 | Luma Genie | 1d:Q | — [P] | si | otro |  | C |
| 220 | Spline AI | 1d:Q | — [P] | si | otro |  | C |
| 221 | Sloyd | 1d:Q | — [P] | si | otro |  | C |

### R. Workspaces y app-builders con agente — 11 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 222 | Bolt.new / bolt.diy | 1d:R | — [P] | si | chat/workspace |  | A |
| 223 | v0 (Vercel) | 1d:R | — [P] | si | chat/workspace |  | A |
| 224 | Lovable | 1d:R | — [P] | si | chat/workspace |  | A |
| 225 | Replit Agent | 1d:R | — [P] | si | chat/workspace |  | A |
| 226 | Dyad (local) | 1d:R | — [P] | si | chat/workspace |  | A |
| 227 | Onlook (visual editor) | 1d:R | — [P] | si | chat/workspace |  | A |
| 228 | Same.dev | 1d:R | https://Same.dev [O] | si | chat/workspace |  | A |
| 229 | Pythagora/GPT-Pilot | 1d:R | — [P] | si | chat/workspace |  | A |
| 230 | Create.xyz | 1d:R | https://Create.xyz [O] | si | chat/workspace |  | A |
| 231 | Firebase Studio (Google) | 1d:R | — [P] | si | chat/workspace |  | A |
| 232 | Glide/Softr | 1d:R | — [P] | si | chat/workspace |  | A |

### S. Memoria, compañeros y mindful — 13 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 233 | Cave (incave.io, homónimo — NO Cave Engine) | 1d:S | https://incave.io [O] | si | otro |  | B |
| 234 | Character.AI | 1d:S | https://Character.AI [O] | si | chat |  | C |
| 235 | Replika | 1d:S | — [P] | si | chat |  | C |
| 236 | Pi (Inflection) | 1d:S | — [P] | si | chat |  | C |
| 237 | Nomi | 1d:S | — [P] | si | chat |  | C |
| 238 | Kindroid | 1d:S | — [P] | si | chat |  | C |
| 239 | Dot | 1d:S | — [P] | si | chat |  | C |
| 240 | Personal.ai | 1d:S | https://Personal.ai [O] | si | chat |  | C |
| 241 | Limitless/Rewind | 1d:S | — [P] | si | chat |  | C |
| 242 | Mem (mem.ai) | 1d:S | https://mem.ai [O] | si | chat |  | C |
| 243 | Notion AI | 1d:S | — [P] | si | chat |  | C |
| 244 | Reflect | 1d:S | — [P] | si | chat |  | C |
| 245 | Tana | 1d:S | — [P] | si | chat |  | C |

### T. Salud y bienestar — 11 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 246 | Nabla | 1d:T | — [P] | si | chat/workspace |  | C |
| 247 | Abridge | 1d:T | — [P] | si | chat/workspace |  | C |
| 248 | OpenEvidence | 1d:T | — [P] | si | chat/workspace |  | C |
| 249 | Ambience | 1d:T | — [P] | si | chat/workspace |  | C |
| 250 | Woebot/Wysa | 1d:T | — [P] | si | chat/workspace |  | C |
| 251 | Ada Health | 1d:T | — [P] | si | chat/workspace |  | C |
| 252 | K Health | 1d:T | — [P] | si | chat/workspace |  | C |
| 253 | Summer Health | 1d:T | — [P] | si | chat/workspace |  | C |
| 254 | Function Health | 1d:T | — [P] | si | chat/workspace |  | C |
| 255 | Whoop Coach | 1d:T | — [P] | si | chat/workspace |  | C |
| 256 | Hippocratic AI | 1d:T | — [P] | si | chat/workspace |  | C |

### U. Agentes legales/financieros — 10 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 257 | Harvey | 1d:U | — [P] | si | chat/workspace |  | B |
| 258 | Rogo | 1d:U | — [P] | si | chat/workspace |  | B |
| 259 | Hebbia | 1d:U | — [P] | si | chat/workspace |  | B |
| 260 | EvenUp | 1d:U | — [P] | si | chat/workspace |  | B |
| 261 | Spellbook | 1d:U | — [P] | si | chat/workspace |  | B |
| 262 | Ironclad | 1d:U | — [P] | si | chat/workspace |  | B |
| 263 | Klarity | 1d:U | — [P] | si | chat/workspace |  | B |
| 264 | Ramp Intelligence | 1d:U | — [P] | si | chat/workspace |  | B |
| 265 | Sierra | 1d:U | — [P] | si | chat/workspace |  | B |
| 266 | Legora | 1d:U | — [P] | si | chat/workspace |  | B |

### V. Tooling local / self-hosted — 17 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 267 | Ollama | 1d:V | — [P] | si | otro |  | B |
| 268 | LM Studio | 1d:V | — [P] | si | otro |  | B |
| 269 | llama.cpp | 1d:V | — [P] | no | otro | runtime de inferencia (CLI, sin UI) | B |
| 270 | vLLM | 1d:V | — [P] | no | otro | servidor de inferencia (sin UI) | C |
| 271 | SGLang | 1d:V | — [P] | no | otro | servidor de inferencia (sin UI) | C |
| 272 | Open WebUI | 1d:V | — [P] | si | otro |  | B |
| 273 | LibreChat | 1d:V | — [P] | si | otro |  | B |
| 274 | Jan | 1d:V | — [P] | si | otro |  | B |
| 275 | GPT4All | 1d:V | — [P] | si | otro |  | B |
| 276 | AnythingLLM | 1d:V | — [P] | si | otro |  | B |
| 277 | LocalAI | 1d:V | — [P] | si | otro |  | B |
| 278 | text-generation-webui | 1d:V | — [P] | si | otro |  | B |
| 279 | SillyTavern | 1d:V | — [P] | si | otro |  | B |
| 280 | ComfyUI | 1d:V | — [P] | si | otro |  | B |
| 281 | AUTOMATIC1111 (SD WebUI) | 1d:V | — [P] | si | otro |  | B |
| 282 | Fooocus | 1d:V | — [P] | si | otro |  | B |
| 283 | InvokeAI | 1d:V | — [P] | si | otro |  | B |

### W. Engines 3D / DCC / assets — 18 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 284 | Blender (+MCP) | 1d:W | — [P] | si | otro |  | B |
| 285 | UPBGE | 1d:W | — [P] | si | otro |  | B |
| 286 | Armory3D | 1d:W | — [P] | si | otro |  | B |
| 287 | Godot | 1d:W | — [P] | si | otro |  | B |
| 288 | Bevy (Rust) | 1d:W | — [P] | si | otro |  | B |
| 289 | Lumix Engine | 1d:W | — [P] | si | otro |  | B |
| 290 | GDevelop | 1d:W | — [P] | si | otro |  | B |
| 291 | Rogue Engine (Three.js) | 1d:W | — [P] | si | otro |  | B |
| 292 | Game Pencil Engine | 1d:W | — [P] | si | canvas |  | B |
| 293 | Blockbench | 1d:W | — [P] | si | otro |  | B |
| 294 | MagicaVoxel | 1d:W | — [P] | si | otro |  | B |
| 295 | Cascadeur | 1d:W | — [P] | si | otro |  | B |
| 296 | Mixamo | 1d:W | — [P] | si | otro |  | B |
| 297 | Sketchfab | 1d:W | — [P] | si | otro |  | B |
| 298 | Poly Haven | 1d:W | — [P] | si | otro |  | B |
| 299 | Poly Pizza | 1d:W | — [P] | si | otro |  | B |
| 300 | Vibe4D | 1d:W | — [P] | si | otro |  | B |
| 301 | CSM Coder | 1d:W | — [P] | si | otro |  | B |

### X. Audio, voz y música — 8 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 302 | ElevenLabs | 1d:X | — [P] | si | otro |  | C |
| 303 | Suno | 1d:X | — [P] | si | otro |  | C |
| 304 | Udio | 1d:X | — [P] | si | otro |  | C |
| 305 | Stable Audio | 1d:X | — [P] | si | otro |  | C |
| 306 | Deepgram | 1d:X | — [P] | si | otro |  | C |
| 307 | fal.ai | 1d:X | https://fal.ai [O] | si | otro |  | C |
| 308 | Replicate | 1d:X | — [P] | si | otro |  | C |
| 309 | Together AI | 1d:X | — [P] | si | otro |  | C |

### Capa 1e — Puntos de apoyo (favoritos) — 132 entradas

| # | Entrada | Capa | URL oficial | tiene_UI | Categoría UX | Motivo (si no UI) | Prioridad |
|---|---|---|---|---|---|---|---|
| 310 | Warp | 1e | https://www.warp.dev/ [O] | si | terminal |  | A |
| 311 | Kiro (AWS) | 1e | https://kiro.dev/ [O] | si | IDE |  | A |
| 312 | Monica | 1e | https://monica.im/ [O] | si | chat/workspace |  | C |
| 313 | Amp | 1e | https://ampcode.com/ [O] | si | otro |  | C |
| 314 | fx.sh | 1e | https://fx.sh/ [O] | si | otro |  | C |
| 315 | Rocket.new | 1e | https://www.rocket.new/ [O] | si | otro |  | C |
| 316 | Base44 | 1e | https://base44.com/ [O] | si | otro |  | C |
| 317 | Lovable | 1e | https://lovable.dev/ [O] | si | otro |  | C |
| 318 | v0 (Vercel) | 1e | https://v0.app/ [O] | si | otro |  | C |
| 319 | Replit | 1e | https://replit.com/ [O] | si | otro |  | C |
| 320 | InsForge | 1e | https://insforge.dev/ [O] | si | otro |  | C |
| 321 | Seenode | 1e | https://seenode.com/ [O] | si | otro |  | C |
| 322 | PortKiller | 1e | https://portkiller.app/ · https://github.com/productdevbook/port-killer [O] | si | otro |  | C |
| 323 | Cursor MCP Directory | 1e | https://docs.cursor.com/tools [O] | si | otro |  | C |
| 324 | GitHub Copilot quickstart | 1e | https://docs.github.com/en/copilot/quickstart [O] | si | otro |  | C |
| 325 | GitHub HydraFusion | 1e | https://github.blog/ai-and-ml/github-copilot/ [O] | si | IDE |  | A |
| 326 | Status Claude | 1e | https://status.claude.com/ [O] | si | otro |  | C |
| 327 | MecAgent | 1e | https://mecagent.com/ [O] | si | otro |  | B |
| 328 | skills.sh | 1e | https://skills.sh/ [O] | si | otro |  | B |
| 329 | Pieces MCP | 1e | https://pieces.app/features/mcp [O] | si | ADE/orquestador |  | B |
| 330 | obra/superpowers | 1e | https://github.com/obra/superpowers [O] | si | otro |  | C |
| 331 | wshobson/agents | 1e | https://github.com/wshobson/agents [O] | si | otro |  | C |
| 332 | agents.md | 1e | https://agents.md/ [O] | si | otro |  | B |
| 333 | OpenSpec | 1e | https://openspec.dev/ [O] | si | otro |  | B |
| 334 | Pencil | 1e | https://www.pencil.dev/ [O] | si | canvas |  | B |
| 335 | PicoBerry | 1e | https://teaser.picoberry.ai/ [O] | si | otro |  | C |
| 336 | MeshTailor | 1e | https://meshtailor.github.io/ [O] | si | otro |  | C |
| 337 | MobiAI.dev | 1e | https://mobiai.dev/ [O] | si | IDE |  | B |
| 338 | Cerebras | 1e | https://www.cerebras.ai/ [O] | si | otro |  | C |
| 339 | Groq | 1e | https://groq.com/ [O] | si | otro |  | C |
| 340 | OpenRouter | 1e | https://openrouter.ai/ [O] | si | otro |  | C |
| 341 | NVIDIA NIM | 1e | https://build.nvidia.com/models [O] | si | otro |  | C |
| 342 | SambaNova | 1e | https://cloud.sambanova.ai/ [O] | si | otro |  | C |
| 343 | Mistral | 1e | https://mistral.ai/ [O] | parcial | chat/workspace | playground/Le Chat + API | B |
| 344 | GooseAI | 1e | https://goose.ai/ [O] | si | otro |  | C |
| 345 | Unsloth | 1e | https://unsloth.ai/docs [O] | si | otro |  | C |
| 346 | CanIRun.ai | 1e | https://www.canirun.ai/ [O] | si | otro |  | C |
| 347 | llmfit | 1e | https://llmfit.org/ [O] | si | otro |  | C |
| 348 | llm-stats | 1e | https://llm-stats.com/ [O] | si | otro |  | C |
| 349 | Artificial Analysis | 1e | https://artificialanalysis.ai/ [O] | si | otro |  | C |
| 350 | arena.ai | 1e | https://arena.ai/ [O] | si | otro |  | C |
| 351 | Hugging Face | 1e | https://huggingface.co/ [O] | si | otro | hub con UI de modelos/datasets/spaces | B |
| 352 | Decodo | 1e | https://decodo.com/ [O] | si | otro |  | C |
| 353 | AIsa | 1e | https://aisa.one/ [O] | si | otro |  | C |
| 354 | Hunyuan3D | 1e | https://3d.hunyuan.tencent.com/ [O] | si | otro |  | C |
| 355 | Stable Diffusion | 1e | https://stablediffusionweb.com/ [O] | si | otro |  | C |
| 356 | Yeri AI | 1e | https://yeri.ai/ [O] | si | otro |  | C |
| 357 | Midjourney | 1e | https://www.midjourney.com/ [O] | si | otro |  | C |
| 358 | Freepik AI | 1e | https://www.freepik.com/ai [O] | si | otro |  | C |
| 359 | Suno | 1e | https://suno.com/ [O] | si | otro |  | C |
| 360 | Mureka | 1e | https://mureka.ai/ [O] | si | otro |  | C |
| 361 | Fish Audio | 1e | https://fish.audio/ [O] | si | otro |  | C |
| 362 | Pika | 1e | https://pika.art/ [O] | si | otro |  | C |
| 363 | Luma | 1e | https://lumalabs.ai/ [O] | si | otro |  | C |
| 364 | Topview | 1e | https://topview.ai/ [O] | si | otro |  | C |
| 365 | Superhive (Blender Market) | 1e | https://superhivemarket.com/ [O] | si | otro |  | C |
| 366 | Quixel Mixer | 1e | https://quixel.com/products/mixer [O] | si | otro |  | C |
| 367 | ReShade | 1e | https://reshade.me/ [O] | si | otro |  | C |
| 368 | Unity | 1e | https://unity.com/ [O] | si | otro |  | C |
| 369 | Unity Asset Store | 1e | https://assetstore.unity.com/ [O] | si | otro |  | C |
| 370 | Unity Gaming Services | 1e | https://dashboard.unity3d.com/ [O] | si | otro |  | C |
| 371 | Bolt Visual Scripting | 1e | https://docs.unity3d.com/Packages/com.unity.bolt@1.4/manual/bolt-installation.html [O] | si | otro |  | C |
| 372 | Code Monkey (cursos Unity) | 1e | https://unitycodemonkey.com/freecourses.php [O] | si | otro |  | C |
| 373 | Unreal Marketplace | 1e | https://unrealengine.com/marketplace/ [O] | si | otro |  | C |
| 374 | MetaHuman | 1e | https://www.metahuman.com/ [O] | si | otro |  | C |
| 375 | Spine | 1e | https://es.esotericsoftware.com/ [O] | si | otro |  | C |
| 376 | Modddif | 1e | https://modddif.com/ [O] | si | otro |  | C |
| 377 | OpenPencil | 1e | https://openpencil.dev/ [O] | si | canvas |  | B |
| 378 | drawDB | 1e | https://www.drawdb.app/ [O] | si | otro |  | C |
| 379 | Story Engine | 1e | https://storyengine.live/ [O] | si | otro |  | C |
| 380 | Kenney | 1e | https://kenney.nl/ [O] | si | otro |  | C |
| 381 | OpenGameArt | 1e | https://opengameart.org/ [O] | si | otro |  | C |
| 382 | Poly Pizza | 1e | https://poly.pizza/ [O] | si | otro |  | C |
| 383 | ambientCG | 1e | https://ambientcg.com/ [O] | si | otro |  | C |
| 384 | Poly Haven | 1e | https://polyhaven.com/ [O] | si | otro |  | C |
| 385 | freepbr | 1e | https://freepbr.com/ [O] | si | otro |  | C |
| 386 | textures.com | 1e | https://www.textures.com/ [O] | si | otro |  | C |
| 387 | Poliigon | 1e | https://www.poliigon.com/ [O] | si | otro |  | C |
| 388 | ShareTextures | 1e | https://www.sharetextures.com/ [O] | si | otro |  | C |
| 389 | Sketchfab | 1e | https://sketchfab.com/ [O] | si | otro |  | C |
| 390 | CGTrader | 1e | https://www.cgtrader.com/ [O] | si | otro |  | C |
| 391 | GameBanana | 1e | https://gamebanana.com/ [O] | si | otro |  | C |
| 392 | S2V (Source 2 viewer) | 1e | https://s2v.app/ [O] | si | otro |  | C |
| 393 | Pixabay | 1e | https://pixabay.com/ [O] | si | otro |  | C |
| 394 | Meshy | 1e | https://www.meshy.ai/ [O] | si | otro |  | C |
| 395 | Tripo | 1e | https://studio.tripo3d.ai/ [O] | si | otro |  | C |
| 396 | Kaedim | 1e | https://www.kaedim3d.com/ [O] | si | otro |  | C |
| 397 | 3D AI Studio | 1e | https://www.3daistudio.com/ [O] | si | otro |  | C |
| 398 | Top3D.ai | 1e | https://www.top3d.ai/ [O] | si | otro |  | C |
| 399 | 80.lv (retopología IA) | 1e | https://80.lv/articles/new-ai-powered-tool-to-automate-retopology-for-3d-artists [O] | si | otro |  | C |
| 400 | NeuralFur | 1e | https://neuralfur.is.tue.mpg.de/ [O] | si | otro |  | C |
| 401 | TRELLIS.2 | 1e | https://microsoft.github.io/TRELLIS.2/ [O] | si | otro |  | C |
| 402 | Laragon | 1e | https://laragon.org/download [O] | si | otro |  | C |
| 403 | XAMPP / Bitnami | 1e | https://www.apachefriends.org/ · https://bitnami.com/ [O] | si | otro |  | C |
| 404 | Contabo | 1e | https://contabo.com/ [O] | si | otro |  | C |
| 405 | Cloudways | 1e | https://www.cloudways.com/ [O] | si | otro |  | C |
| 406 | Hostinger | 1e | https://www.hostinger.com/ [O] | si | otro |  | C |
| 407 | alwaysdata | 1e | https://www.alwaysdata.com/ [O] | si | otro |  | C |
| 408 | Railway | 1e | https://railway.app/ [O] | si | otro |  | C |
| 409 | Render | 1e | https://render.com/ [O] | si | otro |  | C |
| 410 | Neon | 1e | https://neon.tech/ [O] | si | otro |  | C |
| 411 | Vercel | 1e | https://vercel.com/ [O] | si | otro |  | C |
| 412 | Netlify | 1e | https://www.netlify.com/ [O] | si | otro |  | C |
| 413 | Cloudinary | 1e | https://cloudinary.com/ [O] | si | otro |  | C |
| 414 | Supabase | 1e | https://supabase.com/ [O] | si | otro |  | C |
| 415 | MongoDB Atlas | 1e | https://www.mongodb.com/atlas [O] | si | otro |  | C |
| 416 | Turso | 1e | https://github.com/tursodatabase/turso [O] | si | otro |  | C |
| 417 | DuckDNS | 1e | https://www.duckdns.org/ [O] | si | otro |  | C |
| 418 | Termius | 1e | https://termius.com/ [O] | si | otro |  | C |
| 419 | Synergy | 1e | https://symless.com/synergy [O] | si | otro |  | C |
| 420 | Radmin VPN / Hamachi | 1e | https://www.radmin-vpn.com/ · https://vpn.net/ [O] | si | otro |  | C |
| 421 | Tailscale | 1e | https://tailscale.com/ [O] | si | otro |  | C |
| 422 | Shadow.tech | 1e | https://shadow.tech/ [O] | si | otro |  | C |
| 423 | MSYS2 | 1e | https://www.msys2.org/ [O] | si | otro |  | C |
| 424 | Zeal | 1e | https://zealdocs.org/ [O] | si | otro |  | C |
| 425 | DevDocs | 1e | https://devdocs.io/ [O] | si | otro |  | C |
| 426 | JSON Crack | 1e | https://jsoncrack.com/ [O] | si | otro |  | C |
| 427 | n8n-workflows | 1e | https://github.com/Zie619/n8n-workflows [O] | si | canvas |  | B |
| 428 | roadmap.sh | 1e | https://roadmap.sh/ [O] | si | otro |  | C |
| 429 | Boot.dev | 1e | https://www.boot.dev/ [O] | si | otro |  | C |
| 430 | Frontend Mentor | 1e | https://www.frontendmentor.io/ [O] | si | otro |  | C |
| 431 | freeCodeCamp | 1e | https://www.freecodecamp.org/ [O] | si | otro |  | C |
| 432 | VisuAlgo | 1e | https://visualgo.net/ [O] | si | otro |  | C |
| 433 | BridgeBench | 1e | https://www.bridgebench.ai/ [O] | si | otro |  | C |
| 434 | OSSU / TeachYourselfCS | 1e | https://ossu.firebaseapp.com/ · https://teachyourselfcs.com/ [O] | si | otro |  | C |
| 435 | adventJS | 1e | https://adventjs.dev/ [O] | si | otro |  | C |
| 436 | codewars / leetcode / hackerrank | 1e | https://www.codewars.com/ · https://leetcode.com/ · https://www.hackerrank.com/ [O] | si | otro |  | C |
| 437 | fullstackopen | 1e | https://fullstackopen.com/ [O] | si | otro |  | C |
| 438 | Open Bootcamp / Codely / EDteam | 1e | https://open-bootcamp.com/ · https://codely.com/ · https://ed.team/ [O] | si | otro |  | C |
| 439 | it's free* | 1e | https://itsfree.dev/ [O] | si | otro |  | C |
| 440 | NoSubscription.org | 1e | https://nosubscription.org/ [O] | si | otro |  | C |
| 441 | LearnXinYMinutes | 1e | https://learnxinyminutes.com/ [O] | si | otro |  | C |

---

## 8. Comandos ejecutados

- `node "C:/Users/Max/AppData/Local/Temp/opencode/gen64.cjs"` — parser+clasificador sobre RFC 62; salida exacta arriba (441 filas).
- Lectura directa de `Atlas OS/research/62` líneas 1–653 [O]; `docs/coordination/PLAN-docs-hardening.md`; `docs/design/CONSENSUS_AUDIT.md`.

> Deep-dive (prioridad A): `Atlas OS/research/ux-catalog/` (fichas por categoría) + `_pending-A.md` (control de completitud A).

---

## 9. Matriz función × referente (para RFC 24 / 65 / 66)

Sirve a la spec de UI: por cada **función** que el HUD de orquestación necesita, qué referentes la evidencian ([Os] verificado esta sesión, [Link] = `docs/design/CONSENSUS_AUDIT.md`, [I] inferido), y el **estado en Atlas** (qué RFC la cubre) y el **hueco** (funciones que Atlas NO documenta hoy). Fuente de columnas "estado/hueco": RFC 24, 65, 66 y el resumen de RFC 26 [O].

| Función | Referentes con evidencia | Evidencia | Estado en Atlas (RFC) | Hueco / acción |
|---|---|---|---|---|
| Estado por agente (`working/blocked/done/idle`) | Herdr (5 estados), Orca (`permission>done>heuristic`), Zed, n8n | [Link] | RFC 24 tarjeta + RFC 66 enum real | Solo Herdr documenta `unknown`; Atlas no lo tiene → **añadir `Unknown`** (RFC 66 OA). |
| **Rollup de estado agente→misión** | Herdr (pane→tab→workspace), Orca, Langfuse (session) | [Link] | RFC 66 (§2.1 matiza) | **Rollup no especificado** en RFC 24/65 → especificar en el Mission rail. |
| Kanban de tareas/issues como raíz | Vibe Kanban, Linear, **Agent Teams AI** (Kanban + terminales) | [Link]/[Os] | RFC 65 Kanban | Cubierto; validar paridad con "Kanban + terminales integradas". |
| Worktree por agente | Orca, Crystal, Omnigent (sandbox/terminal) | [Link]/[Os] | RFC 05 §3, RFC 24 §12 | Cubierto. |
| Revisión de diff inline con comentarios | Vibe Kanban, Zed, Cline (Compare) | [Link]/[Os] | RFC 24 demos-over-diff | Parcial: comentario inline→agente solo está en Vibe; verificar en RFC 24. |
| Plan↔Act (modos con contexto preservado) | Cline, Kiro (specs), opencode | [Os] | RFC 21, RFC 23 (`architect/code`) | Cubierto conceptualmente; falta el toggle de UI documentado. |
| Checkpoints / rewind | Cline (shadow-git), Kiro (rewind/fork), Atlas | [Os] | RFC 19 checkpoints | Cubierto; adoptar las 3 granularidades (files/task/both). |
| **Auto-approve + snapshots (binomio)** | Cline (auto-approve↔checkpoints), Warp (approve-before-lands) | [Os] | RFC 24 approvals (parcial) | **No documentado como binomio** autonomía↔red-de-seguridad → añadir a RFC 24/66. |
| **Gate vs question (dos canales de aprobación)** | Orca (`decision gate` bloquea tarea vs `ask` del worker) | [Link] | RFC 66 approvals dock (un solo dock) | **Hueco**: RFC 66 colapsa gate+question → split propuesto (OA-66). |
| Command palette con modos (`>`/`#`/archivo) | VS Code, Orca, opencode (`/` + leader `ctrl+x`) | [Link]/[Os] | RFC 65 CommandPalette | Cubierto; decidir `:` (TUI) vs `Ctrl+Shift+P` (IDE) explícitamente. |
| **Canvas ↔ log sincronizado** | n8n (log docked + selection sync) | [Link] | RFC 66 §10 | **Activity Spine sin link al canvas** → acoplar selección canvas↔log. |
| **Aggregated ↔ Expanded** | Langfuse (agent graph), LangSmith (Messages/Turns/Details) | [Link]/[Os] | RFC 65 | Propuesto en RFC 66 §10; confirmar implementación. |
| Traza paso a paso (model/retrieval/tool) | Phoenix, Langfuse, LangSmith, n8n | [Os]/[Link] | RFC 24 §10 audit / execution log | Parcial; unificar con la Timeline. |
| Coste/tokens por step | DeerFlow (token budget que agrega subagentes), Helicone | [Os] | RFC 24 §6/§7 Cost&Health | Cubierto. |
| **Gauge de contexto persistente en header** | DeerFlow (`context_window` gauge + compaction) | [Os] | RFC 66 "Calm Instrumentation" | **No documentado** → añadir gauge de contexto al header del chat/HUD. |
| Skills/MCP activation visible por conversación | AionUi (skill indicator en header), Kiro (Powers/Skills) | [Os] | RFC 24 §8; RFC 65 item 10 | Parcial: skills activables; **MCP read-only** → decidir activación MCP. |
| Sesiones paralelas + tareas programadas | Cline Desktop, Warp (Agent Mgmt Panel), Agent Teams AI | [Os] | RFC 05 swarm | Parcial; falta UI de scheduling. |
| **Takeover humano (IDE/browser)** | Devin (IDE embebido + Browser interactivo con takeover) | [Os] | RFC 66 HITL | **No documentado** → especificar takeover sin perder el hilo. |
| **Handoff local↔cloud** | Warp (steer/handoff, contexto compartido) | [Os] | RFC 36 remote | **No documentado** a nivel de UX. |
| Multi-harness / BYOA (orquestar CLIs de terceros) | AionUi (20+ CLIs), Omnigent (meta-harness), Orkas | [Os] | RFC 27 BYOA; RFC 63/64 | Parcial; es la tesis de plataforma → reforzar en la UI. |
| Specs como artefactos (req→design→tasks) | Kiro (Specs + Bugfix Specs) | [Os] | RFC 12, RFC 14, RFC 23 | Cubierto; adoptar plantilla req→design→tasks visible. |
| Done con prueba de trabajo (proof of work) | Symphony (CI/PR/video), Cline (Compare) | [Os] | RFC 14 §10 EvidenceGate | Cubierto. |
| **Canvas infinito con paneles dock/undock** | Cate (canvas zoomable; tabs/ventanas) | [Os] | RFC 65 Canvas view | **No documentado** → considerar como modo avanzado. |
| **Voz como canal de orquestación** | OpenClaw (workflows por voz), qwen-audio-agent | [Os] | RFC 24 §16 mobile/remote | **No documentado** → candidato post-MVP. |
| Full-vs-sandbox como control visible | OpenClaw (full/sandbox "your choice") | [Os] | RFC 18, RFC 21 | Parcial; hacer sandbox el default visible. |
| Onboarding guiado (sign-up→key→first run) | Helicone, Kiro (first project), opencode (`/init`) | [Os] | RFC 17 noob mode | Parcial; falta flujo de primer arranque documentado. |

**Funciones que Atlas NO documenta hoy (huecos detectados):** rollup de estado agente→misión (parcial), binomio auto-approve↔snapshots, split gate/question en aprobaciones, canvas↔log sync, gauge de contexto en header, takeover humano IDE/browser, handoff local↔cloud (UX), canvas infinito con paneles dock/undock, voz como canal. Todas trazables a evidencia [Os]/[Link] arriba.

---

## 10. Cobertura del deep-dive (reporte final — SEGUNDA PASADA)

- **Prioridad A total:** 93.
- **Fichadas:** **81** = **63 [Os]** (verificadas en vivo, fuente citada) + **18 [P] con motivo concreto** (URL + error o ausencia de doc pública).
- **Enlazadas a `CONSENSUS_AUDIT.md`** (no se repiten): **12**.
- **[P] sin ficha:** **0** — todas resueltas a ficha (eventualmente [P] con motivo) o enlace.
- **Categorías de ficha:** `terminal.md`, `ide.md`, `ade-orquestador.md`, `observabilidad.md`, `chat-workspace.md`, `otro.md`, + `_pending-A.md` (control).
- **Criterio de cierre:** catálogo 100% (441 ✅) · cada A con ficha (81) o enlace (12), 0 pendientes ✅.
- **Matriz función×referente:** **§9 de este archivo**.


