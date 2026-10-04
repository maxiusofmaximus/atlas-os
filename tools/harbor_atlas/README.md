# Harbor adapter (Atlas OS EVAL.3 / Fase 25)

> **Paso a paso completo:** [`RUNBOOK.md`](./RUNBOOK.md) (Docker/WSL → Harbor → ingestión).
> Este README es la referencia corta; el runbook es el procedimiento operativo.

## Baseline medido (2026-10-04) — Terminal-Bench 2.0, 89/89

Primera corrida **completa** de Atlas bajo Harbor, verificada end-to-end en esta máquina
(Docker vía WSL2, Harbor 0.23.0):

| Harness                         | Agente                    | Modelo                   | Trials    | pass_rate | errores                                                      | fecha      |
| ------------------------------- | ------------------------- | ------------------------ | --------- | --------- | ------------------------------------------------------------ | ---------- |
| terminal-bench/terminal-bench-2 | `harbor_atlas:AtlasAgent` | moonshotai/kimi-k3 (NIM) | **89/89** | **0.000** | 4 excepciones (3× `NonZeroAgentExitCode`, 1× `AgentTimeout`) | 2026-10-04 |

Contraste: **`oracle`** (solver de referencia) da **mean ≈ 0.88** en el **mismo** harness →
el instrumento mide correctamente; **el 0.000 es la capacidad real de Atlas hoy**, no un
artefacto del andamio. Atlas aún no ejecuta comandos de terminal arbitrarios ni verifica
artefactos por sí mismo (lo que sí hace `oracle`); ese es el siguiente salto de capacidad.

**Gap de observabilidad pendiente:** el `AgentContext` va con tokens/coste `null` — el adapter
no rellena `populate_context_post_run` (habría que volcar `model_invocations` del journal).
Por eso el harness no ve el coste de Atlas.

### Corridas (2026-10-04), consolidado

| #   | Modelo                                       | Modo       | Trials | pass_rate                             | coste  |
| --- | -------------------------------------------- | ---------- | ------ | ------------------------------------- | ------ |
| 1   | `moonshotai/kimi-k3` (NIM)                   | `--coding` | 89/89  | 0.000                                 | API    |
| 2   | `openai/gpt-oss-120b` (Groq)                 | `--agent`  | 11/11  | 0.000                                 | API    |
| 3   | `qwen3.8-flash-next-iq2_xs` (125B **local**) | `--agent`  | 89/89  | **>0** (`fix-git`, `prove-plus-comm`) | **$0** |

`oracle` ≈ 0.88 (control). Ver `README.md` §Evaluation baseline y `research/61 §8bis`.

**Requisitos para la corrida local (#3):** binario `atlas` **estático musl** montado en el
contenedor; Strata sirviendo el modelo en el host; `networkingMode=mirrored` en WSL + relay
`socat` (WSL `:8099` → Strata `:8080`) para que el contenedor alcance el modelo; `--ae`
con `ATLAS_LLM_BASE_URL=http://172.17.0.1:8099/v1`. Modelo: `ATLAS_AGENT_MODE=agent`.

### Modo agente (Fase 39)

El adapter tiene dos modos (env `ATLAS_AGENT_MODE`): `agent` (default, el terminal agent loop
`atlas agent "<task>"`) y `pipeline` (mission→plan→execute --coding). El modo agente es el que
**ejecuta comandos** (paradigma Terminal-Bench). Corrida de agente (Groq `gpt-oss-120b`,
2026-10-04): el agente opera el terminal y termina sin crash, pero **0/11 passed** en las 11
tareas completadas — las tareas exigen razonamiento de nivel frontera. Ingestión verificada.

Pipeline verificado (reproducible):

```
harbor run -d terminal-bench/terminal-bench-2 \
  --agent harbor_atlas.atlas_agent:AtlasAgent --model moonshotai/kimi-k3 \
  --mounts '<atlas+env bind>' -o jobs/<id>
atlas eval import jobs/<id>      # → eval_runs/eval_cases (schema Harbor 0.23: <trial>/result.json)
atlas eval metrics               # → pass_rate / tokens / $ / failure kinds
```

Dev-only integration to run **Atlas OS as an agent under [Harbor](https://github.com/harbor-framework/harbor)**
(Harbor is the official harness for **Terminal-Bench 2.0**; it also drives
SWE-bench Verified, Aider Polyglot, etc.).

This directory is **not** part of the Atlas single-binary distribution
(RFC 25 §11). It requires Harbor + Docker + a model endpoint.

## What the agent runs

`atlas_agent.py` drives the **real orchestrated pipeline** (not the Phase-1
single pass):

```
atlas --profile harbor mission new --force "<instruction>"   # heuristic understand + lock
atlas --profile harbor plan     "<mission_id>"               # Planning Engine
atlas --profile harbor execute  "<mission_id>"               # orchestrator loop (routing+cascade+gate+cost)
```

- `--force` force-locks the mission (offline heuristic verdict may be below the
  auto-lock threshold), so the pipeline never stalls on a confidence gate.
- `execute` (Fase 25 v25.2/v25.3) is what makes the measured harness the
  **orchestrated** one — routing, fallback cascade, reliability gate and cost
  guard all run, and each step persists a `model_invocations` row + `AgentTokens`.

## Pipeline

1. Run Atlas under Harbor (from the repo root):

   ```powershell
   .\tools\harbor_atlas\run-harbor.ps1 -Model <provider/model>
   ```

   or directly:

   ```bash
   PYTHONPATH=tools uv run harbor run \
     --dataset terminal-bench@2.0 \
     --agent harbor_atlas.atlas_agent:AtlasAgent \
     --model <provider/model> \
     --n-concurrent 4
   ```

2. Ingest the baseline into Atlas's evaluation store:

   ```bash
   atlas eval import jobs/<job-id>      # jobs/<job-id>/result.json (JobResult)
   atlas eval report                    # pass/fail + tokens + cost per harness×model
   ```

## Prerequisites / caveats

- **Atlas binary in the image**: the task's Dockerfile (or a bind mount) must
  put `atlas` on PATH. `install()` only adds system packages.
- **Profile + credentials**: a `harbor` Atlas profile with a deployment whose
  `api_key_env` is present in the agent environment (e.g. `OPENAI_API_KEY`).
  Harbor passes secrets via the agent env.
- The importer (`atlas eval import`) parses Harbor's `JobResult`/`TrialResult`
  schema **offline** — it does not require Harbor to be installed.
- The exact CLI flags must match the installed Atlas (`mission new --force`,
  `plan`, `execute`); adjust `atlas_agent.py` if they diverge.
- This file is a documented scaffold, not a CI-exercised path.
