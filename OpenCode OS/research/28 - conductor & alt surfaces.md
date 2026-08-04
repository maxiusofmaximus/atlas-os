# Conductor Analysis + Alt Surfaces (Terminal-UI & Remote-Live)

**Fecha:** 2026-08-04 · **Orquestador:** opencode (z-ai/glm-5.2)
**Source:** conductor.build (público), 4 repos terminal-UI MIT (InquirerPy, Inquirer.js, rich, terminal-kit)
**Propósito:** Documentar patrones portables de Conductor (parallel-agent IDE), comparar 4 libs terminal-UI para una future sister IDE-in-a-terminal, y definir el roadmap **remote-live dual-PC** (modelo Nate Gentile aplicado a coding/LLM, no captura estática). Sectores A/B/C alimentan Phase 2-5; Sector D alimentar Phase 5/v2.

---

## SECTOR A — Conductor (conductor.build)

### A.1 Qué es

Conductor es una **Mac app nativa** (Swift/AppKit, no Electron) que orquesta múltiples agentes de coding (Claude Code, Codex CLI, Cursor, OpenCode) en paralelo sobre un mismo repositorio. Lanzada 2025 por Mitchell Holman. OpenCode support added in v0.69.0 (Jun 2026). Modelo freemium: Free $0 (local-only), Pro $50/mo (cloud workspaces + API + multiplayer), Teams $60/mo/user (SSO + admin), Enterprise (custom).

**Tesis central:** "un agente secuencial es lento; N agentes paralelos sobre worktrees aislados es rápido **si** el UI te deja verlos todos a la vez y mergear sin fricción."

### A.2 Unidades arquitectónicas

| Unidad | Definición | Análogo OpenCode OS |
|---|---|---|
| **Project** | 1 repository Git. Root de namespaces. | Workspace dir + `journal.sqlite` |
| **Workspace** | 1 branch + 1 git worktree + 1 agent sandbox. Shippable unit. | `Mission` + Swarm `subagent_id` (RFC 05) |
| **Lives board** | Grid UI de workspace cards (status, last diff, checks). | `<Kanban>` view en HUD Mission Control (RFC 24 §3.1) |
| **Diff viewer** | Per-workspace diff pane with checks status. | `<Canvas>` view (RFC 24 §3.2) |
| **`.context/`** | Folder per-workspace: handoff notes, scratchpad, agent intent. | `mission.context` blob en `journal.missions` |

**Layout filesystem real:**
```
~/conductor/workspaces/<repo>/<workspace>/
  ├── .git/                  # worktree git metadata
  ├── <repo files...>
  └── .context/
      ├── NOTES.md           # human handoff
      ├── AGENT.md           # agent intent log
      └── review-path.md     # suggested review order
```

### A.3 Workflow patrón (portable)

1. **Break** el problema en shippable units (1 unit → 1 workspace).
2. **Spawn** 1 agent por workspace (CLI: `claude`, `codex`, `opencode`, `cursor`).
3. **Watch** the Lives board (paralelo, at-a-glance status).
4. **Review** diff per workspace cuando el agent signal done.
5. **Run checks** (build/test/lint) from Conductor UI button.
6. **Merge** → workspace archiva, branch se elimina.
7. **Re-sync**: workspaces restantes hacen rebase automático sobre el main actualizado.

### A.4 Patrones portables a OpenCode OS

| ID | Patrón | Origen Conductor | Destino OC-OS | Phase |
|---|---|---|---|---|
| **CN-001** | Worktree por subagent con `.context/` folder | `~/conductor/workspaces/` | `src-tauri/src/swarm/worktree.rs` (RFC 05) + `mission.context` field | 4 (Swarm) |
| **CN-002** | Lives board (grid de workspace cards con status + diff preview) | Swift AppKit `NSCollectionView` | `<Kanban>` Svelte 5 establecido en RFC 24 §3.1 — **ya existe** en spec | 1 ✅ |
| **CN-003** | Rebase automático post-merge en workspaces vivos | Conductor `git pull --rebase` trigger | `swarm::rebase_after_merge()` on `mission.completed` event | 4 |
| **CN-004** | Checks button (build/test/lint) por workspace, resultado inline | Conductor Checks panel | `Validation Engine` (RFC 14) ya tiene stages; exponer como HUD action button | 2 |
| **CN-005** | Multiplayer (v0.77.0): multi-user edit same workspace con CRDT | Conductor Cloud | Out-of-scope per Roadmap ("IDE distribuido multi-usuario simultâneo"). Re-evaluar v2. | v2 |
| **CN-006** | Cloud workspaces (v0.78.0): worktree remoto en VM | Conductor Cloud | `opencode remote attach` command + SSH backend (ver Sector D) | 5 |
| **CN-007** | API (v0.77.0): crear/listar/steer workspaces via REST | Conductor Cloud API | axum HUD server ya corre en `127.0.0.1:0`; exponer `/api/v1/workspaces` | 2 |
| **CN-008** | Agent-mode switch (Ask/Architect/Code/Context) per workspace | Conductor mode dropdown | RFC 21 §12 define estos 4 modos — **ya existe** en spec; HUD selector pendiente | 2 |

### A.5 No-portable (rechazos conscientes)

- **CRDT real-time co-editing** (CN-005): rechazado para v1 (Roadmap "out of scope"). Re-evaluar cuando Yjs/Automerge en Rust madure (`y-rs` inestable 2026-Q3).
- **Cursor agent integration**: Conductor soporta Cursor como agent host. OpenCode OS **es** el agent host — no hay nada que integrar.
- **Mac-only native**: Conductor es Swift/AppKit. Nosotros somos Tauri (cross-platform). El UX pattern (Lives board) es portable; la implementación no.

---

## SECTOR B — Terminal-UI Libraries (4 candidates)

**Caso de uso:** una sister **IDE-in-a-terminal** alongside the WebView HUD. Permite: (a) full TUI para power users sobre SSH, (b) misma sesión sincronizada con desktop app, (c) base para mobile SSH client (Sector C). Criterion: debe ser **full-screen TUI** (alternate-screen, mouse, focus) — NO secuencial prompt-only.

### B.1 Comparativa

| Lib | Lang | Stars | Last push | Full TUI | Prompts | Async | Styling | Deps | License |
|---|---|---|---|---|---|---|---|---|---|
| **InquirerPy** | Python | ~1.1k | Aug 2024 (stale) | ❌ | 18+ | ✅ `execute_async` | CSS-like token hex | `prompt_toolkit 3.x` | MIT |
| **Inquirer.js** | Node | ~21.6k | active (ESM v8.5) | ❌ | 10 | ❌ sync core | ANSI `theme.style.*` | 0 (ESM) | MIT |
| **rich** | Python | ~57k | active | ❌ (foundation for Textual) | ❌ | ✅ `Live` | hex + named colors | 2 (`markdown-it-py`, `pygments`) | MIT |
| **terminal-kit** | Node | ~3.4k | v3.1.4 active | ✅ **Document Model** | ✅ widgets | ✅ Promise variants | full RGB `ScreenBufferHD` | 8 runtime | MIT |

### B.2 Análisis por criterion

**InquirerPy** — Port de Inquirer.js sobre `prompt_toolkit`. 18+ prompt types (input/list/checkbox/password/fuzzy/path/expand). CSS-like styling. **Maintenance mode** (last push Aug 2024). Sequential prompts only, NOT full-screen TUI. Foundation for nothing. **Reject** como base TUI.

**Inquirer.js** — 21.6k stars, 28M weekly. ESM-first `@inquirer/prompts` v8.5. React-hooks core (`useState`/`useKeypress`/`useEffect`). 10 prompt types. `AbortSignal` cancel. **Incompatible con `ink`/`blessed`** (stdin ownership conflict). Sequential only. **Reject** como base TUI.

**rich** — 57k stars, Python ≥3.9, 2 deps only. Tables, panels, layouts, `Live` updates, progress, trees, syntax highlighting, markdown, JSON, tracebacks. `Layout` for multi-panel split-screen. Foundation for **Textual** (full-screen TUI framework, 30k stars). Cross-platform incl. legacy `cmd.exe`. **Could** use rich for non-TUI dashboards ( SQLAlchemy logs, audit dump pretty-print). **Reject** como base sister-IDE TUI (sequential output, not interactive grid).

**terminal-kit** — 3.4k stars, Node ≥16.13, 8 runtime deps. **Full TUI**: menus, inputs, file picker, buttons, sliders, drop-downs, `ScreenBuffer`/`ScreenBufferHD` (32-bit RGBA), `TextBuffer`, image loading (PNG/JPEG/GIF), mouse+keyboard+focus events, fullscreen alternate-screen, **Document Model** for rich app GUIs with widgets + layout containers. Promise-returning variants. Actively maintained (177k weekly). **Best candidate for sister IDE-in-a-terminal.**

### B.3 Recomendación

**Tier 1 (base sister-IDE TUI):** **terminal-kit** — único candidate con `Document Model` full-screen, widgets, mouse, image support, RGBA buffers. Node ≥16.13 compatible con nuestro toolchain (`pnpm` workspace).

**Tier 2 (aux pretty-print):** **rich** (Python only — consumo vía `subprocess` para dump formateado de audit/journal desde CLI `opencode audit --pretty`). No como base TUI.

**Tier 3 (rechazados):** InquirerPy (stale), Inquirer.js (sequential only, stdin conflict).

### B.4 Puente Rust ↔ Node (terminal-kit)

OpenCode OS core es Rust. terminal-kit es Node. Patrón propuesto:

```
┌─────────────────────────────────────────────────┐
│  src-tauri (Rust core)                           │
│    ├── axum HUD server (127.0.0.1:0)             │
│    └── WebSocket Kernel Bus                      │
└───────────┬─────────────────────────────────────┘
            │ ws://127.0.0.1:<port>/bus  (already exists)
            │
┌───────────▼─────────────────────────────────────┐
│  src/cli-tui/  (Node + terminal-kit)             │
│    ├── index.ts         # Document Model shell   │
│    ├── panels/          # Kanban, Canvas, Audit  │
│    ├── bus-client.ts    # ws client → Svelte-like │
│    └── package.json     # "terminal-kit": "^3.1" │
└─────────────────────────────────────────────────┘
```

- **Bus reuse**: el WebSocket ya existe (`src-tauri/src/hud/`). El TUI Node se conecta al mismo endpoint que el SvelteKit HUD — recibe los mismos `BusEvent` JSON.
- **State sync**: si desktop y TUI corren concurrently, ambos reciben el mismo stream. Acciones (approve/fork/steer) mandan commands al mismo endpoint axum. **No hay state en el cliente** — el kernel es source of truth.
- **Single binary intact**: `src/cli-tui/` es un sub-paquete `pnpm` bundleado en el postinstall. No rompe RFC 25 §11 (no new external tool bundled — es JS dentro del mismo workspace `pnpm`).

### B.5 Obsolescencia / riesgo

- terminal-kit 8 runtime deps — auditar quarterly por supply-chain (per AGENTS.md security rules).
- `prompt_toolkit` (InquirerPy dep) stagnante — confirmed rejection.
- `ink` (React-in-terminal, ~12k stars) considered, rejected: stdin ownership conflict con Inquirer.js, y React reconciler en terminal es overhead para nuestro caso.

---

## SECTOR C — Remote-Live Dual-PC (Nate Gentile model)

### C.1 El modelo Nate Gentile

Nate Gentile (YouTuber hardware/PC) documented un setup **dual-PC**:
- **PC servidor** (headless): corre los juegos, GPU potente, recursos gráficos.
- **PC cliente** (thin): donde él juega directamente, accediendo al servidor en vivo.
- **NO es captura de pantalla** — es **live remote desktop**: input/teclado/ratón del cliente se inyecta en el servidor; el framebuffer del servidor se stream al cliente en tiempo real (<20ms latencia perceptual). Protocolos: Moonlight/Sunshine ( Sunshine = open-source host, Moonlight = open-source client), Parsec, Steam Remote Play.

**Análogo OpenCode OS:**
- **PC servidor**: corre OpenCode OS con modelos locales (Ollama) o API keys, HUD, journal, engines, ACP server, toast, calendar. Potente CPU/GPU, headless OK.
- **PC cliente / móvil**: accede en **live** al servidor para controlar programación y modelos. No descarga screenshots — render el HUD en vivo y inyecta clicks/teclado.

### C.2 Por qué NO captura estática (aclaración）

Early discussionmightoría sugería "captura de pantalla + click injection" — **esto es incorrecto**. El modelo Nate Gentile es **stream video en vivo** ( framebuffer 60fps comprimido H.264/H.265) + input forwarding. La diferencia operacional:

| Dim | Captura estática (rechazada) | Live remote desktop (adoptar) |
|---|---|---|
| Latencia | 200-2000ms (snap, upload, display) | <20ms (video stream + input forward) |
| Frame rate | 1-5 fps | 60 fps |
| Bandwidth | Alto (PNG por frame) | Bajo (H.265 ~5Mbps) |
| UX | Laggy, no apto para typing | Fluído, apto para programar |
| Protocolo | HTTP + S3 | WebRTC / Sunshine / VNC |

### C.3 Arquitectura propuesta (OpenCode OS Live Remote)

```
┌──────────────────────────────────────────────────────────────────┐
│  PC SERVIDOR (headless OK, GPU potente)                          │
│                                                                  │
│  ┌────────────────────┐   ┌──────────────────────────────────┐  │
│  │ OpenCode OS Rust   │   │  Sunshine host (open-source)     │  │
│  │  core + axum HUD   │◄──┤  - Captura framebuffer Tauri     │  │
│  │  + ACP + Ollama    │   │  - H.265 encode (NVENC/QSV)      │  │
│  └─────────┬──────────┘   │  - WebRTC signaling              │  │
│            │ WebSocket     └──────────────┬───────────────────┘  │
│            │ :<port>/bus                  │                      │
│  ┌─────────▼──────────┐                   │                      │
│  │ Tauri WebView      │◄──────────────────┘                      │
│  │ (HUD render)       │  (framebuffer source)                    │
│  └────────────────────┘                                           │
└──────────────────────────────────────────────────────────────────┘
                               │ Internet (TCP 47984-47990)
                               │ Sunshine default ports
┌──────────────────────────────────────────────────────────────────┐
│  PC CLIENTE / MÓVIL                                              │
│                                                                  │
│  ┌────────────────────┐   ┌──────────────────────────────────┐  │
│  │ Moonlight client   │   │  (opcional) terminal-kit TUI     │  │
│  │  - Video decode    │   │  over SSH for power users        │  │
│  │  - Input forward   │   │  - Bus WebSocket client          │  │
│  │    (kbd/mouse/touch)│  │  - Document Model panels         │  │
│  └────────────────────┘   └──────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

### C.4 Componentes

| Componente | Rol | Stack | Status |
|---|---|---|---|
| **Sunshine host** | Capture Tauri framebuffer, H.265 encode, WebRTC server | Rust crate `sunshine` (fork) o wrapper sobre [LizardByte/Sunshine](https://github.com/LizardByte/Sunshine) (GPL-3.0 ⚠️ → verificar compat) | **Pending license audit** — GPL-3.0 conflictúa con MIT/Apache. Evaluar alternative: **rustDesk** (Apache-2.0 ✅) o implementación WebRTC nativa con `wgpu` capture. |
| **Tauri WebView** | Render HUD (ya existe) — fuente de framebuffer | Svelte 5 runes (present) | ✅ |
| **Moonlight client** | Video decode + input forward en cliente | Moonlight is open-source GPL — clientes existen para Android, iOS, Windows, Mac, Linux | **Reusa cliente existente** — no desarrollamos client |
| **terminal-kit TUI over SSH** | Power-user alt surface, mobile vía SSH app (Termux/JuiceSSH) | Node + terminal-kit (see B.4) | Phase 5 |
| **Input injection ↦ Kernel Bus** | Input del cliente → evento `hud.input` → Router → action | axum handler `POST /hud/input` → `BusEvent::UserInput` | Phase 5 |

### C.5 Input forwarding (detalle)

Cliente (Moonlight) envía input events (kbd/mouse/touch) con timestamps. Sunshine host los recibe y los inyecta en:

1. **Window manager** (para control nativo de la Tauri window: focus, resize, move) — vía Win32 `SendInput` en Windows, XTest en Linux, CGEvent en macOS.
2. **Kernel Bus** (para actions semánticas: approve, fork, steer). Input forward llega a Tauri WebView como eventos DOM nativos → Svelte handlers → axum POST. **No requiere pathway nuevo**.

### C.6 Latencia budget

| Segment | Target | Mechanism |
|---|---|---|
| Capture → encode | <5ms | NVENC hardware encode (NVIDIA) o QSV (Intel) o AMF (AMD) |
| Network (LAN) | <2ms | Gigabit Ethernet direct |
| Network (WAN) | <40ms | UDP WebRTC, jitter buffer adaptivo |
| Decode → display | <8ms | Hardware decode (cliente) |
| Input round-trip | <50ms | UDP input, sin TCP head-of-line blocking |
| **Total percebido** | **<60ms** | Aceptable para programación (60% del budget es <40ms para typing fluído) |

### C.7 Alternativas consideradas

| Solución | Licencia | Latencia | Veredicto |
|---|---|---|---|
| **Sunshine + Moonlight** | GPL-3.0 (host) / GPL (client) | <20ms LAN | ⚠️ GPL conflict — evaluar fork Apache o alternative |
| **RustDesk** | Apache-2.0 ✅ | <30ms LAN | ✅ **Preferido** — license-compatible, Rust native, active dev |
| **Parsec** | Propietario (gratuito) | <16ms | ❌ No portable, no self-host |
| **Steam Remote Play** | Propietario | variable | ❌ No self-host |
| **VNC** (TigerVNC/TightVNC) | GPL/BSD mix | 100-300ms | ❌ Latencia demasiado alta para programación |
| **WebRTC nativo** (custom) | MIT (libwebrtc) | <20ms | ⚠️ Build complexity alta — solo si RustDesk no encaja |

### C.8 Recomendación Phase 5

**Adoptar RustDesk** (Apache-2.0, Rust native):
- Self-hostable sin vendor lock-in.
- Software renderer + hardware encoder (NVENC/AMF/QSV) available.
- Clientes nativos para Android, iOS, Windows, Mac, Linux ya existen.
-hbrust-sdk crate permite embed host en OpenCode OS Rust core (no subprocess).
- Encryption end-to-end, flea relay opcional.

Pattern: cuando una mission requiere GPU heavy (Ollama 70B, Fine-tuning), el usuario arranca el servidor en el PC potent, conecta desde laptop/móvil con RustDesk client, y opera el HUD en vivo. El Journal persiste en el servidor. Sync entre clientes = el kernel es source of truth.

---

## SECTOR D — Roadmap integración

### D.1 Phasing propuesto

| Phase | Sector Alimentando | Entregable |
|---|---|---|
| **Phase 4 (Swarm)** | A.4 (CN-001, CN-003, CN-004) | Worktree por subagent + rebase auto + checks button in HUD |
| **Phase 5 (Alt Surfaces)** | B.4 + C.4 | `src/cli-tui/` Node + terminal-kit + RustDesk host embed |
| **Phase 6 (Remote-Live)** | C.5, C.6 | Input injection ↦ Kernel Bus + latencia budget validar |
| **v2** | A.4 (CN-005, CN-006) | Multiplayer CRDT + Cloud workspaces (re-evaluar) |

### D.2 Dependencias RFC

- **Sector A**alimenta RFC 05 (Swararm), RFC 24 (HUD Kanban), RFC 21 (modes).
- **Sector B**alimenta RFC 08 (CLI), RFC 25 (stack — Node sub-paquete), RFC 24 (HUD alt surface).
- **Sector C**alimenta RFC 17 (UI remote access ya mentionado), RFC 25 §11 (single-binary — RustDesk embed o subprocess), RFC 18 (security — input injection trust boundary).

### D.3 No-action items

- **No añadir Rust binding a terminal-kit** — sería port mínimo; mejor reusar Node via `pnpm` sub-paquete.
- **No añadir GPL-3.0 deps** (Sunshine). Si RustDesk no encaja, implementar WebRTC nativo.
- **No romper single-binary** (RFC 25 §11). RustDesk se bundlea como feature-gated `remote-live` default-off. terminal-kit TUI es JS dentro del workspace existente.

---

## SECTOR E — Atribución

| Source | Licencia | Uso |
|---|---|---|
| `conductor.build` docs/changelog/pricing | Público (citar URL) | Análisis arquitectónico (Sector A) |
| `inquirerpy/InquirerPy` | MIT | Comparativa (Sector B) |
| `SBoudrias/Inquirer.js` | MIT | Comparativa (Sector B) |
| `Textualize/rich` | MIT | Comparativa (Sector B) |
| `cronvel/terminal-kit` | MIT | Comparativa + adopción Tier 1 (Sector B) — Context7 ID `/cronvel/terminal-kit`, 1533 snippets, High reputation, score 78.86 |
| `LizardByte/Sunshine` | GPL-3.0 | **Rechazado** (Sector C.4) |
| `rustdesk/rustdesk` | Apache-2.0 | Adoptar (Sector C.8) |
| Nate Gentile dual-PC video | Público (canal YouTube) | Conceptual model (Sector C.1) |

**Próximo paso:** update RFC 20 §Roadmap con referencia a este doc en sección Phase 5; update RFC 26 catálogo con entrada research.
