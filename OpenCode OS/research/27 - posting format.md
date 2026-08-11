# 27 - Research: `posting` TUI (darrenburns) — Applicability to OpenCode OS

**Investigación deep-dive** para alimentar un RFC sobre integración parcial en OpenCode OS
(Rust/Tauri 2/SvelteKit/axum/SQLite). Fuentes verificadas entre 2026-07-25.

---

## 1. Cita canónica del schema `.posting.yaml`

Fuente: `src/posting/collection.py` (rama `main`, leído vía raw.githubusercontent.com).
El `RequestModel` (pydantic) se serializa con `model_dump(exclude_defaults=True, exclude_none=True)`
+ representer custom (`str_presenter`) que usa estilo YAML literal-block `|` para strings multilínea
(`src/posting/yaml.py`). Campos del schema:

```
name, description, method (GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS),
url, body{content,form_data[]}, auth{type: basic|digest|bearer_token, ...},
headers[{name,value,enabled}], params[{name,value,enabled}],
path_params[{name,value}], cookies[exclude], posting_version,
scripts{setup, on_request, on_response}, options{follow_redirects,
verify_ssl, attach_cookies, proxy_url, timeout}
```

- **Colección = directorio plano** + `*.posting.yaml`. Sin metadata, sin manifest. Carga con
  `directory.rglob("*.posting.yaml")`.
- **`posting_version` embebido** en cada archivo — evolución de schema sin migración.
- **Variables**: sintaxis `$VAR` y `${VAR}`; cargadas desde `--env file.env` o `posting.env`
  auto-loaded en CWD. `use_host_environment=true` permite acceso a OS env.
- **Scripts**: tres hooks en el mismo archivo YAML (paths relativos al collection dir):
  `setup(posting)` → `on_request(request, posting)` → `on_response(response, posting)`.
  `posting.set_variable/get_variable/notify`. Viven **en el mismo proceso Python** (no sandbox).
- **Import**: curl (paste en URL bar, experimental), OpenAPI 3.x (`posting import path.yaml`),
  Postman (`--type postman collection.json`, extrae vars a `.env`).
- **Jump mode** (`src/posting/jump_overlay.py`, 4 KB): overlay de letras sobre widgets focusables;
  referente UX confirmado por HN user `connordavenport` ("I love the jump mode navigation").
- **Themes**: SCSS, builtin + custom; tree-sitter para syntax highlight.

Docs referencia: <https://posting.sh/guide/requests/>, /collections/, /scripting/, /environments/, /importing/.

---

## 2. Testimonios de comunidad (URLs verificadas)

- **Show HN 2024-07-10, 184 pts / 51 comments**: <https://news.ycombinator.com/item?id=40926211>
  - `tusharsadhwani`: *"already the best API testing client that I have found... runs on a VPS
    out of the box over SSH"* — patrón: SSH-to-remote.
  - `kbd`: *"first http gui that I'm actually sticking with... more efficient than
    up-arrow+enter"* — patrón: reemplazo permanente de Postman/Insomnia.
  - `astrodude`: refiere a `kiranz/just-api` (YAML-based) — patrón: YAML-on-disk gana minds.
  - `gregwebs` + respuesta de Darren: *"integration with Hurl could be very nice... further
    down the line"* — dirección oficial del maintainer.
  - `eigenvalue`: pedido automático OpenAPI→YAML desde FastAPI — patrón de uso generativo.
  - Friction real reportada: `saila` (sobreescritura doble sin warning), `yoavm` (defección a
    JetBrains HTTP client), usuario anónimo ("Everything looks great except Python... I'll work
    on something similar with Go") — Python = fricción pública.
- Reddit: sin hilos dedicados en r/CLI en ventana 2024-07 (búsqueda vía reddit search JSON).
- X de Darren: handle `_darrenburns` (confirmado en footer de posting.sh); no scrapeable sin auth.

---

## 3. Ideas extrapolables a OpenCode OS

### (a) RFC 24 §10 AuditLog → export YAML on-disk

`RequestModel` es un **subconjunto propio** de nuestros M1–M12 entry types (especialmente
`tool_call` y `http_request`). Accionable:

1. Añadir `journal::export::posting_yaml(entry: &AuditEntry) -> String` en `src-tauri/src/journal/`.
2. Reutilizar `serde_yaml` (crate ya considerado en RFC 25 §3.4) + helper para `literal-block`
   (equivalente a `str_presenter`).
3. `opencode audit --export-posting -n 50 -o ./snapshots/` escribe un collection dir.
4. **El usuario puede abrirlo directamente con `posting --collection ./snapshots/`** — cero
   acoplamiento runtime, solo formato.

Compatibilidad: 100% — nuestro `http_request` entry mapea 1:1 con `RequestModel`. Nuestros
`tool_call` (no-HTTP) pueden omittir `method/url` o usar un extension key bajo
`x-opencode-*` (YAML permite keys arbitrarias).

### (b) Brecha H (RFC 27 §3.H — Audit log retention/ttl)

Posting es el **visualizador ideal para snapshots retenidos** post-TTL. Flujo propuesto:
1. SQLite rota entries con TTL (RFC 27 §3.H).
2. Antes del purgado, worker SQLite hook exporta las entries>=threshold a
   `~/.opencode/snapshots/YYYY-MM-DD/.posting.yaml`.
3. Infinite retention on-disk, bounded retention in-DB. El usuario inspecciona con posting
   sin necesidad de reabrir la HUD webview (que asume journal vivo).

### (c) MCP request bundles (RFC 23 — Prompt Understanding & Refinement)

Posting's collection-tree (dir + subdirs, sin manifest) es un **esquema de bundling de
requests** ya probado. Aplicable a skills:

- `skills/<name>/requests/*.posting.yaml` como **MCP request bundles** declarativos.
- El skill `prompt-clarify` puede shippear una collection de "diagnostic probes" (HTTP calls
  a `/v1/models`, `/v1/health`) como YAML checkin-able.
- Resolución de variables via `.env` por-profile encaja con `src-tauri/src/profiles/` (Hermes-style
  multi-profile del AGENTS.md §1). Un profile = un `.env` para posting.

**No ejecutar Python**: nuestros scripts son código Rust declarado en el skill (RFC 23 §7.2 ya
sigue este modelo). El campo `scripts` del YAML lo ignora OpenCode; posting sí lo usa si el
usuario abre la misma collection en su TUI.

### (d) Jump mode — referente UX para `+page.svelte` drawer

`src/posting/jump_overlay.py` (~4 KB, simple) es **portable a Svelte trivialmente**: un overlay
con hot-letters absolutas sobre focusables del drawer Mission Control. RFC 24 §3 (card anatomy)
no menciona jump — gap explícito. Propuesta: añadir `jump_mode: bool` en HUD card spec, rendering
de `[a]` badges y handler de tecla única. Issue/RFC distinto (UX HUD, no posting-import).

### (e) Alternativas Rust-native — comparativa

| Herramienta | Stars | Lenguaje | Single-binary | Bindings | Estado |
|---|---|---|---|---|---|
| **posting** (darrenburns) | ~3k (2024) | Python (Textual) | No (uv+py3.13) | N/A | Real, activo |
| **hurl** (Orange-OpenSource) | 19k | **Rust** | **Sí** (libcurl) | C ABI expuesta, sin bindings Rust oficiales | Real, Apache-2.0, mantenido |
| **atavia** | — | — | — | — | **No existe** (404 en github) |
| **hurlfmt** | — | — | — | — | **No existe** (404 — el subcomando real es `hurl fmt`, integrado en hurl) |
| **cargo-httpie** | — | — | — | — | No localizado; `httpie` CLI es Python. Crate `http-cli` (denyok) existe pero ~ abandoned |

Conclusión: la únicia alternativa Rust-native real y mantenida es **hurl**. Su formato `.hurl`
(plain-text, no YAML) es más test-orientado (asserts, captures) que desarrollador-orientado.
Posting elige YAML para diffs/VC; hurl elige DSL para asserts. **Para OpenCode OS, el formato
YAML de posting es mejor ajuste a nuestro journal SQLite-typed** (rows→YAML es directo;
rows→hurl DSL pierde semántica).

No recomiendo `hurlfmt` ni `atavia` (no existen — evitar inventar).

---

## 4. Puntos de fricción: por qué NO integrar posting como runtime

1. **RFC 25 §11 single-binary**: `posting` exige Python 3.13 + uv toolchain. No es bundleable
   sin romper el contrato single-binary. **Prohibido** como runtime.
2. **AGENTS.md §6 boundary**: *"No new external tools bundled (no Conda, no pyinstaller)"* —
   un subprocess `posting` violates literalmente. Evitar.
3. **Scripts Python en YAML** son superficie de ataque: ejecutan `setup/on_request/on_response`
   en proceso, sin sandbox. Si importamos collections de terceros en OpenCode OS para
   MCP request bundles (3c), **debemos ignorar la key `scripts`** al parsear. Documentar
   explícitamente en el RFC.
4. **`=httpx`/`=pydantic` dependency leakage**: el schema asume esos tipos. Si re-implementamos
   el parser en Rust con `serde`, matchear el `Literal["basic","digest","bearer_token"]` de `Auth`
   requiere enum explícito — menor, pero no trivial.
5. **`posting_version`** drift: cada archivo lleva version; nuestro exporter debe escribirla
   o el loader TUI se queja. Ver `<VERSION>` en `src/posting/version.py`.

**Features transferibles sin tocar el binario**:
- `.posting.yaml` schema (formato) ✅
- `--env` multi-file + override semantics ✅ (ya tenemos profiles)
- `posting import --type postman` → nuestro importador YAML ✅ (codec, no runtime)
- Jump mode overlay pattern ✅ (Svelte port)
- Command palette fuzzy search ✅ (Svelte port, RFC 24 HUD)
- Carga "directory as collection" sin manifest ✅ (filesystem walker en Rust)

**NO transferibles**:
- Python hooks (3) — reemplazo: skills declarativos Rust (RFC 23 §7.2).
- Textual widgets — irrelevantes, tenemos Svelte.
- Tree-sitter highlight — Svelte already has highlight.js/shiki; comparar costos en RFC.

---

## 5. Output para RFC consumidor

**Recomendación top**: Integrar `posting` como **formato de exportación** (no runtime).
- Nuevo módulo: `src-tauri/src/journal/export/posting.rs` — `fn entry_to_yaml(entry) -> String`.
- Nuevo CLI: `opencode audit --export-posting -n N -o DIR`.
- Brecha H (RFC 27 §3.H) usa esta infraestructura para snapshots pre-TTL.
- Skills shippean MCP request bundles en formato `.posting.yaml` (RFC 23 §7.2 ext).
- Jump mode → RFC separado (HUD UX), citar `src/posting/jump_overlay.py` como referencia.
- NoInvoker el campo `scripts` en parser (security boundary, AGENTS.md §6).

**Cargo.toml delta sugerido** (validar via Context7 antes del RFC):
- `serde_yaml = "0.9"` (probablemente ya presente).

**Sustentación RFC 22 (Research Findings)**: este reporte debe ser referenciado como
justificación para añadir el módulo de exportación. Sin nueva crate externa requerida
(`serde_yaml` ya cubierto). Cumple §6 boundary rules.

---

## Fuentes

- Repo + código: <https://github.com/darrenburns/posting>, rama `main`, leído 2026-07-25.
- Docs: <https://posting.sh/guide/> (collections, requests, scripting, environments, importing).
- Schema fuente: <https://raw.githubusercontent.com/darrenburns/posting/main/src/posting/collection.py>.
- YAML helper: <https://raw.githubusercontent.com/darrenburns/posting/main/src/posting/yaml.py>.
- HN Show HN: <https://news.ycombinator.com/item?id=40926211> (184 pts, 51 comments, 2024-07-10).
- Hurl: <https://github.com/Orange-OpenSource/hurl> (19k stars, Rust, Apache-2.0).
- `atavia`, `hurlfmt`, `cargo-httpie` como crates nombrados por el requester — **NO
  localizados/existentes**; anotado para que el RFC no los invente.
- X handle Darren Burns: `_darrenburns` (footer posting.sh); perfil no scrapeable sin auth.
