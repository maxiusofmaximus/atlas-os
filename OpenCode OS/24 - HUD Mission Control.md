# 24 - HUD Mission Control

> Panel de mando integrado de OpenCode OS. No es un dashboard de métricas pasivas: es el **centro de control de misión** donde un humano coordina un swarm de agentes y aprueba/steer las decisiones en caliente. Presenta al usuario el trabajo de los subagentes como Jira presenta tickets, como Cursor presenta demos, como Notion presenta documentos, como n8n presenta un canvas de ejecución, y como Hermes HUD presenta health en vivo — todo en un único espacio de trabajo web y desktop multiplataforma.

---

## 0. Por qué este RFC existe y qué lo diferencia

Existen dos polos opuestos en el mercado:

- **Hub de control público tipo Hermes** (Nous Research) — web dashboard con jerarquía de subagentes, health status WebSocket, auditoría de costos y gestión de skills/MCP en caliente, profile switcher (https://docs.nousresearch.ai/hermes/overview/profiles/, https://docs.nousresearch.ai/hermes/overview/bots/). El humano ve todo en vivo.
- **Admin personal sin observabilidad** tipo AionUI desktop — configuras agentes pero no hay un Mission Control que te permita *steer*los visualmente. Te quedas mirando el chat.

Ninguno de los dos ofrece **simultáneamente**:

1. Jerarquía recursiva Mission → Objective → Task → Subtask (Linear/Asana, sin límite de profundidad).
2. Demos over diffs (Cursor): video, screenshot, URL de preview, logs — no solo texto.
3. Comentarios block-level sobre cada decisión (Notion).
4. Canvas de nodos con execution log por nodo (n8n), mapeado al reasoning chain-of-thought de cada subagente.
5. Health status, cost/token audit, y skills/MCP drag-and-drop en caliente (Hermes).
6. Approvals queue multi-dispositivo web (Hermes) + campo móvil (Loom) para revisión async.
7. Audit timeline append-only con hash chaining (estilo block explorer) de cada acción sensible.
8. Worktrees visuales con mini-git-graph por Mission.
9. Execution Mode selector 🕊 / 🤝 / 🛫 / 🚀 (`21`) y **Modo de uso** `ask` / `architect` / `code` / `context` (`23`) — ambos visibles como badges.
10. Mission consolidated panel que muestra el `PublicUnderstandingVerdict` (`23`) antes de desbloquear el Planning Engine.

La integración de las 10 es propia de OpenCode OS. Hermes cumple 5, 7 (parcial), 9 (parcial); el resto no.

---

## 1. Layout maestro del panel HUD

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Top Bar     [ Logo ]  Mission selector ▾   Profile/worktree switcher       │
│             🔔 approvals (n)   ⚙ settings   🌙 dark   👤 user                │
├──────────┬─────────────────────────────────────────────────────┬───────────┤
│ Sidebar  │  Main pane: view selector                             │ Activity │
│          │  ┌───────────────────────────────────────────────┐  │  streamer │
│ Missions │  │                                                 │  │ (live    │
│  ▸ Auth  │  │     [Kanban] [Canvas] [Outline] [Timeline]      │  │  ticker) │
│  ▸ Dashb. │  │     [Cost & Res] [Health KPIs] [Audit]         │  │          │
│  ▸ Bug 422│  │                                                 │  │          │
│          │  │     ┌──────────┐ ┌──────────┐ ┌──────────┐      │  │          │
│ Filters  │  │     │ AgentX   │ │ AgentY   │ │ AgentZ   │      │  │          │
│  Status  │  │     │ Building │ ● Idle    │ ⚠ doom   │      │  │          │
│  Model   │  │     │ +262/-26 │  4m idle  │ ⛔ halt  │      │  │          │
│  Cost    │  │     │ [▶][⏸][⏹]│ [▶][⏸]   │ [🚑]    │      │  │          │
│  Skill   │  │     └──────────┘ └──────────┘ └──────────┘      │  │          │
│          │  │                                                 │  │          │
│ + New M. │  │     Reasons timeline (chain-of-thought ticker)   │  │          │
│          │  └───────────────────────────────────────────────┘  │          │
├──────────┴─────────────────────────────────────────────────────┴───────────┤
│ Approvals queue docked bottom: [APR] [DENY] [STEER] [FORK]  ⚡mode selector │
└────────────────────────────────────────────────────────────────────────────┘
```

Componentes:

- **Top Bar**: logo, mission selector dropdown (cambia worktree activo), profile switcher (estilo Hermes `?profile=`), alerta de approvals (badge con contador), settings, dark mode, user.
- **Sidebar**: árbol de Misions recursivo (Objective → Task → Subtask). Cada nodo es expandible y arrastra al canvas. Filtros por status, model, cost, skill. Botón `+ New Mission` que invoca el panel de Prompt Understanding (`23`) directamente.
- **Main pane**: 8 views intercambiables (ver §2).
- **Activity streamer** (sidebar derecho): ticker en vivo estilo Twitter feed de eventos del Kernel Bus (`02 - Agent Operating System.md` §3.1) formateados para humanos.
- **Approvals queue docked**: debajo del main pane, expone `APR / DENY / STEER / FORK` para el ítem seleccionado + execution mode selector (badge 🕊/🤝/🛫/🚀).

---

## 2. Multi-view selector

Una sola fuente de datos — el **Execution Journal** — proyectada en 8 vistas. Cada vista responde a una pregunta distinta del humano:

| View | Pregunta que responde | Referente |
|---|---|---|
| **Kanban** | ¿Qué está bloqueado, hecho, revisando? | Jira, Linear |
| **Canvas** | ¿Cómo están cableados los subagentes? | n8n, Windmill |
| **Outline** | ¿Cómo se descompuso la mission? | Notion |
| **Timeline** | ¿Cuándo pasó cada cosa? | Linear |
| **Cost & Res** | ¿Cuánto cuesta y qué recursos gasta? | Hermes |
| **Health KPIs** | ¿Están saludables los subagentes? | Hermes |
| **Audit** | ¿Qué decisiones sensibles se tomaron y por quién? | SOC2 / block explorer |
| **Worktrees** | ¿En qué branch está cada subagente? | GitKraken / `git graph` |

Cambiar de vista **no** detiene la ejecución. Es pura proyección.

---

## 3. Tarjeta por subagente — anatomía

Patrón mezcla del ticket de Jira + card de Linear + mission card de Cursor + block de Notion.

```
┌─────────────────────────────────────────────────────────┐
│ ●  AgentX · researcher · Gemini-1.5-Pro          12m  ⋮ │
│ 📌 Mission: Auth refactor · Obj 2 · Subtask 3            │
├─────────────────────────────────────────────────────────┤
│ Status:  reading docs ▶                                 │
│ Confidence: 0.87  Judgment: HIGH                         │
│ Files touched: 4 (auth.go, session.go, +2)              │
│ Diff:         +262 / -26                                 │
│ Skill:        ★ ★ ★ ★ ★ React UI Expert  [active]       │
│ Tokens in:    58,123     out: 9,212      cost: $0.42    │
│ Tool calls:   8 (web 3, fs 5)  · times called: 12       │
│ Execution mode: 🤝 HUMAN_IN_LOOP                        │
│ Modo uso:         🏛 architect                          │
├─────────────────────────────────────────────────────────┤
│ [▶ Run]  [⏸ Pause]  [⏹ Stop]  [🔱 Fork]  [🚑 Recover]  │
│ [💬 Steer]  [👁 Show reason]  [🎬 Demo]                  │
└─────────────────────────────────────────────────────────┘
```

### 3.1 Campos (≥15, como pidió el usuario)

1. `agent_id`, `role`, `model_id`.
2. `mission_id` (referencia al árbol de la sidebar).
3. `status` (one of: `queued`, `reading`, `planning`, `coding`, `reviewing`, `idle`, `paused`, `doom_loop`, `error`, `success`).
4. `confidence` + `judgment` (HIGH/MEDIUM/LOW/BLOCK — viene de `23`).
5. `files_touched[]` con diff total.
6. `plan_text` (link al Plan guardado — `12`).
7. `evidence_links[]` (link al Research Run Report — `10`).
8. `tokens_in`, `tokens_out`, `cost_so_far`.
9. `tool_calls` (per tool, hits).
10. `skill_id[]` activo.
11. `execution_mode` (badge 🕊/🤝/🛫/🚀).
12. `modo_uso` (ask/architect/code/context — badge).
13. `elapsed_ms` y `eta_ms`.
14. `last_heartbeat`/iso_ts.
15. `worktree_path` (path físico) + `branch` + `dirty?` bool.
16. `doom_loop_count` y `goal_drift` indicator.
17. `checkpoint_id` actual (link a `19`).
18. `[Demo]` preview URL, screenshot o Loom-like video (ver §9).

### 3.2 Acciones por tarjeta

- `▶ Run` — arranca o resume.
- `⏸ Pause` — checkpoint blando (preserva Journal).
- `⏹ Stop` — checkpoint duro (abandona loop, preserva partial diff).
- `🔱 Fork` — crea sesión hija `HUMAN_IN_LOOP` con contexto copiado, igual que Cursor fork. Permite explorar dos branches de razonamiento sin perder la original.
- `🚑 Recover` — lanza Execution Supervisor (`19`) para reanudar desde último checkpoint.
- `💬 Steer` — abre inline prompt para inyectar feedback al subagente (pattern Hermes `hermes chat --steer`).
- `👁 Show reason` — expande el chain-of-thought ticker de ese subagent en un panel dedicado (ver §5).
- `🎬 Demo` — abre el player de demo (ver §9).

### 3.3 Card types (taxonomy)

Esta RFC reconoce los siguientes **tipos de card** distintos:

| Tipo | Componente | Fuente | Notas |
|---|---|---|---|
| `agent` | `AgentCard.svelte` | RFC 03 engine events | Tipo default descrito en §3.1 |
| `autoresearch` | `AutoresearchCard.svelte` | RFC 19 §11 / RFC 28 §A | Baseline, best metric, step progress, sparkline, button Pause / Stop. Emite `POST /autoresearch/cancel`. Live desde commit `853da30` (Phase 1.5b). |
| `graph` (planned) | `<GraphView>` | RFC 28 §C / M15 `mission_graph` | Render del DAG persistido para una mission. Toggle viewport kanban↔cascade↔graph. Card badge: `EXTRACTED`/`INFERRED`/`AMBIGUOUS`. Detrás de feature `dag_mode` (default off). Endpoint `GET /hud/graph/:id` (RFC 28 §C item 7). |

Cualquier nuevo card type debe declararse aquí primero, añadirse a la sidebar toggle (# views) y exponer su tipo de evento en `src/lib/stores/hud.ts`.

---

---

## 4. Live updates — WebSocket events + animación

El HUD es **single source of truth** pero la fuente es el Journal en SQLite. El desktop twitch: runtime Rust publica eventos por un WebSocket local; cualquier cliente web (móvil, otro monitor, otro perfil) se suscribe.

### 4.1 Eventos

```typescript
type HudEvent =
  | { type: 'agent.status'; agent_id: string; status: AgentStatus; ts: string }
  | { type: 'agent.diff';   agent_id: string; delta: Diff;        ts: string }
  | { type: 'agent.tokens';  agent_id: string; in: number; out: number; cost: number; ts: string }
  | { type: 'agent.heartbeat'; agent_id: string; ts: string }
  | { type: 'approval.request'; approval_id: string; agent_id: string; action: SensibleAction; ts: string }
  | { type: 'approval.decision'; approval_id: string; decision: 'apr'|'deny'|'steer'|'fork'; user_id: string; ts: string }
  | { type: 'doom_loop.detected'; agent_id: string; count: number; ts: string }
  | { type: 'goal_drift.detected'; agent_id: string; drift: number; ts: string }
  | { type: 'journal.checkpoint'; checkpoint_id: string; ts: string }
  | { type: 'mission.consolidated'; mission_id: string; verdict_id: string; confidence: ConfidenceLevel; ts: string }
  | { type: 'mission.locked';       mission_id: string; planning_session_id: string; ts: string }
  | { type: 'research.completed';  research_run_id: string; ts: string }
  | { type: 'skill.activated';      agent_id: string; skill_id: string; ts: string }
  | { type: 'cost.threshold.crossed'; agent_id: string; threshold: number; cumulative: number; ts: string }
  | { type: 'worktree.dirty';       agent_id: string; path: string; dirty: boolean; ts: string };
```

### 4.2 Animaciones

- Cambios de `status` → pulse 0.6s en la card, color tie al dominio (running=verde, reviewing=amarillo, doom_loop=rojo).
- `agent.diff` → líneas del diff aparecen en orden, no batch.
- `cost.threshold.crossed` → ribbon naranja, push al activity streamer con confirm.
- `doom_loop.detected` → overlay rojo con badge ⛔ + dos botones: `🚑 Recover` o `🔓 Override with reason` (auditable).
- `approval.request` → dock en la barra inferior + sonido (opcional).

### 4.3 Hermes-style WebSocket channels

Siguiendo el patrón Hermes (https://docs.nousresearch.ai/hermes/cli/commands/hermes/), distinguimos 3 canales:

- `mission` channel: estado de missions y subagentes.
- `audit` channel:Every sensible action.
- `chat` channel: steer del usuario hacia un agente.

Cada cliente web se autentica con OIDC (`17 - UI.md` §6) y se suscribe a un subconjunto por scope.

---

## 5. Approvals queue — sidebar dedicado

Tres modalidades (mas allá del queue solo):

1. **Batch approval** — aprueba una lista de `SensibleAction` juntas si no hay conflictos de archivo.
2. **Scope approval** — aprueba un patrón (p.ej. "todos los edits a `src/auth/*.go` por el backend agent en esta mission"). Sirve para no tener que aprobar 1 a 1.
3. **Pauserule** — en vez de por acción, establece reglas tipo ` cuando detecte un paso que cree archivo nuevo` → `auto pause`. Las reglas persisten en `18 - Security.md`.

```
┌──────────────────────────────────────────────┐
│ Approvals (4)                          [⋮]   │
├──────────────────────────────────────────────┤
│ ⚠ AgentX   crear src/auth/session.go        │
│   pattern:  new file                        │
│   [APR] [DENY] [STEER] [FORK]               │
├──────────────────────────────────────────────┤
│ ⚠ AgentZ   abrir puerto 4317                │
│   pattern:  network binding                  │
│   [APR] [DENY] [STEER]                       │
├──────────────────────────────────────────────┤
│ Batch: 3 seleccionados (0 conflictos de file)│
│ [APR all]  [DENY all]                        │
├──────────────────────────────────────────────┤
│ ⛙ Approval rules                            │
│   + Por patrón                               │
│   + Por scope                                │
└──────────────────────────────────────────────┘
```

Permite también reglas automáticas (pauserule modo Hermes `auto skip`):

```json
{
  "pauserule": {
    "when_pattern": "network_binding",
    "action": "pause_always",
    "audit": true
  }
}
```

---

## 6. Costo & recursos view

Tomado de Hermes (https://docs.nousresearch.ai/hermes/features/dashboard/) y ampliado por modelo local:

- Cost por subagente (rolling bar).
- Cost por mission (stacked bar por model role).
- Tokens in/out line chart por hora.
- VRAM usage gauge (cuda info vía LM Studio / `nvidia-smi`).
- RAM usage.
- Throughput tok/s por modelo.
- Concurrency de conexiones (útil para respetar rate limit de free providers `21 - Execution Modes.md` §3).
- Budget restante (del Execution Supervisor `19`).

Muestra **alertas de cost boundary**: al 80% / 100% del presupuesto de mission, overlay amarillo/rojo.

---

## 7. Health status KPIs

Estilo Hermes HUD (https://docs.nousresearch.ai/hermes/overview/). Un panel superior con:

```
┌─────────────────────────────────────────────────────────────┐
│ Agents:  ● 4 healthy   ⚠ 1 degraded   ⛔ 1 doom_loop       │
│ Avg confidence:  ███████░░░ 0.72                           │
│ Heartbeat:        💓 last 2s                                │
│ Queue depth:      3 (backlog ~5min)                         │
│ Checker status:  ✅ LSP ✅ tsc ✅ vitest ⚠ biome    │
│ Journal writes/s: 4.2                                      │
│ Latency HUD:      38ms                                      │
└─────────────────────────────────────────────────────────────┘
```

- **depth queue**: cuántos subagentes en estado `queued` esperando slot.
- **Heartbeat**: cada subagente publica latido cada 2s. Si falta ≥ 10s → `degraded`. Si falta ≥ 30s → pedir revivir.
- **Checker status**: verde/amarillo/rojo por cada tool del Validation Engine (`13`).
- **Journal writes/s**: throughput de actividad del Kernel Bus.
- **Latency HUD**: latencia end-to-end desde evento emit hasta render (objetivo <100ms, ver `17` §10).

---

## 8. Skill + MCP management en caliente

Drag-and-drop de skills/MCPs a agentes activos, sin restart. Patrones basados en Hermes `hermes bots refresh` y `hermes status` (https://docs.nousresearch.ai/hermes/overview/bots/).

```
┌──────────────────────────┬──────────────────────────┐
│ Skills catalog           │ AgentX skills active     │
│  ★ mail:                 │  • React UI Expert       │
│   ▸ auth-expert ★ ★ ★ ★ ★│  • Tailwind Expert       │
│   ▸ stripe ★ ★ ★ ★       │ [+ drag here]            │
│   ▸ pytest ★ ★ ★ ★ ★    │                          │
│ MCP servers:             │ Drag-drop transaction:   │
│   ▸ github               │  1. Validate             │
│   ▸ context7             │  2. Snapshot             │
│   ▸ postgres              │  3. Activate             │
└──────────────────────────┴──────────────────────────┘
```

Al arrastrar ocurre en orden:

1. **Validate** — Skill firmada y compatible con el rol del subagent (Security `18`).
2. **Snapshot** — checkpoint de Journal.
3. **Activate** — Kernel Bus emite `skill.activated`, el subagente la carga.

Si falla validate, animation de *drag-back* con toast rojo explicando el veto.

### 8.1 Refresco en caliente

Botón `Refresh` invoca `hermes bots refresh`-equivalente local: re-enumera skills/MCP sin matar sesiones. El subagent activo queda expuesto a nuevas skills en su próximo step.

### 8.2 Per-skill overrides

Como ya está en `17 - UI.md` §4 — cualquier override manual persiste y retroalimenta Learning Engine (`16`).

---

## 9. Demos over diffs (Cursor lift)

Cada card con `[🎬 Demo]` produce su propia grabación equivalente a Loom (https://www.loom.com/):

- **Video del razonamiento**: narración generada por TTS del chain-of-thought del subagent. La track de audio es transcript generada automáticamente (como Loom).
- **Frame viewer con timestamps**: cada tool call se marca como un chapter; click en el chapter salta a ese punto del video.
- **Comentarios con timestamp**: el usuario puede comentar el frame, y ese comentario se vuelve un `feedback` que Learning (`16`) registra.
- **Overlays de draws**: screenshot delination de archivos tocados con flechas (`Draw on screenshot`).
- **URL de preview** si el subagent expuso uno (`http://localhost:3000`).
- **QR para móvil** (Loom tiene iOS/Android) — abre el video en el celular.

El efecto es: en vez de leer un diff de 600 líneas, el usuario ve un screen recording de 40s con narración. Cursor lo introdujo para cloud agents (https://cursor.com/cloud); lo traemos local.

---

## 10. Audit timeline (append-only, hash-chained)

Estilo block explorer. Cada acción sensible (sensible_action `18 - Security.md`) se appenda con:

```
{ timestamp, actor (agent or user), action, inputs, outputs,
  previous_hash, this_hash, signature? }
```

`previous_hash = sha256(prev_entry || this_entry_canonicalized)`.

```
┌──────────────────────────────────────────────────────────────────────┐
│ Audit                                                                │
├──────────────────────────────────────────────────────────────────────┤
│ #a8c9 By AgentX   2026-07-13T14:21:09Z   Skill activated codex5     │
│        prev: #a8c8   hash: 0x74ab1c…                                  │
│ #a8c8 By user      2026-07-13T14:20:55Z   STeer "use J clue"        │
│        prev: #a8c7   hash: 0x223fe9…                                  │
│ #a8c7 By AgentZ   2026-07-13T14:20:02Z   File created auth.go       │
│        prev: #a8c6   hash: 0x9921a4…                                  │
└──────────────────────────────────────────────────────────────────────┘
```

- Filtro por actor, mission, action_type, time range.
- Export a JSON/SARIF.
- Tamper-evident: Detectionde chain break recalculando hashes → alerta roja.

Cualquier `SENSIBLE_ACTION` de `18 - Security.md` emite un entry. Non-sensible actions solo van al activity streamer sin hashear.

### 10.1 YAML on-disk export (posting-format compatible)

**Origen:** RFC 28 §D (Phase 1.5a). **Implementado en Phase 1.5a.**

La Audit timeline del §10 puede exportarse a disco como colección de archivos `.posting.yaml` compatibles con el formato del CLI open-source `darrenburns/posting` (Apache-2.0). **No hay dependencia runtime a posting** — solo se replica el schema on-disk para portabilidad humana e inspección con `posting --collection ./snapshots/` (opcional). Versión de schema persistida en cada snapshot bundle.

**Endpoints/UI:**
- HUD botón **Export as posting** en la card Audit (RFC 24 §3) — llama `POST /audit/export-posting` con `{ last?, output_dir? }`.
- CLI: `opencode audit --export-posting [DIR]` (default DIR = `<profile>/snapshots/`). Opcional `--snapshot-maybe` para forzar snapshot before purge.
- Tauri save dialog nativo para elegir `output_dir` (no plugin extra).

**Packing** (implementado en `src-tauri/src/journal/export/retention.rs::snapshot_entries`):
- Day-bucketed: `<snapshot_root>/<YYYY-MM-DD>/audit_<n>.posting.yaml`.
- Atomic write: `.tmp` + `rename` (no `.tmp` residuo en success).
- Max 100 entries por archivo (split en chunks `audit_000.posting.yaml`, `audit_001.posting.yaml`, …).
- Document separators (`---`) entre entries en un mismo archivo (modo collection).

**Mapping audit entry → posting request** (`src-tauri/src/journal/export/posting.rs::entry_to_posting_yaml`):
- HTTP entries (`action == "http_request"`) → typed fields: `method`, `url`, `headers`, `query_params`, `body`, `auth` (basic/digest/bearer_token).
- Non-HTTP entries → campos `x-opencode-*` (extension keys) preservando `actor`, `action`, `inputs`, `outputs`, `previous_hash`, `this_hash`, `signature`, `timestamp` (mapeados desde `AuditEntry`).
- Header fijo: `x-opencode-exported: RFC 28 §D` (campo `x_opcode_exported` en el struct, serializado con `serde(rename = "x-opencode-exported")`) como provenance-line del snapshot dir, self-describing para git review.
- **Campo `scripts:` deliberadamente ausente** — security boundary AGENTS.md §6. Boundaries enforced en struct + reject de top-level `scripts:` en tests.

**Compatibilidad:** parses clean con `yq` / `posting --collection` / `serde_yaml::from_str`. Snapshot bundle incluye `README.md` con versión de schema y `posting_version`.

Referencias: RFC 28 §D para derivación de algoritmos y apéndice de atribución; `OpenCode OS/research/27 - posting format.md` para investigación de port del helper `str_presenter` → `literal_block`.

---

## 11. Mission activity log (streamercono colors)

El activity streamer (sidebar derecho) usa colores por evento para que el humano pueda *scanningar* de lejos:

| Color | Evento |
|---|---|
| ⬜ gris | info |
| 🟩 verde | success / code merged / validation pass |
| 🟨 amarillo | approval requested / waiting user |
| 🟧 naranja | cost threshold / degradation |
| 🟥 rojo | doom_loop / goal_drift / error |
| 🟪 púrpura | research completed / Learning rule extraída |
| 🟦 azul | steer from user |

La barra vertical es orden cronológico; el usuario puede tocar un row → abre la tarjeta del subagent y salta al frame de Demo (ver §9).

---

## 12. Worktrees visuales

Para cada Mission: un mini-git-graph local con subagentes como nodos.

```
Mission "Auth refactor"
├─ main ● ─────────────────────● ─────────────────●
│             \                ↑                  ↑
│              ├─ worktree ~/oc-auth/backend  ● MERGE
│              ├─ worktree ~/oc-auth/frontend ● MERGE
│              └─ worktree ~/oc-auth/db       ⛔ halted
│
└─ Origin: ~/dev/auth-refactor
```

Cada rama color tie al contador de dirty files. Permite al usuario *ver* cuántos worktrees están abiertos y actuar (`Close worktree`, `Merge now`, `Compare`).

Sigue el patrón Hermes que ata profile a worktree (https://docs.nousresearch.ai/hermes/overview/profiles/), pero **multi-agente dentro de un mismo profile**.

---

## 13. Canvas view (estilo n8n)

Vista alternativa del main pane. En vez de kanban, un grafo dirigido donde los nodos son subagentes y las aristas son dependencias/handoff. Patrones heredados de n8n (https://docs.n8n.io/workflows/) y Windmill (https://docs.windmill.dev/).

```
┌──────────────────────────────────────────────┐
│ Planner ──┬──▶ Researcher ─▶ Architect       │
│           │                          │       │
│           └──▶ Architect ──┬─▶ Backend       │
│                            ├─▶ Frontend      │
│                            └─▶ Database      │
│                                                 │
│   Backend → Reviewer ← Frontend                 │
│              │                                  │
│              ▼                                  │
│           Merger                               │
└──────────────────────────────────────────────┘
```

Click en nodo → seleccionar y ver razón. Al dispararse el `doom_loop` en un nodo, parpadea rojo y las aristas downstream quedan dimmed. Siguientes patrones de n8n:

- **Execution log por nodo** — click → panel con chain-of-thought prompt-era tool calls en orden, con timestamps, tokens, cost, inputs/outputs JSON. Esto reemplaza "mirar el log" para detectar por qué el agente falló.
- **Retry / Continue-from-here** — botones por nodo.
- **Trigger manual** — dispara un subagent en particular.

`Reasoning trail` = chain-of-thought prompt-era tool calls ordenados con timestamps + tokens + cost + inputs/outputs JSON (isto es el `n8n execution log` pero aplicado al reasoning agent).

---

## 14. Outline view (estilo Notion)

Muestra la mission consolidada (`23`) y su descomposición como árbol Notion:

```
Mission Auth Refactor (mission_id) — [locked 14m ago] 🏛 architect
├─ Objective 1: Setup passport
│   ├─ Task 1: Install deps                       [✅ Done]
│   ├─ Task 2: Config strategy local              [▶ Doing AgentX]
│   └─ Task 3: Add tests                           [⏸ waiting review]
├─ Objective 2: Session management
│   ├─ Task 4: Redis session store                 [▶ Doing AgentY]
│   └─ Task 5: Cookie secure flag                  [🔍 Researching]
├─ Objective 3: OAuth integration
│   └─ Task 6: Google OAuth                        [🛑 Blocked]
```

Comentarios **block-level** estilo Notion: el usuario hace hover sobre una Task → `💬 Comment`, y el comment queda attachado a la Task en el Journal. Cada comment puede invocar `/steer` o `/fork`.

Propiedades estilo Notion por task: `status`, `assignee`, `priority`, `tags[]`, `effort_estimate`, `model_id`, `skill[]`.

Permite **ARkery editing**: arrastrar tasks entre objectives reordena el Plan sin re-generar todo.

---

## 15. Timeline view

Eje horizontal tiempo, tracks verticales por subagente. Eventos del Journal como diamonds:

```
AgentX ─◇─◇─◇───────◆───◆──✱─────◇─
AgentY ─────◇───◇───◆───────────✱──
AgentZ ─────────◇───────◆─★─────────

◆ = tool call
◇ = heartbeat
✱ = decision
★ = error
```

Zoom in/out (click → frame de demo, click → entry audit). Es una visual muy útil para analítica "¿cuándo el subagent se atascó?".

---

## 16. Mobile + remote access

Siguiendo Hermes (https://docs.nousresearch.ai/hermes/overview/) y Loom (https://www.loom.com/):

- **Web responsive** — accesible desde móvil/desktop. Autenticación OIDC (`17 §6`).
- **Push notifications**:
  - Approval waiting.
  - Cost threshold crossed.
  - doom_loop detectado.
- **Mobile review**: una Approvals queue simplificada móvil. Acciones `[APR]` `[DENY]` en una mano. Sirve para revisiones async (patrón Loom).
- **Share mission link**: genera URL public/private con expiration para enseñar el HUD de una mission a un colega (read-only audit + demo player).
- **Capture terminal**: tap en una card → screenshot del frame actual del subagent.

Esto da el acceso desde cualquier parte del mundo que pidió el usuario (`17 - UI.md` §6).

---

## 17. Reporter de approving (rúbrica)

Después de cada mission cerrada, el HUD despliega un reporte como in-app card (estilo Cursor end-of-mission):

```
╔════════════════════════════════════════════════════════════════╗
║ Mission Auth Refactor — Completed                              ║
║                                                                 ║
║ Confidence initial → final:   0.72 → 0.91                       ║
║ Verdict initial:        MEDIUM (3 gaps)                          ║
║ Clarifications asked: 2 of 3 auto-resolved from Architecture    ║
║ Memory                                                           ║
║                                                                 ║
║ Duration: 14m 22s                                               ║
║ Files touched: 6                                                ║
║ Diff:        +262 / -26                                          ║
║ Tokens in:    58,123 out: 9,212                                 ║
║ Cost:        $0.42                                               ║
║                                                                 ║
║ Tasks: 6  ✅ 4   ⏸ 1   🛑 1                                      ║
║                                                                 ║
║ Validation: ✅ tsc  ✅ vitest  ✅ biome  ⚠ semgrep 0 warns     ║
║                                                                 ║
║ Approvals required: 4 approvals in 14m 22s                     ║
║   3 auto-approved via scope policy                              ║
║   1 manually approved (user) at 9m 18s                          ║
║                                                                 ║
║ Skill activations: 2                                             ║
║   `auth-expert`, `passport.js`                                  ║
║                                                                 ║
║ Learning rules extracted: 3                                     ║
║   • "separate Redis adapter for sessions"                       ║
║   • "use bcrypt cost factor 12 by default"                      ║
║   • "validate origin before applying oauth"                     ║
║                                                             [⌘]  ║
╚════════════════════════════════════════════════════════════════╝
```

Integra con:

- `22 - Research Findings.md`: fuente de los metrics.
- `21 - Execution Modes.md`: confidence threshold que permitió modo architect o code.
- `23 - Prompt Understanding & Refinement.md`: verdict mostrado arriba para que el humano vea qué se asumió.
- `19 - Execution Supervisor.md`: contadores de approvals y cost.

---

## 18. Workflows integrados con el HUD

### 18.1 Workflow "usuario pide algo"

1. Usuario escribe prompt crudo en `+ New Mission` (sidebar).
2. HUD muestra **Mission understanding panel** (panel inline superior) con:
   - `PublicUnderstandingVerdict` (ver `23 §3`).
   - Barra de confidence (color tied HIGH/MED/LOW/BLOCK).
   - `clarification_questions` listadas.
3. Si `confidence = HIGH`, botón `🔓 Lock & Plan` activado; si `MEDIUM`, botón `🏛 Architect + plan`; si `LOW/BLOCK`, botón `💬 Ask user`.
4. Al click de `Lock & Plan`, HUD anima la Mission consolidated to Plan view (Outline).

### 18.2 Workflow doomscape

```
Detected doom_loop → HUD overlay rojo
   ⛔ doom_loop detected on AgentX (count: 3 idéntico consecutivo)
   Action: [🚑 Recover via Execution Supervisor]
          [🔓 Override with reason]
          [⏹ Stop agent]
   Reason override: ____________________________
   [Submit]  (logged in audit)
```

`Override reason` cadenas el audit con hash chain (`§10`). La submission envía al Learning Engine que extrae "cual prompt cuerda causa doom_loop" como nueva regla.

### 18.3 Workflow goal_drift

Similar pero naranja. Indica que el `intent` original (`23`) y el último step plan/dispatch se distanciaron. Badge ⚠ en card de subagent. Botón `Steer realine` abre prompt prefilled con el intent original para reenviar al agente.

### 18.4 Workflow cost budget exceeded

```
AgentX cost threshold crossed (cumulative $0.42 ≥ budget $0.40)
Options:
   [ +10% budget ]  [+'s notificado ]  [⏸ pause]
```

Persistente en audit.

---

## 19. Keyboard navigation

Toda la UI navegable por teclado (para humanos rápidos, ver `17 - UI.md` §12):

| Key | Acción |
|---|---|
| `:m` | Cambiar mission |
| `:v` | Cambiar view (Kanban/Canvas/Outline/Timeline/etc) |
| `:a` | Foco a approvals queue |
| `:n` | New mission |
| `:f` | Fork selected |
| `:s` | Steer selected |
| `:d` | Show Demo |
| `:r` | Run selected |
| `:p` | Pause selected |
| `:x` | Stop selected |
| `:c` | Comment on selected |
| `:e` | Expand selected to canvas全景 |
| `:o` | Mode selector (ask/architect/code/context) |
| `:t` | Toggle Health KPIs dock |
| `?` | Help overlay |

---

## 20. Performance y scale

- Todas las vistas se renderizan de **virtualized list / virtualized tree** (scroll pequeño ~200 DOM nodes visibles).
- Diff highlight vía web worker para no bloquear main thread.
- WebSocket events encolados, flushed cada 50ms → batch updates reactivos.
- Snapshot de Journal cada 30s salvage/restaurable (cf `19`).
- En desktop Tauri 2 Rust consume ~30MB para 主线程 HUD (ver `25`).

---

## 21. Configuración del HUD

`~/.opencode/hud.yaml`:

```yaml
views:
  default: kanban
  pinned: [kanban, canvas, audit]
activity_streamer:
  enabled: true
  height_px: 240
push:
  approvals: true
  doom_loop: true
  cost_threshold_pct: 80
  cost_threshold_pct_critical: 100
demos:
  tts_narration: true
  draw_overlays: true
mobile:
  enabled: true
  oidc_provider: https://auth.example.com
audit:
  hash_chain: true
  export_format: [json, sarif]
keyboard:
  custom:
    "Ctrl+Shift+F": "fork_selected"
```

---

## 22. Limitaciones explícitas

- El HUD **no consume prompts ni escribe código**. Es la visibilidad y el steering. El LLM作業 corre en el Agent Engine Kernel (`02`).
- No replace el editor de código para diff side-by-side detailed; ese flujo abre `diff pane` separado (están en `17 - UI.md`).
- DEMos via TTS requiere permiso de audio; si el usuario desactiva, queda solo Frame viewer sin narración.
- Hash chain audit está diseñado para tamper-evidence local; para compliance multi-user, requiere write-through to external append-only log (out of scope por ahora; ver `18 - Security.md`).
- Multi-user moderno simultáneo en un mismo HUD no está soportado en v1; cada OIDC profile tiene su HUD sliding NYI.

---

## 23. Outputs hacia otros RFCs

- `02 - Agent Operating System.md` — Kernel Bus debe emitir eventos `HudEvent` (`§4.1`) con timestamps.
- `05 - Swarm.md` — subagentes publican heartbeats y state transitions consumidos por HUD; el patrón "see swarm from anywhere" de `05 §7` se materializa aquí.
- `10 - Research Engine.md` — HUD consume research.completed para taggear tarjetas con "evidence N".
- `12 - Planning Engine.md` — Outline view consume el Plan generado, Plan objects expuestos para drag-edit.
- `16 - Learning Engine.md` — Demo comments, steer texto y override reasons son fuente de aprendizaje.
- `17 - UI.md` — Reemplaza la Mission Control figurada y expande; muchas partes (`17 §2.3`) son referenciadas.
- `18 - Security.md` — Sensible actions vetoada aparecen en approvals queue.
- `19 - Execution Supervisor.md` — doom_loop, goal_drift, recovery actions boost === HUD; checkpoints consumidos por Worktrees view.
- `21 - Execution Modes.md` — badges 🕊/🤝/🛫/🚀 visibles en toda tarjeta.
- `23 - Prompt Understanding & Refinement.md` — Mission understanding panel refiere user prompts crudos → verdict → consolidated; HUD los expone.
- `25 - Stack Técnico Multiplataforma.md` — stack técnico que permite este HUD (Tauri 2, Rust WebSocket server).

---

## 24. Estado

- Status: Draft v1
- Depends on: `02`, `05`, `10`, `12`, `16`, `17`, `18`, `19`, `21`, `23`, `25`
- Referencias externas: Hermes HUD docs (Nous Research), Cursor Cloud Agents, Loom timeline, Linear/Asana recursive hierarchy, Notion block-level comments, n8n canvas execution logs, Windmill flows.
- Cubre explícitamente el pedido del usuario: "panel admin estilo Jira + Cursor subagentes + Notion + n8n integrado".
