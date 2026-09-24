# 32 — Phase 5 Learning + Compression (plan refinado)

**Propósito:** Refinar la Fase 5 RFC 20 ("Learning + Compression") con la evidencia del audit RFC 30 (patrón "System One compaction" de `tamaratran/fast-jev-compaction`) y la RFC 16 (Learning Engine). Output: sub-fases atómicas commiteables (commits no PRs), sin crates nuevas (RFC 25 §11).

**Gestión (RFC 22 Round 7 — framework de delegación):** el trabajo mecánico pesado (bulk-draft de módulos, tests repetitivos, verificación) se delega a `muse-spark-1.3-contributor-free` vía `opencode run` con spec detallada; el gestor revisa (senior review), commitea atómicamente y pushea. Judgment calls (arquitectura, seguridad, decisiones de diseño) permanecen en el gestor.

---

## SECTOR A — Decisiones ya tomadas (evidencia)

### A.1 Reflection Engine — la base existe

`src-tauri/src/learning/{types, runner}.rs` ya emite draft patterns (confidence 0.7) desde `RepairReport` con `RuleLifecycle`/`RuleWhen`/`RuleThen`/`PatternMetrics` y tracking `was_correct` (M7 `pattern_runs`). Phase 5 formaliza el loop completo: **error → root cause → qué regla faltó → nueva regla → guardar → nunca repetir**, con dedup de errores repetitivos y promoción draft→verified.

### A.2 System One compaction (fast-jev-compaction pattern, RFC 30 §2)

El patrón: la compaction de history la hace el **modelo pequeño/barato** (`weak_model` tri-model), no truncado ni con el modelo grande. En el core: stub determinista (resumen por rolling window) + wiring LLM-driven documentado como follow-up. Coincide con Aider: "weak hace compactación de history, no commits" (sub-fase 2.0).

### A.3 Auto-reglas YAML (RFC 16 §4)

`src-tauri/src/skills/manifest.rs` documenta: "YAML (RFC 16 §4 `.opencode/rules/*.yaml` generado por el Learning Engine). Phase 1 solo lee TOML; Phase 2 añadirá YAML". Phase 5 materializa el writer + loader.

### A.4 Compresión de skills (RFC 06 §5)

"skills auto-compresoras": detectar similitud → fusionar → resumir → actualizar. Sin embeddings obligatorios (fastembed feature-gated): similitud determinista por token-overlap (Jaccard) es suficiente para el MVP; embeddings opcionales como follow-up.

---

## SECTOR B — Plan refinado Phase 5 (5 sub-fases atómicas)

### Sub-fase 5.0 — Foundation (M30 + YAML rules)
- M30 migration: `learned_rules` (id, when_trigger JSON, then_action JSON, priority INTEGER, lifecycle TEXT CHECK, was_correct INTEGER nullable, n_applied INTEGER, created_at, updated_at) + `compaction_events` (id, mission_id, entries_before, entries_after, summary, model_id, created_at).
- `learning/rules.rs`: writer YAML `.opencode/rules/<id>.yaml` (RFC 16 §4 — RuleWhen/RuleThen → YAML shape) + loader YAML→Pattern (round-trip).
- `Journal::{save_learned_rule, list_learned_rules, promote_rule, deprecate_rule}` (idempotente).
- Tests: M30 idempotente, YAML round-trip, promote/deprecate lifecycle transitions.

### Sub-fase 5.1 — Reflection Engine formal
- Loop: error repetitivo (dedup por firma del error en `RepairReport`) → root cause → nueva regla draft → promoción con `was_correct` (M7 pattern_runs ya lo trackea) → deprecate si `is_deprecated` (stale).
- Extiende `learning/runner.rs`: `reflect(repairs: &[RepairReport]) -> Vec<Pattern>` (dedup + agrupación por firma) + `promote_draft(pattern) -> Pattern` (lifecycle draft → verified, was_correct >= threshold).
- CLI: `atlas learn rules [-n N]` / `atlas learn promote <id>` / `atlas learn deprecate <id>`.
- Tests: dedup determinista, promoción con threshold, deprecate stale.

### Sub-fase 5.2 — Compresión de Skills (RFC 06 §5)
- `learning/compress.rs`: similitud Jaccard por token-overlap entre skills (bundled + perfil) → pares > umbral (default 0.7) → propuesta de fusión (dry-run: `CompressProposal {keep, merge, similarity}`) / `--apply` escribe la skill fusionada (skill.toml merge + summary) y deprecada la absorbida.
- CLI: `atlas skills compress [--threshold 0.7] [--apply]`.
- Tests: similitud determinista (fixtures), umbral filtra, fusión mergea campos, dry-run no toca disco.

### Sub-fase 5.3 — System One compaction
- `learning/compaction.rs`: cuando el tail de journal_events de una mission excede `COMPACTION_THRESHOLD` (default 100 entradas), genera resumen por rolling window (stub determinista — headlines + conteo por kind) → `compaction_events` + `Journal::compacted_summary(mission_id)`.
- Wiring LLM-driven (weak_model resume real): documentado como follow-up — el stub deja el shape listo.
- CLI: `atlas learn compact --mission <id>` / `atlas learn summary <mission_id>`.
- Tests: threshold dispara compaction, summary round-trip, entries_after < entries_before.

### Sub-fase 5.4 — Ajuste dinámico de prompts (RFC 16 §5)
- Hook en `prompt/runner.rs`: consulta las `learned_rules` consultables (`is_consultable`) del profile y las inyecta como hints de contexto en el pipeline (match por `RuleWhen.trigger` contra el prompt) — solo si el wiring no rompe firmas existentes (mismo criterio que brecha C).
- Tests: rule match inyecta hint, no match no inyecta, reglas deprecadas no se consultan.

**Entregable:** el editor **mejora solo** según el uso. KPI (RFC 20): skills redundantes reducidas -30% en 3 meses; errores repetitivos → reglas automáticas.

---

## SECTOR C — Fuera de alcance (esta iteración)

- Embeddings para similitud de skills (Jaccard determinista es el MVP; fastembed feature-gated como follow-up).
- Compaction LLM-driven real (weak_model resume con API — stub primero, wiring post-MVP).
- Marketplace de reglas compartidas entre usuarios (Phase 10).

## SECTOR D — Siguiente paso operativo

1. Commit de este plan + RFC 20 update.
2. Empezar sub-fase 5.0 (Foundation) — delegada a muse-spark-1.3.
3. Sub-fases 5.1–5.4 siguen el mismo patrón de gestión.
