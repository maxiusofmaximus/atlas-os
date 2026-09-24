# 30 — Phase 3 Research Engine (plan refinado)

**Propósito:** Refinar la Fase 3 RFC 20 ("Research Engine") con la evidencia del audit RFC 30 (ecosistema + Jev) y las decisiones ya tomadas sobre Context7Max / anydoc / grill-me. Output: sub-fases atómicas commiteables (commits no PRs), cada una demo-ready standalone, feature-gated default-off cuando toca crates nuevas (RFC 25 §11).

---

## SECTOR A — Decisiones ya tomadas (evidencia RFC 30 + Round 6)

### A.1 Context7Max — fuente primaria de docs (proyecto propio del operador)

`D:\Documentos\Projects\Context7Max` — self-hosted Context7 alternativo: Supabase (Postgres + pgvector + Edge Functions) + Vercel (API) + GitHub (repo + Actions). Indexa documentación real de librerías/CLIs/APIs, extrae **ejemplos de código verbatim** y los sirve con búsqueda híbrida (vectorial + full-text). Capas: `library`/`docs` (Context7 clásico), `guide` (53 dominios paso a paso), `skill` (skills instalables), `mcps` (registro MCP), `add --pack` (packs canónicos por dominio). Embeddings sin cuota: `gte-small` local (Hugging Face Transformers).

**Decisión:** fuente primaria del Research Engine para documentación; fallback Context7 MCP (comportamiento actual de AGENTS.md §5) cuando ctx7max no está instalado/reachable. Integración vía CLI (`ctx7max docs <id> "<query>"`) — CLI/API externa, sin crate nueva (RFC 25 §11).

### A.2 anydoc — ingestion de documentos locales

`firecrawl/anydoc` — convierte Word/PowerPoint/Excel/OpenDocument/RTF/EPUB/CSV/PDF → Markdown limpio. **Built in Rust**, con bindings Node.js/Python. Complementa §E Firecrawl (que cubre web ingestion): el Research Engine también consume documentos locales (specs PDF, notas .docx, dumps CSV).

**Decisión:** candidato Phase 3.2 como feature-gated crate (audit single-binary-safety previo obligatorio — verificar tamaño + deps transitive antes de añadir a Cargo.toml).

### A.3 grill-me — stress-test de planes

`mattpocock/skills → grill-me` — "Relentless interviewing skill that stress-tests plans and designs through systematic questioning. Conducts deep-dive questioning across all aspects of a plan, walking through decision trees branch-by-branch until shared understanding is reached."

**Decisión:** skill bundled (brecha D, RFC 29 §D) + gate del Planning Engine: antes de `Plan.lock`, el motor ejecuta un grill pass que genera preguntas sobre decisiones no resueltas y las presenta al usuario (RFC 12 anti-gate: no Coding si `Plan.confidence < 0.7` — grill-me es el mecanismo de subida de confidence).

### A.4 "System One judgments" — patrón Jev para el Research Engine

El audit Jev (RFC 30 §2) destila el patrón dominante: un modelo pequeño/barato toma decisiones rápidas (routing, eval, compaction). Para el Research Engine: el `weak_model` (Aider tri-model, sub-fase 2.0) clasifica fuentes, filtra ruido y resume; el modelo fuerte solo sintetiza el consenso final. `n_samples < 3` → "no data" (mismo invariant que `AffinityRow::MIN_SAMPLES`).

---

## SECTOR B — Plan refinado Phase 3 (6 sub-fases atómicas)

### Sub-fase 3.0 — Foundation (Report types + M25 + consensus scorer)

- M25 migration: SQLite `research_runs` (id, query, status, confidence, recommended, created_at, completed_at) + `research_sources` (id, run_id, kind, url, score, fetched_at) + `research_consensus` (run_id, dimension, score, note).
- `ResearchRunReport` tipos canónicos (RFC 10 §7 YAML shape) en `research/report.rs` — serde round-trip.
- `trait ConsensusScorer` + `enum ConsensusDimension { Community, Enterprise, Academic, Official }` en `research/consensus.rs`.
- `enum ResearchRunKind { Full, Targeted, Mega }` (RFC 10 §1 disparadores) — el Planning/Prompt Understanding decide el kind, el usuario fuerza con `/research`.
- Tests: migration idempotente, report round-trip, dimension enum.

### Sub-fase 3.1 — Docs gateway (Context7Max adapter)

- Adapter facade `research/docs_gateway.rs` estilo firecrawl (RFC 28 §E): `DocsGateway::from_env()` — si `ATLAS_CTX7MAX_URL` / CLI `ctx7max` en PATH → ctx7max; fallback Context7 MCP shape; fallback último: webfetch docs oficiales.
- `query_docs(library_id, question) -> Vec<DocSnippet>` — ejemplos de código verbatim + hybrid search.
- CLI: `atlas research docs <library> "<question>"` (sub-comando del `research` existente).
- HUD: results surface (RFC 24) — anotado como follow-up.
- Sin crate nueva (CLI/API externa). Tests: env resolution, fallback chain, empty-env.

### Sub-fase 3.2 — Document ingestion (anydoc, feature-gated)

- Crate `anydoc` (o SDK equivalente) — **audit single-binary-safety primero** (RFC 25 §11): tamaño, deps transitive, MSRV. Si el audit falla → parser mínimo propio (PDF text extraction + pandoc external tool opt-in, NO bundled).
- Feature `doc-ingest` default off. `atlas research ingest <file>` → Markdown + `research_sources` row (kind=Document).
- Tests: round-trip smoke, unsupported format error.

### Sub-fase 3.3 — Collective Engineering Intelligence

- `ConsensusScorer` impls por dimensión (RFC 10 §5): GitHub Issues/Discussions vía `gh` CLI + REST (maintainers/enterprise), arXiv vía webfetch (academic), docs vía 3.1 (official), community vía SO/Reddit/HN.
- Scoring 0–100 por dimensión + referencias citadas; `confidence` combinado ponderado (hands-on notes §3 pesan ×1.5).
- Weak-model pre-filter: el `weak_model` clasifica cada fuente antes de score (A.4) — modelo fuerte solo sintetiza.
- CLI: `atlas research query "<pregunta>"` — run completo, output YAML (RFC 10 §7).
- Tests: scorer determinista (fixture sources), hands-on weight, fail-safe threshold.

### Sub-fase 3.4 — Hands-on + ramas de aplicación

- Notas de evidencia experta (RFC 10 §3): `research_notes` SQLite (M25) + firma user + CLI `atlas research note --title ... --decision ...`.
- Ramas Opción A/B/C (RFC 10 §4): cada rama une consenso + docs + encaje en arquitectura (Context Engine Project Map) + coste estimado en este código.
- Journal ref: cada run produce `journal_ref` referenciado en el Journal (auditable).
- Tests: nota round-trip, rama shape.

### Sub-fase 3.5 — probe_feasibility + fail-safe + grill gate

- `probe_feasibility(topic, domain)` (RFC 10 §11, spec Added Draft v1): Fuentes por dominio (Software/Hardware/Academic/Vendor), reglas de puerta (≥3 fuentes, ≥1 primaria, sin red_flags), cache Vector KB (12 días TTL).
- CLI: `atlas research feasibility "<topic>" --domain ...`.
- Fail-safe (RFC 10 §10): Confidence < umbral → "needing human" + replanificación — anti-alucinación duro.
- Grill gate (A.3): antes de `Plan.lock`, grill pass sobre decisiones no resueltas — skill `grill-me` bundled + prompt del Planning Engine.
- Tests: red_flags detection, cache hit, fail-safe escape.

**Entregable:** la IA decide con evidencia social y académica, no con suposición. KPI: confidence medio de decisiones ≥ 0.75 (RFC 20); "creo que..." eliminado del vocabulario del Research Engine.

---

## SECTOR C — Fuera de alcance (esta iteración)

- Reddit/SO scrapers dedicados (rate limits + ToS — webfetch puntual es suficiente para 3.3).
- arXiv MCP dedicado (webfetch con parseo alcanza el mismo resultado).
- Context7Max como crate embebida (es CLI/API externa por diseño — el operador la corre self-hosted).
- Semantic Scholar Graph API formal (webfetch primero, API formal post-MVP).

## SECTOR D — Siguiente paso operativo

1. Commit de este plan.
2. Actualizar RFC 20 Fase 3 con las 6 sub-fases + RFC 10 con markers IMPLEMENTED/PENDING por sub-fase + RFC 26 catálogo.
3. ~~Empezar sub-fase 3.0 (Foundation) — M25 + tipos + consensus scorer trait.~~ ✅ 3.0, 3.1, 3.2, 3.3, 3.4 y 3.5 IMPLEMENTADAS — Phase 3 completa (3.5: `research::feasibility` + `atlas research feasibility` + M27 `feasibility_cache` + grill gate `planning::grill` + skill `grill-me`).
