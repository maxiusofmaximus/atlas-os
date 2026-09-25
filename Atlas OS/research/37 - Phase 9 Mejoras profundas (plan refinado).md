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

### Sub-fase 9.1 — Laya classifier backend (4º backend — el "System One" real) — M33
- Audit candle-core/candle-nn/candle-transformers (deps transitive, MSRV, peso binario vs ort-sys) — RFC 25 §11 previo obligatorio.
- Si pasa: feature `laya` default off + `ClassifierKind::Laya` + `LayaClassifier` (pesos safetensors + tokenizers ya en deps → TaskType tipado).
- Si el audit falla: defer con audit documentado (el wiring compaction 5.3 queda como está).
- Tests: inferencia determinista happy-path + failure-path (modelo ausente → fallback al backend activo).

### Sub-fase 9.2 — Tree-sitter AST Context Engine — M34
- Audit tree-sitter + grammars. Si pasa: feature `ast` default off, `context_engine/ast.rs`: parse símbolos (fn/class/struct/enum) → `AstSymbol {kind, name, file, line}` — alimenta Skill Picker (8.1) y LSP (9.4).
- Sin crate viable → defer con audit documentado.

### Sub-fase 9.3 — Dependency-cruiser límites de capas — M35
- TS frontend: dependency-cruiser como dev-dep pnpm + `.dependency-cruiser.cjs` (reglas: `lib/` no importa `routes/`, `stores/` no importa `components/`, `components/` no importa `stores/` salvo via stores API) — RFC 20 "límites de capas".
- Rust: check propio sin crate (module layering en `core/pipeline.rs` o defer).
- Tests: reglas violadas detectadas (fixture) + suite limpia.

### Sub-fase 9.4 — LSP Confidence por símbolo — M36
- `lsp/` host (tower-lsp ya presente): hover/diagnostics exponen `Confidence` por símbolo — fuente: Skill Picker relevance (8.1) + AstSymbol presence (9.2 si existe).
- Tests: hover devuelve confidence determinista, failure-path sin datos.

**Entregable:** razonamiento sobre AST y reglas de seguridad profundas (RFC 20). KPI: Confidence medio ≥ 0.75, alucinaciones ≤ 1/100 diffs.

## SECTOR C — Fuera de alcance (esta iteración)

- Semgrep/CodeQL embebidos (proceso externo solo, jamás bundling).
- Modelos propios entrenados desde cero (RFC 20 out of scope).
- Marketplace/SDK público (Phase 10).

## SECTOR D — Siguiente paso operativo

1. Commit de este plan + RFC 20 update + Index 26.
2. Gestor audita candle + tree-sitter en paralelo con la delegación 9.0 (9.0 no necesita audit — zero-dep).
3. Sub-fase 9.0 (delegada a muse-spark-1.3) → revisar → commit → push.
4. Sub-fases 9.1-9.4 según audits.
