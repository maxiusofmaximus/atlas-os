# Harbor adapter (Atlas OS EVAL.3 / Fase 25)

> **Paso a paso completo:** [`RUNBOOK.md`](./RUNBOOK.md) (Docker/WSL → Harbor → ingestión).
> Este README es la referencia corta; el runbook es el procedimiento operativo.

## Baseline medido (2026-10-04)

Primera corrida real de Atlas bajo Harbor, verificada end-to-end en esta máquina (Docker vía
WSL2, Harbor 0.23.0):

| Harness                         | Agente                    | Modelo                   | Tareas (en curso) | pass_rate | failure kind | fecha      |
| ------------------------------- | ------------------------- | ------------------------ | ----------------- | --------- | ------------ | ---------- |
| terminal-bench/terminal-bench-2 | `harbor_atlas:AtlasAgent` | moonshotai/kimi-k3 (NIM) | 0/6               | 0.0%      | `VERIFY`     | 2026-10-04 |

`oracle` (solver de referencia) da mean ≈ 0.88 en el **mismo** harness — así que el harness
mide bien: el 0.0% es la capacidad actual de Atlas, no un artefacto del andamio. Atlas aún no
ejecuta comandos de terminal arbitrarios ni verifica artefactos por sí mismo; ese es el
siguiente salto de capacidad (post-F38).

Pipeline verificado:

```
harbor run … --agent harbor_atlas.atlas_agent:AtlasAgent -o jobs/<id>
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
