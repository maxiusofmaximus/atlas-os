# 11 - Context Engine

El motor continuo que **mantiene al agente enterado del proyecto entero**. Es lo que evita que el modelo, al editar un archivo, olvide el resto.

---

## 1. Funciones

```
Context Engine
   │
   ├── analiza el proyecto
   ├── mantiene memoria arquitectónica
   ├── entiende el dominio
   └── actualiza el mapa interno
```

Está siempre activo en segundo plano, tanto en Modo Manual como Modo IA.

## 2. Fase de Descubrimiento

Antes de cualquier modificación, el Context Engine corre Discovery:

```
Proyecto
  ↓
Leer estructura de carpetas
  ↓
Leer documentación (README, docs/, ADRs)
  ↓
Leer arquitectura (schema.prisma, openapi, package.json, go.mod, Cargo.toml, pyproject)
  ↓
Detectar tecnologías (lenguaje / framework / runtime)
  ↓
Generar Project Map
  ↓
Crear mapa interno en Vector KB
  ↓
Ahora sí, empezar a editar
```

> Una IA no debería modificar un proyecto sin antes entenderlo.

## 3. Project Map

Salida canónica del Discovery, persistida en la Architecture Memory:

```yaml
project_map:
  backend:
    language: typescript
    framework: nest
    runtime: bun
    ports: [4000]
  frontend:
    language: typescript
    framework: next-app
    paths: ["apps/web"]
  shared:
    - packages/ui
    - packages/contracts
  api:
    spec: openapi.yaml
    transport: trpc
  database:
    engine: postgres
    orm: prisma
    schema: prisma/schema.prisma
  tests:
    - vitest
    - playwright
  ci:
    - .github/workflows/ci.yml
    - .lefthook.yml
  deployment:
    - docker-compose.yml
    - pulumi/index.ts
  architecture:
    style: modular-monolith
    layers: [domain, application, infrastructure, presentation]
  dependencies:
    critical:
      - "@prisma/client"
      - "@trpc/server"
      - "@effect/io"
    risky:
      - "lodash@4.17.21"     # flagged by Socket
```

## 4. Architecture Memory (permanente)

Cada módulo o cambio actualiza el grafo:

```
Componentes
Servicios
Dependencias
Capas
Patrones
Responsabilidades
```

Formato: grafo en SQLite con vecinos por archivo (graphs `nodes`, `edges`, `relations`). Cada nodo:
- tipo (component | service | module | layer | pattern | schema | endpoint | test),
- propietario (responsabilidad asignada),
- dependencias directas,
- patrón,
- última modificación.

Los agentes que van a tocar un nodo consultan primero el grafo.

## 5. Actualización incremental

- Cada `file.saved` triggers un diff-graph delta sin reindexar todo.
- Detección semántica vía **Tree-sitter** por AST en vez de texto. Patrón declarado en el documento: *"Tree-sitter, para comprender el código a nivel de AST en lugar de depender solo de texto."*
- Cambios que rompen una capa / un patrón marcan alerta para el Planning Engine.

## 6. Recuperación por consulta

Cualquier motor llama:
```
ContextEngine.project_map() -> ProjectMap
ContextEngine.who_owns(file) -> Node|null
ContextEngine.deps_of(component) -> Node[]
ContextEngine.affects_where(change) -> [Node]
ContextEngine.fresh() -> boolean
```

El Coding Engine jamás escribe sin mirar `who_owns` y `affects_where`.

## 7. "Memoria corta" del prompt

Sobre la lectura del grafo, el Context Engine inyecta en el prompt solo **la porción relevante**:
- 5 vecinos del nodo tocado,
- contratos afectados,
- tests adyacentes.
- patrón / patrón target confirmado por Research.

Esto garantiza que inclusive un modelo local de 7B puntualice la coherencia arquitectónica.

## 8. Políticas

- Si un cambio cruza dos capas prohibidas → bloqueo con alerta.
- Si un cambio toca schema.prisma o openapi.yaml → obliga revisión por Merger / Reviewer.
- Si un cambio afecta un test → marcar el test como "dirty", la Validation lo vuelve a ejecutar.

## 9. Versionado del Project Map

Si el usuario hace revert de Git, el Context Engine regenera el map desde el tree en lugar de diff. Soporta branches y worktrees del swarm.

## 10. Sub-fase 9.2 — definiciones de símbolos (M34, COMPLETO)

`src-tauri/src/context/ast.rs`: parse de definiciones (fn/class/struct/enum/trait/module/method) → `AstSymbol {kind, name, file, line}` con `validate()` (nombre no vacío, `line > 0`, extensión conocida) — alimenta el Skill Picker (8.1) vía `presence_boost` (+0.1) y el LSP (9.4) vía `confidence_for_symbol(base, present)`.

Audit RFC 25 §11 **APROBADO** (RFC 22 §7 AN-9.2): reutiliza el `tree-sitter 0.26` + `tree-sitter-rust 0.24` ya vendoreados por `codebase-graph` (RFC 28 §C) — cero crates nuevas. Feature `ast` (default off, alias de `codebase-graph`): con el feature, `extract` usa walk AST real sobre Rust (los `fn` bajo `impl_item` se tipan `method`); sin el feature, heurístico std-only determinista (Rust + Svelte/TS/JS, prefijos `pub`/`export`/`async`/`declare`). Svelte usa heurístico en ambos modos (el grammar taguea `<script>` como `raw_text`, RFC 22 §10.2).

Persistencia M34 (schema 33): tabla `ast_symbols` (`UNIQUE (file, name, kind, line)`, CHECK de `kind` espejo de `AstSymbolKind::tag`, `line > 0`) + `Journal::record_ast_symbol` (upsert idempotente RFC 02 §3.1.2) / `ast_symbols_for_file` (ordenado por línea) — fuente de `who_owns` y `affects_where` por símbolo en 9.4.
