# Inventario del estado actual del frontend — Atlas OS

**Builder-2 · 2026-10-06 · solo lectura** (entrada para RFC 67 - UI Specification).
Alcance (a)–(g) del plan `docs/coordination/PLAN-docs-hardening.md`. Nada de esto toca `src/` ni `src-tauri/`.

**Metodología y procedencia.** Toda la evidencia fue observada esta sesión [O]: `git log`/`git status`, `Get-ChildItem`, análisis con regex sobre `src/**` (reproducible con los comandos citados por sección) y lectura directa de ficheros. Lo marcado [I] es inferencia razonada a partir de lo observado; [P] = pendiente de confirmar. Ninguna cifra sin comando o lectura que la respalde.

- Verificación ejecutada al cierre: `pnpm check` → **0 errores, 0 warnings** [O]; `pnpm test` → **4 test files, 116 tests passed, 2.06s** (vitest 4.1.10) [O].

---

## (a) Rutas y layouts de `src/`

| Fichero | Líneas | Contenido [O] |
|---|---|---|
| `src/routes/+layout.svelte` | 8 | Shell raíz: `const { children } = $props()` (línea 4) + `{@render children()}` (línea 6). El comentario (línea 3) admite: "Design tokens (CSS file) loaded at +page level until deps installed". |
| `src/routes/+layout.ts` | 7 | `ssr = false` (línea 3), `prerender = false` (línea 4), `trailingSlash = 'ignore'` (línea 5) — CSR-only para Tauri (RFC 25 §3.3). |
| `src/routes/+page.svelte` | 1237 (36.198 bytes) | Única página: la HUD Mission Control completa. |
| `src/routes/+page.ts` | 19 | `load()` (línea 8): `hudUrl` via Tauri `invoke('hud_url')` (línea 13); fuera de Tauri queda `undefined` (línea 18) y la HUD store no conecta. |
| `src/app.d.ts` | 25 | Tipos ambientales. |
| `src/app.html` | 14 | Plantilla HTML. |
| `src/app.css` | — | **ABSENT** [O]. No existe fichero de tokens (ver (e)). |

No hay rutas hijas, layouts anidados ni `+error.svelte` en `src/routes/` [O: `Get-ChildItem -Recurse src/routes`]. [I] La navegación no usa el router de SvelteKit: es un panel CSR único que intercambia secciones con `{#if}/{:else if $activeView}` en `+page.svelte:386-502`.

---

## (b) Componentes de `src/lib` (24 `.svelte`)

Recuento [O]: `Get-ChildItem src/lib/components -Name` → 24 componentes `.svelte` + 4 ficheros de test. Fuentes de datos = función de `src/lib/stores/hud.ts` o `views.ts` que consume, con el endpoint real que esa función golpea (mapeo completo en (c)). `props@N` = línea donde se desestructura `$props()`; `err@N` = primera línea de la UI de error. Todos son Svelte 5 runes (`$props()`, `$state`, `$derived`, `$effect`) [O: lectura directa].

| Componente | Líneas | Props | Stores | Datos / endpoints que consume | Poll |
|---|---|---|---|---|---|
| `AgentCard.svelte` | 253 | `hudUrl` @23 | `$hud` (sub. WS) | `$hud.events` → proyección `agent_step` (RFC 63 §9); `fetchRemoteStatus` (→ `GET /remote/status`, línea 58-60 en OnMount) | OnMount@58 + setInterval@60 |
| `ApprovalQueue.svelte` | 180 | `hudUrl` @13 | `$hud` | `approveApproval`/`denyApproval` → `POST /hud/approvals/:id/{approve,deny}` (proyección sobre `$hud.events`) | err@48 (sin loading) |
| `AuditTimeline.svelte` | 152 | `hudUrl` @14 | `hud.ts` | `fetchAudit` @23 → `GET /hud/audit` | OnMount@29 + setInterval@31 |
| `AutoresearchCard.svelte` | 300 | `snapshot`, `candidates`, `hudUrl` @23 | `hud.ts` | `postAutoresearchCancel` @63 → `POST /autoresearch/cancel` | err@168 (sin loading) |
| `AvailabilityCard.svelte` | 162 | `hudUrl` @14 | `hud.ts` | `fetchAvailability` @23 → `GET /hud/availability` | OnMount@29 + setInterval@31 (15 s); loading@45, err@63 |
| `CanvasView.svelte` | 100 | `hudUrl` @14 | `hud.ts` | `fetchMissions` @24 → `GET /tail/missions`; dentro renderiza `GraphView` (→ `GET /graph/:id`, requiere feature `dag_mode` en el backend — `server.rs:119-120`) | OnMount@31; err@50 (sin loading) |
| `CommandPalette.svelte` | 188 | `open`, `available`, `onclose`, `onaction` @15 | `views.ts` (`VIEWS`, `activeView`) | — (acciones: `refresh` → `window.location.reload()` en `+page.svelte:381-383`, `export` → acción no cableada) | — |
| `CostDashboard.svelte` | 294 | `hudUrl` @14 | `hud.ts` | `fetchCost` @28 → `GET /hud/cost` | OnMount@36 + setInterval@38; loading@21, err@65 |
| `DemoPane.svelte` | 200 | `hudUrl` @15 | `hud.ts` | `fetchDemos` @26 → `GET /hud/demos` | OnMount@35 + setInterval@37; loading@19, err@60 |
| `EvalCard.svelte` | 217 | `hudUrl` @14 | `hud.ts` | `fetchEvalSummary` @23 → `GET /hud/eval/summary` | OnMount@29 + setInterval@31; err@55 (sin loading) |
| `GraphView.svelte` | 247 | `hudUrl`, `missionId` @22 | `hud.ts` | `fetchGraph` @36 → `GET /graph/:id` | loading@25, err@92 |
| `HealthKPIs.svelte` | 297 | `hudUrl` @21 | `hud.ts` + `$hud` | `fetchHealth` @38 → `GET /hud/health`; liveness de supervisor desde el bus `agent_heartbeat` (`$hud.events`) | OnMount@44 + setInterval@46; err@68 (sin loading UI dedicado) |
| `JournalObserver.svelte` | 346 | `hudUrl` @17 | `hud.ts` | `fetchJournalPage` @42 → `GET /hud/journal` | OnMount; loading@27, err@133 |
| `KanbanBoard.svelte` | 214 | `hudUrl` @20 | `hud.ts` | `fetchMissions` @53 → `GET /tail/missions` (Kanban Pending/Running/Done/Failed) | OnMount@59 + setInterval@61; err@77 (sin loading) |
| `McpView.svelte` | 395 | `hudUrl` @22 | `hud.ts` | `fetchMcp` @39 → `GET /hud/mcp`; `saveMcpAllowlist` @95 → `POST /hud/mcp/allowlist` | OnMount@52; loading@32, err@134 |
| `ModelReadyCard.svelte` | 141 | `payload`, `hudUrl` @19 | `hud.ts` | `postMissionResume` @33 (resume de mission tras reset de modelo) | err@78 |
| `OutlineView.svelte` | 175 | `hudUrl` @13 | `hud.ts` | `fetchPayload` @48 → `GET /payload/plan/:id` (planes vía `GET /tail/plans`) | OnMount@58; loading@25, err@82 |
| `SettingsView.svelte` | 232 | `hudUrl` @14 | `hud.ts` | `fetchSecretSlots` @26 → `GET /hud/secrets`; `postSecret` @43 → `POST /hud/secrets`; `deleteSecret` @59 → `DELETE /hud/secrets/:account` | OnMount@67; err@101 |
| `SkillMcpRail.svelte` | 308 | `hudUrl` @23 | `hud.ts` | `fetchMcp` @39 → `GET /hud/mcp`; `postActivateSkill` @67 → `POST /hud/skills/:id/activate` | OnMount@45; err@87 (sin loading) |
| `SpendLimitErrorCard.svelte` | 215 | `payload`, `backupProfileId`, `hudUrl` @25 | `hud.ts` | `postProfileSwitch` @84 (switch de perfil tras spend-limit) | err@143 |
| `SwarmConsole.svelte` | 588 | `hudUrl`, `missionId`, `agents`, `messages` @20 | `hud.ts` | `fetchSwarmChecks` @91; `postSwarmSend` @123 → mailbox swarm | loading@24, err@182 |
| `TimelineView.svelte` | 137 | `hudUrl` @12 | `hud.ts` | `fetchJournalPage` @22 → `GET /hud/journal` | OnMount@30 + setInterval@32; err@50 (sin loading) |
| `ViewSwitcher.svelte` | 72 | `available` @14 | `views.ts` (`activeView`) | — | — |
| `WorktreesView.svelte` | 176 | `hudUrl` @14 | `hud.ts` | `fetchWorktrees` @26 → `GET /hud/worktrees` | OnMount@34; loading@19, err@53 |

**`+page.svelte` como orquestador** [O]: importa 22 componentes (líneas 26-48); define `availableViews` con los 13 ids (321-335); registra el listener global de teclado con `$effect` (352-355); en `onMount` conecta el WS (`hud.connect(data.hudUrl)`, 282), pide `/remote/status` (283) y arranca un poll global de 5 s (`refreshAll`, 288); `swarmAgents`/`swarmMissionId`/`swarmMessages` se derivan del stream WS (`$derived`, 92-94). El branch default (Overview, `{:else}` 503+) monta además: HUD health (504), Skill & MCP rail (518), audit export form (528-585, estado `exportState` con busy/error/result), AutoresearchCard (587), SwarmConsole (596), AvailabilityCard (611), EvalCard (621), journal tail (630), JournalObserver (647) y la grid `tails` (657-684) con 11 tail boxes (`tailKinds`, 64-76) cada uno con estado propio `{ rows, loading, error }` (78-85).

---

## (c) Stores y su forma de datos

Dos ficheros en `src/lib/stores/` (ni router ni librerías de estado — `svelte/store` writable) [O].

### `src/lib/stores/hud.ts` — 1585 líneas, 149 exports [O]

- **`HudEvent`** (18-23): `{ id: string; ts: string; kind: string; payload: unknown }` — forma normalizada de cada frame WS.
- **`HudState`** (25-29): `{ connected: boolean; url: string | null; events: HudEvent[] }`.
- **Store `hud`** (409-425): `Readable<HudState>` + `connect(target)` / `disconnect()`. Reconexión con backoff exponencial `×2`, cap 10 s (`scheduleReconnect`, 401-407). El evento llega por `WebSocket` y se normaliza con `normalizeWsEvent` (345-338) — acepta tanto el frame del bridge `{ id, kind: { type, ... }, ts }` como el plano `{ kind, payload }`.
- **Proyecciones sobre `$hud.events`** (consumidas por componentes): `projectSwarmAgents` (+page.svelte:92), `projectPendingApprovals` (1152), `projectLatestHeartbeat` (1295), `projectKanban`-class sobre `fetchMissions`.
- **REST helpers** (todas `export async function`, golpean `${trimmed}<path>` con `trimmed = hudUrl.replace(/\/$/, '')`):

| Función (línea) | Endpoint [O] |
|---|---|
| `fetchAnnotations` (103) / `postAnnotation` (112) | `GET/POST /diff/:id/annotation` |
| `fetchTail<T>` (204) | `GET /tail/{kind}` — genérico, 14 kinds (`TailKind`, 181) |
| `fetchEvalSummary` (253) | `GET /hud/eval/summary` |
| `fetchAvailability` (285) | `GET /hud/availability` |
| `fetchSecretSlots` (309) / `postSecret` (318) / `deleteSecret` (330) | `GET/POST /hud/secrets`, `DELETE /hud/secrets/:account` |
| `postExportPosting` (446) | `POST /audit/export-posting` |
| `postAutoresearchCancel` (506) | `POST /autoresearch/cancel` |
| `fetchGraph` (514) | `GET /graph/:id` |
| `fetchSwarmAgents` (722) / `fetchSwarmInbox` (731) / `fetchSwarmChecks` (820) / `postSwarmSend` | swarm REST |
| `fetchJournalPage` (846) | `GET /hud/journal` (paginado + filtro kind) |
| `fetchMissions` (1047) | `GET /tail/missions` |
| `approveApproval` (1152) / `denyApproval` (1184) | `POST /hud/approvals/:id/{approve,deny}` |
| `fetchCost` (1226) | `GET /hud/cost` |
| `fetchHealth` (1276) | `GET /hud/health` |
| `fetchAudit` (1315) | `GET /hud/audit` |
| `fetchPayload` (1343) | `GET /payload/:kind/:id` |
| `fetchWorktrees` (1377) | `GET /hud/worktrees` |
| `fetchDemos` (1408) | `GET /hud/demos` |
| `fetchMcp` (1474) / `saveMcpAllowlist` (1497) / `probeMcp` (1531) | `GET /hud/mcp`, `POST /hud/mcp/allowlist`, `POST /hud/mcp/probe` |
| `postActivateSkill` (1556) | `POST /hud/skills/:id/activate` |
| `fetchRemoteStatus` (1580) | `GET /remote/status` |

- **Tipos espejo de Rust** (selección) [O]: `ModelSwapRow` (37-44), `StepStateRow` (46-52), `StepPhaseTag` (54: `'pending'|'executing'|'verifying'|'done'|'blocked'`), `phaseColor` (58), `DiffAnnotation` (80), `EvalSummary`/`EvalGroupSummary` (224/239), `Availability` (271), `SecretSlot`/`SecretsResponse` (299/304), `ExportPostingRequest/Response` (434/439), `AutoresearchSnapshot/Candidate/Cancel` (470-501), `GraphNode/EdgeKind/NodeKind/Provenance` (528-534), `SwarmRole/AgentState/EventKind` (595-650), `KanbanColumn` (1028), `AgentStepAction/Verdict` (949), `MonitorPressure` (878).
- [I] No hay stores de estado global de UI aparte de `hud` (WS) y `views` (routing): el estado efímero de cada componente vive en `$state` local.

### `src/lib/stores/views.ts` — 79 líneas [O]

- `ViewId` (12-25): 13 ids — `overview, agent, kanban, approvals, cost, health, audit, canvas, outline, timeline, worktrees, settings, mcp`.
- `ViewDef` (27-34): `{ id, label, key, fase: 'P0'|'P1'|'P2' }`.
- `VIEWS` (36-50): catálogo con hotkey por view — `o, a, k, p, c, h, u, g, l, t, w, s, m` (ver (d)).
- `activeView` (55-73): writable + `set`/`cycle(dir)` — el ciclo ahora recorre TODAS las fases (65-67).
- `viewForKey` (76-79): resuelve letra→view. **Solo la usan los tests** (`rfc65-components.test.ts`) — no hay consumidor en app [O: grep `viewForKey` en `src/`].

### Tests [O]

| Fichero | Tests (regex `it(`/`test(`) |
|---|---|
| `src/lib/stores/hud.test.ts` | 81 |
| `src/lib/components/rfc65-components.test.ts` | 4 |
| `src/lib/components/JournalObserver.test.ts` | 6 |
| `src/lib/components/SwarmConsole.test.ts` | 5 |

`pnpm test` (vitest 4.1.10) reporta **4 files, 116 tests passed** [O] — el recuento por regex bajoestima (hay `it.each`/bloques anidados); los números de vitest son los válidos.

---

## (d) Mapa de hotkeys real vs RFC 24 §19

**Lo único cableado globalmente** [O — `+page.svelte:338-350`, `onGlobalKey` registrado en `$effect` 352-355, ignora INPUT/TEXTAREA/contentEditable (340-343)]:

| Tecla real | Acción | Ref |
|---|---|---|
| `:` | Abre el command palette | +page.svelte:344-346 |
| `Escape` | Cierra el palette | +page.svelte:347-349 |
| `ArrowUp`/`ArrowDown`/`Enter`/`Escape` | Navegación interna del palette | CommandPalette.svelte:66-80 |

**Hotkeys por view declarados en `views.ts:36-50` pero NO cableados** [O]: el listener global no llama `viewForKey` ni `activeView.cycle`; `viewForKey` solo aparece en tests y `cycle` no tiene ningún consumidor en `src/` [O: grep]. Los `title` del ViewSwitcher anuncian `(:)o`, `(:)a`, etc. (ViewSwitcher.svelte:24) — atajos que hoy no responden.

**Desalineación con RFC 24 §19** (tabla en `Atlas OS/24 - HUD Mission Control.md:621-637`) [O]:

| Tecla | RFC 24 §19 dice | Real (código) | Severidad [I] |
|---|---|---|---|
| `:m` | Cambiar mission | View **MCP** (views.ts:49) | conflicto de binding |
| `:v` | Cambiar view | **no implementado** (solo ciclo vía palette) | ausente |
| `:a` | Foco a approvals queue | View **Agent** (views.ts:38) — approvals quedó en `:p` (views.ts:40) | conflicto de binding |
| `:n` | New mission | no implementado | ausente |
| `:f` | Fork selected | no implementado | ausente |
| `:s` | Steer selected | View **Settings** (views.ts:48) | conflicto de binding |
| `:d` | Show Demo | no implementado (DemoPane solo es alcanzable dentro de la view Agent, +page.svelte:394) | ausente |
| `:r` | Run selected | no cableado — solo hint cosmético del palette ("Refresh HUD data", CommandPalette.svelte:36) | ausente |
| `:p` | Pause selected | View **Approvals** (views.ts:40) | conflicto de binding |
| `:x` | Stop selected | no implementado | ausente |
| `:c` | Comment on selected | View **Cost & Res** (views.ts:41) | conflicto de binding |
| `:e` | Expand to canvas | no cableado — solo hint cosmético ("Export audit posting", CommandPalette.svelte:42) | ausente |
| `:o` | Mode selector (ask/architect/code/context) | View **Overview** (views.ts:37) | conflicto de binding |
| `:t` | Toggle Health KPIs dock | View **Timeline** (views.ts:46) | conflicto de binding |
| `?` | Help overlay | no implementado | ausente |

[I] Resumen: 6 atajos con binding desalineado (`:m`, `:a`, `:s`, `:p`, `:c`, `:o`, `:t`), 8 ausentes y solo 3 operativos (`:`, `Escape`, flechas/Enter en palette).

---

## (e) Colores y valores hardcodeados

**Comando reproducible** [O]: regex `#[0-9a-fA-F]{3,8}\b` + `rgba?\(` + ~140 colores CSS con nombre, sobre `src/**/*.svelte|css|html` (no hay `.css`, así que solo `.svelte`).

**Resultado [O]:**

- **Total: 531 literales de color en 25 ficheros.**
- **Hex únicos: 30** — la cifra de RFC 66 ("~30 colores hardcodeados") es exacta a nivel de valores únicos [O]; a nivel de literales totales es 531.
- **`src/app.css` ABSENT** [O] — no existe el fichero de tokens que RFC 65 §6 presupone; `+layout.svelte:3` lo admite ("Design tokens (CSS file) loaded at +page level until deps installed").
- **Sin selector dark/light** [O]: `+page.svelte` tiene 0 menciones de `dark`/`light` y ningún toggle de tema — RFC 65 §6 afirma "dark/light (selector ya en `+page.svelte`)" y eso es hoy **falso** (discrepancia RFC↔código).

**Top hex únicos por frecuencia [O]** (valor×apariciones): `#8b949e`×80, `#21262d`×56, `#30363d`×44, `#6e7681`×43, `#f85149`×39, `#c9d1d9`×35, `#161b22`×31, `#0d1117`×30, `#58a6ff`×24, `#79c0ff`×21, `#56d364`×20, `#d29922`×20, `#f0883e`×8, `#3fb950`×8, `#fff`×4, `#238636`×3; el resto (14 valores) aparecen 1-2 veces. [I] La paleta es la de GitHub Dark sin tokenizar; los 30 valores son el objetivo natural de extracción a variables CSS.

**Recuento por fichero [O]** (total = hex + rgba + named; comando citado arriba):

| Fichero | Total | hex | rgba | named |
|---|---|---|---|---|
| `src/routes/+page.svelte` | 61 | 57 | 1 | 3 |
| `src/lib/components/SwarmConsole.svelte` | 60 | 51 | 5 | 4 |
| `src/lib/components/McpView.svelte` | 35 | 35 | 0 | 0 |
| `src/lib/components/AutoresearchCard.svelte` | 29 | 24 | 5 | 0 |
| `src/lib/components/AgentCard.svelte` | 28 | 22 | 5 | 1 |
| `src/lib/components/JournalObserver.svelte` | 27 | 25 | 0 | 2 |
| `src/lib/components/CostDashboard.svelte` | 25 | 24 | 1 | 0 |
| `src/lib/components/SkillMcpRail.svelte` | 25 | 24 | 1 | 0 |
| `src/lib/components/HealthKPIs.svelte` | 22 | 21 | 0 | 1 |
| `src/lib/components/KanbanBoard.svelte` | 19 | 19 | 0 | 0 |
| `src/lib/components/ApprovalQueue.svelte` | 17 | 12 | 5 | 0 |
| `src/lib/components/GraphView.svelte` | 17 | 15 | 1 | 1 |
| `src/lib/components/SettingsView.svelte` | 17 | 17 | 0 | 0 |
| `src/lib/components/AvailabilityCard.svelte` | 16 | 13 | 3 | 0 |
| `src/lib/components/DemoPane.svelte` | 16 | 16 | 0 | 0 |
| `src/lib/components/EvalCard.svelte` | 16 | 14 | 2 | 0 |
| `src/lib/components/WorktreesView.svelte` | 16 | 16 | 0 | 0 |
| `src/lib/components/AuditTimeline.svelte` | 12 | 11 | 0 | 1 |
| `src/lib/components/OutlineView.svelte` | 12 | 12 | 0 | 0 |
| `src/lib/components/SpendLimitErrorCard.svelte` | 12 | 12 | 0 | 0 |
| `src/lib/components/CommandPalette.svelte` | 11 | 8 | 3 | 0 |
| `src/lib/components/ModelReadyCard.svelte` | 11 | 11 | 0 | 0 |
| `src/lib/components/TimelineView.svelte` | 11 | 10 | 0 | 1 |
| `src/lib/components/ViewSwitcher.svelte` | 9 | 7 | 2 | 0 |
| `src/lib/components/CanvasView.svelte` | 7 | 7 | 0 | 0 |

Otros valores no-color hardcodeados [O]: intervalos de poll `15000` ms en AvailabilityCard:31, AuditTimeline:31, EvalCard:31, TimelineView:32; `5000` ms en KanbanBoard:61 y +page.svelte:288; backoff cap `10_000` ms en hud.ts:404; `560px`/`92vw`/`46vh` del palette (CommandPalette.svelte:136,158); `12vh` del overlay (132). [I] Los intervalos inconsistentes (15 s vs 5 s) conviene unificar en la spec de UI.

---

## (f) Estados de carga/vacío/error por view

Detección [O]: regex sobre cada componente (`{#if error}`, `loading`, `No matching`, `no <recurso>`, `empty`). La interpretación de cada fila es [I] salvo la ref.

| Componente (view) | error | loading | vacío | Notas [O]/[I] |
|---|---|---|---|---|
| AgentCard (Agent) | ✗ | ✗ | ✓ | [O] sin UI de error pese a depender del WS; el vacío se cubre (empty=true) |
| ApprovalQueue (Approvals) | ✓@48 | ✗ | ✓ | |
| AuditTimeline (Audit) | ✓@55 | ✓ | ✓ | |
| AutoresearchCard (overview) | ✓@168 | ✗ | ✓ | |
| AvailabilityCard (overview) | ✓@63 | ✓@45 | ✗ | |
| CanvasView (Canvas) | ✓@50 | ✗ | ✓ | |
| CommandPalette | ✗ | ✗ | ✓@117 ("No matching command.") | [O] estado vacío del filtro |
| CostDashboard (Cost & Res) | ✓@65 | ✓@21 | ✓ | |
| DemoPane (Agent) | ✓@60 | ✓@19 | ✓ | |
| EvalCard (overview) | ✓@55 | ✗ | ✓ | |
| GraphView (Canvas) | ✓@92 | ✓@25 | ✓ | |
| HealthKPIs (Health KPIs) | ✓@68 | ✓ | ✓ | |
| JournalObserver (overview) | ✓@133 | ✓@27 | ✓ | |
| KanbanBoard (Kanban) | ✓@77 | ✗ | ✓ | [O] sin loading pese a poll 5 s |
| McpView (MCP) | ✓@134 | ✓@32 | ✓ | |
| ModelReadyCard | ✓@78 | ✗ | ✗ | [I] card de contexto, recibe payload por props |
| OutlineView (Outline) | ✓@82 | ✓@25 | ✓ | |
| SettingsView (Settings) | ✓@101 | ✓ | ✓ | |
| SkillMcpRail (overview) | ✓@87 | ✗ | ✓ | |
| SpendLimitErrorCard | ✓@143 | ✗ | ✗ | [I] card de contexto |
| SwarmConsole (overview) | ✓@182 | ✓@24 | ✓ | |
| TimelineView (Timeline) | ✓@50 | ✗ | ✓ | |
| ViewSwitcher | ✗ | ✗ | ✗ | [I] no hace fetch |
| WorktreesView (Worktrees) | ✓@53 | ✓@19 | ✓ | |

- **+page.svelte** (overview): cada tail box de la grid `tails` tiene error (666-667) / vacío "No rows yet." (668-669) / render por kind (670-684) [O]; el form de audit export tiene busy/error/result (548-584) [O].
- **Resumen [I]:** 21/24 componentes tienen estado de error, 12/24 loading, 21/24 vacío. Las ausencias de loading se concentran en los fetchers más antiguos (Kanban, Timeline, Canvas, Eval, Autoresearch, Approvals, SkillMcpRail, AgentCard); la de error en AgentCard es la más visible porque es la view que depende del WS en vivo.
- **Estado global de carga al primer render:** [I] no hay skeleton/splash — el primer frame muestra secciones vacías hasta el primer poll.

---

## (g) Las 8 views de RFC 65 — qué existe ya

Todas las 8 views del RFC 65 §3 están **implementadas y cableadas en `+page.svelte`** (ramas `{#if $activeView}` 386-502), además de `settings` y `mcp` añadidas después (catálogo `views.ts:36-50`, 13 ids) [O].

| View (RFC 65 §3) | Componente(s) | Estado [O] | Cableado y datos |
|---|---|---|---|
| Kanban | `KanbanBoard.svelte` (+ columnas inline) | **implementado** | +page.svelte:396-405; `GET /tail/missions` (fetchMissions@53) |
| Canvas | `CanvasView.svelte` + `GraphView.svelte` | **implementado** | +page.svelte:446-454; `GET /tail/missions` + `GET /graph/:id` (requiere feature `dag_mode` en el backend — server.rs:119-120) [O] |
| Outline | `OutlineView.svelte` | **implementado** | +page.svelte:455-465; `GET /tail/plans` + `GET /payload/plan/:id` (fetchPayload@48) |
| Timeline | `TimelineView.svelte` | **implementado** | +page.svelte:466-475; `GET /hud/journal` (fetchJournalPage@22) |
| Cost & Res | `CostDashboard.svelte` (sparkline inline) | **implementado** | +page.svelte:415-424; `GET /hud/cost` (fetchCost@28) |
| Health KPIs | `HealthKPIs.svelte` | **implementado** | +page.svelte:425-435; `GET /hud/health` (fetchHealth@38) + bus `agent_heartbeat` |
| Audit | `AuditTimeline.svelte` | **implementado** (sin verificación de cadena — el hint lo declara: +page.svelte:441-442) | +page.svelte:436-445; `GET /hud/audit` (fetchAudit@23) |
| Worktrees | `WorktreesView.svelte` | **implementado** | +page.svelte:476-484; `GET /hud/worktrees` (fetchWorktrees@26) |
| (extra) Settings | `SettingsView.svelte` | **implementado** | +page.svelte:485-493; secrets API (keychain del OS, RFC 25 §3.10) |
| (extra) MCP | `McpView.svelte` | **implementado** | +page.svelte:494-502; `GET /hud/mcp` + `POST /hud/mcp/allowlist` (RFC 07) |
| (extra) Overview | branch default `{:else}` | **implementado** | +page.svelte:503-656: HUD health, Skill & MCP rail, audit export, Autoresearch, Swarm, Proactive, Eval, journal, observer, 11 tail boxes |

Lo que **no existe** de las 8 views / extras de RFC 65 [O]:

- `DemoPane` solo está montado dentro de la view Agent (+page.svelte:394), no como view propia ni `:d` global.
- `SkillMcpRail` está en el Overview (+page.svelte:525); el propio hint declara "Hot-swap activation is not wired in this build — drag only stages a selection" (522-523) [O].
- Sin `:d`/`:v`/`:n`/`:f`/`:r`/`:x`/`?` globales (ver (d)) — la navegación por teclado de RFC 24 §19 está a medio cablear [O].

---

## Resumen de hallazgos para RFC 67

1. **Hotkeys desalineados y sin cablear** [O]: 7 bindings en conflicto con RFC 24 §19, 8 ausentes, 3 operativos; `viewForKey`/`cycle` sin consumidor. Los `title` del ViewSwitcher anuncian atajos muertos.
2. **30 hex únicos / 531 literales / 0 tokens / 0 dark-light** [O]: `src/app.css` no existe y RFC 65 §6 afirma lo contrario — extraer tokens es el paso 1 del design system.
3. **`+page.svelte` = 1237 líneas** [O] (RFC 66 estimaba ~1250 — confirmado), con 22 componentes importados y todo el estado efímero del Overview inline.
4. **8/8 views RFC 65 + settings + mcp implementadas** [O] — el gap de RFC 66 no es funcional sino de calidad de UI (tokens, hotkeys, estados).
5. **Estados de carga inconsistentes** [O]: 12/24 con loading, intervalos de poll dispares (5 s vs 15 s), sin skeleton global.
6. **`CanvasView` depende de `dag_mode`** [O] (server.rs:119-120) — la view degrada a error si el backend se compila sin esa feature; la spec de UI debe definir el fallback.
