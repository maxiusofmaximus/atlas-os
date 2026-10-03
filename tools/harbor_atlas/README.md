# Harbor adapter (Atlas OS EVAL.3)

Dev-only integration to run **Atlas OS as an agent under [Harbor](https://github.com/harbor-framework/harbor)**
(Harbor is the official harness for **Terminal-Bench 2.0**; it also drives
SWE-bench Verified, Aider Polyglot, etc.).

This directory is **not** part of the Atlas single-binary distribution
(RFC 25 §11). It requires Harbor + Docker + a model endpoint.

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

## Files

- `atlas_agent.py` — `AtlasAgent(BaseInstalledAgent)`; the `run()` body is the
  integration point (point it at the Atlas headless/ACP entrypoint).
- `run-harbor.ps1` — convenience runner.

## Notes

- The importer (`atlas eval import`) parses Harbor's `JobResult`/`TrialResult`
  schema offline — it does not require Harbor to be installed.
- `AtlasAgent.run()` currently issues `atlas mission new … && atlas run`; adjust
  it to the exact headless entrypoint once the ACP host loop is wired to a
  container-friendly command. This file is a documented scaffold, not a
  CI-exercised path.
