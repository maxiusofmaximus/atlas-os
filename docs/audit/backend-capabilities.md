# Inventario de capacidades del backend — Atlas OS

**Builder-2 · 2026-10-06 · solo lectura** (entrada para RFC 67 - UI Specification §1.7/§19/§20).
Alcance (a)–(e) del plan `docs/coordination/PLAN-docs-hardening.md`. Nada de esto toca `src/`, `src-tauri/`, RFCs ni git.

> **Actualizado 2026-10-07 (cierre G9, tras FASE 10/11 backend):** rutas 41→**45**, tablas 61→**62**, `schema_version` **41**; B1–B9 reflejados (H-01…H-09). Totales re-derivados del código real (`hud/server.rs`, `journal/schema.rs`, `core/bus.rs`, `hud/{cost,missions,approvals,annotate,snapshot}.rs`, `journal/frecency.rs`), no de memoria.

**Metodología y procedencia.** Todo [O] = observado esta sesión por lectura directa de ficheros (`src-tauri/src/hud/server.rs` completo, 24 módulos `hud/*.rs`, `core/bus.rs`, `cli/proto.rs`, `journal/schema.rs`) y por regex reproducibles (cada sección cita el suyo). [I] = inferencia razonada; **NO-VERIFICADO** = no ejecuté un test que lo ejerza. [P] = pendiente.

**Tests ejecutados esta sesión** [O — 3 corridas `cargo test --manifest-path src-tauri/Cargo.toml --lib <filtro>`]:
1. `-- hud::cost hud::health hud::audit hud::worktrees hud::demos hud::mcp hud::skills hud::secrets hud::approvals hud::tail hud::observer hud::annotate hud::export hud::autoresearch hud::remote_status hud::availability hud::reliability hud::eval hud::cards core::bus` → **63 passed, 0 failed** (1.94 s).
2. `-- journal::tests mission_graph` → **100 passed, 0 failed** (5.33 s).
3. `-- cli::commands` → **61 passed, 0 failed** (0.35 s).

Total (auditoría original): **224 tests Rust ejecutados, 0 fallos** [O]. Tras FASE 10/11: `cargo test --manifest-path src-tauri/Cargo.toml --lib` → **1429 passed, 0 failed, 1 ignored** [O, corrida del Project Lead tras B1–B8].

---

## (a) Rutas axum del HUD

**Fuente [O]: `src-tauri/src/hud/server.rs` (196 líneas, leído completo)** + los 24 módulos handler de `src-tauri/src/hud/`.

**Servidor [O]:** bind loopback efímero `127.0.0.1:0` (`serve`, server.rs:19-27) o explícito host:port (`serve_on`, server.rs:32, RFC 29 §3.A `atlas serve`); puerto persistido a `<profile>/hud_port.txt` (53) y anunciado con `HudServed` (60-64); **CORS permissive** (165) + TraceLayer (166); graceful shutdown por `CancellationToken` (172-174).

**Recuento [O]: 45 entradas de ruta — 44 siempre montadas + 1 feature-gated (`/atlas-calendar.ics` con `calendar-ics`; `/graph/:mission_id` ya **sin gate** — B1).** Métodos: GET×36 (incl. 1 upgrade WS), POST×12, DELETE×1; 4 entradas exponen 2 métodos (`/diff/:id/annotation`, `/task/:id/annotation`, `/hud/secrets`, `/hud/missions`).

| # | Ruta | Método | Handler (fichero:línea) | Request | Response | Test ejecutado [O] |
|---|---|---|---|---|---|---|
| 1 | `/health` | GET | `health` (server.rs:180) | — | `"ok"` | — |
| 2 | `/ws` | GET(WS) | `ws::ws_handler` (ws.rs:19) | upgrade; mensajes cliente ignorados (Phase 0, ws.rs:33-47) | broadcast del Kernel Bus serializado a JSON (`fan_out`, ws.rs:52-76) | **NO-VERIFICADO** (no hay `hud::ws::tests`) |
| 3-16 | `/tail/{journal,missions,verdicts,consolidated,plans,diffs,validation_reports,repairs,patterns,checkpoints,skills,model_swaps,step_states,agent_steps}` | GET | `tail::tail_*` (tail.rs:154-212, genérico `tail_of::<R>` tail.rs:69) | `?last=N` (TailQuery tail.rs:34, clamp `[1,200]`, default 20 — tail.rs:29-30) | `Vec<Value>` (rows; serialización fallida → `Null`, tail.rs:52-54) | 6 (`hud::tail::tests`: clamps + 3 tails contra journal vacío) |
| 17 | `/hud/journal` | GET | `observer::get_journal_page` (observer.rs:59) | `JournalPageQuery` (observer.rs:39): `limit?`/`offset?`/`kind?` | `JournalPageResponse` (observer.rs:52): `{rows, total}`; 400 limit/offset inválidos | 9 (`hud::observer::tests`: paginado, filtros, 400s) |
| 18 | `/hud/approvals/:id/approve` | POST | `approvals::approve` (approvals.rs:128) | body opcional `DecisionBody` (approvals.rs:26-34): `{user_id?, reason?}` | `DecisionAck`: `{approval_id, decision, user_id}`; publica `ApprovalDecision` **y persiste** la decisión (`record_approval_decision`, journal/mod.rs:398); 400 id inválido | 2 (`hud::approvals::tests`: parse body) |
| 19 | `/hud/approvals/:id/deny` | POST | `approvals::deny` (approvals.rs:146) | idem approve | idem; **`reason` ahora se persiste** (antes se descartaba — H-02) | (mismos tests + `batch_applies_all_and_persists_reason`) |
| 20 | `/hud/approvals/batch` | POST | `approvals::batch` (approvals.rs:165) | `BatchBody` (approvals.rs:54): `{decision: "approve"\|"deny", items:[{approval_id, files[]}], user_id?, reason?}` | `BatchAck` (64): `{decision, user_id, applied[]}`; **409** si dos items comparten fichero, id duplicado o decisión contradice una previa; **400** items vacío/decision inválida/uuid inválido | 4 (`hud::approvals::tests`: happy + 409 fichero + 409 opuesta + 400) — B3/H-02 |
| 21 | `/diff/:id/annotation` | POST+GET | `annotate::post_annotation` (annotate.rs:48) / `get_annotations` (79) | `AnnotationPost` (annotate.rs:28) | `AnnotationPosted` (42); 400 diff_id malformado, 422 body vacío | 4 (`hud::annotate::tests`: round-trip, orden, 400/422) |
| 22 | `/task/:id/annotation` | POST+GET | `annotate::post_task_annotation` (annotate.rs:110) / `get_task_annotations` (143) | `TaskAnnotationPost` (annotate.rs:95): `{body, author, file_path?, line_no?}`; `task_id` opaco (string) | `TaskAnnotationPosted` (104): `{id, task_id, created_at}`; 400 task_id vacío, 422 body vacío | 2 (`hud::annotate::tests` task round-trip + 422) — B8/H-05 |
| 23 | `/hud/agent/:run_id/snapshot` | GET | `snapshot::get_agent_snapshot` (snapshot.rs:44) | run_id en path | `SnapshotResponse` (snapshot.rs:27): `{run_id, sandbox, root, file_count, files[{path, sha256}]}`; **404** `{reason}` (run desconocido / sin sandbox / root ilegible) | 3 (`hud::snapshot::tests`: frame, 404 unknown, 404 no-sandbox) — B6/H-08 |
| 24 | `/audit/export-posting` | POST | `export::post_export_posting` (export.rs:48) | `ExportPostingRequest` (export.rs:34): `{last?, output_dir?}` | `ExportPostingResponse` (41): `{files_written, entries_packed, entries_purged, snapshot_root}`; `last` clamp 10000 | 5 (`hud::export::tests`: dirs, clamps, journal vacío) |
| 25 | `/autoresearch/cancel` | POST | `autoresearch::post_autoresearch_cancel` (autoresearch.rs:49) | `AutoresearchCancelRequest` (37): `{run_id, outcome}` | 204; 400 vacío/oversized; acepta `paused` forward-compat | 7 (`hud::autoresearch::tests`) |
| 26 | `/payload/:kind/:id` | GET | `tail::payload` (tail.rs:234) | kind ∈ `{verdict, plan, diff, validation_report, repair, pattern, checkpoint}` (tail.rs:236-248) | payload crudo como JSON string; 404 kind desconocido | (vía `journal::tests` round-trips de payloads) |
| 27 | `/payload/skill/:skill_id/:version` | GET | `tail::payload_skill` (tail.rs:261) | skill_id+version | payload de manifest | [P: no ejecutado] |
| 28 | `/remote/status` | GET | `remote_status::get_remote_status` (remote_status.rs:21) | headers (`Authorization`/`Cookie`) | status snapshot (bearer configured?, OIDC set?); **con feature `remote-ui`**: enforce bearer, 401 si no (remote_status.rs:26-50) | 1 (`status_never_leaks_the_token`); el test de enforce 401 está tras `#[cfg(feature = "remote-ui")]` (remote_status.rs:109-132) — **NO-VERIFICADO con feature** |
| 29 | `/hud/eval/summary` | GET | `eval::get_eval_summary` (eval.rs:22) | `EvalQuery` (17): `last?` | summary + groups | 1 (`hud::eval::tests::summary_reports_seeded_cases`) |
| 30 | `/hud/cost` | GET | `cost::get_cost` (cost.rs:32) | `CostQuery` (18-22): `window?` (clamp 1..5000, default 200) | `{window, totals, by_model (costliest first), cumulative_usd, pressure {level, warn_usd=5.0, crit_usd=20.0}, pending_resets[], **budget_usd, remaining**}` — budget leído de `<profile>/budget_usd.txt` (cost.rs:36,58-59,79-80); ausente ⇒ `null` | 3 (`hud::cost::tests`: totals + budget presente/null) — B5/H-06 |
| 31 | `/hud/health` | GET | `health::get_health` (health.rs:24) | `HealthQuery` (18-22): `last?` (clamp 1..200, default 20) | `{agent_events {total, by_type, recent}, agent_runs[], swarm_agents[]}` | 1 (`hud::health::tests::reports_event_counts_and_run_states`) |
| 32 | `/hud/audit` | GET | `audit::get_audit` (audit.rs:24) | `AuditQuery` (18-22): `last?` (clamp 1..1000, default 50) | `{rows[], count}` — sin verificación de hash-chain (declarado NO implementado, audit.rs:4-7) | 1 (`hud::audit::tests::empty_audit_log_returns_no_rows`) |
| 33 | `/hud/worktrees` | GET | `worktrees::get_worktrees` (worktrees.rs:27) | `WorktreesQuery` (17-21): `repo?` (default cwd) | `{repo, ok, reason, entries[{path, branch, detached}]}`; fail-safe `ok:false` + reason (worktrees.rs:23-25) | 1 (`non_repo_path_reports_ok_false_with_reason`) |
| 34 | `/hud/demos` | GET | `demos::get_demos` (demos.rs:38) | `DemosQuery` (17-21): `last?` (clamp 1..500, default 50) | `{artifacts[{id, run_id, kind, path, sha256, verified, preview_url}], count}`; `preview_url` solo http(s) (demos.rs:26-36) | 2 (`hud::demos::tests`) |
| 35 | `/hud/mcp` | GET | `mcp::get_mcp` (mcp.rs:32) | `McpQuery` (23) | servers + policy (sandbox/supply-chain/allowlist) aplicada | 7 (`hud::mcp::tests`: enrich, missing config, allowlist round-trip) |
| 36 | `/hud/mcp/allowlist` | POST | `mcp::set_allowlist` (mcp.rs:114) | `AllowlistBody` (105) | ok/refleja; 400 server desconocido | (mismos tests) |
| 37 | `/hud/mcp/probe` | POST | `mcp::probe` (mcp.rs:159) | `ProbeBody` (150) | tools del server; 400 sin supply-chain | (mismos tests: probe_refuses…) |
| 38 | `/hud/skills/:id/activate` | POST | `skills::activate` (skills.rs:44) | body opcional `ActivateBody` (31-36): `{agent_id?}` (default nil UUID = operador) | `ActivateAck` (38-42): `{skill_id, agent_id}`; publica `SkillActivated`; 400 id vacío/>128 (skills.rs:26) | 2 (`hud::skills::tests`: parse body) |
| 39 | `/hud/secrets` | GET+POST | `secrets::get_secrets` (43) / `post_secret` (55) | GET: —; POST: `SetSecretBody` (31-35): `{account, value}` | GET: `{service, slots[{account, present}]}` — **nunca devuelve el valor** (secrets.rs:8-11); POST: `{stored}`; 400 blank | 2 (`hud::secrets::tests`: catálogo sin valores, 400 blank) |
| 40 | `/hud/secrets/:account` | DELETE | `secrets::delete_secret` (72) | account en path | `{deleted, account}` | (parse tests CLI) |
| 41 | `/hud/availability` | GET | `availability::get_availability` (availability.rs:14) | — | `{enabled, policy, availability, pending_mission}` | 1 (`reports_policy_and_pending_mission`) |
| 42 | `/hud/reliability` | GET | `reliability::get_reliability` (reliability.rs:14) | — | `{policy, models (reliabilities)}` | 1 (`reports_policy_and_model_reliabilities`) |
| 43 | `/hud/missions` | GET+POST | `missions::get_missions` (missions.rs:67) + `missions::post_mission` (missions.rs:32) | GET: `MissionsQuery` (missions.rs:63): `sort?` ∈ `{recent, frecency}`; POST: `NewMissionRequest` (24): `{prompt}` | GET: `{sort, count, missions:[{id, label, status, frecency}]}` (frecency = actividad del journal; journal vacío ⇒ orden estable `created_at DESC`); POST: 201 `{mission_id, status:"received"}`, 400 prompt vacío/largo | GET 2 (`hud::missions::tests`: frecency order + empty stable — B4/H-01); POST 2 (`creates_a_mission_from_a_prompt`, `rejects_blank_prompt` — B7/H-09) |
| 44 | `/graph/:mission_id` | GET | `graph::get_graph` (graph.rs:30) — **sin gate** (server.rs:154; B1) | mission_id | `MissionGraph` (`{mission_id, nodes[], edges[]}`); 404 si no hay grafo | [P: handler NO-VERIFICADO por test propio; schema `m15_*` sí] |
| 45 | `/atlas-calendar.ics` | GET | `calendar::ics_route::get_calendar_ics` — **cfg `calendar-ics`** (server.rs:159-163) | token | ICS feed | [P: no ejecutado] |

**Auth [O]:** ninguna ruta autentica por defecto — el HUD es loopback-only y CORS permissive (server.rs:165). La ÚNICA ruta con gate es `/remote/status` **con** feature `remote-ui` (bearer `Authorization` o cookie `atlas_session`, 401 si no — remote_status.rs:26-50). [I] El resto de rutas (secrets incluidas: secrets.rs:9-11) queda expuesta sin bearer si el HUD se publica con `atlas serve --host` — la spec de UI debe tratarlo (RFC 18).

---

## (b) Eventos del Kernel Bus/WebSocket y consumo en `hud.ts`

**Fuente [O]: `src-tauri/src/core/bus.rs` (391 líneas) + `hud/ws.rs` (76 líneas) + `src/lib/stores/hud.ts` (1585 líneas).**

**Sobre [O]:** `BusEvent { id: Uuid, idempotency_key: String, kind: BusEventKind, ts: DateTime<Utc> }` (bus.rs:8-13) — entrega at-least-once con `idempotency_key` (RFC 02 §3.1.2, bus.rs:3).

**`BusEventKind` (bus.rs:17-189) — 26 variantes**, serde `tag = "type", rename_all = "snake_case"` (bus.rs:15-16). Payload de cada una [O]:

| Evento `type` (WS) | Payload | Líneas bus.rs |
|---|---|---|
| `task_received` | `{raw_prompt, session_id}` | 18-21 |
| `mission_consolidated` | `{mission_id, verdict_id, confidence: Confidence}` | 22-26 |
| `mission_locked` | `{mission_id, planning_session_id}` | 27-30 |
| `plan_generated` | `{plan_id, mission_id}` | 31-34 |
| `agent_status_changed` | `{agent_id, status: AgentStatus}` | 35-38 |
| `agent_diff` | `{agent_id, files, lines_added, lines_removed}` | 39-44 |
| `agent_tokens` | `{agent_id, tokens_in, tokens_out, cost_usd}` | 45-50 |
| `agent_step` | `{run_id, step, action, observation?, verdict?, tokens_in, tokens_out, cost_usd}` | 53-62 |
| `agent_heartbeat` | `{agent_id}` | 63-65 |
| `approval_request` | `{approval_id, agent_id, action}` | 66-70 |
| `approval_decision` | `{approval_id, decision: ApprovalDecisionKind, user_id}` | 71-75 |
| `doom_loop_detected` | `{agent_id, count}` | 76-79 |
| `goal_drift_detected` | `{agent_id, drift}` | 80-83 |
| `journal_checkpoint` | `{checkpoint_id}` | 84-86 |
| `cost_threshold_crossed` | `{agent_id, threshold, cumulative}` | 87-91 |
| `worktree_dirty` | `{agent_id, path, dirty}` | 92-96 |
| `skill_activated` | `{agent_id, skill_id}` | 97-100 |
| `artifact_preview_opened` | `{mission_id?, artifact, url, source}` | 104-109 |
| `research_completed` | `{research_run_id}` | 110-112 |
| `hud_served` | `{hud_port}` | 113-115 |
| `mission_steered` | `{mission_id, message}` | 121-124 |
| `model_swapped` | `{mission_id, prev_model_id, new_model_id, initiator: SwapInitiator}` | 130-135 |
| `step_phase_changed` | `{mission_id, plan_id, step_id, phase: StepPhase}` | 140-145 |
| `autoresearch_cancelled` | `{run_id, outcome}` | 153-156 |
| `spend_limit_observed` | `{provider, model, status_code, resets_at_ms, error_type, toast_enqueued_id?}` | 163-177 |
| `hardware_snapshot` | `{ram_total_mb, ram_used_mb, vram_total_mb?, vram_used_mb?, cost_usd}` | 182-188 |

**Enums auxiliares [O]:** `AgentStatus` (bus.rs:224-236): `Queued, Reading, Planning, Coding, Reviewing, Idle, Paused, DoomLoop, Error, Success, **Unknown**` — **11 estados**; `Unknown` añadido por B2 (H-03 resuelto). `ApprovalDecisionKind` (240-245): `Apr, Deny, Steer, Fork`. `Confidence` (204-209): `High, Medium, Low, Block` con `threshold()` (212-219). `SwapInitiator` (194-199): `User, Auto`.

**Entrega WS [O:** `hud/ws.rs`]: `ws_handler` (19-24) → `client_loop` (26-50): cada cliente subscribe al broadcast del bus (28) y `fan_out` (52-76) serializa cada `BusEvent` a `Message::Text`; `Lagged` → warn + descarte (70-72); **cliente→servidor ignorado en Phase 0** (33-47) — [I] el WS es solo-bajada; todo steer/decisión va por REST.

**Consumo en `hud.ts` [O]:**
- `normalizeWsEvent` (hud.ts:345) normaliza cada frame: acepta el frame del bridge `{id, kind: {type, ...}, ts}` (el objeto interno se vuelve el `payload`) y el plano `{kind, payload}` (tests/más viejos) — ambas rutas cubiertas por 4 tests de `hud.test.ts:987-1008` (describe RFC 65 normalizeWsEvent) [O].
- Store `hud` (hud.ts:409-425): `connect(target)` (resuelve `ws(s)://` con `toWsUrl`, 162) / `disconnect()`; reconexión backoff `×2` cap 10 s (`scheduleReconnect`, 401-407).
- Proyecciones sobre `$hud.events` [O]: `projectSwarmAgents` (+page.svelte:92), `projectPendingApprovals` (hud.ts:1152), `projectLatestHeartbeat` (hud.ts:1295), `projectKanban` sobre `fetchMissions`.
- [I] El contrato `hud.ts` ↔ `bus.rs` coincide: el `kind.type` snake_case del bus es el `HudEvent.kind` del store (test `core::bus::tests::every_tag_is_lowercase_snake_with_no_spaces` VALIDADO [O]).

**`KernelCommand` (bus.rs:249-276) — 8 comandos [O]:** `NewMissionFromPrompt{raw_prompt}`, `SwitchProfile{profile_id}`, `StopAgent{agent_id}`, `PauseAgent{agent_id}`, `ResumeAgent{agent_id}`, `SteerAgent{agent_id, message}`, `ForkAgent{agent_id}`, `DecideApproval{approval_id, decision, user_id}`.
**⚠️ DEAD CODE [O]:** el enum lleva `#[allow(dead_code)]` (bus.rs:248) y `grep KernelCommand src-tauri/src` da **un único match: la declaración** — ningún consumidor en todo `src-tauri`. La spec RFC 67 §1.7 los cita como "Comandos Kernel" del contrato; en realidad **no están cableados** (las acciones operativas van por REST: approve/deny → `ApprovalDecision`, steer → `atlas steer` → `MissionSteered`). [I] severidad media: contrato aspiracional, no comportamiento.

---

## (c) Comandos CLI `atlas` con flags

**Fuente [O]: `src-tauri/src/cli/proto.rs` (130 líneas) + 39 ficheros en `src-tauri/src/cli/commands/` + `bin/opencode.rs` / `bin/atlas-tui.rs`.**

**Recuento [O]: 34 subcomandos top-level** (`Commands`, proto.rs:49-130) — 32 siempre compilados + `Serve` (cfg `hud`, proto.rs:85-86) + `Toast` (cfg `toast`, proto.rs:128-129). Flags globales: `--profile` (proto.rs:28, env `OC_PROFILE`, resuelto en `active_profile()` 41-45) y `-v/--verbose` (32, Count). Dispatch en `commands/mod.rs:84+` (`Commands::Mission(c) => mission::run(c, profile).await`, …).

| Comando | proto.rs | Descripción (doc comment) | Flags clave [O: lectura directa] |
|---|---|---|---|
| `mission` | 51 | crear mission desde prompt | subacciones (mission.rs:24) + `--force` `--autoresearch` `--metric` `--max-steps` `--timebox` |
| `plan` | 53 | Plan desde Mission locked | — |
| `run` | 55 | ejecutar Plan (Coding→Validation→Repair) | `--mode` (run.rs:26) |
| `resume` | 57 | resume desde checkpoint | — |
| `fork` | 59 | fork de sesión (Cursor pattern) | — |
| `steer` | 61 | steer mid-run | — |
| `swap-model` | 63 | hot-swap de modelo (RFC 27 §B) | — |
| `models` | 65 | registry + affinity refresh (RFC 04 §8) | subcomandos (models.rs:16); `--list` `--min-samples` `--min-pass-rate` `--allow-unknown` `--strict` `--enable` `--disable` |
| `monitor` | 67 | snapshot VRAM/RAM/cost (RFC 20 8.2) | `--ram-warn` `--ram-crit` `--cost-warn` `--cost-crit` (monitor.rs:22-31, defaults = MONITOR_*_USD) `--no-publish` (34) |
| `mobile` | 69 | testing vía artemis externo | subcomandos (mobile.rs:29) |
| `exec` | 71 | LLM-driver Phase 2 (RFC 27 §F) | subacciones (exec.rs:31): Step/Wait/Tail/Publish |
| `execute` | 73 | loop del orchestrator (RFC 20 Fase 25) | `--max-attempts` `--coding` `--apply` `--root` `--research-refs` `--research-run` (execute.rs:254-273) |
| `agent` | 75 | loop terminal agent (RFC 20 Fase 39) | `--list-tools` (agent.rs:22) `--root` (25) `--max-steps` (28) `--command-timeout` (31) `--max-attempts` (34) `--verify` (38) `--web` (42) `--mcp` (46) `--sandbox` (50; valores `local\|wsl2\|daytona\|e2b`, agent.rs:173) |
| `browser` | 78 | terminal-browser pane (RFC 28 §I) | subacciones (browser.rs:23) Probe/Open/Ls/Action + `--bin` `--mission` `--no-publish` |
| `eval` | 80 | harness de evaluación (RFC 20 Fase 22) | subacciones (eval.rs:24): Run{suite, strict}/List/Report/Import/Metrics; única suite `golden` (eval.rs:117-118) |
| `hud` | 82 | control del HUD | `--auth-status` (hud.rs:16) `--rotate-token` (20) |
| `serve` | 86 (cfg hud) | daemon headless (RFC 29 §3.A) | `--host` (serve.rs:29) `--port` (32) |
| `mcp` | 88 | MCP servers (RFC 07) | subacciones (mcp.rs:27) + `--command` `--args` `--env` `--trusted` `--sandbox` `--allow` `--strict` |
| `channels` | 90 | gateway multi-canal (RFC 29 §3.B) | `--json` (channels.rs:18) |
| `skill` | 92 | skills (RFC 06) | subacciones (skill.rs:11) + `--from` `--engine` `--threshold` `--apply` |
| `domain` | 94 | domain packs (RFC 64) | subacciones (domain.rs:22) |
| `sister` | 97 | IDE-in-a-terminal frame (RFC 20 8.4) | `--ticks` |
| `security` | 99 | supply-chain gate (RFC 18 §4) | subacción Gate + `--manifest` (security.rs:22) |
| `secrets` | 101 | OS keychain (RFC 25 §3.10) | subacciones (secrets.rs:20): set/get/list/delete + `--value` `--show` |
| `validate` | 103 | Semgrep/CodeQL externos (RFC 20 12.0) | `--semgrep` `--codeql` (validate.rs tests) |
| `swarm` | 105 | role presets (RFC 05 4.1) | subacciones (swarm.rs:17) + `--preset` `--mission` `--unread-only` `--mark-read` `--limit` |
| `remote` | 107 | RustDesk externo (RFC 20 8.5) | subcomandos (remote.rs:25) + `--launch` `--bin` `--peer-id` |
| `profile` | 109 | switcher (RFC 25 §4) | subacciones (profile.rs:12) |
| `audit` | 111 | audit log (RFC 24 §10) | `--verify` (audit.rs:40) `--json` (45) `--export-posting` (50) `--snapshot-maybe` (57) + subacción Validate (60) |
| `journal` | 113 | tail (RFC 19) | `--last` `--search` (journal.rs:23-24) |
| `calendar` | 115 | ICS + busy windows (RFC 28 §G) | subacciones (calendar.rs:32) Busy/Availability/Policy/SyncIcs/Subscribe/Unsubscribe + `--eta-ms` `--weight` `--horizon-ms` `--enable` `--disable` |
| `learn` | 117 | learning rules (RFC 16) | subacciones (learn.rs:39): Rules/Promote/Deprecate/Compact/Summary/Export/Import + `--min-correct` |
| `research` | 126 | docs/query/ingest/ingest web (RFC 10/28) | subacciones (research.rs:75): Docs/Query/Note/Branches/Feasibility/Ingest/Scrape/Search/Crawl/Extract |
| `toast` | 129 (cfg toast) | toast queue (RFC 28 §F) | subacciones (toast.rs) |

**Tests CLI ejecutados [O — `cargo test --lib -- cli::commands` → 61 passed]:** parse de flags (audit×10, serve×2, browser×4, calendar×4, mobile×4, research×15, run×3, secrets×4, validate×6, exec×8, sister×1). `atlas agent` **no tiene tests unitarios** en `cli::commands` — su lógica está cubierta por `eval::golden` (CI gate) y tests live [I]; marcado **NO-VERIFICADO** por test unitario propio.

---

## (d) Tablas del journal y migraciones relevantes para la UI

**Fuente [O]: `src-tauri/src/journal/schema.rs` (1909 líneas).**

**Recuento [O]: 62 tablas** (`CREATE TABLE IF NOT EXISTS`, extraídas por regex sobre schema.rs) + `schema_version` (@22, versión actual **41**) como tabla de control. 43 referencias únicas a hitos M## (M0–M38 + M49–M52) en los comentarios de migración.

**62 tablas con línea [O]:** `schema_version`@22, `journal_events`@36, `missions`@49, `subagents`@57, `approvals`@72, `audit_log`@84, `research_runs`@97, `prompt_verdicts`@124, `mission_consolidated`@141, `plans`@172, `diffs`@207, `validation_reports`@243, `repair_runs`@278, `pattern_runs`@319, `mission_checkpoints`@358, `skill_manifests`@389, `model_swaps`@427, `step_states`@460, `diff_annotations`@491, `autoresearch_runs`@531, `autoresearch_candidates`@549, `agent_session_events`@598, `mission_graph_nodes`@648, `mission_graph_edges`@658, `learning_graphs`@672, `toast_queue`@721, `toast_history`@740, `calendar_busy_windows`@786, `calendar_auth`@802, `model_resets`@838, `models`@891, `deployments`@908, `model_aliases`@932, `model_groups`@938, `model_invocations`@988, `reflection_episodes`@1054, `council_votes`@1073, `task_classifier_decisions`@1122, `model_affinity_cache`@1139, `research_sources`@1236, `research_consensus`@1248, `research_notes`@1284, `feasibility_cache`@1320, `user_profile`@1352, `swarm_agents`@1389, `agent_mailbox`@1405, `learned_rules`@1446, `compaction_events`@1465, `dir_access`@1502, `ast_symbols`@1597, `calendar_subscriptions`@1635, `eval_runs`@1671, `eval_cases`@1692, `proactive_policy`@1727, `reliability_gate`@1750, `agent_runs`@1767, `agent_steps`@1781, `tool_invocations`@1806, `artifacts`@1820, `domain_packs`@1839, `domain_runs`@1849, `task_annotations`@1875.

**Tablas que la UI consume directamente [O: cada handler de (a) lee su tabla]:**

| Tabla | La lee | Via |
|---|---|---|
| `journal_events` | `/hud/journal`, `/tail/journal` | observer.rs / tail.rs |
| `missions` | `/tail/missions` | tail.rs:212 |
| `prompt_verdicts`, `plans`, `diffs`, `validation_reports`, `repair_runs`, `pattern_runs`, `mission_checkpoints`, `skill_manifests`, `model_swaps`, `step_states`, `agent_steps` | `/tail/*` + `/payload/:kind/:id` | tail.rs (trait `TailRow` 61-63) |
| `audit_log` (con `previous_hash`/`this_hash` BLOB, schema.rs:84-93) | `/hud/audit`, `/audit/export-posting` | audit.rs:30, export.rs |
| `approvals` | `/hud/approvals/:id/{approve,deny}` + `/hud/approvals/batch` (escribe la decisión) | approvals.rs (`record_approval_decision`, journal/mod.rs:398); la cola pendiente sigue llegando por `approval_request` |
| `agent_session_events` | `/hud/health` | health.rs:30-34 |
| `agent_runs` | `/hud/health`, AgentCard, `/hud/agent/:run_id/snapshot` | health.rs:35; snapshot.rs:51 |
| `swarm_agents`, `agent_mailbox` | SwarmConsole (REST swarm) | hud.ts:722-820 |
| `model_invocations` | `/hud/cost` | cost.rs:38-40 |
| `model_resets` | `/hud/cost` (pending_resets) | cost.rs:41-43 |
| `diff_annotations` | `/diff/:id/annotation` | annotate.rs |
| `task_annotations` (M52) | `/task/:id/annotation` | annotate.rs:110/143 |
| `eval_runs`, `eval_cases` | `/hud/eval/summary` | eval.rs |
| `proactive_policy`, `calendar_busy_windows` | `/hud/availability` | availability.rs:20-27 |
| `mission_graph_nodes`/`edges` | `/graph/:mission_id` (**sin gate**, B1) | graph.rs:30 |
| `dir_access` | **ninguna ruta** (solo CLI `atlas swarm jump`) | la frecency de **misiones** (`/hud/missions?sort=frecency`) deriva de `journal_events`, no de `dir_access` (B4/H-01) |
| `artifacts` | `/hud/demos` | demos.rs:44 |

**Migraciones relevantes para la UI [O — comentarios + tests]:** M14 crea `agent_session_events` (+ índices ts/task, schema.rs:598-608); M15 crea `mission_graph_nodes`/`mission_graph_edges`/`learning_graphs` **unconditionally — sin feature flag — porque leer el grafo es barato aunque el writer esté gated** (schema.rs:642-646; el writer/DAG emitter es lo gated: `dag_mode`/`codebase-graph`); M19 crea `model_resets` (+ índices uniq/pending, 838-855); M21 crea `model_invocations` (988-1017); M31 crea `dir_access` (test `m31_advances_schema_version_and_creates_dir_access`); M49/M50 crean `agent_runs`/`agent_steps`/`tool_invocations`/`artifacts` (1767-1835); **M52 (v41) crea `task_annotations`** (+ índices task/author, schema.rs:1875-1886) — B8/H-05.

**Tests schema ejecutados [O — `cargo test --lib -- journal::tests mission_graph` → 100 passed]:** round-trips de verdicts/diffs/plans/repairs/patterns/skills/checkpoints/verdicts (idempotencia + tail newest-first), M14/M15/M19/M22/M25/M26/M27 schema tests (idempotencia de migración incluida), frecency (M31), eval, model_invocation means.

---

## (e) IMPLEMENTADO vs VALIDADO (por elemento)

Criterio: **VALIDADO** = IMPLEMENTADO + existe un test que lo ejerce **y lo ejecuté esta sesión** (nombre citado). **NO-VERIFICADO** = implementado sin test propio ejecutado.

| Elemento (fichero) | Estado | Test ejecutado [O] |
|---|---|---|
| `GET /hud/cost` (cost.rs:32) | IMPLEMENTADO + **VALIDADO**; incluye **`budget_usd`+`remaining`** desde `<profile>/budget_usd.txt` (B5) | `hud::cost::tests` (3: totals + budget presente + budget null) |
| `GET /hud/health` (health.rs:24) | IMPLEMENTADO + **VALIDADO** | `hud::health::tests::reports_event_counts_and_run_states` |
| `GET /hud/audit` (audit.rs:24) | IMPLEMENTADO + **VALIDADO** (solo camino vacío; sin verificación de cadena — declarado audit.rs:4-7) | `hud::audit::tests::empty_audit_log_returns_no_rows` |
| `GET /hud/worktrees` (worktrees.rs:27) | IMPLEMENTADO + **VALIDADO** (camino error) | `hud::worktrees::tests::non_repo_path_reports_ok_false_with_reason` |
| `GET /hud/demos` (demos.rs:38) | IMPLEMENTADO + **VALIDADO** | `hud::demos::tests::lists_artifacts_with_preview_url` + `extract_preview_url_only_accepts_http` |
| `GET /hud/mcp` + allowlist + probe (mcp.rs) | IMPLEMENTADO + **VALIDADO** | 7 × `hud::mcp::tests` (enrich, missing config, allowlist round-trip, probe refuses) |
| `POST /hud/skills/:id/activate` (skills.rs:44) | IMPLEMENTADO + **VALIDADO** (parse del body; el publish no tiene test de handler) | 2 × `hud::skills::tests` |
| `/hud/secrets` GET/POST/DELETE (secrets.rs) | IMPLEMENTADO + **VALIDADO** | `get_lists_the_catalog_with_present_flags_and_no_values`, `post_rejects_blank_input_before_touching_the_store` |
| `/hud/approvals/:id/{approve,deny}` (approvals.rs:128/146) | IMPLEMENTADO + **VALIDADO**; **persiste la decisión y el `reason`** (B3, `record_approval_decision`) | 2 × `hud::approvals::tests` (parse) |
| `POST /hud/approvals/batch` (approvals.rs:165) | IMPLEMENTADO + **VALIDADO** (B3) | 4 × `hud::approvals::tests` (happy + 409 fichero + 409 opuesta + 400) |
| `GET /hud/missions?sort=frecency` (missions.rs:67) | IMPLEMENTADO + **VALIDADO** (B4) | 2 × `hud::missions::tests` (frecency order + empty stable) |
| `POST /hud/missions` (missions.rs:32) | IMPLEMENTADO + **VALIDADO** (B7) | 2 × `hud::missions::tests` (creates + reject blank) |
| `GET /hud/agent/:run_id/snapshot` (snapshot.rs:44) | IMPLEMENTADO + **VALIDADO** (B6) | 3 × `hud::snapshot::tests` (frame + 404 unknown + 404 no-sandbox) |
| `POST+GET /task/:id/annotation` (annotate.rs:110/143) | IMPLEMENTADO + **VALIDADO** (B8) | 2 × `hud::annotate::tests` (task round-trip + 422) |
| `/tail/*` (tail.rs, 14 rutas) | IMPLEMENTADO + **VALIDADO** (clamps + 3 tails contra journal vacío) | 6 × `hud::tail::tests` |
| `/hud/journal` paginado (observer.rs:59) | IMPLEMENTADO + **VALIDADO** | 9 × `hud::observer::tests` (paginación, filtros, 400s) |
| `/diff/:id/annotation` (annotate.rs) | IMPLEMENTADO + **VALIDADO** | 4 × `hud::annotate::tests` |
| `/audit/export-posting` (export.rs:48) | IMPLEMENTADO + **VALIDADO** | 5 × `hud::export::tests` |
| `/autoresearch/cancel` (autoresearch.rs:49) | IMPLEMENTADO + **VALIDADO** | 7 × `hud::autoresearch::tests` |
| `/remote/status` (remote_status.rs:21) | IMPLEMENTADO + **VALIDADO** (modo informational; el enforce-bearer está tras `#[cfg(feature = "remote-ui")]`, remote_status.rs:109-132 — **NO-VERIFICADO con feature**) | `status_never_leaks_the_token` |
| `/hud/availability` (availability.rs:14) | IMPLEMENTADO + **VALIDADO** | `reports_policy_and_pending_mission` |
| `/hud/reliability` (reliability.rs:14) | IMPLEMENTADO + **VALIDADO** | `reports_policy_and_model_reliabilities` |
| `/hud/eval/summary` (eval.rs:22) | IMPLEMENTADO + **VALIDADO** | `summary_reports_seeded_cases` |
| `/payload/:kind/:id` (tail.rs:234) | IMPLEMENTADO + **VALIDADO** (vía round-trips journal de payloads) | journal::tests (100-test run) |
| `/payload/skill/:skill_id/:version` (tail.rs:261) | IMPLEMENTADO | [P] handler NO-VERIFICADO |
| `/health` (server.rs:180) | IMPLEMENTADO | NO-VERIFICADO (trivial) |
| `/ws` fan_out (ws.rs:52-76) | IMPLEMENTADO | **NO-VERIFICADO** — no existe `hud::ws::tests` [O: `--list`] |
| `/graph/:mission_id` (graph.rs:30, **sin gate** — B1) | IMPLEMENTADO (siempre montado) | **NO-VERIFICADO** por test de handler propio (schema `m15_*` sí) |
| `/atlas-calendar.ics` (cfg calendar-ics) | IMPLEMENTADO (feature-gated) | [P] NO-VERIFICADO |
| `BusEventKind` 26 eventos (bus.rs:17-189) | IMPLEMENTADO + **VALIDADO** (shape snake_case, tags estables) | 4 × `core::bus::tests` (`every_tag_is_lowercase_snake_with_no_spaces`, `mission_steered_tag_is_stable`, …) |
| `AgentStatus` 11 estados con **`Unknown`** (bus.rs:224-236) | IMPLEMENTADO + **VALIDADO** (B2: round-trip serde + rechazo de string inválido) | `core::bus::tests` |
| `KernelCommand` 8 comandos (bus.rs:249-276) | **DEAD CODE** [O: `#[allow(dead_code)]` bus.rs:248, grep = 1 match] | — |
| CLI 34 comandos (proto.rs:49-130) | IMPLEMENTADO + **VALIDADO** (parse de flags: 61 tests) | `cli::commands` (61-test run); `atlas agent` **NO-VERIFICADO** por test unitario (sin tests en cli::commands) [O: `--list`] |
| Journal: 62 tablas + migraciones (schema.rs) | IMPLEMENTADO + **VALIDADO** (round-trips + idempotencia M14-M27 + frecency M31 + `task_annotations` M52) | `journal::tests` + `mission_graph` (100-test run) |

---

## Cierre: capacidad → view RFC 67, y huecos H-01..H-09

### Tabla capacidad del backend → view o componente RFC 67 que la necesita

| Capacidad backend [O] | View / componente RFC 67 [O: §1.7, §3, §20] |
|---|---|
| `/ws` + `BusEventKind` (26 eventos) | Todo el HUD; `AgentCard` (§4, `agent_step`/`agent_status_changed`), Approvals Dock (§5, `approval_request`/`approval_decision`), Mission Rail (§2, `mission_*`), StatusBar (`hud.connected`) |
| `/tail/missions` + `missions` | Mission Rail (§2), Kanban (§6) |
| `/tail/agent_steps` + `agent_steps` | `AgentCard` (§4): steps + tool evidence (RFC 63) |
| `/tail/diffs` + `diffs` | Diff view (§7), Mission Rail cards |
| `/hud/cost` + `model_invocations` + `model_resets` | Cost & Res (§11): totals, by_model, pressure, **`budget_usd`/`remaining`** (B5, `<profile>/budget_usd.txt`) |
| `/hud/health` + `agent_session_events` + `agent_runs` + `swarm_agents` | Health KPIs (§12); heartbeat vía WS `agent_heartbeat` (§1.3) |
| `/hud/audit` + `audit_log` | Audit (§13) — sin verificación de cadena (declarado) |
| `/hud/worktrees` + `swarm/` | Worktrees (§14) |
| `/graph/:mission_id` (**sin gate**, B1) | Canvas (§8) — siempre legible; el fallback a Outline ya no depende de un feature flag |
| `/hud/journal` paginado | Timeline (§10), JournalObserver |
| `/hud/approvals/:id/{approve,deny}` + `/hud/approvals/batch` | Approvals Dock (§5) — batch 1-a-N con **409** por conflicto y **`reason` persistido** (B3) |
| `/diff/:id/annotation` + `/task/:id/annotation` | Diff comments (§7) y **task comments** (B8, tabla `task_annotations` M52) |
| `/hud/demos` + `artifacts` | Agent Card demos (§4) — **sin video/TTS (H-07, v2)** |
| `/hud/agent/:run_id/snapshot` + `agent_runs` | AgentCard (§4) frame/`👁 Reason` (B6) |
| `POST /hud/missions` | Mission Rail (§2) `+ New` (B7) |
| `/payload/:kind/:id`, `/payload/skill/:id/:version` | Drill-down de Outline (§9), Audit, skills |
| `/hud/eval/summary` + `eval_runs` | Eval (RFC 20 Fase 22 card) |
| `/hud/availability` + `proactive_policy` + `calendar_busy_windows` | Proactive card (RFC 20 Fase 23) |
| `/hud/secrets` GET/POST/DELETE | Settings (§15) |
| `/hud/mcp` GET + allowlist + probe | MCP view (§16) |
| `/hud/skills/:id/activate` | Skill & MCP rail (§16) |
| `/remote/status` | TopBar (§1.1), Settings (§15) |
| `/hud/reliability` | Reliability gate card (RFC 20 Fase 24) |
| `/hud/missions?sort=frecency` (deriva de `journal_events`) | Mission Rail (§2): orden por frecuencia; journal vacío ⇒ orden estable (B4) |
| `KernelCommand` (dead code) | **RFC 67 §1.7 los cita; no operativos** |

### Huecos H-01..H-09 — veredicto [O cada uno contrastado con código; estado tras FASE 10/11]

| # | Hueco (RFC 67 §20-A) | Veredicto | Evidencia |
|---|---|---|---|
| H-01 | Orden por **frecency** de misiones | **RESUELTO (backend):** `GET /hud/missions?sort=frecency` ordena por actividad derivada de `journal_events`; journal vacío ⇒ orden estable | `hud/missions.rs:67` + `journal/frecency.rs:96` (`mission_frecency`); tests `hud::missions::tests::{frecency_orders_missions_by_activity, empty_journal_keeps_a_stable_order}` (VALIDADO, B4) |
| H-02 | **batch/scope/pauserule** de aprobaciones | **RESUELTO (backend, batch):** `POST /hud/approvals/batch` 1-a-N con **409** por conflicto (fichero compartido / decisión opuesta previa / id duplicado); `reason` **persistido** (antes se descartaba) | approvals.rs:165 + `record_approval_decision` (journal/mod.rs:398); tests `hud::approvals::tests` ×4 (VALIDADO, B3). **`scope`/`pauserule`** siguen en v2 |
| H-03 | **`Unknown`** en `AgentStatus` | **RESUELTO:** variante `Unknown` añadida al enum + serde | bus.rs:235; test `core::bus::tests` (round-trip + rechazo string inválido) (VALIDADO, B2) |
| H-04 | **Fallback** de Canvas con `dag_mode` off | **RESUELTO:** la lectura `/graph/:mission_id` ya **no está gated** | server.rs:154 (B1); tablas `mission_graph_*` ya eran unconditional (schema.rs:642-646) |
| H-05 | Comments a nivel de **Task** | **RESUELTO (backend):** `/task/:id/annotation` + tabla `task_annotations` | annotate.rs:110/143 + schema.rs:1875 (M52); tests `hud::annotate::tests` task (VALIDADO, B8) |
| H-06 | **Budget restante** para BudgetBar | **RESUELTO (backend):** `/hud/cost` añade `budget_usd`+`remaining` desde `<profile>/budget_usd.txt` (ausente ⇒ `null`) | cost.rs:36/58-59/79-80; tests `hud::cost::tests` ×3 (VALIDADO, B5) |
| H-07 | **demo video/TTS** | **NO resuelto (v2):** declarado sin implementar; `/hud/demos` sigue cubriendo solo artefactos | demos.rs:4-6; `preview_url` solo http(s) (demos.rs:26-36) |
| H-08 | **snapshot de terminal** por agente | **RESUELTO (backend):** `GET /hud/agent/:run_id/snapshot` puentea `Sandbox::snapshot`; 404 `{reason}` si no hay frame | snapshot.rs:44; tests `hud::snapshot::tests` ×3 (VALIDADO, B6) |
| H-09 | **nueva misión por REST** | **RESUELTO (backend):** `POST /hud/missions` | missions.rs:32; tests `hud::missions::tests` ×2 (VALIDADO, B7) |

**Resumen [O tras FASE 10/11]:** **8/9 huecos resueltos por backend** (H-01, H-02, H-03, H-04, H-05, H-06, H-08, H-09); **H-07 (video/TTS)** queda en v2. Siguen en v2: `scope`/`pauserule` de aprobaciones (resto de H-02) y hot-swap/grafo de MCP/Worktrees. Hallazgo que la spec no vio: `KernelCommand` = dead code (§1.7 lo cita como contrato).
