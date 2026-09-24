# 31 — Phase 4 Swarm (plan refinado)

**Propósito:** Refinar la Fase 4 RFC 20 ("Swarm") con la evidencia del audit RFC 30 (agency-agents + munder-difflin) y el research previo (`28 - conductor & alt surfaces.md`, patrones CN-001/CN-003/CN-004). Output: sub-fases atómicas commiteables (commits no PRs), feature-gated default-off cuando toque crates nuevas (RFC 25 §11).

---

## SECTOR A — Decisiones ya tomadas (evidencia RFC 30 + RFC 22)

### A.1 Roles con personalidad (agency-agents, 154k★, MIT)

Cada rol del Swarm lleva `personality` / `processes` / `deliverables` — presets de roles instalables que el operador puede elegir. El catálogo bundled (brecha D) ya sienta el patrón de manifest RFC 06 §1 completo; los roles lo extienden con identidad.

### A.2 Office floor 2D + mailbox + memoria por agente (munder-difflin)

"Each agent gets long-term memory, a mailbox, and a desk on a 2D office floor — your clone routes work between them while you watch." El floor es la visualización divertida del Swarm Console; la mailbox es el mecanismo de coordinación (mensajes entre agentes, no solo broadcast del Kernel Bus).

### A.3 Worktree isolation + auto-rebase (Conductor CN-001/CN-003/CN-004)

RFC 22 `28 - conductor & alt surfaces.md` Sector A.4: worktree Git por agente (aislamiento), rebase automático post-merge en workspaces vivos (CN-003), checks button por worktree en HUD (CN-004). Worktrees via **git CLI** (`std::process::Command` — mismo patrón que el supervisor usa para `git reset --hard`) — **sin crate nueva** (git2 NO se añade; audit RFC 25 §11).

### A.4 Paralelismo por rol (pool swarm estilo Kimi)

10 agentes cooperan en una mission: Planner/Researcher/Architect no tocan código; Backend/Frontend/DB/Security/Testing ejecutan en paralelo; Reviewer/Merger consolidan. Tokio tasks + locks de archivos (RFC 13 file locks).

---

## SECTOR B — Plan refinado Phase 4 (6 sub-fases atómicas)

### Sub-fase 4.0 — Foundation (Roles + registry M29 + worktree manager)

- M29 migration: SQLite `swarm_agents` (id, mission_id, role, model_id, state, personality JSON, created_at) + `agent_mailbox` (id, from_agent, to_agent, body JSON, read_at, created_at).
- `enum Role { Planner, Researcher, Architect, Backend, Frontend, Database, Security, Testing, Reviewer, Merger }` en `swarm/roles.rs` — RFC 05 §1, as_str/parse round-trip estilo ConsensusDimension.
- `WorktreeManager` en `swarm/worktrees.rs` — git CLI (`git worktree add/remove/list`), paths bajo `~/.opencode/worktrees/<mission>/<agent>/`, fail-safe si git no está.
- Tests: migration idempotente, roles round-trip, worktree add/list/remove happy-path (git real en test si disponible, skip si no) + failure-path.

### Sub-fase 4.1 — Role presets (agency-agents port)

- `swarm/presets.rs`: definiciones de rol con personality/processes/deliverables (port del patrón agency-agents) — presets: `atlas-team` (10 roles completos), `pair-programming` (2), `solo-plus` (1 + reviewer).
- CLI: `atlas swarm presets` / `atlas swarm start --preset <id> --mission <id>`.
- Tests: presets parsean, asignación de modelo por rol (architect→strong, editor→weak, Aider tri-model del Registry).

### Sub-fase 4.2 — Pool swarm (paralelismo por rol)

- `swarm/pool.rs`: despacho paralelo por rol — tokio tasks por agente, semáforo por provider (reusa `backpressure::BackPressure` del orchestrator), locks de archivos (RFC 13) antes de tocar código.
- SwarmRunner: orquesta el ciclo planner→research/architect→executors paralelos→reviewer→merger (RFC 05 topología), con checkpoints del Execution Supervisor (RFC 19) por agente.
- Tests: pool con N agentes stub, locks evitan colisión, checkpoint por agente.

### Sub-fase 4.3 — Mailbox + memoria por agente (munder-difflin pattern)

- `Journal::send_message` / `inbox_for(agent)` / `mark_read` (M29) — coordinación entre agentes.
- Memoria por agente: cada agente hereda el Journal (mismo checkpoint system RFC 19) — reanudación exacta.
- CLI: `atlas swarm send <from> <to> <body>` / `atlas swarm inbox <agent>`.
- Tests: send/inbox round-trip, mark_read idempotente.

### Sub-fase 4.4 — Auto-rebase post-merge (CN-003 port)

- `swarm/rebase.rs`: tras merger acepta el merge, rebase automático en workspaces vivos (`git fetch + rebase` via CLI), fail-safe a manual si hay conflictos.
- Tests: rebase clean happy-path (git real, skip si no), conflicto → fail-safe.

### Sub-fase 4.5 — Swarm Console HUD (frontend, Phase 8 parcial)

- `<SwarmConsole.svelte>`: floor 2D (munder-difflin) + desks por agente con state + mailbox drawer + checks button por worktree (CN-004).
- WS events: `swarm_agent_spawned`/`swarm_message`/`swarm_state_changed` (Kernel Bus broadcast existente).
- HUD routes + cards siguiendo el patrón de las cards existentes (RFC 24).
- Tests: vitest store + card render.

**Entregable:** 10 agentes cooperan en una mission. KPI: latencia end-to-end UI <100ms (RFC 20); re-ingresos sin perder trabajo 100%.

---

## SECTOR C — Fuera de alcance (esta iteración)

- git2/libgit2 como crate (git CLI es suficiente; audit single-binary RFC 25 §11).
- Multi-PC distribuido (Phase 8 — remote-live dual-PC, research `28` Sector C).
- VRAM/RAM monitor en el Swarm Console (Phase 8).

## SECTOR D — Siguiente paso operativo

1. Commit de este plan + RFC 20 update.
2. Empezar sub-fase 4.0 (Foundation) — M29 + roles + worktree manager.
3. Sub-fases 4.1-4.4 siguen; 4.5 (frontend) cierra.
