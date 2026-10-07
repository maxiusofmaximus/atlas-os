# ux-catalog / IDE

Fichas de referentes prioridad A con **primario = IDE (o extensión/editor)**.
Formato y procedencia como en `terminal.md` ([Os]/[O]/[I]/[P]). Los ya auditados en `CONSENSUS_AUDIT.md` no se repiten (enlaces al final).

---

## Kiro (AWS) — IDE + CLI + Web — prioridad A — [Os]

- **URL (en vivo):** https://kiro.dev/docs/ [Os]
- **Funcionalidades clave (catálogo "I want to… → Use"):** **Specs** (requirements → design → tasks); **Bugfix Specs** (RCA + prevención de regresión); **Chat** (IDE) / sesión (CLI); **Steering** (estándares de proyecto automáticos); **Hooks** (automatizar acciones ante cambios de archivo, uso de tool o fin de tarea); **MCP**; **Permissions** (control de acceso del agente); **Custom agents**; **Skills** (paquetes de instrucciones reutilizables); **Powers** (tools con conocimiento que se activan on-demand); **Sub-agents** (delegación paralela); **Checkpoints and rewind** (deshacer cambios o forkear conversación); **Kiroignore** (mantener secretos fuera del agente); **Compaction** (sesiones largas dentro del límite de contexto) [Os].
- **Layout y navegación:** IDE con panel de chat/specs; la misma capacidad en CLI y Web [Os]. Navegación por el índice de docs por intención [Os]. Detalle de paneles exactos **[P]**.
- **Estados y feedback:** Hooks actúan en eventos (file change / tool use / task completion) — modelo de eventos explícito [Os]. Estados visuales concretos **[P]**.
- **Aprobaciones / HITL:** **Permissions** documentadas como control de acceso del agente; **Kiroignore** como frontera de secretos [Os]. Taxonomía de aprobaciones **[P]**.
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** "Set up the IDE, CLI, or Web in under 5 minutes" + "Your first project" hands-on (specs, steering, hooks) [Os]. Estados vacío/error **[P]**.
- **Accesibilidad:** [P].
- **Adoptar:** (a) **spec-driven**: Specs (req→design→tasks) + Bugfix Specs (RCA) como artefactos de primera clase — encaja con RFC 12/14/23; (b) **Hooks** por evento (file change/tool use/task completion) — mecanismo limpio para automatizaciones del HUD; (c) **Powers** (tools que se activan on-demand con conocimiento) ≈ skills/MCP de Atlas; (d) **Sub-agents** (delegación paralela) y **Compaction** documentadas como features de UX, no tras bambalinas; (e) **Kiroignore** como frontera de secretos visible.
- **Evitar:** la proliferación de conceptos solapados (Custom agents + Skills + Powers + Sub-agents) sin una jerarquía visual clara; Atlas debe nombrar y agrupar para no confundir.

---

## Cline / Roo — IDE + terminal — prioridad A — [Os]

- **URL (en vivo):** https://docs.cline.bot/ · /features/plan-and-act · /features/checkpoints [Os]
- **Funcionalidades clave:** agente que vive en el editor y el terminal; **Cline Desktop App** con **parallel sessions + scheduled tasks** y elección de modelo [Os]; **Cline CLI** [Os].
- **Layout y navegación:** panel de chat lateral del editor; modos conmutable en el mismo hilo [Os].
- **Estados y feedback:** **Plan mode** (puede leer/buscar/discutir, **no** modifica archivos ni ejecuta comandos) vs **Act mode** (modifica/ejecuta); al cambiar de modo **el historial de conversación se conserva** [Os]. Checkpoints con **Compare** entre snapshots [Os].
- **Aprobaciones / HITL:** **Auto-approve** configurable por tipo (edits/commands); combinado con checkpoints como red de seguridad: "enable auto-approve → let it work → review final → restore si falla" [Os].
- **Atajos de teclado:** [P] (modo Plan/Act es un toggle; tecla no documentada en páginas indexadas).
- **Onboarding / vacío / error:** "Cline (usage-billing): fastest setup path with one sign-in, built-in billing, free model options" [Os]. Restore opciones: **Restore Files**, **Restore Files & Task**, **Restore Task Only** (keep files) [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **Plan↔Act** como par de modos con contexto preservado (mapea a RFC 21/23 `architect/code`); (b) **checkpoints en shadow-git** (restaurar código sin perder conversación) — patrón fuerte para RFC 19; (c) **auto-approve + checkpoints** como binomio (autonomía con red de seguridad) — directamente aplicable al APPROVALS+Demos de RFC 24/66; (d) las 3 granularidades de restore (files / task / both).
- **Evitar:** auto-approve sin snapshots (Atlas debe garantizar el checkpoint antes de permitir autonomía).

---

## Continue.dev — IDE (extensión) + CLI — prioridad A — [Os parcial]

- **URL (en vivo):** https://raw.githubusercontent.com/continuedev/continue/main/README.md (docs.continue.dev redirige por JS, 0 KB servido) [Os]
- **Funcionalidades clave:** "Continue is a coding agent available as a **CLI**, **VS Code extension**, and **JetBrains plugin**"; "**Pioneering open-source coding agent**" (Apache-2.0) [Os]. Los modos (**Agent / Chat / Edit / Autocomplete**) están documentados en `docs.continue.dev` **[P]** (la URL de docs es JS-redirect y no sirvió contenido).
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar:** separación explícita **CLI + VS Code + JetBrains** sobre un mismo motor; el resto **[P]**.
- **Nota de honestidad:** README raw abierto (2 campos); sigue **[Os parcial]** (<4). Falta la doc real (`docs.continue.dev/...`, servible vía el `docs/` del repo en GitHub).

---

## opencode — extensión de IDE — prioridad A — [Os]

- **URL (en vivo):** https://opencode.ai/docs/ide/ [Os]
- **Funcionalidades clave:** "The OpenCode extension for VS Code, Cursor, and other IDEs"; "OpenCode integrates with VS Code, Cursor, or any IDE that supports a terminal. Just run `opencode` in the terminal to get started" [Os].
- **Layout y navegación:** "**Quick Launch**: use `Cmd+Esc` (Mac) or `Ctrl+Esc` (Windows/Linux) to open OpenCode in a **split terminal view**, or focus an existing terminal session" [Os].
- **Estados y feedback:** [P] (la página de IDE no los detalla).
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** `Cmd+Esc`/`Ctrl+Esc` (Quick Launch, split terminal); `Cmd+Shift+Esc`/`Ctrl+Shift+Esc` (New Session); `Cmd+Option+K`/`Alt+Ctrl+K` (insertar referencia de fichero, p.ej. `@File#L37-42`); **Context Awareness** comparte la selección/tab actual [Os].
- **Onboarding / vacío / error:** "Just run `opencode` in the terminal to get started" (cero config de extensión) [Os].
- **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** **Adoptar** (a) la extensión de IDE como **superficie secundaria sobre el mismo motor** (split terminal), no como producto aparte; (b) **Context Awareness** (selección/tab → contexto) y el atajo de **referencia de fichero con rango de líneas** (`@File#L37-42`) — útil para el Input Enhancement de RFC 23. **Evitar** depender de la extensión como superficie principal (Atlas es desktop/CLI-first).
- **Nota:** la ficha principal de opencode está en `terminal.md`.

---

## Windsurf / Devin Desktop (Cognition) — IDE — prioridad A — [Os]

- **URL (en vivo):** https://docs.windsurf.com/windsurf/cascade/cascade (y `/windsurf/getting-started`) [Os]. La marca "Windsurf" se integró en **Devin Desktop** (Cognition): el paquete se llama `devin-desktop` y `windsurf` queda como transitorio [Os].
- **Funcionalidades clave:** "Devin Desktop is a next-generation AI IDE built to keep you in the flow"; **Cascade** = "one of two local agents in Devin Desktop; the other is the Devin Local agent" [Os]. Tools de Cascade: Search, Analyze, Web Search, MCP y terminal [Os]; "Cascade can automatically fix linting errors on generated code" [Os]; **Memories**, **Voice input**, **MCP** [Os].
- **Layout y navegación:** Cascade se abre con `Cmd/Ctrl+L` o el icono arriba a la derecha; el texto seleccionado en editor/terminal se incluye solo [Os]. Starting page centrada en el input del agente; **Recent sessions** ⇄ **Recent projects**; **Go to sessions list** → **Agent Command Center** [Os]. Settings en la barra inferior (tabs **Plan Info / Settings / AI Shortcuts**) [Os].
- **Estados y feedback:** "Cascade will create a **Todo list** within the conversation to track progress on complex tasks"; "a specialized planning agent continuously refines the long-term plan while your selected model focuses on short-term actions" [Os]; **Queued Messages** (encolar mientras trabaja) [Os]; **Named Checkpoints and Reverts** [Os]; en Chat mode propone código para aceptar/insertar [Os].
- **Aprobaciones / HITL:** límites con confirmación ("... limit and prompts you to continue") [Os]; **Restricted Mode** deshabilita los agentes mientras el workspace está en ese modo [Os]; revert a checkpoint [Os].
- **Atajos de teclado:** `Cmd/Ctrl+L` (abrir Cascade), `⌘⇧P`/`Ctrl+Shift+P` (Command Palette, buscar "Devin") [Os].
- **Onboarding / vacío / error:** starting page sin sesiones sugiere prompts ("Explore my codebase and diagram how it works"); **Open project / Clone repository / Connect via SSH**; importar config de VS Code/Cursor [Os]; **Explain and Fix** para errores del editor y **Send to agent** desde el panel Problems [Os].
- **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** **Adoptar** (a) **Todo list + planning agent** en la conversación (progreso visible de tareas largas) — patrón directo del Kanban/Activity de RFC 65; (b) **Queued Messages** (steer diferido sin perder trabajo) y **Named Checkpoints + Revert** (RFC 19); (c) **Agent Command Center** como vista de todas las sesiones (Mission rail); (d) `.devinignore`/`.codeiumignore` como frontera de contexto/secretos. **Evitar** la ambigüedad "Cascade vs Devin Local" (dos agentes locales sin jerarquía clara).
- **Nota:** cubre las dos entradas A (**Capa 1 `Windsurf`** y **1c:B `Windsurf (Cognition)`**).

## Cloud dev environments — prioridad A — [Os parcial]

Referentes 1c:G (entornos de desarrollo en la nube). Desdoblado en **una ficha por producto**.

### Coder — cloud dev env — prioridad A — [Os parcial]
- **URL (en vivo):** https://coder.com/docs/user-guides/workspace-access [Os]
- **Funcionalidades clave:** "AI development infrastructure for customizable workspaces, enabling builders **and their AI coding agents to work side by side** in secure, consistent environments" [Os]; el **Web Terminal** "uses xterm.js and WebSocket technology... persistent sessions, Unicode support, and clickable URLs" [Os].
- **Layout y navegación:** "You can see the primary methods of connecting to your workspace in the **workspace dashboard**"; conexión por **SSH**, **Web Terminal**, **RDP** y el IDE de Google **Antigravity** (extensión Coder) [Os].
- **Atajos de teclado:** el Web Terminal documenta "keyboard shortcuts" (página propia) [Os, referenciado].
- **Estados / Aprobaciones / Onboarding / Accesibilidad:** [P].
- **Adoptar:** workspaces donde humano **y** agente trabajan lado a lado + **Web Terminal en el navegador** (útil para el remote HUD, RFC 24 §16). **Nota:** 3 campos → **[Os parcial]**.

### GitHub Codespaces — cloud dev env — prioridad A — [Os parcial]
- **URL (en vivo):** https://docs.github.com/en/codespaces [Os]
- **Funcionalidades clave:** "create a code space to start developing in a secure, configurable, and **dedicated development environment** that works how and where you want" [Os].
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar:** entorno **dedicado y configurable por misión** (shape = worktree/container por agente). **Nota:** 1 campo → **[Os parcial]**.

### DevPod — cloud dev env (devcontainers) — prioridad A — [Os parcial]
- **URL (en vivo):** https://devpod.sh/docs [Os]
- **Funcionalidades clave:** "DevContainers everywhere": crea entornos reproducibles, cada uno en su **contenedor separado** especificado por `devcontainer.json`; providers gestionables [Os].
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar:** **reproducibilidad declarativa** (devcontainer.json) + provider abstracto — patrón para sandbox/worktrees. **Nota:** 1-2 campos → **[Os parcial]**.

### Gitpod / Ona — cloud dev env (background agents) — prioridad A — [Os parcial]
- **URL (en vivo):** https://www.gitpod.io/docs (→ Ona Documentation) [Os]
- **Funcionalidades clave:** "**Ona is the platform for background agents**. Run a team of AI software engineers in the cloud, orchestrated, governed, and secured at the kernel"; "Start a single interactive session, or **run fleets of agents in the background on a schedule, on pull request events, or from your issue tracker**" [Os].
- **Layout y navegación:** home screen con "a **new session prompt**, **left navigation**, and quick actions for common coding tasks" [Os].
- **Onboarding:** "Get started in less than 5 minutes on Ona Cloud, or run Ona in your own VPC on AWS or GCP"; importar config de Claude Code/Cursor + `AGENTS.md` + skills [Os].
- **Estados / Aprobaciones / Atajos / Accesibilidad:** [P].
- **Adoptar:** **fleets de agentes en background por triggers** (schedule / PR / issue) — paralelo al Swarm (RFC 05) y a los triggers del Supervisor; `AGENTS.md` + skills como config portable. **Nota:** 3-4 campos → **[Os parcial]** (subiría con `/docs/ona/...`).

### code-server / openvscode-server — cloud dev env (IDE en navegador) — prioridad A — [P]
- **Motivo [P]:** no abierta esta sesión; URL candidata `coder.com/docs/code-server` (VS Code en navegador). Patrón: IDE en el navegador para el remote HUD.

### Bunnyshell — cloud dev env — prioridad A — [P]
- **Motivo [P]:** no abierta esta sesión; URL candidata `documentation.bunnyshell.com`.

## Qoder (Alibaba / Bright Zenith) — IDE (plataforma agéntica) — prioridad A — [Os]

- **URL (en vivo):** https://docs.qoder.com/quick-start (y `/`) [Os]
- **Funcionalidades clave:** "Qoder is an **agentic platform for real work**... an end-to-end loop: understand the task and its context, plan the work, use tools to execute it, verify the result, and iterate" [Os]. Superficies: **NEXT** (next-edit suggestions), **Inline Chat**, **Ask / Agent** en el Chat panel, y **Quest** ("for long-running, multi-step delegation") [Os].
- **Layout y navegación:** dos modos: **Editor** ("for in-flow assistance") y **Quest** ("long-running, multi-step delegation" en una ventana dedicada con "board, status, and deliverables in one workspace") [Os]. **Mobile & Web**: "Monitor supported IDE and CLI tasks, review plans, and handle approvals away from your computer" [Os].
- **Estados y feedback:** Quest muestra "board, status, and deliverables in one workspace"; el loop incluye verificación e iteración [Os]. Estados visuales concretos **[P]**.
- **Aprobaciones / HITL:** "carry out multi-step work independently while **retaining necessary review points**"; en el Chat, "Use **Run** or confirm actions as the UI suggests" [Os]; aprobaciones remotas vía Mobile & Web [Os].
- **Atajos de teclado:** `⌘I`/`Ctrl+I` (Inline Chat), `⌘L`/`Ctrl+L` (Chat), `⌘⏎`/`Ctrl+Enter` (aplicar), `⌘O`/`Ctrl+O` (abrir proyecto), `⌘⇧,`/`Ctrl+Shift+,` (cuenta) [Os].
- **Onboarding / vacío / error:** flujo 1-2-3-4 (descargar → sign in → **Open / Clone project** → explorar NEXT/Inline Chat/Ask-Agent/Quest) [Os].
- **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** **Adoptar** (a) la separación **Editor (in-flow) ↔ Quest (delegación larga con board/status/deliverables)** — mapea a los modos code↔architect (RFC 21/23) y al Kanban de RFC 65; (b) **review points retenidos** en trabajo autónomo (HITL explícito); (c) **aprobaciones y monitorización remotas** (Mobile & Web) — alinea con RFC 24 §16. **Evitar** la proliferación de superficies (NEXT/Inline/Chat/Quest) sin jerarquía visual.
- **Nota:** cubre también **1c:C `Qoder + Tongyi Lingma`** (mismo producto/ecosistema).

## Blackbox AI — IDE — prioridad A — [P]

- **Motivo [P]:** `https://docs.blackbox.ai/` devuelve una **página de redirección SPA** ("Redirecting to the docs → /api-reference/chat") sin contenido servido (0.1 KB, fetch [Os]). Requiere abrir `/api-reference/chat` o la app en una próxima pasada.
- **Adoptar:** [P].

## GitHub Copilot / Project HydraFusion — IDE (asistente) — prioridad A — [Os]

- **URL (en vivo):** https://docs.github.com/en/copilot/concepts/agents/coding-agent/about-coding-agent [Os]
- **Funcionalidades clave:** "Explore your repositories, plan changes, and **delegate coding tasks to GitHub Copilot** without leaving GitHub.com" [Os]; el **cloud agent** "can research a repository, plan changes, and implement them in the background... in an **ephemeral cloud development environment**" [Os].
- **Layout y navegación:** "You can start work from the **agents panel, a conversation, or an issue or pull request** on GitHub.com"; **session logs** muestran "the work and tools used" [Os].
- **Estados y feedback:** "While the session runs, you can continue chatting with Copilot about its **progress** and **steer** the work"; "Copilot Chat can also answer questions about pull requests created by Copilot by pulling in the relevant **agent session logs** — you can ask what changed, what was validated, and why" [Os].
- **Aprobaciones / HITL:** "You can **review changes and request refinements before creating a pull request**, or request a pull request in your initial prompt"; **code review** "reviews pull request changes, identifies potential issues, and suggests fixes... review and apply the suggested changes"; "Logs **do not replace your own review and testing**" [Os].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** [P] (quickstart cubierto en 1e).
- **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** **Adoptar** (a) **delegar desde el panel/issue/PR** (la tarea nace donde ya está el trabajo) — paralelo al Mission rail; (b) **session logs que responden "what changed / what was validated / why"** — patrón para el Audit/Timeline de RFC 65; (c) **review-before-PR + code review asistido** como gate HITL (RFC 14). **Evitar** delegar al cloud sin sandbox local (Atlas mantiene sandbox propio, RFC 18).
- **Nota:** cubre **Capa 1 `GitHub Copilot Project HydraFusion`** y **1e `GitHub HydraFusion`**; el detalle "HydraFusion" no aparece en la doc indexada (línea de producto del ecosistema, no doc propia).

## Pi / Pi Agent IDE — IDE + harness — prioridad A — [P]

- **Motivo [P]:** **sin URL de producto canónica localizada** esta sesión. Evidencia parcial [Os] vía `awesome-cli-agents`: aparece como runtime que Orca/Omnigent orquestan ("Codex, Claude Code, OpenCode, and **Pi**", y Omnigent incluye **Pi**). Requiere identificar el repositorio/producto (¿`pi` harness?) en una próxima pasada.
- **Adoptar:** [P].

## Cobertura [P] de esta categoría

Estado tras la pasada 4 (este fichero):
- **[Os] real (URL + ≥4 campos):** Windsurf/Devin Desktop, opencode IDE, Qoder, GitHub Copilot.
- **[Os parcial]:** Continue.dev (README raw), Coder, GitHub Codespaces, DevPod, Gitpod/Ona.
- **[P] con motivo:** Blackbox (SPA redirect), Pi (sin URL canónica), code-server/openvscode-server y Bunnyshell (no abiertas).
- Fuera de alcance de esta pasada: **CodeGeeX / CodeBuddy / Comate** (1c:C/B), **Kiro (layout exacto)**.

## Enlaces a referentes ya auditados (no se repiten)

- **Zed** (IDE, Agentic layout, collapse-on-completion `Worked for Nm`): `docs/design/CONSENSUS_AUDIT.md` §1.3 #4, §2.6.
- **VS Code** (Activity Bar, Secondary Side Bar/Chat, Command Palette con modos): `CONSENSUS_AUDIT.md` §1.3 #8, §2.2, §2.8.
- **JetBrains / IntelliJ** (New UI, "reduce visual complexity + progressive disclosure", Compact mode): `CONSENSUS_AUDIT.md` §1.3 #9, D-66-03.
- **Cursor** (ADE/IDE): `CONSENSUS_AUDIT.md` §1.3 #12 (repo-sourced [I]).
