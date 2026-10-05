# 03 - Engine Architecture

Atlas OS está formado por **nueve motores** independientes más una **capacidad transversal de razonamiento** (`§4` — integral, *no* un motor aislado). Algunos son continuos, otros reactivos. Esta especificación define responsabilidades, interfaces y eventos de cada uno.

---

## 1. Inventario de motores

| # | Motor | Tipo | Detalle |
|---|---|---|---|
| 1 | Context Engine | Continuo | `11` |
| 2 | Research Engine | Reactivo | `10` |
| 3 | Planning Engine | Reactivo | `12` |
| 4 | Coding Engine | Reactivo | `13` |
| 5 | Validation Engine | Reactivo incremental | `14` |
| 6 | Repair Engine | Reactivo | `15` |
| 7 | Learning Engine | Continuo + reactivo | `16` |
| 8 | Model Orchestrator | Reactivo | `04` |
| 9 | Execution Supervisor | Continuo | `19` |

**Capacidad transversal (no motor): Reasoning** — `§4`. No es un motor aislado; cualquier motor la invoca. Materializada en `orchestrator/aggregation/` (`moa.rs` + `majority_vote.rs` = MoA/voto; `council.rs` = debate multi-agente; `self_refine.rs`/`reflexion.rs`/`self_discover.rs` = self-reflection) y `orchestrator/reliability_gate.rs` (verificación + confidence gate). No existe —ni se requiere— un directorio `reasoning/` (reconciliación `research/61 §8.4`).

A estos se suman subsistemas transversales:
- **Swarm Coordinator** (`05`)
- **Skill Graph** (`06`)
- **MCP / CLI / Vector KB** (`07`, `08`, `09`)
- **Security Layer** (`18`)
- **UI / Command Center** (`17`)

## 2. Contrato común de un motor

Todo motor expone:

```
Motor {
  id: string
  status: idle | running | blocked | failed | done
  inputs:  Event[]
  outputs: Event[]
  policy:  Policy  // cuándo dispararse, prioridad, límites
  journal: JournalHandle
  api:     MotorAPI
}
```

### Eventos canónicos
- `engine.start`
- `engine.progress`
- `engine.need_input`
- `engine.blocked`
- `engine.success`
- `engine.failed`
- `engine.handoff`  // cede el control a otro motor

## 3. Flujo típico

```
Mission
   │
   ▼
[Planning]──▶[Research]──▶[Reasoning]──▶[Orchestrator]──▶[Swarm]
                                                              │
                                                              ▼
                                                        [Coding]
                                                              │
                                                              ▼
                          ┌─────────────────────────────── [Validation]
                          ▼                                       │
                       [Repair] ◀─────────────────────────────────┤
                          │                                       │
                          └──────────────▶ [Learning] ◀───────────┘
                                              │
                                              ▼
                                    [Execution Journal]
```

## 4. Reasoning — capacidad transversal (integral)

El razonamiento no es un motor aislado que se ejecuta una vez: es una capacidad que **cualquier motor puede invocar**. Métodos expuestos:

> **Materialización (verificado 2026-10, `research/61 §8.4`):** no hay módulo `reasoning/` (sería superficie sin profundidad). MoA/voto y debate viven en `orchestrator/aggregation/` (`moa.rs`, `majority_vote.rs`, `council.rs`); self-reflection en `orchestrator/aggregation/{self_refine,reflexion,self_discover}.rs`; verificación + confidence gate (Meta-Reasoning) en `orchestrator/reliability_gate.rs`.

| Método | Cuándo |
|---|---|
| Chain of Thought (CoT) | razonamiento lineal simple |
| Tree of Thoughts (ToT) | bifurcaciones, decisiones con varias ramas |
| Self-Reflection | después de cada output crítico |
| Debate (multi-agente) | bifurcaciones arquitectónicas |
| Verification | toda afirmación con evidencia exigida |
| Confidence Score | salida estándar de cualquier razonamiento |
| Meta-Reasoning | si Confidence < umbral, re-entrar en razonamiento |

### Meta-Reasoning checklist
```
Confidence 92%
¿Inventé algo?       → NO
¿Tengo evidencia?    → Sí
¿Estoy suponiendo?   → No
¿Existe incertidumbre? → Sí
   → Necesito investigar más
```

Este checklist está hardcodeado en el contract del Reasoning Engine. No es opcional.

## 5. Separación de responsabilidades

- **Planning** no escribe código. Solo piensa y divide.
- **Research** no decide. Solo recolecta.
- **Reasoning** no escribe archivos. Solo concluye y emite Confidence.
- **Coding** no decide arquitectura. Implementa lo decidido.
- **Validation** no arregla. Reporta.
- **Repair** no decide nueva arquitectura. Replanifica con Planning.
- **Learning** no toca código. Genera reglas.

Cada motor es **estúpido en su vecindario y experto en su dominio**. Eso permite que cualquiera pueda ser ejercido por un modelo pequeño y barato.

## 6. Politica de paralelismo

El Kernel decide independencia de motores:
- Research y Context pueden correr en paralelo.
- Planning corre cuando Research emite `research.completed` o `research.skipped`.
- Coding solo corre con `planning.locked`.
- Validation corre incrementalmente con cada `file.saved`.
- Repair corre con `validation.failed`.
- Learning corre *después* de cualquier success o failure (suscripción a `engine.success` y `engine.failed`).

## 7. Failure domains

Cada motor es un failure domain aislado:
- Sus propias notas en el Journal.
- Su propio timeout.
- Su propio límite de retries.
- Sus propios recursos (modelo, tokens, red).

Un motor caído no tira el sistema. El Execution Supervisor lo respawn.

## 8. Versionado de motores

Cada motor expone `version`, `contract_version`, `capabilities[]`. El Kernel hace match semántico con el Skill Graph y con los modelos compatibles (ver `06 - Skills.md` y `04 - Model Orchestrator.md`).

## 9. Componentes transversales (no-motores)

### 9.1 `DoomLoopDetector`

Detector **mecánico** (no heurístico) de bucles infinitos. OpenCode ya expone la política `doom_loop` que dispara `ask` tras 3 repeticiones idénticas de un mismo tool call. Aquí lo elevamos a componente formal del Kernel:

```ts
DoomLoopDetector {
  window_seconds: 60                          // ventana deslizante
  min_repeats: 3                              // 3 inputs idénticos = trip
  key: "{tool_name}|{normalized_input_hash}"  // hashestable del input
  on_match: trigger_permission(doom_loop)     // hard-deny en modo AUTONOMOUS
  log: every_iteration_with { n, input_hash, mode_before, mode_after }
}
```

Este componente supera a Aider, Cline/Roo, Continue.dev y a los Cloud Agents de Cursor, que no exponen públicamente un detector mecánico de repetición. Es primero del estado del arte, replicado del runtime OpenCode (https://opencode.ai/docs/permissions).

### 9.2 `GoalTracker`

Memoria persistente del **objetivo** de cada sesión. Evita el *goal drift* en loops largos:

```
GoalTracker {
  goal_text: string            // "hacer pasar npm test"
  success_predicate: string    // "exit_code == 0"
  iteration_count: uint
  last_progress_step: uint     // último step que redujo errors_count / tocó un nuevo archivo
  on_goal_drift: trigger_permission(goal_drift)  // tras K iter sin progress
}
```

Es la formalización del patrón "scheduled agentic loop inspirado en Sakana Fugu": cada `agent.run` rejuvenece `last_progress_step`; si `iteration_count - last_progress_step > K_auto` se dispara `goal_drift`.

### 9.3 `LanguageIdResolver`

Routing de servidores de lenguaje **por extensión de archivo**, emulando VS Code:

```ts
LanguageIdResolver {
  associations: Map<Extension, LanguageId>   // por defecto estilo VS Code
  overrides: .opencode/languages.json        // override del usuario
}
```

VS Code enruta cada `textDocument/*` al servidor cuyo `documentSelector` coincide con el LanguageId del documento activo (https://code.visualstudio.com/docs/languages/overview). Atlas OS implementa el mismo patrón: al abrir `foo.ts` se arranca/resume el `typescript-language-server`; al abrir `bar.py` se arranca `pylsp`/`pyright`. Múltiples archivos de distintos lenguajes → múltiples procesos servidor activos en paralelo.

### 9.4 `ProjectSymbolTable`

Cache de símbolos por proyecto, estilo IntelliSense / Roslyn Workspace. Inicialización incremental vía LSP `workspace/symbol`; invalidación on `textDocument/didChange` (no espera al save). Esto da autocompletado tradicional en el Modo Manual Classic (ver `21 - Execution Modes.md`). La sub-fase 9.4 (M36) consume esta tabla vía `lsp::confidence::SymbolConfidence`: hover/diagnostics exponen Confidence por símbolo (base = Skill Picker relevance 8.1 + presencia AstSymbol 9.2).

### 9.5 `SemanticEmbeddingIndex`

Para el Modo IA: compone el `ProjectSymbolTable` + el `VectorKnowledgeBase` y expone recall semántico a cualquier motor.

### 9.6 `ExecutionSupervisor` State Machine

`ExecutionSupervisor` no es un loop `while true` — es un *state machine* explícito:

```
idle → planning → executing → verifying → recovering → halted → done
```

Transiciones deterministas; ver `19 - Execution Supervisor.md` y `21 - Execution Modes.md`.
