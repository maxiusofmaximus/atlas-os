# 50 - Roadmap v3 (plan)

Plan atómico del Roadmap v3 de Atlas OS — post-Roadmap v2 (agotado). Decisión del
operador (2026-10-03): prioridad elegida por el gestor tras comparar los cuatro
candidatos: **D — Windows Calendar real**. Phase 13 (Laya) queda como
`research/49` (propuesta sin implementar).

## SECTOR A — Decisiones y evidencia ya tomada

### A.1 Windows Calendar vía RFC 28 §G — NO WinRT `AppointmentManager`
- G.1: `AppointmentManager` requiere la capability restringida
  `appointmentsSystem` → `E_ACCESSDENIED` en desktop no-MSIX; viola RFC 25 §11
  (single-binary, sin capabilities restringidas). Rechazado.
- Dirección elegida: **WRITE** `.ics` (RFC 5545, `METHOD:PUBLISH`) servido por el
  HUD axum + **READ** Microsoft Graph `/me/calendarView` (`Calendars.Read` +
  `offline_access`), refresh token cifrado (AES-256-GCM) en SQLite.

### A.2 Terreno ya construido (verificado en repo)
- `src-tauri/src/calendar/payload.rs`: `BusySource`, `BusyWindow::overlaps`,
  `IcsMissionStatus`, `IcsMission::{ical_uid,is_all_day}` ya existen.
- `src-tauri/src/calendar/queue.rs`: `BusyWindowQueue` (`upsert`,
  `insert_manual`, `delete`, `delete_by_source`, `get`, `overlapping`,
  `list_by_source`, `list`, `count`) ya existen.
- Schema m18 (`calendar_busy_windows` + `calendar_auth`) ya migrado
  (`journal/schema.rs`).
- `calendar` ya se consume en `lib.rs`, `hud/server.rs`,
  `journal/export/retention.rs`, `toast/payload.rs`.
- **ICS WRITE YA IMPLEMENTADO (verificado 2026-10-03):** `calendar/ics_writer.rs`
  (`CalendarWriter::render`) + `calendar/ics_route.rs` (`get_calendar_ics`)
  cableado en `hud/server.rs:115` (`GET /atlas-calendar.ics`); feature
  `calendar-ics`; `cargo test --features calendar-ics --lib calendar` → **42 ok**.
- **Gap real (revisado):** `calendar/auth.rs` y `calendar/graph_reader.rs` son
  **placeholders** (feature `calendar-graph`: compilan vacíos, sin lógica),
  `AppState.context_busy_windows` **no existe**, no hay CLI `atlas calendar`, y
  el endpoint `.ics` **valida** el token opaco del §G.2 (implementado 2026-10-03, `calendar/token.rs`).

### A.3 Descarte de candidatos (evidencia)
- **A (Axum 0.8):** higiene de deps; 25 rutas en `hud/server.rs`; sin feature
  visible → diferido.
- **B (Swarm HUD):** YA implementado — `src/lib/components/SwarmConsole.svelte`
  (566 LOC, wired en `+page.svelte:388`, RFC 31 §B.4.5) → no es candidato v3.
- **C (context-mode MCP nativo):** infra MCP existe (`cli/commands/mcp.rs`,
  `mobile/mcp_template.rs`, `classifier/mcp_filter.rs`) pero falta definición de
  alcance → diferido hasta un RFC concreto.

## SECTOR B — Plan v3 (Fase v3.1: Windows Calendar real)

### Sub-fase v3.1.0 — ICS WRITE (✅ YA IMPLEMENTADO)

- `calendar/ics_writer.rs` + `calendar/ics_route.rs` + ruta `hud/server.rs:115`.
- **Token opaco ✅** (2026-10-03): `calendar/token.rs` genera `base64url(16 bytes)`
  (vía `Uuid::as_bytes`, sin crate nueva), lo persiste en
  `<root>/calendar_ics_token.txt` y la ruta exige `?token=…` (401 si falta o es
  erróneo); `atlas calendar feed` emite la URL con `?token=`. 4 tests.

### Sub-fase v3.1.1 — Graph READ (implementar desde los stubs)

- `calendar/auth.rs`: OAuth `Calendars.Read` + `offline_access`; refresh token
  cifrado AES-256-GCM en `calendar_auth` (m18).
- `calendar/graph_reader.rs`: `CalendarReader::{new,authenticate,poll_once,spawn}`
  (API proyectada en el stub); poll 60 s `/me/calendarView` →
  `BusyWindowQueue::upsert` con `source=Graph`.
- Feature `calendar-graph` ya declara `graph-rs-sdk`/`aes-gcm`/`ring` y compila
  (`cargo check --features calendar-graph` OK).
- Tests: token cipher round-trip, expirado → silent refresh, fallo → Toast re-login.

### Sub-fase v3.1.2 — Planning wiring
- `AppState.context_busy_windows` (RFC 12 §3) recibe los `BusyWindow` de Graph;
  `overlaps(turn_eta)` decide si encolar el turn proactivo.
- Threshold: >5 eventos en la próxima hora → `busy` con `weight=1.0`.
- Tests: `overlaps` boundary, threshold, precedencia de `source`.

### Sub-fase v3.1.3 — CLI surface
- `atlas calendar status/feed/login/logout` + `atlas calendar manual add <start> <end>`
  (hoy **no** existe `cli/commands/calendar.rs`).

**Entregable:** el operador ve los runs de Atlas en su calendario nativo (webcal)
y el Planning respeta sus busy windows. **KPI:** feed RFC 5545 válido en
Outlook/Apple/Google; ningún turn proactivo se encola dentro de una busy window.

## SECTOR C — Fuera de alcance v3

- WinRT `AppointmentManager` (capability restringida — prohibido).
- iOS, multi-usuario, HTTP registry (heredado del out-of-scope v2).
- C (context-mode) y A (Axum 0.8) — diferidos a v4 salvo prioridad nueva.

## SECTOR D — Siguiente paso operativo

1. Commit de este plan + RFC 49 + Index 26 + README.
2. v3.1.0 **COMPLETO** (token opaco §G.2 ✅ `calendar/token.rs`).
3. v3.1.1 (Graph) → v3.1.2 (Planning) → v3.1.3 (CLI), un commit por sub-fase.
4. Phase 13 permanece en `research/49` hasta confirmar write-access/versión.

## Estado de ejecución

- **v3.1.0 ICS WRITE:** ✅ implementado (pre-existente, ruta HUD + `ics_writer`).
- **v3.1.1 Graph READ:** ✅ implementado y **verificado end-to-end**.
  - OAuth: authorization-code + PKCE con loopback (`calendar/auth.rs`); abre el
    navegador y captura el `code` solo (sin pegar códigos). `reqwest` en lugar de
    `graph-rs-sdk` (retirado de `calendar-graph`). Refresh token cifrado
    AES-256-GCM con clave `<profile_root>/calendar.key`.
  - Cliente: `atlas calendar login` / `sync` / `status` (`calendar-graph`).
  - Reader: `calendar/graph_reader.rs` (`CalendarReader` → `me/calendarView`,
    `graph` busy windows, dedupe `(source, external_id)`, eviction de la corrida
    previa).
  - Registro Azure (no es secreto): `client_id = a271f4c7-b9bb-48b2-84e5-47c1a6c3e8af`,
    authority `common`, audience `AzureADandPersonalMicrosoftAccount`,
    `Calendars.Read` delegado con consent `AllPrincipals`, public client con
    redirects `http://localhost` + nativeclient. Overrides:
    `ATLAS_GRAPH_CLIENT_ID` / `ATLAS_GRAPH_TENANT`.
  - Smoke real: `sync` trajo 2 eventos ("Reunión con Pedro", "Ingeniero Plan de
    Aula") → `calendar busy list` muestra 2 busy windows `graph` (w=0.50).
- **v3.1.3 CLI surface:** ✅ implementado — `atlas calendar feed` +
  `atlas calendar busy list/count/add/rm`, sobre los envoltorios
  `Journal::busy_window_*` (`journal/calendar_ops.rs`).
- **v3.1.4 ICS subscription READ:** ✅ implementado — `atlas calendar sync-ics <url>`
  (`calendar/ics_reader.rs`, feature `calendar-ics`), parser `icalendar 0.17`
  (decisión y justificación en `22 §15`); busy windows `source=ics_local`.
  3 tests (timed + all-day + malformado). Smoke: ICS público → 317 windows.
- **v3.1.A.2 ICS subscriptions durables:** ✅ implementado — tabla
  `calendar_subscriptions` (migración 34) + `atlas calendar
  subscribe/unsubscribe/subscriptions/sync-all`. Feeds namespaced
  (`{name}:{uid}`) con borrado por prefijo (varias suscripciones conviven en
  `ics_local` sin pisarse) y GET condicional (`If-None-Match` /
  `If-Modified-Since` → `304`). Repo en `journal/calendar_ops.rs`. 5 tests
  nuevos (schema + queue + CLI). Smoke: subscribe + sync-all → 317 windows;
  Google no envía validadores (304 best-effort).
- **v3.1.A.3 Poller en background:** ✅ implementado — `calendar/poller.rs`
  (`CalendarPoller`, mismo patrón que `ToastDriver`) spawneado desde
  `AppState::bootstrap`; reusa `ics_reader::sync_all_ics`; cadencia vía
  `ATLAS_CALENDAR_POLL_SECS` (default 900 s, `0` desactiva) y primer sync a
  los 30 s. El I/O de red (`fetch_ics`) se separó de la escritura
  (`apply_ics`) → testeable sin red. 4 tests nuevos. **v3.1.A.2 cerrada.**
- **v3.1.2 Planning wiring:** pendiente — **bloqueado en infraestructura**: el
  repo **no tiene** un mecanismo de "turn proactivo" ni `AppState.context_window`
  (grep repo-wide sin resultados). RFC 28 §G.4 item 8 (`Planning::next_free_slot`)
  asume ese motor; construirlo es una fase con RFC propio, no un wiring. La
  utilidad ya disponible es `Journal::busy_windows_overlapping` (v3.1.3).
