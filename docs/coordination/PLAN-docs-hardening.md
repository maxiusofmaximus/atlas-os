# Plan de coordinación — Endurecimiento de documentación antes de construir (2026-10-06)

Autor: Project Lead. Decisiones delegadas por el operador ("tus decisiones son las mías").

## Problema

El operador detecta que la documentación no es fiable para construir el producto:

1. La referencia de inspiración UI/UX (~425 entradas en `Atlas OS/research/62`) nunca se revisó para UX; RFC 66 se apoya en 17 de ~88–100 candidatos.
2. Las funcionalidades no están bien documentadas por el Researcher.
3. Los aspectos de UI no están documentados de forma construible (pantallas, estados, componentes, interacciones).
4. Hay documentos que no coinciden con el código.

## Decisión

No se inicia FASE 10 (implementación UI) ni nueva funcionalidad hasta cerrar documentación fiable. Esta ronda es SOLO documentación, inventarios y auditoría. Nada de esto toca código.

## Reglas comunes (obligatorias para todos)

- No modificar `src/` ni `src-tauri/`. No instalar dependencias. No `git commit`, no `git push`: el Project Lead integra y commitea.
- Escribir solo en tus rutas de salida (abajo). No editar archivos de otro rol. Si necesitas un cambio en un archivo ajeno, repórtalo.
- Procedencia en cada afirmación: **[O]** observado en este repo esta sesión, **[Os]** observado en vivo en la fuente citada (URL), **[I]** inferido o de memoria, **[P]** pendiente. Sin fuente no hay afirmación. No inventes proyectos, cifras ni funciones.
- Evidencia primero: comandos ejecutados, URLs consultadas, archivos creados. No declares terminado si un criterio de aceptación no se cumple; di qué falta.
- Pnpm, nunca npm. Sin comentarios en código (aplica si ejecutas alguna prueba que genere código temporal: en el scratchpad, no en el repo).
- Entrega parcial por lotes: guarda el archivo tras cada lote para que el trabajo sobreviva a un corte.

## Rutas de salida (disjuntas)

| Rol | Salida |
|---|---|
| Researcher | `Atlas OS/research/64 - UX reference catalog.md` + `Atlas OS/research/ux-catalog/*.md` |
| UI/UX Designer | `Atlas OS/67 - UI Specification.md` + `docs/design/*` (solo los suyos) |
| Builder | `docs/audit/backend-capabilities.md` |
| Builder-2 | `docs/audit/frontend-current-state.md` |
| Builder-3 | `docs/audit/rfc-vs-code.md` |
| Todo Updater | `docs/TODO.md` |

## Dependencias

```text
Builder, Builder-2, Builder-3 (inventarios del código, en paralelo)  ──┐
Researcher (catálogo completo → deep-dive UI-relevante)               ──┼→ UI/UX Designer (spec final) → Project Lead (revisión)
                                                                        │
Todo Updater (consolida TODO; se actualiza al cierre de cada rol)  ←───┘
```

El Designer puede empezar de inmediato con la plantilla y el esqueleto de la spec, y rellena con lo que entreguen Researcher y Builders.

## Criterio de cierre de la ronda

- Cada entrada de RFC 62 clasificada (UI-relevante sí/no, categoría, motivo) con recuento exacto.
- Cada entrada UI-relevante con ficha: funciones, patrones UI, estados, atajos, fuente [Os].
- Spec de UI construible por pantalla (RFC 67), sin ambigüedad y trazable a RFC 24/65/66 y a la API real.
- Inventario de código y lista de discrepancias RFC↔código con severidad.
- TODO central coherente con roadmap, código, tests y git.
