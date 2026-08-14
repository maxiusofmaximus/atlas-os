# 06 - Skills

Atlas OS no acumula skills. Las **comprime, las asocia a motores, las prioriza y las muestra al usuario**. Aquí inventamos algo que todavía no existe en ningún editor de código ni CLI.

---

## 1. Skill Graph (en lugar de lista plana)

Cada skill es un nodo en un grafo. Esquema:

```yaml
name: react-ui-expert
description: Generación de componentes React accesibles y performantes.
engine: Coding       # motor al que se asocia automáticamente
priority: 90         # 0-100, sugerida vs grisada
domain: frontend
language: typescript
framework: react
confidence: 0.88
estimated_tokens: 4200
estimated_time: 18s
dependencies:
  - tailwind-expert
  - accessibility-audit
compatible_models:
  - claude-sonnet-4.5
  - gemini-2.5-pro
  - gpt-4o
  - local-qwen2.5-14b-instruct
vector_embedding: [0.012, -0.43, ...]   # semántico
summary: "Componentes React accesibles, con tokens, animados, sin hooks abusivos."
conflicts:
  - vue-ui-expert
auto_generated: false
verified: true
version: 1.4.2
author: atlas-os
license: MIT
home: ./skills/react-ui-expert/
```

## 2. Asociación automática a un motor

Toda skill declara `engine`. El Kernel enruta:

| Skill | Motor |
|---|---|
| Skill de **planificación** | Planning Engine |
| Skill de **investigación** | Research Engine |
| Skill de **diseño / UI** | Coding Engine (subagente de UI) |
| Skill de **validación / tests** | Validation Engine |
| Skill de **seguridad** | Validation Engine + Security Layer |
| Skill de **infraestructura** | Coding Engine (subagente IaC) |
| Skill de **investigación previa** | Research Engine |
| Skill de **meta-razonamiento** | Reasoning Engine |
| Skill de **aprendizaje / auto-reglas** | Learning Engine |

El motor aloja todas las skills de su dominio y decide cuál disparar por **embeddings + prioridad**.

## 3. Pipeline de selección de Skills

```
Problema / Mission
   ↓
Embeddings de la intención
   ↓
Búsqueda semántica en Skill Graph
   ↓
Candidatos top-K
   ↓
Agrupar por dominio conflicto
   ↓
Eliminar duplicados (compresión)
   ↓
Resumir (una línea por skill)
   ↓
Ordenar por prioridad + relevancia
   ↓
Mostrar al usuario
```

Esto evita cargar 4000 skills: el sistema solo muestra las relevantes.

## 4. UI de Skills: iluminadas vs grisadas

Esta es **la idea-mejor** del documento original. La UI muestra un *lightbox* de skills:

```
UI
★ ★ ★ ★ ★  React UI Expert        ← iluminada (sugerida)
★ ★ ★ ★ ★  Tailwind Expert        ← iluminada (sugerida, alta relevancia)
★ ★ ★ ★    Accessibility           ← iluminada (sugerida, relevancia media)
★ ★        Motion                  ← grisada (relevancia baja)
★          Vue                      ← grisada (no sugerida, conflicto de dominio)
```

- Las sugeridas se **iluminan** (alta opacidad, color de marca).
- Las no sugeridas se **grisan** (opacidad baja, no seleccionables salvo override manual).
- El usuario puede **pinchar** ("usa esa para la UI y no esta") — el override persiste en el Journal y retroalimenta el Learning Engine.

## 5. Compresión / Evolución de Skills

El motor de Compresión corre en segundo plano (Learning Engine lo dispara):

```
120 skills
   ↓
Detección de similitud (cos > umbral)
   ↓
Fusionar embeddings / metadatos
   ↓
 Nueva Skill híbrida
   ↓
Eliminar redundancia
   ↓
Actualizar grafo de embeddings
```

Bajo esa presión el editor **mejora solo**.
- Las skills redundantes se marcan `deprecated`.
- La skill resultante hereda dependencias, modelos compatibles y descripción resumida.
- Se audita en un diff de Skills para que el usuario apruebe.

## 6. Detección de skills disponibles

Atlas OS escanea:
- `~/.opencode/skills/**` y `.opencode/skills/**` (estilo Claude)
- `~/.agents/skills/**` (estilo InsForge)
- MCPs que aportan *tools* (ver `07 - MCP.md`)
- CLIs expuestos como tools (ver `08 - CLI.md`)
- Skills instadas por el usuario explícitamente

Cada skill detectada se ingesta en el Skill Graph con `auto_generated: false`.

## 7. Skills auto-generadas

El Learning Engine puede **crear** skills a partir de patrones repetidos del usuario (una secuencia de comandos PowerShell, un flujo de commits, un patrón de refactor). Estas skills nacen con:
- `auto_generated: true`
- `verified: false`
- `confidence: 0.50`

Hasta que no se validan en uso (X ejecuciones con resultado correcto) no suben de `priority`.

## 8. Conflictos

`conflicts[]` indica skills que **no deben coejecutarse** sucesivamente:
- (`vue-ui-expert`, `react-ui-expert`)
- (`prisma-orm`, `drizzle-orm`)
- (`tailwind`, `css-modules`)

El Planner respeta conflicts y nunca programa dos skills conflictivas en el mismo objetivo.

## 9. Skills cerradas vs abiertas

Atlas OS soporta skills **abiertas** (en YAML/Markdown) y **cerradas** (paquetes firmados). Las skills cerradas requieren firma criptográfica (ver `18 - Security.md`) para instalarse — esto resuelve la crítica contra OpenClaw (skills sin sandbox).

## 10. Reemplazo de la lista plana original

El antiguo harness tenía una lista de ~18 herramientas (Zod, Prisma, Biome, Knip, Socket, Snyk, etc.). En Atlas OS, esas herramientas son **skills** dentro del Skill Graph, cada una:
- con `engine` asignado,
- con `compatible_models`,
- con `priority` derivada del dominio,
- sujetas a compresión y evolución.

El resultado: el mismo poder de combate-a-alucinaciones del harness anterior, sin paralizar al agente.
