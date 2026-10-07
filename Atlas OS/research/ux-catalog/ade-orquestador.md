# ux-catalog / ADE / orquestador

Fichas de referentes prioridad A con **primario = ADE / orquestador de agentes**.
Procedencia como en `terminal.md`. Los ya auditados en `CONSENSUS_AUDIT.md` no se repiten (enlaces al final).

---

## OpenClaw — ADE / asistente open-source — prioridad A — [Os]

- **URL (en vivo):** https://openclaw.ai/ · docs https://docs.openclaw.ai/ [Os]
- **Funcionalidades clave:** asistente con **"full system access"**: navega la web, rellena formularios, lee/escribe archivos, ejecuta shell — **full o sandboxed "your choice"**; flujos por **voz** (inspeccionar builds, diagnosticar, redeploy, PR) [Os]. **Control UI** para chat, config y sessions; conexión de **canales** (Discord/Signal/Telegram/WhatsApp) [Os].
- **Layout y navegación:** **Control UI (browser dashboard)** = chat + config + sessions; un **Gateway** corre en foreground o como **servicio de fondo** [Os].
- **Estados y feedback:** [P].
- **Aprobaciones / HITL:** el binomio **full vs sandboxed** es la frontera de confianza explícita [Os]; taxonomía de aprobaciones **[P]**.
- **Atajos de teclado:** [P] (la doc cita Ctrl+C para parar el Gateway) [Os].
- **Onboarding / vacío / error:** installer por OS (`curl|bash`, `iwr|iex`); **onboarding** con **Quick start** (reusa acceso AI detectado y abre el dashboard) o **Custom setup**, más `openclaw onboard --classic`; `openclaw gateway install` → `openclaw dashboard` [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **onboarding guiado Quick/Custom** + `--classic`; (b) **Gateway como servicio de fondo** + Control UI; (c) **full-vs-sandbox como control visible**; (d) canales múltiples (móvil).
- **Adoptar / Evitar para Atlas:** adoptar **onboarding guiado + dashboard Control UI + gateway-servicio**; evitar "full system access" como default percibido (sandbox por defecto, peligroso opt-in).

---

## Devin (Cognition) — ADE autónomo — prioridad A — [Os]

- **URL (en vivo):** https://docs.devin.ai/ [Os]
- **Funcionalidades clave:** ingeniero de software autónomo que escribe/ejecuta/prueba código; ataca tickets Linear/Jira, features completas, bug reports, testing de apps, migraciones/refactors, unit tests, docs [Os]. **CLI** (`curl -fsSL https://cli.devin.ai/install.sh | bash`) [Os].
- **Layout y navegación:** sesión con **sidebar de tools**; **progress steps** clicables en la sesión; **IDE embebido** ("follow Devin's work real-time and take over to run commands, make direct code edits or test") con atajos de IDE familiares; **Browser** interactivo con takeover [Os].
- **Estados y feedback:** progreso por pasos en la sesión; trabajo en paralelo ("tackling many tasks in parallel") [Os].
- **Aprobaciones / HITL:** takeover manual en IDE/Browser (intervención humana directa) [Os]; gates explícitos **[P]**.
- **Atajos de teclado:** IDE embebido con "shortcuts you're familiar with" [Os] (combinaciones concretas **[P]**).
- **Onboarding / vacío / error:** sign-up en app.devin.ai; Help (icono `?`) al fondo del sidebar → Contact support; feedback channel visible [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **takeover** del IDE/browser por el humano sin perder el hilo — patrón fuerte para el HITL de Atlas; (b) **progress steps clicables** que llevan a la tool relevante (n8n-like, alineado con canvas↔log sync de CONSENSUS_AUDIT); (c) IDEs/browser embebidos en la sesión, no ventanas externas.
- **Evitar:** opacidad del estado interno durante tareas largas (Devin lo mitiga con pasos clicables; Atlas debe hacerlo desde el inicio).

---

## claude-flow / ruflo — harness multi-agente — prioridad A — [Os parcial]

- **URL (en vivo):** https://github.com/ruvnet/claude-flow (repo ahora "ruflo") [Os]
- **Funcionalidades clave:** harness de agentes; **plugins**: `ruflo-core` (server, health checks, plugin discovery), `ruflo-swarm` (coordina varios agentes como equipo), `ruflo-autopilot` (loop autónomo), `ruflo-loop-workers` (tareas de fondo por temporizador), `ruflo-workflows` (plantillas multi-paso reutilizables), `ruflo-federation` (agentes en máquinas distintas colaboran) [Os].
- **Layout y navegación:** **slash commands** vía plugins de Claude Code (Path A "lite") + Core & Orchestration [Os]. UI gráfica propia **[P]**.
- **Estados y feedback:** [P].
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** quick start por "paths" (plugins lite vs full) [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **nombrado por responsabilidad** (core/swarm/autopilot/loop-workers/workflows/federation) como taxonomía de capacidades de orquestación; (b) **federation** (agentes cross-machine) como referencia para multi-PC (RFC 36 remote / RFC 63 sandbox lateral).
- **Evitar:** empaquetar todo en "swarm" sin sub-capacidades nombradas.

---

## DeerFlow (ByteDance) — harness super-agente — prioridad A — [Os]

- **URL (en vivo):** https://github.com/bytedance/deer-flow [Os]
- **Funcionalidades clave:** harness de "long-horizon SuperAgent" que investiga, programa y crea; **sub-agentes**, **skills**, **memoria**, **sandboxes**, **message gateway**, varios niveles de tarea (minutos a horas) [Os]. Interfaces múltiples: **terminal workbench `deerflow`** (Textual TUI + headless `--print`), **web UI** y **canales IM** [Os].
- **Layout y navegación:** **chat header** con **gauge de ventana de contexto** (cuando el modelo declara `context_window`), y **compaction manual** [Os]. Aislamiento: trabajo confinado a un **sandbox workspace por-thread** salvo host mounts declarados [Os].
- **Estados y feedback:** el gauge de contexto mantiene el porcentaje previo mientras recarga [Os]; **token budget/usage** (cada run del lead-agent incluye el usage de subagentes completados) [Os].
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** guías de configuración y arquitectura; `SubagentRuntime` con `max_running` y límites de run [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **gauge de contexto persistente en el header** (feedback de presupuesto de contexto siempre visible) — refuerza el "Calm Instrumentation" de RFC 66; (b) **token budget que agrega uso de subagentes**; (c) **aislamiento por thread** con host mounts explícitos; (d) TUI-textual + headless + web sobre el mismo motor.
- **Evitar:** [P].

---

## Symphony (OpenAI) — ADE / orquestador de issues — prioridad A — [Os]

- **URL (en vivo):** https://github.com/openai/symphony [Os]
- **Funcionalidades clave:** "**turns project work into isolated, autonomous implementation runs, allowing teams to manage work instead of supervising coding agents**"; monitoriza un **board de Linear** y lanza agentes para las tareas [Os].
- **Layout y navegación:** el trabajo vive como **board (Linear) → runs** aislados con **workspace por issue**; el ingeniero "manages the work at a higher level" en vez de supervisar Codex [Os]. Se distribuye como **spec + referencia Elixir + CLI escript** [Os].
- **Estados y feedback:** cada run entrega **"proof of work": CI status, PR review feedback, complexity analysis y walkthrough videos** [Os].
- **Aprobaciones / HITL:** **gate de aceptación** — sólo "when accepted, the agents land the PR safely" [Os].
- **Atajos de teclado:** [P] (no documentados en el README).
- **Onboarding / vacío / error:** aviso explícito "**low-key engineering preview for testing in trusted environments**" [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **board → runs aislados por issue** con workspace propio; (b) **proof-of-work obligatorio** (CI/PR/video) alineado con `EvidenceGate` (RFC 14 §10); (c) gate de **"accepted → land"** explícito (HITL claro).
- **Evitar:** [P].
- **Adoptar / Evitar para Atlas:** adoptar el par **board-de-trabajo + run aislado con evidencia**; evitar supervisión manual por-agente (Symphony demuestra que se puede gestionar trabajo, no procesos).

## Omnigent (Databricks) — meta-harness multi-agente — prioridad A — [Os]

- **URL (en vivo):** https://github.com/omnigent-ai/omnigent [Os]
- **Funcionalidades clave:** "open-source AI agent framework and **meta-harness**: **orchestrate Claude Code, Codex, Cursor, Pi, and custom agents — swap harnesses without rewriting, enforce policies and sandboxing, and collaborate in real time from any device**" [Os].
- **Layout y navegación:** **una capa de orquestación** sobre varios harnesses; agentes definidos en **YAML** (incl. un **tech-lead orchestrator** que delega a sub-agentes en **parallel git worktrees**); cada terminal de agente envuelto en **sandbox (bwrap/seatbelt/cloud)** [Os].
- **Estados y feedback:** **colaboración en tiempo real desde cualquier dispositivo** [Os].
- **Aprobaciones / HITL:** **enforce approval, spend, and tool policies** (políticas por agente) [Os].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** "swap harnesses without rewriting" (BYOA sin migración) [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **multi-harness/BYOA** con `tech-lead` que delega a sub-agentes en worktrees paralelos — exactamente el modelo de RFC 05/27; (b) **políticas (approval/spend/tool) por agente**; (c) sandbox por terminal.
- **Adoptar / Evitar para Atlas:** adoptar **meta-harness con políticas y sandbox por agente** y worktrees paralelos; evitar acoplar la UI a un solo harness.

## Agent Teams AI — control plane de equipos — prioridad A — [Os parcial]

- **URL (en vivo):** vía `awesome-cli-agents` (README agregador, fetch esta sesión) [Os].
- **Funcionalidades clave:** "cross-platform **desktop control plane** with integrated terminals and a **Kanban board** for autonomous coding-agent teams; agents coordinate, message each other, and review work across Codex/Claude Code/OpenCode/Cursor/Grok/Copilot/Kiro/Z.AI/MiniMax/Kimi and 75+ model providers" [Os].
- **Layout y navegación:** **Kanban + terminales integradas** [Os].
- **Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar el par **Kanban + terminales integradas** como raíz de un HUD de equipos (coincide con Mission rail + Kanban de RFC 65); evitar [P].

## Cate — canvas infinito de superficies — prioridad A — [Os]

- **URL (en vivo):** https://github.com/0-AI-UG/cate · cate.cero-ai.com [Os]
- **Funcionalidades clave:** "**An infinite zoomable canvas for coding. Editor, terminal, and browser panels in a spatial workspace**"; **agent chats** integrados (T3 Code corre Codex/Claude Code/Cursor/Grok/OpenCode/Antigravity con streaming, tool calls y approvals); git multi-repo + ripgrep + `Cmd+K` [Os].
- **Layout y navegación:** paneles "**on a canvas or in a dock**": float, **dock into tabs and splits**, o **detach a su propia ventana**; **el layout persiste por proyecto**; Monaco editors, viewers PDF/imagen/DOCX, canvases anidados [Os].
- **Estados y feedback:** **agent-aware terminals** reportan turn start/end y **permission prompts** → cada panel muestra **running / waiting / finished** y **te avisa cuando necesita una respuesta**; **las agent sessions sobreviven reinicios** (scrollback + resume) [Os].
- **Aprobaciones / HITL:** **permission prompts** por panel + approvals en el chat [Os].
- **Atajos de teclado:** **Cmd+K** para comandos/paneles/ficheros; atajos **rebindables en Settings** [Os].
- **Onboarding / vacío / error:** worktrees se crean desde el prompt (off de rama local/remota o PR abierto); **local y remoto son el mismo camino** (SSH/WSL) [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **canvas espacial con paneles dock/detach + layout persistente por proyecto** (modo avanzado de Canvas view, RFC 65); (b) **paneles que muestran running/waiting/finished y avisan** (feedback de estado por superficie); (c) **sesiones que sobreviven reinicios con scrollback**; (d) **Cmd+K** universal.
- **Adoptar / Evitar para Atlas:** adoptar **canvas + dock/detach + avisos de estado por panel**; evitar el runtime Electron (Atlas es Tauri).

## Orkas — orquestador desktop local — prioridad A — [Os parcial]

- **URL (en vivo):** `github.com/Orkas-AI/Orkas` (vía `awesome-cli-agents`) [Os].
- **Funcionalidades clave:** "Electron desktop app that **spawns and drives real local coding-agent sessions**" [Os].
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar "**sesión real por agente**" como unidad visual; evitar [P].

## Crystal — sesiones paralelas en worktrees — prioridad A — [Os parcial]

- **URL (en vivo):** `github.com/stravu/crystal` (vía `awesome-cli-agents`) [Os].
- **Funcionalidades clave:** "Execute **multiple Codex and Claude Code sessions in parallel git worktrees**" [Os].
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **worktree-por-agente** como modelo de paralelismo (ya en RFC 05/24 §12); evitar [P].

## agx — motor de ejecución por checkpoints — prioridad A — [Os parcial]

- **URL (en vivo):** `github.com/ramarlina/agx` (vía `awesome-cli-agents`) [Os].
- **Funcionalidades clave:** "**Checkpoint-based execution engine** for AI coding agents; durable **Wake→Work→Sleep loops that resume instantly across sessions**"; CLI + web dashboard + app macOS [Os].
- **Layout:** CLI + **web dashboard** + app macOS [Os].
- **Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **loops durables Wake→Work→Sleep** + dashboard web (RFC 19 checkpoints); evitar [P].


---

## Amp (Sourcegraph) — ADE / dev environment — prioridad A — [Os]

- **URL (en vivo):** https://ampcode.com/ (docs `docs.ampcode.com` → **DNS ENOTFOUND** en este entorno) [Os].
- **Funcionalidades clave:** "Coding agent and dev environment built for the frontier"; modelo de **orbs** — **cada agente tiene su propio orb** ("Run agents anywhere"); mensajes de producto: "Send prompt, close laptop / Continue from your phone / Continue while you sleep / **Forget worktrees**" [Os].
- **Layout y navegación:** **Review from Anywhere**: "review the agent's changes and browse the orb's files from any device, and edit like you're developing locally", con pestañas **Agent Changes / Portals / Files / Terminal** y acciones **Ship / Review / Sync**; ejemplo de diff con comentario de revisor ("Brett · just now · Weekly default is right. Ship it.") [Os].
- **Estados y feedback:** el par Ship/Review/Sync como ciclo de estado del cambio [Os].
- **Aprobaciones / HITL:** comentario de revisor inline en el diff + acción "Ship" (aprobación explícita) [Os]; gates formales **[P]**.
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** [P].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **orb-por-agente** como unidad visual (agente + su entorno), friendly para móvil — alternativa/complemento a la worktree card de RFC 24 §12; (b) **review remoto desde cualquier dispositivo** con pestañas **Agent Changes / Portals / Files / Terminal** — patrón directo para RFC 24 §16 (mobile/remote); (c) **comentario inline del revisor → Ship** (HITL ligero); (d) "forget worktrees" = abstraer el worktree de la UX (el usuario ve agentes/orbs, no ramas).
- **Evitar:** el marketing "continue while you sleep" sin explicitar el modelo de aprobación (Atlas debe hacer visible cuándo pide permiso).

## gastown — multi-agent workspace manager — prioridad A — [Os parcial]

- **URL (en vivo):** https://github.com/gastownhall/gastown (el RFC 62 lo cita como `steveyegge/gastown`) [Os]
- **Funcionalidades clave:** "Gas Town — **multi-agent workspace manager**"; multi-agent orchestration con **persistent work tracking** [Os vía `awesome-cli-agents`].
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P] (README indexado sin detalle de UI).
- **Adoptar / Evitar para Atlas:** adoptar **persistent work tracking** multicapa; evitar [P].

## OMK (open-multi-agent-kit) — CLI control plane — prioridad A — [Os parcial]

- **URL (en vivo):** https://github.com/dmae97/omk [Os]
- **Funcionalidades clave:** "Open-source coding agent CLI. **Switch models in one session** and **finish explicit goals against checks you approve**"; control plane provider-neutral: enruta runtimes, acota MCP, corre **DAG workers**, **verifica evidencia antes de completar** [Os].
- **Layout / Estados / Atajos / Onboarding / Accesibilidad:** [P].
- **Aprobaciones / HITL:** "checks you approve" (gates aprobados por el usuario) [Os].
- **Adoptar / Evitar para Atlas:** adoptar **DAG workers + verificación de evidencia antes de "done"** y **goals con checks aprobados** (RFC 14 §10, RFC 19); evitar [P].

## kodo — orquestador multi-agente overnight — prioridad A — [Os]

- **URL (en vivo):** https://github.com/ikamensh/kodo [Os]
- **Funcionalidades clave:** "Autonomous multi-agent coding that **runs overnight using your installed coding agents**. An orchestrator **delegates implementation and review across work cycles**, with **resumable logs and checkpoints**" [Os].
- **Layout y navegación:** orquestador que habla con roles nombrados (**architect**, **worker_smart**, **worker_fast**, **tester**, **tester_browser**) en un log con timestamps por turno (`orchestrator → architect`, etc.) [Os].
- **Estados y feedback:** log de **ciclos** con veredictos **✅ / ❌ REJECTED** y rondas de verificación ("7 more verification rounds") [Os].
- **Aprobaciones / HITL:** **gate de verificación** por architect/tester — *caveat citado:* "Verification is currently directed by the orchestrator … does **not enforce an independent verification gate**" [Os].
- **Atajos de teclado:** [P]. **Onboarding:** usa tus agentes CLI ya instalados [Os]. **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **ciclos con roles architect/worker/tester + checkpoints resumibles**; evitar confiar en un gate "auto-declarado" sin verificación independiente (Atlas debe forzar evidencia real).

## wreckit — Ralph Wiggum Loop sobre el roadmap — prioridad A — [Os]

- **URL (en vivo):** https://github.com/mikehostetler/wreckit [Os]
- **Funcionalidades clave:** "A CLI that runs a **Ralph Wiggum Loop** over your roadmap: `ideas → research → plan → implement → PR → done`"; es el workflow **Research → Plan → Implement** de HumanLayer, automatizado; "**Files are truth.** Everything lives in `.wreckit/` as JSON and Markdown. Git-trackable. Inspectable. **Resumable**" [Os].
- **Layout y navegación:** estado en ficheros `.wreckit/` (JSON+Markdown), git-trackable, sin base de datos [Os].
- **Estados y feedback:** el agente investiga → planifica → ejecuta **story-by-story** hasta dejar un **PR para tu review** [Os].
- **Aprobaciones / HITL:** "You review. Merge. Ship." (gate humano en el PR) [Os].
- **Atajos / Onboarding:** `npm i -g wreckit` → `wreckit init` → `wreckit ideas < IDEAS.md` → `wreckit` [Os]. **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **estado en ficheros (no DB) resumible** y el pipeline **ideas→research→plan→implement→PR** con review humano; evitar [P].

## OpenCastle — 19 agentes especialistas coordinados — prioridad A — [Os]

- **URL (en vivo):** https://github.com/monkilabs/opencastle · docs `opencastle.dev/docs/cli` [Os]
- **Funcionalidades clave:** "turns AI coding assistants (Copilot, Cursor, Claude Code, OpenCode, Windsurf, Codex, Antigravity) into **19 coordinated specialist agents**"; CLI con **task decomposition, parallel work, quality gates** [Os].
- **Layout y navegación:** CLI-driven con comandos `npx opencastle` (qué está instalado/drift/próximo), `sync`, `sync --check`, `add <x>`, `doctor`, `ci` [Os].
- **Estados y feedback:** **drift detection** (`sync --check` falla si algo derivó; pensado para CI) [Os].
- **Aprobaciones / HITL:** **quality gates** por fase [Os].
- **Onboarding:** "Commit the generated config, **like a lockfile**"; `npx opencastle ci` escribe un GitHub Actions que corre `sync --check` en cada PR [Os]. **Atajos:** [P]. **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **config generada versionada como lockfile + drift check en CI** y **quality gates**; evitar [P].

## 5dive — empresa de agentes self-hosted — prioridad A — [Os]

- **URL (en vivo):** `github.com/5dive-ai/5dive` (vía `awesome-cli-agents`, fetch [Os])
- **Funcionalidades clave:** "Run a **company of AI coding agents on a server you own**: one-command spin-up of named agents (Claude Code, Codex, Grok…), **cron + heartbeat scheduling**, multi-agent orchestration, Telegram control" [Os].
- **Layout y navegación:** **babysit + "needs-you" triage dashboard**; control por Telegram [Os].
- **Estados y feedback:** triage "needs-you" (qué requiere tu atención) [Os].
- **Aprobaciones / HITL:** el triage "needs-you" como cola de intervención [Os].
- **Atajos / Accesibilidad:** [P]. **Onboarding:** one-command spin-up, self-hosted, MIT [Os].
- **Adoptar / Evitar para Atlas:** adoptar **triage "needs-you" + scheduling cron/heartbeat** (RFC 19/24 §7); evitar [P].

## CliDeck — dashboard de agentes CLI — prioridad A — [Os]

- **URL (en vivo):** https://github.com/rustykuntz/clideck [Os]
- **Funcionalidades clave:** "A **dashboard for running and coordinating multiple AI CLI agents** at once"; descrito como "WhatsApp-like browser dashboard … **live status detection, session resume**" [Os].
- **Layout y navegación:** **browser dashboard** estilo WhatsApp (lista de sesiones/conversaciones) [Os]/[O].
- **Estados y feedback:** **live status detection** por sesión [Os vía agregador].
- **Aprobaciones / HITL:** [P] (la nota de v2 indica que **autopilot y control móvil fueron retirados**; los harnesses aportan su propio acceso remoto) [Os].
- **Atajos:** [P]. **Onboarding:** v2 importa sesiones/proyectos/prompts legacy; `UPGRADING.md` + `SESSION-BACKUP.md` [Os]. **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **status por sesión + resume** en un dashboard único; evitar duplicar acceso remoto si el harness ya lo ofrece.

## GridBash — terminal grid de agentes PTY — prioridad A — [Os]

- **URL (en vivo):** https://github.com/jasonsuhari/gridbash [Os]
- **Funcionalidades clave:** "Cross-platform **terminal grid** for running Codex, Claude, Gemini, and other **CLI agents side by side**"; **precise input routing** (focused pane / selected set / entire grid); hasta **100 PTY-backed panes**; **agent-first profiles** (Codex, Claude, Gemini, Aider, OpenCode, Goose, Amp, Cursor, Copilot, shells, custom); **BashBot Director** por grid [Os].
- **Layout y navegación:** grids por **filas × columnas** con nombre y carpeta de proyecto; **tabbed grids**; launch `gridbash 2x3 --profile codex`; `--layout auto` [Os].
- **Estados y feedback:** **inspect stable pane activity**, **restore sessions**, "optionally generate concise AI work summaries"; `--worktrees` aísla cada pane en **git worktree** [Os].
- **Aprobaciones / HITL:** [P]. **Atajos:** [P] (comandos CLI sí documentados).
- **Onboarding:** four-field launch con Tab completion (`gridbash`, `gridbash resume`, `--list-profiles`) [Os]. **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **grid de PTYs con input routing (pane/set/grid)** + **worktree por pane** + **resume de sesión**; evitar [P].

## tlbx — terminal browser multiplexer self-hosted — prioridad A — [Os]

- **URL (en vivo):** https://github.com/tlbx-ai/tlbx · tlbx.ai [Os]
- **Funcionalidades clave:** "**Self-hosted terminal browser multiplexer for persistent shells and coding agents** on Windows, macOS, and Linux"; runs Codex/Claude Code/Gemini CLI/Grok Build/OpenCode/Copilot CLI y cualquier PTY app **en las máquinas que tienen tus repos y credenciales** [Os].
- **Layout y navegación:** se supervisa **desde cualquier desktop/tablet/phone browser** [Os].
- **Estados y feedback:** "**Sessions survive disconnects**"; CLI `mt` expone history, multi-session dispatch y el control plane **como JSON** para que los agentes lo manejen [Os].
- **Aprobaciones / HITL:** [P]. **Atajos:** [P]. **Onboarding:** self-hosted [Os]. **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **sesiones que sobreviven desconexiones** + **control-plane como JSON que los propios agentes usan** (API=UI, Herdr); evitar [P].

## ADHDev — dashboard hub de agentes — prioridad A — [Os parcial]

- **URL (en vivo):** https://github.com/vilmire/adhdev · adhf.dev [Os]
- **Funcionalidades clave:** "**ADHDev — Agent Dashboard Hub. Monitor & control AI coding agents from a single dashboard. Self-hosted, open-source**" [Os].
- **Layout:** dashboard web self-hosted [Os]. **Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar el **dashboard self-hosted de monitor+control**; evitar [P].

## cmux — plataforma multi-agente en paralelo — prioridad A — [Os parcial]

- **URL (en vivo):** `github.com/manaflow-ai/cmux` (vía `awesome-cli-agents`) [Os]
- **Funcionalidades clave:** "Open-source platform for **running multiple coding agents in parallel**" [Os].
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar paralelismo de agentes open-source; evitar [P].

## AgentSwarms — orquestación swarm — prioridad A — [P]

- **Motivo [P]:** **búsqueda realizada** en `awesome-cli-coding-agents` [Os] → **sin entrada** para "AgentSwarms". Sin repo/doc localizada.

## Overbrilliant OB-1 — agente terminal con memoria de grafo — prioridad A — [Os parcial]

- **URL (en vivo):** `github.com/Overbrilliant/ob-1` (vía `awesome-cli-agents`) [Os]
- **Funcionalidades clave:** "**Terminal coding agent that runs with no account, API key, or card** against a free-model catalog. **Persistent project memory builds a fact and relationship graph** from real work"; npm `ob1`, Apache-2.0 [Os].
- **Atajos de teclado:** `/memory` (surface el grafo de memoria) [Os].
- **Layout:** terminal [Os]. **Estados / Aprobaciones / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **memoria como grafo de hechos/relaciones** (RFC 09/16); evitar [P].

## darce — CLI multi-modelo ultraligero — prioridad A — [Os]

- **URL (en vivo):** https://github.com/AmerSarhan/darce-cli [Os]
- **Funcionalidades clave:** "AI coding agent for your terminal. Reads, writes, edits code, runs commands. Any model. **14 kB**"; **7 tools** (Read, Write, Edit, Bash, Glob, Grep, WebFetch); **smart routing** (auto model por tarea); streaming; git-aware; **session resume**; context compaction; **cost tracking** [Os].
- **Layout y navegación:** TUI con **status bar** (token count + spend en tiempo real); **account dashboard** en `cli.darce.dev/dashboard` [Os].
- **Estados y feedback:** coste real-time en la status bar [Os].
- **Aprobaciones / HITL:** [P]. **Atajos:** **Ctrl+M** cambiar modelo, **Ctrl+C** cancelar/salir, **Up/Down** historial, **"""** multi-línea; slash `/help /model /clear /cost /compact /quit` [Os].
- **Onboarding:** `--resume` reanuda [Os]. **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **status bar con coste/tokens en vivo** y **slash commands + Ctrl+M** para cambio de modelo; evitar [P].

## Forge (Norvia Labs) — agente + editor + shell unificados — prioridad A — [Os]

- **URL (en vivo):** https://github.com/NorviaLabs/forge · forge.norvialabs.com [Os]
- **Funcionalidades clave:** "**unifies an AI agent, code editor, and shell** in one focused workflow"; "The agent is **part of the environment rather than the entire environment**: you can work directly in the editor and shell, supervise long-running agent work, switch between independent sessions, and **resume durable state after an interruption**" [Os].
- **Layout y navegación:** workspace de terminal con **agente + editor vim-style + shell** [Os].
- **Estados y feedback:** **durable SQLite session journal**; resume tras interrupción [Os].
- **Aprobaciones / HITL:** **approval-aware command execution** [Os].
- **Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **agente integrado en el entorno (no toda la UI)** + **journal durable SQLite** + approval-aware; evitar [P].

## CLAII — pair-programmer terminal multi-agente — prioridad A — [Os parcial]

- **URL (en vivo):** https://github.com/agencyswarm/CLAII [Os]
- **Funcionalidades clave:** "terminal-native AI pair-programmer with **multi-agent orchestration, MCP toolchains, and context-aware, memory-persistent refactors**"; patrón de orquestación **Planner → Implementer → Tester → Reviewer** (spawn de `run_agent` con roles + memoria compartida) [Os].
- **Layout:** CLI/TUI terminal-native [Os]. **Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar el pipeline **Planner→Implementer→Tester→Reviewer con memoria compartida**; evitar [P].

## Nausicaa — harness de Lanes — prioridad A — [Os]

- **URL (en vivo):** https://github.com/jackispm/nausicaa-harness [Os]
- **Funcionalidades clave:** "treats an Agent run as a **dynamic topology of Lanes** rather than one fixed linear loop. The model can decide when to **observe, fan out, delegate, or collaborate across Runs**; the **Host keeps identity, permissions, durability, and recovery as explicit facts**" [Os].
- **Layout y navegación:** **Lanes addressables** + **Teto observer lane** opcional [Os].
- **Estados y feedback:** **durable Run ledgers**, daemon/worker lifecycle, **A2A cross-Run** [Os].
- **Aprobaciones / HITL:** **permissions como hecho explícito del Host** [Os]. **Atajos / Onboarding (npm `nausicaa-harness`) / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **topología de Lanes (observe/fan-out/delegate)** y **permissions/durability como estado explícito** (RFC 19); evitar [P].

## Jazz — agent harness multi-superficie — prioridad A — [Os]

- **URL (en vivo):** https://github.com/lvndry/jazz · npm `jazz-ai` [Os]
- **Funcionalidades clave:** "**One agent. Every surface. Your rules.**"; defines **primary + companion models, persona, tools, permissions en un único JSON**; "Same agent, same tools, same memory" en todas las superficies [Os].
- **Layout y navegación (superficies):** **Terminal** (`jazz`), **scripts** (`jazz run --json`), **cron/launchd** (`jazz workflow schedule`), **GitHub PRs/Actions**, **Telegram**, **Discord**, **iMessage**, **WhatsApp**, **SSH server** (`/detach <host>`) [Os].
- **Estados y feedback:** "In a bot conversation **it asks you right there when a job needs approval**" [Os].
- **Aprobaciones / HITL:** **tools permission-gated que piden aprobación donde estés** [Os].
- **Atajos:** [P]. **Onboarding:** "Install it once and it runs everywhere" [Os]. **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **definición del agente en un JSON (modelos/persona/tools/permisos)** y **aprobación en el canal donde esté el humano** (móvil/chat) — refuerza RFC 24 §16; evitar [P].

## Smelt — TUI Rust con modos y permisos — prioridad A — [Os]

- **URL (en vivo):** https://github.com/leonardcser/smelt · docs `leonardcser.github.io/smelt` [Os]
- **Funcionalidades clave:** "A fast, **Lua-scriptable AI coding agent for the terminal**"; **subagentes en paralelo**; **sistema de permisos granular**; **vim keybindings**; modo headless scriptable; multi-provider [Os].
- **Layout y navegación:** TUI; plugins bundled (`which_key`, request inspector, LSP-backed semantic tools) desde `~/.config/smelt/init.lua` [Os].
- **Estados y feedback:** **modos: Normal → Plan → Apply → Yolo** (ciclo de modo por defecto) [Os].
- **Aprobaciones / HITL:** permisos granulares + modos (Plan/Apply/Yolo) como gradiente de autonomía [Os].
- **Atajos:** vim keybindings + plugin `which_key` [Os]. **Onboarding:** `smelt auth` / wizard; providers por suscripción o API key [Os]. **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **modos Normal/Plan/Apply/Yolo** y **permisos granulares** (gradiente de autonomía, RFC 21); evitar [P].

## Fetch.ai / FetchCoder — ADE / plataforma de agentes — prioridad A — [Os parcial]

- **URL (en vivo):** https://fetch.ai/ [Os]
- **Funcionalidades clave:** "Fetch.ai — Search and Discover"; superficies **ASI:One**, **Fetch Business**, **Agentverse**; mensaje "Your personal AI … Flights Booked. Dinner Reserved. Calendar Managed. Done your way, with your personal AI agent" [Os]. **FetchCoder** = su agente/CLI de coding **[P]** (no en la home).
- **Layout/Estados/Aprobaciones/Atajos/Onboarding/Accesibilidad:** [P].
- **Adoptar:** (a) **Agentverse** (marketplace/registro de agentes) como concepto para descubrimiento de capacidades; (b) personalización multi-dominio (calendario/viajes) como ejemplo de agente generalista conectado a servicios.
- **Evitar:** [P].

## Dots (OpenAI) — agente always-on — prioridad A — [P]

- **Motivo [P]:** **búsqueda realizada** en `awesome-cli-coding-agents` (fetch [Os], URL: `raw.githubusercontent.com/bradAGI/awesome-cli-coding-agents/main/README.md`) → **sin entrada**. Anuncio DevDay 2026 sin doc de producto; sin URL de UI verificable.

## Grok Bot (xAI) — agente always-on — prioridad A — [P]

- **Motivo [P]:** **búsqueda realizada** en `awesome-cli-coding-agents` [Os] → sólo aparecen **Grok CLI** (community) y **Grok Build** (xAI, TUI), **no "Grok Bot"**. Beta 2026 sin doc de producto localizada.

## Meta Muse · Google Gemini Spark · Instinct — agentes always-on — prioridad A — [P]

- **Motivo [P]:** **búsqueda realizada** en `awesome-cli-coding-agents` [Os] → **sin entrada**. Productos anunciados sin doc de UI pública.
- **Patrón:** agente persistente con **cloud computer propio** (RFC 63 §1).

## Cortex Code (Snowflake) — ADE/CLI corporativo — prioridad A — [P]

- **Motivo [P]:** reintentado `https://docs.snowflake.com/en/user-guide/snowsight-cortex-code` y `.../cortex-code` → **HTTP 404** (fetchs [Os]). URL canónica pendiente; sin doc de UI verificable.

## Tabnine CLI — ADE/CLI corporativo — prioridad A — [Os parcial]

- **URL (en vivo):** https://www.tabnine.com/ [Os]
- **Funcionalidades clave:** la página sirve hoy "**Agentic Quality Engineering Platform**" (Tricentis): "Ship AI-generated code w…"; navegación Company/Careers/News/Partners + Customer Portal [Os]. Detalle de **CLI** [P].
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar el ángulo **QE agéntico (calidad de código generado)** como vertical; evitar [P].

## Mentat CLI — ADE/CLI corporativo — prioridad A — [P]

- **Motivo [P]:** `https://github.com/AbanteAI/mentat` → **HTTP 404** (el repo no está en esa ruta; fetch [Os]). Requiere localizar el repo nuevo/archivado.

## Amazon Q Developer — ADE/CLI corporativo — prioridad A — [Os parcial]

- **URL (en vivo):** https://aws.amazon.com/q/developer/ · docs `docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/what-is.html` [Os]
- **Funcionalidades clave:** "**Coding Assistant — Amazon Q Developer**"; features + pricing + documentación [Os].
- **Onboarding:** guía "What is Amazon Q Developer?" con quick start [Os].
- **Layout / Estados / Aprobaciones / Atajos / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar el **CLI enterprise de AWS** como referencia del ángulo corporativo; evitar [P].

## Kiro CLI — ADE/CLI — prioridad A — [Os parcial]

- **URL (en vivo):** https://kiro.dev/docs/cli/ [Os]
- **Funcionalidades clave:** Kiro CLI replica el catálogo de features del IDE/Web (Specs, Steering, Hooks, MCP, Permissions, Sub-agents, Checkpoints, Compaction) [Os].
- **Onboarding:** Installation / Authentication / "Your first project" [Os].
- **Layout / Estados / Aprobaciones / Atajos / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar el **mismo catálogo de capacidades en IDE/CLI/Web**; ver ficha Kiro en `ide.md`; evitar [P].

## Cobertura [P] de esta categoría

Prioridad A sin ficha en vivo (ver `_pending-A.md`): **Manus**, **Amp (Sourcegraph)**, **MiniMax Agent**, **Abacus.AI**, **Qoder**, **Blackbox**, **FetchCoder**, **gitpod/Ona**, **Codex/Claude/Gemini (como ADE)**, **Dots**, **Grok Bot**, **Meta Muse/Gemini Spark**.

## Enlaces a referentes ya auditados (no se repiten)

- **Orca** (ADE: worktree-per-agent, status dot `permission>done>heuristic`, decision gates, Design Mode): `docs/design/CONSENSUS_AUDIT.md` §1.3 #2, §2.1, §2.4, §2.5.
- **Conductor** (orquestador): `CONSENSUS_AUDIT.md` §1.3 #14.
- **Vibe Kanban** (ADE+kanban, inline diff comments, dev server en contexto): `CONSENSUS_AUDIT.md` §1.3 #6, §2.4–2.6.
- **Herdr** (rollup de estado pane→tab→workspace, prefix/navigate modes, API=UI): `CONSENSUS_AUDIT.md` §1.3 #1, §2.1, §2.8.
- **Cursor** (ADE): `CONSENSUS_AUDIT.md` §1.3 #12 (repo-sourced [I]).
