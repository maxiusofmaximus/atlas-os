# 34 — Phase 7 Security & Compliance (plan refinado)

**Propósito:** Refinar la Fase 7 RFC 20 ("Seguridad & Compliance") con la RFC 18 (spec completa de seguridad). Estado actual: `security_scan` + `supply_chain` stages del Validation ya existen; `sha2 = "0.10"` + `hex = "0.4"` ya están en deps (firmas sin crates nuevas). Lo faltante: firmas de skills, sandbox levels, compliance skills, supply-chain install gate. Output: sub-fases atómicas, sin crates nuevas salvo audit previo (RFC 25 §11).

**Gestión:** trabajo mecánico pesado delegado a `muse-spark-1.3-contributor-free` vía `opencode run`; el gestor revisa (senior review), commitea y pushea. Judgment calls (decisiones de seguridad) permanecen en el gestor.

---

## SECTOR A — Decisiones ya tomadas (evidencia RFC 18 + estado)

### A.1 Firmas: SHA-256 MVP + ed25519 diferido

RFC 26 catálogo: "Firmas de skills con SHA-256 + minisign". El MVP: **checksum SHA-256** por skill (manifiesto + archivos) verificado en install — `sha2` + `hex` ya en deps, cero crates nuevas. **ed25519/minisign diferido**: son crates nuevas (`ed25519-dalek`/`minisign`) — audit single-binary-safety previo obligatorio; el MVP con checksum cubre la integridad del catálogo bundled (que además se compila via `include_str!` — trusted by construction) y las skills de perfil.

### A.2 Sandbox levels: tipos + policy mapping (ejecución real diferida)

RFC 18 §6: `none`/`vuOnly`/`container`/`wasm`. El MVP: `enum SandboxLevel` + mapping a la política de aprobaciones (RFC 02 §"Política de ejecución": auto/confirm/forbidden) + fail-safe hacia lo seguro. La **ejecución real** en Docker/Podman (container) es infraestructura externa — via CLI docker si el operador lo tiene, feature/documentada como follow-up.

### A.3 Supply chain: gate determinista + Socket/Snyk follow-up

RFC 18 §4: telemetría Socket (typosquatting, postinstall, env-var access) + Snyk CVE. El MVP: **gate determinista** pre-install (heurística typosquatting por distancia de edición sobre el nombre del package, detección de postinstall scripts y env-var access en el package.json/manifest) — las APIs SaaS (Socket/Snyk) se integran como follow-up documentado (son servicios externos con API key).

### A.4 Compliance skills (RFC 18 §12)

OWASP/GDPR/PCI/HIPAA skills como bundled skills del engine `security` — checklists accionables (no texto narrativo): cada skill lleva `RuleWhen`/`RuleThen` shape + prioridad.

---

## SECTOR B — Plan refinado Phase 7 (4 sub-fases atómicas)

### Sub-fase 7.0 — Firmas de skills (SHA-256 MVP)
- `security/signature.rs`: `skill_checksum(dir) -> Result<String>` (SHA-256 canónico de skill.toml + README, orden estable) + `verify_skill(dir, expected) -> Result<ChecksumVerdict {Ok, Mismatch, Missing}>` + `.checksum` sidecar writer/reader.
- `security/mod.rs` (nuevo módulo) + integración: `atlas skill install` verifica el checksum antes de cargar (fail-safe Forbidden si mismatch — RFC 18 §5); bundled skills verifican por construcción (test que el checksum del catálogo es estable).
- Tests: checksum determinista, mismatch detectado, missing → verdict, round-trip sidecar, install gate fail-safe.

### Sub-fase 7.1 — Sandbox levels (tipos + policy)
- `security/sandbox.rs`: `enum SandboxLevel {None, VuOnly, Container, Wasm}` (as_str/parse round-trip) + `approval_for(level, action) -> Approval {Auto, Confirm, Forbidden}` — mapping RFC 18 §2/§6 (escritura FS workspace Confirm, red saliente Forbidden, exec sandbox Confirm...) + fail-safe Forbidden en edge case (nivel desconocido, acción desconocida).
- Tests: round-trip, mapping exacto de la tabla §2, fail-safe.

### Sub-fase 7.2 — Compliance skills (OWASP/GDPR/HIPAA)
- 3 skills bundled en `skills/` (engine = "security", manifests RFC 06 §1 completos): `atlas-owasp-check` (Top 10 checklist accionable), `atlas-gdpr-check` (Art 5/25/32/33 checklist), `atlas-hipaa-check` (45 CFR §160/164 safeguards checklist).
- Cada una con skill.toml + README + el checklist como contenido del README (referencias de ley citadas verbatim — el patrón data-breach-blast-radius: labels "exact" vs "heuristic").
- Tests: los 3 manifests parsean con el loader real, engine security, verificados.

### Sub-fase 7.3 — Supply-chain install gate
- `security/supply_gate.rs`: `evaluate_package(name, manifest_json opcional) -> SupplyVerdict {Pass, Warn, Block, reasons}` — determinista: typosquatting heurística (edit distance <= 2 contra una lista de nombres conocidos/populares si se pasa, o contra el registry local), postinstall scripts detection (preinstall/postinstall en manifest_json), env-var access detection. Sin red en el MVP (Socket/Snyk APIs follow-up documentado).
- Integración: el stage `supply_chain` del Validation reúsa el gate (si el wiring es trivial) o función exportada + CLI `atlas security gate <name>` (comando `atlas security` nuevo).
- Tests: typosquatting detectado, postinstall detectado, clean → Pass, determinista.

**Entregable:** extensible como OpenClaw pero seguro. KPI: acciones bloqueadas ≥99% antes de impacto (RFC 20).

---

## SECTOR C — Fuera de alcance (esta iteración)

- ed25519/minisign (crates nuevas — audit RFC 25 §11 previo; el checksum MVP cubre integridad).
- Ejecución real en Docker/Podman (container exec — infraestructura externa, follow-up).
- Socket/Snyk API integration (SaaS externos con API key — follow-up documentado).
- Telemetría de seguridad dashboard (RFC 18 §13 — HUD Phase 8 parcial).
- Vector KB encrypted-at-rest con SQLCipher (RFC 18 §9 — SQLite está bundled sin SQLCipher; follow-up con audit).

## SECTOR D — Siguiente paso operativo

1. Commit de este plan + RFC 20 update.
2. Sub-fase 7.0 (delegada) → revisar → commit → push.
3. Sub-fases 7.1–7.3 (delegadas) → revisar → commit → push.
4. Cierre de Fase 7 (gestor): RFC 20/RFC 18/26/README markers.
