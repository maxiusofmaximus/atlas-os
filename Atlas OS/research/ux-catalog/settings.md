# ux-catalog / settings (paneles de configuración)

Fichas de referentes para la pantalla **Settings** (configuración de IDE/agentes). Referencias transversales pedidas por el Project Lead (tercera pasada, tarea d).
Procedencia como en `terminal.md`.

> **Pasada 4:** subidas a [Os] real las fichas con doc abierta en vivo; las que no llegan a 4 campos quedan [Os parcial] (honesto).

---

## VS Code — Settings (editor + JSON) — prioridad (referencia transversal) — [Os]

- **URL (en vivo):** https://code.visualstudio.com/docs/getstarted/settings [Os]
- **Funcionalidades clave:** Settings editor con **vista GUI y `settings.json`**; se abre el JSON con el comando de paleta **"Preferences: Open Application Settings (JSON)"** (⇧⌘P / Ctrl+Shift+P); settings a nivel **usuario y workspace** [Os].
- **Layout y navegación:** **Settings editor** (UI con buscador) + **JSON subyacente**; panel de **Profiles** para conjuntos de customización conmutables [Os].
- **Estados y feedback:** **Settings Sync** comparte settings, atajos y extensiones entre máquinas (activado desde "Backup and Sync Settings" o el menú de Accounts) [Os].
- **Aprobaciones / HITL:** [P] (no aplica).
- **Atajos de teclado:** Command Palette ⇧⌘P / **Ctrl+Shift+P** para abrir settings JSON [Os].
- **Onboarding / vacío / error:** [P] (la doc cubre ubicación de archivos por SO: Windows `%APPDATA%\Code\User\profiles\<id>\settings.json`, etc.) [Os].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) **dual UI (form) + JSON** — el usuario puede editar visualmente o por texto (clave para una config de agente potente); (b) **Profiles** con settings scoped y **switch** rápido — paralelo a los perfiles de Atlas (RFC 25 §4); (c) **Settings Sync** opt-in (settings+atajos+extensiones).
- **Evitar para Atlas:** sincronizar extensiones en ventanas **remote/devcontainer/WSL** (VS Code no lo hace) — cuidado con el modelo remoto de Atlas.

---

## Cursor — Settings / Customize — prioridad (referencia transversal) — [Os]

- **URL (en vivo):** https://docs.cursor.com/en/settings [Os]
- **Funcionalidades clave:** hub **Customize** con **Plugins, Rules, Skills, Subagents, Hooks, MCP** en un solo lugar; **Command Palette** de búsqueda de comandos [Os].
- **Layout y navegación:** navegación lateral por secciones (Get Started, Agent, Customize…) + **Command Palette**; incluye "What you can do with Cursor" (understand code, plan/build, fix bugs, review, customize, connect workflow) [Os].
- **Estados y feedback:** [P].
- **Aprobaciones / HITL:** [P] (rules/hooks configuran comportamiento; detalles [P]).
- **Atajos de teclado:** **Command Palette** (búsqueda de comandos) [Os]; combinación concreta [P].
- **Onboarding / vacío / error:** sección **Get Started → Quickstart** [Os].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) **agrupar toda la customización (Rules/Skills/MCP/Hooks/Subagents) en un hub** — modelo directo para el panel de configuración de Atlas (skills/MCP de RFC 06/07/65); (b) Command Palette como acceso universal.
- **Evitar para Atlas:** [P].

---

## Zed — Settings (`settings.json` completo) — prioridad (referencia transversal) — [Os parcial]

- **URL (en vivo):** https://zed.dev/docs/configuring-zed + https://zed.dev/docs/key-bindings [Os]
- **Funcionalidades clave:** **referencia completa de settings** en `~/.config/zed/settings.json`; ejemplo documentado con `theme`, `tab_size`, `buffer_font_size/family`, `autosave` (`on_focus_change`), `format_on_save`, `vim_mode`, **terminal** (font/blinking) y **languages** (override por lenguaje: formatter, line length, soft_wrap) [Os]. Incluye el **Outline Panel** (ver `outline.md`) [Os].
- **Layout y navegación:** archivo JSON único con secciones y **overrides por lenguaje**; **settings window** GUI (`cmd-,`/`ctrl-,`) además del JSON (`cmd-alt-,`/`ctrl-alt-,`) [Os].
- **Estados y feedback:** [P].
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** keymap customizable (`keymap.json`), **`base_keymap`** seleccionable (VS Code, JetBrains, Sublime…), y **Keymap Editor** (`cmd-k cmd-s` / `zed: open keymap`) [Os].
- **Onboarding / vacío / error:** [P].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) **JSON de config con overrides por contexto** (por lenguaje/por proyecto) — modelo para overrides por misión/skill; (b) defaults documentados uno-a-uno (buena trazabilidad de config); (c) **base_keymap conmutable** (elegir el esquema de atajos del usuario) — útil para el HUD.
- **Evitar para Atlas:** [P].
- **Nota [Os parcial]:** 3 campos concretos (funcionalidades, layout, atajos); el resto no está en la doc abierta → no se eleva a [Os] real.

---

## Warp — Appearance / Settings — prioridad (referencia transversal) — [Os]

- **URL (en vivo):** https://docs.warp.dev/terminal/appearance/themes (+ hub `/terminal/appearance`, con subpáginas `text-fonts-cursor`, `custom-themes`, `prompt`, `input-position`, `size-opacity-blurring`, `pane-dimming`, y `more-features/accessibility`) [Os]
- **Funcionalidades clave:** "Customize Warp's visual appearance, including **themes, fonts, prompts, app icons, input position, and pane behavior**" [Os].
- **Layout y navegación:** **theme picker** por flujo documentado — "**Settings > Appearance** → click the **Custom Themes** box → select a theme" [Os].
- **Estados y feedback:** "The Theme setting **persists**, meaning Warp will open with the same settings in the next session"; checkmark guarda / X revierte [Os].
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** [P].
- **Accesibilidad:** "**OS theme sync** … synchronizing your theme with the OS's **light and dark** themes … select a specific theme for when the OS is in light mode and dark mode" [Os].
- **Adoptar para Atlas:** (a) el conjunto **themes/fonts/prompts/input-position/pane-behavior** como checklist de apariencia configurable; (b) **sync con el tema del SO (light/dark)** como requisito de accesibilidad del HUD (hoy RFC 65 §6 lo da por hecho y **no** existe — ver `docs/audit/frontend-current-state.md` §(e)); (c) confirmar/descartar con checkmark/X (feedback explícito de cambio).
- **Evitar para Atlas:** [P].
