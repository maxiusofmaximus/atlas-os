# TODO central — Atlas OS

- **Autor:** Todo Updater · **Fecha:** 2026-10-06 · **Snapshot git:** HEAD `1292fe8` · `origin/main` `82373ed` (**31 ahead**) · working tree **69 entradas** sin commitear. [O]
- **Fuente de verdad:** cuadra con `PLAYBOOK.md`, `20 - Roadmap.md`, `research/63`, `RFC 67 v3`, `docs/audit/*` y `PLAN-docs-hardening.md`. Si discrepan: gana código+git, y se registra en §Contradicciones.
- **Alcance:** solo este fichero. Sin git, sin `src/` ni `src-tauri/`. [O]=observado en repo, [Os]=dicho por el PL y contrastado con fichero, [I]=inferido, [P]=pendiente.

## 1. Tabla consolidada

| ID | Grupo | Item | Dueño | Estado | Deps | Evidencia / aceptación | Prio |
|---|---|---|---|---|---|---|---|
| R1 | Ronda | Catálogo UX (`research/64`) + deep-dive `ux-catalog/*` | Researcher | **IMPLEMENTADO** | — | `research/64` **ACEPTADO**: recuento exacto **441** (`tiene_UI` sí376/no38/parcial27; 94 HUD-relevantes) [O]. `count.mjs` (pegado): **132 fichas · [Os] 62 · [Os parcial] 55 · [P] 15 · sin_tag 0** (PL, 2026-10-06 tras cierre; `count.mjs` ya independiente del cwd; prioridad A: 39 Os / 27 parcial / 11 enlazada / 12 P / 4 sin ficha); por fichero ade40/chat21/term22/obs11/ide10/outline5/canvas5/kanban4/settings4/otro5 [O]. `_pending-A.md` recalculado por script y `_funcion-x-referente.md` entregado (4 funciones NO-DOCUMENTADAS) → **IMPLEMENTADO** | P0 |
| R2 | Ronda | UI Specification (RFC 67) | UI/UX Designer | **IMPLEMENTADO (doc) / VALIDADO parcial** | R1,R3,R4,R5 | `RFC 67` **v3**, 670 líneas; §22 plan FASE 10 + §22.3 gates G1–G8 [O]. PL verificó huecos **H-02/H-03/H-04/H-08/H-09** contra código [Os] | P0 |
| R3 | Ronda | Inventario backend | **Builder-2** (relevó al Builder) | **VALIDADO parcial** | — | `docs/audit/backend-capabilities.md` (276L): **41 rutas** (39+2 gated), **26 WS**, **34 subcomandos CLI**, **61 tablas** + `schema_version` + **1 virtual FTS** [O]; PL verificó 41/34/61+schema+FTS [Os] | P0 |
| R4 | Ronda | Inventario frontend actual | Builder-2 | **VALIDADO** | — | `docs/audit/frontend-current-state.md` (277L): `+page.svelte` **1237** / `hud.ts` **1585** / `views.ts` **79**; **`src/app.css` AUSENTE**; **24 componentes**; 531 literales/30 hex [O]; PL verificó esas cifras [Os] | P0 |
| R5 | Ronda | Auditoría RFC↔código + log de correcciones | Builder-3 | **VALIDADO** | — | `rfc-vs-code.md` (138L) + `rfc-corrections-log.md` (**21** correcciones) [O]; PL verificó 8 cifras contra código, coinciden [Os] | P0 |
| R6 | Ronda | TODO central | Todo Updater | **IMPLEMENTADO** | R1–R5 | este fichero [O] | P0 |
| R7 | Ronda | Cierre de la ronda | Project Lead | **PLANIFICADO** | R1–R6 | `PLAN-docs-hardening.md` §"Criterio de cierre" [O] | P0 |
| F0 | FASE 10 FE | Tokens: crear `src/app.css` `--a-*`, sustituir 531 literales | *(propuesto: Builder-2; decide PL)* | **PLANIFICADO** | — | RFC67 §22.1: `rg "#[0-9a-fA-F]{3,8}" src -g "*.svelte"` → 0; captura dark/light | P0 |
| F1 | FASE 10 FE | Agent Card real (§4, capas 0–3, error/loading) | *(propuesto: Builder-3)* | **PLANIFICADO** | F0 | RFC67 §22.1: test de componente render+vacío+**error**; H-03 | P0 |
| F2 | FASE 10 FE | Mission Rail (nuevo) + disolver `overview` | *(propuesto: Builder)* | **PLANIFICADO** | F0,F1 | RFC67 §22.1: cambiar misión no reconecta WS | P0 |
| F3 | FASE 10 FE | Activity Spine (ticker, `aria-live`) | *(propuesto: Builder)* | **PLANIFICADO** | F0 | RFC67 §22.1: `doom_loop_detected`→`err`; reduced-motion | P0 |
| F4 | FASE 10 FE | Approvals Dock (gate/pregunta) | *(propuesto: Builder)* | **PLANIFICADO** | F0 | RFC67 §22.1: `Apr`→`POST …/approve`; H-02 | P0 |
| F5 | FASE 10 FE | `views.ts`/hotkeys 13→10 ids; RFC 24 §19 | *(propuesto: Builder-2)* | **PLANIFICADO** | F0 | RFC67 §22.1: `views.test`; `:v`, `:a` | P0 |
| F6–F9 | FASE 10 FE | Refactor por view (loading/error/skeleton) | *(propuesto: Builder/Builder-2)* | **PLANIFICADO** | F0–F1 | RFC67 §22.1: `pnpm check`+`pnpm test` por componente | P1 |
| F10 | FASE 10 FE | MCP/Worktrees scope v1 vs v2 | *(propuesto: Builder-3)* | **PLANIFICADO** | F0 | RFC67 §22.1: no anunciar hot-swap ni grafo | P1 |
| B1 | FASE 10 BE | Des-gatear `/graph/:mission_id` (H-04) | *(propuesto: Builder)* | **PLANIFICADO** | — | RFC67 §22.2: test feliz+fallo; `cargo test … hud::graph` | P0 |
| B2 | FASE 10 BE | `AgentStatus::Unknown` + serde (H-03) | *(propuesto: Builder)* | **PLANIFICADO** | — | RFC67 §22.2: round-trip + string inválido | P0 |
| B5 | FASE 10 BE | `budget_usd`+`remaining` (H-06) | *(propuesto: Builder)* | **PLANIFICADO** | — | RFC67 §22.2: con/sin budget | P0 |
| B7 | FASE 10 BE | `POST /hud/missions` + `hud/missions.rs` (H-09) | *(propuesto: Builder)* | **PLANIFICADO** | — | RFC67 §22.2: crea; prompt vacío→400 | P0 |
| GATE | FASE 10 | Cierre FASE 10: **G1–G8** | Project Lead (verifica) | **PLANIFICADO** | F0–F10, B1/B2/B5/B7 | RFC67 §22.3: G1 `pnpm check`=0 · G2 `pnpm lint`=0 · G3 `pnpm test` verde · G4 clippy `-D warnings` · G5 `cargo test --lib` verde (B1/B2/B5/B7) · G6 0 hex en `.svelte` · G7 capturas dark/light (4 comp.) · G8 0 `KernelCommand` en `src/lib` | P0 |
| B3 | FASE 11 BE | `POST /hud/approvals/batch` + persistir `reason` (H-02) | *(propuesto: Builder-2)* | **PLANIFICADO** | GATE | RFC67 §22.2: batch→todas; conflicto→409 | P1 |
| B4 | FASE 11 BE | `GET /hud/missions?sort=frecency` (H-01) | *(propuesto: Builder-2)* | **PLANIFICADO** | GATE | RFC67 §22.2: orden frecency; journal vacío estable | P1 |
| B6 | FASE 11 BE | `Sandbox::snapshot`→`GET /hud/agent/:run_id/snapshot` (H-08) | *(propuesto: Builder-3)* | **PLANIFICADO** | GATE | RFC67 §22.2: frame; sin sandbox→404+reason | P1 |
| B8 | FASE 11 BE | `task_annotations` (schema + ruta) (H-05) | *(propuesto: Builder-3)* | **PLANIFICADO** | GATE | RFC67 §22.2: round-trip; body vacío→422 | P1 |
| F32 | F32–F38 | Baseline externo (Terminal-Bench 2) | Harness | **VALIDADO** | modelo (A1) | commits `7775174`,`86b844e`,`c878339`; `eval/harbor.rs` [O]; pass_rate 0.000, oracle 0.88 | P1 |
| F33–F37 | F32–F38 | Gate fiabilidad · coste/Diff · apply · research-evidence · swarm merge | Orchestrator/Swarm | **IMPLEMENTADO** | — | `0735daf`,`98ea6bc`,`e39edc6`,`db75839`,`4248293` + código [O] | P1 |
| F38 | F32–F38 | Harness agent-mode | Harness | **IMPLEMENTADO** | F39,F32 | `0bf6b0e` (corrida **0/11**) [O] | P1 |
| A1 | Pendientes | Correr harness con modelo de frontera | operador/modelo | **BLOQUEADA** | endpoint | `research/63 §E #1` "frente #1 = A1"; `§A1` | P1 |
| A3 | Pendientes | Adaptador multicanal `teloxide` | operador | **PENDIENTE-DECISIÓN** | token | `research/63 §E #4`: núcleo+seam ✅ (`90f18b5`,`eaaad4b`); adaptador pendiente | P2 |
| BUILD | Pendientes | Decidir default build (fastembed/dag_mode) | operador | **PENDIENTE-DECISIÓN** | ADR 0002 | `research/63 §E #2`; ADR 0002 (`3475d25`) fastembed ICEa | P2 |
| FASE13 | Pendientes | Laya real | — | **BLOQUEADA** | upstream | `research/63 §C` | P3 |
| GIT | Git | Publicar 31 commits locales | Project Lead | **PLANIFICADO** | — | HEAD `1292fe8` vs `82373ed` [O]; no ejecutado aquí | P1 |

**Línea base (2026-10-06):** `cargo test --lib` **1392 ok / 1 ignored**; `pnpm test` **116**; `pnpm check` **0**; clippy `-D warnings` limpio; `pnpm lint` verde tras añadir a `.prettierignore` `docs/audit/ docs/coordination/ docs/design/ docs/TODO.md "system prompt.md"` y formatear `docs/adr/0002`. [O: `research/63` L23-24 + `rfc-corrections-log` L7 + `.prettierignore`; clippy/check [Os PL].]

## 2. Contradicciones

**RESUELTAS** (verificadas en disco):
- **C1** PLAYBOOK obsoleto → `PLAYBOOK.md` L11-16 "## Estado (reconciliado…2026-10-06)": F32 MEDIDO, F33–F37 CERRADAS, F38 corrido, F39 CERRADA. [O]
- **C2** Roadmap F38 "Siguiente" → `20 - Roadmap.md` L579 "**F38 hecho** … 0/11 (`0bf6b0e`)". [O]
- **C3** Vistas HUD 11 vs 13 → `research/63` L22 "**13**". [O]
- **C4** Tests 1383/67 vs 1392/116 → `research/63` L23-24 "**1392 ok** / **116 ok**". [O]
- **C6** §E "frente #1 = A2" → `research/63` L96 "frente #1 es **A1**". [O]
- **C7** puente `ToolRegistry` "pendiente" → `rfc-corrections-log` #19 lo mueve a hecho. [O]
- **C5** RFC 67 afirmaba `research/64` "AÚN NO EXISTE" → **resuelto en v3**: encabezado L6 "**ENTREGADO [O]** … 441". [O]

**NUEVAS** (encontradas ahora):
- **C9** RFC 67 cuenta las fichas de forma incoherente: L6 "**6 ENTREGADAS**" vs L8 "**ampliadas … 9 con ficha**" vs disco: `count.mjs` lista **10** ficheros de categoría (+`_pending-A`). [O]
- **C10** RFC 67 §1.12 (L250) cita `ux-catalog/herdr.md` y `orca.md` que **no existen** en `ux-catalog/` (ni los lista `count.mjs`). [O]
- **C11** Off-by-one de tamaño: RFC 67 dice `backend-capabilities.md` "275 líneas" y `frontend-current-state.md` "276"; en disco son **276** y **277**. [O]
- **C12** Árbol muy sucio (69 entradas sin commitear; incluye `docs/audit/`, `docs/coordination/`, `docs/design/`, `research/64`, `ux-catalog/`, RFC 67) mientras el snapshot sigue en `1292fe8`. Todo el inventario caduca al primer commit. [O]

**ABIERTAS / no-factuales:** `research/63 §E #2` (default build) y `#4` (adaptador `teloxide`) son **decisiones de operador**, no correcciones. [O]

## 3. Próximos pasos

1. **Project Lead:** decidir dueños reales de los lotes F0–F10 / B1–B8 (aquí van propuestos) y cerrar la ronda (R7).
2. **Researcher (R1):** recalcular `_pending-A.md` (62 prioridad-A) → cierra R1.
3. **Designer:** corregir C9/C10 (recuento de fichas y citas `herdr/orca`) en RFC 67 v4.
4. **Al cerrar R7:** desbloquear **FASE 10** con RFC 67 §22; construir en orden F0→F1→(F2·F3·F4·F5)→F6–F10→B1/B2/B5/B7→**G1–G8**; después FASE 11 (B3/B4/B6/B8).
5. **Builder-3:** aplicar C11 (líneas) en el log de correcciones.
6. **Project Lead:** commitear/integrar y publicar (GIT).
