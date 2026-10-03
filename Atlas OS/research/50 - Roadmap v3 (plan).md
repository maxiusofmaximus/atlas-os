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
- **Gap real:** endpoint `.ics`, adaptador Graph (OAuth + token cifrado + poll),
  y wiring de `BusyWindow{Graph}` al Planning.

### A.3 Descarte de candidatos (evidencia)
- **A (Axum 0.8):** higiene de deps; 25 rutas en `hud/server.rs`; sin feature
  visible → diferido.
- **B (Swarm HUD):** YA implementado — `src/lib/components/SwarmConsole.svelte`
  (566 LOC, wired en `+page.svelte:388`, RFC 31 §B.4.5) → no es candidato v3.
- **C (context-mode MCP nativo):** infra MCP existe (`cli/commands/mcp.rs`,
  `mobile/mcp_template.rs`, `classifier/mcp_filter.rs`) pero falta definición de
  alcance → diferido hasta un RFC concreto.

## SECTOR B — Plan v3 (Fase v3.1: Windows Calendar real)

### Sub-fase v3.1.0 — ICS WRITE (`GET /atlas-calendar.ics`)
- Ruta HUD en `hud/server.rs` + handler en `calendar/` reutilizando
  `IcsMission*` (ya existen): construye el feed RFC 5545 desde missions/schedules
  del journal.
- Token opaco `?token=base64url(16 random bytes)` (unguessable, sin auth) +
  path reservado.
- Tests: ICS bien formado (line folding/escaping), token inválido → 404, feed
  vacío sigue siendo válido.
- Smoke: `atlas calendar feed` imprime la URL `webcal://…`.

### Sub-fase v3.1.1 — Graph READ (OAuth + token cifrado + poll)
- `calendar_auth` (m18) almacena el refresh token cifrado (AES-256-GCM).
- CLI `atlas calendar login` (paste `client_id`+`tenant`, o popup), scopes
  `Calendars.Read` + `offline_access`.
- Poll 60 s (`tokio::spawn` desde `AppState::new()`): `/me/calendarView` →
  `BusyWindowQueue::upsert` con `source=Graph` + `weight`.
- Tests: token cifrado round-trip, expirado → silent refresh, fallo → Toast de
  re-login.

### Sub-fase v3.1.2 — Planning wiring
- `AppState.context_busy_windows` (RFC 12 §3) recibe los `BusyWindow` de Graph;
  `overlaps(turn_eta)` decide si encolar el turn proactivo.
- Threshold: >5 eventos en la próxima hora → `busy` con `weight=1.0`.
- Tests: `overlaps` boundary, threshold, precedencia de `source`.

### Sub-fase v3.1.3 — CLI surface
- `atlas calendar status/feed/login/logout` + `atlas calendar manual add <start> <end>`.

**Entregable:** el operador ve los runs de Atlas en su calendario nativo (webcal)
y el Planning respeta sus busy windows. **KPI:** feed RFC 5545 válido en
Outlook/Apple/Google; ningún turn proactivo se encola dentro de una busy window.

## SECTOR C — Fuera de alcance v3

- WinRT `AppointmentManager` (capability restringida — prohibido).
- iOS, multi-usuario, HTTP registry (heredado del out-of-scope v2).
- C (context-mode) y A (Axum 0.8) — diferidos a v4 salvo prioridad nueva.

## SECTOR D — Siguiente paso operativo

1. Commit de este plan + RFC 49 + Index 26 + README.
2. Sub-fase v3.1.0 (ICS WRITE, autocontenida con el payload existente) → tests → commit.
3. v3.1.1 (Graph) → v3.1.2 (Planning) → v3.1.3 (CLI), un commit por sub-fase.
4. Phase 13 permanece en `research/49` hasta confirmar write-access/versión.
