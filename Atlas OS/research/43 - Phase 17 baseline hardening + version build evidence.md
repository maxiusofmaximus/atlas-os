# 43 - Phase 17: Baseline de verificación, higiene de lint y evidence fix

## Objetivo

Tras el cierre + hardening de Phase 15/16, fijar una línea base verificada y corregir
la deuda silenciosa que dejaba `pnpm lint` en rojo.

## Lo encontrado en la auditoría

- `pnpm lint` fallaba por `prettier --check`: el propio workflow `release-android.yml`
  (Phase 15) y `.dependency-cruiser.cjs` no cumplían formato — el gate de formato no
  era usable como señal del repo.
- El header del HUD seguía en `v0.1.0` (fix de Phase 15 pendiente de build).
- Sin corrida local reciente de `cargo test` tras los commits de cierre.

## Implementación

1. `pnpm exec prettier --write .github/workflows/release-android.yml .dependency-cruiser.cjs`
   → verificado con `pnpm arch` (28 modules, 40 deps, sin violaciones).
2. `.prettierignore` extendido con los archivos de trabajo del operador
   (`decision_phase15.md`, `session-ses_*.md`, `siguiente_paso*.md`) — no rastreados, no distribuidos.
3. Verificación del fix del header: `pnpm build` → `build/_app/immutable/nodes/...js` contiene `h(K,"v0.1.1")` (evidence).
4. `cargo test --lib` → **1103 passed, 0 failed** (baseline local).

## Criterios satisfechos

- `pnpm lint`: eslint ✅ + prettier ✅
- `pnpm check`: 0 errors 0 warnings
- `pnpm arch`: sin violaciones
- `cargo test --lib`: 1103 ok
- Header HUD: v0.1.1 compilado

## Fuera de alcance (no tocado, correcto)

- `journal/mod.rs` 98 KB / `schema.rs` 80 KB: refactor recomendado para Phase 18 —
  requiere spec propia (división por submódulos con tests de `journal/tests.rs` como red).
- Layer de Laya: Phase 13 sigue BLOQUEADA (re-audit fechado en `research/42`).
