# 52 — Phase 23: Proactive Turn Engine (v3.1.2)

- **Estado:** plan aprobado; v3.1.2.0–v3.1.2.2 **COMPLETAS** (falta v3.1.2.3).
- **Fecha:** 2026-10-03.
- **Motivación:** cerrar el único ítem de producto bloqueado del Roadmap v3 — el
  "turno proactivo" que `RFC 28 §G.4 item 8` asumía (`Planning::next_free_slot`)
  y que el repo **no tiene** (`research/50` §Estado: bloqueo de infraestructura).
  Ahora es **medible**: la Fase 22 EVAL (`research/51`) provee golden suite +
  métricas + gate, así que construir el motor deja de ser fe ciega.

## 1. Qué es un "turno proactivo"

Hoy Atlas solo trabaja cuando el operador lo pide. El turno proactivo es la
capacidad de **iniciar/continuar trabajo cuando el operador está libre**, usando
el calendario como señal de disponibilidad:

- **Fuente de disponibilidad:** las *busy windows* (`calendar_busy_windows`,
  M18) que ya pueblan Graph (poll), ICS (subscriptions + poller) y `manual`.
- **Decisión:** dado `now`, una estimación de duración (`eta_ms`) y las ventanas
  ocupadas por encima de un umbral de peso, decidir **RunNow / WaitUntil(ts) /
  Blocked**.
- **Disparo:** el Execution Supervisor (RFC 19, ya puro) emite un
  `SupervisorAction::EnqueueProactiveTurn` cuando hay trabajo pendiente y la
  disponibilidad lo permite.

Evidencia externa que respalda el enfoque (research/51 §1): *bounds, not retries*
(Laursen) — el turno proactivo debe llevar max-attempts/spend/revert; y el
principio de harness engineering de que la conducta proactiva necesita una
**métrica** que la valide (por eso la Fase 22 va primero).

## 2. Diseño

### 2.1 Núcleo puro (v3.1.2.0) — `planning::availability`

```rust
pub struct TurnPolicy { pub eta_ms: i64, pub weight_threshold: f64, pub horizon_ms: i64 }

pub enum Availability { RunNow, WaitUntil(i64), Blocked }

pub fn next_free_slot(busy: &[BusyWindowRow], now_ms: i64, policy: &TurnPolicy) -> Availability
pub fn availability_now(journal: &Journal, now_ms: i64, policy: &TurnPolicy) -> Result<Availability>
```

Algoritmo: filtrar ventanas con `weight >= weight_threshold` y `ends_at > now`;
recorrer intervalos en `[cursor, cursor+eta)` empujando `cursor = max(cursor,
ends_at)` hasta punto fijo (half-open, igual que `BusyWindowQueue::overlapping`);
`Blocked` si el slot libre excede `horizon_ms`.

Default conservador: `eta_ms = 15 min`, `weight_threshold = 1.0` (solo *hard
busy* bloquea; el *soft busy* 0.5 no), `horizon_ms = 24 h`.

### 2.2 Disparo del supervisor (v3.1.2.1)

- `AppState` expone `context_availability(policy)` (envuelve
  `availability_now`) — el `context_window` que faltaba.
- `supervisor::tick` gana `SupervisorEvent::ProactiveCheck` y, cuando
  `Availability::RunNow` y hay misión pendiente, emite
  `SupervisorAction::EnqueueProactiveTurn { mission_id }`. Respeta `BudgetCaps`
  (max-attempts/spend/revert) ya existentes (RFC 19 §6).

### 2.3 Política + CLI + HUD (v3.1.2.2)

- `TurnPolicy` persistida por perfil (defaults + override por env/CLI).
- HUD: indicador de disponibilidad en `<EvalCard>`/nueva card; endpoint
  `GET /hud/availability`.
- CLI `atlas calendar availability [-e eta_ms] [-w weight]`.

### 2.4 Medición (v3.1.2.3)

- Golden task `planning.availability` (determinista) que fija
  `free → RunNow`, `covering now → WaitUntil(end)`.
- Métricas de turnos proactivos (arrancados/saltados por busy) en el store EVAL.

## 3. Sub-fases atómicas (un commit por sub-fase)

- **v3.1.2.0 — Núcleo de disponibilidad** (`planning/availability.rs` +
  `TurnPolicy`/`Availability`/`next_free_slot`/`availability_now` + 7 tests +
  golden task `planning.availability` + CLI `atlas calendar availability`). **COMPLETA.**
- **v3.1.2.1 — Disparo del supervisor** — **COMPLETA**:
  - `SupervisorEvent::ProactiveCheck { availability, pending_mission }` +
    `SupervisorAction::EnqueueProactiveTurn { mission_id }` (el host aporta
    disponibilidad + misión pendiente; el FSM sigue puro).
  - `tick` inicia turno solo si `phase.accepts_new_mission()` y
    `availability == RunNow`; respeta `BudgetCaps` (evaluadas al inicio del tick).
  - `AppState::context_availability(policy)` — el `context_window` que faltaba.
  - 4 tests (libre/con misión, busy window, supervisor ocupado, sin backlog).
- **v3.1.2.2 — Política + CLI + HUD** — **COMPLETA**:
  - Persistencia: migración **M37 (schema v36)** tabla `proactive_policy` (fila
    única) + `journal/proactive.rs` (`load/save_proactive_policy`,
    `next_pending_mission` = misión más antigua en `received`).
  - **Host caller** `supervisor/host.rs`: `proactive_check` / `proactive_check_at`
    (disponibilidad + backlog → `tick(ProactiveCheck)`), puro y testeable.
  - CLI `atlas calendar policy [--eta-ms] [--weight] [--horizon-ms]
    [--enable|--disable]` y `atlas calendar proactive`.
  - HUD `GET /hud/availability` (política + disponibilidad + misión pendiente).
  - 5 tests (journal ×2, host ×2, handler HUD). Card Svelte diferida.
- **v3.1.2.3 — Medición** (métricas de turnos proactivos en EVAL).

## 4. KPI

- Decisión de disponibilidad **determinista y testeada** (0 falsos RunNow sobre
  ventana *hard busy*).
- Turnos proactivos medidos en `eval_cases` (arrancados / esperados / bloqueados).

## 5. No-goals / riesgos

- **No** lanzar trabajo sin `BudgetCaps` (max-attempts/spend/revert) — bounds, no
  retries infinitos (Laursen; RFC 19 §6).
- **No** decidir por LLM: la disponibilidad es aritmética de intervalos.
- Riesgo: umbral/política mal calibrados → turnos molestos; mitigado exponiendo
  `weight_threshold` y midiendo con EVAL.

## 6. Referencias

- `Atlas OS/28 - External Tool Integration.md` §G.4 item 8 (`Planning::next_free_slot`).
- `Atlas OS/19 - Execution Supervisor.md` (state machine, caps, doom_loop).
- `Atlas OS/12 - Planning Engine.md`.
- `Atlas OS/research/50` (§Estado: v3.1.2 bloqueado) y `research/51` (EVAL).
- Laursen, 18 Months of Production AI Agents (bounds-not-retries).
