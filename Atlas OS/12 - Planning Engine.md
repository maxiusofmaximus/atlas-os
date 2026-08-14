# 12 - Planning Engine

El motor que **solo piensa**. No escribe código. Descompone la mission en objetivos y pasos, calcula riesgo, estima impacto, genera un roadmap y decide estrategia.

---

## 1. Funciones

```
Planning Engine
   ├── divide objetivos
   ├── calcula riesgo
   ├── estima impacto
   ├── genera roadmap
   └── decide estrategia
```
(Continuamente re-entra si los Validation o Repair escapan replanificación.)

## 2. Entradas

- **`MissionConsolidated`** del Prompt Understanding Pipeline (RFC `23 - Prompt Understanding & Refinement.md`). El Planning Engine **no** acepta `raw prompt`; exige `mission.locked = true`, falla en caso contrario.
- `PublicUnderstandingVerdict` asociado (RFC `23 §3`): `intent`, `confidence_rubric`, `gaps[]`, `accepted_assumptions`, `suggested_mode`, `recommended_research_queries`.
- Project Map (Context Engine, RFC `11`).
- Skills disponibles (Skill Graph filtrado por dominio, RFC `06`).
- Research Run Reports (si la decisión es crítica o si `MissionConsolidated.requires_research_first = true` — RFC `10`).
- Confidence Score del Reasoning Engine.
- Estado del Journal (progreso previo, decisiones tomadas).

### 2.1 Handoff de Prompt Understanding a Planning Engine

```
PublicUnderstandingVerdict ─┐
   verdict.confidence       │
   verdict.gaps[]           │
   verdict.suggested_mode   │
                            ▼
                ┌────────────────────────┐
                │ MissionConsolidated    │
                │   if mission.locked    │
                └────────────────────────┘
                            │
                            ▼
                    Planning Engine
                            │
                            ▼
                        Plan
```

Si `verdict.confidence < HIGH` y el usuario no ha aceptado las `clarification_questions`, el Planning Engine rechaza el input y emite `task.rejected: reason=missing_clarification`. Esto bloquea la planificación prematura, fuente #1 de alucinación.

### 2.2 Modes sugeridos por el verdict

`MissionConsolidated.suggested_mode` (`23 §8`) actúa como **hint** para el Planning Engine:

| `suggested_mode` | Efecto en Planning |
|---|---|
| `ask` | Plan generado en readOnly; soloSteps de research/clarify, no coding steps. |
| `architect` | Plan generado en dual-model: architect model propone, editor model ejecuta. Igual que Aider `/architect`. |
| `code` | Plan full coding, ejecución normal. |
| `context` | Plan generado tras `context_engine.reorder(verdict.keys, verdict.lost_in_the_middle=true)` para evitar clave al medio (Liu et al. 2023). |

El Planning respeta el mode sugerido pero puede elevar/disco si el usuario lo pide explícitament e via override (auditable en Journal).

## 3. Salida: Plan

Estructura:
```
Plan {
  mission_id
  mission: string          // en lenguaje natural
  objectives: [Objective]
  steps: [Step]
  strategy: string
  risk: 0..1
  impact: "minor|major|breaking"
  roadmap: [Milestone]
  skills_used: [SkillRef]
  models_needed: [ModelRef]
  research_runs: [RRRef]
  confidence: 0..1
  resume_point: StepId
  blocked: [Blocker]
}
```

### 3.1 DAG mode (RFC 28 §C, behind `dag_mode`)

Cuando la feature `dag_mode` está activa (`Cargo.toml` feature, default off), el Planner puede emitir un **DAG** — no una lista lineal — en vez del `steps: [Step]` anterior. El DAG esPersistido en la tabla M15 `mission_graph_{nodes,edges}` (ver RFC 28 §C item 2) y respeta el mismo contrato de auditoría (`precondition` + `guard` + `visit_count` por edge, `provenance` EXTRACTED|INFERRED|AMBIGUOUS por node). El Coding Engine branchea cuando Múltiples paths válidos; el Validator puntúa cada rama; el Repair reescribe edges fallidos en vez de rehacer toda la cadena. Tags `AMBIGUOUS` (choices under confidence threshold) defieren al usuario via RFC 24 HUD "steer" button.

**Migración gradual:** default-off, sólo opt-in por mission flag `dag_mode=true`. Sin este flag, la salida permanece como `steps: [Step]` lineal para no romper callers existentes (Codificación, Validación, supervisor). Esta sección es la **declaración espec**; el emitter vive en `src-tauri/src/planner/graph_emitter.rs` (RFC 28 §C item 4).

## 4. Cálculo de riesgo / impacto

Riesgo proviene de:
- cuántos nodos del Architecture Memory se alteran,
- número de tests rotos en seco,
- cuántas skills conflict,
- dependencias transitivas críticas.

Impacto afecta a la política de aprobaciones: cualquier `impact=breaking` sube a `confirm` aunque el sub-modo sea `auto`.

## 5. Roadmap

Plan en milestones, no en una sola lista lineal:
```
M1: Contratos (Prisma schema, OpenAPI)
M2: MSW mocks de la API nueva
M3: Backend tRPC routers
M4: Frontend consume
M5: Tests E2E
M6: IaC (Pulumi) + CI
```
Cada milestone expone dependencias y puede paralelizarse por el Swarm Coordinator.

## 6. Estrategias

El engine elige una estrategia antes de empezar:
- **TDD**: test primero, después código.
- **Strangler fig**: reemplazar pieza vieja por una nueva.
- **Big bang**: solo si impacto=major y research lo recomienda.
- **Incremental**: por defecto.
- **Pair**: dos agentes (IA + usuario) coeditan (modo Mixed).

## 7. Anti-inicio defectuoso

Posee un guardia: si la mission no termina en `Plan.confidence ≥ 0.7`, NOT init Coding Engine.
En su lugar, pide más Research o más meta-razonamiento.

El guardia también inspecciona el `verdict.confidence` del Prompt Understanding Pipeline (RFC `23 §5`): si está en `LOW` o `BLOCK`, no se **genera** Plan aunque el reasoner diga 0.9; está bloqueado por la entrada. El reasoner no tiene derecho a invalidar el verdict sin intervención humana.

## 8. Permite Overrides manuales

El usuario puede editar el plan a mano desde el Command Center antes de desbloquear Coding. Cualquier override queda registrado en el Journal y alimentando al Learning.

## 9. Replanning

Disparadores:
- `validation.failed` persistente (>2 intentos de Repair)
- `confidence.research<umbral`
- Cambio de stack por Research
- Falta de un modelo en el sub-mode activo

Replanificar parse el Plan de cero, no incrementa. La antigua versión persiste para diff y aprendiz.
