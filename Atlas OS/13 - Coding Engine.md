# 13 - Coding Engine

Donde **se escribe el código**. Pero no "cualquier código": bajo arquitectura decidida por Planning, evidencia de Research, restricciones de reglas, asociado al Skill Graph y validado incrementalmente.

---

## 1. Funciones

```
Coding Engine
   ├── genera código
   ├── mantiene arquitectura
   ├── aplica patrones
   └── escribe documentación
```

## 2. Contrato

Input: `Plan + Step + Skills(approved) + ContextMap`.
Output: `Diff(s)` a aplicar. La aplicación pasa por la Validation Engine antes de consolidarse.

## 3. Restricciones (reglas, no consejos)

Tomadas del harness original y traducidas a checks:

- Antes de cualquier archivo, validar el diagrama C4 / texto plano si la decisión es nueva.
- MSW mocks primero si se introduce una llamada a API nueva.
- Zod para todo input externo (TS).
- tRPC/ts-rest para endpoints TypeScript end-to-end.
- Effect para lógica con errores declarados.
- Prisma (o equivalente nativo) si se toca la DB.
- TypeDoc comments en cada módulo nuevo.

Estas reglas son **Skills** con `engine=Coding`. El motor las invoca automáticamente; no son "sugerencias".

## 4. Native por lenguage (Capability Resolver)

Si el proyecto no es TypeScript:
- Python → Pydantic + SQLAlchemy 2.0 + Ruff + MyPy + pytest.
- Go → go-zero / gin + validator/v10 + golangci-lint.
- Rust → serde + sqlx + clippy + cargo-deny.
- Java/Kotlin → Spring + Jackson + ArchUnit + Spotless.
- C# → FluentValidation + EF Core + TUnit + Roslyn analyzer.

El Coding Engine nunca usa Prisma para Python ni Zod para Go.

## 5. Auto-documentación

Cada cambio importante exige doc del módulo (AdR / micro-RFC). TypeDoc para TS, Sphinx para Python, godoc para Go, etc. Sin doc → Validation falla.

## 6. Documentación no solo formal, sino explicativa

El Coding Engine **escribe** documentación arriba del cambio:
- qué hacía,
- qué hace ahora,
- por qué se tocó,
- evidencia (Research refs),
-Decision de riesgo.
La UI lo muestra en el Agent Console para que el humano lo apruebe con un vistazo.

## 7. Patterns y bienas prácticas

Va al circuito de Skills de lang/framework específicas. La Skill Graph expone skills como:
- `react-query-pagination`,
- `effect-dataflow`,
- `trpc-context-binding`,
- `pydantic-v2-dataclasses`,
- `clippy-rules-custom`, etc.

No se instalan todas: se iluminan/grisan según relevancia (ver `06 - Skills.md`).

## 8. Diffs, no archivos enteros

El Coding Engine siempre opera con **diffs**. No reescribe archivos. Esto porque:
- El Reviewer valida diffs.
- El Merger fusiona diffs dentro del swarm.
- El Journal los persiste.
- El Learning los usa para extraccción de patrones.

## 9. Pacing económico

El motor pide al Model Orchestrator:
- modelo bueno para la **diseño** inicial,
- modelo rápido para repetir diffs similares,
- modelo local para reformatos.

No derrocha tokens frontier en tareas triviales.

## 10. Subroles

Dentro del Coding Engine:
- BackendCoder
- FrontendCoder
- DatabaseMigrator
- IaCCoder (Pulumi / OpenTofu)
- DocsWriter

El Swarm los reparte.
