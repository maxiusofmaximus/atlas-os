# 14 - Validation Engine

Validación **incremental**: cada `file.saved` dispara el pipeline. Si algo falla, no se permite commit. El agente recibe **restricciones**, no sugerencias.

---

## 1. Pipeline incremental

```
Guardar archivo
   ↓
Biome
   ↓
TypeScript / TypeCheck-equivalente (Capability Resolver)
   ↓
Tests unitarios afectados
   ↓
Playwright / E2E (si se toca UI)
   ↓
Knip (dead code)
   ↓
Socket (si se instala alguna dependencia)
   ↓
Snyk (si se toca lockfile)
   ↓
Docker / Testcontainers (si se toca config)
   ↓
Semgrep / CodeQL (si se añade seguridad crítica)
   ↓
Dependency-cruiser (si_contenta límite de mod)
   ↓
Commit permitido
```

Cualquier FAIL bloquea y dispara Repair.

## 2. Etapas del pipeline

### Lint + Format (rápido)
- Biome en TS (reemplaza ESLint + Prettier).
- Equivalente nativo en otros lenguajes: Ruff (Python), golangci-lint (Go), clippy (Rust), checkstyle (Java), Roslyn analyzer (C#).

### Type-check
- `tsc --noEmit` (TS), `mypy` (Py), `go vet` (Go), `cargo check` (Rs).

### Tests
- Solo tests afectados (selección por graph delta y heurística de Coverage delta).
- Vitest/Jest para TS, pytest para Python, etc. Cobertura medible, no opcional.

### E2E
- Playwright con **modo compacto** (`[E21]`, `[E26]`) para no saturar contexto.
- Corre en Background job.

### Dead code / límites
- Knip en TS.
- dependency-cruiser para limitar dependencias entre capas.

### Supply-chain & Security
- Socket para telemetría de paquetes.
- Snyk para CVEs.
- Semgrep para reglas custom; CodeQL para flujo profundo (en `security_gate=true`).

### Infraestructura
- Docker con Testcontainers para servicios reales.
- OpenTofu/Pulumi `plan` sin aplicar.

### Hooks definitivos
- Lefthook bloque pre-commit si todo lo anterior no pasa.

## 3. Stage summaries canónicos

Cada stage emite un summary estructurado, no logs:
```yaml
stage: biome
status: FAIL
elapsed: 240ms
findings:
  - file: "src/ui/Button.tsx:24"
    rule: no-unused-vars
    suggestion: "Remove unused prop"
  - file: "src/api/routes.ts:8"
    rule: import/order
    autoFixable: true
summary: "2 issues, 1 auto-fixed"
```
El agent consume el summary: nada de strings crudos.

## 4. Throttling

Si una etapa falla, no avanzan las siguientes. El Repair Engine recibe el summary, decide y reprocesa.

## 5. Eventos
- `validation.running`
- `validation.stage.<name>.pass|fail`
- `validation.pass`
- `validation.fail`

## 6. Métricas

Cada run registra: duración por stage, valor de cada fixing, tokens ahorrados por modo compact Playwright.

## 7. Modos

- **Strict** (por defecto): nada pasa a main sin 100% verde.
- **Loose** (override del user): permite commit soft sin Tests, manteniendo Lint+TypeCheck.Útil para trabajo exploratorio.
- **PR mode**: obliga a E2E completo y tests de rama completa antes de abrir PR.

## 8. Combinación con Repair

Si una stage falla twice, Validation Engine emite `validation.fail.critical` y Repair toma control.

## 9. Badge del archivo

Cada archivo abierto en el editor muestra un badge con el último estado (verde / amarillo / rojo) tan pronto como Validation termina una pasada.

## 10. Evidence-gated done (RFC 30 §2.1, patrón `qkal/Canny`)

Ningún agente puede aterrizar "done" sobre una afirmación: el gate es determinista (sin modelo, sin IO) y solo los hechos bloquean.

- **Stage terminal `EvidenceGate`** (último en `StageKind::pipeline_order()`): si el diff toca código, debe traer evidencia propia — narrativa (qué/cómo/por qué, ≥10 caracteres) más un artefacto (fichero de test tocado, `research_refs` o `risk_decision`). Sin ambas cosas → `Fail` (bloquea commit y dispara Repair con `ErrorClass::TestFailure`); con solo una → `Warn`. Sin código (docs, imágenes, lockfiles) → `Pass` directo; en modo `Loose` el gate es advisory (`Skipped`).
- **Hook `evaluate_done_claim(report, claim)`** (`validation::evidence`): el host lo llama antes de aceptar un "done". Exige IDs coincidentes, `outcome == Pass`, claim verificable (≥8 caracteres), cita al report verde y al menos un check ejecutado (`test/typecheck/lint/e2e`; en modo PR el `e2e` es obligatorio). `ManualNote` solo nunca cuenta. Devuelve `Allowed` o `Blocked { reason, missing }` — el `reason` se muestra verbatim como hace Canny.
- **Supervisor**: evento `DoneClaimed { report_id, claim, evidence }` → `Done` + `MarkDone` solo con claim y evidencias no vacías; si no, `BlockDone { reason }` sin salir de `Verifying`. `ValidationPassed` se mantiene por compatibilidad; los hosts nuevos deben preferir `DoneClaimed`.
- **Ledger**: el Journal existente (`diffs` + `validation_reports` + `journal_events` + `audit_log` hash-chained) es el ledger append-only; `collect_diff_evidence` deriva los items desde los stages en `Pass`, así que cualquier `replay` re-deriva el mismo veredicto.

Estado: implementado Phase 2.5+ (cierra el gap "Evidence-gated done ✗" de RFC 30 §2.1).
