# 05 - Swarm

El **Swarm Coordinator** orquesta múltiples agentes trabajando **en paralelo** sobre la misma mission. Es lo que nos permite superar a Hermes sin copiarlo: haciéndolo distribuido.

---

## 1. Roles de agente

No todos los agentes escriben código. Cada rol tiene un propósito cerrado:

| Rol | Hace | No hace |
|---|---|---|
| Planner | descompone la mission | escribe código |
| Researcher | recolecta evidencia | decide |
| Architect | diseña | implementa |
| Backend | implementa backend | toca frontend |
| Frontend | implementa UI | toca backend |
| Database | altera esquemas | toca lógica de negocio |
| Security | audita | añade features |
| Testing | escribe y corre tests | refactoriza lógica |
| Reviewer | critica y valida | edita archivos |
| Merger | fusiona ramas de trabajo de los demás | genera contenido nuevo |

## 2. Topología distribuida

```
Mission
   ↓
Planner ──▶ Researcher ──▶ Architect
                              │
   ┌──────────────┬───────────┼───────────┬─────────────┐
   ▼              ▼           ▼           ▼             ▼
Backend       Frontend    Database    Security       Testing
   │              │           │           │             │
   └──────────────┴─────┬─────┴───────────┴─────────────┘
                        ▼
                    Reviewer
                        ▼
                     Merger
                        ▼
                  Validation Engine
```

Todos pueden trabajar **al mismo tiempo** sobre ramas virtuales separadas o worktrees de Git reales.

## 3. Asignación de modelo por rol

El Swarm Coordinator pide al Model Orchestrator un modelo apropiado **para cada rol**, no para toda la mission. Ejemplo realista en una máquina del usuario:

- Planner → Claude (remoto)
- Researcher → Gemini (gran ventana)
- Backend → GPT
- Frontend → modelo local 7B (alta VRAM, baja latencia)
- Database → DeepSeek (preciso con esquemas)
- Security → modelo local 14B instruido
- Testing → Cerebras / Groq (rápido)
- Reviewer → o3 o Claude
- Merger → modelo rápido (Sambanova)

Resultado: **10 agentes trabajando en paralelo, cada uno con el cerebro óptimo**.

## 4. Coordinación de estado

El swarm no comparte un único contexto (eso saturaría tokens). Comparte:

- **Ramas de trabajo** (worktrees Git reales o virtuales).
- **Shared Journal**: solo diffs, no contexto completo.
- **Architecture Memory** (lectura; escritura únicamente por Merger/Reviewer).
- **Locks** sobre archivos: un archivo solo lo edita un agente a la vez.

### Protocolo de merge
1. Cada agente emite un **diff** sobre su rama.
2. El Reviewer valida cada diff (correctitud, estilo, seguridad).
3. El Merger aplica en orden determinístico.
4. La Validation Engine corre sobre la fusión.
5. Si hay conflicto, el Planner decide replanificar el sub-objetivo.

## 5. Pool swarm estilo Kimi

OpenCode OS admite **pool swarms**: un solo rol con **N instancias** intercambiables (por ejemplo 3 Backends) para acelerar tareas largas.

Configuración del pool:
```json
{
  "role": "Backend",
  "pool_size": 3,
  "strategy": "shard_by_module",
  "models": ["gpt-4o", "local-qwen2.5-14b", "local-deepseek-7b"],
  "sync": "journal_snapshot_every_5_steps"
}
```

El pool comparte un Journal snapshot cada N pasos para mantenerse coherente.

## 6. Subagentes locales sin usar

El usuario con buena GPU / RAM suele tener modelos locales ociosos. El swarm los consume automáticamente:

- Detecta modelos disponibles vía LM Studio / Ollama / llama-server.
- Mide VRAM y RAM libres → decide cuántos pueden correr en paralelo.
- Les asigna roles que no requieren frontier (Testing, Reviewer junior, Frontend rápido, exploración de archivos).

## 7. Visibilidad desde cualquier parte del mundo

El Command Center y el Agent Console exponen:

- Cada agente activo (rol, modelo, archivo, progreso, tokens).
- Cada diff en vivo.
- Cada decisión y su evidencia.
- Cola de aprobaciones pendientes.

Estos paneles pueden servirse por una UI web segura (ver `17 - UI.md`), replicando el principio de Hermes: ver el trabajo de los agentes desde cualquier dispositivo con autenticación.

## 8. Límites

- `max_agents_per_mission`
- `max_files_locked_per_agent`
- `max_concurrent_local_models` (calculado por VRAM)
- `max_remote_concurrency` (presupuesto)

Se evitan:
- Deadlocks: orden de locks determinístico por archivo.
- Starvation: planner reasigna roles muertos tras N segundos sin heartbeat.
- Aluvión de diffs: el Merger aplica lotes, no diff por diff.

## 9. Modo "un solo cerebro largo"

Si el usuario lo pide, el swarm puede colapsar a un único agente largo (estilo Claude Code) para tareas que exigen contexto unificado. Es la excepción, no el default.
