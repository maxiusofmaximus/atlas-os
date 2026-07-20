# 15 - Repair Engine

El motor que **se arregla solo**. Convierte cada error en un micro-ciclo de reparación, sin esperar al usuario.

---

## 1. Flujo

```
Error
   ↓
Clasificar
   ↓
Analizar
   ↓
Buscar origen (root cause)
   ↓
Proponer reparación
   ↓
Aplicar (via Coding Engine)
   ↓
Validar otra vez (via Validation Engine)
```

## 2. Clasificación

Tipos:
| Tipo | Acción |
|---|---|
| Sintaxis/formato | auto-fix (Biome auto-fix, rubocop -A, clippy --fix) |
| Type-check | pedir Coding amendment con nodo contraint |
| Test fallo | leer diff, pedir Coding revise, retry |
| E2E | Playwright snapshot -> Coding |
| Dead code | Knip auto-remove |
| Vuln (Snyk/Socket) | bump de versión + retry test |
| Config | reprovisión Pulumi / OpenTofu |
| Rating confidence bajo | escalar a Planning para replan+research |

## 3. Búsqueda de root cause

El Repair Engine:
1. Lee el stage summary.
2. Adapta el graph del Context Engine: who_owns(file), affects_where(change).
3. Pregunta al Journal `last_decisions()` y `last_research_runs()` como referencias.
4. Identifica si el error se repite → Learning Engine debería haber generado una regla.

Si encuentra una regla ID en el Learning Engine, **la aplica directamente** (patrón cacheado).

## 4. Límites

- Para cada file y stage: hasta **3 intentos** consecutivos.
- Tras 3: planning replanifica.
- Tras otras 3 fallos en la mission: escalar a usuario en el Agent Console.

Si el profundo estancamiento indica un bug estructural, never entra en bucle infinito.

## 5. Politica de presupuesto

Repair consume el mismo presupuesto que Coding:el Model Orchestrator puede preferir modelos rápidos para "loop de bug trivial" y reservar front-tier para el análisis root cause complejo.

## 6. Salida

Cada Repair Run guarda en Journal:
- error_id
- root_cause
- técnica aplicada
- diff
- tiempo
- exito
Si exitoso, emite event `repair.applied`. Si fallo, `repair.failed`. El Learning Engine ambos insertan en el repositorio de patterns.

## 7. No es único fix del usuario

El usuario puede en cualquier momento **editar manualmente** dentro del Modo Manual: el cambio corre la misma Validation; y si pasa, también retroalimenta el Learning. El agente ahorra trabajo a la mayoría que prefiere no pelear con los detalles.
