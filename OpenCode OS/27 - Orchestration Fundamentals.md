# 27 - Orchestration Fundamentals

> Audit comparativo de 4 orquestadores de agentes de IA de 2025-2026 — `tmux-orchestrator-ai-code`, `orca`, `herdr`, `traycer` — con AionUI como referencia estructural. Extrae **6 principios fundacionales** que se repiten entre todos, mapea qué tanto OpenCode OS cumple cada uno, y lista **8 brechas concretas** contra los fundamentos. Las brechas son el input de Phase 2+; este RFC es el spec para dirigir trabajo futuro en lugar de abrir frentes ad-hoc.

---

## 1. Contexto y motivación

OpenCode OS funde 27 RFCs en una specificación densa, pero la inspiración inicial fue mayormente AionUI (`22 §3`) y Hermes. El campo se ha movido rápidamente entre el lanzamiento de la primera especificación (`00 Vision`, junio 2025) y ahora (Q3 2026): orquestadores de agentes pasaron de ser experimentos pequeños a productos comerciales estabilizados con 20k+★ y miles de commits. Antes de cerrar Phase 2 (LLM-driven), hacemos un audit comparativo para:

1. **Validar las decisiones ya tomadas** (Tauri vs Electron, SQLite + Journal, axiom WS que sobrevive webview crashes, spec-first).
2. **Identificar principios fundacionales** que no hemos articulado explícitamente pero que subyacen a las decisiones del Phase 1.
3. **Catalogar brechas** específicas donde OpenCode OS está detrás del estado del arte y priorizarlas para Phase 2+.

Las fuentes auditadas son los 4 orquestadores más representativos de la categoría:

| Orquestador | Repo | Stars | Commit count | Lenguaje | Filosofía |
|---|---|---|---|---|---|
| `tmux-orchestrator-ai-code` | `bufanoc/tmux-orchestrator-ai-code` | 3 | ~10 | Python + shell | Minimalismo, masturbación intelectual con tmux |
| `orca` | `stablyai/orca` | 27.4k | 7144 | Electron + React | ADE comercial-grade, mobile companion, worktrees |
| `herdr` | `ogulcancelik/herdr` | 20k | 1203 | Rust puro | Multiplexer tipo tmux re-imaginado para agentes, **un binario sin Electron** |
| `traycer` | `traycerai/traycer` | 738 | 406 | Electron + Bun/TS | Spec-first, agent-to-agent comms, model hot-swap |

AionUI queda fuera del auditable (era la fuente original) pero se invoca cuando un principio aparece ahí también.

---

## 2. Los 6 principios fundacionales

Comparando los 4, hay 6 invariantes que se repiten. No son features, son propiedades estructurales que cualquier orquestador maduro exhibe.

### §2.1 Aislamiento por worktree/pane, no por proceso

**Citado en:** orca (git worktrees), herdr (panes), tmux-orchestrator (windows tmux).

Cada agente tiene su propio FS o pane separado. No comparten estado de archivos en tiempo de ejecución. La unidad de aislamiento NO es el proceso — es el worktree (físico) o el pane (terminal). Sin esta separación, dos agentes convergen sobre el mismo archivo y se pisan mutuamente.

- orca: "fan one prompt across five agents, each in its own isolated git worktree — compare the results and merge the winner".
- herdr: pane por agent, registry persistente.
- tmux-orchestrator: window tmux por agente (PM/Engineer/Orchestrator).

**Implicación:** OpenCode OS RFC 05 (Swarm) define subagentes virtuales con roles, pero no especifica worktrees físicos por subagente. **Esta es una brecha estructural (§3.A).**

### §2.2 Estado visible at-a-glance

**Citado en:** herdr (3 colores), orca (sidebar con sentinels), traycer (workspace panels).

El operador siempre sabe en segundos quién está `blocked / working / done` sin escarbar logs. El estado visible NO es un dashboard_post_mortem — es una sola ojeada con códigos de color. herdr en particular se enorgullece de "every agent at a glance" como core philosophy.

- herdr: 4 colores (idle/working/blocked/done).
- orca: cada agente tiene status pill.
- traycer: workspace con cards por Task.

**Implicación:** OpenCode OS HUD (RFC 24) tiene tail boxes por artefacto (verdicts, plans, diffs, etc.), pero NO tiene códigos de color por **step state**. La granularidad es mission-level, no step-level. **Brecha G (§3).**

### §2.3 Remote attach / persistence detach

**Citado en:** herdr (`ctrl+b q detach → agentes siguen`, reattach desde SSH), orca (SSH worktrees + mobile companion).

El agente runtime sobrevive a la desconexión del client. Reattach desde cualquier máquina u OTA. La separación proceso↔TTY es fundacional — sin ella, cierres de laptop matan jobs de 6 horas.

- herdr: SSH reattach is just `herdr` from another terminal.
- orca: `orca serve` en headless Linux box, attach desde desktop/mobile.

**Implicación:** OpenCode OS complies — el servidor axum WS sobrevive webview crashes (`25 §3.2`), y el motor Rust no reside en el webview. Esta propiedad está satisfecha por la arquitectura Tauri 2.

### §2.4 Reflexividad — el orquestador puede ser orchestrated

**Citado en:** orca CLI (`orca worktree create / snapshot / click / fill`), herdr socket API (`.pi` skills que drivean herdr), traycer agent-to-agent comms.

El orquestador expone una API que los propios agentes pueden invocar. Phase 2 (LLM-driven) descansa sobre esto: si el LLM no puede drivear el orquestador, no hay verdadero agentic behaviour.

- orca CLI: programabilidad completa.
- herdr: Unix socket API + agent skill doc.
- traycer: agent-to-agent delegate/review/negotiate.

**Implicación:** OpenCode OS RFC 03 §3.5 IPC existe para Tauri webview calls, pero NO hay una API CLI que el LLM pueda invocar desde dentro de un step. `opencode spawn-step` y similares no existen. **Brecha F (§3).**

### §2.5 BYOA — Bring Your Own Agent

**Citado en:** orca, herdr, traycer — los tres aceptan 20+ agentes CLI.

El orquestador no te fideliza a un modelo. Acepta tu suscripción existente (Codex, Claude Code, Cursor, OpenCode, etc.) y te deja mezclar. Sin esto, la adopción es un cliff: el operador abandona su stack para entrar al nuevo.

- orca: "any CLI agent — if it runs in a terminal, it runs in Orca".
- herdr: detección automática de 15+ agentes.
- traycer: BYOA + native inference opcional como default.

**Implicación:** OpenCode OS RFC 04 Model Orchestrator soporta 13 providers HTTP. Pero Phase 1 no ha expuesto un endpoint "trae tu propio agente CLI" — un wrapper que delege al agente CLI del operador en lugar de llamar a un LLM HTTP. **Brecha: implícita en la arquitectura, no documentada explícitamente.** Aceptable para Phase 1, debe explicitarse en Phase 2.

### §2.6 Spec-first — sin spec, no hay mission

**Citado en:** traycer (spec-first development), tmux-orchestrator (project_spec.md con PROJECT/GOAL/CONSTRAINTS/DELIVERABLES/SUCCESS_CRITERIA), orca (project_spec).

Cada mission parte de un documento estructurado. Sin spec, el agente deriva. Los specs vienen de prompts raw pero se cristalizan antes de planear.

- tmux-orchestrator: PROJECT/GOAL/CONSTRAINTS/DELIVERABLES/SUCCESS_CRITERIA.
- traycer: "Traycer brings spec-first development to AI coding workflows".
- orca: project_spec.md antes de crear worktree.

**Implicación:** OpenCode OS RFC 23 (Prompt Understanding & Refinement) parsea el prompt y produce `MissionConsolidated` con `mission_statement`, `deliverables`, `constraints`. **Esta propiedad está satisfecha.** La único a mejorar es exponer el spec generado en un documento legible (no solo un row JSON en el Journal) — mejora de UX, no estructural.

---

## 3. Las 8 brechas contra los fundamentos

Las 8 brechas concretas que el audit identifica. Cada una está entrada como `(prioridad, esfuerzo estimado, principal受益ario)`. Las primeras 4 son estructurales; las últimas 4 son de superficie o documentación.

### §3.A — Worktrees físicos por subagente **(P0, M, orquestación)**

**Patrón:** §2.1 aislamiento.
**Status:** RFC 05 define subagentes virtuales. `src-tauri/src/skills/manifest.rs` y `core/swarm.rs` (no existe) no aterrizan en FS separado.

**Propuesta:** Añadir `core/worktree.rs` con `Worktree::spawn(mission_id, step_id) -> PathBuf` que crea `~/.opencode/{profile}/worktrees/{mission_id}/{step_id}/`. El Coding Engine escribe ahí. El Supervisor (RFC 19) borra/consolida al Done. Esto desbloquea el patrón orca "5 agents → 5 worktrees → merge winner", clave para evaluar múltiples estrategias en paralelo.

### §3.B — Hot-swap de modelo in-place **(P0, S, Model Orchestrator)**

**Patrón:** traycer unified context (model switch sin perder context window).
**Status:** RFC 04 fija provider por profile. Una Mission no puede cambiar de provider en medio sin perder el prompt elaborado.

**Propuesta:** Añadir `ModelOrchestrator::swap_model(mission_id, new_model) -> Result<()>` que serialice el PromptPayload actual y lo re-instantiate con el nuevo provider. Cero pérdida de contexto. Esto es estructuralmente factible porque `PromptPayload` ya es `Serialize + Deserialize`. La recompensa: el operador puede empezar con un modelo barato (Haiku/Ice) y escalar a Sonnet/Opus solo para pasos críticos.

### §3.C — Mobile companion HUD **(P2, M, HUD)**

**Patrón:** §2.3 remote attach. orca mobile companion.
**Status:** RFC 25 §3.2 lo menciona en roadmap lejano. Sin diseño.

**Propuesta:** PWA envoltura sobre el mismo HUD WS server. El axum server ya escucha en `127.0.0.1:0` — para mobile, exponer طریق SSH tunnel o Tailscale (no abrir a 0.0.0.0). El HUD SvelteKit es responsive-enough para mobile. Phased: Phase 2.2 añadir view narrowing (`?view=kanban` etc.), Phase 2.3 publish como PWA.

### §3.D — Agent-to-agent peer comms **(P1, M, Swarm)**

**Patrón:** §2.4 reflexividad. traycer agent-to-agent delegate/review/negotiate. herdr socket API peer-to-peer.

**Status:** RFC 05 (Swarm) define jerarquía Planner → [Researcher, Architect, Backend, etc.], no peer-to-peer. No hay mailbox.

**Propuesta:** Añadir `core/swarm_mailbox.rs` con `AgentMailbox` que persist (SQLite table `agent_messages`, M10) mensajes entre agentes. Cada subagente tiene un `agent_id`. Commands: `delegate(target_agent_id, task)`, `ask_for_review(target_agent_id, diff_id)`, `negotiate(target_agent_id, position)`. Capacidad matrix documenta qué may/what may not (ver traycer).

### §3.E — Annotate-diff UI **(P1, S, HUD)**

**Patrón:** orca "annotate AI diffs" + traycer space comentarios.
**Status:** RFC 24 §6 "card actions" menciona el concepto pero no está implementado. HUD Phase 1 solo muestra las colas, no permite comentarios.

**Propuesta:** Añadir `hud/annotate.rs` con endpoints `POST /diff/{id}/annotation` y `GET /diff/{id}/annotations`. Persist en nueva tabla `diff_annotations` (M11) con `id, diff_id, line_no, body, author, ts`. El HUD renderiza inline en el drawer del diff. El siguiente `run` o `resume` recoge las annotations como contexto adicional del Coding Engine.

### §3.F — API CLI auto-programable **(P0, S, CLI)**

**Patrón:** §2.4 orca CLI + herdr socket API.

**Status:** `opencode mission new / run / resume / fork / steer` son operator-facing. No hay `opencode spawn-step`, `opencode wait-step`, `opencode read-tail`, etc. para que un LLM-driven Phase 2 pueda drivear el orquestador.

**Propuesta:** Añadir sub-comandos del namespace `opencode exec` que el LLM puede invocar desde un step. Subset inicial: `opencode exec step <plan_id> <step_id>` (run único), `opencode exec wait <diff_id>` (block hasta report), `opencode exec tail <kind> <N>` (readJournal tail), `opencode exec publish <kind> <payload>` (emite BusEvent). Sin necesidad de orchestration loop — el LLM lo hace. Documentar en `08 CLI` §A.

### §3.G — State-color at-a-glance en HUD **(P0, S, HUD)**

**Patrón:** §2.2 visibilidad. herdr 3-4 colores por agente.

**Status:** HUD Phase 1 tiene tail boxes por artefacto. No tiene códigos de color por step state. La Mission card muestra rojo/verde al final, no por estado vivo.

**Propuesta:** Añadir `StepPhase` enum análogo a `MissionPhase` (RFC 19): `Pending, Blocked, Working, Done, Halted`. HUD lo muestra con una pill por step. Mission-card mira el agregado: si todos `Done`→verde, si alguno `Blocked`→rojo, si todos `Pending`→gris, si mezcla→amarillo polling.

### §3.H — Spec document legible **(P2, S, Prompt Understanding)**

**Patrón:** §2.6 spec-first. traycer + orca + tmux-orchestrator materializan specs en docs humanos.

**Status:** RFC 23 produce `MissionConsolidated` con `mission_statement`, `deliverables`, `constraints` — pero solo vive como JSON payload en `mission_consolidated` table.

**Propuesta:** Añadir `opencode spec show <mission_id>` que formatea el JSON como markdown human-legible y lo imprime / escribe a `~/.opencode/{profile}/specs/{mission_id}.md`. Esto es post-Prompt, pre-Planning. Permite al operador editar el spec a mano antes de planear. Empata con `opencode plan` ya existente.

---

## 4. Cross-references entre brechas y RFCs existentes

| Brecha | RFC principal | RFCs colaterales | Migración schema |
|---|---|---|---|
| A (worktrees) | 05 Swarm, 19 Supervisor | 13 Coding, 25 Stack | (FS only, no schema) |
| B (model hot-swap) | 04 Model Orch | 23 Prompt Under | (no schema, registry edit) |
| C (mobile HUD) | 24 HUD | 25 Stack | (no schema) |
| D (agent mailbox) | 05 Swarm | 19 Supervisor | M10 `agent_messages` |
| E (annotate-diff) | 24 HUD | 13 Coding, 19 Super | M11 `diff_annotations` |
| F (CLI exec) | 08 CLI | 19 Supervisor, 25 §3.9 | (no schema) |
| G (step colors) | 24 HUD | 19 Supervisor | M12 `step_states` (optional cache) |
| H (spec show) | 23 Prompt Under | 08 CLI | (no schema, format-only) |

---

## 5. Priorización por dependencia

```
B (hot-swap)   ─┐
F (CLI exec)   ─┤
G (step color) ─┼─► Phase 2.1 (LLM-driven básico, unos 2-3 sprints)
A (worktrees)  ─┘

D (mailbox)    ─┐
E (annotate)   ─┼─► Phase 2.2 (multi-agent + HUD interactivity, ~2 sprints)
H (spec show)  ─┘

C (mobile HUD) ──► Phase 3 (mobile experience, ~2 sprints)
```

**Orden recomendado de implementación (por return/effort):**
1. B (hot-swap): Effort S, desbloquea Phase 2 LLM-driven. Beneficia a todo operador.
2. G (step colors): Effort S, mejora visibilidad. Complementa B en feedback loop.
3. F (CLI exec): Effort S, base de Phase 2 agent-driven. Sub-comandos pequeños.
4. A (worktrees): Effort M, abre camino a parallel strategies (orca pattern).
5. E (annotate-diff): Effort S, mejora review workflow.
6. D (mailbox): Effort M, paso a multi-agent communication real.
7. H (spec show): Effort S, quick win UX.
8. C (mobile HUD): Effort M, deferred hasta post Phase 2.

---

## 6. Principios no cubiertos por las brechas — ya cumplidos

Para evitar double-counting y dejar claro que algunos fundamentos ya están implementados, este § los listo:

| Principio | Implementado en | Cumple |
|---|---|---|
| §2.3 Remote attach | `25 §3.2` axum WS server survives webview crashes | ✓ |
| §2.5 BYOA | `04` Model Orchestrator 13 providers (HTTP) | ✓ (falta CLI external-agent) |
| §2.6 Spec-first | `23` Prompt Under. Produce `MissionConsolidated` | ✓ (falta doc legible, brecha H) |
| Journal durability | `02 §3.1.2` at-least-once idempotente | ✓ |
| Sandbox security | `18` niveles Sandbox, SHA-256 firmas skills | ✓ |

---

## 7. Status de este RFC

- **Versión:** 1.0 (audit completo, Q3 2026).
- **Tipo:** Informativo + priorización. No introduce APIs nuevas inmediatamente.
- **Cambio relacionado:** Phase 1 ( thermo-nuclear cleanup commit `3f84151`) sienta la base technique sobre la que las brechas se implementarán.
- **Cierre de brechas:** Cada brecha se cierra con un PR etiquetado `feat(orch-RFC27-§X): ...`. Quando las 8 estén cerradas, este RFC pasa a status: **implemented** y se mueve al apéndice histórico.

---

## 8. Fuentes de auditoría

READMEs oficiales consultados vía webfetch (Jul 2026):

- https://github.com/bufanoc/tmux-orchestrator-ai-code (Python, 10 commits, MIT).
- https://github.com/stablyai/orca (Electron+React, 7144 commits, MIT, 27.4k★).
- https://github.com/ogulcancelik/herdr (Rust, 1203 commits, Apache-2.0, 20k★).
- https://github.com/traycerai/traycer (Electron+Bun/TS, 406 commits, MIT, 738★).
- https://github.com/iOfficeAI/AionUi (referencia estructural histórica, Electron+React).

OpenCode OS RFC cross-references:

- §2.1 → 05 Swarm, 19 Supervisor
- §2.2 → 24 HUD
- §2.3 → 25 Stack §3.2
- §2.4 → 08 CLI, 03 Engine Arch
- §2.5 → 04 Model Orch
- §2.6 → 23 Prompt Under

---

> Apego a la directriz AGENTS.md §6 «No adding módulos nuevos de monolithic features. New engines go in their own RFC first». Este RFC introduce 8 frentes como direcciones, no como code. Cada frente se implementa con su propio sub-RFC o PR etiquetado según se priorice.
