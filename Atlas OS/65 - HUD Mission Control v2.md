# RFC 65 — HUD Mission Control v2 (producto)

**Author:** opencode architect agent · **Date:** 2026-10-04
**Status:** In progress — 8 de los 12 items implementados (P0: ViewSwitcher, AgentCard, Kanban, ApprovalQueue, CommandPalette — `c76bf68`; P1: CostDashboard/HealthKPIs/AuditTimeline; P2: Canvas/Outline/Timeline/Worktrees).
**Depends on:** RFC 17 (UI base), RFC 19 (Supervisor), RFC 24 (HUD Mission Control — spec), RFC 25 (Stack), RFC 04 §9 (frontends), RFC 05 (Swarm), RFC 14 (Validation), RFC 63 (Agentic Capability — `AgentCard`).
**Scope:** Cierra la **brecha B** de la auditoría `61`: la HUD real implementada es un **panel de debug** (8 cards + "tail boxes"), no el Mission Control especificado en RFC 24 (8 views + approvals queue + agent cards). Este RFC es el **plan de implementación** del producto descrito por RFC 24 — no añade motores; aterriza la UI.

---

## 1. Contexto

`src/routes/+page.svelte` renderiza hoy: health WS, audit export, `<AutoresearchCard>`, `<SwarmConsole>`, `<AvailabilityCard>`, `<EvalCard>`, journal tail, `<JournalObserver>` y 11 "tail boxes". Sirve para **operar**, no para **coordinar** un swarm (la promesa de RFC 24 §1: "ver el trabajo de los subagentes como Jira presenta tickets, Cursor demos, Hermes health y n8n un canvas").

Faltan (vs RFC 24):
- Las **8 views**: Kanban, Canvas, Outline, Timeline, Cost & Res, Health KPIs, Audit, Worktrees.
- **Approvals queue** multi-dispositivo (batch/scope/pauserule).
- **Agent card** de 15+ campos (steps, tool calls, evidencia, coste, modelo, worktree).
- **Demos over diffs** (video/screenshot/preview URL), **skill/MCP drag-drop**, **mobile OIDC**.

## 2. Objetivos

1. Entregar las **8 views** sobre los datos que ya emite el Kernel Bus + rutas HUD.
2. **Approvals queue** real (RFC 24 §6) con batch/scope/pauserule.
3. **AgentCard** en vivo que consume `agent_steps`/`tool_invocations` (RFC 63).
4. **Command palette** + hotkeys `:` (RFC 24 §tabla de atajos).
5. Responsive **desktop + mobile** (OIDC ya existe en `remote_auth/`).
6. Fidelidad de diseño (tokens, dark/light) acorde a un producto, no a un debug panel.

## 3. Inventario de componentes (RFC 24 → Svelte)

| View | Componente | Fuente de datos |
|---|---|---|
| Kanban | `KanbanBoard.svelte` (+ `KanbanColumn`, `TaskCard`) | missions/plans/diffs/checkpoints |
| Canvas | `CanvasView.svelte` (nodos + edges) | M15 `mission_graph` (RFC 28 §C) |
| Outline | `OutlineView.svelte` | plan milestones |
| Timeline | `TimelineView.svelte` | `journal_events` |
| Cost & Res | `CostDashboard.svelte` + `CostSparkline` | `model_invocations`, `model_resets` |
| Health KPIs | `HealthKPIs.svelte` | `agent_session_events`, supervisor heartbeats |
| Audit | `AuditTimeline.svelte` | audit hash-chain |
| Worktrees | `WorktreesView.svelte` | `swarm/` worktrees |
| (todas) | `AgentCard.svelte` (RFC 63) · `ApprovalQueue.svelte` · `CommandPalette.svelte` · `DemoPane.svelte` · `SkillMcpRail.svelte` | — |

Reutiliza lo ya existente (`AutoresearchCard`, `AvailabilityCard`, `EvalCard`, `GraphView`, `JournalObserver`, `SwarmConsole`, `ModelReadyCard`, `SpendLimitErrorCard`) como *detail panels* dentro de las views.

## 4. Contrato de datos

- **WS (Kernel Bus)** — suscripción existente `$lib/stores/hud.ts`. Añadir eventos tipados: `agent_step`, `tool_invocation`, `artifact_verified`, `approval_requested/resolved` (ya parcial), `cost_delta`.
- **REST (axum)** — reusar `GET /hud/journal`, `/hud/eval/summary`, `/hud/availability`, `/hud/reliability`, `/graph/:id`; añadir `GET /hud/agent/:run_id`, `POST /hud/approvals/:id/{approve,deny}`, `GET /hud/cost?window=`.
- **Tipos** en `src/lib/stores/hud.ts` espejo de los Rust (SCREAMING_SNAKE / snake_case).

## 5. Layout, navegación y atajos

- Layout maestro RFC 24 §2: `ModeBar` + `ViewSwitcher` + área de view + `AgentConsole` lateral + `ApprovalsDrawer`.
- Hotkeys (RFC 24 tabla): `:v` cambiar view · `:a` approvals · `:n` new · `:f` fork · `:s` steer · `:d` demo · `:r` run · `:p` pause · `:x` stop · `:c` comment.
- Command palette (`CommandPalette.svelte`) con fuzzy search sobre acciones + missions.

## 6. Design system

Tokens en `src/app.css` (`--bg`, `--panel`, `--text`, `--accent`, `--ok/--warn/--err`), dark/light (selector ya en `+page.svelte`). Tipografía/espaciado consistentes; sin dependencias UI nuevas (Svelte puro) salvo aprobación explícita.

## 7. Fasing (entrega incremental)

- **P0 (MVP producto):** Kanban + AgentCard + ApprovalQueue + CommandPalette. (Hace la HUD usable de verdad.)
- **P1:** Cost & Res + Health KPIs + Audit Timeline.
- **P2:** Canvas (GraphView) + Outline + Timeline + Worktrees.
- **P3:** Demos over diffs (video/screenshot/preview) + Skill/MCP drag-drop + mobile OIDC.

Cada fase es un `ViewSwitcher` que enciende una view sin romper las demás.

## 8. Tests

- Vitest por componente (render + props + estados vacío/error) — patrón ya usado (`hud.test.ts`, `SwarmConsole.test.ts`).
- E2E mínimo (Playwright opcional) para Kanban drag + approvals batch.
- Contrato: tipos HUD ↔ Rust (test de serialización).

## 9. Actualizaciones a RFCs existentes

- **RFC 24** — pasa de "spec" a "spec + plan de implementación (RFC 65)"; marcar brechas cerradas.
- **RFC 17** — aclarar relación editor ↔ HUD v2.
- **RFC 63** — `AgentCard` es consumidor de `agent_steps`.
- **RFC 26** — cross-refs (`KanbanBoard`, `ApprovalQueue`, `CommandPalette`).

## 10. Checklist

1. [x] Design tokens + `ViewSwitcher` + routing de views.
2. [x] `AgentCard.svelte` (consume RFC 63) + tipos WS.
3. [x] `KanbanBoard.svelte` (missions/plans/diffs) — **P0**.
4. [x] `ApprovalQueue.svelte` + endpoints approve/deny batch/scope.
5. [x] `CommandPalette.svelte` + hotkeys `:`.
6. [x] `CostDashboard.svelte` + `GET /hud/cost` — **P1**.
7. [x] `HealthKPIs.svelte` + `AuditTimeline.svelte` (`GET /hud/health`, `GET /hud/audit`).
8. [x] `CanvasView` (GraphView) + `OutlineView` + `TimelineView` + `WorktreesView` — **P2**.
9. ⏳ `DemoPane.svelte` (video/screenshot/preview URL) — **P3**.
10. ⏳ `SkillMcpRail.svelte` (drag-drop skills/MCP).
11. ⏳ Mobile OIDC + responsive audit.
12. ⏳ Tests vitest + e2e + contrato tipos.
