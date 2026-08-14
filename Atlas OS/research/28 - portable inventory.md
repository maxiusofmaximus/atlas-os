# Inventario Exhaustivo de Portabilidad — Repos fuente → Rust/Atlas OS

**Fecha:** 2026-07-25 · **Orquestador:** opencode (z-ai/glm-5.2)
**Source:** 4 repos públicos MIT/Apache-2.0 (permite portar con atribución)
**Propósito:** No perder el hilo. Garantizar que cada item aprovechable de los repos investigados termine referenciado en RFC 28 y portado en la fase que le corresponde.

## PRE-FETCH CORRECCIONES DE REALIDAD

- **graphify**: bundle real es `safishamsi/graphify` (default branch `v8`, Apache-2.0, 95.4k stars). Las queries viven en `serve.py` y la lógica AST/call-flow en `extract.py` (244K — biggest file in repo). `cluster.py` para Leiden/Louvain. No hay `queries.py` ni `ast.py` separados.
- **posting**: no tiene `serialize.py`. El `str_presenter` vive en `src/posting/yaml.py` (720 bytes, 30 LOC). Jump mode en `jumper.py` + `jump_overlay.py`. Tests canónicos: `test_curl_export.py`, `test_open_api_import.py`, `test_variables.py`, snapshots en `tests/__snapshots__/`.
- **intelligent-terminal**: installer Rust real en `tools/wta/src/agent_hooks_installer.rs` (~7000 LOC, 269K). `app.rs` es 772K. `main.rs` 163K. Spec de eventos en `doc/specs/llm-agent-event-integration.md`. NO hay `127.0.0.1/mcp` en IT — corregido.
- **autoresearch**: `program.md` fetched 1:1. README en `master` (no `main`). `prepare.py` y `train.py` completos.

---

## GRAPHIFY (GR-*)

| ID | Origen | Lang | LOC | Resumen | Crates Rust | Coste | Destino OC-OS |
|---|---|---|---|---|---|---|---|
| GR-001 | `graphify/cluster.py:1-260` (`_partition`, `cluster`, `_split_community`) | Python | ~260 | Leiden via graspologic con fallback Louvain. Split oversized >25%, cohesion re-split <0.05, exclude hubs percentile, remap greedy a comunidad prev. | `petgraph`; custom Leiden impl O(N log N) o crate `leiden_alg` si existe; FFI a `leidenalg` C si no madura. | L | `src-tauri/src/graph/community.rs` |
| GR-002 | `graphify/cluster.py:label_communities_by_hub` + `community_member_sigs` | Python | ~30 | Hub-naming determinista (mayor grado gana, ties por node id), sha256 fingerprint de members para detectar stale labels. | `sha2`, `petgraph::deg`. | XS | `src-tauri/src/graph/community.rs` |
| GR-003 | `graphify/report.py:1-180` `generate()` | Python | ~180 | Genera `GRAPH_REPORT.md`: God Nodes, Surprising Connections, Community Hubs, Import Cycles, Hyperedges, Ambiguous Edges, Knowledge Gaps. Taggeo `EXTRACTED`/`INFERRED`/`AMBIGUOUS`. | `serde_json`; templating con `format!` literals. | M | `src-tauri/src/graph/report.rs`. Schema `graph.json` ~1KB |
| GR-004 | `graphify/paths.py:1-200` `_atomic_replace`, `write_json_atomic`, `_is_test_path`, `disambiguate_ambiguous_candidates` | Python | ~200 | Atomic write (temp+rename, fallback copy Windows), test-path classifier (regex), tiebreaker bare-call por test/non-test + path proximity. | `std::fs::rename`, `tempfile`, `regex`. | S | `src-tauri/src/graph/paths.rs` + `src-tauri/src/journal/atomic.rs` |
| GR-005 | `graphify/serve.py` (~92K) — MCP server stdio con tools: `query`, `path`, `explain`, `add`. Implementación MCP completa. | Python | ~15K in scope | MCP server stdio (initialize, tools/list, tools/call, notifications). | `rmcp` crate o impl directa. JSON-RPC sobre stdio. | M | `src-tauri/src/graph/serve.rs` |
| GR-006 | `graphify/extract.py` (244K) — tree-sitter bindings, AST→call-graph, cross-file resolution, `find_import_cycles` | Python | ~6000 | Tree-sitter parsers Python/TS/JS/Go/Rust/Java/C/C++/Ruby/C#. Call-flow HTML 81K separado. | `tree-sitter` + `tree-sitter-languages`; `petgraph`. **Scope down**: portar AST + ciclo détector, dejar callflow_html para v2. | L | `src-tauri/src/graph/extract.rs` + `extractors/<lang>.rs` |
| GR-007 | `graphify/affected.py` (9.4K) — impact analysis "qué cambió" para `--update` incremental | Python | ~250 | SHA256 cache de archivos, re-extract solo changed, merge en grafo existente. | `sha2`, `dashmap`, `walkdir`. | S | `src-tauri/src/graph/cache.rs` |
| GR-008 | `graphify/_minhash.py` (4.1K) — MinHash near-duplicate document detection | Python | ~110 | MinHash con bandas/rows, Jaccard threshold. Detección de documentos similares sin embeddings. | `wyhash` o impl directa; `bitvec`. Alternativa: usar `fastembed-rs` ya en stack. | XS | `src-tauri/src/graph/dedup.rs` |
| GR-009 | `graphify/skill-agents.md` y `skill-claw.md` (40-60K cada uno) — skill templates para 6 CLIs | Markdown | ~20K total | Skill prompt base para Claude/Copilot/Codex/Gemini/Aider/Devin. Template TRIGGER, INSTRUCTIONS, OUTPUT. | **Copia_uso** (Markdown directo). | XS | `skills/graphify/<cli>.md` + atribución. RFC 23 §7 |
| GR-010 | `graphify/hooks.py` (31K) — `graphify hook install` post-commit hook | Python | ~800 | Instala `.git/hooks/post-commit` idempotente que re-build el grafo. | `std::fs`, `git2` (opcional). | XS | `src-tauri/src/cli/hooks.rs` |
| GR-011 | `graphify/install.py` (96K) — installer skill graphify en CLAUDE.md, multi-CLI detection | Python | ~2500 | Self-install en `~/.claude/skills/`, detecta si CLI ya tiene el skill, idempotente. | `dirs`, `std::env`. | S | `src-tauri/src/cli/install.rs` |
| GR-012 | `graphify/cli.py` (180K) — CLI total: `graphify`/`add`/`query`/`path`/`explain`/`--watch`/`--mcp` | Python | ~5000 | Parser con subcomandos, watch mode (watchfiles), MCP mode. | `clap`. | L | `src-tauri/src/cli/bin/opencode.rs` subcommand `graph` |

## POSTING (PT-*)

| ID | Origen | Lang | LOC | Resumen | Crates Rust | Coste | Destino OC-OS |
|---|---|---|---|---|---|---|---|
| PT-001 | `src/posting/collection.py:1-260` `RequestModel`, `Auth`, `Header`, `QueryParam`, `Cookie`, `Options`, `RequestBody`, `Scripts` | Python | ~260 | Schema completo `.posting.yaml`: method/url/headers/params/cookies/body/auth/scripts/options/path_params. `apply_template` (string.Template vars), `to_httpx`, `to_curl`, `save_to_disk`. | `serde` + `serde_yaml` (campos `Option<T>` con `#[serde(default)]`), `reqwest`. | M | `src-tauri/src/profiles/request_model.rs`. RFC 25 §3.5 |
| PT-002 | `src/posting/collection.py:Collection.from_openapi_spec`, `from_directory`, `save_to_disk` | Python | ~80 | OpenAPI import → Collection tree; recursive directory load + sort. README generator from OpenAPI info. | `openapiv3` o `utoipa`. `walkdir`. | S | `src-tauri/src/profiles/collection.rs` |
| PT-003 | `src/posting/yaml.py:1-26` `str_presenter` registered globally | Python | 26 | YAML scalar representer: si string tiene `\n`, emite como literal block `|` con `rstrip()` por línea. **Oro puro — explica diff-friendliness.** | `serde_yaml` no soporta custom representers fácilmente → wrapper `yaml-rust2` o impl manual. | XS | `src-tauri/src/journal/yaml_format.rs` |
| PT-004 | `src/posting/themes.py:1-400` `Theme`, BUILTIN_THEMES (galaxy/nebula/sunset/aurora/nautilus/cobalt/twilight/hacker/manuscript/hypernova/synthwave) | Python | ~400 | Theme system: primary/secondary/background/surface/panel/warning/error/success/accent + syntax colors + method colors (GET/POST/PUT/DELETE/PATCH/OPTIONS/HEAD). **11 builtin themes**. | `serde_yaml`, `palette` crate. Convertir Textual → CSS vars Svelte 5. | M | `src/lib/stores/themes.ts` + `src-tauri/src/theme.rs` |
| PT-005 | `src/posting/jumper.py:1-50` + `jump_overlay.py:1-110` | Python | 160 | Jumpable Protocol, JumpInfo, Jumper class `get_overlays()`, JumpOverlay ModalScreen con debounce resize 50ms, ESC dismiss. **Algoritmo Amp-style para navegación espacial**. | Svelte store + custom events; CSS overlay absolute. ResizeObserver con debounce. | S | `src/lib/components/JumpOverlay.svelte` + `src/lib/stores/jumper.ts` |
| PT-006 | `tests/test_curl_export.py` (7.7K) + `tests/test_curl_import.py` (8.5K) | Python pytest | ~600 total | Tests canónicos: serialización a cURL, parsing curl. Edge: URL con query, form_data con [[]], auth basic/digest, body vazio. | `#[test]` + `assert_eq` sobre strings cURL. | S | `src-tauri/src/profiles/request_model.rs` con `#[cfg(test)] mod tests` |
| PT-007 | `tests/test_open_api_import.py` (10K) + `tests/test_postman_import.py` (3.9K) | Python | ~500 | Import OpenAPI 3.0 y Postman v2.1. Edge: refs, security schemes, server variables, examples. | `openapiv3` + `serde_json`. | M | `src-tauri/src/profiles/importers/` |
| PT-008 | `src/posting/commands.py` (8.5K) — slash command palette | Python | ~250 | Command palette con fuzzy search, keybinding registration, `?help`. | `clap`; Svelte CommandPalette. | S | `src/lib/components/CommandPalette.svelte` |
| PT-009 | `src/posting/suggesters.py` (392 bytes) + `textual-autocomplete` (dep 4.0.6) | Python | ~10 stub + ~800 lib | FilePathSuggester, URLSuggester, HeaderSuggester. Registry por widget. | `fuzzy-matcher`; Svelte typeahead. | S | `src/lib/components/AutoComplete.svelte` |
| PT-010 | `src/posting/variables.py` (4.7K) — `Template.substitute` con SubstitutionError | Python | ~150 | Variable `${name}` y falla clara cuando falta. Envs override. | `regex` o parser manual. `thiserror`. | XS | `src-tauri/src/profiles/variables.rs` |
| PT-011 | `src/posting/scripts.py` (6.7K) — pre/post request Python scripts | Python | ~200 | Ejecuta script setup/on_request/on_response. Sandboxing? NO. | **No portable directo** — Py embed. Alternativa: scripts en Lua (`mlua`) o Rhai. | M | `src-tauri/src/profiles/scripts.rs` con `mlua` |
| PT-012 | `src/posting/urls.py` (2.7K) `ensure_protocol`, `substitute_path_params` | Python | ~80 | Asegura `http://` si no hay scheme; reemplaza `:param` en URL path. | `url` crate. | XS | `src-tauri/src/profiles/request_model.rs` |

## INTELLIGENT-TERMINAL (IT-*)

| ID | Origen | Lang | LOC | Resumen | Crates Rust | Coste | Destino OC-OS |
|---|---|---|---|---|---|---|---|
| IT-001 | `tools/wta/src/agent_hooks_installer.rs` (269K) — confirmation via `wt-agent-hooks/README.md` | **Rust native** | ~7000 | Installer idempotente multi-CLI (Claude/Copilot/Codex/Gemini/OpenCode). Resolution chain: env override → exe-dir → walk parents → embedded `include_str!` blobs. Strips stale entries pre-reinstall. | **Port shallow** — copiar ergonomic patterns. `dirs`, `include_str!`, `walkdir`, `target::host`. | S / L | `src-tauri/src/cli/install.rs` |
| IT-002 | `tools/wta/wt-agent-hooks/<cli>/wt-agent-hooks/hooks/send-event.ps1` (~250 LOC) — byte-identical across subtrees | PowerShell | ~250 | **Exit-0 trap pattern**: `trap { exit 0 }` + outer try/catch swallowed broad. Stdio discipline. Diagnostic trace 5MB rotation. CLI-source detection. Strip campos grandes. CommandLineToArgvW escape. Threshold 25000 chars. | **No port a Rust** (es PowerShell). Copy_uso verbatim del script. Añadir nuestra propia cola emit-event. | XS (copy) | `src-tauri/hooks/send-event.ps1` + `src-tauri/src/cli/hooks_emit.rs` |
| IT-003 | `tools/wta/wt-agent-hooks/copilot/wt-agent-hooks/hooks/hooks.json` | JSON manifest | ~70 | 10 hooks domains → WTA topics: SessionStart→agent.session.start, UserPromptSubmit→agent.prompt.submit, PreToolUse→agent.tool.starting, PostToolUse→agent.tool.finished, PostToolUseFailure→agent.tool.failed, StopFailure→agent.error, Stop→agent.stop, SubagentStop→agent.subagent.stop. `-CliSource copilot` al final. | **Copy_uso** (JSON). Generar variantes por CLI programaticamente. | XS | `src-tauri/hooks/<cli>/hooks.json`. Schema RFC 19 |
| IT-004 | `doc/specs/llm-agent-event-integration.md` (~400 LOC) — proposal completo | Markdown spec | ~400 | **Brief de oro**. Two-channel: COM `wtcli send-event` (out-of-band) + OSC 9001 AgentEvent (in-band, zero-dep). JSON envelope `{type:"event", method:"agent_event", params:{pane_id, event, timestamp, agent, ...}}`. 7 event types standard (`agent.started`, `agent.task.{started,completed}`, `agent.tool.{invoked,completed}`, `agent.error`, `agent.idle`). Custom passthrough. Fire-and-forget, no server storage, 4KB soft cap. | **Copia_uso spec**. Implementar en Rust: OSC parser `src-tauri/src/hud/osc9001.rs`, producer `src-tauri/src/cli/send_event.rs`. | S / M | `src-tauri/src/hud/event_envelope.rs` con enum `AgentEvent` 1:1 con schema. Posible RFC nuevo |
| IT-005 | `doc/wtcli-commands.md` — 17 subcomandos | Markdown | ~150 | tablas: `list-windows/lsw`, `list-tabs/lst`, `list-panes/lsp`, `active-pane`, `capture-pane`, `pane-status`, `new-tab/neww`, `split-pane/splitw`, `kill-pane/killp`, `focus-pane/focusp`, `wait-for`, `listen` (long-run, `-t` filter, `--event "agent.*" glob), `send-event/se`, `publish`, `info`, `test-pipe`, `set-env/setenv`. JSON output mode. | **Copia_uso** como spec. `clap` derive enum. | M | `src-tauri/src/cli/bin/opencode.rs` + `src-tauri/src/hud/control.rs` |
| IT-006 | `tools/wta/src/app.rs` (772K) — main app loop, `route_one_hook` switch at ~line 582, `publish_event_blocking` for autofix | Rust | ~20000 | Routing de hooks → SessionState transitions. Lee `cwd` del payload. `is_user_input_tool` chequea `tool_input.{question,prompt,message}`. Maneja `_truncated`, `_original_size`. | **No port directo** (TUI intimate). Extrar `route_one_hook` enum + state machine. `serde_json::Value` para payload dinámico. | L | `src-tauri/src/hud/router.rs` |
| IT-007 | `tools/wta/src/agent_sessions.rs` (147K) + `session_registry.rs` (162K) + `session_mgmt.rs` (24K) + `session_history.rs` (5K) | Rust | ~9500 | Session lifecycle: SessionStarted (cwd), Working, Idle, Ended (exit_code). Pane GUID binding. History loader from disk. Concurrent multi-ACP locking. | `parking_lot::RwLock`, `dashmap`, `serde`. | L (3 sem) | `src-tauri/src/profiles/session.rs`, `src-tauri/src/journal/sessions.rs` |
| IT-008 | `tools/wta/src/agent_check.rs` (26K) — auto-detect installed CLIs | Rust | ~700 | Enumera CLIs instalados, WinGet install de Copilot por defecto. Version detection. | `which`, `windows` registry/winget. | M | `src-tauri/src/cli/agent_check.rs`. RFC 04 §3 |
| IT-009 | `tools/wta/wt-agent-hooks/opencode/wt-agent-hooks.js` + `plugin.json` — plugin OpenCode V1 | JS + JSON | ~150 (estimado) | Plugin JS mapea eventos OpenCode V1 (session.created/updated, chat.message, tool.execute.before/after, permission.*, question.*, session.idle/error/deleted, dispose) a WTA topics. Child sessions con `parentID` ignoradas. | `boa_engine` (JS runtime en Rust) si runtime JS plugins, **ON**: RFC 25 §11 → evaluar. Alternativa: plugin API en Rust puro. | M / L | `src-tauri/src/skills/plugin_v1.rs` |
| IT-010 | `tools/wta/sync-hooks.ps1` (2K) — dev utility copy hooks | PowerShell | ~50 | Copia send-event.ps1/hooks.json desde repo a `~/.copilot/installed-plugins/wt-local/wt-agent-hooks/hooks/` y `~/.gemini/extensions/wt-agent-hooks/hooks/`. Para dev iterativo. | **Copy_uso verbatim**. | XS | `scripts/sync-hooks.ps1` |
| IT-011 | `tools/wta/terminal-acp-shell-integration.md` (34K) — Phase 2 events (pane.process.exited, pane.cwd.changed, pane.command.completed via OSC 133;D) | Markdown spec | ~900 | Server-originated events: pane exit, CWD change via OSC 7/9;9, command completed via OSC 133 FTCS. Mapped a `s_NotifyEventToComClients`. | `vtparse` parser OSC. Custom impl OSC 133/7/9;9/9001. | M | `src-tauri/src/hud/shell_integration.rs` |
| IT-012 | `tools/wta/Cargo.toml` (2.1K) | TOML | ~70 | Crates: `tokio`, `serde`, `serde_json`, `clap`, `windows` (Win32+COM), `tracing`, `ratatati`, `parking_lot`, `dashmap`, `anyhow`, `thiserror`. | **Reference only** — no port. Notas para nuestra `Cargo.toml`: `windows = { features = ["Win32_System_Com", ...] }` solo Win target. | XS | `src-tauri/Cargo.toml` |
| IT-013 | `tools/wta/src/main.rs` (163K) — CLI entrypoint con `wta` subcommands | Rust | ~4500 | Argparse completo, dispatch a módulos. Patrón idiomático `anyhow::Result`. Demo CLI Rust sobre `clap` con 17+ subcomandos. | **Reference** para nuestro `src-tauri/src/cli/bin/opencode.rs`. | S | inspiración, no copia literal |
| IT-014 | `tools/wta/src/event.rs` (7.7K) — definición del envelope event y parser | Rust | ~200 | Struct `Event { r#type, method, params: serde_json::Map }`. Deserialize forwards-compat. | `serde`, `serde_json`. **Direct hint impl**. | XS | `src-tauri/src/hud/event.rs`. Schema 1:1 con IT-004 |

## AUTORESEARCH (AR-*)

| ID | Origen | Lang | LOC | Resumen | Crates Rust | Coste | Destino OC-OS |
|---|---|---|---|---|---|---|---|
| AR-001 | `program.md` (1:1 fetched, ~520 LOC markdown) | **Markdown template** | 520 | **Skill base template 1:1**. 6 secciones: Setup, Experimentation, Output format, Logging results, The experiment loop, NEVER STOP. Guardrails: NEVER STOP, NEVER pause to ask, rewind sparingly. TSV `commit\tval_bpb\tmemory_gb\tstatus\tdescription`. Metric `grep "^val_bpb:" run.log`. Loop budget 5min, kill >10min. Crashes: typo fix re-run, broken idea skip. | **Copia_uso verbatim** (Markdown). Citar en RFC. | XS | `skills/autoresearch/program.md`. RFC nuevo derivado de RFC 16 §4 |
| AR-002 | `prepare.py:1-280` constants block (`MAX_SEQ_LEN=2048`, `TIME_BUDGET=300`, `EVAL_TOKENS=40*524288`, `VOCAB_SIZE=8192`, `SPLIT_PATTERN`, `SPECIAL_TOKENS`, `BOS_TOKEN`, `CACHE_DIR`, `BASE_URL`) | Python | ~60 | Constantes pinned: 5min time budget, 8192 vocab, BOS handling, HF dataset URL. | `const` block Rust. "fixed constants" como contrato. | XS | `src-tauri/src/experiments/constants.rs` |
| AR-003 | `prepare.py`: `download_single_shard` + `download_data` + `train_tokenizer` (rustbpe+tiktoken) | Python | ~120 | Retry 5 intentos backoff `2^attempt`, parallel pool, atomic `.tmp` rename. BPE tokenizer train → pickle con tiktoken encoding. | `reqwest` + `reqwest-middleware`, `tokio::task::spawn`, `rusqlite` para cache en vez de pickle. `tokensearch` o BPE impl pura Rust. | L | `src-tauri/src/experiments/data_prep.rs` |
| AR-004 | `prepare.py`: `make_dataloader` BOS-aligned best-fit packing | Python | ~80 | Best-fit packing: para cada row T+1 tokens, encontrar doc más grande que cabe, recortar shortest. Pre-allocate CUDA. | Portable a CPU. `ndarray` o `candle-core` si GPU. Packing algoritmo en Rust puro. | M | `src-tauri/src/experiments/dataloader.rs` |
| AR-005 | `prepare.py`: `evaluate_bpb` (fixed metric, **DO NOT CHANGE**) | Python | ~30 | Bits per byte: `total_nats / (math.log(2) * total_bytes)`. Excluye special tokens. Métrica vocab-size-independent. | `std::f64::consts::LN_2`. Single-line port. | XS | `src-tauri/src/experiments/metrics.rs::evaluate_bpb` |
| AR-006 | `train.py:1-280` GPT Model — CausalSelfAttention con Value Embedding residual (ResFormer), MuonAdamW optimizer polar express | Python (torch) | ~280 | GPT con RoPE, sliding window attention, Ve gate, value embedding per alternating layer, logit softcap 15. Muon+AdamW fused. `@torch.compile`. | **No port a Rust** — bindings `candle-core` o `burn`. autoresearch callee es editable por el agente — portarlo fija impl. Port ref-only. | L (1 sem solo wrapper) | `src-tauri/src/experiments/model.rs` (opcional) |
| AR-007 | `train.py`: `MuonAdamW` optimizer with polar express orthogonalization coeffs | Python | ~80 | 5 coeff tuples polar decomposition iterative orthogonalization Nesterov momentum + NorMuon variance reduction + cautious weight decay mask. | `ndarray-linalg` para SVD/QR. Polar express → impl directa O(d^2). | L | `src-tauri/src/experiments/optimizer.rs` |
| AR-008 | `train.py`: hyperparameters block (`ASPECT_RATIO=64`, `HEAD_DIM=128`, `TOTAL_BATCH_SIZE=2**19`, `WARMUP_RATIO=0.0`, `WARMDOWN_RATIO=0.5`) + schedules | Python | ~40 | Hyperparams top-of-file editable por agente. Schedules: warmup 0%, warmdown linear 50%, momentum linear interp 0.85→0.95, weight decay linear `(1-progress)`. | `const` + funciones puras. | XS | `src-tauri/src/experiments/schedules.rs` |
| AR-009 | `train.py`: training loop fast-fail + GC + EMA smooth loss con debias | Python | ~30 | Fast-fail en NaN/loss>100 aborta. GC management evita stalls. EMA `0.9` debiased `ema/(1-0.9**step)`. | Fast-fail directo en Rust. GC no aplicable. EMA straightforward. | XS | `src-tauri/src/experiments/loop.rs::train_step` |
| AR-010 | `train.py`: logging block final `---` separator + 9 metricas | Python | ~15 | Output structured por stdout con `---` delimiter. **Formato canónico parseable por `grep "^val_bpb:"`**. | `println!("---")` + formato `f64:.6`. | XS | `src-tauri/src/experiments/report.rs::print_summary` |
| AR-011 | `program.md` "results.tsv" format spec | Markdown | ~30 | Header + 5 cols: `commit\tval_bpb\tmemory_gb\tstatus\tdescription`. status ∈ {keep, discard, crash}. Untracked by git. Example con 4 filas baseline. | `csv` crate con `\t` delimiter, o TSV builder manual. | XS | `src-tauri/src/experiments/results.rs::ResultsTsv` |
| AR-012 | `program.md` "**NEVER STOP**" guard + "rewind sparingly" hard rules + pacing modelo | Markdown prose | ~80 | Como prompt-engineering patterns: autonomía absoluta, no preguntar al human, rollback última opción, idear combos de near-misses. | **Copia_uso verbatim** del prose. Material de skill. | XS | `skills/autoresearch/NEVER_STOP.md` |
| AR-013 | `README.md` "Notable forks" — autoresearch-macos, autoresearch-mlx, autoresearch-win-rtx, autoresearch (AMD) | Markdown hyperlink | ~10 | Pointer a adaptaciones platform-specific. | n/a | XS | Nota en `22 - Research Findings §X` |

---

## RECOMENDACIÓN DE PORTADO POR COST-BENEFIT

### Fase 0 (inmediato, 1-2 días, todo XS copy_uso)
1. **AR-001, AR-012**: copiar `program.md` verbatim a `skills/autoresearch/` — no traducción, solo attribution Karpathy MIT.
2. **AR-002, AR-005, AR-008, AR-010**: constantes + métrica BPB + schedules + log format → ~150 LOC Rust. Foundation para experimentos sin tocar modelo.
3. **IT-003**: `hooks.json` mapping tabla — schema canónica RFC 19 §3 state IDs.
4. **IT-004**: spec OSC 9001 — copy_uso texto como base.
5. **PT-003**: `str_presenter` — 26 LOC gold para YAML legible en profiles/journal.

### Fase 1 (1-2 semanas, M items)
6. **PT-001, PT-002, PT-006**: RequestModel + Collection + tests canónicos curl. Habilita HTTP testing pipelines (RFC 14 Validation).
7. **GR-007, GR-004**: `affected.py` SHA256 cache + `paths.py` atomic writes/disambiguate. Reusable para journal idempotencia.
8. **IT-014, IT-013**: event envelope + CLI dispatch pattern.
9. **PT-005**: Jumper UX → jump-mode in HUD Svelte.
10. **GR-009**: skill-agents.md / skill-claw.md como templates para propia CLI integración.

### Fase 2 (1 mes, L items — req. RFC nuevo Knowledge Graph)
11. **GR-001, GR-003, GR-006**: Leiden + report + tree-sitter extractors. Núcleo graphify port. Posible FFI a `leidenalg` C si impl Rust no madura.
12. **IT-001, IT-006**: agent_hooks_installer pattern + route_one_hook state machine.
13. **AR-003, AR-004**: data_prep + dataloader (solo si ajustamos a candle-core).

### Fase 3 (3+ meses, opcional)
14. **IT-007**: session_registry full.
15. **GR-002, GR-008, GR-011**	shale label hashing + minhash + install orchestrator.
16. **AR-006, AR-007**	model + MuonAdamW (high risk, requires candle-core refactor; copiar como ref skeleton).

---

## URLs NO VERIFICADAS (HONESTAMENTE marcadas not-fetched-suspecting-exists)

- `tools/wtcli/src/main.cpp` — path no verificado via API listing; wtcli está en `src/tools/wtcli/` off-tree. `doc/wtcli-commands.md` confirma existe pero no fetched source lines.
- `tools/wta/wt-agent-hooks/opencode/wt-agent-hooks.js` — README confirma existe, byte content no fetched.
- `tools/wta/wt-agent-hooks/claude/`, `codex/`, `gemini-extension/`, `hook-debug/` — listings fetched, byte content no.
- `src/cascadia/TerminalProtocol/TerminalProtocol.idl` — referenced en spec, no fetched; IDL WinRT no relevante.
- `graphify/__main__.py` (29K), `build.py` (63K), `cache.py` (47K), `llm.py` (141K) — listed via API, no fetched content.
- `tests/__snapshots__/` — gold snapshots referenced de posting, contenido no inspeccionado.
- `src/posting/widgets/` — referenciados por app.py, no explorados.
- `tools/wta/src/agent_sessions.rs` (147K), `session_registry.rs` (162K), `coordinator.rs` (113K) — sizes confirmados via API, line-level content fetched 0.

## ATRIBUCIÓN REQUERIDA

- **Graphify** (Apache-2.0): `"Ported from graphify <https://github.com/Graphify-Labs/graphify>; Copyright 2026 Graphify Labs. Licensed under the Apache License, Version 2.0."` Distribuir LICENSE + NOTICE si redistribuimos.
- **Posting** (Apache-2.0): `"Ported from posting <https://github.com/darrenburns/posting>; Copyright Darren Burns. Apache-2.0."`
- **Intelligent Terminal** (MIT): `"Ported from microsoft/intelligent-terminal <https://github.com/microsoft/intelligent-terminal>; Copyright Microsoft Corporation. MIT License."` NOTA: MS trademarks policy (NO usar "Windows" branding sin permiso — llamarlo "agent pane" no "Intelligent Terminal integration").
- **Autoresearch** (MIT): `"Includes verbatim text from karpathy/autoresearch program.md <https://github.com/karpathy/autoresearch>; Copyright Andrej Karpathy. MIT License."`

---

## RESUMEN EJECUTIVO

**30 items portables identificados** (GR-001..012, PT-001..012, IT-001..014, AR-001..013):
- ~13 son **XS (1-3h)** puros copy_uso o port trivial.
- ~8 son **S (1 día)**.
- ~7 son **M (3 días)**.
- ~6 son **L (>1 semana)**.

**Adopción agresiva**:
- graphify completo aportaría un **GraphRAG engine** (nuevo RFC derivado de 27 §C).
- intelligent-terminal aporta un **Agent Event Bus protocol** (nuevo RFC derivado de 27 §B).
- posting aporta **HTTP request/collection model** + **theme system** + **UX patterns** (RFCs 14, 24).
- autoresearch aporta **autonomous experiment loop framework** (RFC 28 §A derivado de 16).

**Recomendación top 5 PRIMERO**: AR-001 (program.md copia), AR-005 (BPB metric), PT-003 (str_presenter), IT-003 (hooks.json), IT-004 (OSC 9001 spec copy). En conjunto ~3h trabajo, sentando las bases legales+contractuales para los portados posteriores.
