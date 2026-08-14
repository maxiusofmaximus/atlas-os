# 02 - Agent Operating System

El núcleo del producto. Define el **kernel** que coordina los motores, los modelos, los agentes y la memoria.

---

## 1. Analogía

Atlas OS se comporta como un sistema operativo:

| SO tradicional | Atlas OS |
|---|---|
| Kernel | Agent Engine Kernel |
| Scheduler de procesos | Scheduler de agentes / modelos |
| Sistema de archivos | Vector Knowledge Base + Execution Journal |
| Drivers | Skills, MCPs, CLIs |
| IPC | SwarmCoordinator |
| Init system | Execution Supervisor |
| Syscalls | Motor API |
| Modo usuario / supervisor | Modo Manual / Modo IA |

## 2. Topología del sistema

```
                    USER
                      │
                      ▼
               Command Center
                      │
       ┌──────────────┼────────────────┐
       ▼              ▼                ▼
    Editor        AI Chat        Agent Console
       │              │                │
       └──────────────┼────────────────┘
                      ▼
            Agent Operating System
                      │
       ┌──────────────┼──────────────────┐
       ▼              ▼                  ▼
    Context      Planning            Research
                      │
                      ▼
                Reasoning
                      │
                      ▼
            Model Orchestrator
                      │
       ┌──────────────┼──────────────────┐
       ▼              ▼                  ▼
    Claude          GPT             DeepSeek
       ▼              ▼                  ▼
            Swarm Coordinator
                      ▼
                Coding Engine
                      ▼
              Validation Engine
                      ▼
                Repair Engine
                      ▼
               Learning Engine
                      ▼
              Execution Journal
```

## 3. Componentes del kernel

### 3.1 Kernel Bus
Canal de eventos interno. Todos los motores publican y consumen eventos:
- `task.received` (entrada del usuario — canalizado por Prompt Understanding Pipeline antes de Planning — ver `23 - Prompt Understanding & Refinement.md`).
- `mission.consolidated` (verdict ready).
- `mission.locked` (planning permission granted).
- `research.completed`
- `decision.made`
- `code.written`
- `validation.failed`
- `repair.applied`
- `learning.registered`
- `agent.stalled`
- `hud.*` (eventos consumidos por el HUD Mission Control — `24 - HUD Mission Control.md` §4.1: `agent.status`, `agent.diff`, `agent.tokens`, `agent.heartbeat`, `approval.request`, `approval.decision`, `doom_loop.detected`, `goal_drift.detected`, `journal.checkpoint`, `cost.threshold.crossed`, `worktree.dirty`, `skill.activated`).

El Kernel Bus **no** publica `task.received` directamente al Planning Engine. El flow correcto es:

```
task.received
  └─▶ Prompt Understanding Pipeline (`23`)
        ├─▶ probe_feasibility (`10 §11`) si gap_type lo exige
        ├─▶ verdict emit
        ├─▶ mission.consolidated emit
        └─▶ mission.locked emit (si user aprobó o confidence ≥ HIGH)
              └─▶ Planning Engine consume `12`
```

Cualquier intento directo del Planning de consumir `task.received` sin `mission.locked=true` se rechaza con `task.rejected reason=missing_unstanding`.

### 3.1.1 SOP estándar de una Mission viva

Secuencia obligatoria con checkpoints formales (patrones tomados de Temporal y LangGraph — ver `22 §8.1`):

```
1. task.received                    [user prompt]
2. verdict.computed                 [Prompt Understanding Pipeline]   CHECKPOINT
3. mission.consolidated              [if HIGH confidence]
4. mission.locked                    [user or auto]                    CHECKPOINT
5. plan.generated                    [Planning Engine]
6. plan.approved (optional)                                            CHECKPOINT
7. swarm.dispatched                  [Swarm Coordinator]
8. agent.* events                    [each subagent]
9. validation.completed              [Validation Engine]               CHECKPOINT
10. repair.applied (if needed)
11. learning.registered              [Learning Engine]
12. mission.completed                [emite final mission outcome]     CHECKPOINT
```

Todos los `CHECKPOINT` se persisten en Journal (`19 - Execution Supervisor.md`). Si el proceso muere, el Execution Supervisor reanuda desde el último checkpoint sin abort.

### 3.1.2 Handoff entre motores

Característica crucial de tomar de Temporal (durable execution): cada evento entre dos motores se relanza al estilo **at-least-once**. El receptor **idempotente** consume el evento con `idempotency_key = event_id`, garantizando que el restart no duplica efectos.

```rust
trait Motor: Send + Sync {
  fn consume(&self, event: KernelEvent, idempotency_key: String)
    -> impl Future<Output = Result<ConsumedResult>>;
}
```

Por ejemplo, el Planning Engine recibe `mission.locked`, genera Plan y emite `plan.generated`. El Swarm Coordinator recibe `plan.approved`, despacha subagents, emite `swarm.dispatched`. La cadena no es un loop infinito: cada motoridad tiene un next-step restraint.

### 3.2 Scheduler de agentes
Decide qué agente está activo, en qué orden, con qué prioridad y con qué modelo. Es distinto al **Model Orchestrator** (que decide el *cerebro*): el Scheduler decide el *trabajador*.

### 3.3 Capability Resolver
Antes de cualquier tarea, resuelve:
```
Proyecto detectado
  ↓
Lenguaje
  ↓
Framework
  ↓
Runtime
  ↓
Arquitectura
  ↓
Skills compatibles
  ↓
Pipeline generado
```
Si el proyecto es Python, no se invoca Prisma. Si es Go, no se invoca Zod. Si es Rust, no se invoca React Doctor. El pipeline se **genera**, no se asume.

### 3.4 Memoria del sistema
Cuatro almacenes persistentes (ver `09 - Vector Knowledge.md`, `11 - Context Engine.md`, `19 - Execution Supervisor.md`):
1. **Execution Journal** — estado de ejecución y checkpoints.
2. **Architecture Memory** — mapa del proyecto en forma de grafo.
3. **Skill Graph** — capacidades indexadas semánticamente.
4. **Vector Knowledge Base** — todo lo anterior recuperable por embeddings.

### 3.5 Política de ejecución
Toda acción sensible pasa por una **cola de aprobación** con tres niveles:
- **Auto**: el sistema la ejecuta.
- **Confirm**: requiere un clic del usuario.
- **Forbidden**: bloqueada por la política de seguridad.

El nivel por defecto lo define `18 - Security.md`.

## 4. Ciclo de vida de una tarea

1. El usuario o un agente emite una **Mission** cruda → `task.received`.
2. El **Prompt Understanding Pipeline** (`23`) interpreta el prompt, detecta `gap_type[]`, genera `PublicUnderstandingVerdict` y `MissionConsolidated`. Si `confidence < HIGH` bloquea y pregunta K clarification questions. Solo cuando `locked = true` se continúa.
3. El **Planning Engine** descompone la `MissionConsolidated` en objectives y steps.
4. El **Capability Resolver** arma el pipeline compatible.
5. El **Research Engine** corre si la decisión es crítica o si `MissionConsolidated.requires_research_first = true` (incluye `probe_feasibility` del `10 §11`).
6. El **Reasoning Engine** razona con CoT/ToT/Self-Reflection y emite un Confidence Score propio.
7. El **Model Orchestrator** elige el cerebro.
8. El **Swarm Coordinator** puede paralelizar si conviene.
9. El **Coding Engine** escribe código.
10. El **Validation Engine** corre incrementalmente.
11. El **Repair Engine** actúa sobre cualquier fallo.
12. El **Learning Engine** extrae reglas + marca `was_correct` si el outcome final da feedback.
13. Todo se persiste en el **Execution Journal**.
14. El usuario ve cada paso en el HUD Mission Control (`24 - HUD Mission Control.md`), eventos WebSocket en vivo, approvals queue, demos over diffs.

El **Execution Supervisor** (`19-propio`) enmarca toda la cadena con checkpoints en cada paso (3.1.1), decides what to reanudar on crash.

## 5. Modos de operación

Atlas OS define **tres ejes ortogonales**:

- **Execution Modes** (cuán autónomo es el agente — ver `21 - Execution Modes.md` §1): `MANUAL_CLASSIC` / `HUMAN_IN_LOOP` / `AUTOPILOT` / `AUTONOMOUS`.
- **Modo de uso** (cómo se interpreta el prompt — ver `21 - Execution Modes.md` §12 y `23 - Prompt Understanding & Refinement.md` §8): `ask` / `architect` / `code` / `context`.
- **Resource Mode** (qué modelos puede usar — §5.2): locales / gratuitos / mixto.

Cualquier combinación válida según la matriz de `21 §12.1`. Los ejes no se mezclan libremente: por ejemplo, `AUTONOMOUS + ask` es inválido y se detecta al parsear el estado (`21 §12.1`); el Kernel hace downgrade seguro automático al modo más conservador compatible.

### 5.1 Execution Modes
| Modo | Descripción | Default para |
|---|---|---|
| `MANUAL_CLASSIC` | Editor clásico: solo LSP + IntelliSense-style completion. Sin agency. Sin diffs automáticos. | archivos `.lock`, `.min.*`, assets generados |
| `HUMAN_IN_LOOP` | El agente actúa pero **cada acción sensible pide aprobación**. El humano lee el plan y el diff antes de apply. | código de producción, refactor grande |
| `AUTOPILOT` | El agente edita y corre tools pero **cada N pasos pide checkpoint**. Reemplaza el "yolo mode" de Cline con checkpoints visibles. | features nuevas end-to-end |
| `AUTONOMOUS` | Loop completo hasta success_predicate, con `doom_loop` hard-deny, budget cap, time cap, compaction auto. Estilo Cursor Cloud Agent pero local. | tareas largas (3–30 min), runs nocturnos |

Cualquier modo puede degradar a uno menos autónomo on-demand (por `doom_loop`, `goal_drift` o petición del usuario).

### 5.2 Resource Mode

- **Solo locales** (LM Studio, Ollama, llama-server, modelos de 7B a 70B en la GPU del usuario).
- **Solo gratuitos** (Nvidia NIM, Google AI, OpenRouter free tier, GitHub, Cerebras, Groq, Sambanova, Cloudflare Workers AI, HuggingFace, Mistral).
- **Mixto** (locales + gratuitos + de pago).

La política la define el Model Orchestrator respetando la restricción.

## 6. Casos de fallo conocidos que el kernel cubre

- **Agente caído a mitad de tarea** → Execution Supervisor reanuda desde el último checkpoint.
- **Modelo se detiene en ejecución** (caso MiniMax M3 en Nvidia NIM) → Heartbeat + Journal + reanudación automática, sin script externo.
- **Bucle infinito** → límite de replanificaciones por checkpoint antes de escalar al usuario.
- **Conflicto entre agentes** → Swarm Coordinator usa votación / fusión / prioridad de dominio.
- **Pérdida de contexto** → imposible: el Journal y la Architecture Memory son la fuente de verdad.

## 7. No-goales explícitos

- No ser un IDE monolítico.
- No acoplarse a un modelo.
- No depender de un único proveedor de nube.
- No reinventar protocolos existen (MCP) sino consumirlos de forma segura.
- No crecer indefinidamente en skills sin compresión.
