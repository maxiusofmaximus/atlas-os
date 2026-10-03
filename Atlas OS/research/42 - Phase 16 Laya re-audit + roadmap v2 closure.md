# 42 - Phase 16: Laya re-audit + cierre del Roadmap v2 (plan y evidencia)

**Estado este documento:** IMPLEMENTADO / TESTED (verificación documental) / VERIFIED parcial en Git.

## Objetivo

Cerrar formalmente el Roadmap v2: verificar que Phase 11–15 están cubiertas, ejecutar el
re-audit periódico del bloqueo de Phase 13 (criterio documentado) con evidencia objetiva,
publicar el veredicto y actualizar roadmap/índice/README.

## Contexto y problema

El Roadmap v2 (`research/40`) cubre Phases 11, 12 y 13. Phase 11 y 12 están COMPLETAS,
Phase 15 (distribución/CI/estrategia de firma) se implementó y cerró con evidencia física.
Phase 13 (Laya real) está bloqueada por criterio upstream (`20 - Roadmap.md` §Fase 13).
El roadmap exige que "el gestor re-audita periódicamente" — eso no se había ejecutado con
evidencia fechada. Phase 16 formaliza ese trabajo.

## Alcance

- Ejecutar el re-audit de `laya` contra crates.io + GitHub (API pública) y registrar evidencia fechada.
- Emitir veredicto del criterio (≥2 mantenedores, >0.2.x, fix rand/tokenizers).
- Marcar Phase 13 como permanece BLOQUEADA o habilitada, según veredicto.
- Añadir §Fase 16 en `20 - Roadmap.md` y fila en `26 - Index & Cross-References.md`.
- Actualizar README con el estado v2.

## Fuera de alcance

- No implementar inferencia Laya (candle/compaction/winnow) mientras el criterio no se cumpla.
- No hacer fork, downgrade ni workaround de Laya.
- No crear Phase 17/18 ni RFC 42 hasta auditar el estado real tras Phase 16.
- No modificar código funcional (sin cambios en `src/` ni `src-tauri/`).

## Arquitectura afectada

Ninguna de código. Solo documentación: `Atlas OS/20 - Roadmap.md`, `26 - Index`, `README.md`, este doc.

## Dependencias y prerequisitos

- Acceso a internet para consultar crates.io y GitHub API.
- Criterio de Phase 13 sin cambios desde `research/40`: `laya` >0.2.x o mantenedores ≥2 o fix de rand/tokenizers.

## Decisión técnica

- El re-audit se ejecuta como Phase 16 en sí misma (auditoría documental), sin tocar código.
- Si el criterio sigue sin cumplirse, Phase 13 continúa BLOQUEADA y se programa el próximo re-audit; no se declara una Phase 17 inventada.
- Las futuras Phase 17+ requieren decisión del operador + RFC propio.

## Criterios de aceptación (verificables)

1. Evidencia fechada del upstream `laya` (versión, mantenedores, dependencias rand/tokenizers) guardada en este documento.
2. Veredicto explícito del criterio, confirma o refutatorio.
3. `20 - Roadmap.md` con §Fase 16 y Fase 13 anotada con el re-audit fechado.
4. `26 - Index` con fila para este documento.
5. README actualizado.
6. `pnpm lint` y `pnpm check` PASS (cambio solo documental → no rompen código).

## Estrategia de pruebas

- Verificación documental (este archivo + roadmap + README coherentes entre sí).
- `pnpm exec prettier --check` sobre los archivos tocados; `pnpm check` para confirmar que no se rompió el build de docs.

## Evidencia requerida

- Este documento contiene la evidencia fechada del re-audit.
- Commit convencional con el cierre de este milestone.

## Definición de DONE

- Re-audit ejecutado con evidencia objetiva y fechada.
- Veredicto publicado (fiel al criterio) y roadmap/índice/README actualizados.
- Commit + push en main.

---

## Evidencia del re-audit — 2026-10-02

Fuente: crates.io API (`https://crates.io/api/v1/crates/laya`) y GitHub API (`aovestdipaperino/laya-rust`).

| Criterio | Valor observado | Veredicto |
|---|---|---|
| `laya` > 0.2.x | `newest_version = 0.1.1`, updated_at 2026-09-20 | ❌ NO cumplido |
| Mantenedores ≥ 2 | 1 contributor en GitHub | ❌ NO cumplido |
| Fix de `tokenizers` | `Cargo.toml` upstream ya declara `tokenizers = "0.21"` | ✅ Resuelto parcialmente (problema original cerrado upstream) |
| Fix de `rand` | `Cargo.toml` upstream ya no declara dependencia directa de `rand` | ✅ Resuelto parcialmente |
| Estado del repo | pushed_at 2026-09-30, 12 stars, 1 open issue | Activo pero solo-autor |

**Veredicto:** el criterio de desbloqueo (`20 - Roadmap.md` §Fase 13) **NO se cumple** (ni versión >0.2.x ni ≥2 mantenedores). **Phase 13 permanece BLOQUEADA upstream.** El sub-problema rand/tokenizers parece haber mejorado, pero eso solo abre la posibilidad de un futuro desbloqueo cuando versión/mantenedores cambien. Próximo re-audit: periódico, no antes de un bump de versión o nuevo maintainer visible.

**Roadmap v2 exhausted:** Phase 11 ✅, Phase 12 ✅, Phase 15 ✅, Phase 14 ✅ (distribuida dentro de v2), Phase 13 ❌ bloqueada. No quedan fases v2 por ejecutar.
```
