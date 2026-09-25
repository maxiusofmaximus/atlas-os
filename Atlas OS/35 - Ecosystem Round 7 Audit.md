# 35 - Ecosystem Round 7 Audit (Symlink + Cloudflare audit + Context-Mode + Zoxide)

> Audit de la Round 7: las 4 referencias aportadas por el operador (Symlink como conjunto de herramientas, `cloudflare/security-audit-skill` 21.4k★, `mksglu/context-mode` 24k★ — proyecto propio, `ajeetdsouza/zoxide`) + deep search multi-agente (12 hallazgos, 22 URLs verificadas por muse-spark-1.3). Extrae las funcionalidades concretas que Atlas OS puede cerrar y las prioriza. Complementa RFC 30 (Round 6) y RFC 34 (Phase 7).

---

## 1. Contexto y motivación

Con Phases 0–7 completas, la Round 7 investiga el ecosistema de tooling/navegación/security-audit que enriquece el core sin crates nuevas. Método: fetch directo de las 4 referencias + deep search multi-agente (2 agentes opencode en paralelo: muse-spark-1.3-contributor-free para hallazgos nuevos, mimo-v2.6-flash-free como segundo par — degradado a reseña del gestor por fallos de comprensión del prompt). El gestor sintetiza y prioriza.

## 2. Symlink — conjunto de herramientas (no un repo)

El toolset nativo: **mklink** (built-in Windows: `/d` symlink de directorio, `/h` hard link, `/j` Directory Junction — sin instalar nada, Windows 10+), **Junction** (Sysinternals v1.07, 504 KB — traversal/delección recursiva de junctions), **ln -s** (Unix). Fuentes: Microsoft Learn (mklink + Junction).

**Cómo Atlas OS lo usa — 4 rutas:**

| Ruta | Qué es | Estado |
|---|---|---|
| **Skills dir linking** | `~/.opencode/profiles/<id>/skills` → junction/symlink hacia `skills/` del repo — single source of truth, el perfil ve el catálogo sin duplicación | ✗ — `profiles::resolve_root` + `skills/` join ya existen; el linking es un paso de bootstrap (mklink /j en Windows, ln -s en Unix) |
| **Portable inventory** (RFC 28) | La distribución portable usa symlinks para assets compartidos entre perfiles | ⚠️ Parcial — research `28 - portable inventory.md` ya lo documenta |
| **Sandbox npm install** | Binarios nativos (esbuild/lightningcss/rollup) symlinked desde el FS local — patrón del skill `sandbox-npm-install` | ✅ (patrón adoptado en el setup del operador) |
| **Worktrees en Windows** | `git worktree` crea junctions internamente en Windows — el `WorktreeManager` ya lo maneja vía git CLI | ✅ (sub-fase 4.0) |

**Decisión:** el linking de skills dirs via junction (Windows) / symlink (Unix) entra en el bootstrap del perfil — cero herramientas externas (mklink es built-in). Follow-up documentado en el RFC 20 bootstrap del perfil.

## 3. cloudflare/security-audit-skill (21.4k★, MIT)

Skill que convierte el agente en security auditor: **6 fases** (reconnaissance → coverage-led hunting → candidate validation → structured output → independent record verification → target-neutral reporting) + **12 hunting classes** (memory-safety/binary, AI/LLM prompt-injection, web-protocol/auth, client-side/DOM, supply-chain/release, cloud/IAM, RPC/messaging, resource-exhaustion, data-isolation/lifecycle) + `report-schema.json` (schema de los 3 verdicts `findings.json`) + `validate-findings.cjs` (validator zero-dependency). Origen: el harness de vulnerability discovery de Cloudflare ("Build your own vulnerability discovery harness").

**Cómo Atlas OS lo usa:**

| Patrón | Destino |
|---|---|
| 6 fases de audit con independently verified findings | **Validation Engine**: el workflow de security audit (EvidenceGate + security_scan ya existen) se estructura en fases con findings machine-readable — el patrón `audit-integrity` del operador (independently verified, 1-10 scoring) ya lo refleja |
| Hunting classes por superficie (12 archivos) | **Compliance skills enrichment**: las 3 skills `atlas-*-check` (7.2) se extienden con hunting classes por superficie (AI/LLM, supply-chain, cloud) — skill bundles adicionales |
| `report-schema.json` + validator zero-dependency | **RFC 18 §8 Auditoría**: el formato machine-readable de findings (JSON schema + validador) para el AuditLog export |
| Sandbox OS-enforced para target builds | RFC 34 §C (container exec diferido) — valida la decisión |

**Decisión:** adoptar el formato machine-readable de findings (schema + validator) como RFC de Phase 9 (Mejoras profundas) — el Validation ya tiene los stages; el schema formaliza el output. Las hunting classes enriquecen las compliance skills como bundled adicionales (prioridad media).

## 4. mksglu/context-mode (24k★, MIT — proyecto propio del operador)

Context window optimization para agentes: **sandbox de tool output** (98% reducción — indexación FTS5/BM25, los bytes crudos no entran en el contexto; solo lo derivado), **session memory persistente** (26 event categories auto-captured: decisions, errors, blockers, plans, user prompts...), **routing enforcement** (MCP + hooks, 17 plataformas).

**Overlap con el core de Atlas OS (ya existe):**

| Patrón context-mode | Atlas OS equivalente |
|---|---|
| Session memory persistente (26 categories) | Journal SQLite (M0-M30, append-only) + compaction 5.3 |
| Tool output sandbox | Kernel Bus broadcast + HUD tail (trunca) + EvidenceGate (deriva evidencia) |
| Routing MCP + hooks | Kernel Bus event kinds + supervisor actions |
| Compaction de contexto | 5.3 System One compaction (threshold + rolling summary) |

**Piezas NUEVAS que aporta:**

1. **FTS5 full-text search sobre `journal_events`** — el Journal es SQLite (rusqlite bundled incluye FTS5); un virtual table FTS5 sobre los payloads + `Journal::search_events(query)` enriquece el Journal Observer (6.1) y el CLI `atlas journal --query`. El patrón del propio proyecto del operador (FTS5 + BM25 + trigram) es la evidencia primaria.
2. **Context budget enforcement en el Kernel Bus** — el HUD tail trunca payloads (120 chars en +page.svelte); un `ContextBudget` por event kind (cap de payload + indexing del overflow) reduce el ancho de banda WS sin perder información (el overflow va a SQLite, el UI consulta por demanda — patrón sandbox).
3. **26 event categories** — el `BusEventKind` del Kernel Bus se puede enriquecer con categorías auto-captured (decision, error, blocker, plan) que el compaction 5.3 usa para resumir por categoría (ya lo hace: `agent_diff:2, task_received:1`).

**Decisión:** FTS5 search + ContextBudget como RFC de Phase 9 (mejoras profundas del Context Engine/Journal) — sin crates nuevas (rusqlite bundled FTS5), evidencia primaria del proyecto propio del operador.

## 5. ajeetdsouza/zoxide (0.10.0, MIT, crates.io)

"Smarter cd" — **frecency** (aging + ranking: recuerda los dirs más usados), storage en database file, Rust. **22 deps** — como LIB arrastra mucho; como **CLI externo** (patrón `find_pandoc`/`find_ctx7max` del core) es la ruta correcta.

**Cómo Atlas OS lo usa:**

| Ruta | Qué es |
|---|---|
| **Navegación frecency a worktrees** | El Swarm spawn de worktrees registra el dir; `atlas swarm jump` consulta zoxide (`zoxide query <pattern>`) para saltar al worktree/mission más frecuente — CLI externo, fail-safe si zoxide no está |
| **Frecency interno propio** | Implementar el algoritmo frecency (aging + ranking) en `Journal` sobre missions/worktrees accedidas — sin crate: `journal::frecency(query) -> Vec<...>` con decay exponencial (RFC 20 Phase 8 Skill Picker usa el mismo patrón de relevancia) |
| **zoxide como crate** | RECHAZADO — 22 deps violan el minimal-dep stance (RFC 25 §11) |

**Decisión:** frecency interno propio (algoritmo port del de zoxide: aging + ranking, determinista) para la navegación de missions/worktrees — RFC de Phase 8 (Skill Picker/navegación). zoxide CLI como conveniencia opt-in del operador, no dependencia.

## 6. Deep search multi-agente — 12 hallazgos (22 URLs verificadas)

Hallazgos de muse-spark-1.3-contributor-free (búsqueda exhaustiva, priorizados por return/effort + riesgo single-binary):

| # | Hallazgo | Effort | Destino |
|---|---|---|---|
| 1 | **`ratatui`** (Rust, MIT) — Sister IDE-in-a-terminal | S | Phase 8 (Sister TUI del roadmap — `src/cli-tui/`) |
| 2 | **`tower-lsp-server` fork** (puro Rust) | S | Phase 9 (LSP host bump) |
| 3 | **`ast-grep`** (Rust) — structural search | M | Phase 9 (Context Engine AST search) |
| 4 | **`axum-oidc-layer`** (puro Rust) — remote auth | M | Phase 8 (Command Center web remoto SSO/OIDC) |
| 5 | **`sysinfo` + `nvml-wrapper`** (Rust) — VRAM/RAM monitor | S | Phase 8 (VRAM/RAM/cost monitor del roadmap) |
| 6 | **`agentskills.io` spec + skills-ref** — SDK público | S | Phase 10 (SDK para Skills) |
| 7 | **`herdr` patrones** — socket/detección PTY | M | Phase 8 (Swarm TUI) |
| 8 | **`semgrep` Guardian / CodeQL** (externos opt-in) | M | Phase 9 (stages profundos sin bundlear) |
| 9 | **`dependency-cruiser`** (frontend + port petgraph Rust) | M | Phase 9 (límites de capas) |
| 10 | **`RustDesk`** (AGPL) — remote dual-PC | L | Phase 8 (integración lateral, jamás link directo — AGPL prohibe bundling) |
| 11 | **`tauri-plugin-axum`** — evaluar vs WS actual | S | Phase 8 (evaluación) |
| 12 | **Marketplaces skills** (VoltAgent/matt pocock/taste — ya en RFC 30) | S | Phase 10 (curar seed MIT) |
| 13 | **`laya = "0.1.1"`** (`aovestdipaperino/laya-rust`, crates.io, Sep 2026 — **la versión open source de Jev**) — "Rust inference for the Laya non-autoregressive typed-decision model (ModernBERT-large + RL decision head)". Stack candle (comparable al ort-sys de fastembed ya presente). Ver §7.1 | M | Phase 9 (ClassifierKind::Laya backend — el "System One" judgments real) |

## 7. Priorización por dependencia

```
FTS5 search (context-mode) ─┐
Frecency interno (zoxide) ──┼─► Phase 8 (Skill Picker + navegación + monitor)
axum-oidc-layer ────────────┤
sysinfo + nvml-wrapper ─────┤
ratatui (Sister TUI) ───────┘

Findings schema (Cloudflare) ─┐
Hunting classes ──────────────┼─► Phase 9 (Mejoras profunas + compliance enrichment)
ast-grep / semgrep / CodeQL ──┤
dependency-cruiser ───────────┘
laya (System One real) ───────┘

agentskills.io SDK ──► Phase 10 (plataforma abierta)
RustDesk (AGPL) ─────► Phase 8 lateral — jamás bundling (AGPL viola MIT distribution)
```

**Orden recomendado (por return/effort):**
1. **Frecency interno propio** (S): navegación missions/worktrees — port del algoritmo zoxide.
2. **FTS5 search** (S-M): `journal_events` full-text — patrón context-mode, sin crates.
3. **sysinfo + nvml-wrapper** (S): VRAM/RAM monitor para el HUD Phase 8.
4. **ratatui Sister TUI** (S-M): Phase 8.
5. **axum-oidc-layer** (M): remote auth Phase 8.
6. **Findings schema + validator** (M): Phase 9 (patrón Cloudflare).
7. **Hunting classes enrichment** (M): compliance skills adicionales.
8. **ast-grep/semgrep/CodeQL** (M): Phase 9 stages profundos.
9. **Laya classifier backend** (M): Phase 9 — el "System One" judgments real (§7.1).

### 7.1 Laya — la versión open source de Jev (el panorama "System One" completo)

`laya = "0.1.1"` (`github.com/aovestdipaperino/laya-rust`, crates.io, Sep 2026, creado hace días): *"Rust inference for the Laya non-autoregressive typed-decision model (ModernBERT-large + RL decision head)"*. Deps: anyhow, candle-core, candle-nn, candle-transformers, clap, serde, serde_json, tokenizers (+ axum/tokio opt). **Stack candle** — comparable en peso al `ort-sys` de fastembed (ya presente vía feature); `tokenizers` ya está en deps.

**El panorama "System One" queda completo — cómo se unen todas las piezas (Round 6 + Round 7 + fases):**

| Pieza | Qué hace | Dónde vive en Atlas OS |
|---|---|---|
| **Laya/Jev (System One)** | Modelo pequeño no-autoregressive: decisiones tipadas rápidas (routing, eval, judging, compaction) | **Phase 2.3** `TaskTypeClassifier` (Lexical/LogReg/Embedding → **Laya como 4º backend** `ClassifierKind::Laya` feature-gated, Phase 9) + **5.3** compaction (wiring weak_model → Laya) + **winnow pattern** (tool-result judging, Phase 9) |
| **HydraFusion (adaptativa)** | Elige el workflow menos complejo esperado | **Phase 2.1-2.4** routing policy + aggregation + Mf + affinity ✅ |
| **context-mode (sandbox + memory)** | Tool output sandboxed (FTS5/BM25) + session memory | **8.0** FTS5 sobre journal_events ✅ + **5.3** compaction ✅ + Kernel Bus event categories |
| **zoxide (frecency)** | Navegación frecency | **8.0** frecency interno propio ✅ |
| **Cloudflare audit (findings)** | Findings machine-readable + validator | **Phase 9** report-schema + validator (EvidenceGate ya emite evidence) |
| **Evidence-gated done (Canny)** | Hooks deterministas antes de "done" | **Phase 2.5** EvidenceGate + DoneClaimed ✅ |
| **MCP/CLIs/Skills (extensibilidad)** | Everything-is-a-plugin | RFC 06/07 + bundled catalog (D) ✅ + deepseek-harness pattern (Phase 5+) |

**Decisión:** `ClassifierKind::Laya` (feature `laya` default off, stack candle — audit RFC 25 §11 previo obligatorio: candle-core/candle-nn/candle-transformers ~comparable al ort-sys ya presente) como Phase 9 — el 4º backend del classifier que materializa el "System One" con un modelo REAL entrenado. El wiring compaction (5.3) y tool-result judging (winnow) quedan anotados como follow-ups del mismo frente.

---

## 8. Status de este RFC

- **Versión:** 1.0 (audit completo, Sep 2026).
- **Tipo:** Informativo + priorización. No introduce APIs ni crates nuevas inmediatamente.
- **Método:** fetch directo de las 4 referencias + deep search multi-agente (muse-spark-1.3: 22 URLs; mimo-v2.6-flash-free: degradado a reseña del gestor — 2 fallos de comprensión del prompt, lección: los prompts de delegación a modelos free pequeños deben llevar las referencias inline cortas, no en archivos largos).
- **Cierre de frentes:** cada frente se cierra con su propio sub-RFC o PR. Quando los frentes de Phase 8/9 estén cerrados, este RFC pasa a **implemented**.

---

## 9. Fuentes de auditoría

- Symlink: `https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/mklink` + `https://learn.microsoft.com/en-us/sysinternals/downloads/junction`
- cloudflare/security-audit-skill: `https://github.com/cloudflare/security-audit-skill` + README raw
- mksglu/context-mode: `https://github.com/mksglu/context-mode` + README raw + context-mode.com
- ajeetdsouza/zoxide: `https://github.com/ajeetdsouza/zoxide` + `https://crates.io/crates/zoxide` + crates.io API
- Deep search: 22 URLs verificadas por muse-spark-1.3 (ratatui, ast-grep, axum-oidc, sysinfo, nvml-wrapper, RustDesk, semgrep, CodeQL, dependency-cruiser, herdr, agentskills.io, tauri-plugin-axum, marketplaces)
