# 66 - UX Architecture & Design System

**Author:** UI/UX Design agent · **Date:** 2026-10-06
**Status:** Draft v2 — FASE 1–8 hechas. FASE 1–7 (Discovery → Research → Audit → Synthesis → IA → Interaction → Design System) + **FASE 8 (wireframes lo-fi + mockup hi-fi)**. Artefactos: `docs/design/wireframe.html`, `docs/design/mockup.html`, `docs/design/PALETTE.md`. FASE 9–10 (validación a11y / guía de implementación) pendientes, ver §16.
**Referencias verificadas en vivo (2026-10-06):** Herdr (`herdr.dev`), Orca (`onorca.dev`, `github.com/stablyai/orca`), Genspark AI Workspace 6.0 — ver §2.1.
**Depends on:** RFC 17 (UI base), RFC 24 (HUD Mission Control — spec), RFC 65 (HUD v2 — implementación), RFC 63 (Agentic Capability / `AgentCard`), RFC 19 (Supervisor — estados), RFC 21 (Execution Modes), RFC 23 (Prompt Understanding), RFC 61 (gap B: el HUD es un panel de debug).
**Scope:** Establece el **modelo mental**, la **arquitectura de información**, el **modelo de interacción** y las **foundations del Design System** de Atlas OS. No implementa; especifica. No añade motores.

---

## 0. Por qué este RFC existe

RFC 24 define **qué** debe mostrar el HUD. RFC 65 define **cómo** renderizarlo en Svelte. Ninguno define **por qué debe verse y comportarse así**: no hay modelo mental unificado, ni arquitectura de información, ni design system, ni identidad visual.

La auditoría `research/61 §5-b` lo dice sin ambigüedad: *"el HUD es un panel de debug, no el Mission Control de RFC 24"*. Y el estado real del código confirma un segundo problema que la auditoría no nombra: **la interfaz es visualmente un clon de GitHub Primer/VSCode** — la paleta está hardcodeada por componente (≈30 valores hex, `#8b949e` aparece 80 veces, no existe `src/app.css`), exactamente el anti-patrón que el brief prohíbe.

Este RFC cierra ese hueco con decisiones trazables a evidencia.

---

## 1. Discovery — estado real (observado en el repo)

**Código (observado):**
- `src/routes/+page.svelte` (1.250 líneas) contiene **todo** el HUD monolítico: `<script>` de ~310 líneas + secciones por vista + `<style>` inline + `<aside class="drawer">`. Layout `grid-template-columns: 14rem 14rem 1fr`.
- 24 componentes Svelte en `src/lib/components/`, todos con estilos **inline y hardcodeados** (sin tokens).
- `src/lib/stores/views.ts` define **13 vistas** (`overview, agent, kanban, approvals, cost, health, audit, canvas, outline, timeline, worktrees, settings, mcp`) — no las 8 de RFC 24; hay vistas extra (`overview`, `agent`, `settings`, `mcp`).
- `src/lib/stores/hud.ts` (49 KB) define el contrato WS + helpers (`phaseColor`, `agentStepColor`, projections).
- **No existe** `src/app.css` ni ningún archivo de tokens, pese a que RFC 65 §6 afirma que los tokens viven ahí.

**Visual (observado):** paleta dominante = GitHub Primer dark: `#0d1117` (bg), `#161b22` (panel), `#21262d`/`#30363d` (bordes), `#c9d1d9` (texto), `#8b949e` (muted), `#58a6ff` (accent), `#3fb950`/`#d29922`/`#f85149` (ok/warn/err), más `#8957e5`/`#d2a8ff` (púrpura).

**Contrato de estados (observado en `src-tauri/src/core/bus.rs`):**
- `AgentStatus` = `Queued, Reading, Planning, Coding, Reviewing, Idle, Paused, DoomLoop, Error, Success` (10).
- `StepPhaseTag` = `pending, executing, verifying, done, blocked` (5).
- `Confidence` = `High, Medium, Low, Block` (4).
- `ApprovalDecisionKind` = `Apr, Deny, Steer, Fork` (4).
- `BusEventKind` = 28 variantes; `KernelCommand` = `NewMissionFromPrompt, SwitchProfile, StopAgent, PauseAgent, ResumeAgent, SteerAgent, ForkAgent, DecideApproval`.

**Documentación (observada):** RFC 17 (layout base del editor + Mission Control), RFC 24 (spec del HUD, 714 líneas), RFC 65 (plan de implementación), RFC 22 §8.3 (patrones UI admin), RFC 61 (gap B), RFC 29 §4 (interpretación de la UI de Genspark), RFC 62 (catálogo de ~425 proyectos referencia).

### 1.1 Leyenda de procedencia (regla anti-alucinación)

| Marca | Significado |
|---|---|
| **[O]** Observado | Leído en código/RFCs de este repo en esta sesión. |
| **[I]** Inferido | Derivado de conocimiento previo / de la investigación ya registrada en el repo (`22`, `29`, `62`). **No inspeccioné las apps externas en vivo.** |
| **[R]** Recomendado | Decisión de diseño propuesta, pendiente de validación del operador. |
| **[P]** Pendiente | Requiere verificación (contraste exacto, test con usuario, prueba técnica). |

---

## 2. Research — síntesis comparativa (evidence-based)

> **Encuadre corregido — ver `docs/design/CONSENSUS_AUDIT.md`.** Esto **NO** es un "consenso del campo". Es una síntesis **evidence-based: 14 productos verificados en vivo [Os]** en esta fase (Herdr, Orca, Genspark, Zed, Langfuse, Vibe Kanban, n8n, VS Code, JetBrains, Linear, tmux, Zellij, Grafana, Notion) **+ 3 del repo [I]/[O]** (Hermes, Cursor, Conductor). La afirmación operador de "320+ proyectos" **no está documentada en ningún sitio**; el catálogo real (`research/62` ≈425) es de **arquitectura**, no de UX (~88–100 con UI relevante). Lo que sigue se lee, pues, como *evidencia n=14+3*, no como consenso universal.

### 2.1 Formato de análisis (regla 15)

**VS CODE [Os]** — *Strengths:* Activity Bar + Sidebar (+ Secondary Side Bar con Chat) + Editor + Panel + Status Bar; Command Palette (`Ctrl+Shift+P`) con modos (`>` comandos, `#` símbolos, quick-open) como infraestructura de descubrimiento; keybindings coherentes; LSP unificado. *Weaknesses:* modelo mental = archivos/carpetas, no objetivos; densidad baja y mucho espacio a barras. *Relevant:* Command Palette, Status Bar como franja de estado, jerarquía de paneles. *Avoid:* el eje archivos-primero, el estilo visual ("VS Code con otro logo"). *Adaptation:* Mission-primero; palette extendida a acciones de agente; status bar como franja de *señales vivas*. *(Fuente: code.visualstudio.com/docs/editing/userinterface.)*

**PYCHARM / JETBRAINS [Os]** — *Strengths:* jerarquía profunda, tool windows dockables, layouts guardables, inspecciones contextuales. **New UI** existe explícitamente para "reducir complejidad visual" y **"disclose progressive[mente] lo complejo según se necesita"**, con **Compact mode** para pantallas menores. *Weaknesses:* densidad de chrome abrumadora, IDE-centric. *Relevant:* tool windows, disclosing progresivo, densidad conmutable. *Avoid:* múltiples toolbars, iconografía críptica. *Adaptation:* disclosure progresivo + modos de densidad (§9.6). *(Fuente: jetbrains.com/help/idea/new-ui.html.)*

**CURSOR [I]** — *Strengths:* **review queue** como primer ciudadano (deja de ser "IA asistida"); **demos over diffs** (video/screenshot/logs); fork de sesión; agentes cloud con branch por agente. *Weaknesses:* el shell sigue siendo un fork de VS Code (identidad prestada). *Relevant:* review-queue-first, demos, fork. *Avoid:* clone visual. *Adaptation:* el HUD *es* el producto (no un fork).

**GENSPARK [Os, + RFC 29 §4]** — *Strengths:* arquitectura de tarea AI→artefacto; workspace como super-app; narrativa de resultado para no-devs; visualiza el plan del agente por sub-pasos (Parallel Search / Read). *Weaknesses:* cloud-first, opaco. *Relevant:* "la card responde a *una* pregunta"; transparencia del plan. *Avoid:* el modelo cloud-caja-negra. *Adaptation:* local-first + transparencia total (RFC 01 P11).

**HERMES [I, vía RFC 22 §1/§8.3]** — *Strengths:* health WS, coste/token en status bar, skills/MCP en caliente, profiles, worktrees, sandbox. *Weaknesses:* no audit hash-chained, no demos, no comments block-level, no review queue móvil. *Relevant:* health KPIs, cost audit, hot-swap skills. *Avoid:* ausencia de jerarquía recursiva. *Adaptation:* sumar lo que Hermes no tiene (audit, demos, canvas, outline).

**n8n [Os] / WINDMILL [I, vía RFC 22 §8.1]** — *Strengths:* canvas de nodos + **execution log por nodo**, **anclado bajo el lienzo**, con **sync de selección** canvas↔log; grupos colapsables; el grupo con error se auto-expande. *Relevant:* Canvas view + reasoning-trail por nodo + sync. *Avoid:* canvas decorativo sin trazabilidad. *Adaptation:* el nodo = subagente; el log = chain-of-thought real; **sync canvas↔log** (§10).

**NOTION [Os] / LINEAR [Os] / JIRA [I, vía RFC 22 §8.3]** — *Strengths:* Notion: **cada item es una página**, con **properties**, **comments** (a nivel de página/bloque, con modos Expanded/Off) y backlinks; **múltiples views** (table/list/board/calendar/gallery/chart) sobre los mismos datos. Linear: chrome "inverted L-shape", layouts list/board/timeline/split, swimlanes. *Relevant:* Outline recursivo + comments + multi-proyección. *Avoid:* profundidad que exija 4 clicks. *(Fuentes: notion.com/help/intro-to-databases; linear.app/docs/board-layout.)*

**ORCA [Os] / HERDR [Os]** — Orca: ADE con **worktree por agente**, **decision gates** que bloquean la task, Quick-open sobre worktrees/agents/commands, Design Mode. Herdr: terminal workspace manager con **estado por agente en sidebar** (`working/blocked/done/idle/unknown`) y **rollup** pane→tab→workspace; CLI y socket = la misma superficie que el agente usa. *Relevant:* estado de worker de primera clase, rollup, worktree-per-agent, gates. *Adaptation:* rollup al rail + §3.1 items 1-7. *(Fuentes: onorca.dev; herdr.dev/agent-guide.md.)*

**CLAUDE CODE / CODEX / OPENCODE [I, vía RFC 22 §5]** — *Strengths:* `doom_loop` como permiso (`allow|ask|deny`); `/loop` con caps de coste/minutos; terminal-first. *Relevant:* el modelo de permisos `allow|ask|deny`, caps. *Adaptation:* unificar con `Execution Modes` (RFC 21).

**Multiplexers / observabilidad / PM [Os] — cobertura cerrada 2026-10-06:** tmux (status bar + flags de ventana + jerarquía session/window/pane), Zellij (status-bar + session-manager + layouts WASM), Grafana (dashboards/panels + Explore + alerting ligado a paneles), Notion (views + comments + properties). Detalle y fuentes en `docs/design/CONSENSUS_AUDIT.md` §1.3/§2.

### 2.2 Matriz comparativa **evidence-based** (regla 16 — la última columna es la que importa)

| Problema (pregunta del humano) | VS Code | PyCharm | Cursor | Genspark | Hermes | n8n | **Mejor patrón Atlas OS [R]** |
|---|---|---|---|---|---|---|---|
| **¿Qué está pasando?** | Explorer | Tool windows | Review queue | Task list | Health HUD | Canvas | **Activity Spine** (ticker vivo + estado por agente) + Kanban |
| **Navegación raíz** | Files | Files | Files/Agents | Tasks | Missions | Workflows | **Missions** (rail izquierdo), no files |
| **Estados de agente** | — | — | status pill | status | health dots | node status | **Spine de estado** mapeado 1:1 a `AgentStatus` |
| **Aprobaciones** | — | — | inline | card | queue | — | **Approvals dock** + scope/pauserule (RFC 24 §5) |
| **Contexto** | open files | project | @-mentions | workspace | memory | — | **Context Rail** (misión/plan/evidencia/archivos) |
| **Terminal** | panel | panel | panel | tool | CLI | node | Panel anclable + **snapshot** por agente (RFC 24 §16) |
| **Multi-agente** | — | — | parallel cards | delegation | subagents | nodes | **Agent grid / Kanban / Canvas** (3 proyecciones) |
| **Feedback** | problems | inspections | demos+diff | chat | streamer | exec log | **Demo-over-diff + Evidence trail** (RFC 24 §9) |
| **Historia** | SCM | VCS | timeline | — | audit(parcial) | runs | **Audit chain + Timeline** |
| **Descubrimiento** | Palette | Search Everywhere | Cmd-K | search | CLI | — | **Command palette `:`** (RFC 24 §19) |

**Conclusión de síntesis [R]:** Atlas OS no debe adoptar *ningún* layout prestado. La respuesta correcta a "¿cuál es la mejor UX?" es un **Mission Control multi-proyección**: un único objeto (la Mission) y su swarm, proyectado en las 8 views de RFC 24, con el humano como **coordinador/reviewer**, no como programador-con-asistente. Layout = *rail de misiones + lienzo de proyección + spine de actividad + dock de aprobaciones*.

### 2.3 Auditoría de consenso (2026-10-06) — ver `docs/design/CONSENSUS_AUDIT.md`

> La afirmación operador de **"consenso tras 320+ proyectos"** **no está documentada en ningún sitio** (`rg` sin match). La cifra real del repo es **~425** (`research/62`), y es un catálogo de **arquitectura**, no de UX. De esos ~425, **~88–100** tienen UI relevante para un HUD de orquestación.

**Cobertura verificada en vivo: 14 proyectos, TODAS las categorías UX.** Herdr (multiplexer/estado+rollup), Orca (ADE/worktree+gates), Genspark (workspace), Zed (IDE/agente-first), Langfuse (observabilidad de trazas), Vibe Kanban (kanban+workspace), n8n (canvas+log), VS Code, JetBrains/IntelliJ, Linear, **tmux, Zellij (multiplexers), Grafana (observabilidad), Notion (doc/PM)**. Otros 3 (Hermes, Cursor, Conductor) son **[I]/[O]** de repo, no verificados en vivo. *(Datadog/Jira sin analizar.)* → **17 analizados de ~88–100.**

**Veredicto: SÍ — condicional** (ronda 2, 2026-10-06). Las decisiones estructurales se **confirman** (Mission-como-unidad, layout Mission Control, estado de agente primero, taxonomía HITL, método de color). El **teal no es consenso de campo** — es elección de diferenciación, no convención (D-66-03). De los **8 huecos de patrón** identificados, **5 ya incorporados** a este RFC + mockups: (1) colapso del turno al completar + "Worked for Nm" (§6.2); (2) sync canvas↔log (§10); (3) toggle Agregado↔Expandido (§10); (4) rollup de estado agente→Mission (§4/§10); (5) estado `Unknown` (§6.1). Los 3 restantes quedan como **decisiones abiertas del operador** (§16): (6) superficie de conversación; (7) split gate vs pregunta; (8) contexto worktree/branch/dev-server + sesión multi-agente.

---

## 3. Modelo mental (regla 7)

### 3.1 Decisión: ¿qué es Atlas OS para el usuario?

**NO** es un editor (RFC 17 §"layout base del editor de texto" queda como superficie secundaria). **NO** es un chat. **ES un centro de operaciones de misiones.**

| Candidato | ¿Unidad fundamental? | Veredicto [R] |
|---|---|---|
| Project | Demasiado estático; agrupa, no acciona. | Contexto, no unidad. |
| Workspace | Ambiente; contiene muchas misiones. | Contenedor. |
| Task | Atómico; pero el usuario piensa en objetivos. | Nodo del árbol. |
| Agent | Es el ejecutor, no el objetivo. | Rol dentro de la Mission. |
| Session | Eje temporal de un agente. | Proyección. |
| Workflow | Grafo de tareas; nuestro `Plan` = DAG (RFC 12). | Es el *plan* de una Mission. |
| **Mission** | **Objetivo del humano → understanding → plan → swarm → artefactos → resultado.** | **Unidad fundamental [R].** |

**Definición operativa:**
> Una **Mission** es la unidad de trabajo. Nace de un prompt (RFC 23), se consolida en un `Plan` (DAG, RFC 12), se ejecuta por un **swarm** de agentes (RFC 05) en un **worktree** aislado, produce **artefactos** y **evidencia** (RFC 63), y termina en un **reporter** (RFC 24 §17) con aprendizaje (RFC 16).

### 3.2 El rol del humano (regla 5, P1)

Tomado de RFC 17 §0 y RFC 22 §3.3 **[O]**: *"no es una IA asistida por humanos, sino un humano asistido por IA"*. El humano es **Coordinador/Reviewer**: define objetivo, aprueba, steer, y juzga. La UI debe optimizar para **revisar y decidir**, no para teclear.

**Corolario:** la pantalla de aterrizaje por defecto es la **cola de trabajo de la Mission** (Kanban/Overview), no el editor (RFC 17 §2.3 ya lo pedía **[O]**; implementación actual no lo cumple **[O]**).

---

## 4. Arquitectura de información (regla 8)

### 4.1 Jerarquía

```
Workspace (profile)
└─ Mission ······························· unidad fundamental
   ├─ Understanding (verdict + confidence + preguntas)   [RFC 23]
   ├─ Plan (Objective → Task → Subtask, DAG)             [RFC 12]
   ├─ Swarm: Agent runs                                   [RFC 05/63]
   │  ├─ Steps / Tool calls / Evidence / Artifacts        [RFC 63]
   │  ├─ Worktree + branch + dirty                         [RFC 05]
   │  └─ Approvals                                        [RFC 18/24]
   ├─ Artifacts (diffs, ficheros, previews, demos)        [RFC 24 §9]
   ├─ Audit (hash-chained)                                 [RFC 18/24 §10]
   └─ Reporter (outcome + learning rules)                 [RFC 24 §17]
```

### 4.2 Asignación a superficies de UI (decisión [R])

| Concepto | Superficie | Razón |
|---|---|---|
| Workspace/Profile | **Top Bar** (switcher) | Cambia el contexto global entero. |
| Mission list | **Mission Rail** (izquierda, persistente) | Navegación raíz; frecency (RFC 36 §A.1 **[O]**). |
| Mission activa (Understanding/Plan/Swarm) | **Lienzo central** (proyección) | Objeto de trabajo. |
| Agent runs / pasos | **Kanban / Canvas / Outline / Timeline** | 4 proyecciones del mismo Journal. |
| Coste/recursos, Health, Audit, Worktrees | **Vistas del lienzo** | Preguntas de distinto tipo (RFC 24 §2). |
| Actividad viva | **Activity Spine** (derecha) | Ticker cronológico con color semántico. |
| Approvals | **Dock inferior** + badge en Top Bar | Requieren acción; no deben perderse. |
| Settings / MCP / Secrets | **Vista** (no modal block) | Configuración raramente usada. |
| Acciones (`run/pause/fork/steer`) | **Card + palette + hotkeys** | 3 vías: mouse, teclado, contextual. |
| Ejecución Mode / Modo uso | **Status Bar** (badges) | Estado global siempre visible. |
| Notificaciones (doom, cost, approval) | **Toast + overlay + spine** | Urgencia decreciente. |

**Regla de oro [R]:** ningún concepto vive en más de una superficie *primaria*; en las demás aparece como **referencia**, nunca duplicado (Calmness, regla 5.10).

---

## 5. Layout (regla 9)

### 5.1 Alternativas evaluadas

| Arquitectura | Veredicto |
|---|---|
| IDE clásico (Activity Bar + Sidebar + Editor + Panel) | ✗ Eje archivos-primero; es el modelo a superar. |
| Chat céntrico + paneles | ✗ Reduce el swarm a un hilo; viola RFC 24. |
| Dashboard de widgets | ✗ RFC 24 §0 lo rechaza explícitamente ("no es un dashboard pasivo"). |
| Single workspace | ✗ No permite paralelismo ni multi-misión. |
| **Mission Control multi-proyección** | ✓ **Decidido [R]:** rail + lienzo + spine + dock. |

### 5.2 Layout decidido [R]

```
┌──────────────────────────────────────────────────────────────────────────┐
│ TOP BAR  ⌂ logo │ [Mission: Auth refactor ▾] │ profile ▾ │ 🔔4  ◐  ⚙  👤 │
├────────────┬───────────────────────────────────────────────┬─────────────┤
│ MISSION    │  VIEW BAR  [Kanban][Canvas][Outline][Timeline] │  ACTIVITY   │
│ RAIL       │            [Cost][Health][Audit][Worktrees]    │  SPINE      │
│                        │                                   │  (ticker)   │
│ ▸ Auth ref… ●          │   ┌────────┐ ┌────────┐ ┌───────┐ │ ● 14:21 ok  │
│ ▸ Dashboard            │   │ AgentX │ │ AgentY │ │AgentZ │ │ ▲ 14:20 apr │
│ ▸ Bug 422              │   │ coding │ │ idle   │ │doom⚠  │ │ ✖ 14:19 err │
│                        │   └────────┘ └────────┘ └───────┘ │ ⬦ 14:18 res │
│ [+ New Mission]        │   (proyección de la Mission activa)│             │
├────────────┴───────────────────────────────────────────────┴─────────────┤
│ APPROVALS DOCK  ⚠ AgentX: create src/auth/session.go  [APR][DENY][STEER]  │
│ STATUS BAR  🕊 Manual · 🏛 architect · conf 0.87 · VRAM 4.1G · $0.42 · ⚠  │
└──────────────────────────────────────────────────────────────────────────┘
```

Notas de decisión:
- **Mission Rail** sustituye al File Explorer como eje raíz (diferencia estructural con VSCode/Cursor).
- **Activity Spine** es una columna, no un panel inferior: la lectura cronológica gana con altura (RFC 24 §11 **[O]**).
- **Approvals Dock** inferior: acción, no información → siempre alcanzable, colapsable.
- **Status Bar** = franja de *señales* (modo, confianza, recursos, coste, alertas), no de metadata de archivo.
- El **editor de texto** (RFC 17) queda como superficie secundaria que se abre desde una card/artifact (split en el lienzo), no como pantalla raíz.

---

## 6. Agentes como ciudadanos de primera clase (regla 10)

### 6.1 Estados UX — mapeados al enum REAL

No se inventa taxonomía: se usa `AgentStatus` **[O]** y se define su semántica visual + acción disponible.

| `AgentStatus` [O] | Significado para el humano | Color (rol semántico) | Acción primaria |
|---|---|---|---|
| `Queued` | En espera de slot | neutro (faint) | `▶ Run` (o esperar) |
| `Reading` | Leyendo contexto | info (cyan) | `⏸ Pause` |
| `Planning` | Planificando | info (cyan) | `⏸ Pause` |
| `Coding` | Editando/ejecutando | info/activo (cyan intenso) | `⏸ Pause · 💬 Steer` |
| `Reviewing` | Autoevaluando | warn (amber bajo) | `⏸ Pause` |
| `Idle` | Ocioso | muted | `▶ Run` |
| `Paused` | En checkpoint blando | warn (amber) | `▶ Resume · ⏹ Stop` |
| `DoomLoop` | **Bucle detectado** | err (rojo) + pulse | `🚑 Recover · 🔓 Override` |
| `Error` | Falló | err (rojo) | `🚑 Recover · 🔱 Fork` |
| `Success` | Terminó OK | ok (verde) | `🎬 Demo · 👁 Reason` |
| `Unknown` **(propuesto)** | Presente, no clasificable | faint `#868686` | `👁 Inspect · ⏹ Stop` |

> **`Unknown` NO está en `AgentStatus` [O]** — es una adición **propuesta** (CONSENSUS_AUDIT §3.1-5): solo Herdr [Os] documenta un estado `unknown` explícito (1 de 14 verificados; la documentación leída de los demás no lo cita). Es una propuesta de **fuente única**, justificada sobre todo porque el detector de Atlas puede no lograr clasificar un agente. Mapea a `--a-text-faint` `#868686` — **medido** 5.33 / 4.97 / 4.52:1 vs `bg` / `surface` / `surface-2` (**AA** en los tres; **sin hex nuevo**). Se distingue de `Queued` (también `faint`) por **glifo** (`?` vs punto hueco). Requiere cambio de enum en Rust (FASE 10, fuera de este encargo).

`StepPhaseTag` **[O]** (`pending/executing/verifying/done/blocked`) se usa a nivel de **paso**, no de agente.

### 6.2 Agent Card — anatomía (rule 10/13)

Reemplaza el **debug panel** actual (AgentCard.svelte solo muestra `run_id` + tokens **[O]**) por la anatomía de RFC 24 §3.1 **[O]**, en capas de disclosure:

- **Capa 1 (siempre):** rol + modelo + estado (spine) + tiempo + diff `+N/-M` + coste.
- **Capa 2 (hover/expand):** confidence+judgment, files touched, tool calls, skill, execution mode, modo uso.
- **Capa 3 (click):** step timeline + evidence + worktree/branch + checkpoint.
- **Capa 0 (al COMPLETAR — `Success`/`Error`):** el card **auto-colapsa** las capas 2-3 a un header `▸ Worked for 14m 22s · Thinking / Commands / Edits` (patrón **Zed [Os]**). El detalle queda accesible al expandir; responde de un vistazo a "¿qué ficheros/comandos tocó?".
- **Acciones** (según estado, §6.1): `▶ ⏸ ⏹ 🔱 🚑 💬 👁 🎬` (RFC 24 §3.2 **[O]**).

---

## 7. Human-in-the-loop (regla 11)

### 7.1 Taxonomía — nunca mezclar (decisión [R])

| Clase | Disparador real [O] | Superficie | Bloquea al agente | Persistencia |
|---|---|---|---|---|
| **Informativo** | `AgentStep`, `AgentStatusChanged` | Spine | no | efímero |
| **Confirmación** | `ApprovalRequest` (acción sensible) | Dock + badge | sí | hasta decisión |
| **Decisión** | varias alternativas (plan/fork) | Card inline | sí | hasta decisión |
| **Bloqueo** | `DoomLoopDetected` | Overlay rojo | sí | hasta acción |
| **Error** | `AgentStatus::Error` | Card + spine | no (recuperable) | hasta recover |
| **Resultado** | `#`/`Success` + Reporter | Reporter card | no | permanente |

### 7.2 Doom loop (workflow, RFC 24 §18.2 **[O]**)
Overlay rojo **modal** (bloquea porque el agente no puede seguir): `🚑 Recover` · `🔓 Override with reason` (input obligatorio → audit hash-chain) · `⏹ Stop`. Nunca auto-descartable.

### 7.3 Aprobaciones (RFC 24 §5 **[O]**)
Tres modalidades: **batch** (varias sin conflicto de fichero), **scope** (patrón, ej. `src/auth/*`), **pauserule** (patrón → auto-pausa persistente). Decisión `Apr/Deny/Steer/Fork` **[O]**.

---

## 8. Modelo de interacción (regla 12)

### 8.1 Loop maestro

```
Prompt ─▶ [Understanding panel] ─▶ Lock & Plan ─▶ [Kanban/Outline swarm]
                     │                                   │
                 (Low/Block → preguntas)          aprobaciones / steer / fork
                                                         │
                                              [Demo/Evidence/Reporter]
```

### 8.2 Disclosure por profundidad (decisión [R])
- **Usuario no técnico:** ve estado, demo, aprobación en lenguaje de outcome (RFC 17 §11 **[O]**).
- **Usuario técnico:** expande a step timeline, tool calls, JSON de inputs/outputs (RFC 24 §13 **[O]**).
- **Regla:** el nivel 1 nunca muestra JSON crudo; el nivel 3 nunca fuerza a leer logs (RFC 24 §13 busca reemplazar "mirar el log").

### 8.3 Handoff y paralelismo
- **Fork** = sesión hija `HUMAN_IN_LOOP` con contexto copiado (RFC 24 §3.2 **[O]**).
- Aristas del Canvas = handoff/dependencia; nodo en doom → downstream *dimmed* (RFC 24 §13 **[O]**).

---

## 9. Design System — Foundations (regla 13)

> **Objetivo:** sacar la paleta y las métricas del código (hoy hardcodeadas [O]) y centralizarlas en **un** archivo de tokens. RFC 65 §6 decía `src/app.css`; ese archivo **no existe** [O] → este RFC lo especifica.

### 9.1 Identidad visual — "Calm Instrumentation" [R]

La identidad **no** se copia: se deriva de la función (regla 14). Atlas OS es un **panel de instrumentos**: alta densidad, autoridad calmada, semántica antes que decoración.

Cinco reglas de identidad:
1. **Neutro de-carbón, no GitHub-blue-grey.** Base grafito ligeramente desaturada; el color *significa*, no decora.
2. **Un solo acento primario** (interacción/atención humana) = **teal señal**; distinto del azul VSCode `#007ACC` y del `#58A6FF` de GitHub **[O]**.
3. **Color de estado = vocabulario cerrado** ligado a `AgentStatus`/`Confidence`/`ApprovalDecisionKind` (no hay colores libres por componente).
4. **Mono para hechos de máquina** (ids, paths, tokens, coste, hashes), **sans para prosa humana**.
5. **El estado se lee por "spine"**: una barra vertical de 3px a la izquierda de cada card/row con el color de estado → escaneo periférico sin leer (patrón de instrument panel).

### 9.2 Color tokens [R] (valores concretos)

> **Addendum A supersedes this draft.** The final palette is **authored in OKLCH, converted to sRGB, and WCAG-validated by computation** — see `docs/design/PALETTE.md` (with measured contrast) and the applied mockup `docs/design/mockup.html`. Dark accent = **teal `#4fd5d1`**; neutrals are **pure graphite, chroma 0** (not GitHub's blue-grey). The values below are the pre-computation draft and are kept only for traceability.

**Dark (default):**

| Token | Valor | Uso |
|---|---|---|
| `--a-bg` | `#0B0E13` | Fondo app |
| `--a-surface` | `#12161D` | Panel |
| `--a-surface-2` | `#1A1F29` | Panel elevado / hover |
| `--a-border` | `#252C38` | Borde base |
| `--a-border-strong` | `#333C4B` | Borde foco/activo |
| `--a-text` | `#E6EAF0` | Texto primario |
| `--a-text-muted` | `#9AA4B2` | Secundario |
| `--a-text-faint` | `#6B7686` | Terciario/placeholder |
| `--a-primary` | `#4fd5d1` | Acento (interacción/atención) — final |
| `--a-primary-strong`| `#4FE0CE` | Hover/foco |
| `--a-info` | `#4C9EF5` | Información / paso activo |
| `--a-ok` | `#46C46A` | Éxito |
| `--a-warn` | `#E0A83E` | Advertencia / pausa |
| `--a-err` | `#F0616D` | Error / doom loop |
| `--a-violet` | `#A98BF5` | Research / fork / learning |
| `--a-focus` | `#35C6B5` | Anillo de foco (2px + offset 1px) |

**Light:** invertir superficie/texto manteniendo la *misma semántica* (`--a-primary` oscurece a `#0E8F82`, `--a-ok` `#1E8A44`, etc.). Valores exactos y **ratios de contraste WCAG ≥ 4.5:1 (texto) / ≥ 3:1 (UI)** → **[P]** validar con `accessibility-audit` (RFC 17 §7).

**Mapeo semántico cerrado [R]:**

| Dominio | Estado | Token |
|---|---|---|
| AgentStatus | queued/idle | `--a-text-faint` |
| | reading/planning/coding | `--a-info` |
| | reviewing | `--a-warn` |
| | paused | `--a-warn` |
| | doom_loop/error | `--a-err` |
| | success | `--a-ok` |
| Confidence | High / Medium / Low / Block | ok / warn / violet? / err → **Low = `--a-err`-tinted, Block = `--a-err`** |
| Approval | Apr / Deny / Steer / Fork | ok / err / info / violet |
| Events | info/success/apr/cost/error/research/steer | muted/ok/warn/warn/err/violet/info (RFC 24 §11 **[O]**) |

### 9.3 Tipografía [R]
- **Sans (prosa/UI):** system stack — `ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif`. Sin fuentes nuevas (boundary rule; RFC 65 §6 **[O]**).
- **Mono (hechos):** `ui-monospace, "JetBrains Mono", "Cascadia Code", Menlo, monospace`.
- Escala: `12 / 13 / 14 / 16 / 20 / 24` px; línea `1.45`; pesos `400/500/600`. Etiquetas en `11px` + `letter-spacing .02em` + `uppercase` para *rail/status*.

### 9.4 Espaciado, radio, elevación [R]
- Espaciado base **4px**: `4/8/12/16/24/32/48`.
- Radio: `6px` (controles), `10px` (cards), `999px` (pills/badges).
- Elevación: **mínima** — separar por borde + `--a-surface` antes que sombra. Overlays: `0 8px 32px rgba(0,0,0,.45)`.
- Foco: anillo `--a-focus` 2px, offset 1px, **siempre visible** (teclado-first).

### 9.5 Motion [R]
- `--a-dur-1` 120ms (hover/focus), `--a-dur-2` 180ms (aparecer/expandir), `--a-pulse` 600ms (status change, RFC 24 §4.2 **[O]**).
- Easing `cubic-bezier(.2,.7,.3,1)`.
- **`prefers-reduced-motion`:** desactiva pulse/transiciones (a11y, **[P]** verificar).
- Regla Calmness: ninguna animación puramente decorativa.

### 9.6 Densidad [R]
Dos modos: **Comfortable** (default, touch) y **Compact** (pro, teclado). El rail y el status bar ganan densidad; el lienzo mantiene padding. Objetivo RFC 24 §20: virtualización ~200 nodos DOM **[O]**.

### 9.7 Iconografía [R]
Set lineal 16/20px; formas funcionales (▶ run, ⏸ pause, ⏹ stop, ⚡ fork, 🚑 recover, 💬 steer, 👁 reason, 🎬 demo — RFC 24 §3.2 **[O]**). Un icono no se reutiliza para dos significados.

---

## 10. Componentes y patrones (regla 13)

**Componentes base [R]:** Button (primary/secondary/ghost/danger), Input, Select, Tabs, SegmentedControl (ViewBar), Panel, Card, AgentCard, TaskCard, StepPill, StatusDot, ConfidenceMeter, Badge/Pill, Tooltip, Dialog, Drawer (Approvals), Toast, CommandPalette, Menu, Table, Tree, TerminalSurface, EditorSurface, DiffView, EvidenceList, Sparkline, Gauge.

**Patrones [R]:** Mission navigation (rail+palette), Agent workspace (grid↔kanban↔canvas), Task management (outline↔kanban), Context management (Context Rail), Tool execution (step pills + evidence), Approval flow (dock→batch→scope→pauserule), Error/doom handling (overlay+recover), Multi-agent monitoring (spine+KPIs), Workflow visualization (canvas+timeline).

**Patrones añadidos por la auditoría (CONSENSUS_AUDIT §3.1, n=14 live):**
- **Collapse-on-completion** — al terminar, el turno del agente colapsa con header `Worked for Nm` (Zed **[Os]**); §6.2 capa 0.
- **Canvas ↔ log sync** — seleccionar un nodo en Canvas selecciona su entrada de log y viceversa; el log puede anclarse bajo el lienzo (n8n **[Os]**).
- **Agregado ↔ Expandido** — toggle en Canvas/Outline: un nodo por *nombre de paso* vs uno por *llamada*; loops como ciclos vs DAG (Langfuse **[Os]**).
- **State rollup** — el estado hijo más severo asciende a la Mission en el rail (Herdr/Orca **[Os]**); §4.

**Estados obligatorios por componente (regla 23):** default, hover, focus, active, selected, disabled, loading, success, warning, error, empty, offline, blocked, permission-required. Para agentes/tareas además: queued, running, waiting, paused, interrupted, completed, failed.

---

## 11. Responsive y multiplataforma (regla 21)

| Ancho | Comportamiento [R] |
|---|---|
| ≥1600 (ultrawide) | Rail + lienzo + spine + dock simultáneos. |
| 1100–1600 | Spine colapsable; dock fijo. |
| 720–1100 | Rail → drawer; spine → badge+drawer; lienzo = 1 columna. |
| ≤720 (móvil, RFC 24 §16 **[O]**) | **Modo review**: Approvals + Mission list + activity; sin canvas/editor. Acciones `APR/DENY` one-hand. |

DPI scaling: usar rem/px lógicos; probar 125/150% (Windows) **[P]**.

---

## 12. Keyboard-first (regla 22)

- **Command palette `:`** con fuzzy sobre acciones + misiones (RFC 24 §19 / RFC 65 §5 **[O]**). Ya existe `CommandPalette.svelte` **[O]**.
- Hotkeys canónicos (RFC 24 §19 **[O]**): `:v` view · `:a` approvals · `:n` new · `:f` fork · `:s` steer · `:d` demo · `:r` run · `:p` pause · `:x` stop · `:c` comment · `:o` modo. **Nota [O]:** `views.ts` **desalinea** varias (`agent`=`a`, `approvals`=`p`, `overview`=`o`) → **debe** reconciliarse con RFC 24 §19 (conflicto detectado).
- Focus management: foco atrapado en overlays; `Esc` cierra; anillo siempre visible.
- Shortcuts configurables vía `hud.yaml` (RFC 24 §21 **[O]**).

---

## 13. Validación del diseño (regla 19)

| Eje | Pregunta | Estado [R] |
|---|---|---|
| Funcionalidad | ¿Cubre las 8 views + approvals + HITL? | ✓ mapea RFC 24 1:1 |
| UX | ¿Modelo mental claro? | ✓ Mission-centric (§3) |
| Efficiency | ¿Rápido para pro? | ✓ palette + hotkeys + 3 vías por acción |
| Consistency | ¿Un solo sistema? | ✓ tokens cerrados (§9) — **pendiente refactor** |
| Scalability | ¿Muchas misiones/agentes? | ✓ virtualización + spine |
| Accessibility | ¿Contraste/teclado/motion? | **[P]** verificar ratios + reduced-motion |
| Feasibility | ¿Compatible con la arquitectura? | ✓ solo Svelte+tokens, sin deps (RFC 65 §6 **[O]**) |
| Maintainability | ¿Deuda? | ✓ token central elimina hardcode [O] |

---

## 14. Registro de decisiones (regla 24)

### D-66-01 — Mission como unidad fundamental
**Problema:** el eje actual es la vista/panel, no el objetivo. **Contexto:** RFC 24 §1 (sidebar=Missions) ya lo insinúa **[O]**; la implementación usa `overview` genérico **[O]**. **Alternativas:** Project / Agent / Session / Mission. **Evidencia:** RFC 01 P4, RFC 12, RFC 24 §1 **[O]**; Cursor/Linear/Notion usan objetivo-jerarquía **[I]**. **Decisión:** Mission. **Trade-offs:** exige rail + ficheros como contexto secundario. **Consecuencia:** IA de §4. **Status:** Aceptada [R], validar operador.

### D-66-02 — Layout Mission Control (no IDE)
**Problema:** el layout actual es IDE-like. **Evidencia:** RFC 24 §0-1 **[O]**; `+page.svelte` grid de 3 columnas **[O]**. **Decisión:** rail + lienzo multi-proyección + spine + dock. **Consecuencia:** §5.2. **Status:** Aceptada [R].

### D-66-03 — Identidad "Calm Instrumentation" + tokens centralizados
**Problema:** paleta GitHub hardcodeada ~30 colores [O] = clon visual. **Decisión:** `src/app.css` con tokens `--a-*`, acento teal, color de estado cerrado. **Nota:** el teal es **elección de DIFERENCIACIÓN, no consenso** — ningún referente verificado usa teal como señal (CONSENSUS_AUDIT §3); lo que sí es consenso es el *método* (OKLCH + semantic tokens + WCAG 2.2/APCA + CVD). **Consecuencia:** §9. **Status:** Aceptada [R]; light/contraste **medidos** (PALETTE §4/§4.1).

### D-66-04 — Estados de agente = enum real (no taxonomía nueva)
**Evidencia:** `AgentStatus` **[O]**. **Decisión:** mapear 1:1. **Status:** Aceptada.

### D-66-05 — Reconciliar hotkeys `views.ts` ↔ RFC 24 §19
**Problema:** conflicto observado **[O]**. **Decisión:** RFC 24 §19 manda; `views.ts.key` se corrige. **Status:** Propuesta [R], requiere tocar código (FASE 10).

---

## 15. Impacto en el código existente (FASE 10, no ejecutado)

1. Crear `src/app.css` (tokens §9) + importarlo en `+layout.svelte` (hoy no carga CSS **[O]**).
2. Refactor de los 24 componentes: reemplazar hex por `var(--a-*)`; extraer `+page.svelte` (1.250 líneas) en las vistas.
3. Reconciliar `views.ts` con el layout y las hotkeys (§12, D-66-05).
4. Sustituir `AgentCard` debug por anatomía §6.2.
5. Añadir Mission Rail + Activity Spine + Approvals Dock como componentes.
6. `accessibility-audit` + `design-tokens` test (vitest) **[P]**.

---

## 16. Roadmap de diseño y decisiones abiertas

**FASE 8 — HECHA (2026-10-06).** Entregables en `docs/design/`:
- `wireframe.html` — lo-fi: layout maestro, ciclo de misión (HITL), anatomía de Agent Card en 3 capas, timeline por lanes, colapso responsive.
- `mockup.html` — hi-fi: el Mission Control completo con la paleta Addendum A, estados de agente reales, dark + light, leyenda de tokens y **` :focus-visible`**. Ahora refleja los patrones de la auditoría: collapse-on-completion ("Worked for Nm"), canvas↔log sync, Agregado↔Expandido, rollup de estado y estado `Unknown`. Verificado por render headless (Edge) en ambos temas.
- `PALETTE.md` — **consenso de MÉTODO** (OKLCH + WCAG 2.2/APCA + CVD) y paleta final con contraste **medido**; el teal queda marcado como **diferenciación, no consenso**.
- `CONSENSUS_AUDIT.md` — auditoría de consenso (14 live + 3 repo), matriz de decisiones, veredicto.

**Patrones incorporados de la auditoría (items 1–5 — spec + mockups):** collapse-on-completion + `Worked for Nm` (§6.2); canvas↔log sync (§10); toggle Agregado↔Expandido (§10); rollup de estado agente→misión (§4/§10); estado `Unknown` (§6.1, PALETTE §5).

**Decisiones del operador ya resueltas:**
1. **Acento primario:** *"investiga el mejor consenso de colores"* → hecho: teal `#4fd5d1` (Addendum A), derivado por investigación + cálculo, **marcado como diferenciación, no consenso** (D-66-03).
2. **Scope del siguiente paso:** wireframes + mockups, **solo diseño, sin tocar código** → respetado (artefactos en `docs/design/`, `src/` intacto).
3. **Editor de texto:** **fuera de alcance v1** (el HUD es el producto).
4. **Referencias externas:** **14 verificadas en vivo** (Herdr, Orca, Genspark, Zed, Langfuse, Vibe Kanban, n8n, VS Code, JetBrains, Linear, tmux, Zellij, Grafana, Notion) + 3 del repo → §2 / `CONSENSUS_AUDIT` §1.3.

**Decisiones abiertas del operador (items 6–8 del audit — registradas, NO implementadas):**

**OA-66-06 — Superficie de conversación.**
- **Problema:** RFC 66 eliminó el chat lateral, pero Vibe Kanban mantiene un *Conversation Panel* y el *Agent Panel* de Zed **es** una conversación. [Os]
- **Opciones:** (a) sin chat (todo card + steer inline); (b) panel de conversación colapsable por agente; (c) híbrido: steer inline + hilo expandible por run.
- **Recomendación [R]:** (c) — el steer inline cubre el caso común; el hilo completo se abre desde `👁 Reason`.
- **DECISIÓN (Project Lead, 2026-10-06, delegada por el operador): (c) híbrido.** Steer inline en la card + hilo expandible por run. Sin panel de chat global.

**OA-66-07 — Approvals Dock: *gate* vs *pregunta*.**
- **Problema:** Orca separa **decision gate** (bloquea la task) de **question** (`ask`, no bloqueante). RFC 66 las une en un solo dock. [Os]
- **Opciones:** (a) dock único (actual); (b) dos canales (gate bloqueante vs pregunta async); (c) dock único con sub-secciones.
- **Recomendación [R]:** (b) — el comportamiento difiere (un gate detiene el swarm; una pregunta no).
- **DECISIÓN (Project Lead, 2026-10-06, delegada por el operador): (b) dos canales.** Gate bloqueante (detiene la task) y pregunta asíncrona (no bloquea), con tratamiento visual y de teclado distintos.

**OA-66-08 — Contexto de ejecución + sesión multi-agente.**
- **Problema:** Vibe/Orca exponen **worktree/branch/dev-server**; Langfuse agrupa varios agentes en una **session** que alimenta un artefacto. RFC 66 no especifica ninguno. [Os]
- **Opciones:** (a) Context Rail solo con misión/plan/evidencia (actual); (b) + worktree/branch/dev-server; (c) + sesión multi-agente explícita.
- **Recomendación [R]:** (b) en v1; (c) como v2 — la Mission ya agrupa agents, pero la *sesión* (varios runs → un artefacto) merece su propio objeto.
- **DECISIÓN (Project Lead, 2026-10-06, delegada por el operador): (b) en v1, (c) en v2.** Context Rail incluye worktree/branch/dev-server en v1; el objeto *sesión multi-agente* queda para v2.

**FASE 9–10 pendientes:**
- **FASE 9 (validación):** correr el detector de `impeccable` (los módulos de parseo HTML faltan en esta máquina → fallback regex, solo em-dash advisory), verificación APCA a tamaños reales, simulador CVD, y `accessibility-audit`.
- **FASE 10 (implementación):** `src/app.css` con los tokens de Addendum A → refactor de componentes → reconcile de hotkeys `views.ts` ↔ RFC 24 §19 → Mission Rail / Activity Spine / Approvals Dock. *(Solo tras aprobación — no se toca código en esta fase de diseño.)*

---

## 17. Estado

- **Hecho:** FASE 1 Discovery, FASE 2 Research (síntesis), FASE 3 Audit, FASE 4 Synthesis (§2), FASE 5 IA (§4), FASE 6 Interaction (§6-8), FASE 7 Design System (§9-10).
- **Hecho (adicional):** FASE 8 wireframes/mockups (`docs/design/`). **FASE 9 validación parcial (2026-10-06):** contraste re-medido en las 3 superficies (`docs/design/PALETTE.md` §4.1): 3 hallazgos (dark `faint` sobre `surface-2` 4.29:1; light `warn` sobre `surface` 4.40:1; APCA < Lc 60 en colores de estado oscuros a tamaño pequeño). Línea base de código verificada: `cargo test --lib` 1392 ok, `pnpm check` 0 errores, `pnpm test` 116 ok.
- **Aplicado:** dark `faint` → `#868686` y light `warn` → `#966000` (AA en todas las superficies, re-medido); `PALETTE.md` y `mockup.html` actualizados. Queda la regla APCA: etiquetas de estado pequeñas usan `--a-text` y el color solo en el glifo.
- **Pendiente:** simulación CVD, auditoría de foco visible en el mockup, y FASE 10 (implementación, requiere aprobación del operador).
- **Cubre:** gap B de RFC 61 **[O]**; ausencia de design system detectada **[O]**; identidad visual propia (regla 14).
