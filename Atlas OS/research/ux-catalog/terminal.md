# ux-catalog / terminal

Fichas de referentes prioridad A con **primario = terminal/TUI** (categoría UX `terminal`).
Formato: funcionalidades clave · layout y navegación · estados y feedback · aprobaciones/HITL · atajos · onboarding/vacío/error · accesibilidad · adoptar/evitar Atlas · URLs.
Procedencia por afirmación: **[Os]** verificado en vivo esta sesión (URL citada) · **[O]** repo · **[I]** inferido · **[P]** pendiente.
Los referentes de terminal ya auditados en `docs/design/CONSENSUS_AUDIT.md` **no se repiten aquí** (ver enlaces al final).

---

## opencode — terminal (TUI) — prioridad A — [Os]

- **URL (en vivo):** https://opencode.ai/docs/ · https://opencode.ai/docs/tui/ [Os]
- **Funcionalidades clave:** agente de coding open-source disponible como **TUI de terminal, desktop app o extensión de IDE**; inicializa el proyecto con `/init` creando `AGENTS.md` en la raíz [Os].
- **Layout y navegación:** interfaz de terminal interactiva para el directorio actual; se lanza con `opencode` [Os]. Detalle de paneles no expuesto en la página raíz **[P]**.
- **Estados y feedback:** no documentado en la página raíz **[P]**.
- **Aprobaciones / HITL:** no documentado en la página raíz **[P]**.
- **Atajos de teclado:** comando por `/` + nombre (p. ej. `/help`); **leader key `ctrl+x`** como prefijo por defecto para la mayoría de atajos; catálogo en `/docs/keybinds` [Os].
- **Onboarding / vacío / error:** install por Chocolatey/Scoop/npm/Mise/Docker; en Windows recomienda WSL; primer paso `/init` → `AGENTS.md` (recomienda commitearlo) [Os].
- **Accesibilidad:** [P].
- **Adoptar:** leader-key `ctrl+x` + `/` slash commands como doble vía de descubrimiento; `AGENTS.md` como artefacto de init trazable; multi-superficie (TUI/desktop/IDE) sobre el mismo motor — coincide con el modelo de frontends de RFC 04 §9.
- **Evitar:** depender de WSL como requisito de experiencia óptima en Windows (Atlas es binario único Windows-first).

---

## Warp — terminal (terminal + agentes) — prioridad A — [Os]

- **URL (en vivo):** https://docs.warp.dev/ · https://docs.warp.dev/agents/agents-overview [Os]
- **Funcionalidades clave:** terminal de escritorio con **Blocks** (editor moderno de comandos), sección **Code** (revisar cambios del agente y editar archivos junto al terminal) y **Agents** (arrancar y dirigir agentes) [Os]. El mismo agente corre en la app, el **CLI** y la nube ("context follows it everywhere": Rules, Skills, MCP, Codebase Context) [Os].
- **Layout y navegación:** terminal + panel de agentes; **Agent Management Panel** para seguir varios agentes en paralelo; **orchestration** donde un agente padre lanza y coordina hijos [Os]. `Code` presenta el diff a revisar junto al terminal [Os].
- **Estados y feedback:** "interactive or delegated, and you can switch mid-task"; handoff a agente cloud y reanudar en local [Os].
- **Aprobaciones / HITL:** **"You approve before anything lands"** — el agente pide permiso antes de ejecutar comandos, editar archivos o llamar MCPs; nivel de autonomía por **Agent Profiles and permissions**; revisión en **interactive code review** [Os].
- **Atajos de teclado:** no hallados en la página indexada **[P]** (buscar en `/terminal/editor/`).
- **Onboarding / vacío / error:** [P].
- **Accesibilidad:** [P].
- **Adoptar:** (a) el mismo agente **local ↔ cloud con handoff** y switch mid-task; (b) **Agent Profiles/permissions** como control de autonomía por superficie (mapea a RFC 21 Modes); (c) aprovechar el terminal como primera clase ("works in a real terminal", REPLs, TUIs) — validación del patrón sister/TUI.
- **Evitar:** acoplar la experiencia a una sola superficie; Atlas debe mantener TUI y HUD equivalentes.

---

## OpenHands — terminal + ADE (Agent Canvas) — prioridad A — [Os]

- **URL (en vivo):** https://docs.all-hands.dev/ [Os]
- **Funcionalidades clave:** tres superficies: **Agent Canvas** (cliente de navegador + centro de control de conversaciones y automatizaciones), **Software Agent SDK** (Python) y **Agent Server** (REST/WebSocket), más un **CLI** basado en el SDK [Os].
- **Layout y navegación:** Agent Canvas se conecta a uno o más agentes-servidor ("connects to one or more Agent Server backends"); el launcher `agent-canvas` arranca Canvas + backends locales como stack todo-en-uno, o el cliente se conecta a un backend local/self-hosted/Cloud/Enterprise [Os].
- **Estados y feedback:** no en la página de intro **[P]**.
- **Aprobaciones / HITL:** no en la intro **[P]** (el hallazgo "ex-OpenDevin con CLI y web" es de `awesome-cli-agents` [Os]).
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** multi-modo de arranque (todo-en-uno vs cliente+backend) — patrón de onboarding por elección de topología [Os].
- **Accesibilidad:** [P].
- **Adoptar:** separar **cliente (UI) ↔ server (ejecución)** por REST/WS con un "canvas" que puede apuntar a varios backends — análogo directo al HUD axum sobre Kernel Bus de Atlas; útil si Atlas quiere multi-daemon o remote.
- **Evitar:** fragmentar la experiencia en 4 superficies históricas (legacy GUI deprecado) — Atlas debe tener una sola UI canónica (HUD) + TUI.

---

## Cline — terminal + editor (IDE) — prioridad A — [Os parcial]

> Primario mixto: vive en **editor y terminal**; su UI de modos y checkpoints es el valor para HUD. Ficha completa en `ide.md` (sección Cline). Aquí solo el ángulo terminal.

- **URL (en vivo):** https://docs.cline.bot/ [Os]
- **Terminal:** "an AI coding agent that lives in your editor **and your terminal**"; existe **Cline CLI** y una **Cline Desktop App** con **parallel sessions + scheduled tasks** y elección de modelo abiertos [Os].
- **Adoptar:** mismo agente en editor y terminal (paridad de superficies) y sesiones paralelas programables.

---

## Kiro — terminal (CLI) + IDE — prioridad A — [Os parcial]

> Primario mixto; ficha principal en `ide.md` (sección Kiro). Ángulo terminal:

- **URL (en vivo):** https://kiro.dev/docs/ [Os]
- **Terminal:** Kiro se ofrece como **IDE, CLI y Web** ("Set up the IDE, CLI, or Web in under 5 minutes"); la tabla "What you can do" aplica a las tres [Os]. Incluye **ACP integrations** para usar Kiro "en otro editor o cliente" [Os].
- **Adoptar:** un catálogo de capacidades ("I want to… → Use…") como mapa de descubrimiento, ofrecido igual en IDE/CLI/Web.

---

## Claude Code — terminal (CLI) + IDE — prioridad A — [Os parcial]

- **URL (en vivo):** https://docs.claude.com/en/docs/claude-code/overview [Os]
- **Funcionalidades clave:** asistente de coding AI que construye features, arregla bugs y automatiza tareas de desarrollo [Os]. Detalle de capacidades/UI **[P]** (la página de overview indexada es una intro; el índice completo está en `/docs/llms.txt` [Os]).
- **Layout y navegación / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** **[P]** (no expuestos en la página raíz indexada esta sesión).
- **Adoptar (a validar):** el modelo "asistente en terminal con helpers de edición" — Atlas ya lo cubre; **[P]** hasta abrir `/cli-reference`, `/interactive-mode`, `/iam`.
- **Evitar:** [P].

## Codex CLI (OpenAI) — terminal (TUI) — prioridad A — [Os]

- **URL (en vivo):** https://developers.openai.com/codex/cli/ [Os]
- **Funcionalidades clave:** inspecciona/edita/ejecuta código y automatiza trabajo repetible sin salir del terminal; **banner TUI** con `model:` y `directory:`, línea de contexto ("100% context left"), y **slash commands**: `/init` (crea `AGENTS.md`), `/status` (config de sesión), `/permissions` (qué puede hacer), `/model` (modelo + reasoning effort), `/review` (revisa cambios y encuentra issues) [Os].
- **Layout y navegación:** TUI de una pantalla con prompt y barra de estado; referencia de CLI en `/codex/developer-commands?surface=cli` [Os]. Paneles múltiples **[P]**.
- **Estados y feedback:** **indicador de contexto restante** ("100% context left") + ayuda de atajos (`? for shortcuts`) [Os].
- **Aprobaciones / HITL:** **Permissions** documentadas: **Profiles**, **Sandboxing**, **Auto-review**, **Agent approvals & security** [Os] — modelo de aprobación por perfiles + sandbox + auto-review.
- **Atajos de teclado:** `?` abre la ayuda de atajos [Os]; combinaciones concretas **[P]**.
- **Onboarding / vacío / error:** `/init` como primer paso (crea `AGENTS.md`) [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **contador de contexto restante** visible en la barra de estado (feedback de presupuesto, alinea con DeerFlow y RFC 66 "Calm Instrumentation"); (b) **Permissions = Profiles + Sandboxing + Auto-review** como modelo de tres capas (control de autonomía) — encaja con RFC 18/21; (c) `/review` como acción de primera clase.
- **Evitar:** [P].

## Gemini CLI (Google) — terminal (CLI) — prioridad A — [Os]

- **URL (en vivo):** https://github.com/google-gemini/gemini-cli (docs: https://www.geminicli.com/docs/) [Os]
- **Funcionalidades clave:** agente AI open-source en la terminal; **built-in tools**: operaciones de fichero, comandos shell, web fetch & search; **integración de MCP servers** para extender con tools propias; **Custom Extensions** (comandos propios compartibles) [Os].
- **Layout y navegación:** TUI de terminal (README raíz) [Os]; **Keyboard Shortcuts** documentadas en `/reference/keyboard-shortcuts` [Os].
- **Estados y feedback:** [P].
- **Aprobaciones / HITL:** [P] (config en `/reference/configuration`).
- **Atajos de teclado:** doc dedicada "Keyboard Shortcuts — Productivity tips" [Os]; combinaciones concretas **[P]**.
- **Onboarding / vacío / error:** Quickstart + Authentication Setup (config de auth detallada) [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **extensions** como unidad compartible de comandos/tools (paralelo a skills de Atlas); (b) separar **built-in tools vs MCP vs custom extensions** en la doc/UX.
- **Evitar:** [P].

## Aider — terminal (pair programming) — prioridad A — [Os]

- **URL (en vivo):** https://aider.chat/docs/ [Os]
- **Funcionalidades clave:** "AI pair programming in your terminal"; edita código en tu repo git local [Os]. **Chat modes**: `code`, `architect`, `ask`, `help` [Os]. **In-chat commands** (`/add`, `/model`, etc.) [Os]. Soporta **voice-to-code**, **images & web pages**, **prompt caching**, y **"Aider in your IDE"** (watch de archivos) [Os].
- **Layout y navegación:** REPL de chat en terminal con comandos `/` [Os].
- **Estados y feedback:** [P] (más allá de las salidas de chat).
- **Aprobaciones / HITL:** modo **architect** (planifica, luego edita) vs **ask** (solo responde) como control de intervención [Os].
- **Atajos de teclado:** [P] (comandos in-chat en `/usage/commands`).
- **Onboarding / vacío / error:** install + docker + Codespaces + Replit paths; "Tips" doc [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **modos de chat nombrados** (code/architect/ask/help) — mapea a RFC 21/23; (b) comandos in-chat `/` como control sin salir del REPL; (c) "watch your files" (aider responde a cambios de archivo) como integración IDE ligera.
- **Evitar:** [P].

## gptme — terminal + web + desktop — prioridad A — [Os parcial]

- **URL (en vivo):** https://gptme.org/docs/ [Os]
- **Funcionalidades clave:** asistente/plataforma de agentes en terminal y navegador con tools potentes (ejecuta python/bash, edita ficheros, busca/navega web) [Os]. Componentes: **gptme CLI**, **gptme-server**, **gptme-webui**, **gptme-agent-template** [Os]. Interfaces: **CLI, TUI, desktop+Android, cloud, editor (ACP), canales** (GitHub, email, chat, voz) [Os]. Extensible con **Lessons/Skills**, **Memory**, **Agents persistentes**, **plugins/custom tools/Hooks/MCP** [Os].
- **Layout y navegación:** CLI/TUI + web UI (servida por el server con REST API) [Os].
- **Estados y feedback:** [P].
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** "Where to start": elegir modelo→provider→config→interfaz [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **paridad de interfaces** (CLI/TUI/web/desktop/editor ACP/canales) sobre el mismo motor — modelo idéntico al de Atlas (frontends de RFC 04 §9); (b) **Lessons + Skills + Memory** como taxonomía de contexto/aprendizaje (mapea a RFC 09/11/16); (c) **canales** (GitHub/email/chat/voz) como superficies de entrada — refuerza RFC 24 §16.
- **Evitar:** [P].

## Devon (entropy-research) — terminal (pair programmer) — prioridad A — [Os parcial]

- **URL (en vivo):** https://github.com/entropy-research/Devon [Os]
- **Funcionalidades clave:** "Devon: An open-source pair programmer" [Os]. Detalle de features/UI **[P]** (el README indexado no expone layout/estados).
- **Layout/Estados/Aprobaciones/Atajos/Onboarding/Accesibilidad:** **[P]**.
- **Adoptar:** [P] hasta abrir el README completo o docs.
- **Nota de honestidad:** evidencia parcial → mayoría de campos [P].

## Coro Code — terminal — prioridad A — [P]

- **Motivo [P]:** **búsqueda realizada** en `awesome-cli-coding-agents` [Os] → **sin entrada**. Sin repo/doc localizada.

## Kode CLI — terminal — prioridad A — [Os parcial]

- **URL (en vivo):** `github.com/shareAI-lab/Kode-cli` (vía `awesome-cli-agents`, fetch [Os])
- **Funcionalidades clave:** "ShareAI's open-source CLI agent for **terminal-native coding with multi-provider support**" [Os].
- **Layout:** CLI terminal-native [Os]. **Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar CLI multi-provider terminal-native; evitar [P].

## QQCode — terminal — prioridad A — [Os parcial]

- **URL (en vivo):** `github.com/qnguyen3/qqcode` (vía `awesome-cli-agents`) [Os]
- **Funcionalidades clave:** "Lightweight CLI coding agent in **Rust** focused on speed, determinism, and developer control; supports **skills**" [Os].
- **Layout:** CLI [Os]. **Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar agente Rust ligero + skills; evitar [P].

## Ferrum — terminal — prioridad A — [Os parcial]

- **URL (en vivo):** `github.com/ominiverdi/ferrum` (dev en Codeberg) [Os]
- **Funcionalidades clave:** "Small Linux-only Rust-native coding agent with **interactive and headless modes**, ACP, **safety-tiered native tools**, **durable JSONL sessions**, Codex/ChatGPT OAuth, MCP, skills, image input" [Os].
- **Layout:** interactive + headless [Os]. **Estados:** durable JSONL sessions [Os]. **Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **safety-tiered tools** + sesiones JSONL durables; evitar limitación Linux-only.

## zot — terminal (harness) — prioridad A — [Os parcial]

- **URL (en vivo):** `github.com/patriceckhart/zot` (vía `awesome-cli-agents`, fetch [Os])
- **Funcionalidades clave:** "**Zero-overhead and lightweight coding agent harness with TUI/JSON/RPC modes**, structured tools, **reviewable file diffs, skills, extensions, and optional guardrails**" [Os].
- **Layout:** tres modos de ejecución (**TUI / JSON / RPC**) [Os]. **Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **modos TUI/JSON/RPC** + diffs reviewables; evitar [P].

## g3 — terminal — prioridad A — [P]

- **Motivo [P]:** **búsqueda realizada** en `awesome-cli-coding-agents` [Os] → **sin entrada**. Sin repo/doc localizada.

## Zap — terminal (skill-first) — prioridad A — [Os parcial]

- **URL (en vivo):** `zap-coding-agent/zap-coding-agent` (vía `awesome-cli-agents`) [Os]
- **Funcionalidades clave:** "**Skill-first Rust TUI coding agent** that injects only the context your task needs; **Single binary, no runtime**; Claude/Gemini/OpenAI + local (LM Studio); **code-indexed via SQLite**; MCP" [Os].
- **Layout:** **TUI** single-binary [Os]. **Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **skill-first context injection** + índice SQLite; evitar [P].

## jcode — terminal — prioridad A — [Os parcial]

- **URL (en vivo):** `github.com/1jehuang/jcode` (vía `awesome-cli-agents`) [Os]
- **Funcionalidades clave:** "Rust **TUI** agent optimized for RAM and startup latency (~28 MB PSS per session), built for **scaling many parallel sessions**; agent memory, **swarm mode**, browser automation, MCP, 40+ providers with OAuth" [Os].
- **Layout:** TUI [Os]. **Estados:** [P]. **Aprobaciones:** [P]. **Atajos:** [P]. **Onboarding:** OAuth login flows [Os]. **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **sesiones paralelas de bajo consumo** + swarm mode; evitar [P].

## Goose (Block) — terminal (agente general) — prioridad A — [Os parcial]

- **URL (en vivo):** https://goose-docs.ai/ (la antigua `block.github.io/goose/` redirige aquí) [Os]
- **Funcionalidades clave:** "goose is a **general-purpose AI agent that runs on your machine**. Not just for code — use it for **research, writing, automation, data analysis**, or anything…"; proyecto movido a la **Agentic AI Foundation (AAIF)** [Os].
- **Layout y navegación:** [P] (index mínimo esta sesión; es un agente CLI/TUI con extensiones).
- **Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar:** el posicionamiento **"general-purpose, not just code"** — coincide con la tesis de Atlas (plataforma, no editor); **extensiones** como modelo de capacidades.
- **Evitar:** [P].

## Plandex — terminal — prioridad A — [Os]

- **URL (en vivo):** https://github.com/plandex-ai/plandex (la doc `docs.plandex.ai` dio DNS ETIMEOUT; README verificado) [Os]
- **Funcionalidades clave:** "terminal-based AI development tool that can **plan and execute large coding tasks that span many steps and touch dozens of files**. It can handle up to **2M tokens** of context directly (~100k per file), and can index directories with **20M tokens or more using tree-sitter project maps**" [Os].
- **Layout y navegación:** herramienta **basada en terminal**; usa **project maps (tree-sitter)** para navegar el repo [Os].
- **Estados y feedback:** "capable of **full autonomy** — it can load relevant files, plan and implement changes, execute commands, and **automatically debug**" [Os].
- **Aprobaciones / HITL:** "highly flexible and configurable, giving developers **fine-grained control and a step-by-step review process** when needed" [Os].
- **Atajos de teclado:** [P] (no en el README indexado).
- **Onboarding / vacío / error:** [P].
- **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar el **project map (tree-sitter) para contexto** y el binomio **full-autonomy ↔ step-by-step review** (gradiente de autonomía, RFC 21); evitar asumir contexto ilimitado sin mapa.



## tmux / Zellij / Overmind — terminal multiplexers — prioridad A — [Os parcial]

- **URL (en vivo):** https://tmux.app/doc/man/ (tmux 3.6a man) · https://zellij.dev/documentation/ [Os]
- **Funcionalidades clave:** **tmux** = "terminal multiplexer" con **arquitectura client-server** y protocolo Unix socket (varios clientes sobre un servidor) [Os]; **Zellij** = "terminal workspace" con secciones de Installation, Configuration, **Layouts** y **Plugins** [Os].
- **Layout y navegación:** Zellij se organiza por **layouts** ("text files that define an arrangement of Zellij panes and tabs"); por defecto carga `default.kdl` del directorio `config/layouts`, y si no existe arranca con **un pane y un tab**; se invoca con `zellij --layout [name]` [Os]; tmux por sesiones/ventanas/paneles bajo el modelo client-server [Os].
- **Estados y feedback:** [P] en las páginas raíz indexadas.
- **Aprobaciones / HITL:** [P] (no aplica).
- **Atajos de teclado:** [P] en estas páginas (el detalle vive en las páginas de keybindings/layouts).
- **Onboarding / vacío / error:** Zellij documenta **Installation** como primer paso [Os].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **client-server con sesiones que sobreviven al cliente** (tmux) — base del requisito "sesiones que sobreviven desconexiones" (tlbx/frontend de RFC 24 §16); (b) **layouts declarativos** (Zellij) como plantilla de workspace reproducible; (c) **plugins** (Zellij) como modelo de extensión.
- **Evitar:** [P].
- **Enlaces:** `docs/design/CONSENSUS_AUDIT.md` §1.3 #15–16 (tmux, Zellij [Os] allí); **Herdr** (§1.3 #1) como multiplexer de agentes.

## Cobertura [P] de esta categoría (restante)

- **Fichas [Os] añadidas en este lote:** Claude Code, Codex CLI, Gemini CLI, Aider, gptme, Devon.
- **[P] con motivo concreto:** **Goose** (redirige a `goose-docs.ai`, sin contenido), **Plandex** (`docs.plandex.ai` DNS ETIMEOUT), **Coro Code / Kode CLI / QQCode / Ferrum / zot / g3 / Zap ; jcode** (sin URL canónica verificable), **SWE-agent** (no fichada), **Devin CLI** (cubierto por la ficha Devin en `ade-orquestador.md`), **Amp** (ficha en `ade-orquestador.md`; `docs.ampcode.com` DNS falló), **tmux / Zellij / Overmind** (enlazado a CONSENSUS_AUDIT).

## Enlaces a referentes ya auditados (no se repiten)

- **tmux**, **Zellij** (multiplexers): `docs/design/CONSENSUS_AUDIT.md` §1.3 #15–16.
- **Herdr** (terminal workspace manager, rollup de estado): `CONSENSUS_AUDIT.md` §1.3 #1.
