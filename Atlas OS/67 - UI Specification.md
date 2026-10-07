# 67 - UI Specification (construible)

**Author:** UI/UX Design agent · **Date:** 2026-10-06 · **Status:** Draft **v3** — backend **cotejado** (`backend-capabilities.md`; §1.7 y §19 **CERRADAS**); views rellenas con `frontend-current-state`; `rfc-vs-code` aplicado; contrato de transición de views (§1.11); **§20 veredictos H-01…H-09**; **§22 plan FASE 10 + gates §22.3**. Fichas del Researcher **en curso** (9 con ficha; las 62 prioridad-A se cuentan cuando el Researcher recalcule `_pending-A.md`). **Sin bloqueos de backend.**
**Depends on:** RFC 17 (UI base) · RFC 24 (HUD spec) · RFC 65 (HUD v2 implementación) · RFC 66 (UX Architecture & Design System) · `docs/design/PALETTE.md` (token system) · `docs/design/CONSENSUS_AUDIT.md` · `docs/coordination/PLAN-docs-hardening.md`.
**Inputs consumidos (estado 2026-10-06):**
- Researcher `Atlas OS/research/64 - UX reference catalog.md` → **ENTREGADO [O]**: recuento **exacto = 441** (RFC 62 decía "~425"; **no coincide con ningún "320+"**); `tiene_UI` si=376/no=38/parcial=27; **94 en categorías HUD-relevantes** (IDE 29 · terminal 21 · ADE 20 · observabilidad 18 · canvas 5 · kanban/PM 1). Fichas `ux-catalog/*.md` → **6 ENTREGADAS [O]** (`ade-orquestador`, `chat-workspace`, `ide`, `observabilidad`, `terminal`, `_pending-A`); **62 prioridad-A pendientes [P]** (`_pending-A.md §3`).
- Builder `docs/audit/backend-capabilities.md` → **ENTREGADO [O]** (275 líneas): **41 rutas** (39 montadas + 2 feature-gated), **26 eventos** WS, **34 subcomandos CLI**, **61 tablas**, y **9/9 huecos H-01…H-09 contrastados con `fichero:línea`** ⇒ §1.7/§19/§20 **cerrados** (era `[P]`).
- Fichas `ux-catalog/` → **ampliadas [O]**: además de las 5 de categoría, **`outline.md`, `settings.md`, `kanban-pm.md`, `canvas.md`** (citadas en §1.12); **62 prioridad-A siguen [P]** (`_pending-A.md`).
- Builder-2 `docs/audit/frontend-current-state.md` → **ENTREGADO [O]** (276 líneas): `+page.svelte` **1237 líneas**; **24 componentes**; **531 literales de color / 30 hex únicos / `src/app.css` ausente / sin selector dark-light**; **13 views** (`views.ts`); hotkeys **3 operativos / 7 en conflicto / 8 ausentes**; tabla de estados carga/vacío/error por view.
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
- **Decisión de diseño:** rail fijo izquierdo + rollup del estado hijo más severo. — Cita Researcher: `research/ux-catalog/herdr.md` (rollup pane→tab→workspace), `orca.md` (worktree por agente) **[cita ficha Researcher → §1.12]**.
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
- **Decisión de diseño:** columna derecha (no panel inferior) + color por clase de evento + **sync con Canvas** (OA parcial) — Cita Researcher: `n8n.md` (log anclado + sync selección), `herdr.md` (ticker de estado) **[cita ficha Researcher → §1.12]**.
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
- **Decisión de diseño:** disclosure en capas **0–3** (0 = colapso al completar, "Worked for Nm"); spine de estado; barra de acciones gated por estado. — Cita Researcher: `zed.md` (colapso de turno al completar), `orca.md` (estado de worker de 1ª clase), `herdr.md` (estados) **[cita ficha Researcher → §1.12]**.
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
- **Decisión de diseño (OA-66-07=(b)):** **dos canales** — *gate* bloqueante vs *pregunta* async [R]. — Cita Researcher: `orca.md` (decision gate vs `ask`) **[cita ficha Researcher → §1.12]**.
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
- **Decisión de diseño:** prefijo `:` + fuzzy; **extiende** a acciones de agente; convención declarada como **TUI-style** (no IDE-style) [R]. — Cita Researcher: `vscode.md` (palette con modos), `herdr.md` (prefix/navigate) **[cita ficha Researcher → §1.12]**.
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
- **Decisión:** coste por agente/misión + VRAM/RAM/throughput + budget; alertas 80%/100%. — Cita Researcher: `hermes.md` (cost/token en status bar), `grafana.md` (dashboards + alerting ligado a panel) **[cita ficha Researcher → §1.12]**.
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
- **Decisión:** KPIs en una banda + checker status del Validation Engine. — Cita Researcher: `hermes.md` (health WS), `grafana.md` (paneles de estado) **[cita ficha Researcher → §1.12]**.
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
- **Decisión:** árbol por misión con worktrees como nodos; dirty → color. — Cita Researcher: `orca.md` (worktree por agente), `vibekanban.md` (1 worktree/agente) **[cita ficha Researcher → §1.12]**.
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
- **Decisión:** editor de allowlist **en sitio**; activación de skill por drop/click; MCP read + probe. — Cita Researcher: `herdr.md`/`hermes.md` (skills/MCP en caliente) **[cita ficha Researcher → §1.12]**.
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
| Researcher | `Atlas OS/research/ux-catalog/*.md` | **[O parcial]** | ✅ §1.12 ahora cita **9 fichas** (5 categoría + `outline`/`settings`/`kanban-pm`/`canvas`); **[P] = 62 prioridad-A** |

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

**B. Bloqueados por fichas del Researcher:** **ninguno crítico** — Outline y Settings **ya tienen ficha** (§1.12) ⇒ cerrado. Quedan **62 prioridad-A [P]** (`_pending-A.md`) que no bloquean componentes construibles.

**C. Cerrados/corregidos:** MCP hot-swap→v2 (#2); Worktrees grafo→v2 (#3); "8 views"→**13** (§1.11); "dark/light ya existe"→**falso** (§1.2); hotkeys de view→colisión, se retiran (§1.5); `KernelCommand`→dead code (§1.7).

---

## 21. Changelog

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
| **F0 — Tokens** | crear `src/app.css` con `--a-*` (PALETTE §3); importar; sustituir los 531 literales | `src/app.css` (nuevo), `src/routes/+layout.svelte`, 24 `.svelte` | `pnpm check && pnpm lint`; `rg "#[0-9a-fA-F]{3,8}" src -g "*.svelte"` → **0**; captura dark/light | — | medio (53 1 sitios): mitigar con `rg` antes/después |
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
| **G6** | `rg "#[0-9a-fA-F]{3,8}" src -g "*.svelte"` | **0** (tokens `--a-*` aplicados; sin hex crudo) |
| **G7** | Capturas **dark/light** de **Mission Rail**, **Activity Spine**, **Approvals Dock** y **1 view** (p.ej. Kanban) | 4 componentes en ambos temas; anillo `:focus-visible` visible |
| **G8** | `rg "KernelCommand" src/lib` | **0** (la UI no se cablea al dead code) |

**FASE 10 se considera cerrada cuando G1–G8 pasan.** FASE 11 (B3/B4/B6/B8) **no** se inicia hasta entonces.



