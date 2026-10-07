# ux-catalog / chat-workspace

Fichas de referentes prioridad A con **primario = chat / workspace / app-builder agéntico**.
Procedencia como en `terminal.md`. Los ya auditados en `CONSENSUS_AUDIT.md` no se repiten (enlaces al final).

---

## AionUi — chat/workspace (cowork) — prioridad A — [Os]

- **URL (en vivo):** https://github.com/iofficeai/aionui [Os]
- **Funcionalidades clave:** app **"24/7 Cowork"** open-source para **OpenClaw, Hermes, Claude Code, Codex, OpenCode y 20+ agentes CLI más** — es decir, un **workspace que agrupa y personaliza agentes CLI de terceros**; permite "customize your assistants | team them up" [Os]. **21 asistentes profesionales** integrados (Cowork, PPT Creator/Morph PPT/Morph PPT 3D, HUMAN 3.0 Coach, Social Job Publisher, …) [Os].
- **Layout y navegación:** chat por conversación; **skill indicator en el header del chat** muestra las skills activas de la conversación; búsqueda y exclusión de skills [Os].
- **Estados y feedback:** el skill indicator por conversación es feedback de configuración activa [Os]. Estados de agente **[P]**.
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** "Built-in Agent — Install & Go, Zero Configuration" [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **workspace que integra agentes CLI externos** (patrón BYOA/multi-harness) — muy alineado con el posicionamiento de Atlas como plataforma sobre cualquier CLI/agente; (b) **skill indicator en el header** (qué capacidades están activas AHORA, por conversación) — feedback barato y claro, candidato para el Skill/MCP rail y el header de RFC 24/65; (c) **three-tier skills** (builtin/custom/extension) con toggle por conversación.
- **Evitar:** [P].

---

## Lovable — app-builder agéntico — prioridad A — [Os]

- **URL (en vivo):** https://docs.lovable.dev/features/agent-mode [Os]
- **Funcionalidades clave:** "Build mode (previously Agent mode) is Lovable's autonomous execution mode... it takes ownership of execution end to end... applies changes across files, and resolves issues that appear during development" [Os]. El project chat tiene **tres modos**: **Chat mode** (discutir, "without a plan or code changes"), **Plan mode** ("investigate and write a plan you edit and approve before building"), **Build mode** ("implement changes and verify the outcome") — se cambia entre ellos en cualquier momento y la conversación se mantiene [Os].
- **Layout y navegación:** el **project chat** conmuta de modo; la **Details view** "opens where the preview usually appears", con pestañas **Timeline** ("every step Lovable took, including tool calls") y **Changes** ("the resulting file changes"); botón **Hide details** vuelve al preview [Os]. En el chat, `@` referencia ficheros del proyecto ("Type `@` and select a file") [Os].
- **Estados y feedback:** mientras trabaja, "tasks appear in the project chat showing: Current step being executed, Files being modified, Tools being used (search, web fetch, image generation), Progress through multi-step implementations" [Os]; el menú **More options** muestra "Credits used while the request is still running, and the final cost when it finishes" [Os]; botón **stop** detiene la tarea conservando el trabajo hecho [Os].
- **Aprobaciones / HITL:** **Plan mode** = "investigate and write a plan you edit and approve before building" [Os]; "review the results before moving on" [Os]; guardrails explícitos en el prompt ("Do not modify @src/shared/Layout.tsx or the existing authentication logic") [Os].
- **Atajos de teclado:** `@` para referenciar ficheros en el chat [Os]; el resto [P].
- **Onboarding / vacío / error:** tarjeta de **créditos agotados** con acciones **Add credits** / **Finish up** (la petición "pauses rather than ending") [Os]; **undo** revierte al estado previo [Os].
- **Accesibilidad:** [P] (no documentada en esta página).
- **Adoptar / Evitar para Atlas:** **Adoptar** (a) los **tres modos Chat/Plan/Build** como el selector Plan↔Act (RFC 21/23) — "approve the plan before building" es el gate de aprobación de RFC 24 §5; (b) la **Details view** con Timeline (pasos + tool calls) y Changes (diffs) sustituyendo al preview — patrón directo para el Activity Spine / Audit de RFC 65; (c) **coste en vivo durante la ejecución** + pausa por presupuesto con acción de reanudar (paralelo al `CostThresholdCrossed` + budget guard de RFC 19). **Evitar** el vocabulario de "credits" (Atlas mide tokens/USD).

---

## Replit — app-builder / IDE cloud — prioridad A — [Os]

- **URL (en vivo):** https://docs.replit.com/replitai/agent [Os]
- **Funcionalidades clave:** "Agent takes action: it sets up your project, creates applications, checks its work, and fixes problems along the way" [Os]. **Task system**: "Kanban planning, background tasks, and changes applied back to the main version" [Os]. **Design Canvas**: "Visual mockups and the hands-on visual editor" [Os]. **Agent Skills**: "Teach Agent specialized knowledge — use pre-built skills or create your own" [Os].
- **Layout y navegación:** el **Project Editor** es la raíz: "In the Project Editor, just start chatting" [Os]; tras describir, "Optionally, select a project type: web app, mobile app, slides, design, data visualization" [Os]; varios artefactos por proyecto, "all sharing the same backend and data" [Os].
- **Estados y feedback:** "Agent tests its own work on a regular basis. Agent also creates **checkpoints** as it works, so you can **roll back to any previous state**" [Os]; modos de modelo **Free / Power / Max** (Auto selecciona modelo por tarea) [Os].
- **Aprobaciones / HITL:** "**Paid actions require confirmation before they start**" [Os]; rollback a checkpoint como control de reversión [Os].
- **Atajos de teclado:** [P] (no en la página del agente).
- **Onboarding / vacío / error:** "Start building now — Describe your idea and let Agent bring it to life — **no setup required**" [Os]; ante errores: "chat with Agent to describe what went wrong and it will fix the issue" [Os].
- **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** **Adoptar** (a) **checkpoints + rollback** explícitos como parte del estado del agente (encaja con RFC 19 y el `JournalCheckpoint` del bus); (b) **Task system = Kanban planning + background tasks + changes applied back to main** (paralelo al Kanban de RFC 65 y a worktrees/swarm); (c) **confirmación de acciones de pago** como precedente del gate HITL por coste. **Evitar** mezclar artefactos heterogéneos (slides/video) en un mismo "project" — Atlas es engineering-first.

---

## Manus (Monica) — chat/workspace (agente "hands-on") — prioridad A — [Os]

- **URL (en vivo):** https://manus.im/ (blog: /blog/introducing-manus-2-0) [Os]
- **Funcionalidades clave:** "Manus: Hands On AI"; **Manus 2.0** anunciado; landing **"What can I do for you?"** con acciones rápidas: **Create slides / Build website / Create games / Video / Design / More** [Os].
- **Layout y navegación:** pattern de **landing de tareas con tarjetas de acción** (entry point por tipo de entrega, no por prompt vacío) [Os]. Layout interno de sesión **[P]**.
- **Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar:** el **entry point por tarjetas de acción** ("¿qué quieres hacer?") en el estado vacío del HUD — buena respuesta al requisito de onboarding/vacío; alternativa a un prompt en blanco.
- **Evitar:** [P].

## MiniMax (MiniMax Agent / MiniMax Code) — chat/workspace + provider — prioridad A — [Os parcial]

- **URL (en vivo):** https://www.minimax.io/ [Os]
- **Funcionalidades clave:** sitio de **modelos** (MiniMax M3, M2.7, …) con secciones LLM/multimodal; MiniMax Agent/Code como producto agéntico [Os]. Detalle de UI de Agent **[P]** (no en la home).
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar:** [P] — cubrir desde la doc de MiniMax Agent (`/agent`) en la próxima pasada.
- **Nota:** doble rol (provider en Capa J + producto agéntico en 1b).

## Abacus.AI (ChatLLM / AI Agent) — chat/workspace — prioridad A — [Os parcial]

- **URL (en vivo):** https://abacus.ai/ [Os]
- **Funcionalidades clave:** "The AI super assistant for professionals and enterprises"; **100+ modelos** listados (Fable 5.1, GPT-6.1 Sol, Opus 5.5, Gemini 3.1 Pro, Grok 4.7, Kimi K3, Nano Banana 2.5…) [Os].
- **Layout y navegación:** selector de **modelo** prominente con catálogo amplio [Os]. Panel de agente/enterprise **[P]**.
- **Estados / Aprobaciones / Atajos / Onboarding-vacío-error / Accesibilidad:** [P].
- **Adoptar:** (a) **catálogo de modelos visible y seleccionable** como control de primer nivel (encaja con el Model Orchestrator de RFC 04); (b) enfoque **profesional/enterprise** (multi-usuario) — referencia para el ángulo de equipo.
- **Evitar:** [P].

## Dust — chat/workspace ("multiplayer AI") — prioridad A — [Os]

- **URL (en vivo):** https://docs.dust.tt/docs/user-documentation/getting-started (y `.../dust-rollout-guide/welcome-to-dust`) [Os]
- **Funcionalidades clave:** "Dust is a platform that enables teams to **create customizable and secure AI agents powered by leading large language models**. These agents **integrate with company data sources**"; "**Unified AI workspace**" que conecta Slack, Google Drive, Notion, Confluence, GitHub [Os].
- **Layout y navegación:** "To use an agent, simply type your message and mention the agent with '**@**'" [Os]; "browse the other agents your team already created or start creating your own. Learn more about any agent (instructions, tools, usage) by clicking on the **3 dots**" [Os].
- **Estados y feedback:** [P].
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** `@` para invocar un agente dentro del mensaje (input pattern) [Os]; el resto [P].
- **Onboarding / vacío / error:** "**Ask your first question**... Start asking questions to **@dust** (set up by default)"; "Talk to specialized agents built by you or your colleagues"; "Now, you probably want to know how to **build your first agent**" [Os].
- **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** **Adoptar** (a) **multi-agent en la misma conversación** ("Dust allows you to call multiple agents in one discussion and have them work together") — paralelo directo al Swarm (RFC 05); (b) la **invocación por `@agente`** como input pattern; (c) el ángulo **"multiplayer"** (humanos + agentes sobre el mismo workspace). **Evitar** la dependencia de datos enterprise externos como único valor.

## Bolt.new / bolt.diy — app-builder agéntico — prioridad A — [Os]

- **URL (en vivo):** https://bolt.new/ [Os]
- **Funcionalidades clave:** "Bolt AI builder: Websites, apps & prototypes"; "Create stunning apps & websites **by chatting with AI**" [Os].
- **Layout y navegación:** prompt central **"What will you build today?"** con dos botones **Plan / Build now** (elección planificar-vs-construir en el entry point) y tipos de proyecto (**Website / Slides / App / Prototype**); "or start from **Figma / GitHub / Team template**" [Os].
- **Estados / Aprobaciones / Atajos / Onboarding-vacío-error / Accesibilidad:** [P].
- **Adoptar:** (a) elección **Plan vs Build now** en el primer clic (muy alineado con Plan↔Act y RFC 21/23); (b) **entry point con tipos de entrega + templates** (Figma/GitHub/Team) como estado vacío rico.
- **Evitar:** [P].

## v0 (Vercel) — app-builder agéntico — prioridad A — [Os]

- **URL (en vivo):** https://v0.app/docs/agentic-features [Os]
- **Funcionalidades clave:** "v0's intelligent agent capabilities for web search, browser use, terminal commands, error fixing, and external tool integration... Every action runs inside an isolated sandbox, and **you control how much autonomy v0 has**" [Os]; "Deploy to production immediately, or **open a pull request for review**" [Os].
- **Layout y navegación:** [P] en `/docs/agentic-features` (no enumera paneles; remite a la sección "User interface").
- **Estados y feedback:** sección **Real-time feedback**: "v0 shows you what it's doing as it works: **Progress indicators** (live updates on each agent action), **Browser screenshots** (visual snapshots from browser use), **Citation links** (clickable sources), **Tool execution cards** (status updates for terminal commands and external tool calls)" [Os]; "Handles errors gracefully: recovers from issues and tries alternative approaches" [Os].
- **Aprobaciones / HITL:** "open a pull request for review" [Os]; "you control how much autonomy v0 has" (autonomía configurable) [Os].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** **Fix with v0**: "When a deployment has errors or warnings, a **Fix with v0** button appears in the deployment popover. Clicking it sends the error logs to v0, which diagnoses the issue and applies a fix automatically" [Os].
- **Accesibilidad:** [P] (v0 lista "add accessibility improvements" como tarea del agente, no la a11y de su propia UI).
- **Adoptar / Evitar para Atlas:** **Adoptar** (a) **Tool execution cards** + progress indicators + screenshots como feedback por acción — patrón directo del Agent Card / Activity Spine de RFC 65; (b) **autonomía configurable** ("control how much autonomy") — encaja con `SandboxLevel`/`ExecutionMode` (RFC 18/21); (c) **PR-for-review** como gate de entrega. **Evitar** el modelo "créditos/free cap" en la UI.

## Dyad — app-builder local — prioridad A — [Os parcial]

- **URL (en vivo):** https://www.dyad.sh/ [Os]
- **Funcionalidades clave:** "**Free, Local, Open-Source** AI App Builder"; "The open-source AI app builder **on your desktop**"; "Use your favorite AI models. **Own your code. Zero lock-in**"; apps nativas macOS (Apple Silicon/Intel) y Windows [Os].
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar:** **local-first + BYO model + own-your-code** — tesis directa de Atlas (local, sin lock-in); desktop nativo por OS.
- **Evitar:** [P].

## Flova AI — agente vertical creativo (video) — prioridad A — [Os parcial]

- **URL (en vivo):** https://flova.ai/ [Os]
- **Funcionalidades clave:** "Flova AI — **AI Animation and Video Generator for Filmmakers and Creators**" [Os]. Detalle (Agent Canvas, skills, CLI puente a Codex/Claude Code, según RFC 62) **[P]** — la home solo expone el título.
- **Adoptar:** [P] hasta abrir la app/docs.
- **Nota:** referente de **agente vertical creativo** (Engine por dominio, RFC 64).

## mr-mak-workspace — workspace local de escritorio (Tauri) — prioridad A — [Os]

- **URL (en vivo):** https://github.com/witnesstodark/mr-mak-workspace [Os]
- **Funcionalidades clave:** "A **local desktop workspace for Codex and Claude Code**, with **project reports**, **creative skills** and **optional voice**"; Tauri (local, Windows x64 + Linux), MIT [Os]. "The native desktop app adds **managed terminals, local file operations and voice**" [Os].
- **Layout y navegación:** **Chats** como superficie: "use **+** in Chats to open an installed CLI and sign in with your own account" [Os]; workspace con project reports + skills creativas + panel MCP [Os].
- **Estados y feedback:** las entregas de assets "can carry a local **SHA-256 manifest**... Verification checks file integrity" [Os].
- **Aprobaciones / HITL:** "**visual approval and engine readiness remain separate decisions**" (aprobación humana explícita, separada de la verificación técnica) [Os].
- **Atajos de teclado:** **Win-key shortcut** opcional, off por defecto: "Leave optional voice, paid providers, MCP connections and the **Win-key shortcut off until I choose to connect them**" [Os].
- **Onboarding / vacío / error:** setup dirigido al propio agente: "Read AGENTS.md and docs/getting-started.md, check the prerequisites... **Leave optional [features] off until I choose**" [Os].
- **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** **Adoptar** (a) **primo directo de Atlas**: workspace local Tauri que orquesta CLIs externos (Codex/Claude Code) con skills + voz opcional + MCP — validación de arquitectura; (b) **aprobación visual separada de la verificación técnica** (dos decisiones distintas) — matiz útil para el Approval Dock de RFC 65; (c) **opt-in explícito** de capacidades (voz/MCP/shortcut off por defecto) — alinea con el default seguro de RFC 18.

## Kimi (Moonshot) — chat/workspace + provider — prioridad A — [Os parcial]

- **URL (en vivo):** https://platform.moonshot.ai/ (API Kimi K3) [Os]
- **Funcionalidades clave:** plataforma LLM **Kimi** con **K3** lanzado; "Build with Kimi API"; partner logos (Vercel, Trickle…) [Os]. El producto de **agente/CLI** (Kimi Code) se documenta aparte **[P]**.
- **Adoptar:** [P] (cubrir Kimi Code / Kimi CLI en próxima pasada).
- **Nota:** cubre **Capa 1 `Kimi (K2.7/k3)`** y **1c:C `Kimi Code`**; el bundle **1c:J `Moonshot Kimi / …`** queda fuera.

## Onlook — app-builder de diseño — prioridad A — [Os parcial]

- **URL (en vivo):** https://github.com/onlook-dev/onlook · https://docs.onlook.com/ [Os]
- **Funcionalidades clave:** "The **Developer Tool for Designers** • An **Open-Source AI-First Design tool** • **Visually build, style, and edit your code with AI** • #1 Developer tool for Designers to design with Real Code" [Os].
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P] (docs.onlook.com es índice; falta la ruta de doc de producto).
- **Adoptar / Evitar para Atlas:** adoptar **diseño visual sobre código real** (designer-first, open-source, self-hosting); evitar [P].

## Create.xyz (ahora "Anything") — app-builder agéntico — prioridad A — [Os parcial]

- **URL (en vivo):** https://www.create.xyz/ (rebrand a **Anything**, `anything.com`) [Os]
- **Funcionalidades clave:** "Anything — AI app builder"; "**Turn your words into mobile apps, sites**…" [Os].
- **Adoptar:** prompt→app multi-target (móvil + web); [P] resto.
- **Layout/Estados/Aprobaciones/Atajos/Onboarding/Accesibilidad:** [P].

## Firebase Studio (Google) — app-builder / IDE cloud — prioridad A — [Os parcial]

- **URL (en vivo):** https://firebase.google.com/docs/studio [Os]
- **Funcionalidades clave:** "Firebase Studio" (entorno de desarrollo asistido por Google; doc oficial) [Os]. Capacidades concretas **[P]** en la indexada.
- **Adoptar:** [P] hasta abrir las guías de la doc.

## Glide — no-code + agentes — prioridad A — [Os parcial]

- **URL (en vivo):** https://www.glideapps.com/ [Os]
- **Funcionalidades clave:** "AI app builder for internal business apps"; "**Turn spreadsheets into apps & agents you can trust** to run your business"; "puts internal tool development in the hands of **operators**" [Os].
- **Layout y navegación:** [P]. **Estados/Aprobaciones/Atajos/Onboarding/Accesibilidad:** [P].
- **Adoptar:** el ángulo **"apps & agents"** desde una fuente de datos (spreadsheet) — refuerza la idea de agente ligado a datos; audiencia **operator-centric** (no dev).

## Softr — portals / internal tools — prioridad A — [Os parcial]

- **URL (en vivo):** https://www.softr.io/ [Os]
- **Funcionalidades clave:** "Build Secure Custom **Portals and Internal Tools**" [Os].
- **Adoptar:** patrón de **portal interno seguro** (útil si Atlas expone HUD web a un equipo); [P] resto.

## Same.dev — app-builder agéntico — prioridad A — [P]

- **Motivo [P]:** `https://same.new/` → **HTTP 429 (rate limit)** en **cuatro** intentos de fetch en esta sesión (incl. reintento con espera) [Os]. Sin repo/doc alternativa localizada. Reintentar en próxima pasada.

## GPT-Pilot (Pythagora) — app-builder agéntico OSS — prioridad A — [Os parcial]

- **URL (en vivo):** https://github.com/Pythagora-io/gpt-pilot [Os]
- **Funcionalidades clave:** "The first real AI developer" — orquesta **11 agentes por rol**: "**Product Owner**, **Specification Writer**, **Architect**, **Tech Lead**, **Developer**, **Code Monkey**, **Reviewer**, **Troubleshooter**, **Debugger**, **Technical Writer**" [Os].
- **Estados y feedback:** pipeline secuencial de roles; "**Reviewer agent** reviews every step of the task and if something is done wrong Reviewer **sends it back to Code Monkey**" (loop de revisión) [Os].
- **Layout y navegación:** CLI: `python main.py --project <app_id> --step <step>` ("Load and continue from a specific step"; "this will delete all progress after the specified step") [Os]; `--delete <app_id>` [Os].
- **Aprobaciones / HITL:** "Specification Writer agent **asks a couple of questions** to understand the requirements better" [Os]. Sin gate de aprobación por paso documentado.
- **Atajos / Onboarding / Accesibilidad:** [P] (CLI; "You enter the app name and the description" como arranque).
- **Adoptar / Evitar para Atlas:** **Adoptar** el **reparto por rol de agente** (Product Owner/Architect/Tech Lead/Developer/Reviewer/…) como taxonomía de subagentes del Swarm (RFC 05) y el **loop Reviewer→Code Monkey** (encaja con Repair RFC 15); el `--step` para reanudar/rehacer desde un paso. **Evitar** el arranque "app name + description" tan pobre.
- **Nota:** 3-4 campos → **[Os parcial]** (sin UI gráfica; es CLI).

## Pazi (antes Pythagora) — plataforma / app-builder agéntico — prioridad A — [Os]

- **URL (en vivo):** https://www.pythagora.ai/ (rebrand a **Pazi**; `docs.pythagora.ai` → DNS ENOTFOUND) [Os]
- **Funcionalidades clave:** "**Pazi — AI Agents That Build and Run Your Business**"; "Tell Pazi — and **it builds a team around you** and starts making it happen" [Os].
- **Layout y navegación:** "**Business Pulse** keeps every project, opportunity, and AI task in sync... all from **one dashboard**"; secciones **Opportunities / Tasks & Calendar** [Os].
- **Estados y feedback:** Business Pulse muestra "**what's done, what's in progress, and what needs you**" — modelo de estado explícito [Os].
- **Aprobaciones / HITL:** el estado "**what needs you**" marca lo que requiere al humano [Os].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** "What's the first thing we'll build together? ... **Let's Start**"; "Describe your business idea"; opción "**Start a new business / I already have a business**" [Os].
- **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** **Adoptar** (a) **Business Pulse** como dashboard de estado agregado ("done / in progress / needs you") — patrón directo del Health KPIs / Mission rail de RFC 65; (b) el estado explícito **"needs you"** (bandeja de atención) ≈ Approval Dock; (c) **"builds a team around you"** (agentes por rol). **Evitar** el giro a "business/emprendimiento" (fuera del foco engineering de Atlas).
- **Nota:** reemplaza el [P] previo (`docs.pythagora.ai` DNS ENOTFOUND); el producto rebrandeó a **Pazi**.

## Cobertura [P] de esta categoría

Estado tras la pasada 4 (este fichero):

- **[Os] real (URL + ≥4 campos):** Lovable, Replit, v0, mr-mak-workspace, Dust, Pazi (ex-Pythagora).
- **[Os parcial] (URL, <4 campos):** GPT-Pilot.
- **[P] con motivo concreto:** Onlook (índice + doc de devs + `/getting-started` 404), Same.dev (HTTP 429 ×3).
- **[P] pendiente (sin abrir esta sesión):** **MiniMax Agent**, **Abacus.AI**, **Dyad**, **Flova AI**, **Kimi (Kimi Code)**, **Create.xyz/Anything**, **Firebase Studio**, **Glide**, **Softr**. Agregados no fichados: **Dots**, **Grok Bot**, **Meta Muse / Gemini Spark**, **Blackbox**.

## Enlaces a referentes ya auditados (no se repiten)

- **Genspark** (AI workspace, ayuda/diseño): `docs/design/CONSENSUS_AUDIT.md` §1.3 #3.
- **Notion** (docs/PM, databases): `CONSENSUS_AUDIT.md` §1.3 #13 (repo) y #18 (live: `/help/intro-to-databases`).
