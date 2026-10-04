# research/60 — Fase 39: Terminal agent loop

- **Fecha:** 2026-10-04
- **Estado:** **COMPLETA** (v39.0).
- **Contexto:** el baseline de Atlas en Terminal-Bench 2 dio **0.000 (89/89)** con el coding
  loop (`execute --coding`), que **edita ficheros pero no ejecuta comandos**. Las tareas de
  Terminal-Bench requieren operar un terminal: descargar, compilar, mover ficheros, verificar.
  `oracle` (que sí ejecuta comandos) da 0.88. El gap de capacidad era exactamente ese.

## 1. Decisión

Añadir un **bucle agéntico de terminal**: el modelo emite un comando por turno
(`{"tool":"run_command","command":"…"}`), Atlas lo ejecuta en el workspace, le devuelve
stdout+stderr, y repite hasta `{"done":true,"summary":"…"}` o agotar el presupuesto de pasos.

## 2. Sub-fase

- **v39.0 COMPLETA** — `orchestrator/agent.rs`:
  - protocolo puro `parse_action` (tolera fences/prosa → `Run`/`Done`/`Invalid`).
  - ejecutor `run_command(root, cmd, timeout)` (shell `sh -c`/`cmd /C`, captura stdout/stderr,
    timeout con kill → exit 124, nunca paniquea).
  - loop `run_agent` genérico sobre `ProviderClient` (mockeable) + `run_agent_real` (shell real).
  - CLI `atlas agent <task> [--root] [--max-steps] [--command-timeout]`.
  - 7 tests (parse, tolerancia a fences, ejecución real, exit ≠ 0, loop con mock).
- **Verificado contra Groq (`openai/gpt-oss-120b`)**: tarea "crea greeting.txt con 'hello world'
  y verifícalo" → el modelo creó el fichero, **vio que `cat` fallaba (exit 1) y se corrigió solo
  con `type`**, verificó y `done=true`. `greeting.txt` = `hello world`. El ciclo
  comando→output→autocorrección funciona.

## 3. Hallazgos

- **NIM/kimi-k3** topa con rate-limit en bucles agénticos (muchas llamadas seguidas) →
  `cascade exhausted … 0 attempts`. **Groq** (`qwen3.8-27b`, `gpt-oss-120b`) aguanta mejor y
  tiene baja latencia: mejor modelo por defecto para el agente.
- En Windows el modelo debe usar `type`/`dir`; el loop se autocorrige al ver el exit code.

## 4. Siguiente (F38 re-run)

Re-correr Terminal-Bench con `--agent` (en vez de `--coding`) para medir el delta de capacidad
frente al 0.000. El adapter de Harbor necesita un modo que use `atlas agent <task>`.

### Corrida F38 (2026-10-04) — hecha

- Adapter en modo `agent` (`ATLAS_AGENT_MODE=agent`), Groq `openai/gpt-oss-120b`.
- **11/89 completados antes de detener** (corrida larga; se paró por coste/tiempo con el
  patrón ya claro): **0/11 passed**. El agente **opera el terminal de verdad** (recibe la
  tarea, corre `atlas agent --max-steps 40`, ejecuta comandos, termina sin crash), pero
  **no resuelve** las tareas: son difíciles (p.ej. `circuit-fibsqrt` requiere escribir 32.000
  líneas de un simulador de puertas lógicas).
- **Conclusión honesta:** F39 elevó la *capacidad mecánica* (ejecutar comandos, autocorregirse),
  pero el 0.000 persiste porque las tareas de Terminal-Bench exigen razonamiento de agente de
  nivel frontera + muchas horas de crédito de modelo. El harness mide bien (oracle 0.88).
- Ingestión verificada: `atlas eval import` + `eval metrics` con desglose por harness/modelo.


## 5. Fuentes

- research/55/60 (coding loop), baseline `tools/harbor_atlas/README.md`, RFC 20 Fase 39.
- `orchestrator/agent.rs`, `orchestrator/wire.rs` (modelo de tool-calling).
