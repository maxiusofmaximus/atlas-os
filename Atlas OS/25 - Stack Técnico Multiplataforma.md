# 25 - Stack Técnico Multiplataforma

> Define la pila concreta que corre Atlas OS en **Windows, Linux y macOS** sin depender de otros de un sistema operativo y sin obligar al usuario a instalar Node.js/Python/servidores externos. El resultado es un binario único (Tauri 2 / Rust) con frontend web, que puede correr tanto como app de escritorio responsive como servicio local accesible desde el móvil, manteniendo **cero dependencias externas** salvo el OS.

La elección no es casual. Reemplazar la combinación típica "Electron + Node + Python + Postgres/Annoy + Redis" por **"Tauri 2 + Rust + SQLite/sqlite-vec + LSP"** logra:

- ~30 MB de installer, no 200+ MB (Electron).
- ~30-40 MB RSS vs 200-600 MB de Electron.
- Arranque cold start <1s vs 3-6s.
- Cero runtime deps. No requiere Node/Python/PostgresGUI/Redis.
- Mismo stack y mismo binario en Windows, Linux, macOS.
- Multi-modal sin instalar Python headless; el motor de IA se comunica por HTTP(S) con proveedores (locales o remotos) **o** con `llama.cpp` bundled como sidecar.

---

## 1. Decisión

| Capa | Elección | Alternativa explícita descartada | Motivo |
|---|---|---|---|
| App shell | **Tauri 2** | Electron, CEFI, Flutter, Qt | Rust-side, webview OS-nativo, 30MB, multi-OS. |
| Lenguaje core | **Rust** (2021 edition, stable 1.84+) | Node, Go, Python | Seguridad memoria, cero-runtime, alto paralelismo, AVE. |
| Frontend | **SvelteKit** (SSR off; CSR only en Tauri webview) | React, Vue, SolidJS | más liviano, signals reactivos, >30k fewer DOM nodes en HUD (`24`). Alternativa Serde JSON + Vanilla para builds minimal. |
| Estado frontend | **Zag.js / Nano Stores** | Redux, Zustand | atomicidad агрегada para HUD live updates. |
| Storage relational | **SQLite** (vía `rusqlite` + WAL mode) | PostgreSQL, DuckDB-embedded | single-file, ACID,deposito ubicuo, sin servidor. |
| Vector store | **`sqlite-vec`** (https://github.com/asg017/sqlite-vec) | ChromaDB, pgvector, FAISS | mismo binario SQLite, vector + SQL, CTE-friendly. |
| Embeddings locales | **`fastembed-rs`** (Qdrant, optimized ONNX) | OpenAI API, sentence-transformers Python | offline, free, no Python runtime. |
| Embeddings remotos (opcional) | Cohere, OpenAI, Voyage | solo OpenAI | diversidad providers. |
| LSP | **Tower-LSP** (Rust) | lsp-rs, direct JSON-RPC | framework oficial de reference implementation. |
| Server runtime web | **`axum`** (Rust) | Actix, Rocket, Node/Fastify | Tokio-compatible, hyper-bound, ideal para WebSocket. |
| WebSocket | **`axum::extract::ws`** | Soketi, ws-rs simple | integrado en axum, handlers type-safe. |
| Serialización | **`serde`** + `serde_json` / `postcard` | msgpack, protobuff | estándar Rust, JSON para protocol HUD. |
| Async runtime | **Tokio** | async-std, smol | standard de facto. |
| HTTP client | **`reqwest`** + `hyper` | ureq | TLS rustls, streaming SSE, multi-provider. |
| CLI modes | **`clap`** (v4) | cobra (no Rust),structop | mejor hacer docs, derivadas. |
| Logging | **`tracing`** + `tracing-subscriber` | log + env_logger | spans jerárquicos, correlacionar con Kernel Bus (`02`). |
| Sandbox exec | **Docker / Podman** (vía API) + **Firejail** (Linux) + **Job Object** (Windows) | `bwrap`, `isolate` | multi-OS, container hot path, fallback OS-level (`18`). |
| Local model bridge | **Ollama / llama-server (llama.cpp) / LM Studio HTTP API** | Candle Rust, Burn Rust | ya instalados; no reimplementamos la inferencia. |
| Bundled sidecar (opcional) | **`llama.cpp`** compilado Rust binding (`llama-cpp-rs`) | custom | para usarios sin Ollama. |
| Secrets | **OS keychain** (`keyring-rs` crate) | `.env`, dotfile plaintext | Credential manager nativo multi-OS. |
| Telemetría local | **SQLite + tracing** | Sentry (cloud), OTel | zero-cloud (privacidad). |
| Self-update | **Tauri Updater** + public-keysigned manifests | electron-updater | incluido en Tauri 2. |
| Bundled distrib | **Tauri 2 MSI** (Windows), **`.deb`/`.rpm`/AppImage**, **`.dmg`**. | Snap, Flatpak | no requiere app store; installation directa. |
| CI/CD | **GitHub Actions matrix** | self-hosted | cross-OS build automático. |
| Builds reproducibles | **Nix flakes** (opcional) | Docker-only | determinismo, útil para marketplace skills firmadas (`18`). |

---

## 2. Topología física

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Native Tauri 2 App                                                        │
│ ┌─────────────────────────────────┐   ┌──────────────────────────────┐   │
│ │ Rust core (process 1)           │   │ Webview (SvelteKit CSR)      │   │
│ │  • axum localhost:0 (HUD WS)    │◀──│  • HUD Mission Control       │   │
│ │  • LSP host (Tower-LSP)         │   │  • Editor (CodeMirror 6)     │   │
│ │  • MCP host (stdio bridge)      │   │  • Approvals dock             │   │
│ │  • Sandbox supervisor           │   │  • Skill picker              │   │
│ │  • Heartbeat daemon             │   └──────────────────────────────┘   │
│ │  • Journal writer (rusqlite)    │                                      │
│ │  • Embedding indexer (fastembed)│                                      │
│ │  • Sandbox sidecars             │                                      │
│ │    - llama.cpp  (opcional)      │                                      │
│ │    - Docker / Podman            │                                      │
│ └─────────────────────────────────┘                                      │
│                     │                                                    │
│                     │ HTTP/SSE/WS                                        │
│                     ▼                                                    │
│ ┌────────────────────────────────────────────────────────────────────┐ │
│ │ Model providers (separados, no bundled salvo opcional sidecar)      │ │
│ │   • Locales:  Ollama (11434)  LM Studio (1234)  llama-server (8080)  │ │
│ │   • Cloud:    Anthropic OpenAI Gemini DeepSeek NIM Cerebras Groq   │ │
│ │               Sambanova Cloudflare HF Inference Mistral Cohere     │ │
│ └────────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────┘

                     ▲
                     │ OIDC + WS  (TLS, pubkey)
                     │
   Móvil/remote browser ──► https://desktop.local/   o   https://<ngrok-tls>/
```

Ventajas físicas:

1. El Rust core es **process 1** — persistente, sobrevive a webview crash/reparate (`19 - Execution Supervisor.md`). Si el webview muere, los subagentes siguen corriendo; el usuario recarga el HUD sin perder estado.
2. El HUD web se sirve desde el propio axum en `localhost:0` y, si el usuario abre forward tunnel/ tailscale spectrum / cloudflare tunnel, también es accionable desde móvil (`24 - HUD Mission Control.md` §16).
3. Cualquier browser puede ser cliente: Tauri desktop, Chrome desktop, Safari mobile, etc. La UI es la misma.

---

## 3. Detalle por componente

### 3.1 Tauri 2 (`tauri@2`, Rust 1.84+)

- **Por qué sí:** Rust-side, multi-OS, webview nativo (WebView2 en Windows, WebKitGTK en Linux, WKWebView en macOS). Tamaño de installer <30MB. Acumula por bundle sidecars non-bloqueante (path Tauri *sidecar* API).
- **Por qué no Electron:** 200+ MB, Chromium bundled, mayor RR, vulnerabilidades supply-chain Node. AionUI usa Electron; Atlas OS supera eso mismo stack.
- **Por qué no Flutter / Qt:**  UI no-nativa, lenta derecho "web dev workflow" del equipo, ecosistema skills/MCP vive en JS/TS (`06 - Skills.md`).
- Build lockfile: `Cargo.lock` + `tauri.conf.json`+ `bun.lockb` o `pnpm-lock.yaml`.

### 3.2 Rust 1.84+ con edition 2021

- crates core: `tokio`, `axum`, `tower`, `tower-lsp`, `rusqlite` (bundled), `fastembed`, `reqwest`, `serde`, `tracing`, `keyring`, `clap`, `notify`, `git2`.
- Edition 2021 (cambios adicionales async closures, let-else para ergonomía).
- `#![deny(unsafe blocks unless justified)]` en el workspace `lib.rs`.
- Targets soportados: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`.

### 3.3 SvelteKit 2 (CSR only)

- No SSR en producción (sería absurdo dentro de Tauri). Usamos SvelteKit como SPA: `adapter-static` para build estático.
- Svelte 5 con **runes** (signals reactivos finos) para HUD de cards: updates por campo sin reconciliation DOM global.
- Store niveles:
  - `nano-stores` permanente para usuario-config (theme, perfil, etc).
  - `svelte/store` (writable) por mission snapshot.
  - **Stream store** WebSocket events (`24 §4`) como `Readable<JournalDelta>` con rolling buffer 200 entries.
- CodeMirror 6 para editor (React/Monaco sería demasiado pesado).
- Soporta WASM (`svelte-wasm`).

### 3.4 SQLite + `sqlite-vec`

- DB file: `~/.opencode/journal.db` (multi-perfil: `~/.opencode/profiles/<profile>/journal.db`).
- **WAL mode** (`PRAGMA journal_mode=WAL`) — concurrente read + single writer, ideal para HUD.
- `PRAGMA synchronous=NORMAL` (mejor throughput, safe enough para desktop).
- Schema mínimo: tablas `journal_entries`, `missions`, `subagents`, `plans`, `diffs`, `skills_active`, `approvals`, `audit_log`, `research_runs`, `chats`.
- Vector store: **`sqlite-vec`** (https://github.com/asg017/sqlite-vec) es un C extension a SQLite; necesario `load_extension` asegurado en `rusqlite` via `LoadExtensionGuard`. Garantiza mismo binario SQLite, **cero servicios externos**.
- Tablas vectoriales:
  - `v Emb_Knowledge_Base(id INTEGER PRIMARY KEY, content TEXT, emb vec_embedding(768))`  (CAST vec literal).
- Index ANN `WHERE v_emb MATCH ?`. Metric: default `cosine`.
- Para dimensiones grandes (1536 OpenAI, 3072 Voyage) escalamos vector dimensionado a (768) por PCA en benchmark local o por mixed-Rank reduction server-side. FOLLOW-UP:  para instancias premium se guarda nativo 1536 o 3072. Por default `fastembed-rs` `all-MiniLM-L6-v2` (384), `bge-small-en-v1.5` (384), `bge-base-en-v1.5` (768). Los 384 son default por latencia.

### 3.5 `fastembed-rs` en vez de Python embeddings

- `fastembed` (crates.io, mantenido por Qdrant) carga ONNX models con `ort` (ONNX Runtime bindings Rust).
- Default model: `BgeBaseEnV15` (768-dim, 130MB). Alternativa `AllMiniLML6V2` (384-dim, fast).Se descarga al primer arranque a `~/.opencode/models/embeddings/`, idempotente (hash-check).
- Cero Python. Cero torch. Cero `transformers`. Cero pyinstaller bundling.
- Para usuarios con GPU, fastembed usa CUDA via `ort` `CUDA-DNN` feature flag.
- Si no hay GPU, fallback CPU se mantiene <50ms per embedding on quad-core x86_64.

### 3.6 LSP via Tower-LSP

- LSP server host (Rust).Permite "externalizar los límites por LSP" como intended in `02 §3.3` y `17 §2`.
- LSP servers externos (pyright, gopls, rust-analyzer, lua-language-server, typescript-language-server) corren en **subprocess**; Tower-LSP actúa como proxy/multiplexer. Patterns comunes: ripgrep_bridge_diagnostics, custom OpenCode LSP que añade `codelens: confidence` por símbolo (ver Roadmap `20` Fase 9).
- Los servers se by default auto-descubren en `$PATH` o se downloadable (Tauri sidecar). Para especificos (csharp-ls), integración con DotNet (`csharp-testing` skill options).

### 3.7 Sandbox con Docker / Podman / Firejail / Job-Object

| Plataforma | Modo default | Fallback |
|---|---|---|
| Linux (systemd distros) | **Podman** rootless | Docker; Firejail; `bubblewrap` |
| Linux (no systemd) | Docker | Firejail |
| Windows | **Podman** (WSL2) | Docker Desktop; Job Object (sin container) |
| macOS | Docker Desktop / Podman | sandbox-exec (deprecated); built-in `App Sandbox` |

Política (`18 - Security.md`): cada skill peligrosa se etiqueta `requires_sandbox: true`. El Sandbox Supervisor spawnea subprocesos que aplican:

- Filesystem whitelist (only worktree ruta).
- Network whitelist (por default: **off**).
- CPU/memory cap.

Sandbox **NO** es necesario para skills read-only o que tocan solo whitelist; altamente sensible skills (`/dev/mem`, IP raw socks) → `Forbidden` (`02 §3.5`), nunca sandboxed, no se ejecutan.

### 3.8 Model providers — multi-cerebro sin instalar nada

Patrón Model Orchestrator (`04`) consume estos 10 providers **sin SDK propio** (todos HTTP):

Tier local:
1. **Ollama** http://localhost:11434 (auto-detected via `/api/tags`).
2. **LM Studio** http://localhost:1234/v1 (provider: openai-compatible).
3. **llama-server (llama.cpp)** http://localhost:8080 (provider: openai-compatible).

Tier free cloud (10 free providers como pidió el usuario, ref `21 - Execution Modes.md`):

4. **Nvidia NIM** https://integrate.api.nvidia.com/v1`/chat/completions` (free tier; refacuando para MiniMax M3)
5. **Google AI Studio (Gemini API)** https://generativelanguage.googleapis.com/v1beta/models
6. **OpenRouter Free** https://openrouter.ai/api/v1 (gateways many labels).
7. **GitHub Marketplace Models** https://models.inference.ai.azure.com/chat/completions (GitHub token).
8. **Cerebras** https://api.cerebras.ai/v1/chat/completions (free tier, fast).
9. **Groq** https://api.groq.com/openai/v1/chat/completions (free).
10. **Sambanova** https://api.sambanova.ai/v2/chat/completions (free tier, fast).
11. **Cloudflare Workers AI** https://api.cloudflare.com/client/v4/accounts/{id}/ai/run/ (free tier).
12. **HuggingFace Inference (free tier)** https://api-inference.huggingface.co/models/{model}.
13. **Mistral La Plateforme** https://api.mistral.ai/v1/chat/completions (free tier).

Tier paid (cordoned separate like Tier 3):
14. Anthropic, OpenAI paid, Gemini paid, DeepSeek paid, Cohere Embed, Voyage AI Embed.

Todos estos metricados en `Model Registry` (`04 §4`), cada uno con `capab_tags`, `price_input / output per 1M tokens`, `rate_limit_per_min`, `auth_method`, `required_env_var`.

### 3.9 CLI Rust

Rust CLI **comandos complementarios al modo interactive**:

```bash
$ opencode
Usage: opencode <command>

Core:
  mission   new <prompt>          Lock a Mission from a prompt (RFC 23).
  mission   list                  List active missions.
  plan      <mission_id>          Generate Plan from locked Mission (RFC 12).
  run       <mission_id> [--mode<mode>] [--resource <tier>]
  resume    <mission_id>          Resume from latest checkpoint (RFC 19).
  fork      <mission_id>          Fork an existing session (Cursor pattern).
  steer     <mission_id> "msg"    Inject steer message mid-run.

Swarm:
  swarm     list                  List active subagents.
  swarm     pause <agent_id>
  swarm     stop <agent_id>
  swarm     recover <agent_id>    Trigger Execution Supervisor recovery.

Research:
  research  query <query>        Run Research Engine (RFC 10).
  research  feasibility <topic>  probe_feasibility call.

HUD:
  hud       serve [--port 0]      Serve HUD local web (RFC 24).
  hud       ws                    WebSocket events stream (JSON).

Skill / MCP:
  skill     list [--active-only]
  skill     install <id>          Pull + verify signature + install.
  skill     activate <agent_id> <skill_id>  (drag-drop CLI equivalent)
  mcp       list
  mcp       refresh               Refresh registry (Hermes pattern).

Model:
  model     list                  List registered providers + status.
  model     ping <provider>       Heartbeat.
  model     switch <role> <m_id>  Override Orchestrator selection.

Audit / Journal:
  audit     tail [--agent <id>] [-n K]
  audit     verify-hash-chain    Recompute hashes; flag breaks.

Config:
  profile   new <name>
  profile   switch <name>
  config    get / set / edit
  secrets   set <key>             stored in OS keychain.

Sandbox:
  sandbox   exec <cmd> [...]     Run inside sandboxProfile.
```

- Permite **CLI-only** users (no GUI necessary), útil para CI/CD and headless Linux servers.(`08 - CLI.md`).
- `clap` derive macros, completo `--help` autodoc.
- Output: colorized when on tty; plain JSON when piped (GNOME terminal, Powershell, cmd).

### 3.10 OS keychain (secretos)

Usar `keyring-rs` (https://crates.io/crates/keyring):

- Windows → Credential Manager.
- macOS → Keychain.
- Linux → Secret Service (GNOME Keyring / KWallet).

Nunca escribir claves a `.env`, dotfile plaintext, ni `journal.db`. La entrada en keychain es `(service="OpenCodeOS", account="<provider>")`. Si el usuario prefiere dotenv, se soporta pero se advierte.

### 3.11 Self-update

Tauri Updater (built-in): firma con clave pública del proyecto del manifest. El updater valida con pubkey compilada en el binario (no online). Sirve para:
- Actualizar el core Rust.
- Actualizar el frontend Svelte.
- Actualizar Skills (separately; Skills via `skill install`).

Los manifests se publican en el repo de Atlas OS Releases (GitHub). Se respeta `16(force=false)` por default.

### 3.12 Telemetría local, sin cloud

- `tracing` spans jerárquicos → correlate causal chain: prompt → plan → step → tool_call → diff → validation.
- SQLite sinks instead of Splunk/Datadog.
- Análisis por SQLite queries:
  ```sql
  SELECT agent_id, count(*) AS decisions, avg(confidence) AS conf
    FROM journal_entries
   WHERE event='decision.made'
   GROUP BY agent_id
   ORDER BY conf;
  ```
- Event sinks opcionales: OpenTelemetry exporter (por env var `OTEL_EXPORTER_OTLP_ENDPOINT`).

---

## 4. Perfiles múltiples (Hermes pattern)

Inspirados por Hermes `?profile=` (`22 - Research Findings.md`). Un usuario puede tener varios perfiles:

```
~/.opencode/
  config.toml
  profiles/
    default/
      journal.db
      worktrees/
      models/
      skills/
    work/
      journal.db
      ...
    experimental/
      journal.db
      ...
```

- Switch `opencode profile switch work` → cambia Journal, skills activas, profile worktree.
- HUD profile switcher (top bar, `24 §1`).
- Cada profile puede tener su model resource mode (Tier local / free / mixed).

---

## 5. Sandboxing de MCPs

Patrones MCPs (`07 - MCP.md`) vienen en muchos casos commo binario independiente. Runtime Tauri:

- Cada MCP server se spawnea como subprocess separado.
- Comunicación via stdio JSON-RPC.
- Sandboxing opcional via `sandbox exec --stdio` (`18 - Security.md`).

Multi-MCP: supports **registry streaming**:
```
$ opencode mcp refresh
[info] Refreshing MCP registry...
[ok]   github-mcp    v0.2.0 (signature verified)
[ok]   context7-mcp  v0.1.4
[warn] unmcp         signature missing — booted en modo quarantine
```

Se aplica verificación supply-chain (`06 Skills` + `18`): hash SHA-256 + minisign pubkey.

---

## 6. Multilenguaje para Skills

Skill format (`06`):
- `.md`  o `.yaml` + script.
- Script puede ser: shell, TS, Rust (compiled lib), Python (con bundling standalone via `shiv`), Go.
- Skills se distribuyen via git repo (http/ssh) con `skills.toml` manifest.
- Si Python requisito, seIntegration con `uv`, instalado in `~/.opencode/python/uv/bin` (no user PATH).
- Conflict resolution por skill graph (`16`; Learning Engine).

---

## 7. Builds (multi-OS)

### Windows

- MSI via **Tauri MSI** + WiX Toolset.
- WebView2 runtime="%(`WebView2Bootstrapper.exe% /silent`, bundled in Tauri 2)`. WebView2 viene con Windows 11 por default; en Windows 10 el bundled bootstrapper lo instala.
- Code signing con Azure Trusted Signing (anteriormente Azure Code Signing).
- Target: `x86_64-pc-windows-msvc` (default) y `aarch64-pc-windows-msvc` (ARM64).
- Sidecar sandbox: Podman en WSL2 (instructions in installer).

### Linux

- **`.deb`** (Debian/Ubuntu).
- **`.rpm`** (Fedora/RHEL).
- **AppImage** (single-file). 
- **Flatpak** (opcional, repo comunitario).
- Deps webview: WebKit2GTK-4.1 declarado.
- Podman como dep opcional (sugerido al first-run si disponible).

### macOS

- **`.dmg`** universal (`lipo` x86_64 + aarch64).
- **`Homebrew Cask`** (third-party tap; no depende de App Store).
- **MAC DMG notarized** by Apple Developer ID + notarytool.
- Sandboxing Podman recommended; por default `sandbox-exec` fallback.

### CI/CD Matrix GitHub Actions

```yaml
jobs:
  build:
    strategy:
      matrix:
        runs-on: [ubuntu-latest, windows-latest, macos-latest]
        target: [x86_64, aarch64]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: ${{ matrix.target }}-${{ matrix.os-arch }} }
      - uses: tauri-apps/tauri-action@v0
        with: { args: --target ${{ matrix.target }}-${{ matrix.os-arch }} }
      - uses: softprops/action-gh-release@v2
```

Reproducible builds: optional **Nix flakes** para skill marketplace (`18`).

---

## 8. Tamaños objetivo

| Componente | Size |
|---|---|
| `OpenCodeOS.exe` (Windows installer) | 30-45 MB |
| `atlas` binario Linux | 25-40 MB estátic |
| `OpenCodeOS.dmg` macOS (universal) | 60-80MB (lipo) |
| `journal.db` recién initial | <100 KB |
| `journal.db` tras 1 mes uso intenso | 50-500 MB |
| Vector embeddings (miniLM, 1k chunks) | ~5MB |
| Vector embeddings (bge-base, 100k chunks) | ~400MB |

---

## 9. Performance budgets

| Métrica | Objetivo |
|---|---|
| Cold start desktop | <1s |
| HUD WS push latency (event→render) | <100ms (ver `17 §10`, `24`) |
| Embedding single chunk local CPU | <100ms |
| SQLite query cualquier vista HUD | <50ms |
| Sidecar spawn LSP | <3s |
| Sandbox spawn Docker | <8s (cache hit) |
| Sandbox spawn Firejail / Job Object | <300ms |
| Reasoning Engine step (model local 7B) | 5-30 tokens/s en CPU; 50-150 tok/s en GPU |
| Reasoning Engine step (Cerebras free) | 300+ tokens/s |
| Reasoning Engine step (Anthropic Sonnet) | 60-80 tokens/s |

---

## 10. Decision Tree de instalación

```
Usuario instala OpenCodeOS →
  ¿Tiene GPU CUDA + 12GB+ RAM?   → sugerimos Ollama+fastembed GPU.
  ¿No GPU pero buen CPU?          → sugerimos fastembed CPU + free providers.
  ¿Offline todos?                 → sugerimos Ollama + bge-small (384 dim).
  ¿Modo cloud free?               → registro OpenRouter/Cerebras/NIM/Groq.
  ¿Paid tier?                     → Anthropic/Gemini paid + API key persisted.

Servicios opcionales:
  ¿Tiene Docker / Podman?          → sandbox container activado.
  ¿No tiene container?            → Firejail / Job Object fallback.

¿Atajo para Steering desde móvil?  → Tunnel auto-config (Cloudflare quick tunnel).
```

---

## 11. Justificación vs AionUI (Electron + Python)

AionUI (https://github.com/iOfficeAI/AionUi, Apache-2.0) usa:
- Electron + TypeScript (frontend React).
- Python + Mem-aibackend (model routing + memory).
- Llamaindex/o deal vectoras.
- Local SQLite (igual que nosotros).

¿Por qué no usar AionUI directly?
- Se necesita reemplazar Python por Rust para tener single-binary, cero runtime Python.
- El ruteo manual de AionUI obliga el ** humano a plan de conocimiento avanzado**. Atlas OS lo automatiza en Model Orchestrator (`04`) con seus lessons AionUI.
- El orquestador propio de AionUI no respeta los user tags multi-key, fail-over automatic y capability tagging. OpenCodeOS sí.
- AionUI no tiene HUD Mission Control; nosotros integramos (`24`).
- AionUI no tiene Skills auto-compresoras (`06`); nosotros sí.
- AionUI no tiene anti- doom_loop mecánico (`19`); nosotros sí.

AionUI sirve como referencia sólida de UX de config (perfiles, modelo override manual); tomamos sus aprendizajes (confirmed in `04 § Registery Lessons`).

---

## 12. Limitaciones explícitas

- LSP servers externos no se bundle (espacio); se descargan, NUMA ad-hoc.
- Sandboxing Windows sin Podman en WSL2 no soporta attach-TTY para interactive tools; ciertos skills `pty` caen al category `Forbidden` o Workspaces Windows + Job Object que restringen sin TTY.
- Mobile UI: HUD desde el móvil es read-only + approvals + steer. La edición/in scripting no se soporta en vía móvil.
- Reverse-engineering binary LSPs en casos de vendors cerrados (ej. `vscode-language-server` for Microsoft-cosas) no garantizado.
- No self-hosted marketplace backend por default: un SaaS mantenido por Atlas OS redistribuye skills firmadas; perfil completo tiene local **mirror** opcional.

---

## 13. Outputs hacia otros RFCs

- `02 - Agent Operating System.md` — Kernel corre en el Rust core.
- `05 - Swarm.md` — swarm coordinator corre en Rust core, subagentes spawn via subprocess.
- `07 - MCP.md` — MCP servers spawn via subprocess + stdio.
- `08 - CLI.md` — `atlas` CLI creado vía `clap`, corre en el mismo binario.
- `09 - Vector Knowledge.md` — sqlite-vec + fastembed-rs son implementation concreta.
- `10 - Research Engine.md` — HTTP client `reqwest` para buscar fuentes externas.
- `11 - Context Engine.md` — SQLite + LSP alimentan el contexto real-time.
- `17 - UI.md` — SvelteKit CSR en webview Tauri.
- `18 - Security.md` — sandboxing impl details y keychain.
- `19 - Execution Supervisor.md` — journal writer en Rust persistente.
- `22 - Research Findings.md` — referencias a AionUI stack electron+Python para contrastar.
- `23 - Prompt Understanding & Refinement.md` — Rust core invoca pipeline; estado del verdict persistente en SQLite.
- `24 - HUD Mission Control.md` — Rust core axum serves HUD WS + HTTP.

---

## 14. Estado

- Status: Draft v1
- Depends on: todos los demás RFCs (Rust core es el substrato).
- Cubre explicitamente el pedido del usuario: stack Tauri 2 + Rust + SQLite/vec + LSP + Docker/Podman sandbox + CLI Rust cross-platform.
- Decisión registrada el 2026-07-13. Tomada tras evaluar Electron, Flutter, Qt, CEF. Justificada en §1, §11.
