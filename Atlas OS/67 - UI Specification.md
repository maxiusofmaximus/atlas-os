# 67 - UI Specification (construible)

**Author:** UI/UX Design agent · **Date:** 2026-10-06 · **Status:** Draft **v4** — backend **cotejado** (`backend-capabilities.md`; §1.7 y §19 **CERRADAS**); views rellenas con `frontend-current-state`; `rfc-vs-code` aplicado; contrato de transición de views (§1.11); **§20 veredictos H-01…H-09**; **§22 plan FASE 10 + gates §22.3**; **§23 backlog FASE 11+**. Fichas del Researcher: **132** en `ux-catalog/` (`count.mjs`: **62 [Os] / 55 [Os parcial] / 15 [P]**; 10 de categoría). **Sin bloqueos de backend.**
**Depends on:** RFC 17 (UI base) · RFC 24 (HUD spec) · RFC 65 (HUD v2 implementación) · RFC 66 (UX Architecture & Design System) · `docs/design/PALETTE.md` (token system) · `docs/design/CONSENSUS_AUDIT.md` · `docs/coordination/PLAN-docs-hardening.md`.
**Inputs consumidos (estado 2026-10-06):**
- Researcher `Atlas OS/research/64 - UX reference catalog.md` → **ENTREGADO [O]**: recuento **exacto = 441** (RFC 62 decía "~425"; **no coincide con ningún "320+"**); `tiene_UI` si=376/no=38/parcial=27; **94 en categorías HUD-relevantes** (IDE 29 · terminal 21 · ADE 20 · observabilidad 18 · canvas 5 · kanban/PM 1). Fichas `ux-catalog/*.md` → **132 [O]** (`count.mjs`: **62 [Os] / 55 [Os parcial] / 15 [P]**; **10 ficheros de categoría** + `_pending-A`/`_funcion-x-referente`).
- Builder `docs/audit/backend-capabilities.md` → **ENTREGADO [O]** (276 líneas): **41 rutas** (39 montadas + 2 feature-gated), **26 eventos** WS, **34 subcomandos CLI**, **61 tablas**, y **9/9 huecos H-01…H-09 contrastados con `fichero:línea`** ⇒ §1.7/§19/§20 **cerrados** (era `[P]`).
- Fichas `ux-catalog/` → **132 [O]** (`count.mjs`: **62 [Os] / 55 [Os parcial] / 15 [P]**): **10 ficheros de categoría**, incl. `outline`/`settings`/`kanban-pm`/`canvas` (citadas en §1.12); `_pending-A.md` se recalcula con `count.mjs`.
- Builder-2 `docs/audit/frontend-current-state.md` → **ENTREGADO [O]** (277 líneas): `+page.svelte` **1237 líneas**; **24 componentes**; **531 literales de color / 30 hex únicos / `src/app.css` ausente / sin selector dark-light**; **13 views** (`views.ts`); hotkeys **3 operativos / 7 en conflicto / 8 ausentes**; tabla de estados carga/vacío/error por view.
- Builder-3 `docs/audit/rfc-vs-code.md` → **ENTREGADO [O]** (10 discrepancias + 3 de drift). Afectan a UI: **#2 MCP hot-swap NO implementado** · **#3 `WorktreesView` sin grafo (tabla)** · **#7 MCP write+probe SÍ existen** · **#9 hay 13 views, no 8**.
- **Regla de cierre:** las secciones que dependen de Builder (endpoints/payloads) NO se cierran hasta leer `backend-capabilities.md`.

**Cross-check de recuento (RFC 67 vs research/64) [O]:** mi estimación previa "~88–100 UI-relevantes" (CONSENSUS_AUDIT §1.2) coincide con el **94** exacto del Researcher. Ambos sustituyen la cifra no documentada "320+".
**Scope:** especificación de UI construible por pantalla (**13 views actuales → 10 tras la transición**, §1.11; + Approvals Dock + Agent Card + Command Palette + Mission Rail + Activity Spine + Settings/MCP). Decide *qué* se construye; el **plan de implementación FASE 10 está en §22**. Doc-only; no toca `src/` ni `src-tauri/`.

---

## 0. Cómo leer este documento

**Leyenda de procedencia (obligatoria en cada afirmación):**

| Marca | Significado |
|---|---|
| **[O]** | Observado en este repo **esta sesión** (código/RFC/rutas reales). |
| **[Os]** | Observado **en vivo** en la fuente citada (URL) esta sesión. |
| **[I]** | Inferido / de memoria; no inspeccionado. |
| **[P]** | Pendiente (de un rol o de verificación). |
| **[R]** | Recomendación de diseño de este agente (la decisión es del Designer; respaldada por operador+PL). |

**Contrato de datos:** cada componente declara su fuente como `GET/POST /ruta` **[O `src-tauri/src/hud/server.rs`]** o evento WS `type` **[O `src-tauri/src/core/bus.rs`]**. Si no existe fuente ⇒ **HUECO → Builder** (no se inventa).

**Datos reales disponibles hoy [O]:**
- WS `/ws` emite `BusEvent { id, idempotency_key, kind, ts }`, con `kind` *internally tagged* por `type` en `snake_case` (`bus.rs:8-16`). El frontend aplana `kind.type` → `kind` string y usa `payload` (`hud.ts:345-363` **[O]**). Reconexión exponencial 1s→10s (`hud.ts:401-407` **[O]**).
- 44 rutas axum (`server.rs:66-146` **[O]**; ver §1.7).

**Dependencias de diseño resueltas por el PL (RFC 66 §16) [O]:** OA-66-06 = **(c)** (steer inline + hilo expandible por run); OA-66-07 = **(b)** (Approvals Dock en dos canales: *gate* bloqueante vs *pregunta* async); OA-66-08 = **(b)** v1 (Context Rail con worktree/branch/dev-server) y **(c)** v2 (sesión multi-agente).

---

## 1. Modelo global (aplica a TODAS las pantallas)

### 1.1 Layout maestro [R]
`TopBar` · `Mission Rail` (izq) · `View Bar` + lienzo (centro) · `Activity Spine` (der) · `Approvals Dock` (abajo) · `StatusBar`. (RFC 66 §5.2.)

### 1.2 Design tokens (estado real: **NO aplicados**)
Fuente única objetivo: `docs/design/PALETTE.md` (OKLCH→sRGB, WCAG + APCA medidos **[O]**). Componentes deben usar `var(--a-*)`; **prohibido hex crudo** [R]. Estado = **color + glifo + label** (regla anti-CVD).
**Real hoy [O `frontend-current-state` §(e)]:** `src/app.css` **ausente**; **531 literales de color** en 25 `.svelte`; **30 hex únicos** (GitHub Dark sin tokenizar; `#8b949e`×80 …); **sin selector dark/light** (RFC 65 §6 lo afirma y es **falso** → corregido aquí). ⇒ **Paso 1 de FASE 10 = crear `src/app.css` + tokenizar** (v1).

### 1.3 Modelo de estado de agente (real)
`AgentStatus` **[O `bus.rs:224-235`]:** `Queued, Reading, Planning, Coding, Reviewing, Idle, Paused, DoomLoop, Error, Success` (+ **`Unknown` propuesto** [R], RFC 66 §6.1; no existe en el enum Rust → HUECO→Builder si se quiere persistir). `StepPhaseTag` **[O `hud.ts:54`]:** `pending/executing/verifying/done/blocked`.

### 1.4 Matriz de estados universal (obligatoria en cada componente)
Cada componente DEBE especificar comportamiento en los **6 estados base** + los suyos:

| Estado | Definición | Comportamiento base [R] |
|---|---|---|
| **Carga** | fetch en vuelo / primer frame | skeleton con la forma final (no spinner) |
| **Vacío** | 200 OK sin filas | empty state con copy + CTA para poblarlo |
| **Error** | fetch falla / 5xx | inline error + `Retry`; nunca bloquea el resto del HUD |
| **Parcial** | hay datos pero incompletos (paginado, feature off) | renderiza lo que hay + badge "parcial" + razón |
| **Desconectado** | WS caído (`hud.connected=false` **[O]**) | banner "sin conexión · reconectando" + datos congelados con timestamp |
| **Unknown** | dato presente, no clasificable | glifo `?` + `--a-text-faint` + label "unknown"; nunca inventa un estado |

### 1.5 Teclado y foco (global)
- **Real hoy [O `frontend-current-state` §(d)]:** solo **3** atajos operativos (`:` abre palette, `Escape` cierra, flechas/`Enter` dentro del palette). Los hotkeys de `views.ts:36-50` **no están cableados**; **7 colisionan** con RFC 24 §19 (`:m,:a,:s,:p,:c,:o,:t` apuntan a views) y **8 están ausentes** (`:v,:n,:f,:d,:r,:x,?,:e`). Los `title` del `ViewSwitcher` anuncian atajos muertos. ⇒ **v1: cablear el mapa canónico RFC 24 §19 y retirar los bindings de view** (D-66-05; ver §1.11).
- Foco: **`:focus-visible`** = anillo `--a-focus` 2px + offset 2px (WCAG 1.4.11 ≥3:1 **[O PALETTE §4.1]**); foco atrapado en overlays; `Esc` cierra.
- Navegación de lista/card: flechas ↑↓ mueven selección; `Enter` abre; `Space` selecciona; `Tab` siguiente región.

### 1.6 A11y baseline [R]
Contraste texto AA (WCAG) y APCA (medido **[O]** — §18); estado nunca solo-por-color (§1.3); `prefers-reduced-motion` desactiva pulse **[O mockup]**; `lang` + landmarks (`nav/main/aside/complementary`); targets ≥ 24px (WCAG 2.5.8).

### 1.7 Contrato de datos real — **cotejado** [O `docs/audit/backend-capabilities.md` §(a)/(b); `src-tauri/src/hud/server.rs`]
| Ruta | Método | Consume |
|---|---|---|
| `/health` | GET | StatusBar |
| `/ws` | GET(WS) | todo el HUD |
| `/tail/{journal,missions,verdicts,consolidated,plans,diffs,validation_reports,repairs,patterns,checkpoints,skills,model_swaps,step_states,agent_steps}` | GET | vistas + cards |
| `/hud/journal` | GET | Timeline (paginado) |
| `/hud/approvals/:id/approve` · `/deny` | POST | Approvals Dock |
| `/diff/:id/annotation` | GET/POST | diff comments |
| `/audit/export-posting` | POST | Audit |
| `/autoresearch/cancel` | POST | Autoresearch card |
| `/payload/:kind/:id` · `/payload/skill/:skill_id/:version` | GET | drill-down |
| `/remote/status` | GET | Settings/TopBar |
| `/hud/{eval/summary,cost,health,audit,worktrees,demos,availability,reliability}` | GET | vistas |
| `/hud/mcp` · `/hud/mcp/allowlist` · `/hud/mcp/probe` | GET/POST/POST | MCP |
| `/hud/skills/:id/activate` | POST | Skill picker |
| `/hud/secrets` · `/hud/secrets/:account` | GET/POST/DELETE | Settings |
| `/graph/:mission_id` | GET | Canvas (feature `dag_mode`) **[O server.rs:152-153]** |

**Cotejo 1:1 contra el backend (ruta real · handler · request/response · estado de test) [O `backend-capabilities.md` §(a)/(e)]:**

| Endpoint (RFC 67) | Handler real | Request/Response real | Estado test [O] |
|---|---|---|---|
| `/health` | `health` server.rs:180 | `"ok"` | NO-VERIFICADO (trivial) |
| `/ws` | `ws_handler` ws.rs:19; `fan_out` 52-76 | `BusEvent` JSON; **cliente→server IGNORADO** (Phase 0, ws.rs:33-47) | **NO-VERIFICADO** (no hay `hud::ws::tests`) |
| `/tail/*` (14) | `tail_of::<R>` tail.rs:69/154-212 | `Vec<Value>`; `?last` clamp `[1,200]` def 20 | VALIDADO (6 × `hud::tail::tests`) |
| `/hud/journal` | `observer::get_journal_page` observer.rs:59 | `{rows,total}`; `limit/offset/kind`; 400 | VALIDADO (9 × `hud::observer::tests`) |
| `/hud/approvals/:id/{approve,deny}` | `approvals::approve/deny` approvals.rs:72/90 | body `{user_id?,reason?}` → `DecisionAck`; **`reason` se DESCARTA** (`let _ = reason;` approvals.rs:64) | VALIDADO parse (2) |
| `/diff/:id/annotation` | `annotate::post/get` annotate.rs:48/79 | `AnnotationPost`; 400/422 | VALIDADO (4) |
| `/audit/export-posting` | `export::post_export_posting` export.rs:48 | `{last?,output_dir?}`→`{files_written,entries_packed,entries_purged,snapshot_root}` | VALIDADO (5) |
| `/autoresearch/cancel` | `autoresearch::post_…` autoresearch.rs:49 | `{run_id,outcome}`→204 | VALIDADO (7) |
| `/payload/:kind/:id` | `tail::payload` tail.rs:234 | 7 kinds (`verdict/plan/diff/validation_report/repair/pattern/checkpoint`) | VALIDADO |
| `/payload/skill/:id/:ver` | `tail::payload_skill` tail.rs:261 | manifest | **NO-VERIFICADO** |
| `/remote/status` | `remote_status::get_remote_status` remote_status.rs:21 | status snapshot; enforce bearer **solo** con feature `remote-ui` | VALIDADO informational |
| `/hud/eval/summary` | `eval::get_eval_summary` eval.rs:22 | summary + groups | VALIDADO (1) |
| `/hud/cost` | `cost::get_cost` cost.rs:32 | `{window,totals,by_model,cumulative_usd,pressure{level,warn_usd=5,crit_usd=20},pending_resets[]}` — **sin budget** (H-06) | VALIDADO (1) |
| `/hud/health` | `health::get_health` health.rs:24 | `{agent_events,agent_runs[],swarm_agents[]}` | VALIDADO (1) |
| `/hud/audit` | `audit::get_audit` audit.rs:24 | `{rows,count}`; **sin verificación de cadena** (declarado audit.rs:4-7) | VALIDADO (vacío) |
| `/hud/worktrees` | `worktrees::get_worktrees` worktrees.rs:27 | `{repo,ok,reason,entries[{path,branch,detached}]}`; fail-safe `ok:false` | VALIDADO (error) |
| `/hud/demos` | `demos::get_demos` demos.rs:38 | `{artifacts[{id,run_id,kind,path,sha256,verified,preview_url}],count}`; **sin video/TTS** (H-07) | VALIDADO (2) |
| `/hud/mcp` + `/allowlist` + `/probe` | `mcp::get_mcp/set_allowlist/probe` mcp.rs:32/114/159 | servers + policy (sandbox/supply/allowlist) | VALIDADO (7) |
| `/hud/skills/:id/activate` | `skills::activate` skills.rs:44 | `{agent_id?}`→`ActivateAck` | VALIDADO (2) |
| `/hud/secrets` + `/:account` | `secrets::get/post/delete` secrets.rs:43/55/72 | `{slots[{account,present}]}`; **nunca el valor** | VALIDADO (2) |
| `/hud/availability` | `availability::get_availability` availability.rs:14 | `{enabled,policy,availability,pending_mission}` | VALIDADO (1) |
| `/hud/reliability` | `reliability::get_reliability` reliability.rs:14 | `{policy,models}` | VALIDADO (1) |
| `/graph/:mission_id` | `graph::get_graph` graph.rs:30 **cfg `dag_mode`** | `MissionGraph` | **NO-VERIFICADO** (solo schema tests) |

> **Auth [O backend §(a)]:** **ninguna ruta autentica por defecto** (loopback + CORS permissive; solo `/remote/status` con `remote-ui`). ⇒ **UI:** si se publica con `atlas serve --host`, la spec debe mostrar aviso de exposición (RFC 18) — ver §20-D.

**Eventos WS (26, snake_case) — payload cotejado [O `bus.rs:17-189`; backend §(b)]:** `task_received{raw_prompt,session_id}` · `mission_consolidated{mission_id,verdict_id,confidence}` · `mission_locked{mission_id,planning_session_id}` · `plan_generated{plan_id,mission_id}` · `agent_status_changed{agent_id,status}` · `agent_diff{agent_id,files,lines_added,lines_removed}` · `agent_tokens{agent_id,tokens_in,tokens_out,cost_usd}` · `agent_step{run_id,step,action,observation?,verdict?,tokens_in,tokens_out,cost_usd}` · `agent_heartbeat{agent_id}` · `approval_request{approval_id,agent_id,action}` · `approval_decision{approval_id,decision,user_id}` · `doom_loop_detected{agent_id,count}` · `goal_drift_detected{agent_id,drift}` · `journal_checkpoint{checkpoint_id}` · `cost_threshold_crossed{agent_id,threshold,cumulative}` · `worktree_dirty{agent_id,path,dirty}` · `skill_activated{agent_id,skill_id}` · `artifact_preview_opened{mission_id?,artifact,url,source}` · `research_completed{research_run_id}` · `hud_served{hud_port}` · `mission_steered{mission_id,message}` · `model_swapped{mission_id,prev_model_id,new_model_id,initiator}` · `step_phase_changed{mission_id,plan_id,step_id,phase}` · `autoresearch_cancelled{run_id,outcome}` · `spend_limit_observed{provider,model,status_code,resets_at_ms,error_type,toast_enqueued_id?}` · `hardware_snapshot{ram_total_mb,ram_used_mb,vram_total_mb?,vram_used_mb?,cost_usd}`.
**WS solo-bajada [O ws.rs:33-47]:** el cliente→servidor **se ignora** (Phase 0) ⇒ **toda acción** (steer/decisión/pausa) va por **REST**.
**⚠️ Comandos Kernel = DEAD CODE [O backend §(b): `#[allow(dead_code)]` bus.rs:248; grep `KernelCommand` = 1 match = la declaración].** RFC 67 v1 los citaba como contrato operativo; **NO lo son**. La UI **no debe** cablearse a `KernelCommand`; las acciones de card (§4) usan REST.

### 1.8 Responsive [R]
≥1600 todo · 1100–1600 spine colapsable · 720–1100 rail→drawer · ≤720 **modo review** (Approvals + Mission list + activity; sin canvas/editor) (RFC 66 §11).

### 1.9 Orden de construcción global y dependencias
Ver §16. Regla: **contrato de datos primero** (Builder confirma endpoints), luego **Agent Card + Mission Rail** (base), luego vistas, luego cross-cut.

### 1.10 Baseline del frontend actual [O `docs/audit/frontend-current-state.md`]

> **Por qué existe:** la spec no debe prometer lo que no existe ni ignorar lo que ya hay. Fuente: Builder-2.

**(a) Rutas/layout** [O fcs §(a)]: `+page.svelte` = **1237 líneas** (todo el HUD monolítico: script ~310 + secciones + `<style>` inline + `aside.drawer`); `+layout.svelte` de 5 líneas (deja cargar CSS "until deps installed").

**(b) Componentes** [O fcs §(b)]: **24 `.svelte`** en `src/lib/components/` (SwarmConsole, McpView, JournalObserver, SkillMcpRail, HealthKPIs, CostDashboard, AutoresearchCard, AgentCard, GraphView, SettingsView, EvalCard, KanbanBoard, SpendLimitErrorCard, DemoPane, CommandPalette, ApprovalQueue, WorktreesView, OutlineView, AvailabilityCard, AuditTimeline, ModelReadyCard, TimelineView, CanvasView, ViewSwitcher).

**(c) Stores** [O fcs §(c)]: `hud.ts` 1585 líneas / 149 exports; `views.ts` 79 líneas.

**(d) Hotkeys reales** [O fcs §(d)]: ver §1.5 (3 operativos, 7 conflictos, 8 ausentes).

**(e) Colores** [O fcs §(e)]: 531 literales / 30 hex / `app.css` ausente / sin dark-light (ver §1.2).

**(f) Estados carga/vacío/error por componente** [O fcs §(f)]:
| Componente (view) | error | loading | vacío |
|---|---|---|---|
| AgentCard (Agent) | ✗ | ✗ | ✓ |
| ApprovalQueue (Approvals) | ✓@48 | ✗ | ✓ |
| AuditTimeline (Audit) | ✓@55 | ✓ | ✓ |
| AutoresearchCard (overview) | ✓@168 | ✗ | ✓ |
| AvailabilityCard (overview) | ✓@63 | ✓@45 | ✗ |
| CanvasView (Canvas) | ✓@50 | ✗ | ✓ |
| CommandPalette | ✗ | ✗ | ✓@117 |
| CostDashboard (Cost) | ✓@65 | ✓@21 | ✓ |
| DemoPane (Agent) | ✓@60 | ✓@19 | ✓ |
| EvalCard (overview) | ✓@55 | ✗ | ✓ |
| GraphView (Canvas) | ✓@92 | ✓@25 | ✓ |
| HealthKPIs (Health) | ✓@68 | ✓ | ✓ |
| JournalObserver (overview) | ✓@133 | ✓@27 | ✓ |
| KanbanBoard (Kanban) | ✓@77 | ✗ | ✓ |
| McpView (MCP) | ✓@134 | ✓@32 | ✓ |
| ModelReadyCard | ✓@78 | ✗ | ✗ |
| OutlineView (Outline) | ✓@82 | ✓@25 | ✓ |
| SettingsView (Settings) | ✓@101 | ✓ | ✓ |
| SkillMcpRail (overview) | ✓@87 | ✗ | ✓ |
| SpendLimitErrorCard | ✓@143 | ✗ | ✗ |
| SwarmConsole (overview) | ✓@182 | ✓@24 | ✓ |
| TimelineView (Timeline) | ✓@50 | ✗ | ✓ |
| ViewSwitcher | ✗ | ✗ | ✗ |
| WorktreesView (Worktrees) | ✓@53 | ✓@19 | ✓ |

Resumen [O/I fcs §(f)]: **21/24 con error, 12/24 con loading, 21/24 con vacío**; sin skeleton global; polls dispares (5 s Kanban/+page:288 vs 15 s Availability/Audit/Eval/Timeline). **Gaps v1:** añadir loading a AgentCard, Kanban, Timeline, Canvas, Approvals, SkillMcpRail, Eval, Autoresearch (fcs §(f)-(g)); **error UI en AgentCard** (depende del WS, es el más visible).

**(g) Qué existe por view** [O fcs §(g)]: **las 8 views de RFC 65 + Settings + MCP + Overview están implementadas y cableadas** (`+page.svelte` ramas `{#if $activeView}` 386-502; `views.ts` = **13 ids**). No existe: `DemoPane` como view/` :d` (solo dentro de Agent, +page:394); hot-swap MCP (SkillMcpRail: "drag only stages", 522-523); `:d/:v/:n/:f/:r/:x/?` globales; `WorktreesView` como grafo (es tabla); Canvas sin `dag_mode` degrada a error.

### 1.11 Contrato de transición: las **13 views actuales** → modelo Mission Rail + proyecciones

> **Problema:** hoy el eje es un catálogo plano de views (`views.ts`) con un `Overview` monolítico. El modelo decidido (RFC 66 §3–5) es **Mission Rail (raíz) + proyecciones de la misión activa + cross-cut docks**. Hay que adecuar las 13 sin perder lo construido.

| View actual [O `views.ts`] | Decisión [R] | En el nuevo modelo | Justificación |
|---|---|---|---|
| `overview` | **Retirar (disolver)** | Su contenido se reparte: missions→Rail, actividad→Spine, health→StatusBar/KPIs, tails→vistas | Era un `{:else}` de 11 tail boxes (`+page:503-656`); no es una "pregunta del humano" única (RFC 24 §2) |
| `agent` | **Fusionar** | `AgentCard` dentro de Kanban/Canvas; `DemoPane`→`🎬 Demo`/`:d` global | El agente es una card, no una view; hoy es view + DemoPane embebido |
| `kanban` | **Conservar** | Proyección P0 | Responde "¿qué está bloqueado/en progreso/review?" |
| `approvals` | **Retirar como view → Dock** | **Approvals Dock** persistente (gate/pregunta, OA-66-07(b)) | Aprobar es **acción**, no navegación; RFC 24 §5/§1 lo ponen en dock |
| `cost` | **Conservar** | Proyección P1 | "¿cuánto cuesta/gasta?" |
| `health` | **Conservar** | Proyección P1 | "¿están sanos?" |
| `audit` | **Conservar** | Proyección P1 | "¿qué se decidió sensible y por quién?" |
| `canvas` | **Conservar** | Proyección P2 (feature `dag_mode`) | "¿cómo está cableado el swarm?" |
| `outline` | **Conservar** | Proyección P2 | "¿cómo se descompuso?" |
| `timeline` | **Conservar** | Proyección P2 | "¿cuándo pasó?" |
| `worktrees` | **Conservar (v1=tabla; v2=grafo)** | Proyección P2 | rfc-vs-code #3: el grafo no existe; v1 usa la tabla actual |
| `settings` | **Conservar (no-blocking)** | Vista/panel P1 | Config raro; nunca debe bloquear el lienzo |
| `mcp` | **Conservar (v1=catálogo+allowlist+probe; v2=hot-swap)** | Vista P1 | rfc-vs-code #2/#7: hot-swap no implementado; write+probe sí |

**Resultado:** **8** views-proyección (kanban, canvas, outline, timeline, cost, health, audit, worktrees) + **2** paneles (settings, mcp) + **3** cross-cut nuevos (Mission Rail, Activity Spine, Approvals Dock); `overview` y `agent` **desaparecen como views** (su contenido se reubica). [R]

**Migración v1 [R]:** (1) `views.ts` pasa de 13 a **10 ids** (8 proyecciones + settings + mcp) + se **eliminan** los bindings `key` de view (colisión con §1.5); (2) `overview` → se construye Mission Rail + Activity Spine + StatusBar con el contenido actual; (3) `agent` → el `AgentCard` se monta en kanban/canvas y `DemoPane` se abre con `:d`. Justificación de no borrar todo: **el 100% de los componentes ya existen y están cableados** [O fcs §(g)] — la transición es **de contenedor**, no de reescritura.

### 1.12 Citas de fichas del Researcher por componente [O `research/ux-catalog/*`]

> Cada decisión de diseño se respalda en una ficha del Researcher. Fichero entregado; la sección concreta se cita. Donde no hay ficha ⇒ **[P]**.

| Componente | Ficha `ux-catalog/*.md` (sección) | Estado |
|---|---|---|
| Mission Rail | `ade-orquestador.md` §Enlaces → CONSENSUS_AUDIT §2.1 (Herdr: rollup pane→tab→workspace) | [O] |
| Activity Spine | `observabilidad.md` §Arize Phoenix (traza paso a paso) + `ade-orquestador.md` §Devin (progress steps clicables) | [O] |
| Agent Card | `ade-orquestador.md` §Devin (takeover + steps) + `terminal.md` §Warp ("you approve before anything lands") | [O] |
| Approvals Dock | `terminal.md` §Warp (approve-before-lands; Agent Profiles/permissions) + `ade-orquestador.md` §OpenClaw (full vs sandboxed) | [O] |
| Command Palette | `terminal.md` §opencode (leader key `ctrl+x` + `/` slash) | [O] |
| Kanban | `kanban-pm.md` **§Linear [Os]** (board/list parity, agrupación conmutable, swimlanes, `+` por columna, atajos Cmd-B / X / Shift-X) | [Os] |
| Canvas | `canvas.md` **§n8n [Link→CONSENSUS_AUDIT §2.9]** (canvas+log acoplado, sync bidireccional, auto-expand error) + **§Langfuse [Link]** (Agg↔Expanded) + **§Cate [P]** (canvas infinito) | [Os parcial: n8n/Langfuse vía Link; Cate/Dify/Flowise [P]] |
| Outline | `outline.md` **§Workflowy [Os parcial]** (una jerarquía, zoom a nodo, kanban derivado) + **§Zed Outline Panel [Os parcial]** (git-status por nodo, indent guides, dock) | [Os parcial; Notion [Link], Obsidian [P]] |
| Timeline | `observabilidad.md` §LangSmith (vistas conmutables Messages/Turns/Details) | [O] |
| Cost & Res | `ade-orquestador.md` §DeerFlow (token budget agrega subagentes) + `observabilidad.md` §Helicone (gateway+obs acoplados) | [O] |
| Health KPIs | `observabilidad.md` §Arize Phoenix (scores sobre spans) + `ade-orquestador.md` §claude-flow (health checks plugin) | [O] |
| Audit | `observabilidad.md` §Arize Phoenix (paso a paso) — bloque-explorer sin ficha | [O parcial] |
| Worktrees | `ade-orquestador.md` §Orquestadores → Crystal (parallel git worktrees) | [O] |
| Settings | `settings.md` **§VS Code [Os]** (GUI+JSON dual, Profiles, Sync) + **§Cursor [Os]** (hub Customize: Rules/Skills/MCP/Hooks) | [Os] |
| MCP + Skills | `ade-orquestador.md` §Orquestadores → Omnigent (sandbox por terminal) + §OpenClaw (full vs sandboxed) | [O] |

### 1.13 Alcance v1/v2 por componente (no prometer lo que no existe) [O fcs §(f)/(g) · rfc-vs-code]

> Corrige la spec donde RFC 24/65 prometían algo que el código no tiene (**rfc-vs-code** #2, #3; y el gap de carga fcs §(f)).

| Componente | Existe hoy [O fcs §(g)] | Estados reales [O fcs §(f)] | **v1 (construir)** | **v2 (diferido)** | Corrección rfc-vs-code |
|---|---|---|---|---|---|
| Mission Rail | **No** (nuevo; sale de disolver `overview`) | n/a | Rail + rollup (orden estable; frecency H-01 = v2) | frecency | — |
| Activity Spine | **No** (nuevo; sale de los tails de overview) | n/a | Spine + clases de color + `aria-live` | ⇄ canvas sync | — |
| Agent Card | `AgentCard.svelte` (solo run_id+tokens) | **error✗ · loading✗ · vacío✓** | 15+ campos + error/loading/collapse | demo video/TTS | — |
| Approvals Dock | `ApprovalQueue.svelte` | error✓@48 · **loading✗** · vacío✓ | Dock gate/pregunta + loading | batch/scope/pauserule | — |
| Command Palette | `CommandPalette.svelte` | **error✗ · loading✗** · vacío✓@117 | Cablear hotkeys RFC 24 §19 | modos `>`/`#` | — |
| Kanban | `KanbanBoard.svelte` | error✓@77 · **loading✗** · vacío✓ | loading + drag-teclado | swimlanes | — |
| Canvas | `CanvasView`+`GraphView` | error✓ · loading✓ · vacío✓ | **fallback a Outline si `dag_mode` off** | grafo completo | fcs §(g) |
| Outline | `OutlineView.svelte` | error/loading/vacío ✓ | comments de task (HUECO H-05) | drag entre objectives | — |
| Timeline | `TimelineView.svelte` | error✓@50 · **loading✗** · vacío✓ | loading + zoom | — | — |
| Cost & Res | `CostDashboard.svelte` | error/loading/vacío ✓ | gauges + budget (HUECO H-06) | — | — |
| Health KPIs | `HealthKPIs.svelte` | error/loading/vacío ✓ | KPIs + degraded | — | — |
| Audit | `AuditTimeline.svelte` | error/loading/vacío ✓ | **aviso "sin verificación de cadena"** (fcs §(g)) | chain-break verify | — |
| Worktrees | `WorktreesView.svelte` | error/loading/vacío ✓ | **v1 = tabla** (como hoy) | **grafo mini-git** (v2) | **#3 GAP-REAL** |
| Settings | `SettingsView.svelte` | error/loading/vacío ✓ | secrets slots (no revela valor) | — | — |
| MCP + Skills | `McpView`+`SkillMcpRail` | error/loading/vacío ✓ | **catálogo + allowlist + probe** | **hot-swap MCP** (v2) | **#2 DOC-ERRONEO · #7** |

---

## 2. Mission Rail (navegación raíz)

- **Propósito [R]:** el eje raíz es la **Mission**, no los ficheros (RFC 66 §3/§4). Lista las misiones con su **rollup de estado** y acceso a nuevas.
- **Decisión de diseño:** rail fijo izquierdo + rollup del estado hijo más severo. — Cita Researcher: `docs/design/CONSENSUS_AUDIT.md` §2.1 (Herdr: rollup pane→tab→workspace), §2.2 (Orca: worktree por agente) **[Herdr/Orca no tienen ficha en `ux-catalog/` → §1.12]**.
- **Datos:** `GET /tail/missions` **[O]** en bootstrap + WS `mission_consolidated`, `mission_locked`, `plan_generated`, `agent_status_changed` (para rollup) **[O]**. Orden por **frecency = HUECO H-01 REAL** [O backend §Cierre: `dir_access`@schema.rs:1502 + `Journal::frecency`@journal/mod.rs:375 existen **para directorios**, sin ruta HUD (`grep frecency src-tauri/src/hud` → 0)]. **UI v1:** orden estable por última actividad; frecency = v2.
- **Componentes + jerarquía:** `MissionRail` → `NewMissionButton` · `MissionListItem` (spine de color + nombre + `RollupBadge` + contador) · `FilterChips` (status/model/cost/skill — RFC 24 §1) · `ProfileSwitcher` (TopBar).
- **Estados:**
  - *Carga:* 3 skeletons de item.
  - *Vacío:* "Sin misiones. [ + New Mission ]" (CTA abre Prompt Understanding, RFC 23).
  - *Error:* item "no se pudo cargar misiones · Retry".
  - *Parcial:* badge "parcial · feature dag off" si aplica.
  - *Desconectado:* punto gris + "reconectando"; lista congelada con hora de último sync.
  - *Unknown:* misión con estado hijo no clasificable → glifo `?`.
- **Interacciones + atajos:** click = cambiar misión activa (el lienzo no pierde el stream — CSR in-place, `views.ts` **[O]**). `:m` cambia misión (RFC 24 §19). `+ New` → **HUECO H-09: no hay ruta REST de "nueva misión"** (solo CLI `atlas mission new`; `KernelCommand::NewMissionFromPrompt` es dead code) — ver §20.
- **Teclado/foco:** `↑/↓` mueven selección; `Enter` activa; `Home/End` extremos; rol `navigation` (`aria-label="Missions"`), item `aria-current="true"`.
- **A11y:** cada item expone nombre + estado como texto (no solo color); rollup anunciado (`aria-label="4 agents, 1 blocked"`).
- **Responsive:** ≤1100 → drawer colapsable; ≤720 → pantalla "missions" en modo review (§1.8).
- **Criterios de aceptación (verificables):**
  1. Cambiar de misión en el rail **no** reinicia la conexión WS (`hud.connected` no parpadea) **[O verificable en test]**.
  2. El rollup refleja el hijo más severo: dado un hijo `doom_loop`, el item muestra color `err` + `1 blocked`.
  3. Con `/tail/missions` en 500 → aparece estado *Error* con `Retry`, sin romper el resto del HUD.
- **Build order/deps:** **P0**, depende del contrato de datos (§1.7). Sin endpoint de frecency: construir con orden estable y dejar el HUECO.

---

## 3. Activity Spine (ticker vivo)

- **Propósito [R]:** ver QUÉ está pasando en orden cronológico, escaneable de un vistazo (RFC 24 §11).
- **Decisión de diseño:** columna derecha (no panel inferior) + color por clase de evento + **sync con Canvas** (OA parcial) — Cita Researcher: `n8n.md` (log anclado + sync selección), `docs/design/CONSENSUS_AUDIT.md` §2.1 (Herdr: ticker de estado) **[cita ficha Researcher → §1.12]**.
- **Datos:** WS `/ws` (todos los eventos) **[O]**; backfill `GET /hud/journal` (paginado) **[O]**; `GET /tail/journal` (últimos N) **[O]**.
- **Componentes + jerarquía:** `ActivitySpine` → header (`live` dot + `⇄ canvas` toggle) · `EventRow` (icono + actor + texto humano + `ts`) · `LoadMore`.
- **Clases de color (RFC 24 §11 [O]):** info=muted, success/code-merged=ok, approval/waiting=warn, cost/degradation=warn, doom/goal-drift/error=err, research/learning=violet, steer=info.
- **Estados:**
  - *Carga:* 4 filas skeleton.
  - *Vacío:* "Sin actividad todavía".
  - *Error:* "stream caído · Reintentar" (mantiene últimos eventos).
  - *Parcial:* si `MAX_EVENTS` truncó → "mostrando últimos N".
  - *Desconectado:* header `live`→`off` + "reconectando 1s".
  - *Unknown:* evento con `type` no reconocido → fila neutra "evento desconocido".
- **Interacciones/atajos:** click fila → salta a la card/nodo (si `⇄ canvas` on, selecciona el nodo). `:a` foco a approvals. `:e` expandir canvas.
- **Teclado/foco:** `↑/↓` recorren filas; `Enter` salta; región `aria-live="polite"` (anuncia eventos sin robar foco).
- **A11y:** `aria-live=polite` con throttle (no saturar lectores); icono con `aria-hidden`, texto siempre presente.
- **Responsive:** ≤1100 → colapsable a badge con contador; ≤720 → fundida en "modo review".
- **Criterios de aceptación:**
  1. Un evento `doom_loop_detected` aparece en <250ms (objetivo RFC 17 §10) con clase `err`.
  2. Con WS cerrado, el header muestra `off` y la reconexión 1s/2s/4s… se observa.
  3. `⇄ canvas` activa sincroniza selección bidireccional (ver §8 Canvas).
- **Build order/deps:** **P0**. Depende del WS store (`hud.ts`, ya existe **[O]**).

---

## 4. Agent Card (ciudadano de primera clase)

- **Propósito [R]:** representar un run de agente con 15+ campos, en capas de disclosure, con acciones por estado (RFC 24 §3.1).
- **Decisión de diseño:** disclosure en capas **0–3** (0 = colapso al completar, "Worked for Nm"); spine de estado; barra de acciones gated por estado. — Cita Researcher: `zed.md` (colapso de turno al completar), `docs/design/CONSENSUS_AUDIT.md` §2.3 (Orca: estado de worker de 1ª clase; Herdr: 5 estados) **[cita ficha Researcher → §1.12]**.
- **Datos:** WS `agent_status_changed, agent_diff, agent_tokens, agent_step, agent_heartbeat, doom_loop_detected, goal_drift_detected, model_swapped, step_phase_changed` **[O]**; `GET /tail/agent_steps` **[O]**; drill-down `GET /payload/:kind/:id`, `GET /hud/journal` **[O]**. Worktree/branch → `GET /hud/worktrees` **[O]**.
- **Campos (RFC 24 §3.1 [O]):** id/role/model · mission_ref · status · confidence+judgment · files_touched+diff · plan link · evidence links · tokens in/out/cost · tool_calls · skill activa · execution mode · modo_uso · elapsed/eta · last_heartbeat · worktree/branch/dirty · doom_loop_count/goal_drift · checkpoint_id · demo (preview/screenshot).
- **Componentes + jerarquía:** `AgentCard` → `StatusSpine` + `Header` (role·model·time) + `MissionRef` + `StatLine` + `Layer2 (grid)` + `ConfidenceMeter` + `Tags (skill·mode·uso)` + `Layer0 (turn-collapse)` + `Actions`.
- **Estados:** los 6 base + **por estado de agente** (§1.3), con reglas: `DoomLoop`→overlay rojo + `Recover/Override/Stop`; `Success/Error`→**colapsa a Layer 0**; `Unknown`→glifo `?`.
- **Interacciones/atajos:** acciones RFC 24 §3.2 `▶ ⏸ ⏹ 🔱 🚑 💬 👁 🎬`; **steer inline** (OA-66-06(c)) + hilo completo desde `👁 Reason`; hotkeys `:r :p :x :f :s :d :c`.
- **Teclado/foco:** card enfocable (`tabindex=0`), `Enter` abre Layer 3, `Space` selecciona, acciones alcanzables por `Tab`.
- **A11y:** estado = color+glifo+label; `role="article"` + `aria-label` con rol+estado+tiempo; pulse respeta `prefers-reduced-motion` **[O]**.
- **Responsive:** grid de cards 3→2→1 columnas; acciones en menú `⋯` en ≤720.
- **Criterios de aceptación:**
  1. Al recibir `agent_status_changed → success`, la card colapsa a Layer 0 con "Worked for Nm" y el detalle es expandible.
  2. `doom_loop_detected` pinta overlay `err` y **solo** ofrece Recover/Override/Stop.
  3. Cada acción llama a su **endpoint REST** (`approve`/`deny`) o CLI (`atlas steer`) — **`KernelCommand` es dead code** [O backend §(b)], no es contrato de UI.
- **Build order/deps:** **P0** (base del HUD). Hoy `AgentCard.svelte` solo muestra `run_id`+tokens **[O]** → reemplazar.

---

## 5. Approvals Dock (HITL)

- **Propósito [R]:** resolver acciones sensibles sin mezclar niveles (RFC 24 §5, RFC 66 §7).
- **Decisión de diseño (OA-66-07=(b)):** **dos canales** — *gate* bloqueante vs *pregunta* async [R]. — Cita Researcher: `docs/design/CONSENSUS_AUDIT.md` §2.4 (Orca: decision gate vs `ask`) **[cita ficha Researcher → §1.12]**.
- **Datos:** WS `approval_request` / `approval_decision` **[O]**; `POST /hud/approvals/:id/approve` y `/deny` **[O]**. **Batch/scope/pauserule = HUECO H-02 REAL** [O backend §(0.3): solo approve/deny individuales; `reason` se descarta].
- **Componentes:** `ApprovalsDock` → `GateChannel` (bloqueante) · `QuestionChannel` (async) · `ApprovalRow` (actor + acción + `pattern` + `scope`) · `BatchBar` · `ActionButtons` (`Apr/Deny/Steer/Fork` **[O bus.rs ApprovalDecisionKind]**).
- **Estados:** *Carga* (skeleton fila) · *Vacío* ("sin aprobaciones") · *Error* (POST falla → fila vuelve + toast err) · *Parcial* (batch con conflictos de fichero → deshabilita "Approve all" + explicar) · *Desconectado* (dock congelado + banner) · *Unknown* (acción no clasificable → requiere decisión humana explícita).
- **Interacciones/atajos:** `:a` foco al dock; `Apr/Deny` one-keystroke; batch multi-select con `Space`.
- **Teclado/foco:** el dock **bloquea** la acción del agente hasta decisión (gate) → foco se lleva al dock si `pauserule` lo marca; `aria-live="assertive"` para gates, `polite` para preguntas.
- **A11y:** color+label por decisión (`ok/err/info/violet` [O PALETTE §5]); nunca auto-descartable.
- **Responsive:** ≤720 → botones full-width one-hand `APR/DENY`.
- **Criterios de aceptación:**
  1. Un `approval_request` de patrón `network_binding` con pauserule `pause_always` **detiene** el agente y aparece en el canal *gate*.
  2. `Apr` llama `POST /hud/approvals/:id/approve` y la fila desaparece; `Deny` idem.
  3. Un batch con conflicto de fichero deshabilita "Approve all" y explica por qué.
- **Build order/deps:** **P0**. Depende de **H-02** (batch/scope) para el batch; gate/pregunta 1-a-1 es v1.

---

## 6. Command Palette

- **Propósito [R]:** acceso rápido por teclado a acciones y misiones (RFC 24 §19).
- **Decisión de diseño:** prefijo `:` + fuzzy; **extiende** a acciones de agente; convención declarada como **TUI-style** (no IDE-style) [R]. — Cita Researcher: `vscode.md` (palette con modos), `docs/design/CONSENSUS_AUDIT.md` §2.8 (Herdr: prefix/navigate) **[cita ficha Researcher → §1.12]**.
- **Datos:** acciones = **REST reales** (`POST /hud/approvals/:id/{approve,deny}`, `POST /hud/skills/:id/activate`, `POST /hud/mcp/probe`) + navegación de views **[O]**; misiones `GET /tail/missions` **[O]**. **`KernelCommand` NO es contrato (dead code).**
- **Componentes:** `CommandPalette` → `Input` (con prefijo `:`/`>`/`#`) · `ResultList` (fuzzy) · `ShortcutHint`.
- **Estados:** *Carga* (—; local) · *Vacío* ("sin resultados para 'x'") · *Error* (comando rechazado → toast) · *Parcial* (entidades no cargadas aún) · *Desconectado* (comandos que requieren backend deshabilitados con razón) · *Unknown* (—).
- **Interacciones/atajos:** `:` abre; `Esc` cierra; `↑/↓` navegan; `Enter` ejecuta; `Tab` autocompleta.
- **Teclado/foco:** foco atrapado mientras abierta; overlay `role="dialog"` `aria-modal`.
- **A11y:** resultados con `role="listbox"/"option"` + `aria-activedescendant`; anillo de foco visible.
- **Responsive:** overlay full-width ≤720.
- **Criterios de aceptación:**
  1. `:v` cambia de view sin recargar (store `activeView` **[O]**).
  2. Ejecutar una acción de card llama su **endpoint REST** correspondiente y la card refleja el estado por WS.
- **Build order/deps:** **P0**. `CommandPalette.svelte` ya existe **[O]**; reconciliar hotkeys con `views.ts` (D-66-05).

---

## 7. Kanban (view `kanban`)

- **Propósito [R]:** ¿qué está bloqueado / en progreso / listo para revisar? (RFC 24 §2).
- **Decisión:** tablero por columna de estado de agente, cards = Agent Card (§4), drag = reordenar prioridad (no ejecutar). — Cita Researcher: `vibekanban.md` (kanban de issues→workspace), `linear.md` (board + swimlanes) **[cita ficha Researcher → §1.12]**.
- **Datos:** `GET /tail/missions|plans|diffs|checkpoints` **[O]**; WS `mission_consolidated, plan_generated, step_phase_changed, agent_status_changed` **[O]**.
- **Componentes:** `KanbanBoard` → `KanbanColumn` (Queued/In progress/Review/Done) · `AgentCard` · `TaskCard`; drag handle.
- **Estados:** *Carga* (columnas skeleton) · *Vacío* (columna "—" + CTA) · *Error* (por columna, con Retry) · *Parcial* (misión sin plan aún) · *Desconectado* (banner + cards congeladas) · *Unknown* (card con estado `?`).
- **Interacciones/atajos:** `:v` cambia view; click card → Layer 3; acciones inline (§4).
- **Teclado/foco:** `←/→` entre columnas, `↑/↓` entre cards; `Space` selecciona; drag alternativo por teclado (mover con `Ctrl+↑/↓`).
- **A11y:** columnas con `role="list"`, cards `role="listitem"`; drag tiene alternativa por teclado (WCAG 2.5.7).
- **Responsive:** 3→2→1 columnas; ≤720 = modo review (sin drag).
- **Criterios de aceptación:** (1) una card con `doom_loop` vive en columna Review con overlay err; (2) con `/tail/plans` 500, esa columna muestra error y las demás siguen.
- **Build order/deps:** **P0**. `KanbanBoard.svelte` existe **[O]**.

---

## 8. Canvas (view `canvas`)

- **Propósito [R]:** ¿cómo están cableados los subagentes? (grafo) + execution log por nodo (RFC 24 §13).
- **Decisión:** grafo dirigido; **toggle Agregado↔Expandido** (Langfuse) + **log anclado con sync de selección** (n8n) [R] (RFC 66 §10). — Cita Researcher: `n8n.md`, `langfuse.md` **[cita ficha Researcher → §1.12]**.
- **Datos:** `GET /graph/:mission_id` **[O, feature `dag_mode` server.rs:152-153]**; WS `plan_generated, step_phase_changed, agent_step` **[O]**. Con `dag_mode` off → **H-04**: degradar a **Outline** (el dato `mission_graph_*` se crea unconditional; falta des-gatear la lectura).
- **Componentes:** `CanvasView` → `GraphView` (nodos=subagentes, edges=handoff) · `NodePanel` (execution log / reasoning trail) · `ProjectionToggle (Aggregated|Expanded)` · `Retry/Continue-from-here`.
- **Estados:** *Carga* (skeleton grafo) · *Vacío* ("sin DAG; activa dag_mode") · *Error* (fetch `/graph` falla) · *Parcial* (feature off → banner + degradar a Outline) · *Desconectado* (nodos congelados) · *Unknown* (nodo sin evento `agent_step`).
- **Interacciones/atajos:** click nodo → selecciona + panel log; toggle Agg↔Exp; `:e` expandir canvas full; **selección sincroniza con Activity Spine** (§3).
- **Teclado/foco:** `Tab` recorre nodos; `Enter` abre log; pan/zoom por teclado (`+/-`, flechas).
- **A11y:** grafo con lista alternativa (árbol) para lectores de pantalla; nodo con `aria-label` rol+estado.
- **Responsive:** ≤1100 canvas colapsa a lista/Outline.
- **Criterios de aceptación:** (1) con `dag_mode` off, la view muestra *Parcial* y ofrece Outline; (2) seleccionar nodo resalta la entrada de log correspondiente y la fila de la Spine.
- **Build order/deps:** **P2**. `CanvasView.svelte`+`GraphView.svelte` existen **[O]**; depende de feature `dag_mode`.

---

## 9. Outline (view `outline`)

- **Propósito [R]:** ¿cómo se descompuso la misión? (árbol Objective→Task→Subtask) (RFC 24 §14).
- **Decisión:** árbol Notion-like con properties por task + comments; reordenar tasks entre objectives. — Cita Researcher: `notion.md` (cada item es página, properties, comments) **[cita ficha Researcher → §1.12]**.
- **Datos:** `GET /tail/plans|consolidated|verdicts` **[O]**; WS `plan_generated, mission_consolidated, mission_locked` **[O]**. Comments/annotations → `GET/POST /diff/:id/annotation` **[O]** (a nivel de diff; comment de task = **H-05**, v2).
- **Componentes:** `OutlineView` → `ObjectiveGroup` → `TaskRow` (status pill + assignee + model + effort) · `CommentThread`.
- **Estados:** *Carga* (árbol skeleton) · *Vacío* ("sin plan") · *Error* · *Parcial* (plan sin verdict consolidado) · *Desconectado* · *Unknown* (task sin estado).
- **Interacciones/atajos:** hover task → `💬 Comment`; drag entre objectives; `:c` comenta selección.
- **Teclado/foco:** tree con `role="tree"/"treeitem"`, `→/←` expanden/colapsan, `↑/↓` navegan.
- **A11y:** `aria-expanded`, niveles anunciados; comentario accesible.
- **Responsive:** 1 columna; indentación reducida ≤720.
- **Criterios de aceptación:** (1) un `mission_consolidated` pinta el badge de confidence en la raíz; (2) colapsar/expandir objective persiste en la sesión.
- **Build order/deps:** **P2**. `OutlineView.svelte` existe **[O]**.

---

## 10. Timeline (view `timeline`)

- **Propósito [R]:** ¿cuándo pasó cada cosa? (eje tiempo × tracks de agente) (RFC 24 §15).
- **Decisión:** diamonds por evento (◆ tool, ◇ heartbeat, ✱ decisión, ★ error); zoom; click → frame demo/audit. — Cita Researcher: `langfuse.md` (traza/sesión secuencial), `n8n.md` (runs) **[cita ficha Researcher → §1.12]**.
- **Datos:** `GET /hud/journal` (paginado, `?limit&offset&kind`) **[O observer.rs:116]**; `GET /tail/journal` **[O]**; WS todos **[O]**.
- **Componentes:** `TimelineView` → `AgentTrack` · `EventDiamond` · `ZoomControl` · `TimeAxis`.
- **Estados:** *Carga* (tracks skeleton) · *Vacío* ("sin eventos") · *Error* · *Parcial* (paginado: "cargar más") · *Desconectado* (eje congelado) · *Unknown* (evento con `type` desconocido → marca neutra).
- **Interacciones/atajos:** click ✱ → audit; click ◆ → demo; `+/-` zoom; rueda = zoom horizontal.
- **Teclado/foco:** `←/→` mueven el cursor temporal; `Enter` abre el evento; `Home/End` extremos.
- **A11y:** cada diamond es un botón con `aria-label` (agente + tipo + hora); tabla alternativa.
- **Responsive:** ≤1100 → lista cronológica (colapsa el eje).
- **Criterios de aceptación:** (1) el rango temporal cubre [primer, último] evento; (2) con `/hud/journal` paginado, "cargar más" añade sin perder posición.
- **Build order/deps:** **P2**. `TimelineView.svelte` existe **[O]**.

---

## 11. Cost & Res (view `cost`)

- **Propósito [R]:** ¿cuánto cuesta y qué recursos gasta? (RFC 24 §6).
- **Decisión:** coste por agente/misión + VRAM/RAM/throughput + budget; alertas 80%/100%. — Cita Researcher: `docs/design/CONSENSUS_AUDIT.md` §1.3 (Hermes [I]: cost/token en status bar), `grafana.md` (dashboards + alerting ligado a panel) **[cita ficha Researcher → §1.12]**.
- **Datos:** `GET /hud/cost` **[O]**; WS `agent_tokens, cost_threshold_crossed, hardware_snapshot, spend_limit_observed` **[O]**. Budget restante (RFC 19) → en `/hud/cost`? **[P verificar Builder]**.
- **Componentes:** `CostDashboard` → `CostSparkline` · `ModelBreakdown` · `VramGauge` · `RamGauge` · `ThroughputChart` · `BudgetBar` · `CostAlert`.
- **Estados:** *Carga* (gauges skeleton) · *Vacío* ("sin uso registrado") · *Error* · *Parcial* (hardware sensor ausente → "n/d") · *Desconectado* · *Unknown* (modelo sin precio → "?" + nota).
- **Interacciones/atajos:** hover punto → tooltip; click modelo → filtra; `:o` modo.
- **Teclado/foco:** gauges con valor textual accesible; tabs de breakdown navegables.
- **A11y:** gráficos con tabla de datos alternativa; alertas también por texto, no solo color.
- **Responsive:** grid 2→1; ≤720 tarjetas apiladas.
- **Criterios de aceptación:** (1) `cost_threshold_crossed` al 80% pinta barra warn y al 100% err; (2) sin sensor VRAM → *Parcial* "n/d", sin error.
- **Build order/deps:** **P1**. `CostDashboard.svelte` existe **[O]**.

---

## 12. Health KPIs (view `health`)

- **Propósito [R]:** ¿están sanos los subagentes? (RFC 24 §7).
- **Decisión:** KPIs en una banda + checker status del Validation Engine. — Cita Researcher: `docs/design/CONSENSUS_AUDIT.md` §1.3 (Hermes [I]: health WS), `grafana.md` (paneles de estado) **[cita ficha Researcher → §1.12]**.
- **Datos:** `GET /hud/health` **[O]**; `GET /hud/reliability` **[O]**; `GET /hud/availability` **[O]**; `GET /hud/eval/summary` **[O]**; WS `agent_heartbeat, agent_status_changed` **[O]**.
- **Componentes:** `HealthKPIs` → `AgentHealthRollup` (healthy/degraded/doom) · `AvgConfidence` · `Heartbeat` · `QueueDepth` · `CheckerStatus` (LSP/tsc/vitest/biome) · `JournalWritesPerS` · `HudLatency` · `AvailabilityCard` · `EvalCard`.
- **Estados:** *Carga* · *Vacío* (no agents) · *Error* · *Parcial* (eval/reliability no disponibles) · *Desconectado* (heartbeat "hace Ns") · *Unknown* (agente sin heartbeat ≥10s → **degraded**, ≥30s → revive (RFC 24 §7 [O])).
- **Interacciones/atajos:** `:t` toggle dock KPIs; click checker → detalle.
- **Teclado/foco:** valores con texto; tabs navegables.
- **A11y:** umbrales por texto+color; `aria-live` para cambios de salud críticos.
- **Responsive:** banda → wrap 2 filas ≤1100.
- **Criterios de aceptación:** (1) sin heartbeat ≥10s el agente pasa a *degraded*; (2) `HudLatency` refleja `now - ts` del último evento.
- **Build order/deps:** **P1**. `HealthKPIs.svelte`+`AvailabilityCard`+`EvalCard` existen **[O]**.

---

## 13. Audit (view `audit`)

- **Propósito [R]:** ¿qué decisiones sensibles se tomaron y por quién? (hash-chain, RFC 24 §10).
- **Decisión:** timeline append-only tipo block-explorer + export posting. — Cita Researcher: `grafana.md` (log/estado), patrón SOC2/block-explorer (RFC 24) **[cita ficha Researcher → §1.12]**.
- **Datos:** `GET /hud/audit` **[O]**; `POST /audit/export-posting` **[O export.rs:102]**; `GET /payload/:kind/:id` **[O]**; WS `approval_decision, skill_activated` **[O]**.
- **Componentes:** `AuditTimeline` → `AuditEntry` (actor+action+inputs/outputs+`prev`/`hash`+`signature?`) · `Filters` · `ExportButton` (Tauri save dialog) · `ChainBreakAlert`.
- **Estados:** *Carga* · *Vacío* ("sin entradas sensibles") · *Error* · *Parcial* (carga paginada) · *Desconectado* · *Unknown* (entrada sin firma → marca "unsigned").
- **Interacciones/atajos:** filtros actor/mission/type/time; click entrada → payload; export → `POST /audit/export-posting`.
- **Teclado/foco:** lista navegable; filtros como formulario accesible.
- **A11y:** hash legible por copia; "chain break" anunciado (`role=alert`).
- **Responsive:** columnas → tarjetas apiladas ≤720.
- **Criterios de aceptación:** (1) recomputar hashes detecta un break → alerta roja; (2) export escribe `.posting.yaml` en el dir elegido (`files_written` en respuesta **[O hud.ts ExportPostingResponse]**).
- **Build order/deps:** **P1**. `AuditTimeline.svelte` existe **[O]**.

---

## 14. Worktrees (view `worktrees`)

- **Propósito [R]:** ¿en qué branch está cada subagente? (mini-git-graph, RFC 24 §12).
- **Decisión:** árbol por misión con worktrees como nodos; dirty → color. — Cita Researcher: `docs/design/CONSENSUS_AUDIT.md` §2.2 (Orca: worktree por agente), `vibekanban.md` (1 worktree/agente) **[cita ficha Researcher → §1.12]**.
- **Datos:** `GET /hud/worktrees` **[O]**; WS `worktree_dirty` **[O]**.
- **Componentes:** `WorktreesView` → `MissionGraph` (main + ramas) · `WorktreeNode` (path/branch/dirty) · `Actions` (Close/Merge/Compare).
- **Estados:** *Carga* · *Vacío* ("sin worktrees") · *Error* · *Parcial* (worktree sin branch detectado) · *Desconectado* · *Unknown* (dirty no determinable → "?").
- **Interacciones/atajos:** click nodo → branch; acciones Close/Merge/Compare.
- **Teclado/foco:** nodos en `role="tree"`; `Enter` abre acciones.
- **A11y:** dirty por texto+color; rutas copiables.
- **Responsive:** lista plana ≤720.
- **Criterios de aceptación:** (1) `worktree_dirty` cambia el color del nodo en <250ms; (2) "Compare" abre diff.
- **Build order/deps:** **P2**. `WorktreesView.svelte` existe **[O]**.

---

## 15. Settings

- **Propósito [R]:** gestión de secretos (OS keychain) y estado de acceso remoto.
- **Decisión:** vista, no modal bloqueante; nunca muestra valores de secretos. — Cita Researcher: (patrón de settings de producto) **[cita ficha Researcher → §1.12]**.
- **Datos:** `GET/POST /hud/secrets` · `DELETE /hud/secrets/:account` **[O server.rs:134-141]**; `GET /remote/status` **[O]**. Slots en `SecretsResponse { service, slots:[{account,present}] }` **[O hud.ts:299-306]**.
- **Componentes:** `SettingsView` → `SecretSlotRow` (account + `present?` + set/clear) · `RemoteStatusBadge` (bearer/OIDC) · `ProfileSection`.
- **Estados:** *Carga* · *Vacío* (sin slots) · *Error* (keychain no disponible) · *Parcial* (algunos slots) · *Desconectado* · *Unknown* (estado keychain indeterminado).
- **Interacciones/atajos:** set/clear secreto; nunca revela el valor; confirmación al borrar.
- **Teclado/foco:** formulario accesible; input de secreto `type=password` `autocomplete=off`.
- **A11y:** nunca leer el valor en claro; `aria-describedby` con "presente/ausente".
- **Responsive:** 1 columna.
- **Criterios de aceptación:** (1) `POST` guarda y `GET` marca `present=true` sin devolver el valor; (2) `DELETE` requiere confirmación.
- **Build order/deps:** **P1**. `SettingsView.svelte` existe **[O]**.

---

## 16. MCP + Skills

- **Propósito [R]:** catálogo MCP con política (sandbox/supply) + allowlist editable + probe; activación de skills (RFC 24 §8).
- **Decisión:** editor de allowlist **en sitio**; activación de skill por drop/click; MCP read + probe. — Cita Researcher: `docs/design/CONSENSUS_AUDIT.md` §2.1/§1.3 (Herdr; Hermes [I]) **[Herdr/Hermes no tienen ficha en `ux-catalog/` → §1.12]**.
- **Datos:** `GET /hud/mcp` **[O]**, `POST /hud/mcp/allowlist` **[O]**, `POST /hud/mcp/probe` **[O]**; `POST /hud/skills/:id/activate` **[O server.rs:133]**. WS `skill_activated` **[O]**. (RFC 65 §10.13: `allowed_tools`, `sandbox`, `supply`.)
- **Componentes:** `McpView` → `McpServerRow` (allowed_tools + sandbox + supply verdict) · `AllowlistEditor` · `ProbeButton` · `SkillMcpRail` (catálogo + activas, drag-drop).
- **Estados:** *Carga* · *Vacío* ("sin servidores MCP") · *Error* (registro ausente/malformado → `ok:false`+reason **[O]**) · *Parcial* (probe sin conexión) · *Desconectado* · *Unknown* (supply no determinable).
- **Interacciones/atajos:** probe conecta y hace `tools/list`; añadir a allowlist desde chips; drop de skill → activate (validate→snapshot→activate, RFC 24 §8).
- **Teclado/foco:** drag tiene alternativa por teclado (menú "Añadir a allowlist"); probe enfocable.
- **A11y:** supply BLOCK por texto+color; rechazo de probe anunciado.
- **Responsive:** 2→1 columnas.
- **Criterios de aceptación:** (1) probe contra un server válido devuelve `tools`; contra uno BLOCK de supply, rechaza con razón; (2) `activate` publica `skill_activated` en el WS.
- **Build order/deps:** **P1**. `McpView.svelte`+`SkillMcpRail.svelte` existen **[O]**.

---

## 17. Orden de construcción y dependencias

| Fase | Componentes | Depende de |
|---|---|---|
| **P0** (base) | Design tokens (`src/app.css`) · Agent Card · Mission Rail · Activity Spine · Kanban · Approvals Dock · Command Palette | contrato de datos (Builder) |
| **P1** | Cost & Res · Health KPIs · Audit · Settings · MCP/Skills | P0 |
| **P2** | Canvas (feature `dag_mode`) · Outline · Timeline · Worktrees | P0/P1 + features |
| **Cross-cut** | Layout maestro (§1.1) · reconciliación hotkeys `views.ts`↔RFC 24 §19 · rollup rail · canvas↔spine sync | P0 |

**Regla [R]:** ningún componente se construye antes de que su endpoint/evento esté confirmado (o el HUECO resuelto por Builder).
**Alcance v1/v2 (detalle por componente en §1.13):** **v1** = tokens (`app.css`) + base P0 + las 8 proyecciones existentes con sus estados de carga/error corregidos; **v2** = hot-swap MCP, grafo de Worktrees, demo video/TTS, batch/scope/pauserule de aprobaciones, ⇄ canvas sync, drag entre objectives, sesión multi-agente. **No se promete en v1 lo que `rfc-vs-code` marcó como no implementado (#2, #3).**

---

## 18. Cierre FASE 9 — APCA y CVD (medidos, no a ojo)

Script: OKLCH→sRGB + **APCA-W3 0.0.98G** + **Machado 2009** (severity 1.0) sobre linear RGB + **CIE76 ΔE**. Tokens de `mockup.html` **[O]**.

**APCA (Lc; negativo = texto claro sobre fondo oscuro). Guía: body≥90, content≥60, large/spot≥45, UI≥30, non-text≥15.**

| Token | vs bg | vs surface | vs surface-2 | Guía |
|---|---|---|---|---|
| text | **−94.2** | −93.1 | −91.5 | body 90 ✓ |
| muted | −56.3 | −55.2 | −53.7 | content 60 ✗ (válido large/bold) |
| faint | −39.5 | −38.3 | −36.8 | large/spot 45 ✗ (válido spot/placeholder) |
| primary | −70.3 | −69.1 | −67.6 | UI 30 ✓ |
| info | −58.1 | −57.0 | −55.5 | UI 30 ✓ |
| ok | −67.0 | −65.8 | −64.3 | UI 30 ✓ |
| warn | −70.7 | −69.6 | −68.0 | UI 30 ✓ |
| err | −51.2 | −50.0 | −48.5 | UI 30 ✓ |
| violet | −57.6 | −56.5 | −55.0 | UI 30 ✓ |
| border-ui | −31.2 | −30.1 | −28.6 | non-text 15 ✓ |
| bg on primary (label) | **+70.9** | — | — | label ✓ |

**Conclusión APCA:** `text` cumple body; **`muted` y `faint` no llegan a 60/45** → **regla**: a tamaño ≤14px usar `--a-text` + glifo; `muted`/`faint` solo para texto grande/negrita, metadatos no críticos y placeholders. (Coincide con PALETTE §4.1 **[O]**.)

**CVD (ΔE CIE76; <10 = confundible):**

| Par | normal | protanopia | deuteranopia | tritanopia |
|---|---|---|---|---|
| err / ok | 98.1 | 27.9 | **8.8** | 103.2 |
| err / warn | 54.6 | 44.8 | 27.0 | 37.2 |
| warn / info | 95.6 | 92.1 | 98.5 | 74.3 |
| ok / info | 76.9 | 70.4 | 66.7 | **14.0** |
| primary / info | 43.8 | 33.8 | 30.4 | **13.3** |

**Conclusión CVD:** bajo **deuteranopia** `err↔ok` colapsa (ΔE 8.8) y bajo **tritanopia** `info↔primary` (13.3) y `ok↔info` (14.0) se acercan. Mitigación **obligatoria y ya en la spec**: estado = **color + glifo + label** (§1.4; `ok=✓`, `err=✖/⚠`, `info=▶`, `primary`=interactivo no-estado). **Cero contenido depende solo del color.** [O]

**Pendiente [P]:** capturas reales en simulador (Chrome DevTools) como confirmación visual; los números ya están medidos.

---

## 19. Verificación cruzada — **CERRADA** (backend · frontend · RFC↔código · catálogo)

> **Sin bloqueos.** Todas las dependencias entregadas y cotejadas 1:1.

| Fuente | Ruta | Estado | Aplicado |
|---|---|---|---|
| Builder | `docs/audit/backend-capabilities.md` | **[O entregado]** | ✅ **§1.7 cotejado** (41 rutas con handler+payload+test; 26 eventos; `KernelCommand`=dead code); **§20 veredictos H-01…H-09** |
| Builder-2 | `docs/audit/frontend-current-state.md` | **[O entregado]** | ✅ §1.2, §1.5, §1.10 (f)/(g), §1.11, §1.13 |
| Builder-3 | `docs/audit/rfc-vs-code.md` | **[O entregado]** | ✅ #2 (MCP hot-swap), #3 (Worktrees grafo), #7 (MCP write+probe), #9 (13 views) |
| Researcher | `Atlas OS/research/64 - UX reference catalog.md` | **[O entregado]** | ✅ recuento **441 / 94** HUD-relevantes |
| Researcher | `Atlas OS/research/ux-catalog/*.md` | **[O]** | ✅ §1.12 cita fichas reales; `ux-catalog` = **132** (`count.mjs`: **62 [Os] / 55 [Os parcial] / 15 [P]**; 10 de categoría) |

**Correcciones de contrato que el cotejo obligó [O backend]:** (a) **`KernelCommand` = dead code** ⇒ la UI usa **REST**, no comandos kernel; (b) **WS solo-bajada** (cliente→servidor ignorado, ws.rs:33-47) ⇒ acciones por REST; (c) `/hud/approvals` **descarta `reason`** (approvals.rs:64); (d) **ninguna ruta autentica por defecto** (loopback + CORS permissive) ⇒ aviso si `atlas serve --host` (RFC 18); (e) `/graph` y `/atlas-calendar.ics` **feature-gated**.

---

## 20. Registro de huecos — **veredicto** [O `backend-capabilities.md` §Cierre, cada uno con `fichero:línea`]

**A. Huecos H-01…H-09 (9/9 REALES) — resolución v1 (degradación) + cambio de backend exigido, con fase:**

| # | Hueco | Veredicto [O] | **UI v1 (degradación)** | **Builder** (fase · cambio concreto) |
|---|---|---|---|---|
| **H-01** | frecency de misiones | REAL (parcial): `dir_access`@schema.rs:1502 + `Journal::frecency`@journal/mod.rs:375 existen **para directorios**; sin ruta HUD | Rail con **orden estable** (última actividad); frecency = **v2** | **FASE 11** · `GET /hud/missions?sort=frecency` (`hud/missions.rs`); extender `frecency` a misiones |
| **H-02** | batch/scope/pauserule | REAL: solo `approve`/`deny` individual; `reason` se descarta (approvals.rs:64) | Dock **1-a-1** (gate/pregunta); batch = **v2** | **FASE 11** · `POST /hud/approvals/batch` + `scope` + **persistir `reason`** (approvals.rs:64) |
| **H-03** | `Unknown` en `AgentStatus` | REAL: enum **10 estados sin `Unknown`** (bus.rs:224-235) | chip `unknown` **derivado en cliente** (heartbeat ausente >30 s) | **FASE 10** · añadir variante `Unknown` a `AgentStatus` (bus.rs) + serde |
| **H-04** | fallback Canvas | REAL (parcial): `mission_graph_*` se crean **unconditional** (schema.rs:642-646) pero la única lectura está gated → **404** | si 404/feature off → **degradar a Outline** + badge *parcial* | **FASE 10** · **des-gatear** `GET /graph/:mission_id` (server.rs:152-153) |
| **H-05** | comments de Task | REAL: solo `/diff/:id/annotation` (+ `diff_annotations`@schema.rs:491) | Outline comenta **por diff** (v1); task/misión = **v2** | **FASE 11** · añadir `task_annotations` (schema + ruta) |
| **H-06** | budget restante | REAL: `/hud/cost` da `pressure{level,warn_usd=5,crit_usd=20}` **sin budget** (cost.rs:59-70) | Cost muestra **pressure** (umbrales 5/20 USD) — es lo que hay | **FASE 10** · añadir `budget_usd`+`remaining` a `cost.rs` + persistir budget |
| **H-07** | demo video/TTS | REAL (declarado, demos.rs:4-6): no implementado; `/hud/demos` = artefactos | **Demo = artefactos** (v1) | **v2** · video/TTS (RFC 24 §9) |
| **H-08** | terminal snapshot | REAL (parcial): `Sandbox::snapshot` existe (f31e6dd) **sin ruta HUD** | Agent Card: `👁 Reason` usa `agent_steps.observation` (v1); snapshot = **v2** | **FASE 11** · puentear `Sandbox::snapshot` → `GET /hud/agent/:run_id/snapshot` |
| **H-09** | **nueva misión por REST** | REAL: **no hay ruta POST de misiones**; `KernelCommand::NewMissionFromPrompt` es dead code (`core/bus.rs:250`, sin consumidor) | `+ New` ejecuta por **CLI/IPC** (v1) | **FASE 10** · `POST /hud/missions` (`hud/missions.rs`) |

**B. Bloqueados por fichas del Researcher:** **ninguno** — `ux-catalog/` = **132** fichas (`count.mjs`: **62 [Os] / 55 [Os parcial] / 15 [P]**); `_pending-A.md` se recalcula con `count.mjs`. No bloquean componentes construibles.

**C. Cerrados/corregidos:** MCP hot-swap→v2 (#2); Worktrees grafo→v2 (#3); "8 views"→**13** (§1.11); "dark/light ya existe"→**falso** (§1.2); hotkeys de view→colisión, se retiran (§1.5); `KernelCommand`→dead code (§1.7).

---

## 21. Changelog

- **2026-10-06 v4.4:** nuevo **§25 FASE 12** (frontend de B3/B4/B6/B8 + cierre de G7): lotes **F11-a** (aprobaciones en lote, Builder-3) · **F11-b** (anotaciones de tarea, Builder-2) · **F11-c** (snapshot del sandbox, Builder-2) · **F11-d** (frecuencia + creación de misión, Builder) — cada uno con wireframe, estados (incl. **conflicto 409**), **copy ES/EN**, hotkeys, criterios verificables y deps; **§25.5** lista exacta de **capturas G7 que faltan** (Approvals Dock real, `:?` en vivo, tema claro con datos) con el estado a reproducir. Nota explícita: no contradice §20.
- **2026-10-06 v4.3:** **§23 desarrollado**: cada función del backlog FASE 11+ (sesiones scrollback/reattach, ping por panel, manifiesto de agente, grid de PTYs) con **referentes [Os] + ficha**, **wireframe textual**, **estados**, **hueco backend + rutas propuestas**, **criterios de aceptación verificables** y **orden de lotes F11-1…F11-4 con dependencias** (§23.5). **§22.3** añade **G9** (cada ruta HUD nueva: test feliz+fallo y presente en `backend-capabilities.md`); cierre FASE 10 pasa a **G1–G9** (G7 en curso).
- **2026-10-06 v4.2:** nuevo **§24 Cabecera y shell** (construible): regiones del shell (§24.1, RFC 66 §5.2), identidad Calm Instrumentation en la cabecera (§24.2, RFC 66 §9.1 + PALETTE §3), contenido con dato real (§24.3), **estados de conexión** color+glifo+label (§24.4), **leader `:`** con `:v :a :n :m :d :?` y comportamiento exacto de `:m` (foco Mission Rail) y `:?` (help overlay, `Esc` cierra) (§24.5), markup Svelte esperado (§24.6), tokens (§24.7) y **criterios de aceptación C1–C10** (§24.8). Añadido lote **F0.5** (§22.1) y **G7** ampliado a la cabecera (§22.3).
- **2026-10-06 v4.1:** **G6** (§22.3) corregido: la regex `#[0-9a-fA-F]{3,8}` daba falsos positivos con `{#each}` (41) y con refs tipo `#10207`; sustituida por `rg -nP "(?<![{])#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b" src -g "*.svelte"` (**0** hoy, verificado ejecutándolo). Alineada la misma regex en la aceptación de **F0** (§22.1).
- **2026-10-06 v4:** ronda C9/C10/C11. **C9:** recuento de fichas unificado a **132** (`count.mjs`: **62 [Os] / 55 [Os parcial] / 15 [P]**; 10 de categoría) — coherente en Status + §1.2/§19/§20 (eliminadas las cifras contradictorias "6 ENTREGADAS" / "9 con ficha" / "62 prioridad-A"). **C10:** citas `herdr.md`/`orca.md` (y `hermes.md`) sustituidas por **`docs/design/CONSENSUS_AUDIT.md`** (§1.3/§2.x) — no existen en `ux-catalog/` (9 líneas: §2/§3/§4/§5/§6/§7/§8). **C11:** tamaños de audit corregidos a **276** (backend) y **277** (frontend). Nuevo **§23 Backlog FASE 11+** (4 funciones NO-DOCUMENTADAS de `_funcion-x-referente.md`).
- **2026-10-06 v3.1:** 5ª pasada (correcciones del PL). **Status** de cabecera alineado con §19 (backend cotejado; fichas Researcher **en curso**). §20: huecos **H-01…H-09 (9/9)**; **H-09** como fila con UI v1 + fase. §22.2 **dividido en FASE 10 (B1/B2/B5/B7) y FASE 11 (B3/B4/B6/B8)** por decisión del PL; "Fuera de FASE 10" reducido a v2 real. **§22.3 gates de cierre**.
- **2026-10-06 v3:** 4ª pasada, **sin bloqueos de backend**. §1.7 **cotejado 1:1** contra `backend-capabilities.md` (41 rutas + handler + payload + test; 26 eventos con payload; `KernelCommand`=dead code). §19 **CERRADA**. §20 con **veredictos H-01…H-09** + resolución UI v1 + cambio de backend exigido. §1.12 cita `outline/settings/kanban-pm/canvas`. Scope corregido a **13 views**. §22 Plan FASE 10. Corregidas 5 referencias a `KernelCommand` en §2/§4/§5/§6.
- **2026-10-06 v2:** encabezado veraz; §1.10 baseline (fcs); §1.11 transición de las 13 views; §1.12 citas; §1.13 v1/v2; §1.5 hotkeys; ajustes `rfc-vs-code` #2/#3/#7/#9.
- **2026-10-06 v1.1 / v1:** esqueleto + `research/64`; FASE 9 (APCA+CVD) en §18.

---

## 22. Plan de implementación FASE 10 (para Builders)

> **Reglas:** lotes pequeños y ordenados; cada lote con **archivos**, **criterio de aceptación verificable**, **dependencias** y **riesgo**. Los lotes de backend exigen **test nuevo: 1 camino feliz + 1 de fallo** (AGENTS.md §4). Nada se construye sin su endpoint confirmado (§1.7). Nunca `npm`; `pnpm`.

### 22.1 Lotes de frontend

| Lote | Qué | Archivos | Criterio de aceptación | Deps | Riesgo |
|---|---|---|---|---|---|
| **F0 — Tokens** | crear `src/app.css` con `--a-*` (PALETTE §3); importar; sustituir los 531 literales | `src/app.css` (nuevo), `src/routes/+layout.svelte`, 24 `.svelte` | `pnpm check && pnpm lint`; `rg -nP "(?<![{])#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b" src -g "*.svelte"` → **0**; captura dark/light | — | medio (53 1 sitios): mitigar con `rg` antes/después |
| **F0.5 — Cabecera y shell** | §24: `TopBar` con identidad + versión + misión activa + bus + leader hint; `HelpOverlay` (`:?`) | `src/lib/components/AppHeader.svelte` (nuevo), `HelpOverlay.svelte` (nuevo), `src/routes/+page.svelte` | `pnpm test` (`AppHeader.test.ts`: C1–C10 §24.8); `pnpm check`; G6 = 0 | F0 | bajo (`:m/:?` se cablean en F5; foco de `:m` depende de F2) |
| **F1 — Agent Card** | reemplazar debug card por §4 (15+ campos, capas 0–3, error/loading) | `src/lib/components/AgentCard.svelte`, `src/lib/stores/hud.ts`, `+page.svelte` | `pnpm test` (test de componente: render + vacío + **error**) ; `pnpm check` | F0 | alto (datos WS/REST; H-03 derivado) |
| **F2 — Mission Rail** | componente nuevo + disolver `overview` | `src/lib/components/MissionRail.svelte` (nuevo), `+page.svelte`, `views.ts` | `pnpm test`; cambiar misión **no** reconecta WS (assert `hud.connected`) | F0/F1 | medio |
| **F3 — Activity Spine** | ticker + clases de color + `aria-live` | `src/lib/components/ActivitySpine.svelte` (nuevo), `+page.svelte` | `pnpm test`; `doom_loop_detected` → clase `err`; `prefers-reduced-motion` | F0 | bajo |
| **F4 — Approvals Dock** | `ApprovalQueue` → Dock gate/pregunta | `src/lib/components/ApprovalQueue.svelte`, `+page.svelte` | `pnpm test`; `Apr`→`POST …/approve`; gate bloquea | F0 | medio (H-02: 1-a-1) |
| **F5 — views.ts/hotkeys** | 13→10 ids; retirar bindings de view; cablear RFC 24 §19 | `views.ts`, `ViewSwitcher.svelte`, `+page.svelte` | `pnpm test` (`views.test`); `:v` cambia view; `:a` enfoca dock | F0 | bajo |
| **F6–F9 — Refactor por view** | loading/error/skeleton + estado real (fcs §(f)) | `KanbanBoard`, `CanvasView`,`GraphView`, `OutlineView`, `TimelineView`, `CostDashboard`, `HealthKPIs`, `AuditTimeline`, `WorktreesView`, `SettingsView`, `McpView` | `pnpm check`; `pnpm test` por componente (render + los 6 estados); captura | F0–F1 | bajo/medio |
| **F10 — MCP/Worktrees scope** | etiquetar v1 (catálogo+allowlist+probe; tabla) vs v2 (hot-swap; grafo) | `McpView.svelte`, `SkillMcpRail.svelte`, `WorktreesView.svelte` | `pnpm check`; no anunciar hot-swap ni grafo | F0 | bajo |

### 22.2 Lotes de backend (cada uno exige test feliz + fallo — AGENTS.md §4)

**FASE 10 — baratos, desbloquean la UI v1 (DECISIÓN DEL PROJECT LEAD):**

| Lote | Cambio (fichero) | Test nuevo exigido | Accept | Riesgo |
|---|---|---|---|---|
| **B1 (H-04)** | des-gatear `/graph/:mission_id` (**server.rs:152-153**) | `hud::graph::tests`: *(feliz)* devuelve grafo con feature OFF; *(fallo)* misión desconocida → 404 | `cargo test … hud::graph` | bajo |
| **B2 (H-03)** | añadir `AgentStatus::Unknown` (**core/bus.rs:224-235**) + serde | `core::bus::tests`: *(feliz)* round-trip `Unknown`; *(fallo)* string inválido rechazado | `cargo test … core::bus` | bajo (10 tags estables) |
| **B5 (H-06)** | `budget_usd`+`remaining` en (**hud/cost.rs:59-70**) | `hud::cost::tests`: *(feliz)* budget presente; *(fallo)* sin budget → `null` | `cargo test … hud::cost` | bajo |
| **B7 (H-09)** | `POST /hud/missions` (nueva misión) + `hud/missions.rs` | `hud::missions::tests`: *(feliz)* crea; *(fallo)* prompt vacío → 400 | `cargo test … hud::missions` | bajo |

**FASE 11 — diferidos tras validar FASE 10; la UI v1 degrada según §20:**

| Lote | Cambio (fichero) | Test nuevo exigido | Accept | Riesgo |
|---|---|---|---|---|
| **B3 (H-02)** | `POST /hud/approvals/batch` + persistir `reason` (**hud/approvals.rs:64**) | `hud::approvals::tests`: *(feliz)* batch sin conflicto → todas; *(fallo)* conflicto de fichero → 409 | `cargo test … hud::approvals` | medio (seguridad RFC 18) |
| **B4 (H-01)** | `GET /hud/missions?sort=frecency` (**hud/missions.rs**) | `hud::missions::tests`: *(feliz)* orden por frecency; *(fallo)* journal vacío → orden estable | `cargo test … hud::missions` | medio |
| **B6 (H-08)** | puentear `Sandbox::snapshot` → `GET /hud/agent/:run_id/snapshot` | `hud::snapshot::tests`: *(feliz)* devuelve frame; *(fallo)* sin sandbox → 404+reason | `cargo test … hud::snapshot` | medio |
| **B8 (H-05)** | `task_annotations` (schema + ruta) | `hud::annotate::tests`: *(feliz)* round-trip task; *(fallo)* body vacío → 422 | `cargo test … hud::annotate` | medio |

**Orden:** F0 → F1 → (F2·F3·F4·F5 en paralelo) → F6–F10 → **FASE 10 backend** (B1/B2/B5/B7, baratos, habilitan H-04/H-03/H-06/H-09) → **gates §22.3** → **FASE 11 backend** (B3/B4/B6/B8). **Cada PR:** `pnpm check && pnpm lint && pnpm test` (frontend) o `cargo clippy … -D warnings && cargo test --manifest-path src-tauri/Cargo.toml --lib <módulo>` (backend), citando el nombre del test.
**Fuera de FASE 10 y FASE 11 (v2 de producto):** hot-swap MCP, grafo de Worktrees, video/TTS, ⇄ canvas sync, sesión multi-agente.

### 22.3 Gates de cierre de FASE 10 (verificables por el Project Lead)

| # | Comando exacto | Resultado esperado |
|---|---|---|
| **G1** | `pnpm check` | 0 errores (svelte-check + tsc) |
| **G2** | `pnpm lint` | 0 warnings (eslint + prettier) |
| **G3** | `pnpm test` | **todo verde**, incluidos los tests de componente nuevos (render + vacío + **error** por view) |
| **G4** | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | 0 warnings |
| **G5** | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | verde; incluye **B1/B2/B5/B7** (cada uno feliz + fallo) |
| **G6** | `rg -nP "(?<![{])#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b" src -g "*.svelte"` | **0** (sin hex crudo; longitudes CSS válidas 3/4/6/8 y `#` no precedido de `{` ⇒ excluye `{#each}` y refs tipo `#10207`; verificado hoy) |
| **G7** | Capturas **dark/light** de **Cabecera** (connected/disconnected), **Mission Rail**, **Activity Spine**, **Approvals Dock** y **1 view** (p.ej. Kanban) | 5 componentes en ambos temas; anillo `:focus-visible` visible |
| **G8** | `rg "KernelCommand" src/lib` | **0** (la UI no se cablea al dead code) |
| **G9** | Para cada **ruta HUD nueva**: `rg -n "<ruta>" docs/audit/backend-capabilities.md` (no vacío) **y** `cargo test --manifest-path src-tauri/Cargo.toml --lib <módulo>` verde (feliz + fallo) | cada ruta HUD nueva **aparece en `backend-capabilities.md`** y tiene **test feliz + fallo** |

**FASE 10 se considera cerrada cuando G1–G9 pasan** (G7 en curso: reserva declarada). FASE 11 backend (B3/B4/B6/B8) se lanzó por decisión del Project Lead antes del cierre completo de G7; G9 cerrado el 2026-10-07.

---

## 23. Backlog FASE 11+ — funciones NO-DOCUMENTADAS (detallado) [R]

> `_funcion-x-referente.md` (Researcher, pasada 4) detectó **4 funciones** presentes en **≥3 referentes** que **no constan en RFC 24/65/66/67**. **DECISIÓN DEL PL: no entran en FASE 10** (cerrada con reservas — G7 parcial). Cada una se detalla abajo: **referentes [Os] con ficha · wireframe · estados · hueco backend + rutas propuestas · criterios verificables · lote F11-x**. Toda ruta HUD nueva debe cumplir **G9** (§22.3).

**Resumen:**

| # | Función | Referentes [Os] (ficha) | Lote | Deps | Hueco backend |
|---|---|---|---|---|---|
| B-23-1 | Sesiones con **scrollback + reattach/resume** | Cate · tlbx · GridBash · CliDeck (`ade-orquestador.md`) | **F11-1** | F10, F0.5 | Persistencia de sesión PTY + ring buffer + replay WS |
| B-23-2 | Panel `running/waiting/finished` + **ping** | Cate · GridBash (`ade-orquestador.md`) · Herdr (`CONSENSUS_AUDIT`) | **F11-2** | F11-1, B2 (H-03) | Estado por panel + cola de atención/ping |
| B-23-3 | **Manifiesto único de agente** editable | Jazz · Smelt · OpenCastle (`ade-orquestador.md`) | **F11-3** | F0.5, F1 | CRUD de manifiesto + validación (tabla) |
| B-23-4 | **Grid de PTYs** multi-pane | GridBash · Cate (`ade-orquestador.md`) · agent-manager/hcom (`_funcion-x-referente.md`) | **F11-4** | F11-1 | PTY host + layout server-side |

### 23.1 F11-1 — Sesiones con scrollback + reattach/resume

**Referentes [Os] (con ficha):**
- **Cate** — `research/ux-catalog/ade-orquestador.md §Cate` (L103): *"las agent sessions sobreviven reinicios (scrollback + resume)"*; paneles en dock/tabs/splits con layout por proyecto.
- **tlbx** — `§tlbx` (L229): *"Sessions survive disconnects"*; control-plane como JSON (API=UI).
- **GridBash** — `§GridBash` (L219): *"restore sessions"*, background panes.
- **CliDeck** — `§CliDeck` (L209): dashboard con *"live status detection, session resume"*.

**Wireframe textual:**
```
┌ Sessions ────────────────┐┌ session: auth-refactor ─────────── [⟳ reattach] ┐
│ ● auth-refactor  running ││ $ opencode mission run auth-refactor           │
│ ◐ dashboard-fix  waiting ││ ... 214 líneas de scrollback restauradas ...    │
│ ○ bug-422        finished││ $ █                                            │
│ ─── detached ───         ││ ── rehydrated @14:21 · seq 3120 ──             │
│ ○ old-run        detached││                                                │
└──────────────────────────┘└────────────────────────────────────────────────┘
```

**Estados:** `loading` (lista) · `empty` ("sin sesiones") · `error` (fetch falla + `Retry`) · `reconnecting` (bus) · `rehydrating` (reattach en curso; spinner en `⟳`) · `resumed` (scrollback restaurado; badge "rehydrated @ts · seq N") · `detached` (sesión viva sin attach). Glifo + label, nunca solo-color (§1.3).

**Hueco backend + rutas propuestas:**
- `GET /hud/sessions` — `{id, mission_id, agent_id, cwd, state, last_seq, created_at}`.
- `GET /hud/sessions/:id` · `GET /hud/sessions/:id/scrollback?from_seq=&limit=` (paginado).
- `POST /hud/sessions/:id/reattach` · `POST /hud/sessions/:id/detach`.
- WS: `session_attached`, `session_detached`, `session_rehydrated` (con `seq`).
- Backend: persistir sesión PTY (id, cwd, mission, created_at, last_seq) + **ring buffer de scrollback**; rehidratar al arranque. RFC 19 cubre *checkpoints de misión*, no la UX de reattach.

**Criterios de aceptación (verificables):**
| # | Criterio | Verificación |
|---|---|---|
| C-23-1.1 | Tras reiniciar, `GET /hud/sessions` devuelve las sesiones previas con el **mismo id** | `cargo test … hud::sessions` (feliz: persiste+relee; fallo: id desconocido → 404) |
| C-23-1.2 | Reattach restaura **≥ N líneas** de scrollback | test: `…/scrollback` devuelve `from_seq`..`last_seq`; UI muestra "rehydrated … seq N" |
| C-23-1.3 | Lista con glifo+label por estado; `rehydrating` visible durante reattach | `pnpm test` (`Sessions.test.ts`: 6 estados §1.4) |
| C-23-1.4 | Bus caído ⇒ datos congelados + `ts` (no se pierde scrollback) | test con `connected:false` |

**Lote:** **F11-1**. **Deps:** F10 (cerrada) + **F0.5** (shell). **Bloquea:** F11-2, F11-4.

### 23.2 F11-2 — Panel `running/waiting/finished` + ping cuando requiere respuesta

**Referentes [Os] (con ficha):**
- **Cate** — `ade-orquestador.md §Cate` (L103): *"cada panel muestra running / waiting / finished y te avisa cuando necesita una respuesta"*.
- **GridBash** — `§GridBash` (L219): *"inspect stable pane activity"*.
- **Herdr** — `docs/design/CONSENSUS_AUDIT.md §1.3 #1 / §2.1` (rollup pane→tab→workspace) [Link].

**Wireframe textual:**
```
┌ pane: agentX ────────────────────────┐
│ ● running          [⚑ ping]          │   ← chip estado + ping
│ $ applying patch src/auth/session.go │
└──────────────────────────────────────┘
  ⚑ = requiere respuesta (waiting) → click lleva al Approvals Dock
```

**Estados:** `running` (`--a-info`) · `waiting` (`--a-warn` + `⚑ ping`) · `finished` (`--a-ok`) · `unknown` (`--a-text-faint`, glifo `?`). Ping: `pending` (⚑) → `acked` (⚑ resuelto/oculto). Regla color+glifo+label.

**Hueco backend + rutas propuestas:**
- `GET /hud/panels` — estado por panel/agente `{run_id, state, needs_response}`.
- `GET /hud/agent/:run_id/attention` · `POST /hud/pings/:id/ack` · `POST /hud/agent/:run_id/ping`.
- WS: `panel_state_changed`, `agent_needs_response`, `ping_acked`.
- Backend: máquina de estados por panel + **cola de atención/ping**. RFC 24 §16 solo cubre push global; H-03 (`AgentStatus::Unknown`) llega en FASE 10 (**B2**).

**Criterios de aceptación:**
| # | Criterio | Verificación |
|---|---|---|
| C-23-2.1 | Panel refleja `running/waiting/finished` | `pnpm test` (`PanelStatus.test.ts`) |
| C-23-2.2 | `agent_needs_response` ⇒ aparece `⚑ ping` en el panel | test WS: emitir evento → assert badge |
| C-23-2.3 | Ack limpia el ping y es **persistente** | `cargo test … hud::panels` (feliz ack; fallo id desconocido → 404) |
| C-23-2.4 | `unknown` usa `--a-text-faint` + glifo, no solo-color | assert token/estilo |

**Lote:** **F11-2**. **Deps:** **F11-1** (host de paneles) + **B2/H-03** (FASE 10).

### 23.3 F11-3 — Manifiesto único de agente (modelos/persona/tools/permisos) editable

**Referentes [Os] (con ficha):**
- **Jazz** — `ade-orquestador.md §Jazz` (L300): *"primary + companion models, persona, tools, permissions en un único JSON"*.
- **Smelt** — `§Smelt` (L310): *permisos granulares* + modos Normal→Plan→Apply→Yolo.
- **OpenCastle** — `§OpenCastle` (L189): config generada **versionada como lockfile** + `sync --check` (drift) en CI.

**Wireframe textual:**
```
┌ Agent Manifest: reviewer ───────────────── [Validar] [Guardar] ┐
│ Modelo    [ claude-opus ▾ ]  Compañero [ haiku ▾ ]             │
│ Persona   [ revisor estricto de seguridad…            ]        │
│ Tools     [x] read  [x] diff  [ ] write  [ ] shell             │
│ Permisos  Normal ( ) Plan ( ) Apply (•) Yolo ( )               │
│ ─ drift: sync --check ✗ (1 tool fuera de allowlist) ─         │
└────────────────────────────────────────────────────────────────┘
```

**Estados:** `loading` · `empty` (sin manifiestos → "crear") · `error` · `valid` · `invalid` (422: tool/modelo no permitido; se marcan los campos) · `drift` (config desviada, como OpenCastle) · `saved`.

**Hueco backend + rutas propuestas:**
- `GET /hud/manifests` · `POST /hud/manifests` · `GET|PUT|DELETE /hud/manifests/:id`.
- `POST /hud/manifests/:id/validate` — valida tools/modelos/permisos.
- WS: `manifest_changed`.
- Backend: tabla `agent_manifests` (model, companion, persona, tools[], permissions) + validación. RFC 06/07 definen skills/MCP, no un manifiesto único por UI.

**Criterios de aceptación:**
| # | Criterio | Verificación |
|---|---|---|
| C-23-3.1 | CRUD round-trip de un manifiesto | `cargo test … hud::manifests` (feliz; fallo: body inválido → 422) |
| C-23-3.2 | Editar y guardar refleja el cambio en la UI | `pnpm test` (`AgentManifestEditor.test.ts`) |
| C-23-3.3 | Tool/modelo fuera de política ⇒ error de campo, no guarda | test `validate` → 422 con `fields` |
| C-23-3.4 | Sin hex crudo | §22.3 G6 = 0 |

**Lote:** **F11-3**. **Deps:** **F0.5** (shell) + **F1** (Agent Card). **Independiente** de PTY.

### 23.4 F11-4 — Grid de PTYs multi-pane por agente

**Referentes [Os] (con ficha):**
- **GridBash** — `ade-orquestador.md §GridBash` (L219): *hasta 100 PTY panes*, *input routing (pane/set/grid)*, `--worktrees` por pane.
- **Cate** — `§Cate` (L103): paneles *"on a canvas or in a dock"* → *"dock into tabs and splits"*, **layout persiste por proyecto**.
- **agent-manager/hcom** — *TUI por pane* [Os en `research/ux-catalog/_funcion-x-referente.md` §23; **sin ficha propia**].

**Wireframe textual:**
```
┌ grid 2x2 ──────────────── [routing: focused ▾] ─┐
│ ┌ pane1 ● running ┐ ┌ pane2 ◐ waiting ┐          │
│ │ $ agentX        │ │ $ agentY        │          │
│ └─────────────────┘ └─────────────────┘          │
│ ┌ pane3 ○ finished┐ ┌ pane4 ? unknown ┐          │
│ └─────────────────┘ └─────────────────┘          │
└──────────────────────────────────────────────────┘
```

**Estados:** `loading` · `empty` (sin panes → spawn) · `error`; por pane: `running/exited/resized`; routing: `focused | set | grid`. `resize` reflow.

**Hueco backend + rutas propuestas:**
- `GET /hud/pty` · `POST /hud/pty` (spawn `{profile, cwd, worktree?}`) · `DELETE /hud/pty/:id` · `POST /hud/pty/:id/resize`.
- Stream: WS `pty_output` (por pane).
- WS: `pty_spawned`, `pty_output`, `pty_exited`, `pty_resized`.
- Backend: **PTY host + layout server-side**. RFC 24 §12 cubre *worktrees*, no la grid física.

**Criterios de aceptación:**
| # | Criterio | Verificación |
|---|---|---|
| C-23-4.1 | Spawn/close de panes y salida en streaming | `cargo test … hud::pty` (feliz spawn+output; fallo: spawn sin perfil → 400/409) |
| C-23-4.2 | Routing `pane/set/grid` envía input solo al destino | test de input routing |
| C-23-4.3 | Layout persiste por misión (como Cate) | `pnpm test` (`PtyGrid.test.ts`) |
| C-23-4.4 | Sin hex crudo | §22.3 G6 = 0 |

**Lote:** **F11-4**. **Deps:** **F11-1** (host de sesiones/PTY). **Peso:** alto (PTY multiplexado).

### 23.5 Orden de lotes F11-x y dependencias

```
FASE 10 (cerrada) ─▶ F0.5 (shell)
                       │
   §22.2 FASE 11 backend (B3/B4/B6/B8) ── primero (diferidos de FASE 10)
                       │
        ┌──────────────┼───────────────┐
        ▼              ▼               ▼
     F11-1 ─────────▶ F11-2          F11-3   (independiente)
     (sesiones)       (paneles+ping)  (manifiesto)
        │
        ▼
     F11-4 (grid PTY)
```

| Lote | Función | Deps | Peso |
|---|---|---|---|
| **F11-1** | Sesiones scrollback/reattach | F10, F0.5 | medio |
| **F11-2** | Panel estado + ping | F11-1, B2 (H-03) | medio |
| **F11-3** | Manifiesto de agente | F0.5, F1 | bajo/medio |
| **F11-4** | Grid de PTYs | F11-1 | alto |

> **Orden recomendado:** §22.2 **B3/B4/B6/B8** → **F11-3** (barato, independiente) → **F11-1** → **F11-2** → **F11-4**. Ninguna bloquea FASE 10 ni la UI v1 (§20 degrada como se describe).

**Relacionado (parcial, no es hueco nuevo):** *ack/aprobación en el canal del humano* y *notificaciones* están **parcialmente** cubiertos por **RFC 24 §16** (`_funcion-x-referente.md`).

**Dependencia:** las 4 exigen **backend nuevo** (lote propio en FASE 11+); se priorizan tras validar FASE 10 (§22.3) y cumplen **G9**.

---

## 24. Cabecera y shell [R — construible]

> **Qué:** el *frame* de la app — el `TopBar` (cabecera) y las regiones del shell que lo rodean — con identidad **Calm Instrumentation**, estados de conexión, y el **leader `:`** con sus acciones. **Fuentes:** layout RFC 66 **§5.2**; identidad RFC 66 **§9.1**; tokens `docs/design/PALETTE.md` **§3.1/§3.2** (medidos **[O]**); hotkeys canónicas RFC 24 **§19** (`?` → `:?`, ver §24.5); estado real del bus **[O `hud.ts`]**. Sin hex crudo (G6 §22.3).

### 24.1 Shell — regiones [O RFC 66 §5.2]

```
┌───────────────────────────────────────────────────────────────────────────┐
│ TOP BAR  Atlas OS v0.1.1 · Mission: <id> · ● connected · : :v :a :n :m :d :?│  ← §24.2–24.5
├──────────────┬──────────────────────────────────────────────┬──────────────┤
│ MISSION RAIL │  VIEW BAR  [Kanban][Canvas][Outline][…]       │ ACTIVITY     │
│ (izq)        │  + lienzo de la proyección activa             │ SPINE (der)  │
│ [+ New]      │                                               │ (ticker)     │
├──────────────┴──────────────────────────────────────────────┴──────────────┤
│ APPROVALS DOCK (inferior, colapsable)                                       │
│ STATUS BAR (modo · conf · VRAM · $ · alertas)                               │
└───────────────────────────────────────────────────────────────────────────┘
```

- **Regiones:** `header[role=banner]` (TopBar) · `nav[data-region=mission-rail]` · `main` (View Bar + lienzo) · `aside[data-region=activity-spine]` · `region[data-region=approvals-dock]` · `footer[role=contentinfo]` (StatusBar).
- **La cabecera NO es el StatusBar.** El `TopBar` lleva **identidad + contexto global** (§24.3); el `StatusBar` lleva **señales** (modo/conf/VRAM/coste/alertas, RFC 66 §5.2). No duplicar (Calmness, RFC 66 §5.10).

### 24.2 Identidad "Calm Instrumentation" en la cabecera [R RFC 66 §9.1]

| Regla | Aplicación en la cabecera |
|---|---|
| Neutro de-carbón, no azul-GitHub | fondo `--a-surface` + hairline inferior `--a-border`; texto `--a-text` |
| **Un solo acento** = teal señal | `--a-primary` **solo** en: marca (spine 3px), leader `:` activo, foco. **Nunca** como color de estado |
| Mono = hechos de máquina | versión, `<id>` de misión, URL del bus → `font-family: var(--a-mono)` |
| Sans = prosa humana | `Atlas OS`, "Mission:", labels de estado |
| Estado = color + glifo + label | ver §24.4 (regla anti-CVD, §1.3) |

### 24.3 Contenido de la cabecera (orden y dato real)

| # | Elemento | Dato / fuente **[O]** | Token |
|---|---|---|---|
| 1 | **Marca** | texto `Atlas OS` + spine teal 3px | `--a-text` + `--a-primary` |
| 2 | **Versión** | `v{import.meta.env.VITE_OC_VERSION ?? version}` — `package.json` = **0.1.1** | `--a-text-muted` (mono) |
| 3 | **Misión activa** | `$activeMissionId` (`views.ts`); dot de rollup (§4) | `--a-text` (id, mono) + color de rollup |
| 4 | **Estado del bus** | `$hud.connected` + `$hud.url` (`hud.ts:26`) | ver §24.4 |
| 5 | **Remote** (condicional) | `remote.local_only` → `local-only` \| `remote` (+ OIDC/bearer) — ya existe **[O `+page.svelte:392-404`]** | `--a-text-muted` |
| 6 | **Leader hint `:`** | leyenda estática de acciones | `kbd`: `--a-text-faint`; `:` activo `--a-primary` |

> **Real hoy [O]:** `+page.svelte` ya renderiza (1) `Atlas OS`, (2) `v{VITE_OC_VERSION ?? '0.1.0'}` y (5) remote; **faltan** (3) misión activa en cabecera, (4) bus en cabecera (hoy está abajo en `.mission-deck`) y (6) leader hint. v1 = subir (3)/(4) al `TopBar` + añadir (6).

### 24.4 Estados de conexión (bus WS) — color + glifo + label

| Estado | Condición | Token | Glifo | Label | Comportamiento |
|---|---|---|---|---|---|
| **connected** | `$hud.connected === true` | `--a-ok` | `●` (lleno) | `connected` | datos vivos; sin banner |
| **reconnecting** [R] | WS cerró con backoff activo (`hud.ts` 1 s→10 s) | `--a-warn` | `◐` | `reconnecting…` | banner sutil; datos **congelados** + `ts` |
| **disconnected** | `$hud.connected === false`, sin timer | `--a-err` | `○` (hueco) | `disconnected` | banner **"sin conexión — reconectando"** + timestamp del último frame (§1.4) |
| **unknown** | sin primer frame / no clasificable | `--a-text-faint` | `?` | `unknown` | nunca inventa estado (§1.4) |

> **[O] real:** el store expone `{ connected, url, events }`; `reconnecting` es **derivado** (timer pendiente) → si no se puede derivar, se añade al store en F1 [R]. El label actual (`connected`/`disconnected`) se **conserva**; solo se le suma glifo + token.

### 24.5 Leader `:` — acciones y comportamiento exacto

**Modelo:** `:` = leader. Pulsarlo **abre la Command Palette** (`CommandPalette.svelte`, ya existe **[O]**); la **segunda tecla** ejecuta la acción. El hint de la cabecera (6) es **decorativo** (`aria-hidden="true"`); la ayuda real es `:?`.

| Acción | Efecto | Estado v1 |
|---|---|---|
| `:v` | abre **View Switcher** (cambiar view: Kanban/Canvas/Outline/Timeline/Cost/Health/Audit/Worktrees/Settings/MCP) | cablear (F5) |
| `:a` | **foco al Approvals Dock** (`[data-region=approvals-dock]`) | cablear (F5) |
| `:n` | **nueva misión** (abre `NewMissionButton`) | cablear (F5) |
| `:m` | **foco a Mission Rail** — ver abajo | cablear (F5) |
| `:d` | **Show Demo** (`DemoPane`) | cablear (F5) |
| `:?` | **panel de ayuda** — ver abajo | cablear (F5) |

> **Reconciliación RFC 24 §19:** la tabla canónica usa `?` sin leader; bajo el modelo leader se **normaliza a `:?`** (y `?` suelto queda como **alias**). Los hotkeys de view de `views.ts` se retiran (§1.5/§1.11).

**`:m` — foco a Mission Rail (exacto):**
1. **No** abre la palette; **no** cambia de view.
2. Mueve el **foco DOM** al contenedor `[data-region=mission-rail]` (roving `tabindex`): foco a la misión **activa**; si no hay activa → primera; si el rail está vacío → foco a `[+ New]`.
3. **Scroll-into-view** del item enfocado.
4. Anuncia por `aria-live="polite"`: `"Mission Rail. <n> misiones. Foco en <id>."`
5. `:focus-visible` visible en el item (anillo §1.5).

**`:?` — panel de ayuda (exacto):**
1. Abre un **overlay** `role="dialog" aria-modal="true"` con la **tabla de hotkeys** (RFC 24 §19 completa, `:?` incluido).
2. **Foco atrapado** dentro del panel; foco inicial en el botón **Cerrar** (`aria-label="Cerrar (Esc)"`).
3. **`Esc` cierra** y **devuelve el foco** al elemento que invocó el panel.
4. Clic en el *scrim* cierra; `Tab` cicla solo dentro.
5. `prefers-reduced-motion`: sin animación de entrada.

### 24.6 Markup esperado (Svelte 5, runes) — sin hex

```svelte
<!-- AppHeader.svelte — RFC 67 §24 -->
<header class="app-header" role="banner">
  <span class="brand">
    <span class="brand-mark" aria-hidden="true"></span>
    <h1 class="brand-name">Atlas OS</h1>
    <span class="brand-version">v{version}</span>
  </span>

  <span class="active-mission" data-state={mission ? 'set' : 'none'}>
    <span class="rollup-dot" data-state={rollup} aria-hidden="true"></span>
    <span class="mission-label">Mission:</span>
    <code class="mission-id">{mission?.id ?? '—'}</code>
  </span>

  <span class="bus" data-state={busState} role="status" aria-live="polite">
    <span class="glyph" aria-hidden="true">{busGlyph}</span>
    <span class="label">{busLabel}</span>
  </span>

  {#if remote}
    <span class="remote" data-state={remote.local_only ? 'local' : 'remote'}>
      {remote.local_only ? 'local-only' : 'remote'}
    </span>
  {/if}

  <span class="leader-hint" aria-hidden="true">
    <kbd class="leader">:</kbd>
    <kbd>:v</kbd><kbd>:a</kbd><kbd>:n</kbd><kbd>:m</kbd><kbd>:d</kbd><kbd>:?</kbd>
  </span>
</header>

<!-- HelpOverlay.svelte -->
{#if open}
  <div class="help-scrim" onclick={close}></div>
  <div class="help-panel" role="dialog" aria-modal="true" aria-labelledby="help-title"
       tabindex="-1" bind:this={panelEl} onkeydown={trapFocus}>
    <header><h2 id="help-title">Atajos</h2>
      <button class="close" aria-label="Cerrar (Esc)" onclick={close}>✕</button></header>
    <table><!-- filas RFC 24 §19: :m :v :a :n :f :s :d :r :p :x :c :e :o :t :? --></table>
  </div>
{/if}
```

### 24.7 Tokens usados (dark `§3.1`; light `§3.2` — vía `var()`, nunca hex)

| Elemento | Token |
|---|---|
| fondo cabecera / hairline | `--a-surface` / `--a-border` |
| marca / `Atlas OS` | `--a-text` + `--a-primary` (spine 3px) |
| versión / `Mission:` / labels | `--a-text-muted` (versión, mono) |
| id de misión / URL bus | `--a-text` + `--a-mono` |
| bus `connected`/`reconnecting`/`disconnected`/`unknown` | `--a-ok` / `--a-warn` / `--a-err` / `--a-text-faint` |
| `kbd` leader | `--a-text-faint`; `:` activo `--a-primary` |
| anillo foco | `--a-focus` 2px + offset 2px (§1.5) |

### 24.8 Criterios de aceptación (verificables)

| # | Criterio | Verificación |
|---|---|---|
| C1 | Cabecera renderiza marca, versión, misión activa, bus y leader hint | `pnpm test` (`AppHeader.test.ts`: los 6 campos presentes) |
| C2 | Versión = `v` + `package.json.version` (**0.1.1**) | test lee `package.json` y compara |
| C3 | `$hud.connected=false` ⇒ `[data-state=disconnected]` **y** banner "sin conexión — reconectando" con `ts` | test con store `connected:false` |
| C4 | `:m` enfoca el Mission Rail (no palette, no cambia view) | test teclado: tras `:m`, `document.activeElement` ⊂ `[data-region=mission-rail]`; `activeView` sin cambios |
| C5 | `:?` abre `role=dialog[aria-modal=true]` con la tabla RFC 24 §19; `Esc` cierra y **restaura foco** | test: abre → assert dialog; `Esc` → assert cerrado + foco previo |
| C6 | `:v/:a/:n/:d` disparan su acción | test por acción |
| C7 | `:focus-visible` = anillo `--a-focus` 2px/offset 2px | assert estilo computado |
| C8 | **Sin hex crudo** | `rg -nP "(?<![{])#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})\b" src -g "*.svelte"` → **0** (G6 §22.3) |
| C9 | `prefers-reduced-motion` ⇒ sin pulse en el dot del bus | media query + captura |
| C10 | A11y: `role=banner`; bus `role=status aria-live=polite`; leader-hint `aria-hidden=true` | assert atributos |

**Lote/deps:** **F0.5 — Cabecera y shell** (nuevo; tras **F0** tokens). El *comportamiento* de `:m/:?` se cablea en **F5** (leader map); el foco de `:m` depende de **F2** (Mission Rail). v1.

### 24.9 Añadido a §22.1 (lote) y changelog
- **§22.1:** añadir fila **F0.5 — Cabecera y shell** (deps F0; habilita §24; `:m/:?` → F5).
- **§22.3 G7:** las capturas dark/light deben incluir **la cabecera** (estados connected/disconnected).

---

## 25. FASE 12 — Frontend de B3/B4/B6/B8 + cierre de G7 [R — construible]

> **Qué:** la UI que consume los lotes backend **FASE 11** (**B3/B4/B6/B8**, §22.2) y cierra **G7** (§22.3). **Nota explícita §20:** §25 **no cambia ningún veredicto de §20**; materializa lo que §20 dejó como **v2** (H-02 batch · H-01 frecency · H-05 task-comments · H-08 snapshot) — los builders pueden seguir §20 tal cual. **Única precisión:** F11-d cubre además la **creación de misión** (H-09 / **B7**, backend **FASE 10**), que pasa de CLI/IPC (v1 §20) a **REST**.

**Mapa de lotes:**

| Lote | Función | Backend | Hueco | Dueño | §20 → FASE 12 |
|---|---|---|---|---|---|
| **F11-a** | Aprobaciones en lote (Dock) | **B3** | H-02 | **Builder-3** | batch = v2 → batch |
| **F11-b** | Anotaciones de tarea | **B8** | H-05 | **Builder-2** | task-comments = v2 → task |
| **F11-c** | Snapshot del sandbox | **B6** | H-08 | **Builder-2** | snapshot = v2 → snapshot |
| **F11-d** | Frecuencia + creación de misión | **B4** (+**B7**) | H-01 (+H-09) | **Builder** | frecency = v2 → frecency; `+New` CLI/IPC → REST |

### 25.1 F11-a — Aprobaciones en lote en el Dock [Builder-3] ← B3 (H-02)

**Wireframe textual:**
```
┌ Approvals Dock ───────────────── [x] sel.  [Apr selec (2)] [Deny selec (2)] ┐
│ [x] ● AgentX  create src/auth/session.go   [APR][DENY][STEER]               │
│ [x] ◐ AgentY  run tests (scope: module)    [APR][DENY]                      │
│ [ ] ○ AgentZ  delete tmp/cache             [APR][DENY]                      │
│ ── Motivo (opcional, se guarda en audit) [____________________] ──          │
└────────────────────────────────────────────────────────────────────────────┘
```

**Estados:** `loading` (cargando cola) · `vacío` (sin pendientes) · `error` (procesar falla → `Retry`) · `parcial` (parte aprobada, parte en conflicto) · **`conflicto 409`** (una aprobación ya resuelta por otro canal). Glifo + label, nunca solo-color.

**UX copy (ES / EN):**

| Contexto | ES | EN |
|---|---|---|
| Selección | `2 seleccionadas` | `2 selected` |
| Aprobar lote | `Aprobar seleccionadas (2)` | `Approve selected (2)` |
| Denegar lote | `Denegar seleccionadas (2)` | `Deny selected (2)` |
| Motivo | `Motivo (opcional) — se guarda en el audit` | `Reason (optional) — saved to audit` |
| Loading | `Procesando 2…` | `Processing 2…` |
| Parcial | `1 de 2 aprobadas · 1 en conflicto` | `1 of 2 approved · 1 conflicted` |
| Conflicto 409 | `Conflicto: ya fue resuelta por otra sesión` | `Conflict: already resolved by another session` |
| Vacío | `Sin aprobaciones pendientes` | `No pending approvals` |
| Error | `No se pudieron procesar: reintentar` | `Couldn't process: retry` |

**Hotkeys (dentro del Dock):** `Space` marca/desmarca fila · `Shift+A` selecciona todo · `a` aprueba selección · `r` deniega selección · `Esc` limpia selección. (Global `:a` = foco al Dock, RFC 24 §19 — sin colisión.)

**Criterios de aceptación:**
| # | Criterio | Verificación |
|---|---|---|
| C-25-a.1 | Selección múltiple + `Apr selec` llama `POST /hud/approvals/batch` | `pnpm test` (`ApprovalsDock.test.ts`) |
| C-25-a.2 | `reason` se envía y **persiste** | test: payload incluye `reason`; assert audit |
| C-25-a.3 | **409** → estado `conflicto` con copy ES/EN; el resto no se pierde | test con respuesta 409 |
| C-25-a.4 | Parcial muestra `n de m` | test mixto |
| C-25-a.5 | Sin hex crudo | §22.3 G6 = 0 |

**Deps:** **B3** (backend) + **F4** (Dock) + F0/F5 (tokens/hotkeys).

### 25.2 F11-b — Anotaciones de tarea [Builder-2] ← B8 (H-05)

**Wireframe textual:**
```
┌ Outline ▸ Task: "add refresh token" ─────────────────────────┐
│ 💬 2 comentarios                                              │
│  • [rev] falta rotación…                       14:20          │
│  • [me]  añade test de expiración              14:22          │
│ ── [ Comentar…                                        ] ──    │
└───────────────────────────────────────────────────────────────┘
```

**Estados:** `loading` · `vacío` (`Sin comentarios en esta tarea`) · `error` (guardar falla) · `parcial` (antes solo a nivel diff; ahora task) · **`conflicto 409`** (el comentario cambió en el servidor).

**UX copy (ES / EN):**

| Contexto | ES | EN |
|---|---|---|
| Vacío | `Sin comentarios en esta tarea` | `No comments on this task` |
| Acción | `Comentar` | `Comment` |
| Placeholder | `Escribe un comentario para el agente…` | `Write a comment for the agent…` |
| Loading | `Guardando…` | `Saving…` |
| Conflicto 409 | `El comentario cambió; recarga para ver la versión actual` | `Comment changed; reload to see the current version` |
| Error | `No se pudo guardar el comentario` | `Couldn't save the comment` |

**Hotkeys:** `c` (task seleccionada en Outline) abre el compositor — global `:c` = "Comment on selected" (RFC 24 §19). `Cmd/Ctrl+Enter` envía.

**Criterios:**
| # | Criterio | Verificación |
|---|---|---|
| C-25-b.1 | Crear/listar comentario de **task** (no solo diff) | `pnpm test` (`TaskComments.test.ts`) |
| C-25-b.2 | Round-trip vía `task_annotations` | `cargo test … hud::annotate` (feliz; fallo: body vacío → 422) |
| C-25-b.3 | 409 muestra copy y **no pierde el texto** | test con 409 |
| C-25-b.4 | Sin hex crudo | §22.3 G6 = 0 |

**Deps:** **B8** + F6–F9 (Outline) + F0/F5.

### 25.3 F11-c — Snapshot del sandbox [Builder-2] ← B6 (H-08)

**Wireframe textual:**
```
┌ Agent Card ▸ 👁 Reason ──────────────────────────────┐
│ [steps] [snapshot]   ← tabs dentro de 👁 Reason       │
│ ┌ snapshot @run r-12 (last 40 lines) ─────────────┐  │
│ │ $ cargo test --quiet                             │  │
│ │ ... 3 passed ...                                 │  │
│ └──────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────┘
```

**Estados:** `loading` (`Capturando snapshot…`) · `vacío` (`Sin snapshot disponible`) · **`error 404`** (`Sin sandbox para este run`) · `parcial` (últimas N líneas). *(Sin 409 — es lectura.)*

**UX copy (ES / EN):**

| Contexto | ES | EN |
|---|---|---|
| Loading | `Capturando snapshot…` | `Capturing snapshot…` |
| Vacío | `Sin snapshot disponible` | `No snapshot available` |
| 404 | `Sin sandbox para este run` | `No sandbox for this run` |
| Parcial | `Snapshot parcial (últimas N líneas)` | `Partial snapshot (last N lines)` |
| Error | `No se pudo obtener el snapshot: reintentar` | `Couldn't fetch the snapshot: retry` |

**Hotkeys:** `👁 Reason` ya existe (Agent Card); tab `snapshot` con `←/→`. Acción global propuesta **`:k`** (**nueva** en RFC 24 §19 — requiere aprobación del PL; si no, solo botón).

**Criterios:**
| # | Criterio | Verificación |
|---|---|---|
| C-25-c.1 | Tab `snapshot` muestra el frame del sandbox | `pnpm test` (`AgentCard.snapshot.test.ts`) |
| C-25-c.2 | 404 → copy "Sin sandbox…", no rompe la card | test con 404 |
| C-25-c.3 | Fallback a `agent_steps.observation` si no hay snapshot | test |
| C-25-c.4 | Sin hex crudo | §22.3 G6 = 0 |

**Deps:** **B6** + **F1** (Agent Card) + F0.

### 25.4 F11-d — Frecuencia + creación de misión [Builder] ← B4 (H-01) + B7 (H-09)

**Wireframe textual:**
```
┌ Mission Rail ─────────────── [orden: Frecuentes ▾] ┐
│ ● auth-refactor   3× hoy                            │
│ ◐ dashboard-fix   1× hoy                            │
│ [+ New Mission]                                     │
└─────────────────────────────────────────────────────┘
┌ New Mission ──────────────────────────── [Crear] ──┐
│ Prompt [ fix the flaky auth test…                ]  │
│ (vacío ⇒ error inline)                              │
└─────────────────────────────────────────────────────┘
```

**Estados:** `loading` · `vacío` (rail sin misiones → CTA) · `error` (crear falla) · `parcial` (frecency sin datos → orden estable) · **`conflicto 400`** (prompt vacío → validación inline). *(409 no aplica; el 400 de prompt vacío es el fallo de B7.)*

**UX copy (ES / EN):**

| Contexto | ES | EN |
|---|---|---|
| Orden | `Frecuentes` / `Recientes` | `Frequent` / `Recent` |
| Loading (crear) | `Creando misión…` | `Creating mission…` |
| Prompt vacío (400) | `El prompt no puede estar vacío` | `Prompt can't be empty` |
| Éxito | `Misión creada` | `Mission created` |
| Error | `No se pudo crear la misión: reintentar` | `Couldn't create the mission: retry` |
| Parcial | `Frecuencia no disponible: orden por actividad` | `Frequency unavailable: ordered by activity` |

**Hotkeys:** `:n` nueva misión (RFC 24 §19) · `:m` foco al rail (§24.5). Toggle de orden **sin hotkey** (control en el Rail).

**Criterios:**
| # | Criterio | Verificación |
|---|---|---|
| C-25-d.1 | Toggle Frecuentes/Recientes llama `?sort=frecency` | `pnpm test` (`MissionRail.test.ts`) |
| C-25-d.2 | `+ New` crea misión vía `POST /hud/missions` (**REST**) | test: POST con prompt → 201 |
| C-25-d.3 | Prompt vacío → 400 → copy inline; no crea | test con 400 |
| C-25-d.4 | Frecency sin datos → orden estable + copy parcial | test |
| C-25-d.5 | Sin hex crudo | §22.3 G6 = 0 |

**Deps:** **B4** + **B7** + **F2** (Mission Rail) + F0/F5.

### 25.5 Cierre de G7 — capturas que faltan (estado a reproducir)

> G7 (§22.3) está **parcial**. Faltan exactamente **3** capturas:

| # | Captura | Tema | Estado a reproducir |
|---|---|---|---|
| **G7-a** | **Approvals Dock con aprobación real** | dark | cola con ≥1 `approval_request` real por WS (`pending`), 1 fila seleccionada, botones `Apr selec` activos; `:focus-visible` visible en la fila |
| **G7-b** | **`:?` en vivo** | dark | `HelpOverlay` abierto (tabla RFC 24 §19 completa), foco inicial en **Cerrar**, `aria-modal=true` |
| **G7-c** | **Tema claro con datos** | light | ≥1 misión en el Rail + ≥1 evento en la Activity Spine + 1 view con filas (Kanban), tokens §3.2 |

**Ya cubierto (G7 parcial):** Cabecera connected/disconnected (dark), Mission Rail, Activity Spine. **Pendiente declarado** en §22.3.

### 25.6 Orden y dependencias (FASE 12)

```
B3 ─▶ F11-a (Builder-3)      B8 ─▶ F11-b (Builder-2)
B6 ─▶ F11-c (Builder-2)      B4+B7 ─▶ F11-d (Builder)
            └──────────────► G7-a / G7-b / G7-c (cierre)
```

| Lote | Deps backend | Deps frontend | Dueño |
|---|---|---|---|
| F11-a | B3 | F4, F0/F5 | **Builder-3** |
| F11-b | B8 | F6–F9, F0/F5 | **Builder-2** |
| F11-c | B6 | F1, F0 | **Builder-2** |
| F11-d | B4, B7 | F2, F0/F5 | **Builder** |

> **Nota §20 (explícita):** §25 **no contradice §20** — convierte a FASE 12 lo que §20 dejó como **v2** (H-01/H-02/H-05/H-08). Si un builder ya construyó el placeholder de §20, **no hay que rehacer**: §25 solo activa el camino REST cuando el backend lande.



