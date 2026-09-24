# 30 - Ecosystem & Jev Audit

> Audit del ecosistema de referencia aportado por el operador (Sep 2026): la lista "Jev" de Charlie Hills (20 repos, tweet `2102044071368012140`), skills de diseño (taste-skill, awesome-design-md, img2threejs), skills de ingeniería (mattpocock/skills, grill-me), orquestadores (munder-difflin, deepseek-harness, HydraFusion), documentación (anydoc, Context7Max) y estándares (zod, agents.md, storybook). Extrae los patrones concretos que Atlas OS puede cerrar y los prioriza contra el roadmap. Este RFC es el input de Phase 3+; complementa RFC 27 (orquestadores CLI-first) y RFC 29 (Genspark cloud-first).

---

## 1. Contexto y motivación

Tras cerrar Phase 2 (sub-fases 2.0→2.4, commit `302b170`), la investigación Round 6 (RFC 22 §13) validó la tesis de orquestación adaptativa contra HydraFusion (GitHub Copilot) y catalogó 23 referencias externas. Este RFC:

1. **Puntúa los 20 repos "Jev"** (audit ejecutado con el script `scripts/audit-jev-repos.mts` de Context7Max — .md count, SKILL.md count, estrellas) y extrae los patrones que Atlas OS puede absorber.
2. **Cataloga el ecosistema de referencia** completo con su destino en el roadmap.
3. **Prioriza por dependencia** para Phase 3+.

## 2. Audit Jev — 20 repos puntuados

"Jev" es un **modelo de juicio pequeño/barato/rápido** ("System One" de TypeSafe AI): el patrón dominante es usar un modelo de frontera pequeño para decisiones rápidas por turno (routing, evaluación de tool results, compaction, review) en vez de un modelo grande en cada paso. Auditoría ejecutada Sep 2026 (fetched vía GitHub API, orden por estrellas):

| Repo | ★ | .md | SKILL.md | Qué es | Patrón para Atlas OS |
|---|---|---|---|---|---|
| `browser-use/jev-ultrafast` | 19209 | 6 | 0 | Web agent "fastest and cheapest" | Routing por coste/latencia (Phase 2 `CostBased` strategy) |
| `tamaratran/fast-jev-compaction` | 6604 | 2 | 0 | Claude Code plugin: compaction summary con Jev decisions | **Phase 5** (Learning + Compression): compaction de history con modelo barato |
| `vercel-labs/json-render` | 18200 | 159 | 31 | Generative UI framework | **HUD v2** (Phase 8): UI generativa data-driven |
| `jarrodwatts/jev-trader` | 2203 | 6 | 0 | Una decisión de trade por Monad block | Decisión periodificada (Execution Supervisor cadencias) |
| `lahfir/agent-desktop` | 1603 | 108 | 3 | Computer use fiable en desktop, **Rust** | **Execution Supervisor / Swarm**: control de desktop por agente |
| `devagrawal09/jev-review` | 582 | 1 | 0 | Staged code-review workflow + dashboard local | Rol `Reviewer` (Phase 4 Swarm) + HUD dashboard |
| `fhshaik/typesafe-mario` | 379 | 1 | 0 | Jev agent juega Super Mario desde emulator state | Diversión / modos de trabajo (gamificación) |
| `jkudish/jev-mcp` | 317 | 5 | 1 | Juicios typed fast/cheap de Jev como **MCP tools** | **RFC 07 MCP**: expender juicios como tools MCP |
| `itsmostafa/typesafe-mcp` | 286 | 7 | 0 | MCP connector para evaluar anything fast/cheap | RFC 07 MCP (evaluación rápida) |
| `0xNatoshi/jev-codex-router` | 258 | 117 | 8 | **Per-turn model & reasoning routing** para Codex, driven by Jev | **Análogo directo del Orchestrator Phase 2** (Mf strategy + classifier) |
| `RomanSlack/jev-drone` | 161 | 5 | 0 | Drone autónomo camera-only en MuJoCo con judgment model | Diversión (sensor-fusion con modelo pequeño) |
| `monteduro/killmyidea` | 146 | 2 | 1 | Describe tu idea, Jev decide: kill/fix/ship | **Diversión / modos de trabajo** (jurado de ideas) |
| `jexp/neo4jev` | 125 | 3 | 0 | Jev navega grafo Neo4j con classifier de vecinos | Knowledge graph traversal (RFC 09/16 + graphify) |
| `irfndi/prism-liquidity-agent` | 96 | 52 | 8 | LP trading agent autónomo con backtested strategies | Swarm autónomo con estrategias (Phase 4) |
| `GhalebDweikat/winnow` | 78 | 4 | 1 | **Context sieve calibrado**: cada tool result juzgado por System One antes de entrar en context | **Context Engine** (RFC 11): compresión + lost-in-the-middle |
| `qkal/Canny` | 73 | 4 | 0 | Hooks deterministas que exigen **evidencia antes de "done"** | **Validation Engine** (RFC 14): quality gates deterministas |
| `ellipsis-dev/blink` | 69 | 2 | 0 | Codebase search powered by Jev | Context Engine search (RFC 11) |
| `sharziki/semdecide` | 53 | 7 | 0 | Typed semantic decisions para pipelines Unix y CI | CLI validation (RFC 14 stages) |
| `AkashPriyadarshii/jev-curate` | 45 | 11 | 0 | Dataset sifter high-throughput, **Rust streaming** | Learning Engine (RFC 16): curación de datos |
| `emrickgarrett/OneVOneJev` | 30 | 1 | 0 | 1v1 quickscope arena — Three.js + System One | Diversión (arena de comparación de modelos) |

### 2.1 Patrones fundacionales extraídos del audit Jev

| Patrón | Qué es | Estado en Atlas OS |
|---|---|---|
| **System One judgments** | Modelo pequeño/barato para decisiones rápidas (routing, eval, compaction) | ✓ (parcial) — `weak_model` (Aider tri-model) + classifier thresholds + affinity 2.4. Falta: usarlo para compaction y tool-result judging |
| **Per-turn routing** | Decisión de modelo/routing por turno, no por sesión | ✓ — `RoutingStrategy::Mf { threshold }` + classifier confidence (sub-fase 2.4) |
| **Evidence-gated done** | Hooks deterministas que exigen evidencia antes de "done" | ✗ — Validation Engine stages existen pero sin gate determinista de "done con evidencia" |
| **Context sieve** | Juzgar cada tool result antes de entrar en context | ✗ — Context Engine reordena (lost-in-the-middle) pero no filtra por juicio |
| **Compaction decisions** | Compaction de history con modelo barato, no truncado | ✗ — Phase 5 (Learning + Compression) |
| **Generative UI** | UI data-driven generada desde el runtime | ✗ — HUD v2 (Phase 8); petgraph/GraphView es el precursor |

## 3. Ecosistema de referencia — destino por frente

| Referencia | Qué aporta | Destino |
|---|---|---|
| **Context7Max** (propio, self-hosted) | Docs ilimitadas + ejemplos verbatim + skills + MCP registry (Supabase+Vercel+GitHub) | **Phase 3 Research Engine**: fuente primaria de docs (fallback Context7 MCP). CLI/API externa — sin crate nueva, cumple RFC 25 §11 |
| **anydoc** (firecrawl) | Word/PPT/Excel/ODF/RTF/EPUB/CSV/PDF → Markdown en **Rust** con bindings | **Phase 3**: ingestion de documentos del Research Engine (feature-gated) |
| **grill-me** (mattpocock) | Stress-test de planes vía questioning sistemático branch-by-branch | **Phase 3 Planning**: skill bundled + gate de confidence |
| **mattpocock/skills** | Skills for real engineers (del `.agents` de Mattpocock) | Brecha D bundled skills |
| **taste-skill** (9 skills) | Anti-slop design: brief inference, GSAP skeletons, redesign audit | Brecha D bundled skills + HUD v2 |
| **awesome-design-md** | DESIGN.md de marcas → UI matching por agentes | Brecha D bundled skills: modos de diseño |
| **img2threejs** | Image → 3D procedural models en TypeScript | Modo de trabajo 3D (diversión); referencia para specs visuales |
| **archify** | Diagramas de arquitectura/workflow/sequence self-contained HTML | Brecha D bundled skills + output de Phase 3 (reports) |
| **agency-agents** (154k★) | Roles con personalidad/procesos/deliverables + app nativa | **Phase 4 Swarm**: presets de roles con personality |
| **munder-difflin** | Office of clones: 2D office floor, mailbox + memoria por agente, Command Center divertido | **Phase 4 Swarm**: floor visual + modos de trabajo divertidos |
| **OpenMontage** | 12 production pipelines + 100 tools + 700 skills | Patrón de **catálogo por dominio** (work modes por dominio) |
| **deepseek-harness** | Everything-is-a-plugin sobre Cordis (arxiv `2608.25512`) | RFC 06/07: evaluar plugin architecture para Phase 5+ |
| **HydraFusion** (GitHub Copilot) | Orquestación adaptativa validada | Evidencia RFC 22 §13; narrativa RFC 04 §3, RFC 00 §7.2 |
| **OmniRoute** | Gateway 359 providers (ya en stack) | Ya integrado (RFC 28 §H) |
| **orca / herdr** | ADE fleet / runtime (ya auditados RFC 27) | Phase 8 (desktop/mobile/remote) |
| **storybook** | Workshop UI components | Referencia HUD component docs (Phase 8) |
| **zod** | Validación TS schema-first | Ya en filosofía Validation (RFC 20 Fase 0) |
| **agents.md** | Estándar de contexto para agentes | Ya adoptado (AGENTS.md del repo) |
| **omarchy** | Opinionated Linux setup | Install experience Linux (Phase 8) |
| **MobiAI-Core** | Familia de proyectos del operador | Referencia de onboarding/testing |
| **NavMeshPlus** | Unity NavMesh 2D pathfinding | Referencia para el 2D office floor del Swarm |
| **Jev repos** (20) | Ver §2 | Pattern absorción (§2.1) |

## 4. Priorización por dependencia

```
Context7Max ─┐
anydoc ──────┼─► Phase 3 Research Engine (inmediato)
grill-me ────┘

System One judgments ─┐
Evidence-gated done ──┼─► Phase 2.5+/Phase 3 (gaps del audit Jev)
Context sieve ────────┘

taste-skill ─┐
design-md ───┼─► Brecha D bundled skills (RFC 29 §D)
archify ─────┼─►   (30 skills out-of-the-box)
mattpocock ──┘

agency-agents ─┐
munder-difflin ┼─► Phase 4 Swarm (floor 2D + roles con personalidad)
NavMeshPlus ───┘

deepseek-harness ──► Phase 5+ (plugin architecture eval)
Generative UI ─────► Phase 8 HUD v2
```

**Orden recomendado de implementación (por return/effort):**

1. **Context7Max integration**: Effort M — desbloquea el Research Engine completo con docs ilimitadas sin cuota. Proyecto propio del operador — sin dependencias externas nuevas.
2. **anydoc ingestion**: Effort M — Rust nativo (crates.io `anydoc`-equivalente o SDK), feature-gated. Complementa §E Firecrawl para documentos locales.
3. **grill-me + mattpocock bundled**: Effort S — quick win, skills bundled.
4. **Evidence-gated done (Canny pattern)**: Effort M — Validation Engine gates deterministas sobre el pipeline existente.
5. **System One compaction (fast-jev-compaction pattern)**: Effort M — Phase 5, `weak_model` hace compaction de history.
6. **Context sieve (winnow pattern)**: Effort L — Context Engine filter por juicio.
7. **Bundled skills design (taste + design-md + archify)**: Effort M-L — brecha D.
8. **Phase 4 Swarm modes (agency-agents + munder-difflin)**: Effort L — floor 2D + presets de roles.
9. **Plugin architecture eval (deepseek-harness)**: Effort L — Phase 5+, decisión documentada en RFC 06/07.

## 5. Cross-references

| Frente | RFC principal | RFCs colaterales |
|---|---|---|
| Context7Max | 10 Research Engine | 07 MCP, 22 §13 |
| anydoc | 10 Research Engine | 28 §E |
| grill-me / mattpocock | 12 Planning, 06 Skills | 23 Prompt |
| Evidence-gated done | 14 Validation | 19 Supervisor |
| System One compaction | 16 Learning | 04 §1 (weak_model) |
| Context sieve | 11 Context Engine | 09 Vector KB |
| Bundled skills design | 06 Skills, 29 §D | 17 UI |
| Swarm modes | 05 Swarm | 24 HUD |
| Plugin architecture | 06 Skills, 07 MCP | 25 §11 |

---

## 6. Status de este RFC

- **Versión:** 1.0 (audit completo, Sep 2026).
- **Tipo:** Informativo + priorización. No introduce APIs ni crates nuevas inmediatamente.
- **Método:** audit ejecutado con `scripts/audit-jev-repos.mts` de Context7Max (GitHub API, 20 repos) + fetch de 23 URLs (ver RFC 22 §13.5).
- **Cierre de frentes:** Cada frente se cierra con su propio sub-RFC o PR etiquetado. Quando los frentes de Phase 3 estén cerrados, este RFC pasa a status: **implemented**.

---

## 7. Fuentes de auditoría

- Tweet de Charlie Hills (lista Jev, 20 repos): `https://x.com/charliejhills/status/2102044071368012140`
- Script de audit: `Context7Max/scripts/audit-jev-repos.mts` (proyecto propio del operador)
- Los 20 repos listados en §2 (fetched Sep 2026 vía GitHub API)
- Las 23 referencias de §3 (fetched Sep 2026, URLs en RFC 22 §13.5)
