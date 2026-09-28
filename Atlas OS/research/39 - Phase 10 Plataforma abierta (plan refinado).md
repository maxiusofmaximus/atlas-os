# 39 - Phase 10 Plataforma abierta (plan refinado)

Plan atómico para la Fase 10 del roadmap (RFC 20) — la última fase del roadmap v1: Atlas OS como **plataforma**. Complementa RFC 38 (artemis lateral) y extiende los patrones validados en las Phases 2–9.

## SECTOR A — Decisiones ya tomadas (evidencia RFCs existentes)

### A.1 Firmas de skills (7.0) como base del marketplace
- `security/signature.rs` (`ChecksumVerdict`) + `.checksum` sidecar + install gate fail-safe (7.0): la firma SHA-256 ya existe — el marketplace la hace OBLIGATORIA (sin `.checksum` → rechaza).

### A.2 Skill Picker (8.1) + bundled catalog (29 §D) como base del SDK
- `skills/manifest.rs` (`Engine`) + `skills/picker.rs` (scoring determinista) + catálogo bundled (12 `opencode-*` via `include_str!`): el SDK documenta cómo escribir Skills que estos consumidores entienden.

### A.3 Reflection formal (5.1) como base del learning social
- `learning/types.rs` (`RuleLifecycle` verified) + `learned_rules`/`.opencode/rules/` YAML: las reglas VERIFICADAS son las compartibles entre usuarios.

### A.4 Provenance pattern (graph/) para el remixing
- `graph/mod.rs` (`Provenance` EXTRACTED/INFERRED/AMBIGUOUS, verbatim graphify): el remixing usa `remixed_from` como provenance — cada skill forkeada conserva su origen.

## SECTOR B — Plan refinado Phase 10 (4 sub-fases atómicas)

### Sub-fase 10.0 — Skill SDK público (scaffold) — M37
- `skills/sdk.rs`: `scaffold_skill(name, engine, dir) -> SkillManifest` — template validado (id snake_case único, version semver, engine enum, lifecycle draft) + escribe el manifest TOML/JSON + skill.md stub.
- CLI: `atlas skill new <name> [--engine <e>]` — scaffold + imprime el path + próximos pasos (firmar con `atlas skill sign` si existe / verificar).
- Tests: scaffold determinista, fallo en name inválido/duplicado, manifest round-trip.

### Sub-fase 10.1 — Marketplace con firma obligatoria — M38
- `skills/marketplace.rs`: `install_skill(ref) -> Verdict` — OBLIGATORIO `.checksum` (7.0): sin firma → RECHAZA (fail-safe, patrón supply_gate 7.3); `publish_skill(dir)` — empaqueta + firma + `.checksum` sidecar.
- CLI: `atlas skill install <path|ref>` / `atlas skill publish <dir>` — install verifica firma ANTES de copiar (patrón approval_for 7.1).
- Tests: install sin firma rechaza, install firmado copia, publish firma determinista.

### Sub-fase 10.2 — Remixing de Skills — M39
- `skills/remix.rs`: `fork_skill(src, new_name) -> SkillManifest` — copia con nueva id/versión + provenance `remixed_from` (patrón graph/) + lifecycle draft (re-verifica el remix antes de promover).
- CLI: `atlas skill fork <ref> --name <new>`.
- Tests: fork determinista, provenance conservada, fallo en src inexistente.

### Sub-fase 10.3 — Learning social (compartir reglas verificado) — M40
- `learning/share.rs`: `export_rules(lifecycle min) -> Path` — exporta `learned_rules` verificado (5.1) a YAML compartible + `import_rules(path)` — valida + importa con firma del archivo (hash — patrón signature.rs) + dedup (5.1).
- CLI: `atlas learn export/import <file>`.
- Tests: export determinista, import valida firma, import dedup, fallo en firma inválida.

**Entregable:** Atlas OS como **plataforma** (RFC 20). KPI: Confidence medio ≥ 0.75, skills redundantes -30%.

## SECTOR C — Fuera de alcance (esta iteración)

- Marketplace REMOTO con red (HTTP registry — el marketplace es LOCAL firmado; la red va a Roadmap v2).
- Hosting de skills de terceros (el operador comparte por git/repos — jamás servidor central).
- Mobile testing surface (lateral 8.6, RFC 38 §5 — evaluar tras Phase 10).

## SECTOR D — Siguiente paso operativo

1. Commit de este plan + RFC 20 update + Index 26.
2. Sub-fase 10.0 (delegada a muse-spark-1.3, zero-dep) → revisar → commit → push.
3. Sub-fases 10.1-10.3 (todas zero-dep, patrón std-only).
4. Tras Phase 10: Roadmap v2 (evaluar 8.6 mobile testing + marketplace remoto).
