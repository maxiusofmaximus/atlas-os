# ux-catalog / kanban-pm

Fichas de referentes con **primario = kanban / gestión de trabajo**. La categoría tenía solo 1 entrada A propia en el catálogo (Vibe Kanban, enlazada a `CONSENSUS_AUDIT`); aquí se añade **Linear [Os]** como referencia real de kanban/PM y se enlazan las demás.

---

## Linear — kanban / PM — prioridad (referencia transversal) — [Os]

- **URL (en vivo):** https://linear.app/docs/board-layout [Os]
- **Funcionalidades clave:** "**Nearly all views in Linear can be shown in board layout** in addition to list view"; **feature parity** casi total entre board y list [Os].
- **Layout y navegación:** board con **columnas por group**; agrupación configurable por **Status (default), Project, Priority, Cycle, Label, Label group, SLA status** y más; columnas **ocultables** (en list solo vía filtros); crear issue con el **`+`** al tope de una columna; **swimlanes** colapsables [Os].
- **Estados y feedback:** statuses ordenados primero→último cuando se agrupa por status; opción **Show empty groups** para ocultar columnas vacías [Os].
- **Aprobaciones / HITL:** [P] (no aplica directamente).
- **Atajos de teclado:** **Cmd/Ctrl B** toggle board/list; **X** seleccionar issue; **Shift X**/_Shift+Click_ multi-select; **Option/Alt Shift ↑/↓** mover issue al top/final de la columna; **T** colapsar/expandir swimlane [Os].
- **Onboarding / vacío / error:** [P].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **mismos atajos en board y lista** (paridad de interacción entre vistas — refuerza "mismos shortcuts" de RFC 66); (b) **agrupación conmutable** (status/project/priority/...); (c) **hide/show empty groups**; (d) `+` por columna para crear en contexto.
- **Evitar:** que board y lista no puedan ordenarse independientemente (limitación documentada) — Atlas debería permitir ejes de orden por vista.

## Vibe Kanban (BloopAI) — ADE + kanban — prioridad A — [Os]

- **URL (en vivo):** https://github.com/BloopAI/vibe-kanban (`docs/workspaces/interface.mdx`) · https://vibekanban.com/ [Os]
- **Funcionalidades clave:** kanban de **issues** que administra agentes de coding; workspace = **repo(s) + branch + dev server**; panel de **contexto** (cambios/logs/preview) [Os]/[Link].
- **Layout y navegación:** "The Workspaces UI uses a flexible **four-panel layout** designed for efficient AI-assisted development workflows" / "Understanding the Workspaces **four-panel layout and navigation**"; seleccionar un issue abre **panel de detalle** [Os]/[Link].
- **Estados y feedback:** **comentarios de diff inline enviados de vuelta al agente**; "the new bottleneck is **planning and review**" [Link].
- **Aprobaciones / HITL:** **approval workflows** en el panel de conversación para revisar planes del agente [Link].
- **Atajos de teclado:** [P] (no en la página de interface indexada).
- **Onboarding / vacío / error:** [P].
- **Accesibilidad:** [P].
- **Adoptar:** (a) **layout de 4 paneles** (sidebar + conversación + contexto + detalle) como referencia directa del Mission Control (RFC 65); (b) **workspace = repo+branch+dev-server** como contexto explícito (RFC 66 Context Rail); (c) comentarios inline→agente; (d) approval workflow en la conversación.
- **Adoptar / Evitar para Atlas:** adoptar el **layout de 4 paneles con contexto (dev server) explícito** y comentarios inline→agente; evitar la fragmentación de la conversación (RFC 66 §16).

## Notion — kanban / databases — prioridad A — [Os]

- **URL (en vivo):** https://www.notion.com/help/boards · /help/intro-to-databases [Os]
- **Funcionalidades clave:** "**Board view groups your database pages** by a…"; las **databases** son una de las piezas fundamentales de Notion [Os].
- **Layout y navegación:** **board (kanban)** como una vista de la misma base de datos (conmutables table/board/calendar) [Os]/[Link].
- **Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **vistas conmutables sobre un mismo modelo de datos** (board/list/calendar) — un modelo, varias proyecciones (refuerza el ViewSwitcher de RFC 65); evitar [P].

## Workflowy — kanban desde outline — [Os parcial]

- **Evidencia:** https://workflowy.com/ incluye **"Using Kanban"** (`/help/kanban-board`) y checklists como vistas del mismo árbol [Os]. **Adoptar:** derivar el kanban del mismo modelo de datos que el outline (una jerarquía, varias proyecciones).
