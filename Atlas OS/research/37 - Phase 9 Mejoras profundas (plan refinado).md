# 37 - Phase 9 Mejoras profundas (plan refinado)

Plan atómico para la Fase 9 del roadmap (RFC 20): razonamiento sobre AST y reglas de seguridad profundas. Complementa RFC 35 §7.1 (Laya/System One) y extiende los patrones ya validados en las Phases 2–8.

## SECTOR A — Decisiones ya tomadas (evidencia RFC 35 §7.1 + RFC 20)

### A.1 Laya como 4º backend del classifier (el "System One" real)
- `laya = "0.1.1"` (`aovestdipaperino/laya-rust`, crates.io, Sep 2026): "Rust inference for the Laya non-autoregressive typed-decision model (ModernBERT-large + RL decision head)". Stack candle — comparable al `ort-sys` de fastembed (ya presente vía feature); `tokenizers` ya en deps.
- RFC 35 §7.1 decide: `ClassifierKind::Laya` como 4º backend del `TaskTypeClassifier` (Lexical/LogReg/Embedding → +Laya), feature-gated `laya` default off.
- Follow-ups del mismo frente: wiring compaction (5.3, weak_model → Laya) y tool-result judging (winnow).

### A.2 findings.json schema + validator (patrón Cloudflare, zero-dep)
- RFC 35 §3: report-schema + validator como la pieza que hace los findings machine-readable.
- EvidenceGate ya emite evidence (Phase 2.5) — el puente natural es `AuditReport::from_evidence`.

### A.3 Crates nuevas a auditar (audit RFC 25 §11 previo obligatorio)
| Crate | Stack transitive a verificar | Uso |
|---|---|---|
| `tree-sitter` + grammars (`tree-sitter-rust`, `tree-sitter-typescript`) | c, cc (build), link | 9.2 AST Context Engine |
| `candle-core` + `candle-nn` + `candle-transformers` | comparable al ort-sys ya presente | 9.1 Laya backend |
| `dependency-cruiser` (Node, dev-dep pnpm) | — | 9.3 límites de capas TS |

Semgrep + CodeQL (RFC 20): proceso EXTERNO lanzado por Atlas OS si están en PATH — jamás bundling (binarios pesados, RFC 25 §11).

### A.4 LSP Confidence por símbolo
- tower-lsp ya en deps (RFC 25, host Phase 2). El Skill Picker relevance (8.1) es la fuente de Confidence — exponerlo por hover/diagnostics.

## SECTOR B — Plan refinado Phase 9 (5 sub-fases atómicas)

### Sub-fase 9.0 — findings.json schema + validator (Cloudflare, sin crates) — M32
- `validation/report.rs`: `AuditReport {findings: Vec<SecurityFinding>}` con `SecurityFinding {id, severity (info/low/medium/high/critical), title, file, line, evidence, remediation}` + `validate_report` (ids únicos, severity enum, evidence no vacía, line > 0) — patrón findings.json Cloudflare.
- Puente: `AuditReport::from_evidence(Vec<Evidence>)` desde el EvidenceGate existente (Phase 2.5).
- CLI: `atlas audit validate <file.json>` + `atlas audit --json` (export del reporte).
- Tests: schema round-trip, fallo en evidence vacía/dup ids/severity inválida, from_evidence.

### Sub-fase 9.1 — Laya classifier backend (4º backend — el "System One" real) — M33 (COMPLETO, audit DIFERIDO)
- Audit candle-core/candle-nn/candle-transformers (deps transitive, MSRV, peso binario vs ort-sys) — RFC 25 §11 previo obligatorio → **FALLA** (RFC 22 §7 AN-9.1: `rand 0.8` vs `0.9`, `tokenizers` no en deps, `axum 0.7` vs `0.8`, weights runtime, 5 días/61 descargas): defer con audit documentado.
- Embarcado: feature `laya` vacío default-off + `ClassifierKind::Laya` + `LayaClassifier` std-only MVP (lexical-delegado determinista, `load` con failure-path → fallback lexical) + M33 (schema 32, CHECK + `'laya'`).
- Tests: inferencia determinista happy-path + failure-path (modelo ausente → fallback al backend activo) ✅. Compaction wiring (5.3) queda como está; winnow como follow-up.

### Sub-fase 9.2 — Tree-sitter AST Context Engine — M34 (COMPLETO, audit APROBADO)
- Audit tree-sitter + grammars (RFC 25 §11, RFC 22 §7 AN-9.2): **PASA** — reutiliza `tree-sitter 0.26` + `tree-sitter-rust 0.24` ya vendoreados por `codebase-graph` (RFC 28 §C, commit `cddcbc2`); cero crates nuevas, cero impacto binario en default.
- Embarcado: feature `ast` default-off (alias de `codebase-graph`) + `context/ast.rs` (`AstSymbol {kind, name, file, line}` + `validate()` + `presence_boost`/`confidence_for_symbol` → Skill Picker 8.1 + LSP 9.4; AST real con el feature, heurístico std-only sin él) + M34 (schema 33, tabla `ast_symbols` + `Journal::record_ast_symbol`/`ast_symbols_for_file`).
- Tests: extracción happy-path (Rust + Svelte) + failure-path (extensión desconocida, nombre vacío, línea 0) en ambos modos; upsert idempotente + CHECK rejects + orden por línea en M34.

### Sub-fase 9.3 — Dependency-cruiser límites de capas — M35
- TS frontend: dependency-cruiser como dev-dep pnpm + `.dependency-cruiser.cjs` (reglas: `lib/` no importa `routes/`, `stores/` no importa `components/`, `components/` no importa `stores/` salvo via stores API) — RFC 20 "límites de capas".
- Rust: check propio sin crate (module layering en `core/pipeline.rs` o defer).
- Tests: reglas violadas detectadas (fixture) + suite limpia.

### Sub-fase 9.4 — LSP Confidence por símbolo — M36 (COMPLETO, std-only MVP sin DB nueva)
- `lsp/confidence.rs` (`SymbolConfidence` + `hover_for_symbol`/`diagnostic_for_symbol` + `who_owns`/`affects_where` sobre filas `ast_symbols` de 9.2): hover/diagnostics exponen `Confidence` por símbolo — fuente: Skill Picker relevance (8.1) como base + `confidence_for_symbol` presence (9.2).
- Tests: hover devuelve confidence determinista, wiring relevance→confidence, failure-path sin datos (símbolo inválido, base no finita, tabla vacía).

**Entregable:** razonamiento sobre AST y reglas de seguridad profundas (RFC 20). KPI: Confidence medio ≥ 0.75, alucinaciones ≤ 1/100 diffs.

## SECTOR C — Fuera de alcance (esta iteración)

- Semgrep/CodeQL embebidos (proceso externo solo, jamás bundling).
- Modelos propios entrenados desde cero (RFC 20 out of scope).
- Marketplace/SDK público (Phase 10).

## SECTOR D — Siguiente paso operativo

1. Commit de este plan + RFC 20 update + Index 26.
2. Gestor audita candle + tree-sitter en paralelo con la delegación 9.0 (9.0 no necesita audit — zero-dep).
3. Sub-fase 9.0 (delegada a muse-spark-1.3) → revisar → commit → push.
4. Sub-fases 9.0 ✅ (M32), 9.1 ✅ (M33), 9.2 ✅ (M34), 9.3 ✅ (M35) y 9.4 ✅ (M36) completas — **Phase 9 COMPLETA**.

---

## SECTOR E — Estado final (cierre Phase 9)

| Sub-fase | Qué quedó | Verificación |
|---|---|---|
| 9.0 (M32) | `validation/report.rs`: `AuditReport`/`SecurityFinding`/`Severity` + `validate_report` + `from_evidence` + `atlas audit validate`/`--json` | 12 tests (993) |
| 9.1 (M33) | `ClassifierKind::Laya` 4º backend std-only MVP (`classifier/laya.rs`, fallback lexical) + gate `laya = []` + M33 schema 32 (audit crate laya DIFERIDO: rand 0.8 vs 0.9, tokenizers 0.21 no en deps, dual runtime vs budget — RFC 22 AN-9.1) | 8 tests (1001) |
| 9.2 (M34) | `context/ast.rs`: `AstSymbol`/`AstSymbolKind` + `extract()` (walk AST real gated `ast`, heurístico std-only default) + `presence_boost` (→ Picker 8.1) + `confidence_for_symbol` (→ LSP 9.4) + Journal M34 schema 33 (tabla `ast_symbols`) | 16 tests feat ast (1015) |
| 9.3 (M35) | `dependency-cruiser 18.4.0` dev-dep + `.dependency-cruiser.cjs` (4 reglas capas, tests exentos) + `pnpm arch` | 0 violaciones (28 módulos, 40 deps) |
| 9.4 (M36) | `lsp/confidence.rs`: `SymbolConfidence`/`ConfidenceSource` + `confidence_for` + hover/diagnostics (fuente: Picker relevance 8.1 base + `confidence_for_symbol` 9.2 presencia) — MVP std-only (multiplexor externo sigue diferido Roadmap Phase 2) | 6 tests (1021) |

**Estado:** Phases 0–9 COMPLETAS. 1021 tests Rust + 67 frontend. Phase 10 (Plataforma abierta: SDK público + marketplace con firma + remixing + learning social) es la última fase del roadmap v1.

**Deviaciones documentadas:** laya crate diferido (9.1), tree-sitter-typescript no añadido — tree-sitter vendored reutilizado (9.2), dependency-cruiser reporterOptions v18 (focusOn eliminado), check `--no-default-features` sin `hud` falla por gating preexistente del módulo hud (no es de 9.x).
