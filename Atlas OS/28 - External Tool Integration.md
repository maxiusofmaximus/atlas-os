# RFC 28 — External Tool Integration

**Author:** opencode architect agent · **Date:** 2026-07-25
**Status:** Phase 1.5 — §A ✅ (`853da30`), §B ✅ (`7f8215e`-`4b4924e`), §C ✅ (`cddcbc2`), §D ✅ (`db25379`), §E ✅ items 1-8 (items 9-10 post-MVP), §F ✅ 9/9, §G ✅ 10/10, §H ✅ 13/13; §I lateral (operator guide `docs/terminal-browser-integration.md`).
**Supersedes:** none · **Superseded by:** —
**Depends on:** RFC 02 (Journal), RFC 03 (Memory), RFC 04 (Orchestrator), RFC 06/23 (Skills), RFC 12 (Planning), RFC 14 (Validation), RFC 15 (Repair), RFC 16 (Learning), RFC 19 (Execution Supervisor), RFC 22 (Research Findings), RFC 24 (HUD), RFC 25 (Stack), RFC 27 (Orchestration Fundamentals)

**Scope:** Adds four loosely-coupled integration surfaces (§A–§D) on top of RFCs 02–27, using migrations **M13/M14/M15** (free per preamble). No new external runtime dependency is bundled — three new crates, all single-binary-safe. Karpathy tweet and graphify URL honesty preserved (§C claims no first-hand tweet fetch).

### Source-derivation map (inventario portabilidad — `Atlas OS/research/28 - portable inventory.md`)

Every algorithm, file format, or pattern implemented in this RFC is derived verbatim or via translation from one of four MIT/Apache-2.0 source repos. Public attribution appears in each module's module-level prose. **No code is taken from non-OSS sources; no runtime dep on any of them stays in the binary.**

| Section | Source repo | Portable item IDs reused (full table in research file) |
|---|---|---|
| §A Autoresearch | `karpathy/autoresearch` (MIT) | AR-001 (program.md verbatim skill), AR-002/005/008/010 (constants/BPB/schedules/log format), AR-011 (results.tsv schema), AR-012 (NEVER_STOP guard) |
| §B IT ACP server | `microsoft/intelligent-terminal` (MIT) | IT-001 installer pattern, IT-002 exit-0 trap discipline, IT-003 hooks.json mapping, IT-004 OSC 9001 spec, IT-005 wtcli subcommand layout, IT-013 main.rs clap pattern, IT-014 event envelope struct |
| §C Mission graph | `safishamsi/graphify` (Apache-2.0) | GR-001 Leiden (deferred to v2), GR-003 report generator, GR-004 atomic write, GR-006 tree-sitter extractors, GR-007 affected.py SHA256 cache, GR-009 skill templates |
| §D YAML export | `darrenburns/posting` (Apache-2.0) | PT-001 RequestModel, PT-002 Collection, PT-003 str_presenter, PT-006 test_curl_export canónico, PT-010 variables.py, PT-012 urls.py |

Phase 0 (XS items, copy_uso + port trivial) batches the cheapest of these first — see "Orden recomendado" § below.

## §A — Karpathy autoresearch loop

### Source items

Derived from [`karpathy/autoresearch`](https://github.com/karpathy/autoresearch) (MIT, Copyright Andrej Karpathy):
- **AR-001** `program.md` — verbatim skill template (520 LOC markdown). Copiado 1:1 a `skills/autoresearch/program.md`. Attribution header required.
- **AR-002, AR-005, AR-008, AR-010** — constants block (`MAX_SEQ_LEN`, `TIME_BUDGET=300`, etc.), `evaluate_bpb` fixed metric, hyperparams schedules (warmup 0%, warmdown linear 50%), structured log format `---` delimiter + `grep "^val_bpb:"` parser.
- **AR-011** — `results.tsv` 5-col schema (commit, val_bpb, memory_gb, status, description).
- **AR-012** — `program.md` "NEVER STOP" guard + "rewind sparingly" hard rules + pacing prose.

### Objetivos

Implementar el hill-climbing greedy de Karpathy dentro del Execution Supervisor (RFC 19), acotado a las 4 restricciones de ecstaticpirate (HN #47399731): **scope acotado + métrica determinista + time-box + git checkpoint**. Resultado: `--autoresearch` produce runs reproducibles, persistidos y observables en HUD.

### Cambios Rust

- `src-tauri/src/journal/autoresearch.rs` (nuevo): `AutoresearchRun { id, baseline_metric, candidates: Vec<Candidate>, kept: Option<CandidateId>, ts, git_sha_start, git_sha_end }`, `Candidate { sha, diff_hunk, metric, kept: bool }`.
- `src-tauri/src/supervisor/loop.rs` (RFC 19): nuevo branch `Mode::Autoresearch` — cuando `mission.options.autoresearch = true`, el supervisor EN LUGAR de plan→code emite `prepare_only → edit → git commit → run → grep métrica → keep/reset --hard`. **El `keep` lo decide el supervisor comparando métrica numérica, no el LLM.** Cumple RFC 19 §"métrica medible".
- `src-tauri/src/cli/bin/opencode.rs`: `opencode mission new --autoresearch --metric "rg -c 'error\[E0\d+\]' src-tauri/**/*.rs | wc -l" --max-steps 50 --timebox 30m`.

### Schema SQL M13

```sql
-- M13_autoresearch_runs.sql
CREATE TABLE IF NOT EXISTS autoresearch_runs (
  id TEXT PRIMARY KEY,
  mission_id TEXT NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
  baseline_metric REAL NOT NULL,
  best_metric REAL,
  git_sha_start TEXT NOT NULL,
  git_sha_end TEXT,
  metric_command TEXT NOT NULL,
  max_steps INTEGER NOT NULL,
  timebox_seconds INTEGER NOT NULL,
  step_count INTEGER NOT NULL DEFAULT 0,
  outcome TEXT NOT NULL CHECK (outcome IN ('running','improved','plateau','timeout','aborted')),
  ts_started INTEGER NOT NULL,
  ts_ended INTEGER
);
CREATE TABLE IF NOT EXISTS autoresearch_candidates (
  id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES autoresearch_runs(id) ON DELETE CASCADE,
  step INTEGER NOT NULL,
  git_sha TEXT NOT NULL,
  diff_hunk TEXT NOT NULL,
  metric_baseline_at_step REAL NOT NULL,
  metric_after REAL NOT NULL,
  kept INTEGER NOT NULL CHECK (kept IN (0,1)),
  rationale TEXT
);
CREATE INDEX IF NOT EXISTS idx_ar_candidates_run ON autoresearch_candidates(run_id, step);
```

### HUD telemetry card

Nuevo componente `src/lib/components/AutoresearchCard.svelte` subscripto a `hudStore` canal `autoresearch:<run_id>`. Renderiza: baseline, mejor métrica, paso actual / barra de progreso, mini-sparkline (svg inline, sin lib), botones **Pause / Stop** (emiten `session/cancel`). Se añade una **quinta card type** `autoresearch` a RFC 24 §3 card taxonomy.

### Tests

- `autoresearch.test.rs`: happy path (3 candidates, 2 mejoran→kept, 1 no→reset); failure path (time-box expira → outcome=timeout); detección de doom-loop (3 resets consecutivos → supervisor aborta con `outcome=plateau`).
- Frontend: `AutoresearchCard.test.ts` (vitest) — mocking del store canal.

### Actualizaciones a RFCs existentes

- **RFC 19**: añadir §"Autoresearch mode" (métrica numérica obligatoria, fallback linear loop si `metric_command` no determinista).
- **RFC 24**: añadir card type `autoresearch` a §3 anatomy.
- **RFC 22**: añadir entry "Karpathy autoresearch pattern — hill-climbing on git commits".

### Riesgos

- **Reset --hard destruye trabajo no commiteado**: si el usuario tiene WIP, lo pierde. Mitigación: pre-check `git status --porcelain` limpio; si no, abortar con error explícito.
- **Métrica determinista es frágil**: `rg|wc -l` puede ser no determinista en Windows (lf vs crlf). Documentar explicit que la métrica corre en `bash` con la misma config que tests.
- **Doom-loop autoresearch**: 50 pasos c/u peor que baseline + git history inflamado. Mitigación: **hard ceiling 200 commits por run** y auto-squash al exportar.
- **Multi-agente branches** (Karpathy tweet 9 Mar 2026 — existe por corroboración ≥5 fuentes pero NO fue fetchado de primera mano): RFC 28 no reclama haberlo visto; solo cita la idea "emular comunidad de investigación". Implementación multi-agente queda **fuera de scope §A** — solo single-agent greedy ahora. Deferido a RFC 29 si procede.

---

## §B — Microsoft Intelligent Terminal ACP server

### Source items

Derived from [`microsoft/intelligent-terminal`](https://github.com/microsoft/intelligent-terminal) (MIT, Copyright Microsoft Corporation). **MS trademark policy**: do not brand features "Intelligent Terminal" in user-facing surfaces — call them "agent pane integration" or "ACP".
- **IT-001** `tools/wta/src/agent_hooks_installer.rs` (~7000 LOC) — idempotent installer pattern (env override → exe-dir walk → embedded `include_str!` blobs). Port shallow: copy ergonomic patterns only.
- **IT-002** `wt-agent-hooks/<cli>/wt-agent-hooks/hooks/send-event.ps1` (~250 LOC, byte-identical across subtrees) — **exit-0 trap discipline**: `trap { exit 0 }` + outer try/catch. Stdio is prompt-injection vector. 5MB diagnostic rotation threshold. CLI-source detection via args. Copy_uso verbatim of PowerShell script + our own emit-event cargo bin.
- **IT-003** `wt-agent-hooks/<cli>/hooks.json` (~70 LOC) — 10-hook domain → WTA topic mapping table. Copy_uso of schema; generate per-CLI variants programmatically.
- **IT-004** `doc/specs/llm-agent-event-integration.md` (~400 LOC) — **OSC 9001 in-band zero-dep event envelope** spec + 7 standard agent event types. Cited verbatim as spec basis.
- **IT-005** `doc/wtcli-commands.md` (~150 LOC) — 17 wtcli subcommands. Used as clap-derive enum inspiration for `atlas` CLI.
- **IT-013** `tools/wta/src/main.rs` (~4500 LOC) — `clap` subcommand dispatch reference, idiomatic `anyhow::Result` patterns.
- **IT-014** `tools/wta/src/event.rs` (~200 LOC) — `Event { r#type, method, params: serde_json::Map }` struct. **Direct port hint** — 1:1 schema with §D OSC envelope.

### Objetivos

Hacer que `atlas` sea un **ACP agent de primera clase** detectable por Intelligent Terminal 0.1+ (autodetecta OpenCode en PATH, README §Get Started). El usuario arrastra `/opencode fix`, `/opencode restart`, `/opencode exec step` en el pane. HUD Mission Control sigue siendo el surface visual canonical; el pane es *entrada ligera diaria*.

### Cambios Rust

- `src-tauri/src/acp/mod.rs` (nuevo): servidor ACP usando `agent-client-protocol` v2.0.0 con `features = ["unstable_session_fork"]` (sólo la feature v2 draft que §B necesita; las demás `unstable_*` quedan apagadas hasta que ACP v2 estabilice). El crate provee su propia transport `Stdio` para el loop JSON-RPC stdio sobre nuestro tokio existente — **no se añade** `sacp-tokio` ni `agent-client-protocol-tokio` (ver verify note abajo). Implementa métodos **required**: `initialize`, `session/new`, `session/prompt`, `session/cancel`. **Optional**: `session/load` (replay Journal M1–M12 via `session/update`), `session/resume`, `session/close`, `session/delete`, `session/list`, `session/set_mode` mapeado a estados RFC 19 (PLAN→architect, EXEC→code, REVIEW→ask), `logout`.

> **Verify note (jul 2026, Context7 fallback a `cargo info` + docs.rs):** `sacp-tokio v11.0.0` es un SDK paralelo de Symposium (repositorio `agentclientprotocol/symposium-acp`, mismo GitHub org, namespace distinto). Depende de `sacp ^11` — un parallel type universe, no re-export de `agent-client-protocol`. `agent-client-protocol-tokio` no existe en el line v2.0.0 (sólo v0.11.x, pinned a core v0.11 — incompatible). El crate core v2.0.0 ya expone `Stdio` y `Agent.builder().connect_to(Stdio::new())` — esquema espejo del ejemplo `simple_agent.rs` del repo oficial. Usar este stack canónico evita vendor lock-in con Symposium y respeta el pin `=2.0.0` original del brief.
- `src-tauri/src/acp/commands.rs`: slash commands advertised via `available_commands_update`: `/opencode mission new`, `/opencode fork`, `/opencode resume`, `/opencode exec step`, `/opencode fix [hint]`, `/opencode restart`. **`/opencode fix`** captura vía `wtcli active-pane` + `wtcli capture-pane --last-prompt`, empaqueta scrollback como `Resource` y enruta al engine **Repair** (RFC 15). **`/opencode restart`** = `session/close` + `session/new` con mismo cwd.
- `src-tauri/src/acp/delegate.rs`: `opencode exec step <mission_id> <step_id>` como delegate CLI (Alt+Shift+B en IT). Sin arg = step picker.
- `src-tauri/src/acp/mode_mapping.rs`: tabla `state → ACP mode` y `ACP mode permission options → RFC 19 §permission`.

### Schema SQL M14

```sql
-- M14_agent_session_events.sql
CREATE TABLE IF NOT EXISTS agent_session_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  ts INTEGER NOT NULL,
  pane_id TEXT,
  event_type TEXT NOT NULL,
  agent TEXT NOT NULL,
  task_id TEXT,
  payload_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_ase_ts ON agent_session_events(ts);
CREATE INDEX IF NOT EXISTS idx_ase_task ON agent_session_events(task_id);
```

Persistido por worker Rust que consume `wtcli listen --event "agent.*" --json` (Channel 2). **NO existe endpoint `127.0.0.1:<port>/mcp` en IT**, corregimos el brief original: el contrato real es `IProtocolServer` over WinRT COM (`WT_COM_CLSID` + `CoCreateInstance(CLSCTX_LOCAL_SERVER)`), surfaced through `wtcli`.

### Pane vs HUD stance

- **Pane** = chat de entrada diaria: plan checklist, tool_call chunks, deep-link a HUD en `_meta.hud_url` de `agent.task.completed`.
- **HUD** sigue siendo canonical para diffs, fork-tree, doom-loop state machine, multi-mission.
- **Decisión**: NO mover Mission Control al pane. ACP v1 no soporta diff estructurado; markdown-only es insuficiente. Deferir hasta ACP v2 `v2/diff-file-states` estabilice.

### Tests

- `acp_server.test.rs`: handshake `initialize` con capabilidades; `session/new`→`session/prompt`→`session/cancel` happy path; `/opencode fix` routing a Repair engine.
- Persist `agent_session_events` roundtrip (write→query→assert JSON payload integrity).

### Actualizaciones a RFCs existentes

- **RFC 19**: ACP `session/set_mode` queda como fuente de override de estado. Documentar cómo entra en la state machine.
- **RFC 04** (Orchestrator): el ACP server es un frontend más (al lado de HUD webview y CLI puro).

### Riesgos

- **ACP v2 aún draft**: `unstable_session_fork` rompe sin semver guarantee. Anclar `agent-client-protocol = "=2.0.0"` con `=` exact version. Cualquier bump ACP exige RFC update.
- **Windows-only**: IT es MSIX, Win10 build 19041+. En macOS/Linux, `atlas` sin IT funciona normalmente pero ACP server idles. Feature flag `acp-server` default off; on solo detectando `WT_COM_CLSID`.
- **PowerShell hooks chez IT evolucionan**: nuestra implementación debe leer `hooks-upgrade-state.json` e idempotente skip si la version bundle IT ≥ nuestra.
- **Multi-agent tracking limitation**: faq Q8 dice IT solo trackea el MISMO agent para pane + delegate. Si usuario usa delegate con CLI diferente, nuestra HUD session list no lo verá. Documentado.
- **Token cost de replay**: `session/load` reenviar Journal M1–M12 al pane completo puede ser Mbytes. Mitigación: payload truncation con `outputByteLimit` del spec.

---

## §C — graphify pattern adoption

### Source items

Derived from [`safishamsi/graphify`](https://github.com/Graphify-Labs/graphify) (Apache-2.0, Copyright Graphify Labs):
- **GR-001** `cluster.py` (260 LOC) — Leiden via graspologic with Louvain fallback + oversized split >25% + cohesion re-split <0.05 + hub exclusion percentile. **Deferred to RFC v2** — no mature Rust Leiden impl (would require FFI to `leidenalg` C or building impl O(N log N)); §C uses `petgraph` union-find for connected components until Leiden port unblocks.
- **GR-003** `report.py` (180 LOC) — `GRAPH_REPORT.md` generator with tagged `EXTRACTED`/`INFERRED`/`AMBIGUOUS` provenance sections. Ported as `src-tauri/src/graph/report.rs`.
- **GR-004** `paths.py` (200 LOC) — atomic write (temp+rename, Windows fallback copy), test-path classifier (regex), tiebreaker disambiguate (bare-call by test/non-test + path proximity). Ported as `src-tauri/src/journal/atomic.rs`.
- **GR-006** `extract.py` (244K, ~6000 LOC) — tree-sitter bindings (Python/TS/JS/Go/Rust/Java/C/C++/Ruby/C#) AST→call-graph, cross-file resolution, `find_import_cycles`. Ported to Rust using `tree-sitter` + `tree-sitter-rust` + `tree-sitter-svelte` crates. **Scoped down**: AST + cycle detector only; callflow_html deferred to v2.
- **GR-007** `affected.py` (250 LOC) — SHA256 file cache + re-extract only changed + merge into existing graph. Ported as `src-tauri/src/graph/cache.rs`.
- **GR-009** `skill-agents.md` + `skill-claw.md` (20K Markdown each) — skill templates for 6 CLIs. **Copy_uso** as base templates for `skills/graphify/<cli>.md` (RFC 23 §7).

### Objetivos

Adoptar el **patrón** (NO la dependencia Python — graphify es Python, viola RFC 25 §11 / AGENTS.md §6). Cuatro usos internos: (1) RFC 19 state machine como DAG navegable en SQLite; (2) HUD `<GraphView>`; (3) skills como graph templates (RFC 23); (4) Planner emite DAG. Un quinto uso **opcional externo**: usuario con graphify MCP HTTP ya instalado puede apuntar Atlas OS skill-backend a él — **zero binary weight**.

### Cambios Rust

- `src-tauri/src/graph/mod.rs` (nuevo): `MissionGraph { nodes: Vec<Node>, edges: Vec<Edge> }`, `Node { id, kind, label, provenance: EXTRACTED|INFERRED|AMBIGUOUS, attrs_json }`, `Edge { src, dst, kind: calls|imports|transitions_to|depends_on, precondition, guard, visit_count }`.
- `src-tauri/src/graph/traverse.rs`: `shortest_path`, `god_nodes`, `get_neighbors`, `community_partition` (**comunidades via union-find simple, NO Leiden** — ver Riesgos). Usa `petgraph` (MIT/Apache-2.0, pure Rust).
- `src-tauri/src/graph/ast.rs`: `tree-sitter` + 2 grammars (`tree-sitter-rust`, `tree-sitter-svelte`) para `EXTRACTED` edges del propio codebase de Atlas OS como meta-skill. Justification single-binary-safe: tree-sitter es C puro linked estáticamente, ya usado por muchos crates Rust.
- `src-tauri/src/hud/server.rs`: nuevo endpoint `GET /hud/graph/:mission_id` devuelve JSON directo desde SQLite. **Sin infra nueva**.
- `src-tauri/src/planner/graph_emitter.rs` (RFC 12 refactor): Planner emite JSON matching `mission_graph` — un **DAG**, no lista lineal. Branching when Coder subagent multiple paths; Validator scores branches; Repair rewrites edges failing. **Detrás de feature flag `dag_mode`** (default off) — lineal por defecto, migrar gradualmente.
- `src-tauri/src/skills/loader.rs`: 4th skill file `graph.toml` declarando sub-graph template (nodes+edges+preconditions). Load = insert en `mission_graph` con fresh IDs.

### Schema SQL M15

```sql
-- M15_mission_graph.sql
CREATE TABLE IF NOT EXISTS mission_graph_nodes (
  id TEXT PRIMARY KEY,
  mission_id TEXT NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('engine_state','mission','skill','external')),
  label TEXT NOT NULL,
  provenance TEXT NOT NULL CHECK (provenance IN ('EXTRACTED','INFERRED','AMBIGUOUS')),
  attrs_json TEXT
);
CREATE TABLE IF NOT EXISTS mission_graph_edges (
  id TEXT PRIMARY KEY,
  mission_id TEXT NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
  src TEXT NOT NULL REFERENCES mission_graph_nodes(id) ON DELETE CASCADE,
  dst TEXT NOT NULL REFERENCES mission_graph_nodes(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('calls','imports','transitions_to','depends_on','references')),
  precondition TEXT,
  guard TEXT,
  visit_count INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_mg_edges_src ON mission_graph_edges(src);
CREATE INDEX IF NOT EXISTS idx_mg_edges_mission ON mission_graph_edges(mission_id);
CREATE TABLE IF NOT EXISTS learning_graphs (
  id TEXT PRIMARY KEY,
  intent_signature TEXT NOT NULL,
  success INTEGER NOT NULL CHECK (success IN (0,1)),
  graph_json TEXT NOT NULL,
  created_ts INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_lg_sig ON learning_graphs(intent_signature);
```

### Front SvelteKit

- `src/lib/components/GraphView.svelte` (nuevo): renderiza `mission_graph` con **d3-force** o `vis-network`. Cards = nodos, transitions = edges, doomed node → red. Click → fly-to-card (toggle con cascade existente).
- `src/lib/stores/graph.ts`: `graphStore` writable, fetch `/hud/graph/:id`, reconcile contra `hudStore` cuando cambian visitas.
- Tests: `GraphView.test.ts` (vitest + DOM mock), `graphStore.test.ts`.

### Actualizaciones a RFCs existentes

- **RFC 19**: states stored as DAG; doom-loop switcher usa `shortest_path(current, healthy_state)`.
- **RFC 24**: nueva sección §"Graph View" junto a cascade; toggle viewport.
- **RFC 23**: skills gain 4th file `graph.toml`; actualiza §7.2.
- **RFC 12**: Planner output es DAG cuando `dag_mode=true`.
- **RFC 16**: persist successful graphs en `learning_graphs`, retrieve top-k por cosine (`sqlite-vec` + `fastembed-rs` ya presentes), diff edges como `INFERRED` hints.
- **RFC 22**: entries para `petgraph`, `tree-sitter`, `tree-sitter-rust`, `tree-sitter-svelte`.

### Riesgos

- **Tres crates suman binario**: `petgraph` ~200 KB, `tree-sitter` ~3 MB (incl C runtime), cada grammar ~500 KB–2 MB. Binary Tauri crece ~5–8 MB. Aceptable para desktop, borderline para uso server. Feature flag `codebase-graph` default off si tight.
- **Leiden en Rust**: no crate madura (solo bindings Python). Implementación in-file pequeña es maintained burden real. Alternative: `petgraph`'s union-find para componentes conexas, defer Leiden a v2. **Decisión presente: comunidades via union-find simple, NO Leiden.**
- **Learning graphs overfitting**: similarities de `intent_signature` pueden colapsar policies. Mitigación: confidence decay + min 5 instancias para adoptar.
- **graphify MCP HTTP externo opcional**: bien, pero si usuario lo pierde/desactualiza, nuestra skill dangling. Mitigación: health-check `initialize` al cargar skill y degradar limpiamente.
- **Planner DAG refactor es grande**: riesgo scope creep. **emitir DAG sólo si mission.flag `dag_mode=true`**, lineal por defecto; migrar gradualmente.
- **AMBIGUOUS tag deferred to user**: cuando Planner confidence bajo, produce nodo AMBIGUOUS y HUD "steer" button. Si user no decide, mission stalls. Mitigación: timeout → algoritmo default greedy.

---

## §D — AuditLog YAML-on-disk export (posting format)

### Source items

Derived from [`darrenburns/posting`](https://github.com/darrenburns/posting) (Apache-2.0, Copyright Darren Burns):
- **PT-001** `src/posting/collection.py:1-260` `RequestModel` (+ `Auth`, `Header`, `QueryParam`, `Cookie`, `Options`, `RequestBody`, `Scripts`) — schema completo `.posting.yaml`. Ported via `serde` + `serde_yaml` with `Option<T>` fields + `#[serde(default)]`. `apply_template` (string.Template vars), `to_httpx`, `to_curl`, `save_to_disk`.
- **PT-002** `collection.py::Collection.from_openapi_spec/from_directory/save_to_disk` (80 LOC) — OpenAPI 3.0 import → Collection tree. **Not needed for §D export** — defer to future if we ever import external collections (security boundary AGENTS.md §6).
- **PT-003** `src/posting/yaml.py:1-26` `str_presenter` — YAML scalar representer: if string has `\n`, emit as literal block `|` with `rstrip()` per line. **Oro puro** — explains diff-friendliness. Ported as `src-tauri/src/journal/yaml_format.rs::literal_block`. `serde_yaml` does not support custom representers directly → wrapper impl manual.
- **PT-006** `tests/test_curl_export.py` (7.7K) + `tests/test_curl_import.py` (8.5K) — canonical tests round-trip curl→RequestModel→curl, edge cases (URL with query, form_data with [[]], auth basic/digest, empty body). Ported directly as `#[cfg(test)] mod tests` in `request_model.rs` — same edge cases.
- **PT-010** `src/posting/variables.py` (150 LOC) — `${name}` substitution with clear `SubstitutionError` when missing. env override semantics. Ported as `src-tauri/src/profiles/variables.rs`.
- **PT-012** `src/posting/urls.py` (80 LOC) — `ensure_protocol` (prepend `http://` if missing), `substitute_path_params` (`:param` replaced in URL path). Ported into `request_model.rs::ensure_protocol`.

### Objetivos

Cerrar el gap implícito de audit log retention/snapshot portability (mencionado en research como "Brecha H" — ver apéndice; RFC 27 §3.H real es "Spec document legible", no audit retention). Export entries a `.posting.yaml` antes de purgar SQLite. **NO hay dependencia runtime a posting**; solo formato. El usuario abre el yaml con `posting --collection ./snapshots/` si tiene posting, o lo versiona en git como YAML crudo.

### Cambios Rust

- `src-tauri/src/journal/export/posting.rs` (nuevo): `fn entry_to_yaml(entry: &AuditEntry) -> String` con `serde_yaml` + helper `literal_block` (equivalente a `str_presenter` de posting). **Ignora el campo `scripts`** al escribir (los entries no tienen scripts; si importamos collections de terceros, también ignora — security boundary AGENTS.md §6).
- `src-tauri/src/journal/export/retention.rs` (nuevo): worker SQLite hook; antes de purgar entries `age >= ttl_days`, exporta a `~/.opencode/snapshots/YYYY-MM-DD/*.posting.yaml`.
- `src-tauri/src/cli/bin/opencode.rs`: `opencode audit --export-posting -n 50 -o ./snapshots/`. Hardcodeamos `posting_version: "1"` con comentario `# x-atlas-exported: RFC 28 §D`. Si posting sube major, bump manual.
- Mapeo: `http_request` entry → `RequestModel` 1:1. `tool_call` non-HTTP → omits `method/url`, usa extension keys bajo `x-opencode-*` (YAML permite keys arbitrarias).

### Comandos CLI nuevos

- `opencode audit --export-posting -n N -o DIR` (one-shot)
- `opencode audit --snapshot-maybe` (invoca retention worker manualmente; cron planificado en supervisor)

### Front SvelteKit

Mínimo: botón **Export as posting** en card Audit (HUD §3). Componente `src/lib/components/SnapshotExport.svelte` que llama endpoint axum `POST /audit/export-posting`. Store sin cambios (file dialog handled por Tauri save dialog nativo).
- Tests: `posting_yaml.test.ts` (parse roundtrip, strip `scripts`).
- Rust test: `posting.rs` happy path (http_request entry → valid posting YAML validated en check structural), failure path (tool_call entry omit method/url gracefully).

### Actualizaciones a RFCs existentes

- **RFC 24 §10 AuditLog**: añadir sección "YAML on-disk export (posting-format compatible)".
- **RFC 27 §3.H Brecha H**: este RFC originalmente listaba "Audit log retention/TTL" como el gap que §D cerraría. Al inspeccionar RFC 27 §3.H se observa que la brecha realmente documentada allí es "Spec document legible" (Prompt Understanding), no audit retention. Por tanto §D **NO cierra §3.H**; subsana un gap implícito (falta de un formato portable on-disk para el audit log) que no estaba formalizado como brecha. Se deja §3.H sin cerrar; se abre una **Brecha I** provisional en RFC 27 §3.J para formalizar "Audit log retention + on-disk portable export" si hace falta, con §D como su cierre — o bien se acepta que el gap era implícito y §D simplemente añade la capacidad sin invocar cierre formal.
- **RFC 25 §3.4**: añadido `serde_yaml = "0.9"` en `src-tauri/Cargo.toml` (sección "Misc") — no rompe single-binary distribution; ~300 KB adicional. Crate pasivo (maintainer no añade features), ver §Riesgos §5 abajo para contingencia con `serde_yml`.
- **RFC 23 §7.2**: skills pueden shippear `requests/*.posting.yaml` bundles — el parser RFC 23 **requiere reject del campo `scripts`** explícitamente (security boundary AGENTS.md §6).

### Riesgos

- **`posting_version` drift**: si posting rompe compatibilidad en major, snapshots antiguos no cargan. Mitigación: persistir el `posting_version` dentro de cada snapshot y un `README.md` en el dir snapshot documentando versión requerida.
- **YAML literal-block fidelity**: nuestro `str_presenter` puede diferir del de pydantic. Mitigación: test contra un `.posting.yaml` sample canónico leído de `raw.githubusercontent.com/darrenburns/posting/main/...`.
- **Volume**: un audit con 10k entries genera 10k archivos yaml si uno por entry — mala UX. Mitigación: **bundle en collection subdirs by day**, un archivo por entry-directory (max 100 entries/arch) — decisión de packing.
- **No usar hurl**: decidido. `.hurl` DSL es asserts-oriented, pierde semántica del journal typed. Documentemos como alternativa rechazada en RFC 22.
- **Import collections de terceros**: superficie ataque. Aunque ignoremos `scripts`, el YAML puede contener URLs/auth en cleartext. Mitigación: parser Rust no reenvía nada; snapshot dir es local-only.
- **Detalles auth enum**: `Literal["basic","digest","bearer_token"]` enumerado necesita struct explicita.

---

## §E — Firecrawl: web ingestion polyfacética (post-graphify)

### Source items

- **firecrawl/firecrawl** (<https://github.com/firecrawl/firecrawl>), MIT — plataforma open-source de web scraping/crawling/search. Mantiene un **SDK Rust oficial y first-party** en la crate `firecrawl = "2.12.1"` (<https://crates.io/crates/firecrawl>, MIT, maintainers `mogery` + `rafaelsideguide` = equipo Firecrawl). NO hay que portar nada: la crate ya existe, está publicada en crates.io, y se actualiza en paralelo con el SDK Node/Python del mismo repo.
- **firecrawl/firecrawl-mcp-server** (<https://github.com/firecrawl/firecrawl-mcp-server>), MIT — MCP server oficial (Node, 7.1k★) que expone `scrape`/`search`/`crawl`/`map`/`extract`/`agent`/`interact`/`monitor`/`research_*`. documenta el **contrato de herramientas** que nuestra facade debe replicar cuando se exponga vía MCP.
- **firecrawl-mcp = "0.7.1"** (<https://crates.io/crates/firecrawl-mcp>, MIT, community `washanhanzi`) — SDK Rust para construir MCP servers firecrawl nativamente sin Node. Features `default = [batch-scrape, crawl, map, scrape, search]`, `self-host`. **Opcional/post-MVP**: usar sólo si decidimos emitir un MCP server propio; si no, basta con `firecrawl = "2"` directa.
- **Hosted MCP endpoint** (`https://mcp.firecrawl.dev/v2/mcp`) — instancia gestionada que los clientes MCP pueden apuntar sin instalar nada. Keyless free tier cubre `scrape`/`search`/`interact`. Nosotros no lo worshipeamos internamente (no external runtime dep) pero lo citamos como referencia de la superficie canónica.

### Por qué polyfacética — el patrón "adapter facade"

Firecrawl reemplaza dos superficies de capacidad que el runtime opencode usa hoy por separado:

1. **`webfetch` builtin** (tool investigación durante sesiones) — limitado a una URL, sin JavaScript rendering, sin structured extraction, sin rate-limit handling. Firecrawl lo supera en todos los ejes.
2. **Ingest pipeline de graphify** (futuro): cuando `codebase-graph` lee repos y `dag_mode` emite grafos de misión, un componente de "research externo" puede enriquecer nodos con metadata scrapeada (READMEs upstream, issue trackers, papers citados en comentarios). Hoy esto no existe; firecrawl lo habilita.

El patrón profesional para integrar una herramienta polyfacética que sirve a N consumers heterogéneos (MCP server, CLI `research`, graphify ingest, `webfetch` fallback, future Skills) es el **adapter facade**: un módulo Rust interno expone una superficie unificada y todos los consumers llaman a esa facade (no a la crate firecrawl directo). La facade abstrae: credenciales (`FIRECRAWL_API_KEY` / OAuth bearer / keyless fallback), rate-limit + retry exponencial, redacción PII (`redactPII: true`), normalización a tipos comunes (`ScrapedDocument { url, markdown, title, metadata }`, `SearchResult { url, title, snippet, highlights }`, `CrawlBatch { docs }`, `ExtractResult { json }`), y fallback graceful a `webfetch` cuando no hay API key o el feature flag está OFF.

Esto evita el anti-patrón "cada consumer llama firecrawl directo con su propio error handling y duplicación de credenciales". La crate `firecrawl = "2"` sigue siendo una dependencia opcional gated tras `firecrawl` feature, pero su API surface se consume **única y exclusivamente** vía `crate::firecrawl::facade`.

### Objetivos

1. Añadir feature flag `firecrawl` (default OFF) a `Cargo.toml`, con dependencia `firecrawl = { version = "2", optional = true }`. Cumple AGENTS.md §4 (no new dep sin RFC first — este §E es el RFC).
2. Implementar `src-tauri/src/firecrawl/{mod, facade, client, error}.rs` (gated `#[cfg(feature = "firecrawl")]`):
   - `facade.rs` — tipos canónicos + traits: `async fn scrape_url(url, opts) -> Result<ScrapedDocument>`, `search_web(query, opts) -> Result<Vec<SearchResult>>`, `crawl_site(url, limit) -> Result<CrawlBatch>`, `extract_structured(urls, schema) -> Result<ExtractResult>`. Cada fn maneja credenciales, retry, redacción internamente.
   - `client.rs` — wrapper fino sobre `firecrawl::Client` (singleton lazy-static con `FIRECRAWL_API_KEY` desde env o `profiles::Profile::secret`).
   - `error.rs` — enum `FirecrawlFacadeError { MissingApiKey, RateLimited, Network, Api(FirecrawlError) }` con `thiserror`.
3. Sub-comando CLI `atlas research` (gated `firecrawl`): `opencode research scrape <url>`, `opencode research search <query>`, `opencode research crawl <url> --limit N`. Usa `facade`. Output a stdout en JSON line-delimited (consumible por pipes).
4. **MCP server nativo Rust** (opcional, post-§E MVP): si perfilamos y Node `firecrawl-mcp` startup cost duele, escribimos `src-tauri/src/firecrawl/mcp.rs` usando `firecrawl-mcp = "0.7.1"` SDK Rust para servir MCP tools sobre stdio. Decisión postergada hasta medir cuello de botella — primer corte: perfil hosted keyless endpoint o subprocess Node.
5. Reemplazar todas las llamadas internas a `webfetch` por `facade::scrape_url` cuando el feature esté ON; si OFF, mantener `webfetch` como fallback hardcoded (no romper single-binary invariant).
6. Cerrar el loop graphify: cuando `dag_mode` + `codebase-graph` + `firecrawl` co-ocurren, un `graph_ingest` step opt-in puede enriquecer nodos con metadata scrapeada. Spec detalle se añade a RFC 16 §3 (structural graph diffing) post-§E MVP.

### Cambios Rust (plan, no implementación)

- `Cargo.toml` — `[features] firecrawl = ["dep:firecrawl"]`, `firecrawl = { version = "2", optional = true, default-features = false }`. NO subir a default.
- `src-tauri/src/firecrawl/mod.rs` — `pub mod facade; pub mod client; pub mod error;` con subtree todo gated `#[cfg(feature = "firecrawl")]`.
- `src-tauri/src/lib.rs` — `#[cfg(feature = "firecrawl")] pub mod firecrawl;`
- `src-tauri/src/cli/commands/research.rs` (nuevo, gated) — sub-comando `opencode research {scrape|search|crawl}`.
- `src-tauri/src/cli/commands/mod.rs` — `#[cfg(feature = "firecrawl")] pub mod research;` + clap subcmd registration gated.
- Sin migración SQL (§E no toca journal). Sin frontend changes a menos que el HUD quiera mostrar "firecrawl active" badge (deferido).

### Comandos CLI nuevos (plan)

```bash
opencode research scrape https://docs.firecrawl.dev --format json
opencode research search "rust async patterns" --limit 5 --highlights
opencode research crawl https://example.com/blog --limit 50
# gated behind `firecrawl` feature; build without flag = subcommand absent, clap returns "no such command"
```

### Front SvelteKit

Ninguno en MVP. Posible futuro: badge "firecrawl connected" en HUD settings panel;(signature `firecrawl_used: true` en mission metadata, optional mostrar en `<MissionCard>`). Deferido.

### Actualizaciones a RFCs existentes (plan)

- **RFC 25 §3.2** — añadir `firecrawl = "2"` (optional, gated `firecrawl` feature) a la lista de crates justificados. NO entra en "core required" — queda fuera del default build.
- **RFC 22 §11 Round 4** — audit exhaustivo de `firecrawl = "2.12.1"`: licensia, maintainers, MSRV, deps transitivas, binary-size, single-binary-safety. (Añadido por separado en este commit.)
- **RFC 16 §3** — placeholder: "structured graph diffing MAY enqueue scrape enrichment when feature `firecrawl` está ON; spec completo tras §E MVP".
- **Este archivo (RFC 28 §E)** — checklist de implementación (ver abajo).
- **RFC 26** — cross-ref rows nuevos (`firecrawl facade`, `atlas research`).

### §E Checklist (no ejecutar hasta post-graphify)

1. ✅ `Cargo.toml` feature `firecrawl` + dep `firecrawl = "2.12.1"` (optional, `default-features = false`). `cargo check --features firecrawl` limpio.
2. ✅ `src-tauri/src/firecrawl/{mod, error}.rs` — `FirecrawlFacadeError` enum (7 variantes) + módulo skeleton (atribución MIT/Mendable en module prose).
3. ✅ `src-tauri/src/firecrawl/client.rs` — `FirecrawlClient::from_env()` / `::from_explicit()` con `FirecrawlEndpoint::{Cloud, SelfHosted}` y `FirecrawlKey::{Bearer, None}`; honra `FIRECRAWL_API_KEY` + `ATLAS_FIRECRAWL_URL`. 8 tests env-var mutex.
4. ✅ `src-tauri/src/firecrawl/facade.rs` — `scrape_url`, `search_web`, `crawl_site`, `extract_structured` con redacción PII (email/phone/card) default ON; DTOs SDK-agnostic (`ScrapedDocument`, `SearchResult`, `CrawlBatch`, `ExtractResult`); builders de opciones y traductores de tipos SDK→facade. 18 tests + 1 `#[ignore]` e2e.
5. ✅ `src-tauri/src/cli/commands/research.rs` — sub-comando clap `ResearchCmd` (`scrape|search|crawl|extract`) wired en `atlas` CLI headless. 7 tests clap parsing + DTOs JSON output.
6. ✅ Integración `webfetch` fallback path: el CLI responde con un `FirecrawlFacadeError::MissingApiKey` claro cuando la feature está ON pero no hay key; con feature OFF el sub-comando no se compila (monomorphization skipped, no runtime cost). El fallback de webfetch hacia el orchestrador interno se spec-añadirá post-§E MVP (RFC 23 §4 hook).
7. ✅ Tests e2e optativos: `scrape_url_e2e` `#[ignore]` tras `FIRECRAWL_API_KEY`, no corren en CI sin key.
8. ✅ Docs + atribución: module-level prose citando firecrawl MIT (Copyright Mendable AI Inc.) en `src/firecrawl/mod.rs` + apéndice atribución abajo.
9. ⏳ Decisión post-MVP: MCP server nativo Rust vs subprocess Node vs hosted endpoint. Bloqueador: medir startup cost + drift SDK. No incluido en commit `c228e4a`.
10. ⏳ Cierre graphify: `graph_ingest` enrichment spec en RFC 16 §3. No incluido en commit `c228e4a`.

**Commit: `c228e4a`** — `feat(rfc-28): Section E items 1-6 — Firecrawl adapter facade + CLI research subcommand`. 10 files changed, 1560 insertions. Tests: default 268, default+firecrawl 306+1 ignored. `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` + `cargo clippy --all-targets --features firecrawl -- -D warnings`: todos limpios.

### Riesgos

1. **API key distribution**: firecrawl cloud requiere key. Self-host posible pero añade infra. Mitigación: keyless free tier cubre scrape/search/interact (rate-limited); key sólo para crawl/map/extract/agent.
2. **Vendor lock-in**: facade abstrae el SDK; si firecrawl cambia breaking v3, sólo `client.rs` se retoca, no los consumers.
3. **Cold-start `firecrawl::Client`**: construirla en primer call async. Mitigación: lazy singleton con `OnceCell`.
4. **Rate limits**: facade retry exponencial + jitter; propagar `429` al caller como `FirecrawlFacadeError::RateLimited` paraque Skills/Planner puedan pausar.
5. **Tests con red**: ningún test unitario toca la red real. `mockito` (dev-dep existente en firecrawl crate, no en nuestro tree) o `wiremock` para mock HTTP. Tests e2e `#[ignore]` con envvar opt-in.
6. **Single-binary invariant**: `firecrawl = "2"` es pure Rust (reqwest/serde/tokio), zero native deps. Cumple AGENTS.md §6. SI usamos `firecrawl-mcp = "0.7.1"` para MCP server, igual (Rust puro). NO introducimos Node runtime en el binario.
7. **Ordering vs graphify**: §E depende de §C MVP completo (ya ✅) PERO no bloquea en graphify runtime — el enriquecimiento `graph_ingest` es un follow-up opt-in. §E puede shipparse sin `dag_mode` + `codebase-graph`. Razón de "post-graphify": (a) prioridad de producto, no técnica; (b) queremos medir `firecrawl` usage desde graphify primero para afinar facade API.

### Atribución (apéndice)

- `src-tauri/src/firecrawl/{mod, facade, client, error}.rs` — `"Uses the official Rust SDK firecrawl = \"2\" published by the Firecrawl team (MIT). Copyright Mendable AI Inc."`. NO porta código fuente desde firecrawl/firecrawl-mcp-server (Node) — usa la crate Rust first-party.
- Si se usa `firecrawl-mcp = "0.7.1"`: `"Rust SDK firecrawl-mcp by washanhanzi (MIT). Copyright washanhanzi."`.

---

## §F — Windows Toast Notifications (Phase 1.5f)

**Status: ✅ 9/9 implementados** (items 1-8 `d6e6e23`; item 9 `tools/toast-smoke.ps1` + `docs/toast-integration.md`, verified against a `--features toast` build).

Atlas OS corre como shell desktop (Tauri 2) pero también como headless CLI/ACP server. Cuando el proceso está ocioso o el webview está minimizado, las notificaciones nativas del SO son el canal correcto para señales asíncronas: reset-window de modelo, fin de turn largo, fallo crítico, calendar reminder. Esta sección define cómo Atlas OS configura AUMID + Start Menu shortcut y dispara Toasts via `winrt-toast-reborn`, persiste historial en SQLite, y responde a activaciones (deep-link al HUD).

### F.1 Crate decision

| Crate | Version | License | Rol |
|---|---|---|---|
| `winrt-toast-reborn` | `0.3.8` | MIT | Toast activation, AUMID registration, event handlers (Windows 10/11) |

**`winrt-toast-reborn`** es el fork mantenido de `winrt-toast 0.1.1` (crates.io ID `winrt-toast-reborn`, author `AtifChy` / Md. Iftakhar Awal Chowdhury, repo `https://github.com/AtifChy/winrt-toast.git`). Publicado `2025-09-01`, 16k downloads, MIT. Expone:

- `Toast::new(&AumId).with_title(...).with_text_body(...).show()?` — disparo síncrono.
- `register(&AumId, &DisplayName, &IconPath)?` — crea Start Menu shortcut + registry AUMID, requisito para activación silenciosa desde proceso no-MSIX.
- `on_activated(|action| ...)`, `on_dismissed(|reason| ...)`, `on_failed(|error| ...)` — callbacks por toast, usados para deep-link al HUD y persistencia de dismiss.
- Sin API para `ScheduledToastNotification` (WinRT appointment-like scheduling). El scheduler propio se construye sobre `tokio::time::sleep_until(expiry)` + SQLite (ver F.3).

**`tauri-plugin-notification`** RECHAZADO como primario en Windows: bug de heurística AUMID ([tauri-apps/plugins-workspace#1545](https://github.com/tauri-apps/plugins-workspace/issues/1545)) hace que el schedule se vuelva silent no-op en desktop non-MSIX, y no devuelve callback de activación (solo fire-and-forget). Se retiene **solo para Linux/macOS** donde los Toasts nativos del SO no requieren AUMID y la activación deep-link no aplica (CLI/desktop WebSocket HUD ya está escuchando).

**Implementación final (commit `d6e6e23`)**: Elimina `tauri-plugin-notification` del plan de dependencias; la feature `toast` incluye unicamente `winrt-toast-reborn` en `cfg(windows)`. En non-Windows, el dispatcher se degrada a `ToastDispatcher::Stub` que logueea via `tracing::info!` y devuelve `Dismissed{timedOut}` — suficiente para pruebas end-to-end del driver loop sin WinRT. WinRT `ToastManager` es **no-Send**, así que el dispatcher Windows usa un std::thread dedicado con mpsc channel; el driver tokio invoca via `tokio::task::spawn_blocking` para mantener la future `Send`.

### F.2 AUMID registration (one-time)

```rust
const AUMID: &str = "dev.opencode.OpenCodeOS.HUD";
const DISPLAY_NAME: &str = "Atlas OS — Mission Control";
const ICON_PATH: &str = "icons/icon.png"; // bundle-relative

winrt_toast_reborn::register(AUMID, DISPLAY_NAME, ICON_PATH)?;
```

Idempotente: si el shortcut existe, plutó corregir `DisplayName`/icon path sin duplicar. Corre una vez en `AppState::new()` (desktop+CLI) si `cfg!(windows)`. En Linux/macOS es no-op (la feature `winrt-toast-reborn` no compila fuera de Windows — se gatea con `#[cfg(windows)]`).

### F.3 Toast scheduler (database-driven)

Las notificaciones scheduled NO usan WinRT `ScheduledToastNotification` (ninguna crate Rust lo expone + activeWinRT appointment APIs requieren `appointmentsSystem` capability, ver §G). En cambio, un **driver tokio** consume la tabla SQLite `toast_queue` y dispara `Toast::show()` al expirar el `fire_at`:

```sql
CREATE TABLE toast_queue (
  id            INTEGER PRIMARY KEY,
  kind          TEXT NOT NULL,       -- 'model_ready' | 'turn_end' | 'validation_failed' | 'calendar_reminder'
  title         TEXT NOT NULL,
  body          TEXT,
  deep_link     TEXT,                -- p.ej. "opencode://mission/{mission_id}/card/{card_id}"
  fire_at       INTEGER NOT NULL,    -- unix millis
  status        TEXT NOT NULL DEFAULT 'pending', -- 'pending' | 'fired' | 'dismissed' | 'failed'
  fired_at      INTEGER,
  dismissed_at  INTEGER,
  dismiss_reason TEXT,               -- 'userCanceled' | 'applicationHidden' | 'timeout' | ...
  attempts      INTEGER NOT NULL DEFAULT 0,
  created_at    INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX toast_queue_pending_idx ON toast_queue(fire_at) WHERE status = 'pending';
```

**Driver loop** (corre como `tokio::spawn` desde `AppState::new()`, cadencia 5s):

```rust
loop {
    let now = unix_millis();
    let next = queue.next_pending(now)?;          // SELECT ... WHERE fire_at <= now AND status='pending' LIMIT 1
    let sleep_ms = match next {
        Some(row) => 0.max(row.fire_at - now),
        None     => 5_000,                        // idle 5s si no hay pendientes
    };
    tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
    if let Some(row) = next {
        fire_toast(&row).await?;                   //Toast::show() + update status='fired'
    }
}
```

Esto garantiza: (a) notificaciones sobreviven crashes (están en SQLite, no en RAM), (b) se pueden backfill en boot, (c) no dependen de un scheduler del SO. Un `VacuumHook` (RFC 16 §3) purga toasts con `status in ('fired','dismissed','failed')` > 30 días.

### F.4 Activación deep-link

Cuando el usuario hace click en el Toast o en un action button (p.ej. "Open Mission"), `on_activated` corre en el mismo proceso (el que registró el AUMID). El handler:

1. Parsea `deep_link` (formato `opencode://...`).
2. Si el HUD tiene webview abierto: envía evento por Kernel Bus (`hud:deep_link`).
3. Si CLI/headless: arranca `pnpm tauri:dev` via `Command::new` (o en MSI/MSIX package: `ShellExecute` con el protocolo `opencode://`).
4. Marca `toast_queue.status='fired'`, `dismissed_at=now`, `dismiss_reason='activated'`.

### F.5 Toast history (audit + dedupe)

Cada `fire_toast` escribe en `toast_history` (tabla append-only, sin cleanup) para auditoría y para Idempotency de reintentos. `kind`+`mission_id`+`fire_at`(±5min) como clave de unicidad: si el proceso reinicia y encuentra un `pending` ya `fired` en `toast_history`, lo marca `status='fired'` y no lo re-dispara.

### F.6 Plan de cambios Rust

| Archivo | Cambio |
|---|---|
| `src-tauri/Cargo.toml` | Añadir feature `toast-notifications` (default OFF). `winrt-toast-reborn = { version = "0.3.8", optional = true }` (`target_os = "windows"`). `tauri-plugin-notification = { version = "2.3.3", optional = true }` (non-Windows). |
| `src-tauri/src/toast/mod.rs` | Módulo raíz. Re-exporta `ToastQueue`, `ToastDriver`. `#[cfg(windows)]` gatea `winrt-toast-reborn`. |
| `src-tauri/src/toast/queue.rs` | `ToastQueue` con `enqueue`, `next_pending`, `mark_fired`, `mark_dismissed`, `mark_failed`. Backed por `journal::toast_queue`. |
| `src-tauri/src/toast/driver.rs` | `ToastDriver` — tokio task loop, `fire_toast()`, `on_activated`/`on_dismissed` handlers. |
| `src-tauri/src/toast/aumid.rs` | `register_aumid()` idempotente (Start Menu shortcut + registry). |
| `src-tauri/src/toast/history.rs` | `toast_history` CRUD + dedupe. Comparte `journal` conn. |
| `src-tauri/src/journal/schema.rs` | M17 migration: `toast_queue` + `toast_history` tables. Schema version 16→17. |
| `src-tauri/src/core/state.rs` | `AppState::new()` llama `toast::aumid::register_aumid()` (Windows) y arranca `ToastDriver::spawn()`. |
| `src-tauri/src/lib.rs` | `#[cfg(feature = "toast-notifications")] pub mod toast;` |

### F.7 Checklist

- [x] **Item 1**: `Cargo.toml` feature flag `toast` + crate dep `winrt-toast-reborn 0.3.8` (Windows-only via `[target.'cfg(windows)'.dependencies]`). `tauri-plugin-notification` retirado del plan (bug #1545 + no callback surface); CLI subcommand `opencode toast queue/list/cancel` añadido.
- [x] **Item 2**: M17 migration `toast_queue` + `toast_history` (schema 16→17) in `src/journal/schema.rs`. Runs unconditionally — schema version is 17 regardless of feature flag.
- [x] **Item 3**: `src/toast/manager.rs::register_aumid()` idempotente (Windows): wraps `winrt_toast_reborn::register(AUMID, DISPLAY_NAME, icon_path)`. Non-Windows no-op. 1 test (non-Windows) + failure logged gracefully.
- [x] **Item 4**: `src/toast/queue.rs::ToastQueue<'a>` — `enqueue`, `next_pending`, `count_pending`, `mark_fired`, `mark_dismissed`, `mark_failed`, `get`, `cancel`, `list`, `append_history`. 11 tests (happy-path + dup + cancels + history-append + descending list).
- [x] **Item 5**: `src/toast/scheduler.rs::ToastDriver` — tokio spawn task, 5-s idle poll, dispatch via `tokio::task::spawn_blocking` (WinRT `ToastManager` no-Send), persist outcome back. 3 tests (non-Windows): fires-one, drains-backlog, handle-is-some.
- [x] **Item 6**: `toast_history` dedupe ledger implemented in `ToastQueue::append_history`. Each `mark_*` call writes an audit row. 1 test (`history_is_appended_per_outcome`) verifies two outcomes produce two rows.
- [x] **Item 7**: `AppState::bootstrap()` integration — calls `register_aumid()` (warnings on failure, non-fatal), spawns `ToastDriver` with shared `Arc<Mutex<Journal>>`. `Journal::toast_*` thin wrappers added. `Inner.toast_driver` owns the handle for shutdown.
- [x] **Item 8** (renamed): `src/cli/commands/toast.rs` — `opencode toast queue|list|cancel` clap subcommand + dispatch wiring. 7 clap parsing tests. Operator guide written: `docs/toast-integration.md` (AUMID troubleshooting + CLI contract).
- [x] **Item 9**: `tools/toast-smoke.ps1` smoke script — verifies the queue surface end-to-end (feature-gate probe → enqueue → list → idempotent cancel) against a `--features toast` build. Feature OFF ⇒ SKIP. Operator guide: `docs/toast-integration.md`.

**Commit: `d6e6e23`** — `feat(rfc-28): Section F items 1-8 — Toast notifications queue + driver + CLI subcommand`. 16 files changed, 1721 insertions. Tests default 270, +toast 301. `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo clippy --features toast -- -D warnings`: todos limpios. `cargo check --features "firecrawl,toast" --lib`: clean (combo verify).

### F.8 Riesgos

1. **Single-binary invariant** (AGENTS.md §6): `winrt-toast-reborn` es pure Rust (windows-rs FFI). Cumple. No se introduce runtime extra.
2. **Non-MSIX desktop activation**: WinRT Toast activation desde desktop (no-MSIX) requiere el Start Menu shortcut creado por `register()` — si falla, los toasts aparecen pero los clicks no regresan al proceso. Mitigación: `register_aumid()` corre en boot y loguea el error; los callbacks se mantienen activos (WinRT permite registry-creation lazy si el AUMID existe con otro valor de DisplayName). Si toast.exe no está registrado en absoluto, los toasts siguen siendo visibles pero no deep-linkable.
3. **AUMID registration conflict**: Si otra app reusa `dev.opencode.OpenCodeOS.HUD` (improbable), WinRT enruta los callbacks al último registrante. Mitigación: AUMID namespace `dev.opencode.OpenCodeOS.*` único al proyecto.
4. **Sobrevive crash del proceso**: queue en SQLite → re-backfill en boot. El driver polls `next_pending` en arranque, así todo `pending` whose `fire_at <= now` dispara inmediatamente.
5. **Linux/macOS parity**: `ToastDispatcher::Stub` logueea via `tracing::info!`. Deep-link callbacks no aplican (no hay AUMID). El HUD WebSocket sirve como canal alternativo y `atlas hud` arranca el server de nuevo.
6. **WinRT ToastManager no-Send**: Resuelto con std::thread dedicado + mpsc channel entre el dispatcher handle y el worker; el driver tokio invoca via `tokio::task::spawn_blocking`. Future remain `Send`.

### F.9 Atribución (apéndice)

- `src-tauri/src/toast/*.rs` — `"Uses winrt-toast-reborn 0.3.8 (MIT) by Md. Iftakhar Awal Chowdhury (AtifChy), fork maintained of winrt-toast 0.1.1. https://github.com/AtifChy/winrt-toast"`. Module prose en `src/toast/mod.rs`.
- `src-tauri/Cargo.toml` — entry under `[target.'cfg(windows)'.dependencies]` carries the crate name + version + license (MIT) in inline comment.

---

## §G — Windows Calendar Integration (Phase 1.5g)

**Status: ✅ 10/10 implementados** (items 1-4 `8d26528`; items 5-8 vía Roadmap v3 / Phase 23; items 9-10 `docs/calendar-integration.md` + `tools/calendar-smoke.ps1`).

Atlas OS planifica (RFC 12 Planning) runs de validación, retrospectives (RFC 16) y schedules de `autoresearch` cadencia. Hoy estas viven en el `journal` SQLite sin affordance para el usuario que quiere verlas en su calendario nativo (Outlook, Apple Calendar, Google Calendar). §G define dos direcciones con un único stack: **WRITE** (Atlas OS publica eventos via `.ics` feed servido desde el HUD axum server) y **READ** (Atlas OS consume el Microsoft Graph `/me/calendarView` endpoint para leer eventos del usuario e inyectarlos como contexto al Planning engine).

### G.1 Decision: reject WinRT `AppointmentManager`

WinRT `Windows.ApplicationModel.Appointments.AppointmentManager` requiere la capability restringida `appointmentsSystem` ([microsoft/WindowsAppSDK docs](https://learn.microsoft.com/en-us/uwp/api/windows.applicationmodel.appointments.appointmentmanager)). En apps desktop no-MSIX (el caso de Tauri 2 default), `ShowAddAppointmentAsync` / `ShowEditAppointmentAsync` / `ShowRemoveAppointmentAsync` devuelven `E_ACCESSDENIED (0x80070005)`. Las APIs de lectura (`FindAppointmentsAsync`) también están sandboxed a la app caller. Esto viola RFC 25 §11 (single-binary, sin capabilities restringidas) y AGENTS.md §6 (no comportamiento dependiente de packaging restricted).

Alternativa: **win32 ICalendar via COM** no tiene Rust bindings mantenidos y depende de Outlook instalado (no cross-platform). Alternativa: **Microsoft Graph REST** — sí cross-platform, funciona con cualquier cliente de calendario que el usuario haya federado a Microsoft 365.

### G.2 WRITE: `GET /atlas-calendar.ics`

#### G.2.1 axum HUD route

```rust
.route("/atlas-calendar.ics", get(handler_calendar_ics))
```

No requiere auth (los IDs en la URL son opacos y unguessable — `?token={base64url(random 16 bytes)}`). El feed es **read-only** (RFC 5545 `METHOD:PUBLISH`). Cadencia: served on demand; el consumidor (Outlook/Apple/Google) polla cada N horas.

Contenido: para cada `mission` con `status in ('planning','in_progress','validation','learning','completed')` y `created_at` dentro de los últimos 30 días, emite un `VEVENT`:

| iCal field | Source |
|---|---|
| `UID` | `mission-{id}@opencode.dev` |
| `DTSTAMP` | `updated_at` (UTC, RFC 3339 → iCal `Z` suffix) |
| `DTSTART` / `DTEND` | `created_at` / `updated_at` (all-day si delta > 24h; timed otherwise) |
| `SUMMARY` | `mission.label` (truncado a 240) |
| `DESCRIPTION` | JSON del delta reciente (engine events top 10) |
| `CATEGORIES` | `mission.status` (e.g. `planning`, `validation`) |
| `STATUS` | map: `planning`→`TENTATIVE`, `in_progress`→`CONFIRMED`, `validation`→`CONFIRMED`, `completed`→`CONFIRMED`, `failed`→`CANCELLED` |
| `RRULE` | Sólo si la mission tiene recurrence programada (p.ej. autoresearch weekly): `FREQ=WEEKLY;BYDAY=MO;COUNT=8` |

#### G.2.2 Crate `ics = "0.5"`

[`ics 0.5.8`](https://crates.io/crates/ics) (MIT OR Apache-2.0, por `hummingly`, repo `https://github.com/hummingly/ics`). Pure Rust, sin deps nativas. API:

```rust
use ics::prelude::*;

let event = ICalendar::new()
    .push_event(Event::new(
        format!("mission-{}@opencode.dev", mission.id),
        format!("{}", chrono::DateTime::from_timestamp_millis(mission.created_at).unwrap().format("%Y%m%dT%H%M%SZ")),
    )
    .add_property(Property::new("SUMMARY", &mission.label))
    .add_property(Property::new("STATUS", status_ical(&mission.status)))
    .add_property(Property::new("CATEGORIES", &mission.status)));
```

Output: `ICalendar::to_string()` produce el RFC 5545 text completo (con proper line folding, escaping).

#### G.2.3 `webcal://` subscription URL

Atlas OS prints en `atlas hud` boot:

```
Calendar subscription URL:
  webcal://127.0.0.1:{port}/atlas-calendar.ics?token={token}
```

El usuario lo añade una vez en Outlook / Apple Calendar / Google Calendar. outlook/Apple aceptan `webcal://` nativamente; Google Calendar requiere meterlo como "From URL" en calendar settings.

### G.3 READ: Microsoft Graph poller

#### G.3.1 Auth: `graph-rs-sdk = "3.0.1"` con `interactive-auth`

[`graph-rs-sdk 3.0.1`](https://crates.io/crates/graph-rs-sdk) (MIT, repo `https://github.com/sreeise/graph-rs-sdk`). `features = ["interactive-auth"]` hace popup wry webview (mismo engine que Tauri 2) para el primer OAuth flow interactivo:

```rust
let graph = Graph::new(&client_id, [&redirect_uri], &scopes)?;
let token = graph.interactive_auth().await?; // wry popup, persists refresh token
```

Scopes: `Calendars.Read` + `offline_access`. Refresh token se guarda en SQLite en una columna encrypted (`AES-256-GCM` via `ring` o `aes-gcm = "0.10"`). La webview popup es opcional — si el usuario prefiere, puede paste client_id+tenant via CLI (`opencode calendar login`).

#### G.3.2 Endpoint: `GET /me/calendarView?startDateTime=...&endDateTime=...`

```rust
let range = (chrono::Utc::now(), chrono::Utc::now() + chrono::Duration::days(7));
let events = graph
    .me()
    .calendar_view(&range.0, &range.1)
    .top(50)
    .send()
    .await?
    .value();
```

Filtra `subject` y `body` por patrones indicativos de coding sessions ("OpenCode", "PR review", "ship"). Inyecta en `AppState.context_busy_windows` que el Planning engine (RFC 12 §3) consulta antes de encolar turn proactivo.

#### G.3.3 Poller loop

Cadencia: 60s, como `tokio::spawn` desde `AppState::new()`. Threshold: si más de 5 eventos en la próxima hora, todos marcados como `busy` con `weight=1.0`. Refresh token check al inicio; si expired, intenta silent refresh; si falla, Notifica via Toast (§F) que requiere re-login.

### G.4 Comunicación bidireccional con Planning

El Planning engine (RFC 12 §3) ya consulta `AppState.context_window` (journal events recientes). §G agrega `AppState.context_busy_windows: Vec<BusyWindow>` con `start, end, source: Graph|IcsLocal|Manual, subject, weight`. `BusyWindow::overlaps(turn_eta)` determina si encolar un turn proactivo ahora o esperar.

```rust
pub fn next_free_slot(&self, turn_eta: Duration) -> Option<(chrono::DateTime<Utc>, chrono::DateTime<Utc>)> {
    let now = chrono::Utc::now();
    self.context_busy_windows
        .iter()
        .fold(now, |cursor, bw| {
            if bw.overlaps(cursor, cursor + turn_eta) { bw.end } else { cursor }
        })
        .into()
}
```

### G.5 Plan de cambios Rust

| Archivo | Cambio |
|---|---|
| `src-tauri/Cargo.toml` | Añadir feature `calendar` (default OFF). `ics = { version = "0.5", optional = true }`. `graph-rs-sdk = { version = "3.0.1", optional = true, features = ["interactive-auth"] }`. |
| `src-tauri/src/calendar/mod.rs` | Re-export `CalendarWriter`, `CalendarReader`, `BusyWindow`. |
| `src-tauri/src/calendar/ics_writer.rs` | `CalendarWriter::from_journal(&Journal) -> String` (RFC 5545 text). Usa `ics 0.5`. |
| `src-tauri/src/calendar/ics_route.rs` | axum handler `handler_calendar_ics`. Token filter. |
| `src-tauri/src/calendar/graph_reader.rs` | `CalendarReader::poll()` — Graph `/me/calendarView` poll, parse `Event` Graph object → `BusyWindow`. |
| `src-tauri/src/calendar/auth.rs` | Interactive auth (wry popup) + refresh token en SQLite encrypted (AES-256-GCM). |
| `src-tauri/src/journal/schema.rs` | M18 migration: `calendar_busy_windows` table + `calendar_auth` (encrypted refresh token). Schema 17→18. |
| `src-tauri/src/hud/server.rs` | Añadir `.route("/atlas-calendar.ics", get(handler_calendar_ics))`. (Ver §G.2.1.) |
| `src-tauri/src/core/state.rs` | `AppState` agrega `context_busy_windows: Arc<RwLock<Vec<BusyWindow>>>`. Spawn poller 60s. |
| `src-tauri/src/lib.rs` | `#[cfg(feature = "calendar")] pub mod calendar;` |

### G.6 Checklist

- [x] **Item 1**: `Cargo.toml` features `calendar-ics` / `calendar-graph` + umbrella `calendar` (deps `ics 0.5`, `icalendar 0.17`, `graph-rs-sdk`, `aes-gcm`, `ring`).
- [x] **Item 2**: M18 migration `calendar_busy_windows` + `calendar_auth` (schema 17→18).
- [x] **Item 3**: `calendar/ics_writer.rs` — `CalendarWriter::from_journal()` (RRULE + `MissionStatus → iCal STATUS` mapping; tests in-module).
- [x] **Item 4**: `calendar/ics_route.rs` — axum handler + token filter (valid / missing / wrong token tests).
- [x] **Item 5**: `calendar/graph_reader.rs` — `CalendarReader::poll()` + Graph event parse.
- [x] **Item 6**: `calendar/auth.rs` — auth + refresh token AES-256-GCM encrypted in SQLite. **Deviation:** OAuth **device-code** flow (not the wry popup the RFC sketched) — no embedded browser, cross-platform, single-binary-safe.
- [x] **Item 7**: `AppState` integration — `CalendarPoller` (60 s; `ATLAS_CALENDAR_POLL_SECS`) fills `AppState.context_availability` (renamed from the sketched `context_busy_windows`).
- [x] **Item 8**: `planning/availability.rs::next_free_slot()` + `availability_now()` consulted before enqueuing a proactive turn (Phase 23 v3.1.2; golden task `planning.availability`).
- [x] **Item 9**: `docs/calendar-integration.md` — subscribe `webcal://`, authorize Graph, ad-hoc/durable `.ics`, availability/policy, troubleshooting.
- [x] **Item 10**: `tools/calendar-smoke.ps1` — feature-gate probe + token format + idempotent feed URL; `-Live` **spawns the headless HUD with `atlas serve`** (RFC 29 §3.A), waits for `hud_port.txt`, fetches `/atlas-calendar.ics` and validates the RFC 5545 skeleton (`BEGIN:VCALENDAR`/`VERSION:2.0`/`END:VCALENDAR`) + 401 on a wrong token.

### G.7 Riesgos

1. **Token storage encryption**: Refresh token en SQLite encrypted via AES-256-GCM. Si el usuario pierde el dispositivo, NO es recoverable. Mitigación: en docs recomendar `opencode calendar logout` antes de mover máquinas.
2. **graph-rs-sdk maintenance**: crate MIT mantenida por `sreeise`, 600 stars. Actualizada a v3.0.1 julio 2025. Si se rompe con nueva API Graph, fallback directo a `reqwest` + `azure_identity` (ya en workspace tree).
3. **WinRT appointment rejected**: Documentado en G.1. Sin fallback — `webcal://` subscription es el sustituto cross-platform total.
4. **Buscar eventos irrelevantes**: Graph `/me/calendarView` devuelve TODO; el filtro `subject.contains("OpenCode")` es heurístico. Alternativa: `categories` field en Graph events; pero requiere que el usuario etiquete eventos manualmente. UI lo configurable en settings (post-§G).
5. **Rate limits Graph**: 10k reqs/10 min por app default. Poller 60s/24h = 1440 reqs/día ≪ limit. OK.
6. **wry popup auth dependencies**: `graph-rs-sdk` `interactive-auth` ya incluye wry transitivamente. Verificar no conflict con la versión wry de Tauri 2 (lock file check).

### G.8 Atribución (apéndice)

- `src-tauri/src/calendar/ics_writer.rs` — `"Uses ics 0.5.8 (MIT OR Apache-2.0) by hummingly. https://github.com/hummingly/ics"`.
- `src-tauri/src/calendar/graph_reader.rs` — `"Uses graph-rs-sdk 3.0.1 (MIT) by sreeise. https://github.com/sreeise/graph-rs-sdk"`.
- `src-tauri/src/calendar/auth.rs` — `"Interactive auth via graph-rs-sdk 3.0.1 (MIT). Encryption via aes-gcm 0.10 (MIT OR Apache-2.0)."`

---

## §H — Model API Reset-Window Notifications (Phase 1.5h)

**Status: ✅ 13/13 implementados** (items 1-11 M19 + `SpendLimitError`/`ResetKind`/`parse_omniroute`/`RetryPolicy`/`handle_spend_limit_error`/`cards.rs`/`SpendLimitErrorCard.svelte`/`ModelReadyCard.svelte`/`Profile` bail-out; items 12-13 `docs/reset-window-notifications.md` + `tools/reset-window-smoke.ps1`).

Atlas OS usa models LLM via upstream providers o vía OmniRoute gateway (§3.8 RFC 25). Cuando un model hittea un rate limit (429) o un spend cap (402/403), los providers devuelven headers/timestamps indicando cuándo el model se resetea y puede volver a usarse. Ningún AI coding tool comercial (Cline, Cursor, Aider, Copilot) **proactivamente notifica** al usuario cuando el model vuelve a estar disponible — el usuario debe reintentar manualmente. Atlas OS capturar `reset_at` desde la respuesta de error, persiste hasta llegada la hora, y dispara una Toast notification `kind='model_ready'` cuando el reset cumple. Esta es una feature diferencial frente a la competencia.

### H.1 Pattern source: Cline PRs #10207, #10963, #10141

- **Cline PR #10207** ([cline/cline#10207](https://github.com/cline/cline/pull/10207)) añade la `SpendLimitError` card con `resets_at`, un botón "Request Increase" con 5-min localStorage cooldown, y la entry es **exempt del auto-retry loop** (no spamea costos).
- **Cline PR #10963** ([cline/cline#10963](https://github.com/cline/cline/pull/10963)) — retry middleware con jitter ±25% + parse `Retry-After` header + `x-ratelimit-reset` header.
- **Cline PR #10141** ([cline/cline#10141](https://github.com/cline/cline/pull/10141)) — bail-out cuando `Retry-After > threshold` (default 60s): para el retry loop en lugar de esperar 5min y agotar sesiones.

Esto se porta a Rust usando:
- Estructura `SpendLimitError` (struct, no panic / Result) con `resets_at: chrono::DateTime<Utc>`, `provider`, `model`, `request_id`.
- Retry middleware (tokio layer) con `RetryPolicy::with_jitter(±25%)`, `RetryPolicy::bail_threshold(Duration)`.
- `SpendLimitError` exempted: no entra en `RetryPolicy` automático; la queue se pausa y el user decide.

### H.2 OmniRoute: parsing de la envelope de error

Atlas OS usa OmniRoute como un único OpenAI-compatible provider (no parsea headers upstream directamente — OmniRoute los normaliza). OmniRoute devuelve una JSON envelope:

```json
{
  "error": {
    "type": "rate_limit" | "spend_limit",
    "message": "...",
    "status": 429 | 402 | 403,
    "provider": "anthropic",
    "model": "claude-3-5-sonnet",
    "resets_at": "2026-08-01T12:34:56Z",
    "request_id": "req_abc123"
  }
}
```

El handler de error HTTP en `Orchestrator::complete()` parsea `error.resets_at` (RFC 3339). Si existe y es en el futuro, persiste en SQLite `model_resets`:

```sql
CREATE TABLE model_resets (
  id           INTEGER PRIMARY KEY,
  provider     TEXT NOT NULL,         -- 'openai' | 'anthropic' | 'omniroute' | ...
  model        TEXT NOT NULL,
  status_code  INTEGER NOT NULL,      -- 429 | 402 | 403
  error_type   TEXT,                 -- 'rate_limit' | 'spend_limit' | NULL
  resets_at    INTEGER NOT NULL,      -- unix millis, parsed from resets_at || Retry-After || x-ratelimit-reset
  request_id   TEXT,
  observed_at  INTEGER NOT NULL DEFAULT (unixepoch()),
  UNIQUE (provider, model, resets_at)
);
CREATE INDEX model_resets_pending_idx ON model_resets(resets_at) WHERE toast_dismissed_at IS NULL;
```

Decision: NO exponer `x-ratelimit-reset-requests` / `x-ratelimit-reset-tokens` (OpenAI) ni `anthropic-ratelimit-*-reset` (Anthropic) directamente. OmniRoute los normaliza a una única `resets_at`. Para providers directos (sin OmniRoute), se parsea el header específico y se mapea al mismo formato.

### H.3 `SpendLimitError` card (HUD)

La HUD Mission Control (RFC 24 §3.3) renderiza una card especial para `kind='model_spend_limit'`, port del diseño Cline #10207:

```
┌─────────────────────────────────────────────────┐
│ ⚠ Spend Limit Reached                            │
│                                                   │
│ Model: claude-3-5-sonnet                          │
│ Resets: 2026-08-01 12:34:56 UTC (in 12 min)      │
│                                                   │
│ [Request Increase]  [Switch Provider]             │
└─────────────────────────────────────────────────┘
```

- Botón **"Request Increase"**: deeplink al provider dashboard (`anthropic.com/usage`, `platform.openai.com/usage`, etc.). 5-min localStorage cooldown por provider (`opencode.llm.cooldown.request_increase.{provider}`) — evita que el usuario lo click múltiples veces mientras espera.
- Botón **"Switch Provider"**: lanza `opencode profile switch {backup_profile}` (RFC 06). El backup profile debe estar configurado (verifica en `profiles.toml`).
- Esta card es **exempt** del auto-retry loop de `Orchestrator` (Cline #10207). No spawnea turn requests mientras se muestre.

### H.4 Scheduler + Toast: `model_ready`

Cuando `resets_at` llega, el **Scheduler Driver** (ver §F.3 — mismo `toast_queue`, `kind='model_ready'`) dispara el Toast:

```
┌─────────────────────────────────┐
│ ✓ Model Ready                    │
│                                   │
│ claude-3-5-sonnet is available.   │
│ Resume mission?                   │
│                                   │
│ [Resume]                          │
└─────────────────────────────────┘
```

- Botón **"Resume"** → `on_activated` deep-link `opencode://mission/{id}/resume`. Re-encola el turn en `Orchestrator`.
- Toast history se persiste para dedupe.

### H.5 Retry policy con jitter y bail-out

Para 429s sin SpendLimitError (rate-limit transitorio), el `Orchestrator` retry-loop aplica (port de Cline #10963 + #10141):

- **Jitter ±25%**: `delay = base * (1 + rng.gen_range(-0.25..=0.25))`, `base = exponential_backoff(attempt)`.
- **Parse `Retry-After`**: si el header existe y es un número de segundos, usarlo como delay base (en lugar de exponential). Si es RFC 3339 datetime, parsear y comparar con `now`.
- **Parse `x-ratelimit-reset`** (en providers directos sin OmniRoute): igual que `Retry-After`.
- **Bail-out threshold**: si `delay > threshold` (default 60s, configurable en `profiles.toml`), NO reintenter — emite `OrchestratorEvent::BailOut(reason)` al HUD y pausa la mission. El usuario decide si esperar manual o switch provider.

### H.6 Plan de cambios Rust

| Archivo | Cambio |
|---|---|
| `src-tauri/Cargo.toml` | No new deps (usa `chrono`, `tokio`, `serde` ya presentes). |
| `src-tauri/src/orchestrator/error.rs` | `SpendLimitError` struct (`resets_at`, `provider`, `model`, `request_id`). |
| `src-tauri/src/orchestrator/parse_error.rs` | OmniRoute envelope parser + header parsers (`Retry-After`, `x-ratelimit-reset`). |
| `src-tauri/src/orchestrator/retry.rs` | `RetryPolicy` con jitter ±25%, exponential backoff, bail-out threshold, exempt `SpendLimitError`. |
| `src-tauri/src/orchestrator/mod.rs` | `Orchestrator::complete()` catch error → parse → persist `model_resets` → enqueue Toast `kind='model_ready'` en `toast_queue`. |
| `src-tauri/src/journal/schema.rs` | M19 migration: `model_resets` table. Schema 18→19. |
| `src-tauri/src/journal/model_resets.rs` | CRUD: `insert`, `pending_for(provider, model)`, `mark_toast_dispatched`. |
| `src-tauri/src/hud/cards.rs` | `SpendLimitErrorCard` component + `ModelReadyCard` component. |
| `src/lib/components/SpendLimitErrorCard.svelte` | Svelte 5 runes. Botones "Request Increase" + "Switch Provider". localStorage cooldown. |
| `src/lib/components/ModelReadyCard.svelte` | Svelte 5 runes. Botón "Resume". |
| `src/lib/stores/hud.ts` | `SpendLimitErrorCard` y `ModelReadyCard` types + renderers. |
| `src-tauri/src/profiles/mod.rs` | `Profile::bail_out_threshold_secs: u64` (default 60) + `Profile::backup_profile_id: Option<String>`. |
| `Atlas OS/24 - HUD Mission Control.md` | Section §3.3 addendum: SpendLimit / ModelReady card anatomy. |
| `Atlas OS/04 - Model Orchestrator.md` | Section §9 addendum: retry policy + bail-out + reset-window. |
| `Atlas OS/06 - Profiles.md` | `bail_out_threshold_secs` + `backup_profile_id` profile fields. |

### H.7 Checklist

- [x] **Item 1**: M19 migration `model_resets` (schema 18→19).
- [x] **Item 2**: `orchestrator/error.rs` — `SpendLimitError` struct. 5 tests.
- [x] **Item 3**: `orchestrator/parse_error.rs` — OmniRoute envelope parser + `Retry-After` header parser + `x-ratelimit-reset` parser. 14 tests (incluye stale-reset guard y nom-kind inference).
- [x] **Item 4**: `journal/model_resets.rs` — CRUD. 8 tests (incluye idempotent upsert, dismissed filter, link/unlink by id y queue_id).
- [x] **Item 5**: `orchestrator/retry.rs` — `RetryPolicy` con jitter ±25 %, exponential backoff, bail-out 60 s, max 5 intentos. 8 tests.
- [x] **Item 6**: `orchestrator/mod.rs` `handle_spend_limit_error` — persist → (toast-gated) enqueue → link → publish `SpendLimitObserved`. 3 tests (incluye idempotent replay y feature-off Toast skip).
- [x] **Item 7**: `src/lib/components/SpendLimitErrorCard.svelte` — Svelte 5 runes, botones "Request Increase" + "Switch Provider", localStorage cooldown 5 min por provider, deeplink por provider-known dashboard URLs. Vitest contract tests en `hud.test.ts` (5 tests para `postProfileSwitch`/payload).
- [x] **Item 8**: `src/lib/components/ModelReadyCard.svelte` — Svelte 5 runes, botón "Resume" invoca `POST /mission/resume`. Vitest contract tests en `hud.test.ts` (3 tests para `postMissionResume`).
- [x] **Item 9**: HUD card pipeline — `hud/cards.rs` con `SpendLimitErrorCardPayload` + `ModelReadyCardPayload` serialisers; `hud.ts` tipos espejo; 5 tests Rust + `hud.test.ts` contratos.
- [x] **Item 10**: `profiles/mod.rs` `Profile` struct + `profile.toml` reader/writer; campos `bail_out_threshold_secs` (default 60) + `backup_profile_id: Option<String>`. 2 tests (roundtrip + default fallback).
- [x] **Item 11**: RFC 24 §3.3 addendum + RFC 04 §9 addendum + RFC 06 placeholder (RFC 06 no existe aún; `Profile` struct establece la base para futura RFC).
- [x] **Item 12**: `docs/reset-window-notifications.md` — operator guide: OmniRoute envelope, thresholds/policy, card anatomy, troubleshooting.
- [x] **Item 13**: `tools/reset-window-smoke.ps1` — verifies the M19 schema + `spend_limit_observed` journal kind + optional Toast-feature binary signature (it cannot synthesise a real 429 without provider credentials; the parser/dispatch behaviour is covered by the Rust unit tests).

### H.8 Riesgos

1. **OmniRoute envelope format drift**: si OmniRoute cambia su error JSON, parser falla. Mitigación: fallback graceful si `error.resets_at` no parsea — log warning + continúa con retry normal sin Toast.
2. **Clock drift**: si el reloj del cliente está adelantado vs el server, `resets_at` puede llegar "en el pasado" → Toast se dispara inmediatamente. Mitigación: si `resets_at < now - 60s` al persistir, no encolar Toast (suponemos ya expiró o reconcile).
3. **Toast spam**: si un model rate-limits frecuentemente (provider degradado), el XOR Toast `model_ready` puede ser molesto. Mitigación: cooldown de 10 min por `(provider, model)` en `toast_queue` — segundo reset dentro de 10 min del primero no dispara Toast, sólo marca `status='fired'` silenciosamente.
4. **SpendLimitError vs RateLimit**: Pay-per-use (402/403) vs rate-limit (429) comparten `resets_at`. UX los distingue: `error_type='spend_limit'` → "Spend Limit Reached"; `error_type='rate_limit'` → "Rate Limited". Misma handler, distinta COPY en la card.
5. **Competitive gap, not parity**: Esta feature ES el diferencial. Si Cline la añade primero, OpenCode pierde ventaja. Implementar temprano, shippear visible.

### H.9 Atribución (apéndice)

- `src-tauri/src/orchestrator/{error,parse_error,retry}.rs` — `"Pattern adapted from Cline (Apache-2.0) PRs #10207 (SpendLimitError card, exempt auto-retry), #10963 (jitter ±25%, Retry-After parse), #10141 (bail-out threshold). https://github.com/cline/cline"`.
- OmniRoute error envelope: campo `resets_at` field nomalizado por OmniRoute gateway; Atlas OS no parse upstream provider headers directamente cuando se usa OmniRoute.

---

## Orden recomendado — Phase 1.5

**Recomendado (justificado):** Start con **Fase 0 (XS copy_uso batch)**, luego **§D primero**, §A segundo, §C tercero, §B cuarto, **§E quinto (post-graphify)**, §F sexto (post-§E, Windows-only), §G séptimo (post-§F), **§H octavo (post-§G — feature diferencial frente a competencia)**. **Status actual: Fase 0, §D, §A, §C, §B, §E items 1-8, §F items 1-8 COMPLETOS; §G, §H documentados (Round 5 research RFC 22 §12). Próximo: §G Phase 1.5g (Windows Calendar + Microsoft Graph reader, M18 schema). RFC 28 listo al 100% en documentación.**

### Razón

0. **Fase 0 — batch de ítems XS (1–2 días, copy_uso + ports triviales)**: ✅ COMPLETO (commit `7ea40e1`). Antes de tocar ninguna sección en serio, este batch sienta las bases legales+contractuales para todos los portados posteriores. Total ~3h trabajo efectivo. **NO toca Cargo.toml**: sólo añade archivos Markdown / constantes Rust puras / YAML / scripts PowerShell en subdirectorios sin dependencias cruzadas. Items:
   - **AR-001** `skills/autoresearch/program.md` (copiar verbatim Karpathy's program.md con header de atribución)
   - **AR-012** `skills/autoresearch/NEVER_STOP.md` (extraer el guard `NEVER STOP` + "rewind sparingly" prose del program.md — para citar por separado en skills de autonomía)
   - **IT-003** `src-tauri/hooks/<cli>/hooks.json` — copy_uso de la tabla mapping 10 hooks → WTA topics (con variants por CLI generadas programáticamente)
   - **IT-004** `src-tauri/specs/osc-9001.md` — copy_uso verbatim del spec `doc/specs/llm-agent-event-integration.md` (referenciado por §B pero NO requiere código aún)
   - **PT-003** `src-tauri/src/journal/yaml_format.rs::literal_block` (26 LOC port — el str_presenter de posting a Rust). **NO toca aún el emisor YAML**, sólo escribe la función standalone + tests unitarios.

1. **§D AuditLog export** (Phase 1.5a, 1–2 sprints): ✅ COMPLETO (commit `db25379`). Crate packaging más pequeño (`serde_yaml`), número más bajo de archivos tocados, cierra el gap implícito de audit-log portable export sin tocar engines. Deployable primero, reduce riesgo. Tests fáciles. Confidence builder. Reusa Fase 0's `literal_block`.

2. **§A Autoresearch loop** (Phase 1.5b, 2–3 sprints): ✅ COMPLETO (commit `853da30`). Extiende RFC 19 supervisor sin tocar arquitectura. Schema M13 aislada, un branch de modo. Requiere `git` subprocess (ya tenemos via CLI bin, no nuevo crate). HUD card incremental. No depende de §B/§C. **Si el ACP server retrasa, autoresearch es demo-ready standalone.** Reusa Fase 0's `program.md`.

3. **§C Mission graph M15** (Phase 1.5c, 3–4 sprints): ✅ COMPLETO (commit `cddcbc2`). El más disruptivo. Toca Planner, Skills, Supervisor, Learning, HUD. Crates `petgraph`+`tree-sitter` son bump binario. Hacerlo **third** aumenta conocimiento del codebase por lo aprendido en §A/§D. **No deberíamos hacer §C primero**: el scope del refactor Planner es grande.

4. **§B IT ACP server** (Phase 1.5d, 4–5 sprints): ✅ **COMPLETO** (commits `7f8215e`–`4b4924e`, items 1–8 todos ✅). Antes: "Último porque depende de (a) el resto del sistema estable, (b) IT 0.1.x y ACP v2.0.0 son moving targets (`unstable_*` features), (c) prueba-target limitada a Windows, (d) si §C introduce DAG planner, la superficie `session/set_mode` mapping se beneficiará de tener states graph ya consolidado. Adicionalmente: el `exec step` de §A es el mismo `opencode exec step` que §B advertisea — armonizar antes de implementar §B reduce rework. Reusa Fase 0's IT-003/IT-004." Decision sustained: §B fue claramente el más disruptivo, hecho último; ahora sirve como referencia para §F (ACP handlers patrón callback) y §G (interactive auth patrón).

### Sub-enum Phase 1.5

- **Fase 0 — batch XS (1-2 días)**:
  1. `skills/autoresearch/program.md` — copy_uso verbatim de `karpathy/autoresearch/program.md` con header `<!-- Source: karpathy/autoresearch MIT -->`. Atribución + sin GitHub Actions check.
  2. `skills/autoresearch/NEVER_STOP.md` — extracto de `program.md` con el guard `NEVER STOP` + "rewind sparingly, if ever" prose.
  3. `src-tauri/hooks/<cli>/hooks.json` para `<cli> ∈ {claude, copilot, codex, gemini, opencode}` — copy_uso de schema de `microsoft/intelligent-terminal/wt-agent-hooks/<cli>/hooks.json`. Variantes programáticas vía generator script `tools/gen-hooks.ps1`.
  4. `src-tauri/specs/osc-9001.md` — copy_uso verbatim de la sección OSC 9001 del spec `intelligent-terminal/doc/specs/llm-agent-event-integration.md`. Header de atribución.
  5. `src-tauri/src/journal/yaml_format.rs::literal_block(s: &str) -> String` — port de `str_presenter` de `darrenburns/posting/src/posting/yaml.py`. 26 LOC + `#[cfg(test)] mod tests` con casos multilinea + senza newlines + trailing whitespace. Sin tocar `serde_yaml` aún.

- **1.5a — §D snapshot export**:
  1. `serde_yaml` in Cargo.toml (Context7 verify)
  2. `posting.rs` + tests contra sample canónico
  3. `retention.rs` SQLite hook + CLI flag
  4. HUD button + endpoint
  5. RFC 24 §10 patch + RFC 28 apéndice clarificando confusión Brecha H
  6. RFC 22 entry

- **1.5b — §A autoresearch**:
  1. ✅ M13 migration (`autoresearch_runs` + `autoresearch_candidates` with FKs, indexes, CHECK constraints)
  2. ✅ `journal/autoresearch.rs` — pure state machine `tick(state, event) -> AutoresearchOutput` (12 tests). Host supervisor branch (git subprocess + metric_command exec) deferred to Phase 2.
  3. ✅ CLI flag `--autoresearch --metric --max-steps --timebox` on `atlas mission new` (stub — emits info, persists `mission` row; Phase 2 supervisor will populate `autoresearch_runs`).
  4. ✅ HUD card `AutoresearchCard.svelte` + WS channel via `BusEventKind::AutoresearchCancelled` + `POST /autoresearch/cancel` endpoint (8 tests).
  5. ✅ RFC 19 §11 "Modo Autoresearch" patch
  6. ⏳ Real-world hill-climbing run on a lint metric as acceptance test — deferred to Phase 2 (requires host-side loop shell-out);
     meanwhile `skills/autoresearch/program.md` (Fase 0) + 12 FSM tests + 7 HUD endpoint tests pin the contract.
  7. ⏳ `AutoresearchCard.test.ts` (vitest) — pending dep-free @testing-library/svelte install (frontend tests are #21 in `hud.test.ts` covering the `postAutoresearchCancel` contract).

- **1.5c — §C mission graph**:
  1. ✅ Crates: `petgraph = "0.8"` (gated `dag_mode`), `tree-sitter = "0.26"`, `tree-sitter-rust = "0.24"`, `tree-sitter-svelte-next = "0.1.1"` (last 3 gated `codebase-graph`). Two features added to `Cargo.toml` (default off); `cargo check --features dag_mode` and `--features codebase-graph` both resolve clean against Rust 1.84. (`tree-sitter-svelte = "0.10"` swaps in `tree-sitter-svelte-next` because 0.10 still pins tree-sitter 0.20 and lacks the `LANGUAGE` const required by 0.26.)
  2. ✅ M15 migration (`mission_graph_nodes`, `mission_graph_edges`, `learning_graphs`); 8 schema tests in `journal::tests::mission_graph_schema_tests`.
  3. ✅ `graph/{mod, traverse, ast}` modules built:
     - `mod.rs` — canonical `Provenance`, `NodeKind`, `EdgeKind`, `NodeId`, `EdgeId`, `Node`, `Edge`, `MissionGraph` types; tag/parse round-trip; 7 tests.
     - `traverse.rs` — `shortest_path` (BFS), `get_neighbors`, `god_nodes` (percentile), `community_partition` (union-find). Gated `dag_mode`; 9 tests gated `#[cfg(all(test, feature = "dag_mode"))]`.
     - `ast.rs` — tree-sitter AST extractor (Rust) + heuristic Svelte extractor (grammar treats `<script>` as `raw_text` so we use a line-classifier for imports/functions) + Tarjan's SCC `find_import_cycles` + `report_to_graph`. Gated `codebase-graph`; 10 tests (`#[cfg(all(test, feature = "codebase-graph"))]`) + 3 fallback tests when feature is off.
  4. ✅ Planner DAG emitter — `src-tauri/src/planning/graph_emitter.rs` (`plan_to_graph(plan: &Plan) -> MissionGraph`, gated `dag_mode`, 9 tests). Pure projection of `Plan` over the canonical `MissionGraph` (RFC 28 §C item 3); root node `mission:{id}` + 1 node per Objective/Milestone/Step + `DependsOn` edges from `depends_on` arrays. Idempotent (string-stem edge ids). Default callers still see the linear `Plan { steps }` contract — `dag_mode` consumers opt-in by calling `planning::graph_emitter::plan_to_graph(&plan)`.
  5. ✅ Skills `graph.toml` loader — `src-tauri/src/skills/graph_loader.rs` (gated `dag_mode`, 12 tests). Parses RFC 23 §7.3 4th-skill-file `graph.toml` via the existing `toml` crate (no new dep). `load_graph_template(dir) -> Option<SkillGraphTemplate>` (None when missing, error on malformed); `instantiate(template, skill_id, mission_id) -> MissionGraph` re-homes node/edge ids under `{skill_id}:{mission_id}:` prefix so two instances of the same skill on a running mission never collide. Provenance `Inferred` for all instantiated nodes (skill spec, not source). Loader contract clearly separated from `manifest::load_skill` — Phase 2 supervisor wires `load_graph_template` into the skill-load pipeline.
  6. ✅ Learning graphs persist + retrieve por cosine — M16 migration
     adds nullable `embedding` (little-endian f32 BLOB) + `emb_model`
     columns to `learning_graphs`. `src-tauri/src/journal/
     learning_graphs.rs` (gated `dag_mode`) implements:
       * `persist_graph(conn, &PersistGraphRequest)` — idempotent UPSERT
         on `id`. Bundled-input struct keeps the call site readable
         and stays under clippy's `too_many_arguments` threshold.
       * `retrieve_similar_graphs(conn, query_embedding,
         intent_signature, top_k) -> Vec<ScoredGraph>` — top-k by
         cosine when an embedding is available; falls back to exact
         `intent_signature` match (score 1.0) and otherwise to recency
         (score 0.0). Only `success = 1` rows are returned (RFC 16 §3
         anti-patterns are persisted for audit but never injected as
         Planner hints).
       * Cosine is computed in-process; the optional `vec0` virtual
         table is intentionally NOT depended on because RFC 25 §3.4
         marks `sqlite-vec` load_extension as best-effort.
     Journal exposes thin lock-and-delegate wrappers
     `persist_learning_graph` and `retrieve_similar_learning_graphs`.
     16 tests (12 module + 4 M16 schema). Schema version 15 → 16.
   7. ✅ `GET /hud/graph/:id` + `<GraphView>` + tests —
      `src-tauri/src/journal/mission_graph.rs` (gated `dag_mode`, 22
      tests): `read_graph(conn, mission_id) -> Option<MissionGraph>`
      rehydrates the M15 rows back into canonical `Node`/`Edge` types
      (parses `attrs_json` lazily via `Node::attrs()`). `Journal::
      read_mission_graph` wraps the lock-and-delegate. `src-tauri/
      src/hud/graph.rs` (gated `dag_mode`, 6 tests): `get_graph` axum
      handler, `GET /graph/:mission_id` returns 200 JSON / 404 / 400
      (mission_id ≤ `MAX_MISSION_ID_LEN = 64`); route registered in
      `hud/server.rs` before `.with_state()`, after payload routes,
      gated `#[cfg(feature = "dag_mode")]`. Frontend: typed
      `MissionGraph`/`GraphNode`/`GraphEdge`/`Provenance`/`NodeKind`/
      `EdgeKind` in `src/lib/stores/hud.ts` mirroring the Rust types
      (SCREAMING_SNAKE_CASE provenance, snake_case kind); `fetchGraph
      (hudUrl, missionId)` client with 404+5xx error paths.
      `src/lib/components/GraphView.svelte` (Svelte 5 runes) renders
      nodes (provenance-coloured chips grouped by `NodeKind`) and the
      edge table (`precondition`/`guard`/`visit_count`); reads-only,
      re-fetches via `$effect` on `hudUrl`/`missionId` change. 6 new
      `hud.test.ts` cases (fetchGraph URL+errors, type contract)
      bring the frontend suite 21 → 27. Clippy `-D warnings` clean;
      default 257, combined 321.
  8. ✅ RFC 12/16/19/23/24 patches:
     - RFC 12 §3.1 — DAG mode emission contract behind `dag_mode`.
     - RFC 16 §3 "Por grafo de misión exitoso" — learning_graphs structural graph diffing via cosine over `sqlite-vec` + `fastembed-rs`.
     - RFC 19 §6.1.1 — State DAG behind `dag_mode`; doom-loop switcher = `shortest_path(current, healthy_state)` from `graph::shortest_path`.
     - RFC 23 §7.3 — Skills gain 4th file `graph.toml` declaring sub-graph templates; loader inserts into M15 with `INFERRED` provenance (spec only; loader = item 5).
      - RFC 24 §3.3 — Card types taxonomy: agent / autoresearch (live) / graph (live, behind `dag_mode`).
  9. ✅ RFC 22 entries (4 crates) — §10 Round 3 añadida: petgraph, tree-sitter, tree-sitter-rust, tree-sitter-svelte-next con justificación single-binary-safe + binary-size budget + sustitución `tree-sitter-svelte` → `tree-sitter-svelte-next` justificada.

- **1.5d — §B IT ACP** (⏳ Planeado — siguiente hito):
  1. ✅ Crates: `agent-client-protocol = "2.0"` (Apache-2.0, MSRV 1.88) gated behind new feature `acp-server` (default OFF en `Cargo.toml`). Sólo `unstable_session_fork` feature activada; `Stdio` builtin del core crate reemplaza cualquier helper tokio externo — verify de Jul 2026 confirmó que `sacp-tokio` es un stack paralelo (Symposium) que vendor-lock-ea, y que `agent-client-protocol-tokio` no existe en el line v2.0.0. `cargo check --features acp-server` compila limpio (cargo download crate + 1 sub-dep `agent-client-protocol-derive`); default build sin acp-server no arrastra la dep.
  2. ✅ M14 migration — `agent_session_events` (id PK, ts INT NOT NULL, pane_id TEXT NULL, event_type TEXT NOT NULL, agent TEXT NOT NULL, task_id TEXT NULL, payload_json TEXT NOT NULL) + indexes `idx_ase_ts` (ts) + `idx_ase_task` (task_id). Schema version 13→14. Tabla NOT feature-gated (cheap; lets HUD read pane telemetry on non-Windows), pero el `wtcli listen` worker que la pobla sí está gated `acp-server`. Schema tests: 5 (≥14, table exists, indexes exists, nullable task/pane roundtrip, envelope payload_json preserved verbatim).
  3. ✅ `acp/{mod, mode_mapping, commands, delegate}` servidos + wire binario. `mod/run_server()` entry point (gated `acp-server`) — handlers de `initialize`/`session/new` (ahora emite `available_commands_update` con el catalogue de `commands.rs`)/`session/prompt` + notificación `$/cancel_request` usando el builder `Agent.builder().on_receive_request(...).connect_to(Stdio::new())` del crate oficial; `session/prompt` responde `StopReason::Refusal` con un chunk `AgentMessageChunk` explicando "Phase 1.5d: agent host loop not wired" (no es `// TODO` — es el comportamiento stub). `mode_mapping.rs` — tabla bidireccional RFC 19 `SupervisorState` {Plan,Exec,Review} ↔ ACP `AcpMode` {Architect,Code,Ask}. `commands.rs` (item 4) — catalogue de 6 slash commands con `build_available_commands_update()`. `delegate.rs` (item 4) — `parse_delegate` + `DelegateOutcome` + `capture_active_pane_scrollback` stub. Binario `cli/bin/opencode.rs` (item 4) detecta `WT_COM_CLSID` y rutea a `run_server()`. Tests: 4 mod + 7 mode_mapping + 5 commands + 10 delegate = 26 tests combinados. Default 262, combined `acp-server,dag_mode,codebase-graph` 352 (+15 sobre item 3).
  4. ✅ CLI plumbing para `/opencode fix`, `/opencode restart`, `/opencode exec step`, `/opencode mission new`. `/opencode fix` captura vía `wtcli active-pane`+`capture-pane --last-prompt`, enruta al engine Repair (RFC 15). NUEVO: `acp/commands.rs` (5 tests) — `AvailableCommand` catalogue (6 commands, `/opencode fix` con `UnstructuredCommandInput`; resto sin input) + `build_available_commands_update()` que envuelve `SessionUpdate::AvailableCommandsUpdate` y se emitido por `session/new` handler en `mod.rs`. `acp/delegate.rs` (10 tests) — `parse_delegate(line, cwd)` reconoce `/opencode exec step <mission_id> <step_id>`, `/opencode fix [hint]`, `/opencode restart`; multi-token matcher `match_command_head` (sliding prefix vs CMD_EXEC_STEP/CMD_MISSION_NEW/.../CMD_RESUME, maneja con o sin `/` inicial); stub `capture_active_pane_scrollback() -> Result<CapturedScrollback, CaptureError>` (Err en non-Windows o ausencia de `WT_COM_CLSID` o `StubNotWired` en este commit; subprocess real lands en item 5). `DelegateOutcome` enum: ExecStep / FixRequested / RestartRequested / NotImplemented. Binario `opencode.rs` reescrito: `should_run_acp_server()` detecta `WT_COM_CLSID` ó `ATLAS_ACP_FORCE=1` (cfg(feature = "acp-server") gating); `run_acp_server()` inicializa tracing y llama `atlas_os::acp::run_server()`. Defecto: cae al CLI reciclando `commands::dispatch`. Default 262, combined `acp-server,dag_mode,codebase-graph` 352 (+15 sobre 337 de item 3). `cargo check/clippy --features acp-server -- -D warnings` limpio; `cargo fmt --check` limpio.
  5. ✅ `wtcli listen --json` worker → events SQLite (Channel 2). Worker Rust spawn del subprocess, parseo JSON-lines, UPSERT en M14. NUEVO: `src-tauri/src/journal/agent_events.rs` (6 tests, módulo non-gated) — `AgentSessionEventRow` (typed row 1:1 con M14), `insert_agent_session_event(conn, ts, pane_id, event_type, agent, task_id, payload_json)`, `agent_session_events_tail(conn, last)`, `agent_session_events_for_pane(conn, pane_id)`. `Journal` añade los wrappers `insert_agent_session_event`, `agent_session_events_tail`, `agent_session_events_for_pane`. `src-tauri/src/acp/listen_worker.rs` (11 tests, gated acp-server via submodule) — `AgentEventEnvelope` + `AgentEventParams` (serde Deserialize+Serialize, `event` con `#[serde(default)]` para reconocer "field-missing" como `MissingEvent`), `parse_envelope(line)` (rejects empty/non-event type/non-agent_event method/missing event), `timestamp_to_epoch_seconds(ts)` (RFC3339→i64, 0 fallback), `persist_envelope(conn, line)` → `PersistOutcome::{Inserted, Dropped}` (Inserted pánico en persistencia failure en Phase 1.5d; Phase 2 surfaceará `SpawnError::Persist`), `spawn_listener()` stub returning `SpawnError::{UnsupportedPlatform, NoWtComClsid, StubNotWired}`. Subprocess real (tokio::spawn `wtcli listen --event "agent.*" --json` + buffered line reader) lands con el host-loop effort de item 5+ / item 6. Default 268 (+6 agent_events), combined `acp-server,dag_mode,codebase-graph` 369 (+11 sobre 358). `cargo check/clippy --features acp-server -- -D warnings` limpio; `cargo fmt --check` limpio.
  6. ✅ RFC 04/19 patches — ACP como frontend más (al lado de HUD/CLI); `session/set_mode` override de state documentado en RFC 19 §6. NUEVO: RFC 19 §6.1.2 añadido — tabla bidireccional ACP mode ↔ `SupervisorState` (architect→plan, code→exec, ask→review), reglas de override (no resetea `mission_failure_count`, anti-hand-stall en `recovering`), persistencia Journal M6 audit con `action = "set_mode_override"` + `source: "acp"`. Override en `dag_mode` se inserta como edge `transitions_to` con `guard = "acp_human_override"`. RFC 04 §9 añadido — frontends del Orchestrator (CLI pura §1, HUD webview §2, ACP server §3 RFC 28 §B) contratación método por método (initialize/session-new/session-prompt/set-mode/cancel) y single-binary safety via ` Stdio` builtin + tokio runtime reuso. **Handler `session/set_mode` implementado en `acp/mod.rs`** (no es sólo documentación): recibe `SetSessionModeRequest`, `tracing::info!` con session_id + mode_tag + `Option<SupervisorState>` parseado via `mode_mapping::parse_acp_mode_id`, responde `SetSessionModeResponse::new()`, y emite `SessionUpdate::CurrentModeUpdate` notification via `SessionNotification` para que IT refleje. 3 tests nuevos en `acp::tests`: set_mode_response_is_default, current_mode_update_round_trips, unknown_acp_mode_is_parsed_as_none. Default 268, combined 372 (+3 sobre 369).
  7. ✅ Install README: cómo IT Settings.json apunta a `atlas` como delegate-agent (Alt+Shift+B); troubleshooting para `WT_COM_CLSID` discovery. NUEVO: `docs/intelligent-terminal-integration.md` (9.8KB) — operator-facing install guide: prereqs (build `--features acp-server`, IT 0.1.1+ Win10 build 19041+), `atlas` on `PATH`, register `atlas` como IT delegate-agent en `Settings.json` (JSONC example con `agents.opencode` + `delegateAgent: "opencode"`), built-in slash commands table (6 commands con behaviour summary), `agent_session_events` worker explanation (Channel 2 + M14 schema), troubleshooting `WT_COM_CLSID` discovery (5 diagnostic steps con `ATLAS_ACP_FORCE=1` override para smoke tests), uninstall recipe. Cross-refs al RFC 28 §B + RFC 19 §6.1.2 + RFC 04 §9 + `src-tauri/specs/osc-9001.md` + `agent-client-protocol` crate. MS trademark note (per Microsoft policy, user-facing surfaces = "agent pane integration", never "Intelligent Terminal").
  8. ✅ Manual validation en IT 0.1.1+ instalado (Windows build 19041+). Script smoke-test: `atlas mission new` desde pane, slash commands, Hud deep-link en `agent.task.completed._meta.hud_url`. NUEVO: `tools/acp-smoke.ps1` (156 LOC) — power-shell smoke-test script that pipes JSON-RPC 2.0 requests at `opencode --features acp-server` via `ATLAS_ACP_FORCE=1` (no IT install required to run the protocol smoke). 15 assertions covers (1) initialize returns the Atlas OS agentInfo, (2) session/new response carries a v4 UUID sessionId, (3) `available_commands_update` notification advertises the 6 slash-command catalogue (`atlas mission new` first, `opencode fix` fifth), (4) `/opencode fix` slash on session/prompt streams `agent_message_chunk` BEFORE the `stopReason: refusal` response (handler order), (5) `session/set_mode` returns response THEN emits `current_mode_update` notification echoing the requested mode. Exit code 0 on full pass, non-zero on first failure (these are the only oils for CI pip upload). Stale note: `cargo build --manifest-path src-tauri/Cargo.toml --bin atlas --features acp-server` must precede invocation. Script run localmente during this item commit: 15/15 assertions pass, exit 0. Fix derivado: `run_acp_server()` en `cli/bin/opencode.rs` ahora redirige tracing stderr via `with_writer(std::io::stderr)` (era stdout, breakpoints JSON-RPC stream) — ACP stdout stays clean.

---

## §I — terminal-browser: browser real embebido en el pane del terminal

**Status: ✅ Fase 1 (items 1-5, 7 implementados: guía operador + módulo `browser/` + `atlas browser probe|open|ls|action` + skill `browser-pane` + evento `artifact_preview_opened`); item 6 card HUD ⏳ (RFC 65). Integración lateral (no bundling). Guía operador: `docs/terminal-browser-integration.md`.**

[`zenbu-labs/terminal-browser`](https://github.com/zenbu-labs/terminal-browser) (MIT) es un navegador real — Chromium vía el offscreen-rendering API de Electron — dibujado **dentro del terminal** mediante el [kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/). Envía sólo los parches de píxeles que cambian por frame, lee eventos de teclado/ratón/trackpad (con un helper Swift de input a nivel de OS en macOS) y los reinyecta sintéticos a Chromium. Da al agente y al operador una **superficie web viva en la misma pestaña/pane** donde corre `atlas`, sin abrir un navegador externo. Esta sección especifica la integración **lateral** (proceso externo, jamás bundleado) y delega la receta de instalación a la guía de operador.

### Source items

- **`zenbu-labs/terminal-browser`** (<https://github.com/zenbu-labs/terminal-browser>), MIT — binario + CLI `terminal-browser`. UI externa en un motor gráfico Rust con un React custom renderer que dibuja chrome + contenido al mismo canvas compartido.
- **`terminal-browser.sh`** — instalador upstream: `curl -fsSL https://terminal-browser.sh/install | bash` o `brew install terminal-browser`; upgrade con `terminal-browser upgrade`.
- **`terminal-browser action`** — CLI **agent-browser-compatible** para interactuar con browsers abiertos. Es el punto de entrada para que un agente (Atlas OS incluido) maneje la pestaña abierta.
- **claude-code-plugin** (<https://github.com/zenbu-labs/terminal-browser/tree/main/claude-code-plugin>) — plugin experimental (marketplace `zenbu-labs/terminal-browser`) que expone `/browser` y una API `Browser.open/close` para otros plugins. Es **referencia de patrón** del contrato "agente abre/cierra la superficie web", no una dependencia.
- **`pixel`** (<https://github.com/zenbu-labs/terminal-browser/tree/main/pixel>) — librería JS extraída de terminal-browser para construir apps gráficas en el terminal.
- **`herdr-plugin`** + **`skill/`** — plugin para el terminal `herdr` (auditado en RFC 27) y skill de agente empacada en el propio repo. Referencia, no dependencia.

### Por qué integración lateral (no bundling)

terminal-browser arrastra un runtime Electron/Chromium completo y (en macOS) un helper Swift de captura de input del OS. Eso **viola RFC 25 §11** (binario único 30–45 MB; sin Electron, sin DLLs). Por tanto se adopta el **patrón lateral 8.5** (proceso externo, ya usado para RustDesk en `research/28 - conductor & alt surfaces.md` y para `google/artemis` en RFC 38): Atlas OS **no** añade crates, **no** añade feature flags, **no** añade migraciones. La integración vive en (1) una guía operador en `docs/` y (2) un shell-out opcional desde el CLI/Skills que sólo se activa si `terminal-browser` está en `PATH`. Cero impacto en el default build.

### Objetivos

1. Documentar la receta de operador (install, terminales soportados, uso, SSH, Windows/WSL, troubleshooting) en `docs/terminal-browser-integration.md` (patrón §B item 7).
2. Añadir detección de capacidad en el CLI: `atlas browser probe` reporta si `terminal-browser` está en `PATH` y si el `$TERM`/terminal declara kitty graphics protocol. Comando de sólo lectura (no lanza el browser).
3. Shell-out `atlas browser open <url> [--split right]` que delega en `terminal-browser open <url>`. Fail-safe: sin el binario responde con instrucciones de install — nunca un no-op silencioso (misma disciplina que §B `/atlas fix`).
4. Skill bundled `browser-pane` (opt-in) que enseña al Planner a abrir artifacts HTML (planes de RFC 12, reportes de Validation RFC 14, diffs) en un split pane al lado del agente vía `terminal-browser open`.
5. (Fase 2) Puente de capability web: envolver `terminal-browser action` en un wrapper Rust tipado (`BrowserAction { navigate, click, type, snapshot }`) para que el Orchestrator use la web como una tool. Decisión post-MVP tras medir el contrato real de `agent-browser`.
6. (Fase 2) Alineación con §B: si una sesión ACP/pane está activa, emitir `artifact.preview.opened` en el Journal para que el HUD muestre qué artifact se está previsualizando.

### Contrato de integración (plan, no implementación)

- **Detección**: `command -v terminal-browser` (WSL/Unix) — cacheada por sesión.
- **Terminal capability**: heurística por `$TERM`/`$TERM_PROGRAM` (`ghostty`, `xterm-kitty`, `cmux`, …) + `$KITTY_WINDOW_ID`/`$GHOSTTY_RESOURCES_DIR`. NUNCA se afirma soporte sin señal (honestidad RFC 28 §Riesgos 6): sin señal → aviso "terminal no verificado para kitty graphics protocol".
- **Apertura**: spawn del proceso `terminal-browser` (detached) con el URL; nunca bloquea el loop del supervisor.
- **Cierre / listado**: `terminal-browser ls` para enumerar; cierre vía el propio CLI.
- **Errores**: `enum BrowserIntegrationError { NotInstalled, TerminalUnsupported, SpawnFailed }`, cada variante con remediación textual.

### Comandos CLI nuevos (plan)

```bash
atlas browser probe                       # ¿instalado? ¿el terminal soporta kitty graphics?
atlas browser open https://example.com    # delega en `terminal-browser open`
atlas browser open ./plan.html --split right
atlas browser ls                          # delega en `terminal-browser ls`
# Sin el binario en PATH => mensaje de install, exit != 0 (no silent no-op)
```

### Seguridad (RFC 18)

- terminal-browser **ejecuta un motor Chromium completo** accesible al agente: nueva superficie de ataque. Por defecto **opt-in** y **fuera** del sandbox de skills firmadas.
- El agente que usa `action` obtiene **capacidad de red/deep-links**: clasificarlo como `SensitiveAction` y exigir confirmación en modos `HUMAN_IN_LOOP`/`AUTOPILOT` (RFC 21).
- No se aceptan URLs provenientes de skills no firmadas sin pasar por la política de SensitiveActions.
- El proceso es externo: no hereda secretos del keychain ni de `profiles::Profile`.

### Actualizaciones a RFCs existentes (plan)

- **RFC 24 §16** — mencionar terminal-browser como superficie de preview externa opcional (junto al HUD web).
- **RFC 08** — documentar la familia `atlas browser` (subcomandos de sólo lectura + shell-out).
- **RFC 18** — registrar la capacidad web del agente como SensitiveAction (opt-in).
- **RFC 26** — cross-ref `terminal-browser` (aplicado en este commit).
- **Este archivo (RFC 28 §I)** — checklist abajo.

### §I Checklist

1. ✅ Guía operador `docs/terminal-browser-integration.md`.
2. ✅ `atlas browser probe` (sólo lectura, sin spawn) — detecta binario + `--version` + capacidad kitty por señal real (`browser/mod.rs`, tests; NUNCA afirma soporte sin señal).
3. ✅ `atlas browser open|ls|action` shell-out + `BrowserIntegrationError { NotInstalled, TerminalUnsupported, SpawnFailed }` con remediación textual; `open` sin binario → exit != 0 (no silent no-op).
4. ✅ Skill bundled `browser-pane` (opt-in; `skills/browser-pane/`, catálogo `BUNDLED_COUNT=16`) que abre artifacts HTML en split pane.
5. ✅ Wrapper tipado `BrowserAction { Navigate, Click, Type, Snapshot }` + `atlas browser action` passthrough + `run_typed_action` (Orchestrator); contrato `agent-browser` NO pinneado (sin inventar flags).
6. ✅ Evento `artifact_preview_opened` (bus `BusEventKind` + Journal) emitido por `atlas browser open`; ⏳ card HUD (RFC 65 P3).
7. ✅ `probe` reporta versión + `ACTION_CONTRACT_PINNED=false` (honestidad si el contrato cambia); validación de shapes pendiente de medir el contrato real.

Atribución: cualquier port de patrón desde `zenbu-labs/terminal-browser` (MIT) debe preservar la nota de copyright en el module prose. Atlas OS **no** redistribuye el binario.

---

## Crates nuevas (justificación single-binary-safe)

| Crate | Versión ancla | Licencia | Justificación | Binario coste |
|---|---|---|---|---|
| `agent-client-protocol` | `=2.0.0` | Apache-2.0 | Implementa ACP v1 + v2 draft; IT ya lo usa | ~500 KB |
| `sacp-tokio` | latest stable | Apache-2.0 | Runtime helper JSON-RPC stdio loop sobre nuestro tokio existente | ~150 KB |
| `petgraph` | latest stable | MIT/Apache-2.0 | Pure Rust graph traversal; sustituye necesidad de Python Leiden | ~200 KB |
| `tree-sitter` | latest stable | MIT | C runtime linked estático, ya usado por ecosistema Rust | ~3 MB |
| `tree-sitter-rust` | latest stable | MIT | Grammar para EXTRACTED edges en codebase Atlas OS | ~1.5 MB |
| `tree-sitter-svelte` | latest stable | MIT | Grammar para EXTRACTED edges en frontend Svelte | ~1.5 MB |
| `serde_yaml` | `0.9` | MIT/Apache-2.0 | (ya plausible en Cargo.toml; verificar) | ~300 KB |

### Rechazadas con reasoning en RFC 22

- **`hurl`** (Orange-OpenSource): aunque Rust + single-binary, su DSL pierde semántica del journal typed. Decisión: YAML > DSL.
- **`graphify` runtime** (Python): viola RFC 25 §11. Sólo patrón; `petgraph`+`tree-sitter` bastan.
- **`atavia`, `hurlfmt`, `cargo-httpie`**: no existen como crates individualizados (404s verificados). No inventar.

---

## Riesgos transversales

1. **Scope del RFC 28 es enorme**: 4 secciones, 3 migrations, ~6 crates, 6+ módulos Rust, 3+ components Svelte, múltiples RFCs tocados. Alto riesgo de bloqueo si se hace en paralelo descentralizado. Mitigación: ordering Phase 1.5a→d estricto, **no mezclar**.

2. **ACP v2 + graphify son "young technologies"**: ambas anunciadas <2 meses (Jul 2026). Pin exact versions; bake-in migration path en RFC 22. Cualquier breakage upstream debe herrumbeado en tests CI.

3. **Karpathy tweet multi-agente branches**: el tweet NO fue fetchado de primera mano (sources corroboran). RFC 28 no reclama implementar branches ahora. Si orquestador quiere multi-agent autoresearch, **RFC 29 separado** — esta es la barrera honesta.

4. **`acp-server` feature flag default OFF en macOS/Linux** sin IT. §B es **opt-in Windows-first**.

5. **`serde_yaml` está mantenido en modo pasivo**: el maintainer declaró que no añade features. Para YAML fidelity estricta con posting, considerar `serde_yml` (fork activa) — monitorear churn.

6. **Honesty sobre URLs no verificadas**: este RFC sólo cita URLs verificadas: `graphify.com`, `hn.algolia.com`, `github.com/microsoft/intelligent-terminal`, `agentclientprotocol.com/protocol`, `crates.io/crates/agent-client-protocol`, `news.ycombinator.com` stories 48373231/48378013/48448582. **NO se cita el URL del tweet de Karpathy**, **NO se cita Reddit JSON**, **NO se cita `127.0.0.1:<port>/mcp` en IT**.

7. **RFC 28 no toca single-binary invariant**: las 3 crates nuevas son statically-linkable, sin Python, sin Conda, sin DLL cargada en runtime. Cumple AGENTS.md §6.

---

## Apéndice — Research sources internos

- `Atlas OS/research/27 - graphify pattern.md` (graphify.com pattern, Karpathy tweet corroboration)
- `Atlas OS/research/27 - posting format.md` (posting.sh dev-only, hurl alternative rechazada)
- `Atlas OS/research/27 - intelligent terminal.md` (Microsoft IT, ACP spec, 7 ideas)
- `Atlas OS/research/28 - portable inventory.md` — **inventario exhaustivo de 30 ítems portables** (GR-001..012, PT-001..012, IT-001..014, AR-001..013) con URLs source exactas, LOCs, costes (XS/S/M/L), atribuciones jurídicas, y recomendación de portado por cost-benefit en 4 fases (0/1/2/3).
- `Atlas OS/research/28 - conductor & alt surfaces.md` — **análisis Conductor (conductor.build) + comparativa 4 libs terminal-UI + roadmap remote-live dual-PC**. Sector A: 8 patrones portables (CN-001..008) de Conductor para Fase 4 (Swarm). Sector B: comparativa InquirerPy/Inquirer.js/rich/terminal-kit — recomienda `cronvel/terminal-kit` (MIT) como base sister IDE-in-a-terminal (`src/cli-tui/` Node sub-paquete conecta al mismo Kernel Bus WS). Sector C: remote-live modelo Nate Gentile (NO captura estática — video stream <60ms + input forward) con RustDesk (Apache-2.0) host embed en Rust. Sector D: phasing Fase 4/5/6/v2.
- `Atlas OS/research/29 - Phase 2 model orchestrator.md` — **plan refinado Fase 2 Multi-model Orchestration** con evidencia primaria (11 papers arxiv cross-verified + LiteLLM + OpenRouter + Aider + async-openai + RouteLLM + docs.rs). 6 sub-fases atómicas commiteables: 2.0 Foundation (Provider enum + ModelRegistry M20 + JSON seed + Aider tri-model Profile) **✅ IMPLEMENTADO** — `orchestrator::{provider, registry}` (20 builtin + Custom, ProviderWire serde-safe, Registry con from_seed/from_bundled_seed/resolve/filter_by_resource_mode/filter_by_capability, 18-model LiteLLM MIT seed bundled, M20 migration 4 tablas, Profile tri-model + resource_mode + effective_*, 24 tests), 2.0.5 Provider Normalization Layer (gaps críticos G1 prompt caching, G2 token counter pre-flight, G5 cooldown per-provider, G8 ToolCall enum cross-provider, G17 Retry-After, G18 back-pressure semáforo) **✅ IMPLEMENTADO** — `orchestrator::{wire, tokenizer, cache_control, cooldown, backpressure}` (wire ~24 tests OpShape+ToolCall normalize/denormalize, tokenizer ~17 tests tiktoken-rs 0.6 o200k_base BPE + CharRatio fallback, cache_control 25 tests Anthropic ephemeral breakpoints+extract, cooldown 21 tests per-provider default + RetryAfterSource + DurationClampExt, backpressure ~20 tests Arc<Semaphore> lazy-init + try_acquire/acquire async + reset_for + plan_reconfigure no-unsafe). Cargo `tiktoken-rs = "0.6"`. 109 tests nuevos, 483 total. clippy/fmt/svelte-check/vitest verdes. 2.1 Routing Policy (RoutingStrategy enum 6 LiteLLM + M21 model_invocations + 3-buckets cascade fallback + G12 idempotency) **✅ IMPLEMENTADO** — `orchestrator::{routing, cascade, idempotency, cost_guard, data_parts}` (routing ~20 tests RoutingStrategy enum 6 variantes SimpleShuffle/LatencyBased/UsageBasedV2/LeastBusy/CostBased/Hybrid+Custom, RouteContext snapshot pura, RoutingConfig 3 buckets+max_fallbacks=5; cascade ~13 tests Cascade::next_target(FailureMode,healthy_for) weighted failover+bucket escalation+max_fallbacks cap+ExhaustionReason; idempotency 7 tests G12 RequestFrame can_cascade+filter_unexecuted; cost_guard 8 tests G11 AggregationPolicy trait shape+LinearCostGuard+NoAggregation; data_parts 11 tests DataPartBuffer Vercel AI SDK pattern transient vs persistent; M21 model_invocations 19 columns + 3 indexes; Profile.routing_config añadido pierde Eq retiene PartialEq impl Default). 39 tests nuevos, 542 total. clippy/fmt verdes. 2.2 Aggregation ✅ IMPLEMENTADO (orchestrator/aggregation/ mod + 6 submodules, AggregationMode 7 variantes serde-tagged, trait Aggregator async-trait, aggregator_for() unit-struct dispatch, 6 mode-specific AggregationPolicy cost guards, M22 reflection_episodes+council_votes, Profile.aggregation field, 39 tests nuevos 581 total, clippy/fmt verdes), 2.3 Auto-routing Classifier + MCP-aware **✅ IMPLEMENTADO** (sub-fase 2.3) — `orchestrator/classifier/` (mod + 5 submódulos: lexical/log_reg/embedding/router_id/mcp_filter). `TaskType` enum (12 concretos + Unknown), `TaskTypeClassifier` trait `#[async_trait]`, `AutoRouterConfig` (off-by-default), `McpToolFilter::pre_filter()` (G19 capability_tags AND tool_names), `RouterId::parse()` "router-auto-0.5"/"router-mf-0.116"/Literal fallback. `LexicalClassifier` regex-counts con word-boundary (12 buckets), `LogisticRegressionClassifier` multi-class softmax scratch (NO `linfa` — deferral AN-2.3-a en RFC 22 §8.2: linfa no tiene MLP feed-forward per Context7 verification, HybridLLM arXiv:2404.14618 §4.3 muestra logreg ya ~94% del MLP), `EmbeddingClassifier` fastembed-rs BGE-small 384-dim feature-gated. M23 migration (`task_classifier_decisions` UNIQUE por `(prompt_hash, classifier_kind)` + `model_affinity_cache` PK compuesta + `INSERT OR REPLACE`). `Profile.auto_router: AutoRouterConfig` añadido. Defaults RouteLLM-mf calibrated: `coding=0.116/plan=0.05/chat=0.20/fix=0.10`. 63 tests nuevos (638 total). clippy + fmt + svelte-check + vitest verdes. 2.4 Feedback Loop (`mf` experimental A/B vs classifier + affinity reader GROUP BY `(task_type, model_id)` → `ArcSwap::store` + `aggregation_cost_estimate` impl real). Auditoría Round 3 detectó 9 gaps críticos y 11 deferrables (G4/G6/G7/G9/G10/G13/G14/G15/G16/G19/G20) — todos categorizados.
- Hallazgos automáticos de `karpathy/autoresearch`: prepare.py readonly + train.py editable + program.md skill. Hill-climbing greedy: baseline → edit → git commit → run → grep métrica → keep/reset --hard. Karpathy tweet 9 Mar 2026: "the goal is not to emulate a single PhD student, it is to emulate a research community" — multi-agente via branches (deferido a RFC 29).

## Apéndice — Atribución obligatoria por módulo

Cada módulo Rust portado debe incluir module-level prose con atribución explícita (cumple AGENTS.md §4 "module-level prose OK"):
- `src-tauri/src/journal/yaml_format.rs` — `"Derived from src/posting/yaml.py:str_presenter in darrenburns/posting (Apache-2.0). Copyright Darren Burns."`
- `src-tauri/src/journal/export/posting.rs` — `"Format compatible with darrenburns/posting .posting.yaml schema (Apache-2.0). No runtime dependency on posting."`
- `src-tauri/src/graph/{cluster,report,paths,extract,affected}.rs` — `"Pattern ported from safishamsi/graphify (Apache-2.0). Copyright Graphify Labs."`
- `src-tauri/src/acp/{mod,commands,delegate}.rs` — `"Pattern ported from microsoft/intelligent-terminal (MIT). Copyright Microsoft Corporation. Does not use 'Intelligent Terminal' name in user-facing surfaces per MS trademark policy."`
- `src-tauri/src/supervisor/loop.rs` (Autoresearch branch) — `"Loop template modeled on karpathy/autoresearch program.md (MIT). Copyright Andrej Karpathy."`
- `skills/autoresearch/program.md` — verbatim copy with header `<!-- Verbatim from karpathy/autoresearch MIT; Copyright Andrej Karpathy. Do not remove this attribution. -->`
- `src-tauri/hooks/<cli>/hooks.json` — `"Mapping schema copied from microsoft/intelligent-terminal wt-agent-hooks/<cli>/hooks.json (MIT)."`

Cada distribución que incluya estos ports debe preservar `LICENSE` y `NOTICE` (Apache-2.0) o `LICENSE-MIT` según el caso.
