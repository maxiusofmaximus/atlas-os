# 21 - Execution Modes

> RFC que define el catálogo de **modos de ejecución** y **cómo se traba cada uno**. Un execution mode controla **cuán autónomo es el agente** — es ortogonal a `permissions` (que controla **qué puede hacer**). Las reglas para que "no cometa locuras" difieren por modo.

---

## 0. Por qué este RFC existe

El documento original del usuario (las conversaciones que generaron esta especificación) pidió explícitamente tres modos:

1. Un modo en el que **la IA hace todo siguiendo las reglas**, fundamentado, sin que tengamos que intervenir.
2. Un modo en el que **la IA pide permiso** para cada acción sensible, el usuario lee lo que hace y lo que va a suceder.
3. Un **modo editor como antaño** funcionaba Visual Studio / Apache NetBeans / VS Code: el usuario escribe, el editor autocompleta según la extensión del archivo.

A esos tres se añade un cuarto modo derivado del estudio de Cursor y OpenCode Desktop: **Autopilot**, que ocupa el punto medio entre "pido permiso siempre" y "loop autónomo" — el agente trabaja pero pausa cada N pasos a pedir confirmación.

> **Filosofía** (Nate Gentile, vídeo *Cursor con Múltiples Agentes*, min 12–14; *OpenCode Desktop*, min 20–22):
> *"No se trata de una IA asistida por humanos, sino de un humano asistido por IA."*
>
> La IA es buena para la laboriosidad repetitiva y lo mecánico. **No** tiene creatividad ni pensamiento lateral. Por eso dejamos al humano en el centro: viendo, decidiendo y bifurcando. Los cuatro modos son el dial (el *"autonomy slider"* que Karpathy describe en la home de Cursor).

---

## 1. Catálogo de modos

| Modo | Agente activo | _ Dial de autonomía _ | Default para |
|---|---|---|---|
| `MANUAL_CLASSIC` | ningún agente | 0 — puro editor | `.lock`, `.min.*`, assets generados |
| `HUMAN_IN_LOOP` | `plan` + asks | 25% — pausa tras cada acción sensible | código de producción |
| `AUTOPILOT` | `build` + checkpoints | 65% — pausa cada N steps | features nuevas end-to-end |
| `AUTONOMOUS` | swarm full + budget | 100% — loop hasta success | runs largos, tareas programadas |

Cada sesión tiene **exactamente un** `mode` activo. Los subagentes heredan el mode del padre en spawn salvo override en `permission.task`.

---

## 2. `MANUAL_CLASSIC` — Editor de antaño

> *"El modo editor como antaño funcionaba Visual Studio, Apache NetBeans y Visual Studio Code con autocompletado por extensión del archivo."* — documento original.

### 2.1 Agente
Ninguno. Cero *agency*. El usuario escribe todo.

### 2.2 Mecanismo de autocompletado
El `LanguageIdResolver` (`03 - Engine Architecture.md` §9.3) enruta cada archivo a su Language Server por extensión:

| Extensión | LanguageId | Language Server |
|---|---|---|
| `.ts/.tsx` | typescript | typescript-language-server (tsserver) |
| `.py` | python | pylsp / pyright / jedi-language-server |
| `.rs` | rust | rust-analyzer |
| `.go` | go | gopls |
| `.c/.cpp/.h` | c cpp | clangd |
| `.java` | java | eclipse.jdt.ls |
| `.cs` | csharp | OmniSharp / C# Dev Kit |
| `.php` | php | intelephense |
| `.rb` | ruby | solargraph |
| `.vue` | vue | Volar |
| `.md` | markdown | marksman |
| default | plaintext | (sin server) |

ES el patrón de VS Code (`files.associations`) — emulado fiel y configurable vía `.opencode/languages.json`.

### 2.3 Servicios LSP activos en este modo
Los mismos que VS Code expone sin extensiones IA:
- `textDocument/completion` + `completionItem/resolve`
- `textDocument/hover`
- `textDocument/signatureHelp`
- `textDocument/definition` / `references` / `rename`
- `textDocument/documentSymbol` / `workspace/symbol`
- `textDocument/diagnostics` (publicDiagnostics)
- `textDocument/formatting` / `rangeFormatting`

El **ProjectSymbolTable** (`03 - Engine Architecture.md` §9.4) se actualiza incrementalmente en cada `textDocument/didChange` — no espera al save (es el patrón IntelliSense / Roslyn Workspace / rust-analyzer salsa).

### 2.4 Permissions effectivos
Solo `read`, `lsp`, `grep`, `glob`, `list`. Cualquier tool tipo `edit`, `bash`, `webfetch` está bloqueado en este modo — el agente no puede "sorprender" al usuario.

### 2.5 Failure mode que evita
El clásico *"auto-agentic surprise"*: el usuario solo quiere completar un símbolo y el agente le reescribe el archivo. En `MANUAL_CLASSIC`, ningún agente está vivo.

### 2.6 Default
Para archivos: `.lock`, `.min.*`, assets generados (`*.generated.*`), `dist/`, `build/`, `node_modules/`. Para cualquier workspace recién abierto: si el usuario cambia a modo clásico, queda.

---

## 3. `HUMAN_IN_LOOP` — La IA pide permiso

> *"El modo en el que la IA va pidiendo permiso para realizar ciertas acciones, el usuario lee lo que está haciendo y lo que va a suceder a continuación."* — documento original.

### 3.1 Agente
Estilo `plan` agent de OpenCode: read-only por defecto; `edit`, `bash`, `task` disparan `ask`.

### 3.2 Elementos del diálogo de aprobación
Cada acción sensible muestra al usuario:
- **Qué hace** (verbo + objeto, p.ej. "Edit `src/api/routes.ts:42`").
- **Diff previsto** (exactamente las líneas que se añadirán/quitarán).
- **Lo que va a suceder a continuación** (los próximos 1–3 pasos del Plan).
- **Evidencia** (Research Run refs, Confidence Score, skill que acabamos de disparar).
- **Botones**: `once | always (este patrón) | reject | ver plan completo`.

### 3.3 Iteraciones máximas
- soft warn a 50 iteraciones,
- hard-stop a 100.

### 3.4 Caso típico
Refactor de código de producción, migración de schema DB, planificación arquitectónica.

### 3.5 Failure mode que evita
El bug "Rest of code here" de Cline (wiki): el LLM inserta un comentario `// rest of code here` y borra el resto del archivo. En `HUMAN_IN_LOOP` el humano siempre ve el diff antes de apply, así que cualquier borrado accidental se rechaza.

---

## 4. `AUTOPILOT` — Checkpoints periódicos

### 4.1 Agente
Estilo `build` agent de OpenCode con `edit:allow`, `bash:allow`, pero **con checkpoints obligatorios cada N steps**.

### 4.2 N configurable
Por defecto `N=10`. Tras 10 steps:
- el agente pausa,
- genera un summary (qué hizo, qué diff produjo, siguiente step propuesto),
- pide `continue?` con botones `continue | fork branch | change mode | stop`.

### 4.3 Iteraciones máximas
100 (suave), 250 (hard).

### 4.4 Compaction automática
Tras 50k tokens consumidos se dispara el subagente `compaction`. El usuario ve un aviso "compacting context…".

### 4.5 Caso típico
Feature nueva end-to-end, migration entre versiones, refactor grande respaldado por Research Run.

### 4.6 Failure mode que evita
El "yolo mode" de Cline sin pausa visible. Aquí el usuario recupera el control periódicamente sin tener que mirar el panel en cada acción.

---

## 5. `AUTONOMOUS` — La IA hace todo con reglas

> *"Un modo en el que la IA hace todo siguiendo las reglas para que no cometa locuras, pero implemente de forma bien fundamentada sin que tengamos que hacer algo."* — documento original.

### 5.1 Agente
Estilo `build` + swarm + loop hasta `success_predicate`.

### 5.2 Pre-requisitos para activarlo
El modo `AUTONOMOUS` **no se entra sin definir presupuesto**: al activarlo, el UI obliga a configurar:
```
max_iterations:  = 250
max_minutes:     = 30 (default)
max_cost_usd:     = 1.50 (obligatorio)
success_predicate = "tests_exit_zero AND lint_clean AND knip_clean"
```
Sin `max_cost_usd` el modo no arranca. Sin `success_predicate` el modo no arranca.

### 5.3 Reglas duras que el agente respeta
Incluso en `AUTONOMOUS` el agente está sujeto a:
- `DoomLoopDetector` con **hard-deny** (no `ask`) tras 3 reps idénticas → downgrade automático a `AUTOPILOT`.
- `GoalTracker` (`goal_drift`) tras 5 iter sin progress → escalar al humano.
- `max_context_compactions = 2` tras las cuales se escapa al humano (no se compacta infinitamente).
- Aprobaciones de acciones sensibles (`Pulumi apply`, drop schema, borrar archivo >50KB, push remoto) exigen `confirm` aunque el modo sea `AUTONOMOUS`.
- Skills con `verified=false` no corren en `AUTONOMOUS`.
- Modelos con `tier=paid` requieren que el presupuesto sea suficiente; si no, el orchestrator cae a `free-only` para esa mission.

### 5.4 Subagents paralelos
Hasta 3 hijos `general` por defecto (configurable hasta 10). Cada uno hereda el `mode` y trabaja en su propio worktree.

### 5.5 Caso típico
Run largos (3–30 min), tareas programadas vía cron, branches de prueba, cleanup nocturno.

### 5.6 Donde **se diferencia de los demás editores**
- vs Cursor Cloud Agents: nosotros corremos en local o en tu propio VPS, no en una VM cerrada de Cursor.
- vs Cline YOLO: nosotros tenemos `doom_loop` hard-deny, no solo `ask`.
- vs OpenCode `--auto`: nosotros tenemos budget hard + compaction limit + goal_drift (no solo `steps`) max).
- vs Aider `/auto`: nosotros integramos Research Engine + capabilidad de swarm, no solo un solo modelo en bucle.

### 5.7 Failure mode que evita
Infinite loop, cost runaway, context explosion, "Rest of code here", flip-flop, race conditions entre subagents, goal drift. Ver tabla §8.

---

## 6. Transiciones de modo

```
MANUAL_CLASSIC ──(prompt en chat / Start IA)──▶ HUMAN_IN_LOOP
HUMAN_IN_LOOP ──(:autopilot o ⌘K M)──────▶ AUTOPILOT
AUTOPILOT     ──(:autonomous + budget)────▶ AUTONOMOUS
cualquiera    ──(doom_loop hard-deny)─────▶ downgrade un nivel
cualquiera    ──(goal_drift persistente)──▶ downgrade un nivel
cualquiera    ──(presencia de agente activo)─┐
                                           └▶ no se puede volver a MANUAL_CLASSIC hasta que libresen
```

### 6.1 Downgrade seguro
El downgrade no aborta el trabajo en curso — se preserva en un checkpoint del Journal y se reanuda en el modo inferior.

---

## 7. Persistencia del mode

- `mode` se persiste en `session.state.mode` (SQLite).
- `mode` **no** se hereda a subagents salvo `permission.task` que lo permite.
- `tools.cmp(mode_old, mode_new)` produce un evento de auditoría.
- El modo de cada agente es visible en el status bar (`17 - UI.md`): 🕊 / 🤝 / 🛫 / 🚀.

---

## 8. Mapa failure modes → mitigación

| Failure mode conocido en otros editores | Mitigación en OpenCode OS |
|---|---|
| Doom loop mecánico (Aider, Cline) | `DoomLoopDetector` trip tras 3 reps → `ask`/`hard_deny` según mode |
| Cost runaway (Cursor Cloud Agents) | `max_cost_usd` obligatorio en `AUTONOMOUS` |
| "Rest of code here" borrando archivo (Cline wiki) | `HUMAN_IN_LOOP` por defecto en código prod; diff siempre visible |
| Flip-flop entre dos estados | `goal_drift` decay detector |
| Race conditions entre subagents | worktree propio por subagent + file locks |
| Context explosion | `max_context_compactions = 2` + subagente `compaction` |
| Goal drift en loops largos | `GoalTracker.last_progress_step` + recovery prompt |
| Auto-agentic surprise | `MANUAL_CLASSIC` para data/locks/assets; cero agente |
| Skill no verificada causando daño | `verified=false` bloqueada en `AUTONOMOUS` |
| Modelo pagado sin presupuesto | orchestrator cae a `free-only` si budget insuficiente |

---

## 9. Defaults por tipo de archivo

El `CapabilityResolver` (`02 - Agent Operating System.md`) sugiere el mode al abrir o crear archivos:

```yaml
file_defaults:
  "*.lock":                   MANUAL_CLASSIC
  "*.min.*":                  MANUAL_CLASSIC
  "dist/**":                  MANUAL_CLASSIC
  "build/**":                 MANUAL_CLASSIC
  "node_modules/**":          MANUAL_CLASSIC
  "src/**/*.generated.*":     MANUAL_CLASSIC
  "src/**/*.test.*":          AUTOPILOT              # tests OK para auto-write
  "src/**/prod/**":           HUMAN_IN_LOOP          # producción pide permiso
  "infrastructure/**/*.tf":   HUMAN_IN_LOOP
  "infrastructure/**/*.ts":   HUMAN_IN_LOOP
  "docs/**/*.md":             AUTOPILOT
  "scratchpad/**":            AUTONOMOUS             # zona libre
```

Override del usuario vía `.opencode/modes.json`.

---

## 10. Configuración del modo por defecto

```yaml
# .opencode/modes.json
{
  "default_mode": "HUMAN_IN_LOOP",
  "default_resource_mode": "mixto",
  "max_concurrent_subagents": 3,
  "ask_patrons": {
    "edit":   "ask",
    "bash":   "ask",
    "task":   "ask",
    "webfetch": "auto"
  },
  "autonomous_request": {
    "max_cost_usd_per_mission": 1.50,
    "max_minutes_per_mission": 30,
    "require_success_predicate": true
  }
}
```

---

## 11. Comparativa rápida

| Editor | Loop autónomo | Detector doom loop | Budget hard | Modo manual clásico | Pedir permiso | Autopilot checkpoints |
|---|---|---|---|---|---|---|
| VS Code + Copilot | ❌ | ❌ | ❌ | ✅ (LSP) | ⚠️ (diff) | ❌ |
| Cursor | ⚠️ Cloud Agent | no público | no expuesto | ⚠️ | ⚠️ | ⚠️ Automations |
| Claude Code | ⚠️ /loop user-mode | ❌ | ❌ | ❌ | ⚠️ | ❌ |
| OpenCode (CLI) | ⚠️ /loop custom | ✅ (`doom_loop`) | ⚠️ steps | ⚠️ inline | ✅ (ask) | ❌ |
| Cline / Roo | ⚠️ YOLO | ❌ | ❌ | ❌ | ✅ | ❌ |
| Aider | ⚠️ /auto, architect mode | ❌ | ❌ | ❌ | ⚠️ verify | ❌ |
| Hermes | ✅ background | ❌ | ❌ | ❌ | ✅ | ❌ |
| Continue.dev | ⚠️ ambient | ❌ | ❌ | ✅ (LSP) | ❌ | ⚠️ |
| **OpenCode OS** | ✅ AUTONOMOUS | ✅ hard-deny mecánico | ✅ obligatorio | ✅ MANUAL_CLASSIC + LSP | ✅ HUMAN_IN_LOOP | ✅ AUTOPILOT |

---

## 12. Eje ortogonal: `Modo de uso` (ask / architect / code / context)

Los 4 *Execution Modes* describen **cuán autónomo** es el agente. Existe un eje **ortogonal** que describe **cómo se interpreta y razona el prompt**: el *Modo de uso*, definido en el RFC `23 - Prompt Understanding & Refinement.md` §8.

| `modo_uso` | Confidence requerido | Significado |
|---|---|---|
| `ask` | LOW / BLOCK | readOnly; bloqueado por clarification loop; no edita archivos. |
| `architect` | MEDIUM | Architect model propone, editor model ejecuta (patrón Aider `/architect`). |
| `code` | HIGH o `locked=true` | Execution normal. |
| `context` | (lost in the middle detectado) | Reordena contexto antes de enviar al Planning. |

### 12.1 Combinaciones válidas

Execution Mode × Mode de uso son **ortogonales**. Ejemplos:

| Execution Mode | Modo uso | ¿Tiene sentido? |
|---|---|---|
| `MANUAL_CLASSIC` | `ask`/`architect`/`code`/`context` | NO — `MANUAL_CLASSIC` no tiene agente, no hay nada que interpretar. El modo de uso se ignora. |
| `HUMAN_IN_LOOP` | `ask` | ✅ — agente propone un plan, el usuario lo aprueba antes de Coding. |
| `HUMAN_IN_LOOP` | `architect` | ✅ — architect model redacta, el usuario aprueba diff por diff. |
| `HUMAN_IN_LOOP` | `code` | ✅ — por defecto del modo cuando el verdict salió HIGH. |
| `AUTOPILOT` | `architect` | ✅ — el agente para cada N steps pero el architect model sigue proponiendo. |
| `AUTONOMOUS` | `code` | ✅ — modo más común para runs largos nocturnos. |
| `AUTONOMOUS` | `context` | ✅ — si el verdict detectó `lost_in_the_middle`, se reordena y se lanza el loop autónomo. |
| `AUTONOMOUS` | `ask` |❌ — si hay que preguntar, no es autónomo. El modo hace downgrade automático a `HUMAN_IN_LOOP`. |

### 12.2 Handoff `23 → 21`

El Prompt Understanding Pipeline emite `mission.modo_uso_sugerido`. Si al activar un Execution Mode se detecta conflicto (véase tabla), el Kernel Bus aplica el downgrade seguro (no aborta el workflow, lo preserva en un checkpoint y restaura en el modo menos autónomo compatible).

### 12.3 Visualización en el HUD

Cada card de subagente muestra **ambos** badges (RFC `24 - HUD Mission Control.md` §3.1):

```
Execution mode: 🤝 HUMAN_IN_LOOP
Modo uso:         🏛 architect
```

Si el usuario cambia el `modo_uso` desde el HUD (`:o architect` atajo), el cambio es persistido en `session.state.modo_uso` y se propaga a subagents salvo override.

---

## 13. Estado

- Status: Draft v1
- Depends on: `02 - Agent Operating System.md`, `03 - Engine Architecture.md`, `17 - UI.md`, `19 - Execution Supervisor.md`, `23 - Prompt Understanding & Refinement.md`, `24 - HUD Mission Control.md`
- Cubre: el pedido explícito del documento original de los modos IA-autónoma / IA-pedir-permiso / editor-clásico. Añade el eje ortogonal `modo de uso` hermano de Aider.
