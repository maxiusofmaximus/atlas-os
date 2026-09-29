# 40 - Roadmap v2 (plan refinado)

Plan atómico para el Roadmap v2 de Atlas OS — post-ROADMAP v1 COMPLETO (Phases 0-10). Orden decidido por el operador: Mobile → Distribución → Laya. Complementa RFC 38 (artemis lateral) y extiende los patrones validados en las Phases 2-10.

## SECTOR A — Decisiones ya tomadas (evidencia RFCs + terreno verificado)

### A.1 artemis lateral (RFC 38 §2-§3) — patrón 8.5 RustDesk
- `google/artemis` VERIFICADO (Apache-2.0, Python/uv, MCP nativo, 99%+ AndroidWorld): proceso EXTERNO solamente (RFC 25 §11 — jamás bundling Python).
- **Terreno verificado (Sep 2026)**: `uv` ✅ en PATH (vía hermes); `artemis`/`adb`/`scrcpy` NO en PATH → las laterales detectan y guían (patrón 8.5: "Sin RustDesk → mensaje útil").
- **El operador tiene dispositivo Android físico con USB debugging** — la validación del flujo artemis end-to-end se hace tras 11.0.

### A.2 Firma obligatoria (10.1) como base del marketplace git-based
- `skills/marketplace.rs` (`install_skill` firma OBLIGATORIA Missing/Mismatch → Forbidden + rollback): el install desde git RE-USA la firma — la red no cambia el posture de seguridad.

### A.3 Semgrep/CodeQL como proceso externo (RFC 20 Phase 9, research/37 SECTOR C)
- "Semgrep/CodeQL embebidos (proceso externo solo, jamás bundling)" — RFC 20 los pedía como Validation stages; la versión externa con detección en PATH es la v2.
- El parse del output → `AuditReport` (M32, sub-fase 9.0) — los findings machine-readable ya existen.

### A.4 Laya BLOQUEADO upstream (RFC 22 §7 AN-9.1)
- Criterio de re-audit: `laya` >0.2.x o mantenedores ≥2 o fix de rand 0.8/tokenizers — hasta entonces NO se delega.
- La gate `laya = []` vacía (9.1) ya tiene el shape fijo para el swap.

## SECTOR B — Plan refinado Roadmap v2 (3 fases, 5 sub-fases atómicas)

### Fase 11 — Mobile testing (artemis lateral 8.6)

#### Sub-fase 11.0 — `atlas mobile` (M41)
- `mobile/mod.rs` nuevo (patrón `remote/` 8.5): `find_artemis_in_path` (artemis CMD + `uv` ya en PATH) + `ATLAS_ARTEMIS_REPO` env (el repo clonado) + `ATLAS_ARTEMIS_BIN` override + `spawn_session(profile, task)` (`uv run artemis run "<task>" --profile flash|pro` via `std::process::Command`) + `setup_steps()` guía (clone + start.bat + USB debugging + `artemis mcp --install all`).
- CLI: `cli/commands/mobile.rs` + `atlas mobile status/guide/run [--profile flash|pro] [--task "<...>"]` — sin artemis/uv → mensaje útil con setup steps (jamás crash).
- Tests: detection determinista, failure-path sin artemis → mensaje, spawn_args shape fija, env override.
- Smoke: `atlas mobile status/guide` (sin dispositivo — mensajes útiles).

#### Sub-fase 11.1 — MCP wiring template (M42)
- `mobile/mcp_template.rs`: `mcp_template() -> String` — template JSON `.opencode/mcp.json` para artemis (los 5 tools tipados RFC 38 §2.1: `mobile_run_task`, `mobile_manage_task`, `mobile_get_device_state`, `mobile_inspect_trace`, `mobile_diagnose`) con el cwd del repo (`ATLAS_ARTEMIS_REPO`).
- CLI: `atlas mobile mcp-template` — imprime el JSON.
- Tests: template serde_json parse válido, determinista, tools tipados presentes.
- **Validación del operador (tras 11.0/11.1):** dispositivo físico con USB debugging — flujo artemis end-to-end (`atlas mobile run --task "..." --profile flash` + el MCP wiring en el IDE).

### Fase 12 — Distribución y red

#### Sub-fase 12.0 — Semgrep/CodeQL stages externos (M43)
- `validation/stages/static_analysis.rs` (gated `static-analysis` feature vacía? NO — proceso externo sin feature): `atlas validate --semgrep`/`--codeql` — `find_tool_in_path` (patrón mobile/remote) → lanza el binario externo si existe → parsea el output (JSON findings de semgrep / SARIF de codeql) → `AuditReport` (M32) → si no está en PATH → mensaje útil + skip fail-safe (el stage nunca bloquea por herramienta ausente).
- Tests: detection determinista, failure-path sin tool → skip, parse de output fixture → findings M32.
- Smoke: `atlas validate --semgrep` sin semgrep → mensaje útil.

#### Sub-fase 12.1 — Marketplace git-based (M44)
- `skills/marketplace.rs` extiende: `install_from_git(url, name, skills_root)` — git CLI (patrón swarm 4.0 `WorktreeManager` — `std::process::Command git clone --depth 1`) a temp dir + `install_from` (firma obligatoria 10.1 INTACTA) + limpia temp.
- CLI: `atlas skill install <name> --from <git-url>` (además del path actual).
- Tests: install desde git LOCAL fixture (repo temp con skill firmada), firma obligatoria intacta, failure-path repo inválido, temp limpiado.
- Marketplace git-based, sin servidor central (compartir por repos — SECTOR C).

#### Sub-fase 12.2 — Cleanup preexistente (M45)
- hud gating: `cfg(feature = "hud")` correcto en los módulos `hud/` que usan axum/tower_http sin gating (`--no-default-features` sin `hud` hoy falla con 37 errores — verificado en la sesión 9.4).
- Flaky test `orchestrator::aggregation::self_discover::tests::selfdiscover_first_call_creates_skeleton` — fix determinista o `#[ignore]` documentado.
- ort-sys ICE con `cargo test --all-features` — documentado (toolchain rustc 1.96 Windows, no fixeable desde el repo — NOTA en README/AGENTS).
- Tests: `--no-default-features --features "tauri,cli"` compila, suite verde estable (3 runs).

### Fase 13 — Laya real (BLOQUEADO upstream — solo cuando el crate madure)
- **Criterio de re-audit:** `laya` >0.2.x o mantenedores ≥2 o fix de rand 0.8/tokenizers 0.21.
- Entonces: real candle inference detrás de la gate `laya` (swap sin cambiar callers — 9.1) + compaction wiring (5.3, weak_model → Laya) + winnow (tool-result judging).
- **NO delegar hasta que el criterio se cumpla** — el gestor re-audita periódicamente.

**Entregable:** testing Android real + distribución git-based + validación estática externa (RFC 20 v2). KPI: latencia UI <100ms, alucinaciones ≤1/100 diffs.

## SECTOR C — Fuera de alcance v2

- iOS (artemis NO lo ha shippado — está en SU roadmap; imposible hasta entonces).
- IDE distribuido multi-usuario, modelos propios entrenados desde cero, hardware de inferencia dedicado (RFC 20 out of scope → v2, sigue).
- HTTP registry (v3 si el git-based se queda corto).
- Marketplace remoto con red HTTP (el git-based cubre el compartir por repos).

## SECTOR D — Siguiente paso operativo

1. Commit de este plan + RFC 20 (Fases 11-13) + Index 26.
2. Sub-fase 11.0 (delegada a muse-spark-1.3, zero-dep) → revisar → commit → push.
3. Sub-fases 11.1, 12.0, 12.1, 12.2 (todas zero-dep, patrón std-only/lateral).
4. **Validación del operador**: dispositivo Android físico con USB debugging — flujo artemis end-to-end tras 11.0/11.1.
5. Phase 13 solo cuando el criterio de re-audit de laya se cumpla.
