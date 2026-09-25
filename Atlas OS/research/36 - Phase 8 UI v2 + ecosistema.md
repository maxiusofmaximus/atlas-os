# 36 — Phase 8 UI v2 + mejoras de ecosistema (plan refinado)

**Propósito:** Refinar la Fase 8 RFC 20 ("UI v2 accesible desde cualquier dispositivo") con la evidencia del RFC 35 (Round 7: frecency de zoxide, FTS5 de context-mode, sysinfo/nvml, ratatui, axum-oidc). Output: sub-fases atómicas, sin crates nuevas salvo audit previo (RFC 25 §11).

**Gestión:** trabajo mecánico pesado delegado a `muse-spark-1.3-contributor-free` vía `opencode run` (prompts con referencias inline cortas — lección Round 7); el gestor audita crates, revisa, commitea y pushea.

---

## SECTOR A — Decisiones ya tomadas (evidencia RFC 35)

### A.1 Frecency interno propio (port zoxide)
Algoritmo frecency (aging + ranking, decay exponencial, determinista) en `Journal` sobre missions/worktrees accedidas — sin crate. La crate zoxide RECHAZADA (22 deps).

### A.2 FTS5 sobre journal_events (patrón context-mode)
`rusqlite` bundled incluye FTS5 — virtual table FTS5 sobre los payloads + `Journal::search_events(query)` enriquece el Observer (6.1) y `atlas journal --query`. Sin crates.

### A.3 Crates nuevas a auditar (audit RFC 25 §11 previo obligatorio)
- `sysinfo` (Rust, MIT) — VRAM/RAM/CPU monitor. + `nvml-wrapper` (Rust, MIT) — NVML NVIDIA.
- `ratatui` (Rust, MIT) — Sister TUI.
- `axum-oidc-layer` / `openidconnect` (Rust) — remote auth SSO/OIDC.
- RustDesk: **AGPL — jamás bundling** (viola la distribución MIT); integración lateral solo.

### A.4 Skill Picker iluminado/grisado (RFC 20 + RFC 17 §4)
El `SkillGraph::find_candidates` (top-K por priority) ya existe; el Picker HUD ilumina las sugeridas y grisa las no relevantes — el scoring de relevancia viene del prompt (embeddings opcionales) + priority + engine match.

---

## SECTOR B — Plan refinado Phase 8 (5 sub-fases atómicas)

### Sub-fase 8.0 — Frecency + FTS5 (quick wins RFC 35, sin crates)
- M31 migration: FTS5 virtual table `journal_events_fts` (contentless sobre payload/kind) + `dir_access` table (frecency de worktrees/missions).
- `Journal::search_events(query) -> Vec<JournalEntry>` (FTS5 MATCH + fallback LIKE) + `Journal::record_dir_access(dir)` + `journal::frecency(prefix) -> Vec<(String, f64)>` (aging + ranking determinista).
- CLI: `atlas journal --query <q>` / `atlas swarm jump <prefix>` (frecency top matches).
- Tests: FTS5 search round-trip, fallback LIKE, frecency determinista + aging decay, idempotencia.

### Sub-fase 8.1 — Skill Picker iluminado/grisado (RFC 17 §4)
- `skills/picker.rs`: `pick_skills(prompt, skills) -> Vec<ScoredSkill {manifest, relevance (0.0-1.0), suggested (bool)}>` — scoring: priority (0.4) + engine/domain match contra el prompt (0.4, keyword match determinista) + lifecycle (verified 0.2) — embeddings opcionales feature-gated como follow-up. Threshold sugerido (default 0.5).
- CLI: `atlas skill pick "<prompt>"` — lista iluminada (★ sugeridas) vs grisada.
- Tests: scoring determinista, threshold filtra, suggested/grisado split.

### Sub-fase 8.2 — VRAM/RAM/cost monitor (crates auditaras → delegar tras audit)
- Audit `sysinfo` + `nvml-wrapper` (deps transitive, MSRV) — si pasan: feature `hardware-monitor` default off, `security`... no — módulo `monitor/` nuevo: `SystemMonitor` (RAM/CPU via sysinfo, VRAM via nvml-wrapper en cfg(windows) con fail-safe), snapshot → Kernel Bus event `hardware_snapshot` + HUD card.
- Si el audit falla: monitor propio (std::process wmic/systeminfo — hacky) o defer a Phase 9. Documenta.

### Sub-fase 8.3 — Command Center web remoto (SSO/OIDC)
- Audit `axum-oidc-layer`/`openidconnect` — si pasan: feature `remote-ui` default off; el axum HUD server añade auth layer OIDC (SSO) para el acceso remoto + session cookies.
- Sin crate viable → defer con audit documentado.

### Sub-fase 8.4 — Sister IDE-in-a-terminal (ratatui)
- Audit `ratatui` — si pasa: `src/cli-tui/` o binario `atlas-tui` (feature `tui` default off): Document Model conecta al mismo Kernel Bus WS que el HUD (RFC 20 Sector B, research `28` Sector B).
- Sin crate viable → defer.

### Sub-fase 8.5 — Remote-live dual-PC (RustDesk lateral, AGPL)
- RustDesk como proceso EXTERNO lanzado por Atlas OS (`std::process::Command` si está instalado — jamás link/bundle AGPL): `atlas remote` detecta RustDesk en PATH y lanza la sesión + el modelo Nate Gentile (PC servidor + PC cliente).
- Sin RustDesk → mensaje útil. Documentación del modelo de negocio (RFC 20 Sector B).

**Entregable:** ve el swarm desde el móvil/tablet, programa en vivo desde un PC thin. KPI: latencia end-to-end UI <100ms (RFC 20).

---

## SECTOR C — Fuera de alcance (esta iteración)

- RustDesk host embed (AGPL — integración lateral solo, jamás bundling).
- Modelos propios entrenados desde cero (RFC 20 out of scope).
- Streaming real-time del monitor a móvil (poll 5s suficiente para MVP).

## SECTOR D — Siguiente paso operativo

1. Commit de este plan + RFC 20 update.
2. Gestor audita crates (sysinfo/nvml/ratatui/oidc) en paralelo con la delegación 8.0.
3. Sub-fase 8.0 (delegada) → revisar → commit → push.
4. Sub-fases 8.1-8.5 según audits.
