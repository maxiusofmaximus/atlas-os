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
