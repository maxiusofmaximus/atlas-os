# ux-catalog — control de completitud de prioridad A (cierre, script `count.mjs`)

> **Generado por `Atlas OS/research/ux-catalog/count.mjs`** (ejecutable desde cualquier cwd). No editar a mano: re-ejecutar el script.

## 0. Mínimo de profundidad para **[Os]**
Una ficha es **[Os]** si cumple **las dos**: (1) URL oficial abierta en esta sesión (citada); (2) **≥4 de 7** campos con contenido concreto citado (funcionalidades · layout/navegación · estados/feedback · aprobaciones HITL · atajos · onboarding/vacío/error · accesibilidad). Sin (1) → **[P]** con motivo; con (1) pero <4 → **[Os parcial]**.

## 1. Fichas por fichero (conteo del script)

| Fichero | total | [Os] | [Os parcial] | [P] | sin_tag |
|---|---:|---:|---:|---:|---:|
| `ade-orquestador.md` | 40 | 19 | 15 | 6 | 0 |
| `canvas.md` | 4 | 2 | 2 | 0 | 0 |
| `chat-workspace.md` | 21 | 9 | 11 | 1 | 0 |
| `ide.md` | 16 | 6 | 6 | 4 | 0 |
| `kanban-pm.md` | 4 | 3 | 1 | 0 | 0 |
| `observabilidad.md` | 11 | 10 | 1 | 0 | 0 |
| `otro.md` | 5 | 0 | 4 | 1 | 0 |
| `outline.md` | 5 | 3 | 1 | 1 | 0 |
| `settings.md` | 4 | 3 | 1 | 0 | 0 |
| `terminal.md` | 22 | 7 | 13 | 2 | 0 |
| **TOTAL** | **132** | **62** | **55** | **15** | **0** |

## 2. Las 93 entradas de prioridad A y su estado

Estado: **enlazada** = cubierta en `docs/design/CONSENSUS_AUDIT.md`; **[Os]** / **[Os parcial]** / **[P]** = ficha en `ux-catalog/`; **SIN_FICHA** = no mapeada.

| # | Entrada A | Capa | Estado | Ficha |
|---|---|---|---|---|
| 1 | opencode (opencode.ai, el runtime del que hereda el nombre del binario) | 1 | Os | `ide.md` |
| 2 | Claude Code | 1 | parcial | `terminal.md` |
| 3 | Codex CLI (OpenAI) | 1 | Os | `terminal.md` |
| 4 | Cursor / Cursor Cloud Agents | 1 | enlazada |  |
| 5 | Windsurf | 1 | Os | `ide.md` |
| 6 | Cline / Roo | 1 | Os | `ide.md` |
| 7 | Aider | 1 | Os | `terminal.md` |
| 8 | Continue.dev | 1 | parcial | `ide.md` |
| 9 | Hermes (NousResearch nousresearch/hermes-agent) | 1 | enlazada |  |
| 10 | OpenClaw | 1 | Os | `ade-orquestador.md` |
| 11 | AionUI (iofficeai/aionui) | 1 | Os | `chat-workspace.md` |
| 12 | Genspark | 1 | enlazada |  |
| 13 | Antigravity / Gemini CLI (Google) | 1 | SIN_FICHA |  |
| 14 | GitHub Copilot Project HydraFusion | 1 | Os | `ide.md` |
| 15 | OpenRouter Fusion / Sakana Fugu | 1 | Os | `observabilidad.md` |
| 16 | Kimi (K2.7 / k3) | 1 | parcial | `chat-workspace.md` |
| 17 | MiniMax (M2/M2.5/M2.7/M3; MiniMax Agent / MiniMax Code) | 1b | parcial | `chat-workspace.md` |
| 18 | Blackbox AI | 1b | P | `ide.md` |
| 19 | Qoder (Alibaba / Bright Zenith) | 1b | Os | `ide.md` |
| 20 | Manus (Monica) | 1b | Os | `chat-workspace.md` |
| 21 | Abacus.AI (ChatLLM / AI Agent) | 1b | parcial | `chat-workspace.md` |
| 22 | Dots (OpenAI, Sep 29 2026, DevDay) | 1b-alwayson | P | `ade-orquestador.md` |
| 23 | Grok Bot (xAI / SpaceXAI, beta Ago 11 2026) | 1b-alwayson | P | `ade-orquestador.md` |
| 24 | Meta Muse · Google Gemini Spark · Instinct | 1b-alwayson | P | `ade-orquestador.md` |
| 25 | OpenHands (All Hands, ex-OpenDevin) | 1c:A | Os | `terminal.md` |
| 26 | Goose (Block) | 1c:A | parcial | `terminal.md` |
| 27 | Aider | 1c:A | Os | `terminal.md` |
| 28 | Plandex | 1c:A | Os | `terminal.md` |
| 29 | gptme | 1c:A | parcial | `terminal.md` |
| 30 | Devon (entropy-research) | 1c:A | parcial | `terminal.md` |
| 31 | Coro Code / Kode CLI / QQCode / Ferrum / zot / g3 / Zap / Coro | 1c:A | P | `terminal.md` |
| 32 | jcode | 1c:A | parcial | `terminal.md` |
| 33 | Coro Code | 1c:A | P | `terminal.md` |
| 34 | Pi / Pi Agent IDE | 1c:A | P | `ide.md` |
| 35 | Claude Code (Anthropic) | 1c:B | parcial | `terminal.md` |
| 36 | Codex CLI (OpenAI) | 1c:B | Os | `terminal.md` |
| 37 | Gemini CLI (Google) | 1c:B | Os | `terminal.md` |
| 38 | Cursor + Cursor CLI + Bugbot | 1c:B | enlazada |  |
| 39 | Windsurf (Cognition) | 1c:B | Os | `ide.md` |
| 40 | Devin (Cognition) | 1c:B | Os | `ade-orquestador.md` |
| 41 | Amp (Sourcegraph) | 1c:B | Os | `ade-orquestador.md` |
| 42 | Cortex Code (Snowflake) / Tabnine CLI / Mentat CLI / Amazon Q Developer / GitHub Copilot CLI | 1c:B | P | `ade-orquestador.md` |
| 43 | Warp | 1c:B | Os | `settings.md` |
| 44 | FetchCoder (Fetch.ai) | 1c:B | parcial | `ade-orquestador.md` |
| 45 | Qoder + Tongyi Lingma (Alibaba) | 1c:C | Os | `ide.md` |
| 46 | Kimi Code + K2.7/K3 (Moonshot) | 1c:C | SIN_FICHA |  |
| 47 | claude-flow (ruvnet) | 1c:D | parcial | `ade-orquestador.md` |
| 48 | DeerFlow (ByteDance) | 1c:D | Os | `ade-orquestador.md` |
| 49 | Symphony (OpenAI) | 1c:D | Os | `ade-orquestador.md` |
| 50 | Omnigent (Databricks) | 1c:D | Os | `ade-orquestador.md` |
| 51 | claude-flow / gastown / OMK / kodo / wreckit / OpenCastle / 5dive | 1c:D | parcial | `ade-orquestador.md` |
| 52 | CliDeck / GridBash / tlbx / ADHDev / cmux | 1c:D | Os | `ade-orquestador.md` |
| 53 | Conductor | 1c:D | enlazada |  |
| 54 | tmux / Zellij / Overmind | 1c:D | enlazada |  |
| 55 | Vibe Kanban (BloopAI) | 1c:D | enlazada |  |
| 56 | AgentSwarms / Overbrilliant OB-1 / darce / Forge / CLAII / Nausicaa / Jazz / Smelt | 1c:D | P | `ade-orquestador.md` |
| 57 | Dify / Flowise / n8n | 1c:E | enlazada |  |
| 58 | Coder | 1c:G | parcial | `ade-orquestador.md` |
| 59 | Gitpod / Ona | 1c:G | parcial | `ide.md` |
| 60 | GitHub Codespaces | 1c:G | parcial | `ide.md` |
| 61 | DevPod | 1c:G | parcial | `ide.md` |
| 62 | code-server / openvscode-server | 1c:G | P | `ide.md` |
| 63 | Bunnyshell | 1c:G | P | `ide.md` |
| 64 | Langfuse (DE, MIT) | 1c:H | enlazada |  |
| 65 | LangSmith | 1c:H | Os | `observabilidad.md` |
| 66 | Arize Phoenix | 1c:H | Os | `observabilidad.md` |
| 67 | Braintrust | 1c:H | Os | `observabilidad.md` |
| 68 | MLflow | 1c:H | Os | `observabilidad.md` |
| 69 | Helicone / Portkey / LiteLLM / OpenRouter / ZenMux | 1c:H | Os | `observabilidad.md` |
| 70 | TruLens / Comet Opik / Lunary / W&B Weave | 1c:H | Os | `observabilidad.md` |
| 71 | AgentOps | 1c:H | Os | `observabilidad.md` |
| 72 | Moonshot Kimi / Zhipu GLM / MiniMax / ByteDance Seed (Doubao) | 1c:J | parcial | `otro.md` |
| 73 | Cursor Composer / SWE-1 (Cognition) | 1c:J | enlazada |  |
| 74 | Dust | 1c:K | Os | `chat-workspace.md` |
| 75 | Qdrant / Langfuse / n8n | 1c:K | enlazada |  |
| 76 | Cognition Devin (vertical SWE) | 1c:L | SIN_FICHA |  |
| 77 | OpenCADStudio (HakanSeven12) | 1c:M | parcial | `otro.md` |
| 78 | mr-mak-workspace (witnesstodark, MIT, 295★) | 1d:directos | Os | `chat-workspace.md` |
| 79 | Flova AI (flova.ai) | 1d:directos | parcial | `chat-workspace.md` |
| 80 | Bolt.new / bolt.diy | 1d:R | Os | `chat-workspace.md` |
| 81 | v0 (Vercel) | 1d:R | Os | `chat-workspace.md` |
| 82 | Lovable | 1d:R | Os | `chat-workspace.md` |
| 83 | Replit Agent | 1d:R | Os | `chat-workspace.md` |
| 84 | Dyad (local) | 1d:R | parcial | `chat-workspace.md` |
| 85 | Onlook (visual editor) | 1d:R | parcial | `chat-workspace.md` |
| 86 | Same.dev | 1d:R | P | `chat-workspace.md` |
| 87 | Pythagora/GPT-Pilot | 1d:R | SIN_FICHA |  |
| 88 | Create.xyz | 1d:R | parcial | `chat-workspace.md` |
| 89 | Firebase Studio (Google) | 1d:R | parcial | `chat-workspace.md` |
| 90 | Glide/Softr | 1d:R | parcial | `chat-workspace.md` |
| 91 | Warp | 1e | Os | `settings.md` |
| 92 | Kiro (AWS) | 1e | parcial | `ade-orquestador.md` |
| 93 | GitHub HydraFusion | 1e | parcial | `ide.md` |

**Recuento A por estado:** Os=39 · parcial=27 · enlazada=11 · SIN_FICHA=4 · P=12 (total 93).

## 3. [P] con motivo (del script)

- [`ade-orquestador.md`] AgentSwarms — orquestación swarm — prioridad A — [P] — - Motivo [P]: búsqueda realizada en `awesome-cli-coding-agents` [Os] → sin entrada para "AgentSwarms". Sin repo/doc localizada.
- [`ade-orquestador.md`] Dots (OpenAI) — agente always-on — prioridad A — [P] — - Motivo [P]: búsqueda realizada en `awesome-cli-coding-agents` (fetch [Os], URL: `raw.githubusercontent.com/bradAGI/awesome-cli-coding-agents/main/README.md`) → sin entrada. Anuncio DevDay 
- [`ade-orquestador.md`] Grok Bot (xAI) — agente always-on — prioridad A — [P] — - Motivo [P]: búsqueda realizada en `awesome-cli-coding-agents` [Os] → sólo aparecen Grok CLI (community) y Grok Build (xAI, TUI), no "Grok Bot". Beta 2026 sin doc de producto localizada.
- [`ade-orquestador.md`] Meta Muse · Google Gemini Spark · Instinct — agentes always-on — prior — - Motivo [P]: búsqueda realizada en `awesome-cli-coding-agents` [Os] → sin entrada. Productos anunciados sin doc de UI pública.
- [`ade-orquestador.md`] Cortex Code (Snowflake) — ADE/CLI corporativo — prioridad A — [P] — - Motivo [P]: reintentado `https://docs.snowflake.com/en/user-guide/snowsight-cortex-code` y `.../cortex-code` → HTTP 404 (fetchs [Os]). URL canónica pendiente; sin doc de UI verificable.
- [`ade-orquestador.md`] Mentat CLI — ADE/CLI corporativo — prioridad A — [P] — - Motivo [P]: `https://github.com/AbanteAI/mentat` → HTTP 404 (el repo no está en esa ruta; fetch [Os]). Requiere localizar el repo nuevo/archivado.
- [`chat-workspace.md`] Same.dev — app-builder agéntico — prioridad A — [P] — - Motivo [P]: `https://same.new/` → HTTP 429 (rate limit) en cuatro intentos de fetch en esta sesión (incl. reintento con espera) [Os]. Sin repo/doc alternativa localizada. Reintentar en pró
- [`ide.md`] code-server / openvscode-server — cloud dev env (IDE en navegador) — p — - Motivo [P]: no abierta esta sesión; URL candidata `coder.com/docs/code-server` (VS Code en navegador). Patrón: IDE en el navegador para el remote HUD.
- [`ide.md`] Bunnyshell — cloud dev env — prioridad A — [P] — - Motivo [P]: no abierta esta sesión; URL candidata `documentation.bunnyshell.com`.
- [`ide.md`] Blackbox AI — IDE — prioridad A — [P] — - Motivo [P]: `https://docs.blackbox.ai/` devuelve una página de redirección SPA ("Redirecting to the docs → /api-reference/chat") sin contenido servido (0.1 KB, fetch [Os]). Requiere abrir 
- [`ide.md`] Pi / Pi Agent IDE — IDE + harness — prioridad A — [P] — - Motivo [P]: sin URL de producto canónica localizada esta sesión. Evidencia parcial [Os] vía `awesome-cli-agents`: aparece como runtime que Orca/Omnigent orquestan ("Codex, Claude Code, Ope
- [`otro.md`] ByteDance Seed (Doubao) — provider — prioridad A — [P] — - Motivo [P]: provider de modelo; sin URL/doc de UI verificada en vivo esta sesión (pendiente console de ByteDance/Volcengine).
- [`outline.md`] DeerFlow — parsing de outline Markdown (bonus, código) — [P] — - Motivo [P]: es un repo de código (`https://github.com/bytedance/deer-flow`), sin documentación de UI; el patrón observado (reconocer ATX headings, ignorar code fences, invalidar por `mtime
- [`terminal.md`] Coro Code — terminal — prioridad A — [P] — - Motivo [P]: búsqueda realizada en `awesome-cli-coding-agents` [Os] → sin entrada. Sin repo/doc localizada.
- [`terminal.md`] g3 — terminal — prioridad A — [P] — - Motivo [P]: búsqueda realizada en `awesome-cli-coding-agents` [Os] → sin entrada. Sin repo/doc localizada.
