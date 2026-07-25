# RFC 28 — External Tool Integration

**Author:** opencode architect agent · **Date:** 2026-07-25
**Status:** Draft for orchestrator review
**Supersedes:** none · **Superseded by:** —
**Depends on:** RFC 02 (Journal), RFC 03 (Memory), RFC 04 (Orchestrator), RFC 06/23 (Skills), RFC 12 (Planning), RFC 14 (Validation), RFC 15 (Repair), RFC 16 (Learning), RFC 19 (Execution Supervisor), RFC 22 (Research Findings), RFC 24 (HUD), RFC 25 (Stack), RFC 27 (Orchestration Fundamentals)

**Scope:** Adds four loosely-coupled integration surfaces (§A–§D) on top of RFCs 02–27, using migrations **M13/M14/M15** (free per preamble). No new external runtime dependency is bundled — three new crates, all single-binary-safe. Karpathy tweet and graphify URL honesty preserved (§C claims no first-hand tweet fetch).

## §A — Karpathy autoresearch loop

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

### Objetivos

Hacer que `opencode` sea un **ACP agent de primera clase** detectable por Intelligent Terminal 0.1+ (autodetecta OpenCode en PATH, README §Get Started). El usuario arrastra `/opencode fix`, `/opencode restart`, `/opencode exec step` en el pane. HUD Mission Control sigue siendo el surface visual canonical; el pane es *entrada ligera diaria*.

### Cambios Rust

- `src-tauri/src/acp/mod.rs` (nuevo): servidor ACP usando `agent-client-protocol` v2.0.0 con `features = ["unstable_mcp_over_acp","unstable_session_fork"]` + `sacp-tokio`. Implementa métodos **required**: `initialize`, `session/new`, `session/prompt`, `session/cancel`. **Optional**: `session/load` (replay Journal M1–M12 via `session/update`), `session/resume`, `session/close`, `session/delete`, `session/list`, `session/set_mode` mapeado a estados RFC 19 (PLAN→architect, EXEC→code, REVIEW→ask), `logout`.
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
- **Windows-only**: IT es MSIX, Win10 build 19041+. En macOS/Linux, `opencode` sin IT funciona normalmente pero ACP server idles. Feature flag `acp-server` default off; on solo detectando `WT_COM_CLSID`.
- **PowerShell hooks chez IT evolucionan**: nuestra implementación debe leer `hooks-upgrade-state.json` e idempotente skip si la version bundle IT ≥ nuestra.
- **Multi-agent tracking limitation**: faq Q8 dice IT solo trackea el MISMO agent para pane + delegate. Si usuario usa delegate con CLI diferente, nuestra HUD session list no lo verá. Documentado.
- **Token cost de replay**: `session/load` reenviar Journal M1–M12 al pane completo puede ser Mbytes. Mitigación: payload truncation con `outputByteLimit` del spec.

---

## §C — graphify pattern adoption

### Objetivos

Adoptar el **patrón** (NO la dependencia Python — graphify es Python, viola RFC 25 §11 / AGENTS.md §6). Cuatro usos internos: (1) RFC 19 state machine como DAG navegable en SQLite; (2) HUD `<GraphView>`; (3) skills como graph templates (RFC 23); (4) Planner emite DAG. Un quinto uso **opcional externo**: usuario con graphify MCP HTTP ya instalado puede apuntar OpenCode OS skill-backend a él — **zero binary weight**.

### Cambios Rust

- `src-tauri/src/graph/mod.rs` (nuevo): `MissionGraph { nodes: Vec<Node>, edges: Vec<Edge> }`, `Node { id, kind, label, provenance: EXTRACTED|INFERRED|AMBIGUOUS, attrs_json }`, `Edge { src, dst, kind: calls|imports|transitions_to|depends_on, precondition, guard, visit_count }`.
- `src-tauri/src/graph/traverse.rs`: `shortest_path`, `god_nodes`, `get_neighbors`, `community_partition` (**comunidades via union-find simple, NO Leiden** — ver Riesgos). Usa `petgraph` (MIT/Apache-2.0, pure Rust).
- `src-tauri/src/graph/ast.rs`: `tree-sitter` + 2 grammars (`tree-sitter-rust`, `tree-sitter-svelte`) para `EXTRACTED` edges del propio codebase de OpenCode OS como meta-skill. Justification single-binary-safe: tree-sitter es C puro linked estáticamente, ya usado por muchos crates Rust.
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

### Objetivos

Cerrar **Brecha H** (RFC 27 §3.H — Audit log retention/ttl). Export entries a `.posting.yaml` antes de purgar SQLite. **NO hay dependencia runtime a posting**; solo formato. El usuario abre el yaml con `posting --collection ./snapshots/` si tiene posting, o lo versiona en git como YAML crudo.

### Cambios Rust

- `src-tauri/src/journal/export/posting.rs` (nuevo): `fn entry_to_yaml(entry: &AuditEntry) -> String` con `serde_yaml` + helper `literal_block` (equivalente a `str_presenter` de posting). **Ignora el campo `scripts`** al escribir (los entries no tienen scripts; si importamos collections de terceros, también ignora — security boundary AGENTS.md §6).
- `src-tauri/src/journal/export/retention.rs` (nuevo): worker SQLite hook; antes de purgar entries `age >= ttl_days`, exporta a `~/.opencode/snapshots/YYYY-MM-DD/*.posting.yaml`.
- `src-tauri/src/cli/bin/opencode.rs`: `opencode audit --export-posting -n 50 -o ./snapshots/`. Hardcodeamos `posting_version: "1"` con comentario `# x-opencode-exported: RFC 28 §D`. Si posting sube major, bump manual.
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
- **RFC 27 §3.H Brecha H**: closing note.
- **RFC 25 §3.4**: confirmar `serde_yaml = "0.9"` ya presente; si no, añadir (sin costo binario real).
- **RFC 23 §7.2**: skills pueden shippear `requests/*.posting.yaml` bundles — el parser RFC 23 **requiere reject del campo `scripts`** explícitamente (security boundary AGENTS.md §6).

### Riesgos

- **`posting_version` drift**: si posting rompe compatibilidad en major, snapshots antiguos no cargan. Mitigación: persistir el `posting_version` dentro de cada snapshot y un `README.md` en el dir snapshot documentando versión requerida.
- **YAML literal-block fidelity**: nuestro `str_presenter` puede diferir del de pydantic. Mitigación: test contra un `.posting.yaml` sample canónico leído de `raw.githubusercontent.com/darrenburns/posting/main/...`.
- **Volume**: un audit con 10k entries genera 10k archivos yaml si uno por entry — mala UX. Mitigación: **bundle en collection subdirs by day**, un archivo por entry-directory (max 100 entries/arch) — decisión de packing.
- **No usar hurl**: decidido. `.hurl` DSL es asserts-oriented, pierde semántica del journal typed. Documentemos como alternativa rechazada en RFC 22.
- **Import collections de terceros**: superficie ataque. Aunque ignoremos `scripts`, el YAML puede contener URLs/auth en cleartext. Mitigación: parser Rust no reenvía nada; snapshot dir es local-only.
- **Detalles auth enum**: `Literal["basic","digest","bearer_token"]` enumerado necesita struct explicita.

---

## Orden recomendado — Phase 1.5

**Recomendado (justificado):** Start con **§D primero**, §A segundo, §C tercero, §B cuarto.

### Razón

1. **§D AuditLog export** (Phase 1.5a, 1–2 sprints): crate packaging más pequeño (`serde_yaml`), número más bajo de archivos tocados, cierra Brecha H sin tocar engines. Deployable primero, reduce riesgo. Tests fáciles. Confidence builder.

2. **§A Autoresearch loop** (Phase 1.5b, 2–3 sprints): extiende RFC 19 supervisor sin tocar arquitectura. Schema M13 aislada, un branch de modo. Requiere `git` subprocess (ya tenemos via CLI bin, no nuevo crate). HUD card incremental. No depende de §B/§C. **Si el ACP server retrasa, autoresearch es demo-ready standalone.**

3. **§C Mission graph M15** (Phase 1.5c, 3–4 sprints): el más disruptivo. Toca Planner, Skills, Supervisor, Learning, HUD. Crates `petgraph`+`tree-sitter` son bump binario. Hacerlo **third** aumenta conocimiento del codebase por lo aprendido en §A/§D. **No deberíamos hacer §C primero**: el scope del refactor Planner es grande.

4. **§B IT ACP server** (Phase 1.5d, 4–5 sprints): **último** porque depende de (a) el resto del sistema estable, (b) IT 0.1.x y ACP v2.0.0 son moving targets (`unstable_*` features), (c) prueba-target limitada a Windows, (d) si §C introduce DAG planner, la superficie `session/set_mode` mapping se beneficiará de tener states graph ya consolidado. Adicionalmente: el `exec step` de §A es el mismo `opencode exec step` que §B advertisea — armonizar antes de implementar §B reduce rework.

### Sub-enum Phase 1.5

- **1.5a — §D snapshot export**:
  1. `serde_yaml` in Cargo.toml (Context7 verify)
  2. `posting.rs` + tests contra sample canónico
  3. `retention.rs` SQLite hook + CLI flag
  4. HUD button + endpoint
  5. RFC 24 §10 patch + RFC 27 Brecha H closed
  6. RFC 22 entry

- **1.5b — §A autoresearch**:
  1. M13 migration
  2. `autoresearch.rs` + supervisor branch
  3. CLI flag `--autoresearch --metric --max-steps --timebox`
  4. HUD card + WS channel
  5. RFC 19 § Autoresearch mode patch
  6. Real-world hill-climbing run on a lint metric as acceptance test

- **1.5c — §C mission graph**:
  1. Crates: `petgraph`, `tree-sitter`, `tree-sitter-rust`, `tree-sitter-svelte` (Context7 verify versiones compatibles con Rust 1.84)
  2. M15 migration
  3. `graph/{mod, traverse, ast}` módulos
  4. Planner DAG emitter — detrás de feature flag `dag_mode`
  5. Skills `graph.toml` loader
  6. Learning graphs persist + retrieve por cosine
  7. `GET /hud/graph/:id` + `<GraphView>` + tests
  8. RFC 12/16/19/23/24 patches
  9. RFC 22 entries (4 crates)

- **1.5d — §B IT ACP**:
  1. Crates: `agent-client-protocol = "=2.0.0"`, `sacp-tokio` (Context7 verify; `unstable_*` features)
  2. M14 migration
  3. `acp/{mod, commands, delegate, mode_mapping}` + tests con mock JSON-RPC stdio
  4. CLI plumbing para `/opencode fix`, `/opencode restart`, `/opencode exec step`
  5. `wtcli listen --json` worker → events SQLite
  6. RFC 04/19 patches
  7. Install README: cómo IT Settings.json apunta a `opencode`
  8. Manual validation en IT 0.1.1+ instalado (Windows)

---

## Crates nuevas (justificación single-binary-safe)

| Crate | Versión ancla | Licencia | Justificación | Binario coste |
|---|---|---|---|---|
| `agent-client-protocol` | `=2.0.0` | Apache-2.0 | Implementa ACP v1 + v2 draft; IT ya lo usa | ~500 KB |
| `sacp-tokio` | latest stable | Apache-2.0 | Runtime helper JSON-RPC stdio loop sobre nuestro tokio existente | ~150 KB |
| `petgraph` | latest stable | MIT/Apache-2.0 | Pure Rust graph traversal; sustituye necesidad de Python Leiden | ~200 KB |
| `tree-sitter` | latest stable | MIT | C runtime linked estático, ya usado por ecosistema Rust | ~3 MB |
| `tree-sitter-rust` | latest stable | MIT | Grammar para EXTRACTED edges en codebase OpenCode OS | ~1.5 MB |
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

- `OpenCode OS/27 - Research - graphify.md` (graphify.com pattern, Karpathy tweet corroboration)
- `OpenCode OS/27 - Research - Posting TUI Integration.md` (posting.sh dev-only, hurl alternative rechazada)
- `C:\Users\Max\AppData\Local\Temp\opencode\it-research-report.md` (Microsoft IT, ACP spec, 7 ideas)
- Hallazgos automáticos de `karpathy/autoresearch`: prepare.py readonly + train.py editable + program.md skill. Hill-climbing greedy: baseline → edit → git commit → run → grep métrica → keep/reset --hard. Skill opencode local `~/.agents/skills/_agents/misc/autoresearch/SKILL.md`. Karpathy tweet 9 Mar 2026: "the goal is not to emulate a single PhD student, it is to emulate a research community" — multi-agente via branches (deferido a RFC 29).
