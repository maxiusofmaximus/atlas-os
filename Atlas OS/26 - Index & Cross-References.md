# 26 - Index & Cross-References

> Mapa maestro de la especificación Atlas OS (AEOS v1). Lista los 27 RFCs (`00`–`26`) con un resumen de una línea, dependencias (estrictas y suaves), two-way cross-references, rutas de lectura recomendada según rol, y un grafo de dependencias en ASCII para navigate. Pensado para que un recién-llegado encuentre qué RFC responder a qué pregunta, y para que cualquier modificación futura de un RFC identifique en segundos qué otros RFCs se ven afectados.

---

## 1. Catálogo completo

| # | RFC | Tamaño | Resume |
|---|---|---|---|
| 00 | [Vision](./00%20-%20Vision.md) | 3.1 KB | Tesis fundacional: Atlas OS como plataforma para agentes, no editor con IA. Define Nombre (Atlas OS), nombre técnico (AEOS), versión (v1.0), licencia, diferencia fundamental vs Cursor/Windsurf/Claude Code/Hermes/OpenClaw. |
| 01 | [Core Principles](./01%20-%20Core%20Principles.md) | 3.4 KB | 10 principios rectores: transparencia > magia, multi-modelo, distribución, verificabilidad, memoria persistente, auto-evolución, seguridad sandbox, of-architecture, agnostic de provider, open-source. |
| 02 | [Agent Operating System](./02%20-%20Agent%20Operating%20System.md) | 10.3 KB | Kernel: Kernel Bus, Scheduler, Capability Resolver, Política de ejecución (auto/confirm/forbidden), 4 Execution Modes × 3 Resource Modes, evento flow, SOP con 4 checkpoints, patrón at-least-once idempotente con `idempotency_key` (patrón Temporal). |
| 03 | [Engine Architecture](./03%20-%20Engine%20Architecture.md) | 8 KB | Topología de 10 motores: Context, Planning, Research, Reasoning, Coding, Validation, Repair, Learning, Model Orchestrator, Swarm. State machine formal del Execution Supervisor en §9 con DoomLoopDetector, GoalTracker, LanguageIdResolver, ProjectSymbolTable, SemanticEmbeddingIndex. |
| 04 | [Model Orchestrator](./04%20-%20Model%20Orchestrator.md) | 6.1 KB | Routing por skill graph + tier + costo. Registry de modelos con protocol, capabilities, api_keys multi-key (rotación rigurosa AionUI-style), runtime_options, capability_tags. Cascada cross-provider con budget enforcement. Lección AionUI anotada. |
| 05 | [Swarm](./05%20-%20Swarm.md) | 4.9 KB | Roles (Planner, Researcher, Architect, Backend, Frontend, Database, Security, Testing, Reviewer, Merger). Topología distribuida, asignación de modelo por rol, worktree own por subagent, pool swarm estilo Kimi, subagentes locales ociosos consumidos automáticamente. |
| 06 | [Skills](./06%20-%20Skills.md) | 5.3 KB | Skill Graph, iluminación/grisado desde Learning, skills auto-compresoras, marketplace con firma obligatoria. Resolución de conflictos. Soporta shell, TS, Rust, Python, Go. |
| 07 | [MCP](./07%20-%20MCP.md) | 3.7 KB | Bridge stdio JSON-RPC para MCPs externos. Multi-MCP con streaming y verificación de firma. |
| 08 | [CLI](./08%20-%20CLI.md) | 2.6 KB | `atlas` CLI: `mission new`, `run`, `resume`, `fork`, `steer`, `swarm` family, `research query`, `research feasibility`, `audit tail`, `audit verify-hash-chain`, `profile switch`, `mcp refresh`. |
| 09 | [Vector Knowledge](./09%20-%20Vector%20Knowledge.md) | 2.9 KB | Vector KB con tiered storage (core/archival/recall, patrón Letta MemGPT). |
| 10 | [Research Engine](./10%20-%20Research%20Engine.md) | 9.2 KB | Research pregunta en internet antes de decisiones críticas. Fuentes 11 categorías. Collective Engineering Intelligence. §11: `probe_feasibility(topic, domain)` para capacity_hallucination / unknown_tool_dependency, consume los 10 `gap_type` de `23`. |
| 11 | [Context Engine](./11%20-%20Context%20Engine.md) | 3.8 KB | Project Map, Tree-sitter, reordenación "Lost in the middle" (`23 §8 context mode`). |
| 12 | [Planning Engine](./12%20-%20Planning%20Engine.md) | 5.1 KB | Input: `MissionConsolidated` (no raw prompt). Output: Plan con milestones, strategies (TDD/strangler/big bang/incremental/pair). Anti-gate: no Coding Engine si `Plan.confidence < 0.7` o si `verdict.confidence < HIGH` sin lock. |
| 13 | [Coding Engine](./13%20-%20Coding%20Engine.md) | 3 KB | Diff single/multi-hunk, herramientas de edit, file locks. |
| 14 | [Validation Engine](./14%20-%20Validation%20Engine.md) | 3.1 KB | Stages incremental: tsc, Biome, Vitest, Knip, Semgrep optional. §10 evidence-gated done (Canny): stage terminal `EvidenceGate` + hook `evaluate_done_claim` + `DoneClaimed`/`BlockDone` (RFC 30 §2.1). |
| 15 | [Repair Engine](./15%20-%20Repair%20Engine.md) | 2.2 KB | Diagnóstico automático a partir de `validation.failed`. |
| 16 | [Learning Engine](./16%20-%20Learning%20Engine.md) | 2.9 KB | Reflexión formal, extracción de reglas, `was_correct` por decision. Alimenta el affinity del Model Registry (`04`) y el iluminado del Skill Picker (`17`). Memories tiers (Letta pattern). |
| 17 | [UI](./17%20-%20UI.md) | 7.1 KB | Layout base (ModeBar + Editor + Command Center + Agent Console). Modo Manual vs IA. Mission Control referente Cursor Desktop + OpenCode sessions. Execution Mode selector 🕊/🤝/🛫/🚀. Acceso web remoto. Comments in-editor. Modo noob. Atajos `:c :p :s :r :v :o`. *Es remplazado por el HUD Mission Control (`24`), pero queda como layout base del editor de texto.* |
| 18 | [Security](./18%20-%20Security.md) | 4 KB | Sandbox levels: vuOnly/container/wasm. Firmas de skills con SHA-256 + minisign. SensibleActions. OWASP/GDPR/HIPAA skills. |
| 19 | [Execution Supervisor](./19%20-%20Execution%20Supervisor.md) | 6.3 KB | Heartbeats, checkpoints SQLite, reanudación desde cualquier estado. Política anti-infinite-loop mecánica con `doom_loop` hard-deny, `caps`, `goal_drift_decay`, `recovery`, `escape_hatch`, `audit`. State machine formal. |
| 20 | [Roadmap](./20%20-%20Roadmap.md) | 3.9 KB | Fase 0 → 10. Cada fase produce sistema funcional más ambicioso. KPIs: confidence ≥ 0.75, alucinaciones ≤ 1 / 100 diffs, latencia HUD < 100ms. |
| 21 | [Execution Modes](./21%20-%20Execution%20Modes.md) | 15.9 KB | Los 4 modos: `MANUAL_CLASSIC` (LSP IntelliSense), `HUMAN_IN_LOOP` (difure que el humano apruebe), `AUTOPILOT` (checkpoints cada N), `AUTONOMOUS` (loop con budget hard). Defaults por tipo de archivo. §12 añade eje ortogonal `modo de uso` (ask/architect/code/context). |
| 22 | [Research Findings](./22%20-%20Research%20Findings.md) | 22.8 KB | Backup documental: Hermes (Nous), AionUI (Apache 29.3k★), Cursor Cloud Agents, OpenCode Desktop, editores clásicos (VS/NetBeans/VS Code LSP), `/loop /goal`, anti-infinite-loop, 30+ orquestadores analizados (Round 2), 10 papers de Prompt Understanding, 10 lecciones UI. |
| 23 | [Prompt Understanding & Refinement](./23%20-%20Prompt%20Understanding%20&%20Refinement.md) | 17.6 KB | Pipeline de 9 pasos (capture → parse ToT → detect gaps → search similar → K STORM → auto-resolve → Self-Refine → loop → lock). 10 `gap_type` catalogados. `PublicUnderstandingVerdict` + `MissionConsolidated` tipos canónicos. Política de confidence. Comando `/refine`, skill `prompt-clarify`. Modos `ask`/`architect`/`code`/`context`. Citas académicas (Self-Refine, Reflexion, ToT, RePrompt, ODUTQA-MDC, LLM-Modulo, LLM-as-judge, Lost in the Middle, STORM, Masterman). |
| 24 | [HUD Mission Control](./24%20-%20HUD%20Mission%20Control.md) | 35.9 KB | Panel HUD. Layout maestro + 8 views (Kanban/Canvas/Outline/Timeline/Cost&Res/Health KPIs/Audit/Worktrees). Tarjeta por subagent con 15+ campos. WebSocket events tipados. Approvals queue con batch/scope/pauserule. Skill/MCP drag-drop en caliente. Audit timeline append-only hash-chained. Demos over diffs estilo Loom. Worktrees visuales. Mobile OIDC. Reporter de approve. Integra Hermes HUD + Jira/Linear + Cursor + Notion + n8n. |
| 25 | [Stack Técnico Multiplataforma](./25%20-%20Stack%20T%C3%A9cnico%20Multiplataforma.md) | 25.9 KB | Tauri 2 + Rust 1.84+ + SvelteKit 2 + SQLite/`sqlite-vec` + `fastembed-rs` + Tower-LSP + axum WebSocket + sandbox Docker/Podman/Firejail/Job-Object + OS keychain + 13 model providers HTTP (3 locales + 10 free cloud + paid tier) + `atlas` CLI Rust. Multi-OS binario único 30-45 MB. Justificación vs Electron/AionUI. Tamaños y performance budgets. |
| 26 | [Index & Cross-References](./26%20-%20Index%20&%20Cross-References.md) | este archivo | Mapa maestro. |
| 27 | [Orchestration Fundamentals](./27%20-%20Orchestration%20Fundamentals.md) | 13.5 KB | Audit comparativo vs `tmux-orchestrator`, `orca`, `herdr`, `traycer`. Extrae 6 principios fundacionales (worktree isolation, at-a-glance state, remote attach, reflexivity, BYOA, spec-first) y cataloga 8 brechas (A–H) contra los fundamentos, con priorización Phase 2+. |
| 28 | [External Tool Integration](./28%20-%20External%20Tool%20Integration.md) | 98.7 KB | 8 superficies: §A Karpathy autoresearch loop (Hill-climbing con métrica medible, M13 `autoresearch_runs`); §B Microsoft Intelligent Terminal ACP server (M14 `agent_session_events`, slash commands `/opencode fix` `/restart`); §C graphify pattern adoption (M15 `mission_graph` + `petgraph`+`tree-sitter`, `<GraphView>` HUD, skills como graph templates, Planner DAG); §D AuditLog YAML-on-disk export formato posting; §E Firecrawl: web ingestion polyfacética (adapter facade, post-graphify); §F Windows Toast Notifications (AUMID + Start Menu shortcut via `winrt-toast-reborn`, `tokio::sleep_until` SQLite-driven scheduler, `on_activated` deep-link); §G Windows Calendar (`.ics` feed via axum HUD `GET /atlas-calendar.ics` + Microsoft Graph `/me/calendarView` reader via `graph-rs-sdk` `interactive-auth`, refresh token SQLite encrypted); §H Model API reset-window notifications (OmniRoute envelope parsing, `SpendLimitError` card port de Cline PR #10207, `model_ready` Toast — feature diferencial frente a competencia). Fase 1.5 orden §D→§A→§C→§B→§E→§F→§G→§H. **Status Phase 1.5: §D ✅ (`db25379`), §A ✅ (`853da30`), §C ✅ (`cddcbc2`), §B ✅ (`7f8215e`–`4b4924e` items 1-8 todos ✅), §E ✅ items 1-8 (`c228e4a` — adapter facade + CLI research subcommand, items 9-10 post-MVP), §F ✅ items 1-8 (`d6e6e23` — Toast queue + driver + CLI subcommand + M17 schema + AppState integration, item 9 smoke post-MVP), §G ✅ items 1-4 (`8d26528` — WRITE path; items 5-8 READ diferidos a Phase 2), §H ✅ items 1-11 (M19 + `SpendLimitError`/`ResetKind`/`parse_omniroute`/`RetryPolicy`/`handle_spend_limit_error`/`cards.rs`/`SpendLimitErrorCard.svelte`/`ModelReadyCard.svelte`/`Profile` bail-out + backup; items 12-13 pending docs/smoke).** Research internos en `Atlas OS/research/` (`27 - *` 3 docs, `28 - portable inventory.md`, `28 - conductor & alt surfaces.md` — Conductor analysis + terminal-UI comparison + remote-live dual-PC roadmap, `29 - Phase 2 model orchestrator.md` — plan refinado Fase 2 Multi-model Orchestration con evidencia primaria). |

| 29 | [Genspark AI Integration](./29%20-%20Genspark%20AI%20Integration.md) | 26.2 KB | Audit comparativo de Genspark AI (MainFunc, $100M Series A) como orquestador de agentes comerciales: Genspark Claw (AI employee persistente multi-canal), AI Workspace 6.0 (super-app 80+ herramientas), Super Agent / MoA (multi-model default). 6 brechas A–F con priorización por dependencia (F/E narrativa → D bundled skills → C user modeling → A cloud serve → B multi-canal) + interpretación de su UI (§4) + fuentes de auditoría (§9). Informativo + priorización — input de Phase 2+. |
| 30 | [Ecosystem & Jev Audit](./30%20-%20Ecosystem%20%26%20Jev%20Audit.md) | ~12 KB | Audit del ecosistema de referencia: 20 repos "Jev" puntuados (script Context7Max) — "System One judgments" (modelo pequeño/barato para decisiones rápidas), per-turn routing, evidence-gated done, context sieve, compaction decisions, generative UI. + 23 referencias externas con destino por frente (Context7Max/anydoc → Phase 3, taste/design-md/archify → bundled skills, agency-agents/munder-difflin → Phase 4 Swarm). Priorización por dependencia — input de Phase 3+. |
| 35 | [Ecosystem Round 7 Audit](./35%20-%20Ecosystem%20Round%207%20Audit.md) | ~14 KB | Round 7: Symlink toolset (mklink /j built-in + Junction Sysinternals — skills dir linking, portable inventory, sandbox npm), cloudflare/security-audit-skill (6 fases + 12 hunting classes + findings.json schema + validator zero-dep), mksglu/context-mode (FTS5/BM25 sandbox + session memory 26 categories — FTS5 search sobre journal_events + ContextBudget), ajeetdsouza/zoxide (frecency aging+ranking — navegación interna propia, crate rechazada 22 deps) + 12 hallazgos deep search multi-agente (22 URLs) + §7.1 Laya (open source de Jev — `laya = "0.1.1"` ModernBERT + RL decision head, stack candle; el panorama "System One" completo con las 7 piezas). Priorización Phase 8/9/10. |
| 38 | [Ecosystem Round 8 Audit](./38%20-%20Ecosystem%20Round%208%20Audit.md) | ~8 KB | Round 8: `google/artemis` (VERIFICADO — Android automation por lenguaje natural, MCP server nativo `mobile_run_task`/`mobile_diagnose` + CLI `--profile flash/pro` + Python SDK `artemis-client`, 99%+ AndroidWorld, Apache-2.0, Python/uv — integración LATERAL proceso externo patrón 8.5 RustDesk, jamás bundling RFC 25 §11; `atlas mobile` como opción lateral 8.6; patrones Flash/Pro + history compression + Safety Net ya materializados en Atlas OS). Phase 10 en orden del roadmap. |
| 40 | [Roadmap v2 (plan refinado)](./research/40%20-%20Roadmap%20v2%20%28plan%20refinado%29.md) | ~8 KB | Roadmap v2 — orden Mobile → Distribución → Laya: Fase 11 (artemis lateral 8.6: `atlas mobile` status/guide/run + MCP template, validación operador con dispositivo físico), Fase 12 (Semgrep/CodeQL stages externos + marketplace git-based `--from <git-url>` + cleanup preexistente), Fase 13 (Laya real BLOQUEADO — criterio de re-audit >0.2.x/mantenedores). Out of scope: iOS, multi-usuario, HTTP registry. |
| 41 | [Ecosystem Round 9 - MaxAppsHub + Tauri Android](./41%20-%20Ecosystem%20Round%209%20-%20MaxAppsHub%20%2B%20Tauri%20Android.md) | ~8 KB | Round 9: `maxiusofmaximus/MaxAppsHub` (launcher Android del operador — `AppRegistry.kt` `ManagedApp` {packageId, githubOwner, githubRepo}, actualizaciones automáticas vía GitHub API Releases, APK firmado v1.0.2) + toolchain Android verificado (JDK 17 Adoptium, SDK android-36 + NDK 29, rustup targets aarch64/armv7/x86_64, TECNO KI7 conectado) + Fase 14: Tauri Android build → GitHub Release → MaxAppsHub entry → artemis testing de la app Atlas Android. |
| 42 | [Phase 16 — Laya re-audit + cierre Roadmap v2](./research/42%20-%20Phase%2016%20Laya%20re-audit%20%2B%20roadmap%20v2%20closure.md) | ~4 KB | Phase 16: re-audit fechado de `laya` (0.1.1, 1 contributor, tokenizers 0.21, sin rand directo) → criterio de desbloqueo NO cumplido → Phase 13 permanece BLOQUEADA; Roadmap v2 exhausted (11 ✅/12 ✅/14 ✅/15 ✅); Roadmap v3 requiere decisión del operador. |
| 43 | [Phase 17 — Baseline + higiene de lint + versión en build](./research/43%20-%20Phase%2017%20baseline%20hardening%20%2B%20version%20build%20evidence.md) | ~3 KB | Phase 17: `pnpm lint` a verde (prettier de workflow + dep-cruiser, `.prettierignore` para working files del operador), evidencia del header HUD `v0.1.1` en el bundle, baseline `cargo test --lib` 1103 ok. |
| 44 | [Phase 18 — ACP real host loop](./research/44%20-%20Phase%2018%20ACP%20real%20host%20loop.md) | ~3 KB | Phase 18: closeout Phase 1.5d — `session/prompt` despacha `delegate::DelegateOutcome` vía `PromptPlan` (exec step → CLI real, Fix/Restart explicados, NotSupported → Refusal), cwd por sesión, cancel simplificado documentado. |
| 45 | [Phase 19 — LSP real server](./research/45%20-%20Phase%2019%20LSP%20real%20server.md) | ~3 KB | Phase 19: `AtlasLspBackend` con initialize/hover/did_open sobre stdio (pipe o `ATLAS_LSP_STDIO=1`); desktop preserva park; stub placeholder eliminado. |
| 46 | [Phase 20 — Journal refactor spec + steps 1-4](./research/46%20-%20Phase%2020%20journal%20refactor%20spec%20%2B%20step1.md) | ~4 KB | Phase 20 (CERRADA): split de `journal/mod.rs` y `tests.rs` sin romper tests — Section H → `model_resets_ops.rs`, `dag_mode` → `dag_mode_ops.rs`, `ModelInvocationRow` + record/read → `model_invocation.rs`, `tests.rs` → `tests/` (15 módulos); 228 journal / 1106 lib intactos. |
| 47 | [Phase 21 — Audit panics/unwraps test-only](./research/47%20-%20Phase%2021%20audit%20panics%20unwraps%20test-only.md) | ~2 KB | Phase 21: las coincidencias de `panic!`/`unwrap()` están todas en bloques `#[cfg(test)]`; cancelada como cambio de producción. |
| 48 | [Phase 18.1 — ACP cancel in-flight](./research/48%20-%20Phase%2018.1%20ACP%20cancel%20in-flight.md) | ~3 KB | Phase 18.1: `exec step` corre en `run_until_cancelled`; `$/cancel_request` → `StopReason::Cancelled` (mandato ACP); 44 tests acp verdes. |
| 49 | [Phase 13 — Laya real RFC propuesta](./research/49%20-%20Phase%2013%20Laya%20real%20RFC%20proposal.md) | ~2 KB | Phase 13 (PROPUESTA, sin implementar): re-audit 2026-10-03 con 2 contributors cumple el eje mantenedores≥2 (caveat: write-access sin confirmar); plan de swap de la gate `laya`. |
| 50 | [Roadmap v3 (plan)](./research/50%20-%20Roadmap%20v3%20(plan).md) | ~6 KB | Roadmap v3: Windows Calendar real (RFC 28 §G). v3.1.0 ICS WRITE ✅, v3.1.3 CLI ✅, v3.1.1 Graph READ ✅ (auth-code+PKCE loopback, `atlas calendar login/sync/status`), v3.1.4 ICS READ ✅ (`sync-ics`, parser `icalendar 0.17`, `22 §15`) y v3.1.A.2 subscriptions durables ✅ (`subscribe/unsubscribe/sync-all`, migración 34, namespacing `{name}:{uid}`); falta v3.1.A.3 (poller) y v3.1.2 (bloqueado en el motor proactivo). |

**Total: 33 RFCs, ~331 KB** de especificación.

---

## 2. Grafo de dependencias estrictas

```
                                    00 Vision
                                       │
                                       ▼
                                    01 Principles
                                       │
                            ┌──────────┼──────────┐
                            ▼          ▼          ▼
                         18 Security   02 AOS     20 Roadmap
                                       │
                                       ▼
                                    03 Engines ─────────┐
              ┌────────────────────┬─────────────────┐  │
              ▼                    ▼                 ▼  ▼
          11 Context            04 Model Orch        14 Validation
              │                    │                    │
              │                    ▼                    │
              │                 05 Swarm                 │
              │                    │                    │
              ▼                    ▼                    ▼
          12 Planning ◀──── 23 Prompt Understanding  15 Repair
              │                                           │
              ▼                                           ▼
          10 Research ──▶ 13 Coding ◀────────────── 16 Learning
              │
              ▼
       Vector KB ─────▶ 09 Vector Knowledge
                            │
                            ▼
                       06 Skills
                            │
                            ▼
                       07 MCP
                            │
                            ▼
                       08 CLI
                            │
                            ▼
                       17 UI ──────▶ 24 HUD ──▶ 25 Stack
                            │           │           │
                            └─► 19 Execution Superv ◄┘
                            │
                  └─► 21 Execution Modes
                            │
                            └─► 22 Research Findings (backup)
```

### 2.1 Dependencias estrictas (`must exist before`)

| RFC | Depende estrictamente de |
|---|---|
| 02 | 03, 18, 19, 23, 24, 25 |
| 03 | 09, 19 |
| 04 | — (puede leerse aislado) |
| 05 | 03, 04, 24 |
| 06 | 14, 16, 18 |
| 07 | 18 |
| 08 | 02, 10, 18, 23, 25 |
| 09 | — (puede leerse aislado) |
| 10 | 02, 11, 23 (vía `probe_feasibility`) |
| 11 | 03, 09 |
| 12 | 02, 10, 23 |
| 13 | 03, 14 |
| 14 | 03 |
| 15 | 03, 14 |
| 16 | 03, 14, 05 |
| 17 | 02, 21, 24 |
| 18 | 01 |
| 19 | 02, 03, 21 |
| 20 | 02 (implícitamente todos) |
| 21 | 02, 17, 19, 23, 24 |
| 22 | — (es backup documental) |
| 23 | 02, 10, 12, 14, 19, 21, 24 |
| 24 | 02, 05, 10, 12, 16, 17, 18, 19, 21, 23, 25 |
| 25 | 02, 05, 07, 08, 09, 10, 11, 17, 18, 19, 23, 24 |
| 26 | todos |

### 2.2 Dependencias suaves (referencia no estricta)

| RFC | Referencia suave (no bloqueante) |
|---|---|
| 04 | 06 (sugiere skills), 22 (lecciones AionUI) |
| 05 | 06, 07, 16, 22, 24 |
| 06 | 08 (CLI instala skills) |
| 08 | 23 (comando `/refine`) |
| 11 | 03, 25 (Tower-LSP) |
| 16 | 09 (Vector KB persiste reglas) |
| 17 | 02 (modos), 21 (selector), 23 (mission panel), 24 (HUD reemplazo) |
| 19 | 21 (modo AUTONOMOUS), 25 (SQLite persistencia) |
| 20 | todos (es el plan de implementación) |
| 22 | todos (es el respaldo documental) |

---

## 3. Two-way cross-references

Tabla bidireccional. Si el RFC A referencia al RFC B, B también aparece listado en A. Útil para rastrear el impacto de un cambio.

| RFC A | Referencias hacia | RFC B |
|---|---|---|
| 02 | → | 03 |
| 02 | → | 05 |
| 02 | → | 18 |
| 02 | → | 21 |
| 02 | → | 23 |
| 02 | → | 24 |
| 02 | → | 25 |
| 03 | → | 19 |
| 03 | → | 09 |
| 03 | → | 02 |
| 04 | → | 22 |
| 04 | → | 06 |
| 04 | → | 16 |
| 05 | → | 03 |
| 05 | → | 04 |
| 05 | → | 24 |
| 06 | → | 14 |
| 06 | → | 16 |
| 06 | → | 18 |
| 07 | → | 18 |
| 08 | → | 02 |
| 08 | → | 23 |
| 10 | → | 02 |
| 10 | → | 11 |
| 10 | → | 23 |
| 11 | → | 03 |
| 11 | → | 09 |
| 11 | → | 25 |
| 12 | → | 02 |
| 12 | → | 10 |
| 12 | → | 23 |
| 13 | → | 03 |
| 13 | → | 14 |
| 14 | → | 03 |
| 15 | → | 03 |
| 15 | → | 14 |
| 16 | → | 03 |
| 16 | → | 05 |
| 16 | → | 14 |
| 17 | → | 02 |
| 17 | → | 21 |
| 17 | → | 24 |
| 19 | → | 02 |
| 19 | → | 21 |
| 19 | → | 25 |
| 20 | → | 02 |
| 21 | → | 02 |
| 21 | → | 17 |
| 21 | → | 19 |
| 21 | → | 23 |
| 21 | → | 24 |
| 22 | → | (todos los referenciados en §1, §8, §9) |
| 23 | → | 02 |
| 23 | → | 10 |
| 23 | → | 12 |
| 23 | → | 14 |
| 23 | → | 19 |
| 23 | → | 21 |
| 23 | → | 24 |
| 24 | → | 02 |
| 24 | → | 05 |
| 24 | → | 10 |
| 24 | → | 12 |
| 24 | → | 16 |
| 24 | → | 17 |
| 24 | → | 18 |
| 24 | → | 19 |
| 24 | → | 21 |
| 24 | → | 23 |
| 24 | → | 25 |
| 25 | → | 02 |
| 25 | → | 05 |
| 25 | → | 07 |
| 25 | → | 08 |
| 25 | → | 09 |
| 25 | → | 10 |
| 25 | → | 11 |
| 25 | → | 17 |
| 25 | → | 18 |
| 25 | → | 19 |
| 25 | → | 22 |
| 25 | → | 23 |
| 25 | → | 24 |
| 28 | → | 04 |
| 28 | → | 06 |
| 28 | → | 19 |
| 28 | → | 20 |
| 28 | → | 22 |
| 28 | → | 24 |
| 28 | → | 25 |

---

## 4. Rutas de lectura por rol

Cada rol debe leer los RFCs en este orden recomendado:

### 4.1 Arquitecto / Tech Lead
Lectura completa, en profundidad: `00 → 01 → 02 → 03 → 04 → 05 → 12 → 23 → 24 → 19 → 21 → 20`.

### 4.2 Ingeniero Backend (model orchestration)
`02 (§3 Kernel, §3.1 SOP) → 03 → 04 → 25 → 08` para levantar el core Rust.

### 4.3 Ingeniero Frontend (HUD)
`24 → 17 → 25 §3.3 (SvelteKit) → 02 §3.1.1 (eventos hud.*)` para levantar el HUD Mission Control.

### 4.4 DevSecOps / SRE
`01 → 18 → 19 → 25 §3.7 (Sandbox) → 02 §6 (Casos de fallo)`.

### 4.5 Investigador
`22 (todo el documento) → 10 → 23 → 03 §9 (state machine) → 19`.

### 4.6 Product manager
`00 → 20 → 21 → 02 §5 (Modes matrix) → 24 §1 (Layout)`.

### 4.7 Recién-llegado general
`00 → 02 → 20 → 21 → 24 → 25` (ruiseñor + tesis: por qué sistema, cómo funciona, cuál es el plan, qué modos, cómo se ve, sobre qué stack). En segundo pase, `23 → 19 → 05 → 04`.

---

## 5. Lista de conceptos y su RFC

Para ubicar dónde definir/clavar un concepto sin buscar desde cero.

| Concepto | RFC |
|---|---|
| `Agent Operating System` | 02 |
| `ACP server frontends the Orchestrator` (CLI / HUD / ACP trio) | 04 §9; 28 §B item 6 |
| `Approvals queue` pauserule | 24 §5 |
| `Architecture Memory` | 02 §3.4 |
| `Audit timeline hash chain` | 24 §10 |
| `Audit YAML export (posting-format)` | 24 §10.1; 28 §D |
| `Autoresearch loop` (Karpathy hill-climbing) | 19 §11; 28 §A |
| `Autoresearch cancel HUD endpoint` | 28 §A (Phase 1.5b §A-3) |
| `BgeBaseEnV15` embeddings | 25 §3.5 |
| `BackPressure` (semáforo per-provider) | 04 §6; 04 §Apéndice 2.0.5 |
| `CachePolicy` (Anthropic ephemeral breakpoints) | 04 §6; 04 §Apéndice 2.0.5 |
| `Cooldown` per-provider + Retry-After | 04 §6; 28 §H; 04 §Apéndice 2.0.5 |
| `ProviderWire` (serde-safe wire enum) | 04 §1; 04 §Apéndice 2.0 |
| `Capabilities` del Registry | 04 §3 |
| `Capability Resolver` | 02 §3.3 |
| `Card types (agent/autoresearch/graph)` | 24 §3.3; 28 §A (autoresearch live), 28 §C (graph live) |
| `Checkpoints SOP` | 02 §3.1.1 |
| `Clarification Questions` (STORM) | 23 §5 |
| `Command Center` (legacy) | 17 §1 (remplazado por HUD) |
| `Compaction` | 21 §4.4 |
| `Confidence Score` | 03 |
| `Confidence thresholds (HIGH/MED/LOW/BLOCK)` | 23 §5 |
| `Consolidated mission` | 23 §4 |
| `Context Engine` | 11 |
| `Cosine vs dot-product` | 09 |
| `Cross-provider cascade` | 04 |
| `Cube mode` (CodeMirror) | 25 §3.3 |
| `Demos over diffs` | 24 §9 |
| `DoomLoopDetector` | 19; 03 §9.1; 21 §5.3 |
| `Evidence-gated done` (Canny: no "done" sin evidencia, solo hechos bloquean) | 14 §10; 30 §2.1 |
| `DoneClaim` / `DoneGateVerdict` / `evaluate_done_claim` / `collect_diff_evidence` | 14 §10 |
| `EvidenceGate` (stage terminal del pipeline) | 14 §1; 14 §10 |
| `AuditReport` (`findings: Vec<SecurityFinding>`, `from_json_str`/`to_json_string_pretty`/`from_evidence`, M32) | 14 §10; 35 §3; research `37` sub-fase 9.0 |
| `SecurityFinding` (`id`/`severity`/`title`/`file`/`line`/`evidence`/`remediation`) + `Severity` (`info`/`low`/`medium`/`high`/`critical`) | 14 §10; 35 §3; research `37` sub-fase 9.0 |
| `validate_report` (ids únicos, title/file/evidence/remediation no vacíos, line > 0) + `atlas audit validate <file.json>` + `atlas audit --json` | 14 §10; 08 CLI; research `37` sub-fase 9.0 |
| `DoneClaimed` / `BlockDone` (supervisor) | 19 §6.1; 14 §10 |
| `dag_mode` feature flag (Planner DAG + State DAG + skills graph templates) | 12 §3.1; 19 §6.1.1; 23 §7.3; 28 §C |
| `Edge-case tool` feasibility probe | 10 §11 |
| `Execution Modes` (4) | 21 |
| `Execution Journal` | 02 §3.4; 19 |
| `Fastembed-rs` | 25 §3.5 |
| `Firecrawl` (web ingestion polyfacética, adapter facade) | 28 §E; 22 §11 (Round 4) |
| `fork` (subagent) | 24 §3.2 |
| `Free providers` | 25 §3.8; 21 §2.1 |
| `Gap types` catalog | 23 §1 |
| `GoalTracker` / `goal_drift` | 19; 03 §9 |
| `Hash-chained audit` | 24 §10 |
| `Health KPIs` | 24 §7 |
| `Hermes HUD` | 24 §0, §4.3 (cite); 22 §1 |
| `HUD Mission Control` | 24 |
| `AffinityIndex` (arc-swap lock-free, clones share snapshot) | 04 §8; 04 §Apéndice 2.4 |
| `AffinityRow` (+ `MIN_SAMPLES=3` confidence floor) | 04 §8; 04 §Apéndice 2.4 |
| `AggregationCostContext::from_journal` (real cost guard, G11, sub-fase 2.4) | 04 §6; 04 §Apéndice 2.4 |
| `Aggregation` (Single/MajorityVote/MoA/Council/SelfRefine/Reflexion/SelfDiscover) | 04 §3; 04 §Apéndice 2.2 |
| `Aggregator` (trait, `#[async_trait]`) + `aggregator_for()` dispatch | 04 §3; 04 §Apéndice 2.2 |
| `AggregationMode` enum (7 variantes, `#[serde(tag="type")]`) | 04 §3; 04 §Apéndice 2.2 |
| `AggregationPolicy` (trait, G11 cost guard) + 6 mode-specific impls | 04 §6; 04 §Apéndice 2.1 + 2.2 |
| `AutoRouterConfig` (off-by-default, `ClassifierKind`, per-task threshold RouteLLM-mf calibrated) | 04 §7; 04 §Apéndice 2.3 |
| `BackPressure` (`Arc<Semaphore>` lazy-init, G18) | 04 §6; 04 §Apéndice 2.0.5 |
| `CachePolicy` (Anthropic ephemeral, G1) | 04 §6; 04 §Apéndice 2.0.5 |
| `Cascade` (3-bucket fallback, exclusion set) | 04 §4; 04 §Apéndice 2.1 |
| `ClassifierKind` (`Lexical`/`LogReg`/`Embedding`, default=`Lexical`) | 04 §7; 04 §Apéndice 2.3 |
| `Cooldown` (per-provider default, G5) | 04 §6; 04 §Apéndice 2.0.5 |
| `DataPartBuffer` (HUD WS DataParts reconciliation, Vercel AI SDK pattern) | 04 §2; 04 §Apéndice 2.1; 24 §4.1 |
| `EmbeddingClassifier` (fastembed-rs BGE-small, `#[cfg(feature="fastembed")]`) | 04 §7; 04 §Apéndice 2.3 |
| `Idempotency key` | 02 §3.1.2 |
| `Job Object` sandbox Windows | 25 §3.7 |
| `Kernel Bus` | 02 §3.1 |
| `LanguageIdResolver` | 03 §9.3; 21 §2.2 |
| `LexicalClassifier` (12-bucket regex word-boundary, `extract_features`) | 04 §7; 04 §Apéndice 2.3 |
| `LogisticRegressionClassifier` (multi-class one-vs-rest, no `linfa`) | 04 §7; 04 §Apéndice 2.3; 22 §8.2 AN-2.3-a |
| `McpServerCatalog` (trait, `NoMcpCatalog` default, G19) | 04 §7; 04 §Apéndice 2.3 |
| `McpToolFilter` (`pre_filter` capability_tags AND tool-names, G19) | 04 §7; 04 §Apéndice 2.3 |
| `MLP-linfa deferral` (AN-2.3-a, `linfa` no MLP — Phase 2.5+) | 04 §7.1; 22 §8.2 AN-2.3-a |
| `Router` trait + `RoutingStrategy` enum (6 LiteLLM variants + `Mf` experimental) | 04 §2; 04 §Apéndice 2.1 |
| `RoutingStrategy::Mf` (strong-vs-weak binary routing, RouteLLM `2406.18665`) | 04 §2; 04 §Apéndice 2.4 |
| `M24` migration (`model_affinity_cache` mirror + `task_classifier_decisions` CHECK fix, schema 23→24) | 04 §8; 04 §Apéndice 2.4 |
| `atlas models refresh` (brazo manual del feedback loop affinity) | 04 §8; 08 CLI |
| `RequestFrame` (idempotency, G12) | 04 §4; 04 §Apéndice 2.1 |
| `RouterId` (`Literal`/`Auto{strong_pct}`/`Mf{threshold}` parsing) | 04 §7; 04 §Apéndice 2.3 |
| `TaskType` (12 concretos + Unknown, `as_str`/`parse` round-trip) | 04 §7; 04 §Apéndice 2.3 |
| `TaskTypeClassifier` (trait `#[async_trait]`) + `classifier_for()` dispatch | 04 §7; 04 §Apéndice 2.3 |
| `Task classifier decisions` (M23 audit table, UNIQUE `(prompt_hash, classifier_kind)`) | 04 §7; 04 §Apéndice 2.3 |
| `Model affinity cache` (M23 mirror table, `INSERT OR REPLACE` idempotente para 2.4 reader) | 04 §7; 04 §Apéndice 2.3 |
| `ReflectionEpisode` (M22 migration `reflection_episodes`, anti-doom-loop `detect_doom_loop()`) | 04 §3; 04 §Apéndice 2.2 |
| `CouncilVote` (M22 migration `council_votes`) | 04 §3; 04 §Apéndice 2.2 |
| `SelfDiscover` skeleton cache (`OnceLock<Mutex<HashMap>>`, SHA-256 cache key) | 04 §3; 04 §Apéndice 2.2 |
| `Launcher CLI` (`atlas`) | 08; 25 §3.9 |
| `Learning graphs` (cosine retrieval of past successful `mission_graph`) | 16 §3 "Por grafo de misión exitoso"; 28 §C |
| `Local models` (Ollama/LM Studio/llama-server) | 25 §3.8 |
| `Lost in the Middle` | 11 (reordenar); 23 §1 C8 |
| `MCP bridge stdio` | 07; 25 §5 |
| `Memory tiers` (Letta) | 09; 16 |
| `Mission Consolidated` | 23 §4; 12 §2 |
| `Mission graph` (graphify pattern) | 28 §C (GR-001/003/004/006/007/009) |
| `MissionGraph::traverse` (`shortest_path`, `god_nodes`, `community_partition`) | 28 §C; 28 §C item 3 (Phase 1.5c) |
| `AST extractor` (`tree-sitter` + heuristic Svelte line-classifier) | 28 §C (GR-006); `src-tauri/src/graph/ast.rs` |
| `Mobile remote access` | 24 §16 |
| `Multi-key rotation` | 04 §3.1; 22 §2 |
| `N8n canvas stile reasoning trail` | 24 §13 |
| `Noob mode` | 17 §11 |
| `atlas research` (CLI subcommand: `docs` ungated + `query` ungated (collective, sub-fase 3.3; desde 3.4 emite `journal_ref` + proposal desde ramas) + `note`/`branches` ungated (hands-on, sub-fase 3.4) + `feasibility` ungated (probe §11 + gate + cache M27 12d, sub-fase 3.5) + `ingest` gated `doc-ingest` feature + scrape/search/crawl/extract gated `firecrawl` feature) | 28 §E; 08 (surface); research `30` sub-fases 3.1–3.5 |
| `Outline view Notion` | 24 §14 |
| `Pauserule` | 24 §5 |
| `Plan` (estrutura) | 12 §3 |
| `Planning mode (architect/code/ask/context)` | 23 §8; 21 §12 |
| `Pool swarm` | 05 §5 |
| `Posting format` (audit export) | 24 §10.1; 28 §D |
| `AUMID registration` (Start Menu shortcut for non-MSIX Toast activation) | 28 §F.2 |
| `Toast queue` (SQLite-driven scheduler, survives crashes) | 28 §F.3 |
| `Toast activation deep-link` (`opencode://mission/{id}/...`) | 28 §F.4 |
| `Toast history` (append-only audit + dedupe) | 28 §F.5 |
| `Calendar ICS feed` (axum HUD `GET /atlas-calendar.ics`) | 28 §G.2 |
| `Microsoft Graph /me/calendarView` reader (`graph-rs-sdk`) | 28 §G.3; 22 §12 (Round 5) |
| `BusyWindow` (`AppState.context_busy_windows`, Planning engine hook) | 28 §G.4; 12 §3 |
| `OmniRoute` (free MIT AI gateway, OpenAI-compatible) | 28 §H.2; 22 §12 (Round 5) |
| `SpendLimitError` card (port de Cline PR #10207) | 28 §H.3; 22 §12 (Round 5); 24 §3.3 |
| `ModelReady` Toast (`kind='model_ready'`, feature diferencial) | 28 §H.4; 22 §12 (Round 5) |
| `Retry policy` (jitter ±25%, exponential, bail-out threshold) | 28 §H.5; 04 §9 addendum |
| `Bail-out threshold` (`Profile::bail_out_threshold_secs`, default 60s) | 28 §H.5; 06 (profile schema) |
| `Backup profile` (`Profile::backup_profile_id`, auto-switch on spend-limit) | 28 §H.3; 06 (profile schema) |
| `WinRT AppointmentManager rejected` (capability `appointmentsSystem`) | 28 §G.1; 22 §12 (Round 5) |
| `ProjectSymbolTable` | 03 §9.4 |
| `probe_feasibility` (`research::feasibility::probe_feasibility` + `evaluate` gate §11.3 + M27 cache 12d + `atlas research feasibility`) | 10 §11; 23 §2.2 |
| `ResearchRunReport` (YAML canónico RFC 10 §7, `validate()` 0..1) | 10 §7; research `30` sub-fase 3.0 |
| `ResearchRunKind` (`Full`/`Targeted`/`Mega`, `as_str`/`parse` round-trip) | 10 §1; research `30` sub-fase 3.0 |
| `ResearchRunStatus` (`running`/`completed`/`needing_human` fail-safe) | 10 §10; research `30` sub-fase 3.0 |
| `ConsensusDimension` (`community`/`enterprise`/`academic`/`official`) | 10 §5; research `30` sub-fase 3.0 |
| `ConsensusScorer` (trait, `MIN_SOURCES=3` no-data floor) + `ConsensusScore` (0..100 + referencias) | 10 §5; research `30` sub-fase 3.0 |
| `CommunityScorer`/`EnterpriseScorer`/`AcademicScorer`/`OfficialScorer` (scorers deterministas por dimensión, kind-relevance) + `score_all` + `top_reference` | 10 §5; research `30` sub-fase 3.3 |
| `prefilter_sources` (weak-model pre-filter determinista A.4: dedupe + drop ruido) + `classify_source_kind` (host → kind) | 10 §5; research `30` sub-fase 3.3 |
| `HANDS_ON_WEIGHT` (×1.5) + `hands_on_weight()` + `HANDS_ON_KIND` (evidencia experta RFC 10 §3) | 10 §3; research `30` sub-fase 3.3 |
| `FAIL_SAFE_CONFIDENCE` (0.6) + `apply_fail_safe()` (→ `needing_human`, anti-alucinación RFC 10 §10) | 10 §10; research `30` sub-fase 3.3 |
| `DimensionOutcome` + `ReportSections` + `build_report()` (YAML canónico RFC 10 §7) | 10 §7; research `30` sub-fase 3.3 |
| `atlas research query` (collective run completo: `gh` + arXiv + docs gateway + `--source`/`--hands-on`, YAML a stdout, run en M25) | 08 CLI; 10 §5/§7; research `30` sub-fase 3.3 |
| `DocsGateway` (Context7Max adapter, `ATLAS_CTX7MAX_URL`/`ctx7max` → Context7 MCP → official docs, `from_env`/`query_docs`) + `DocSnippet` (verbatim code hit) + `DocsBackend` | 10 §6; research `30` sub-fase 3.1 |
| `combined_confidence` (media ponderada, hands-on ×1.5) | 10 §3/§5; research `30` sub-fase 3.0 |
| `IngestedDocument` (Markdown + provenance `format`/`bytes`/`title`) + `DocFormat` (`Markdown`/`PlainText`/`Csv`/`Pdf`/`PandocAssisted`) + `IngestError` (`UnsupportedFormat`/`ExternalToolMissing`/…) | 10 §6; research `30` sub-fase 3.2 |
| `doc-ingest` feature flag (parser propio sin crate nueva por audit RFC 25 §11, `pandoc` externo opt-in vía `ATLAS_PANDOC_BIN`/`PATH`, default off) | 25 §11; research `30` sub-fase 3.2 |
| `M25` migration (`research_sources` + `research_consensus`, `status`/`recommended` en `research_runs`) | 10 §7; research `30` sub-fase 3.0 |
| `ResearchNote` (evidencia experta RFC 10 §3: title/project/decision/outcome/confidence/tags/attached_at/signature, `validate()` + `mentions()`) + `parse_tags` | 10 §3; research `30` sub-fase 3.4 |
| `ApplicationBranch` (Opción A/B/C RFC 10 §4: summary/pros/cons/confidence/cost_estimate/fits_stack/sources) + `build_branches()` (top-3 determinista, +0.05 por respaldo experto) + `branch_proposal_lines()` + `journal_ref_for_run()` (rr→jr) | 10 §4/§7; research `30` sub-fase 3.4 |
| `M26` migration (`research_notes` con CHECK 0..1; ramas derivadas, no tabla; `journal_ref` como fila `journal_events` kind=`research_run`) | 10 §3/§4/§7; research `30` sub-fase 3.4 |
| `atlas research note` (adjunta caso experto a M26) + `atlas research branches` (reconstruye A/B/C desde sources + consensus + notes) | 08 CLI; 10 §3/§4; research `30` sub-fase 3.4 |
| `FeasibilityProbe` (topic/domain/min_sources/require_artifact_evidence, `validate()`) + `FeasibilityDomain` (`software`/`hardware`/`academic`/`vendor`, `as_str`/`parse`) | 10 §11.1; research `30` sub-fase 3.5 |
| `ArtifactEvidence` (kind/url/fetched_at/raw_metadata/stars/last_release) + `ArtifactKind` (8 variantes, `is_primary()` — `VendorDocs` no cuenta) | 10 §11.1; research `30` sub-fase 3.5 |
| `FeasibilityReport` (probe_id/topic/found/evidence/confidence/red_flags/recommended_next_step, `validate()` rechaza found-con-flags) | 10 §11.1/§11.3; research `30` sub-fase 3.5 |
| `evaluate()` (§11.3 gate: ≥min_sources distintas + ≥1 primaria + 0 red_flags, determinista) + `detect_red_flags()` (abandoned/no-artifact/no-primary/no-academic) + `fail_safe_status()` (§10 → `needing_human`) | 10 §10/§11.3; research `30` sub-fase 3.5 |
| `probe_feasibility()` (colectores best-effort por dominio §11.2 sin crate nueva: npm/crates.io/PyPI/GitHub REST, vendor fetch + link-rot, arXiv, Wayback; objetivo <5s §11.7) + `ProbeMetrics` + `parse_domains` | 10 §11.2/§11.7; research `30` sub-fase 3.5 |
| `cache_key` + `is_cache_fresh()` (TTL 12 días §11.6) + `M27` migration (`feasibility_cache`, `INSERT OR REPLACE`) | 10 §11.6; research `30` sub-fase 3.5 |
| `atlas research feasibility` (fan-out multi-dominio, bloque legible §11.8 a stdout, métricas a stderr, `--no-cache`) | 08 CLI; 10 §11.8; research `30` sub-fase 3.5 |
| `grill_plan()` + `GrillQuestion` (blocking/advisory) + `GrillReport` (`can_lock`) — pass mecánico antes de `Plan.lock` (skill `grill-me` bundled, impreso por `atlas plan`) | 12 §7; research `30` sub-fase 3.5 (A.3) |
| `Prompt Understanding Pipeline` | 23 §2 |
| `PublicUnderstandingVerdict` | 23 §3 |
| `Reasoning trails` (n8n-stile) | 24 §13 |
| `Reflexion` | 16; 23 §2.1 |
| `Resource Mode` (local/free/mixto) | 02 §5.2; 21 §10 |
| `Roadmap` (fases) | 20 |
| `Sandbox` levels | 18; 25 §3.7 |
| `SandboxLevel` (`none`/`vuOnly`/`container`/`wasm`, `as_str`/`parse`) + `approval_for(level, action)` + `Approval`/`SensitiveAction` (fail-safe Forbidden, tabla RFC 18 §2) | 18 §2/§6; research `34` sub-fase 7.1 |
| `skill_checksum` (SHA-256 canónico skill.toml + README) + `ChecksumVerdict` (`Ok`/`Mismatch`/`Missing`) + `.checksum` sidecar + `install_gate` (fail-safe Forbidden en `atlas skill install`) | 18 §5; research `34` sub-fase 7.0 |
| `evaluate_package` (supply-chain gate determinista: typosquat Levenshtein ≤2 + postinstall Block + env-var Warn, RFC 18 §4) + `atlas security gate` | 18 §4; research `34` sub-fase 7.3 |
| `Scope approval` | 24 §5 |
| `Self-Refine` | 23 §2.1 |
| `Semantic Embedding Index` | 03 §9.5 |
| `SenseActions / policy` | 18 §2; 02 §3.5 |
| `Single source of truth` (SQLite Journal) | 02 §3.4 |
| `Skill Graph` | 06 |
| `Bundled skills catalog` (12 `opencode-*` skills via `include_str!`, feature `bundled-skills` default on) | 29 §3.D; 06 §1 |
| `Skill Picker` iluminado | 17 §4 |
| `pick_skills` (scoring determinista: priority 0.4 + keyword-match 0.4 + verified 0.2, threshold 0.5, suggested/dimmed) + `atlas skill pick` | 17 §4; research `36` sub-fase 8.1 |
| `Artemis` (`google/artemis` VERIFICADO: Android automation por lenguaje natural, MCP server nativo + CLI flash/pro + Python SDK, 99%+ AndroidWorld, Apache-2.0 — lateral proceso externo patrón 8.5, jamás bundling Python RFC 25 §11; `atlas mobile` opción 8.6) | 38 §2-§3; 20 Fase 8.5 |
| `Artemis MCP template` (`mobile/mcp_template.rs`: snippet `.opencode/mcp.json` stdio `uv --directory <repo> run artemis mcp` + 5 tools `mobile_run_task/manage_task/get_device_state/inspect_trace/diagnose` + `atlas mobile mcp-template [--write]`; Sub-fase 11.1 M42 ✅) | 38 §2.1; 20 Fase 11; research `40` §11.1 |
| `Laya` (`laya = "0.1.1"`, open source de Jev — typed-decision model ModernBERT + RL, stack candle; `ClassifierKind::Laya` 4º backend Phase 9 sub-fase 9.1 M33 ✅ std-only MVP, audit candle DIFERIDO, feature `laya` vacío default-off) | 04 §7; 35 §7.1; 22 §8.2 AN-9.1; research `37` §B 9.1 |
| `AstSymbol` (`kind`/`name`/`file`/`line` + `validate`, `presence_boost`/`confidence_for_symbol`, `extract` AST-tras-`ast` / heurístico std-only en default; tabla `ast_symbols` M34 schema 33 + `Journal::record_ast_symbol`/`ast_symbols_for_file`; Phase 9 sub-fase 9.2 M34 ✅, audit tree-sitter APROBADO) | 11 §10; 22 §8.2 AN-9.2; research `37` §B 9.2 |
| `SymbolConfidence` (`name`/`file`/`line`/`confidence`/`present` + `hover_for_symbol`/`diagnostic_for_symbol` + `who_owns`/`affects_where` sobre `ast_symbols`; base = Skill Picker relevance 8.1, presencia = 9.2; Phase 9 sub-fase 9.4 M36 ✅ std-only MVP sin DB nueva) | 03 §9.4; research `37` §B 9.4 |
| `Dependency-cruiser arch rules` (dev-dep 18.4.0 + `.dependency-cruiser.cjs`: `lib/` no importa `routes/`, `stores/` no importa `components/`, `components/` no importa `routes/`, `lib/` no importa core Node-only — tests `*.test.ts` exentos; `pnpm arch`; Phase 9 sub-fase 9.3 M35 ✅) | 20 Fase 9; research `37` §B 9.3 |
| `Skill SDK scaffold` (`skills/sdk.rs`: `validate_skill_id` RFC 06 §1 + `parse_engine` + `scaffold_skill` (skill.toml + SKILL.md stub) + `atlas skill new <name> [--engine]`; Phase 10 sub-fase 10.0 M37 ✅) | 06 §1; research `39` §B 10.0 |
| `Marketplace local firmado` (`skills/marketplace.rs`: `publish_skill` sidecar `.checksum` + self-check + `install_skill` firma OBLIGATORIA ANTES de copiar (Missing/Mismatch → Forbidden, rollback) + `atlas skill install <path|ref>`/`publish`; Phase 10 sub-fase 10.1 M38 ✅) | 20 Fase 10; research `39` §B 10.1 |
| `Remixing de skills` (`skills/remix.rs`: `fork_skill` copia sin `.checksum` + id nueva + versión reset 0.1.0 draft + `remixed_from` provenance + `atlas skill fork <PATH|REF> --name`; Phase 10 sub-fase 10.2 M39 ✅) | 20 Fase 10; research `39` §B 10.2 |
| `Learning social` (`learning/share.rs`: `export_rules` solo verificado 5.1 `candidate`/`active` a YAML determinista + sidecar `<file>.checksum` SHA-256 + `import_rules` firma OBLIGATORIA pre-parse (Missing/Mismatch → Forbidden) + dedup `rule_id` first-write-wins + `atlas learn export/import <FILE>`, sin migración M30; Phase 10 sub-fase 10.3 M40 ✅) | 20 Fase 10; research `39` §B 10.3 |
| `Skill refresh en caliente` | 24 §8.1 |
| `Marketplace local firmado` (`install_skill` firma OBLIGATORIA pre-copy + `publish_skill` sidecar `.checksum`, `InstalledSkill`, `atlas skill install <path\|ref>`/`publish <dir>`, Phase 10 sub-fase 10.1 M38 ✅) | 06 §9; 18 §5; research `39` §B 10.1 |
| `Marketplace git-based` (`skills/marketplace.rs`: `install_from_git` git CLI `clone --depth 1 --` patrón swarm 4.0 + `resolve_skill_dir_in_clone` (raíz/`<name>`/`skills/<name>`/scan por id) + firma 10.1 INTACTA + temp auto-limpia + `atlas skill install <name> --from <git-url>`, sin servidor central; Fase 12 sub-fase 12.1 M44 ✅) | 20 Fase 12; research `40` §B 12.1 |
| `Skills as graph templates` (`graph.toml`, 4th skill file) | 23 §7.3; 28 §C |
| `State DAG` (RFC 19 supervisor as DAG) | 19 §6.1.1; 28 §C |
| `session/set_mode` override (ACP → SupervisorState) | 19 §6.1.2; 28 §B item 6 |
| `SvelteKit CSR` | 25 §3.3 |
| `sqlite-vec` | 25 §3.4; 09 |
| `Stack technical` | 25 |
| `Steer` (en caliente) | 05 §7; 24 §3.2 |
| `Swarm roles` | 05 §1 |
| `Swarm registry` (M29 `swarm_agents` + `agent_mailbox`, `SwarmAgentRow`, `Role`) | 05 §1; research `31` sub-fase 4.0 |
| `Tauri 2` | 25 §3.1 |
| `Team Mode` (AionUI) | 22 §2 (cited); 05 |
| `Tiered memory` | 09; 16 |
| `Tokenizer` (tiktoken-rs / CharRatio) | 04 §6; 04 §Apéndice 2.0.5 |
| `ToolCall` enum cross-provider (OpShape) | 04 §3; 04 §Apéndice 2.0.5 |
| `Tower-LSP` | 25 §3.6 |
| `Worktree own per subagent` | 05 §3; 24 §12; 25 §3.2 |
| `WorktreeManager` (git CLI via `std::process`, sin `git2`) | 05 §4; research `31` sub-fase 4.0 |
| `SwarmRunner` (pool dispatch por rol + topología RFC 05 §2 + checkpoint por agente) | 05 §2/§4/§5; research `31` sub-fase 4.2 |
| `FileLockRegistry` (locks determinísticos + caps `max_agents`/`max_files`, RFC 05 §8) | 05 §4/§8; research `31` sub-fase 4.2 |
| `rebase_after_merge` (auto-rebase post-merge CN-003 en workspaces vivos, git CLI sin `git2`, fail-safe a manual en conflicto) | 05 §4; research `31` sub-fase 4.4; research `28` §A.4 CN-003 |
| `resume_state` (reanudación desde cualquier estado, RFC 19 §6.4/§5) | 19 §5; research `33` sub-fase 6.0 |
| `Journal Observer` (`GET /hud/journal` paginado + payload íntegro, RFC 19 §10) | 19 §10; research `33` sub-fase 6.1; 24 §3 |
| `Hardware monitor` (`HardwareSnapshot` std-only: RAM wmic//proc + VRAM nvidia-smi fail-safe + `total_model_cost_usd`, consts `MONITOR_*`, `BusEventKind::HardwareSnapshot` + `atlas monitor` + `projectHardwareSnapshot`/`monitorPressureOf` HUD; sysinfo/nvml-wrapper diferidos RFC 25 §11) | 20 Fase 8; research `36` sub-fase 8.2 |
| `Remote-live dual-PC` (RustDesk lateral AGPL jamas bundling: `RemoteRole` server/client + `ATLAS_RUSTDESK_BIN`/`PATH` detection + `spawn_session` + `atlas remote status/guide/serve/connect` + modelo Nate Gentile servidor-potente/thin-cliente, KPI <100ms) | 20 Fase 8; research `36` sub-fase 8.5 |
| `Remote auth` (bearer propio `ATLAS_REMOTE_TOKEN`/`remote_token.txt` + `verify_bearer` + cookie `atlas_session` + `discovery_url` OIDC shape + `GET /remote/status` informativo, bearer-enforced con feature `remote-ui`; axum-oidc-layer/openidconnect diferidos RFC 25 §11 + `atlas hud --auth-status/--rotate-token`) | 20 Fase 8; research `36` sub-fase 8.3 |
| `Sister TUI` (Document Model propio sin crates: `SisterSnapshot` + `SisterFeed` cap 20 + `render_frame` determinista + `ws_url` mismo Kernel Bus WS + `atlas sister --watch` + binario `atlas-tui` tras feature `tui`; ratatui diferido RFC 25 §11) | 20 Fase 8; research `36` sub-fase 8.4; research `28` Sector B |
| `Worktrees view` | 24 §12 |
| `Zag Nano Stores` | 25 §3.3 |

---

## 6. Reglas para editar estos RFCs

1. Al editar cualquier RFC, **anotar el cambio en `22 - Research Findings.md` §6/§8.4** si el cambio se origina en investigación. Si no, dejarlo documentado en el propio RFC, sección "Estado".
2. Si se añade o renombrar un concepto referenciado desde otros RFCs, **actualizar la tabla del §5 de este RFC `26`**.
3. Si se añade/borra una dependencia estricta, **actualizar §2.1** y rehacer el grafo ASCII del §2.
4. Si se añade un RFC nuevo (p.ej. `27 - Something.md`), **anotarlo en §1 catálogo, §2 dependencias, §3 cross-references, §4 ruta de lectura, §5 concepts**.
5. Mantener **cada RFC autónomo** — un humano que abre solo ese archivo debe poder entenderlo sin otro. La dependencia en la cabecera enumera solo "qué RFCs tienen algo que añadir sobre este tema", no "qué RFCs son prerrequisito intelectual".
6. **No romper linkage**: cualquier referencia tipo \`nn - Title.md\` debe resolver a un archivo existente. Verificado en el §3.

---

## 7. Conversión futura mkdocs (out-of-scope)

Para un futuro si se quiere servir el RFC como sitio navegable:

- Cada RFC se vuelve una página.
- Mkdocs material permite Mermaid diagram (grafo de dependencias §2).
- Search bar.
- Tags por dominio (Backend, Frontend, Security, …).

Mientras tanto, este archivo `26 - Index & Cross-References.md` sirve como índice navegable.

---

## 8. Estado

- Status: Draft v1
- Depends on: todos los demás RFCs
- Realiza: compendio y map navigateable. En una single página, un recién-llegado puede encontrar qué RFC responde su pregunta.
