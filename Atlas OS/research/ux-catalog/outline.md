# ux-catalog / outline (árboles jerárquicos)

Fichas de referentes para la **Outline view** (árbol jerárquico + comentarios por bloque). No son entradas de RFC 62: son referencias transversales pedidas por el Project Lead (tercera pasada, tarea d).
Procedencia como en `terminal.md` ([Os]/[O]/[I]/[P]).

> **Pasada 4:** subidas a [Os] real las fichas con doc abierta en vivo; las no accesibles quedan [P] con motivo.

---

## Workflowy — outliner puro — prioridad (referencia transversal) — [Os]

- **URL (en vivo):** https://workflowy.com/help/ + https://workflowy.com/help/keyboard-shortcuts/ [Os]
- **Funcionalidades clave:** outliner de bullets; "**Write anything, anywhere.** Notes, tasks, ideas, plans. It all goes in the same place. Press `Enter` to add a new item." [Os].
- **Layout y navegación:** "**Nest to organize.** Press `Tab` to indent an item, making it a child of the one above. Go as deep as you like." + "**Zoom in to focus.** Click any bullet and it becomes your whole world. Everything outside it disappears. Click the **breadcrumb** at the top to zoom back out." [Os].
- **Estados y feedback:** "**Search to find anything.** Start typing and your document **filters in real time**. Edit right there in the results." [Os].
- **Aprobaciones / HITL:** [P] (no aplica).
- **Atajos de teclado:** tabla documentada — Zoom in `Alt+.`, Zoom out `Alt+,`, Jump to any item `Ctrl+K`, Indent `Tab` / Outdent `Shift+Tab`, Expand `Ctrl+↓` / Collapse `Ctrl+↑`, Expand or collapse `Ctrl+Space`, Complete `Ctrl+Enter` [Os].
- **Onboarding / vacío / error:** página "**Get started** — Everything you need to know to get going" + "**Four ideas that make it click**" [Os].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) **una jerarquía, sin carpetas** como eje de la Outline view (todo es un nodo colapsable); (b) **zoom a nodo + breadcrumb** como "entrar/salir" de un subárbol; (c) **atajos de expansión/colapso** (`Ctrl+↓/↑`, `Ctrl+Space`) y `Ctrl+K` para saltar a cualquier nodo — modelo directo para la Outline de RFC 24/65.
- **Evitar para Atlas:** la ausencia de estructura documental si Atlas necesita adjuntar artefactos (Atlas debe permitir nodos con payload).

---

## Zed — Outline Panel (código) — referencia transversal — [Os]

- **URL (en vivo):** https://zed.dev/docs/outline-panel + https://zed.dev/docs/configuring-zed + https://zed.dev/docs/key-bindings [Os]
- **Funcionalidades clave:** Zed expone **modal outline** y **outline panel**: "In addition to the modal outline (`cmd-shift-o`), Zed offers an outline panel." El panel se despliega con `cmd-shift-b` ("**outline panel: toggle focus**") o clicando el botón `Outline Panel` en la barra de estado; muestra símbolos con **prefijo de tipo** ("struct", "fn", "mod", "impl") [Os].
- **Layout y navegación:** en buffer singleton, clic en una entrada salta a la sección y **auto-scroll** a la posición del cursor; en **multibuffer** agrupa por archivo/carpeta (orden preservado), con **archivos borrados en strikethrough**; casos documentados: **Project Search Results**, **Project Diagnostics**, **Find All References** [Os]. (Config `outline_panel { default_width:300, dock:"right", git_status, indent_guides… }` en `configuring-zed` [Os].)
- **Estados y feedback:** **git status** por nodo del outline (estado del archivo visible en el árbol) [Os].
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** keymap totalmente customizable (`~/.config/zed/keymap.json`), `base_keymap` seleccionable (VS Code, JetBrains, Sublime…), **contextos** (`ProjectPanel && not_editing`) y **Keymap Editor** vía command palette (`zed: open keymap`) [Os].
- **Onboarding / vacío / error:** [P].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) **outline con estado git por nodo** (qué archivo cambió, visible en el árbol) — directamente aplicable a la Outline/Timeline de RFC 24/65; (b) **indent guides + dock configurable**; (c) **bindings por contexto** (un atajo distinto según el panel activo) como modelo de atajos del HUD.
- **Evitar para Atlas:** [P].

---

## Notion — outline de documento + comentarios por bloque — referencia transversal — [Os]

- **URL (en vivo):** https://www.notion.com/help/comments-mentions-and-reminders [Os]
- **Funcionalidades clave:** **top-level page discussions** + **inline page comments** (comentario por bloque) + **comments pane** + database comments + @-mentions + reactions [Os].
- **Layout y navegación:** "By default, page comments will appear **to the right of the corresponding text**. Longer comment threads will be collapsed."; **comments pane** (click `💬` at the top); modo `Customize page → Inline comments → Default | Minimal` [Os].
- **Estados y feedback:** "a **red circle** beside it if there are any **unread** comments"; resolver (`✔️`), re-abrir (`↪️`), filtrar por **persona o estado** (`Resolved`) [Os].
- **Aprobaciones / HITL:** comentarios/@-mentions como **colaboración asíncrona** (feedback humano anclado a un bloque) [Os].
- **Atajos de teclado:** "Use the shortcut **`cmd/ctrl` + `shift` + `M`** to make a comment on whatever you have selected"; `@` para mencionar [Os].
- **Onboarding / vacío / error:** [P].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) comentario **anclado a un bloque/nodo** (no al documento global) con **hilo colapsable a la derecha** — modelo directo para la Outline y los comments in-editor de RFC 24; (b) **comments pane** con filtro por estado (open/resolved) + badge de no-leídos — análogo a la ApprovalQueue; (c) atajo `cmd/ctrl+shift+M` para comentar en el cursor.
- **Evitar para Atlas:** [P].

---

## Obsidian — outline / headings — referencia transversal — [Os parcial]

- **URL (en vivo):** https://raw.githubusercontent.com/obsidianmd/obsidian-help/master/en/Plugins/Outline.md [Os] (la web `help.obsidian.md` es JS-rendered y devolvía cuerpo vacío; el **repo** sí).
- **Funcionalidades clave:** "Outline is a **core plugin** that **lists the headings in the active note**." [Os].
- **Layout y navegación:** "To navigate to that section in the note, **click on the heading** in the outline." [Os].
- **Estados y feedback:** [P].
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** es un **core plugin** (se activa/desactiva desde Core plugins) [Os] — detalle de la UI de ajustes [P].
- **Accesibilidad:** [P].
- **Interacción:** "To **rearrange sections** in the note, **click and drag the heading** within the outline." [Os].
- **Adoptar para Atlas:** (a) outline como **lista de headings del documento activo** (no un árbol de archivos): el nodo es una **sección**, y **drag** reordena el documento real — modelo "outline = estructura del artefacto"; (b) outline como **core plugin** conmutable (Atlas: view activable).
- **Evitar para Atlas:** [P].
- **Nota [Os parcial]:** la página del plugin es muy corta (2 campos concretos: funcionalidades + navegación/interacción); no llega a 4 → **[Os parcial]** honesto.

---

## DeerFlow — parsing de outline Markdown (bonus, código) — [P]

- **Motivo [P]:** es un **repo de código** (`https://github.com/bytedance/deer-flow`), **sin documentación de UI**; el patrón observado (reconocer ATX headings, ignorar code fences, invalidar por `mtime`) es de implementación, no de interfaz → no cumple el mínimo de campos UI. [Os] la URL, pero **no** es [Os] de UX.
- **Adoptar para Atlas (a validar):** reglas **explícitas y testables** para construir el outline de Markdown (qué cuenta como sección) — evita nodos basura en el árbol; extraído del código, no de doc de UI.
- **Evitar para Atlas:** [P].
