# 33 — Phase 6 Execution Supervisor completo (plan refinado)

**Propósito:** Refinar la Fase 6 RFC 20 ("Execution Supervisor completo") con el estado actual del supervisor (RFC 19 ya materializado: state machine formal + doom_loop + heartbeats + checkpoints persistidos — test `supervisor_driven_checkpoints_round_trip_through_journal`). Lo faltante: **reanudación desde cualquier estado** (reconstrucción del `SupervisorState` desde el journal) y el **observer web del Journal**. Output: sub-fases atómicas, sin crates nuevas (RFC 25 §11).

**Gestión:** el trabajo mecánico pesado se delega a `muse-spark-1.3-contributor-free` vía `opencode run`; el gestor revisa (senior review), commitea atómicamente y pushea. Judgment calls permanecen en el gestor.

---

## SECTOR A — Estado actual (ya implementado, NO duplicar)

| Pieza RFC 19 | Estado |
|---|---|
| State machine formal (§6.1, 8 fases + transiciones) | ✅ `supervisor/{types, runner}.rs` — `tick()` FSM pura |
| DoomLoopDetector (§6.2 anti-infinite-loop) | ✅ `supervisor/doom_loop.rs` + recovery budget (RFC 19 §6) |
| Heartbeats (§6.3 stall detection) | ✅ `SupervisorEvent::Heartbeat` — stall kicks + recovery on delta |
| Checkpoints SQLite (§5) | ✅ `Journal::save_checkpoint`/`checkpoint_tail` + test RFC 19 §6.1 round-trip |
| Reanudación exacta (§6.4) | ⚠️ PARCIAL — `agent_resume` (swarm 4.3) une checkpoint; falta el mission-level `SupervisorState` reconstruido desde el journal |
| Observer web del Journal (§6.5) | ⚠️ PARCIAL — tail routes `/tail/*` existen; falta el view de inspección completa (payload + paginación) |

## SECTOR B — Plan refinado Phase 6 (3 sub-fases atómicas)

### Sub-fase 6.0 — Reanudación desde cualquier estado
- `supervisor/resume.rs`: `resume_state(journal, mission_id) -> Result<(SupervisorState, MissionCheckpoint)>` — lee el último checkpoint de la mission, parsea el phase tag → reconstruye el `SupervisorState` (budget caps + mode persistidos o default si faltan) → el FSM continúa desde ahí con `tick()`.
- Integración: `atlas resume` CLI ya existe — extiéndelo para reconstruir el estado del supervisor (si el wiring es trivial; si complica, función exportada + documentado).
- Tests: resume desde cada fase (planning/executing/verifying/recovering/halted), mission sin checkpoints → default fresh state, round-trip persist→resume→persist.

### Sub-fase 6.1 — Observer web del Journal
- axum: `GET /hud/journal?limit=N&offset=M&kind=K` — inspección completa con paginación + payload JSON íntegro (el tail actual trunca payloads — mira `tail.rs` y añade el route con payload completo).
- Svelte: `<JournalObserver.svelte>` (patrón SwarmConsole/GraphView): tabla/lista de entradas con kind/ts/payload expandible + paginación + filtro por kind. Rest helper `fetchJournalPage` en `hud.ts` (patrón `fetchSwarmChecks`).
- Tests: Rust route happy-path + paginación + filtro + 400 en params inválidos; vitest store helper round-trip + failure-path; componente compile test (patrón SwarmConsole.test.ts).

### Sub-fase 6.2 — Cierre de Fase 6
- RFC 20 marker COMPLETA + RFC 19 §6.4/§6.5 markers IMPLEMENTED + RFC 26 índice + README status.

**Entregable:** sistema durable, recuperable de fallos sin scripts externos. KPI: re-ingresos de model crash sin perder trabajo 100% (RFC 20).

---

## SECTOR C — Fuera de alcance (esta iteración)

- Anti-bucles adicionales (doom_loop ya cubre RFC 19 §6.2; G6 circuit-breaker half-open es del orchestrator, 2.5+).
- Observer web remoto (SSO/OIDC — Phase 8).
- Compaction LLM-driven en el observer (follow-up 5.3+).

## SECTOR D — Siguiente paso operativo

1. Commit de este plan + RFC 20 update.
2. Sub-fase 6.0 (delegada) → revisar → commit → push.
3. Sub-fase 6.1 (delegada) → revisar → commit → push.
4. Sub-fase 6.2 (gestor) → cierre.
