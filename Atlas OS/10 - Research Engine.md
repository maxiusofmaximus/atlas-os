# 10 - Research Engine

Investiga la pregunta **internet adentro** antes de cualquier decisión crítica. La IA ya no responde *"creo que..."*: responde *"el 83% de las fuentes recomienda X"*.

> **Estado de implementación.** Phase 1/1.5 materializan el CLI `atlas research` (Firecrawl web ingestion, RFC 28 §E, feature-gated). La Fase 3 está planificada con 6 sub-fases atómicas (3.0 Foundation → 3.1 Docs gateway Context7Max → 3.2 anydoc ingestion → 3.3 Collective Engineering Intelligence → 3.4 Hands-on + ramas → 3.5 probe_feasibility + grill gate) en `Atlas OS/research/30 - Phase 3 research engine.md`. **§3.0 Foundation ✅ IMPLEMENTADO** (`research::{report, consensus}` + `journal/research.rs` + M25 schema 24→25 + `ResearchRunKind`/`ConsensusScorer`/`combined_confidence`); **§3.1 Docs gateway ✅ IMPLEMENTADO** (`research::docs_gateway::DocsGateway` + `atlas research docs`, Context7Max vía `ATLAS_CTX7MAX_URL`/`ctx7max` CLI con fallback Context7 MCP shape → official docs, sin crate nueva); **§3.2 Document ingestion ✅ IMPLEMENTADO** (`research::ingest::ingest_file` + `atlas research ingest`, parser propio sin crate nueva por audit single-binary-safety RFC 25 §11 — md/txt passthrough, CSV → tabla Markdown, extracción de texto PDF — más `pandoc` externo opt-in para office/epub/rtf, feature `doc-ingest` default-off, fila `research_sources` kind=`document`); **§3.3 Collective Engineering Intelligence ✅ IMPLEMENTADO** (`research::collective` — pre-filtro weak-model determinista `prefilter_sources`, scorers por dimensión `Community`/`Enterprise`/`Academic`/`Official` 0–100 con referencias citadas + `score_all`, `top_reference` por dimensión, `HANDS_ON_WEIGHT=1.5`, fail-safe `FAIL_SAFE_CONFIDENCE=0.6` → `NeedingHuman` vía `apply_fail_safe`, `build_report` + `ReportSections` al YAML canónico RFC 10 §7, colectores best-effort sin crate nueva — `gh` CLI, arXiv API, docs vía 3.1 — más `atlas research query "<pregunta>" [--source/--hands-on/--library/--gh-repo/--min-confidence]`, run persistido en M25); **§3.4 Hands-on + ramas ✅ IMPLEMENTADO** (`research::hands_on` — `ResearchNote` evidencia experta RFC 10 §3 con `validate()` + `mentions()`, `ApplicationBranch` Opción A/B/C RFC 10 §4 con pros/contras/confianza/coste/`fits_stack`/fuentes citadas + `build_branches` determinista top-3 con +0.05 por respaldo experto y `branch_proposal_lines` al YAML canónico, `journal_ref_for_run` rr→jr + `parse_tags`, M26 `research_notes` con CHECK 0..1 + `save/get/list_research_notes` + `record_research_journal_ref` auditable en `journal_events` kind=`research_run`, más `atlas research note --title/--decision/…` y `atlas research branches --run-id`, `query` ahora emite `journal_ref` y proposal desde ramas con notas persistidas incluidas); **§3.5 probe_feasibility + grill gate ✅ IMPLEMENTADO** (`research::feasibility` — tipos §11.1, colectores por dominio §11.2 sin crate nueva, gate §11.3 vía `evaluate`, `fail_safe_status` a `NeedingHuman` §10, cache 12 días §11.6 en M27 `feasibility_cache`, métricas §11.7 vía `ProbeMetrics`, más `atlas research feasibility`; grill gate `planning::grill::grill_plan` + skill bundled `grill-me` + pass impreso por `atlas plan` antes de `Plan.lock`). Evidencia: audit RFC 30 (ecosistema + Jev) + HydraFusion (RFC 22 §13) + decisiones A.1–A.4.

---

## 1. Cuándo corre (obligatorio)

| Disparador | Acción |
|---|---|
| Bifurcación arquitectónica | Full Run |
| Elección de patrón / librería | Full Run |
| Bug no doméstico / desconocido | Targeted Run |
| Cambio de stack principal | Mega Run |
| Antes de migrar / refactor grande | Mega Run |
| Tarea trivial de código local | Skip (auto) |

El usuario puede forzar Research con un comando (`/research pregunta`).

## 2. Fuentes consultadas

```
Documentación oficial
  ↓
GitHub (Issues, Discussions, Releases)
  ↓
RFC oficiales
  ↓
Papers (arXiv / Semantic Scholar)
  ↓
Libros anexos (referencias citadas)
  ↓
Blogs técnicos (curados)
  ↓
StackOverflow
  ↓
Reddit (subreddits técnicos)
  ↓
Videos técnicos (transcripciones)
  ↓
Benchmarks (canchas públicas + propias)
  ↓
Buenas prácticas (Microsoft / Google / AWS / CNCF)
  ↓
Comunidad (Hacker News, Lobsters, Dev.to)
  ↓
Comparar soluciones
  ↓
Extraer consenso
  ↓
Confidence Score
```

## 3. Hands-on: *"esto yo lo hice así (profesionalmente)"*

El Research Engine no solo recopila literatura. Permite al usuario (o al equipo) adjuntar **casos reales cerrados**:
```
note:
  title: "Lo hice así en producción"
  project: "fintech X"
  decision: "Event Sourcing + Kafka"
  outcome: "exitoso pero costoso en ops"
  confidence: 0.81
  tags: [architecture, event-sourcing, k]
  attached_at: 2026-07-04
  signature: <user>
```

Estas notas se tratan como **evidencia experta**, pesan más que cualquier blog.

## 4. *"Cómo podrías hacerlo tú"* — planteamientos

El motor también produce **ramas de cómo se podría aplicar una solución al proyecto actual**. Output canónico:

```
Opción A  → pros / contras / confianza / coste estimado en este código
Opción B  → pros / contras / confianza / coste estimado en este código
Opción C  → pros / contras / confianza / coste estimado en este código
```

Cada opción une:
- lo que dice la comunidad,
- lo que recomienda la documentación oficial,
- cómo encaja en la arquitectura actual (usando Context Engine).

## 5. Collective Engineering Intelligence

La sabiduría social, formalizada:

```
¿Qué hace la comunidad?
   ↓
¿Qué hacen los maintainers?
   ↓
¿Qué hacen las empresas?
   ↓
¿Qué hacen los autores?
   ↓
¿Qué dice la documentación oficial?
   ↓
¿Qué hace la mayoría?
   ↓
   Genera:
     - Community Consensus
     - Enterprise Consensus
     - Academic Consensus
     - Official Consensus
   ↓
Confidence combinado
   ↓
Solo entonces se decide
```

Cada una de las consensos lleva score (0–100) y referencias citadas.

## 6. Motor de búsqueda concreto

Atlas OS orquesta varias herramientas:
- `webfetch` para páginas citadas
- query GitHub Issues / Discussions vía `gh` (CLIs → ver `08 - CLI.md`)
- búsqueda arXiv (MCP dedicado o webfetch con parseo)
- búsqueda en Reddit (Mis: reddit-mcp o scraper sandbox)
- StackOverflow API
- Context7 para docs live (MCP → ver `07 - MCP.md`) — ✅ vía `research::docs_gateway::DocsGateway` (sub-fase 3.1): Context7Max primario (`ATLAS_CTX7MAX_URL` / CLI `ctx7max`), fallback Context7 MCP shape, último recurso webfetch docs oficiales; CLI `atlas research docs <library> "<pregunta>"`.
- Documentos locales (specs PDF, notas .docx, dumps CSV) — ✅ vía `research::ingest::ingest_file` (sub-fase 3.2, alcance anydoc): md/txt passthrough, CSV → tabla Markdown, texto PDF nativo; office/epub/rtf vía `pandoc` externo opt-in (`ATLAS_PANDOC_BIN` / `PATH`, nunca bundled); CLI `atlas research ingest <file> [--run-id ID] [--raw]` con fila `research_sources` kind=`document`. Feature `doc-ingest` default-off (RFC 25 §11).

## 7. Salidas

Cada Research Run produce ResearchRunReport:

```yaml
id: rr-2026-07-04-001
query: "¿Event sourcing vs CRDT para nuestro ledger?"
sources: 87
consensus:
  community: strategy_pattern             (0.71)
  enterprise: event_sourcing             (0.66)
  academic:   crdt                        (0.55)
  official:   event_sourcing             (0.78)
authors:
  - "Martin Kleppmann → crdt"
  - "The Rust maintainer X → event_sourcing"
enterprise:
  - "Microsoft → event_sourcing"
  - "Confluent → event_sourcing"
bugs:
  - "GitHub issue #1234: crash bajo X carga en Kafka 3.7"
proposal:
  - "A: Event Sourcing + Kafka, fits actual stack, +18% ops cost"
  - "B: CRDT via Automerge, más simple pero menos tooling"
  - "C: Hybrid, no recomendado por consenso"
confidence: 0.81
recommended: A
journal_ref: jr-2026-07-04-001
```

## 8. Métricas

Se registran: número de fuentes, tiempo, tokens, modelos usados, calidad del outcome final (`was_correct` marcado a posteriori por el Learning Engine).

## 9. Cache

Resultados recientes (~7 días) se cachean en Vector KB para no repetir Research sobre el mismo tema. Invalidación si cambia el stack del proyecto.

## 10. Fail-safe

Si el Research Engine no logra Confidence ≥ umbral:
- marca la decisión como "needing human",
- escapa al Planning Engine para replanificar,
- sigue sin forzar una respuesta inventada.

Este es el **anti-alucinación duro**: si no hay evidencia, no hay output.

---

## 11. `probe_feasibility` — verificación de capacidad edge-case

Capability consumida por el Prompt Understanding Pipeline (RFC `23 §2.2` paso 6) cuando el `gap_type` señala `capacity_hallucination` o `unknown_tool_dependency`. Antes de que el sistema termine de creer que "sí se puede", el Research Engine verifica mecánicamente si existe la herramienta/dependencia/ruta/dominio.

### 11.1 Firma

```rust
pub struct FeasibilityProbe {
  pub topic: String,         // p.ej. "impresora 3D de casas, marca ICON"
  pub domain: FeasibilityDomain,
  pub min_sources: u8,        // default 3
  pub require_artifact_evidence: bool,
}
pub enum FeasibilityDomain {
  Software,    // npm, crates.io, pypi, hex, nuget, gem, go modules
  Hardware,    // vendor sites + specs
  Academic,    // arXiv, Semantic Scholar, OpenReview
  Vendor,      // compañía + productos oficiales
}

pub struct FeasibilityReport {
  pub probe_id: String,
  pub topic: String,
  pub found: bool,
  pub artifact_evidence: Vec<ArtifactEvidence>,
  pub confidence: f32,         // 0..1
  pub red_flags: Vec<String>,
  pub recommended_next_step: String,
}

pub struct ArtifactEvidence {
  pub kind: ArtifactKind,
  pub url: String,
  pub fetched_at: String,
  pub raw_metadata: serde_json::Value,
  pub stars_or_stargazers: Option<u64>,
  pub last_release_at: Option<String>,
}
pub enum ArtifactKind {
  Package,           // npm/crates.io/pypi/…
  GithubRepo,
  Release,
  Paper,
  VendorProduct,
  VendorDocs,
  AcademicDataset,
  BenchmarkRun,
}
```

### 11.2 Fuentes por `FeasibilityDomain`

| Domain | Sources |
|---|---|
| `Software` | npm registry API, crates.io API, PyPI JSONRPC, Hex, nuget, RubyGems, Go modules proxy, GitHub releases, stars via REST `GET /repos/{o}/{r}` |
| `Hardware` | vendor doc URLs (estilo `iconbuild.com`, `winsun3d.com`), HTTP fetch con head extraction, link-rot detection |
| `Academic` | arXiv (`/abs/<id>`), Semantic Scholar Graph API, OpenReview, Connected Papers |
| `Vendor` | sitio oficial + Wayback fallback + 30 derniers jours de releases |

### 11.3 Reglas de puerta

`found = true` solo si:

1. ≥ 3 fuentes distintas (`min_sources`).
2. Al menos una fuente primaria (artifact original + GitHub release) no secuencia.
3. Sin `red_flags`: 0 releases en 12 meses → "abandoned" (aún si stars altas), o "no results" en academic.

### 11.4 Integración con el Prompt Understanding

```
user: "construye una casa con impresora 3D que casi nadie conoce"
  ↓
Verdict detecta gap_type = capacity_hallucination + unknown_tool_dependency
  ↓
Paso 6 (auto-resolve) hit: NO se conoce el tool exacto en Project Map
  ↓
probe_feasibility(topic="casas con impresión 3D",
 domain=Hardware+Vendor+Academic)
  ↓
FeasibilityReport {
  found: true,
  artifact_evidence: [
    { kind: VendorProduct, url: iconbuild.com/vulcan, fetched_at: … },
    { kind: Paper,        url: arxiv.org/abs/2006.03002, … },
    { kind: GithubRepo,   url: github.com/ForkRobotics/foldable-house-printer, stars: 87 },
  ],
  confidence: 0.67,
  red_flags: []
  recommended_next_step:
    "confirmar con el usuario; pídele que especifique marca/modelo
     antes de construir pipeline de simulación de piezas"
}
  ↓
Clarification questions generadas para resolver alcance
  ↓
Verdict final MEDIUM (confidence 0.62) → suggested_mode = architect
```

### 11.5 Anti-patterns que se bloquean

- "fly to the galaxy" (capacity_hallucination puro) → `probe_feasibility` retorna `found=false` con `red_flags=["no artifact evidence", "no academic source"]`. El Prompt Understanding bloquea el avance y pide confirmación del usuario (`23 §5`).
- Paquetes abandonados con releases 2018 → red flag "abandoned", se sugiere alternativa activa antes de aceptar.
- Mundo de paquetes coquete (`npm i shittier`) → mismo procedimiento.

### 11.6 Cache

`probe_feasibility` cachea en Vector KB (semántica similar a Research Run). Si el usuario repite prompt similar (`'impresora casa 3d obras'`), reutiliza el report (12 días TTL).

### 11.7 Métricas

- `probe_feasibility` hit rate (encontró lo pedido): distribución.
- `red_flags` por dominio.
- tiempo típico: objetivo <5s con 3 fuentes paralelas.

### 11.8 CLI

```bash
$ opencode research feasibility "casas con impresora 3D" --domain hardware,vendor,academic
Found: true
Confidence: 0.67
Evidence:
  [1] https://iconbuild.com/vulcan (vendor product, fetched 2026-07-13T14:02Z)
  [2] https://arxiv.org/abs/2006.03002 (paper, peer-reviewed)
  [3] github.com/ForkRobotics/foldable-house-printer (87 stars, last release 2026-04)
Red flags: [none]
Recommended: confirm brand/model with user before simulating parts.
```

### 11.9 Estado

- Status: ✅ IMPLEMENTADO (Phase 3 sub-fase 3.5: `research::feasibility` + `atlas research feasibility` + M27 `feasibility_cache`)
- Depends on: `23 - Prompt Understanding & Refinement.md` §2.2 (paso 6)
- Consumido por: Prompt Understanding Pipeline (`23`) cuando `gap_type ∈ {capacity_hallucination, unknown_tool_dependency}`.
