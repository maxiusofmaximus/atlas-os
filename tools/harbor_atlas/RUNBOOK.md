# Runbook — Baseline externo de Atlas OS (F32 / F38)

Guía paso a paso para correr Atlas como **agente bajo Harbor** (Terminal-Bench 2.0 /
SWE-bench Verified) y **ingerir el número** en el store de evaluación. Cierra las Fases
F32 (cerrar B) y F38 (comparativa antes/después) de `PLAYBOOK.md`.

> **Por qué esto vive fuera del repo:** el harness numérico requiere **Docker** (cada tarea
> corre en su contenedor). Atlas en sí ya está probado end-to-end **sin** Docker contra un
> LLM real (NIM/kimi-k3): `mission new --force → plan → execute --coding --apply`. Este
> runbook añade la **medición** contra un benchmark público.

---

## 0. Requisitos previos

| Requisito                    | Cómo verificarlo              | Si falta                                                                       |
| ---------------------------- | ----------------------------- | ------------------------------------------------------------------------------ |
| **Docker** (Engine + daemon) | `docker info`                 | Win: Docker Desktop con backend **WSL2**; Linux: `sudo systemctl start docker` |
| **WSL2** (Win)               | `wsl --status`                | `wsl --install` (requiere admin + reinicio)                                    |
| **uv / uvx**                 | `uv --version`                | `pip install uv` o `winget install astral-sh.uv`                               |
| **Python 3.11+**             | `python --version`            | `winget install Python.Python.3.12`                                            |
| **Salida a internet**        | `curl https://hub.docker.com` | proxy/firewall                                                                 |
| **Endpoint de modelo**       | ver §2                        | `.env` (ya configurado)                                                        |

Skill recomendado si usas el agente de este repo: **`android-cli`** no; para Docker puro basta
la CLI. (Harbor se instala vía `uv`, no requiere el MCP de Android.)

---

## 1. Instalar Harbor y confirmar el dataset

Desde la **raíz del repo** (`C:\Users\Max\Desktop\atlas-os`):

```powershell
# 1.1 Harbor en un venv efímero (uv lo gestiona)
uv tool install harbor

# 1.2 Confirmar la etiqueta del dataset (puede cambiar de versión)
uv run harbor datasets list | Select-String "terminal-bench"

# 1.3 Inspeccionar el esquema real de resultados (para el import)
uv run harbor --help
```

Anota la etiqueta exacta (p.ej. `terminal-bench@2.0`). Ajusta `run-harbor.ps1`/comandos si cambia.

---

## 2. Endpoint de modelo

Harbor pasa las credenciales al contenedor por **env del agente**. Atlas ya resuelve el
endpoint desde `.env` (`ATLAS_LLM_BASE_URL`, `ATLAS_LLM_MODEL`, `NVIDIA_API_KEY`).

Verifica que el endpoint responde **desde tu host** antes de meter Docker de por medio:

```powershell
# Lee .env y prueba un POST mínimo (sin exponer la key)
$e = @{}; Get-Content .env | % { if ($_ -match '^\s*([A-Z_]+)\s*=\s*(.*)$') { $e[$matches[1]]=$matches[2].Trim() } }
Invoke-RestMethod "$($e.ATLAS_LLM_BASE_URL)/chat/completions" -Method Post `
  -Headers @{ Authorization = "Bearer $($e.NVIDIA_API_KEY)"; 'Content-Type'='application/json' } `
  -Body (@{ model=$e.ATLAS_LLM_MODEL; messages=@(@{role='user';content='ping'}); max_tokens=5 } | ConvertTo-Json -Depth 5)
```

> **Caveat real:** el **daemon Docker** y el **contenedor** necesitan alcanzar el endpoint.
> Si el modelo es local (Ollama/vLLM en el host), el contenedor debe llegar al host
> (`host.docker.internal:PORT`) y ese puerto ha de estar expuesto. Para NIM/Groq/Cerebras
> (HTTPS público) basta con que el contenedor tenga salida.

---

## 3. Meter el binario `atlas` en la imagen

El adaptador (`tools/harbor_atlas/atlas_agent.py`) asume `atlas` en `PATH` **dentro del
contenedor**. Dos opciones:

**Opción A — bind-mount (rápida, Linux/Dev):** añade a la config de entorno de Harbor un
`mounts:` con el binario Linux compilado:

```yaml
# standalone.yml (o el que use tu tarea de Harbor)
environment:
  mounts:
    - type: bind
      source: /abs/path/atlas
      target: /usr/local/bin/atlas
      read_only: true
    - type: bind
      source: /abs/path/.env # o pasa las vars por environment.env
      target: /work/.env
      read_only: true
```

**Opción B — construir el binario dentro de la imagen:** extensión al `Dockerfile` de la tarea:

```dockerfile
# tras las deps de la tarea
COPY atlas /usr/local/bin/atlas      # binario Linux (musl/glibc) compilado aparte
RUN chmod +x /usr/local/bin/atlas
```

Compilar el binario **Linux** desde Windows (WSL2):

```powershell
wsl bash -lc "cd /mnt/c/Users/Max/Desktop/atlas-os && cargo build --release --manifest-path src-tauri/Cargo.toml --bin atlas"
# -> src-tauri/target/release/atlas  (ELF Linux)
```

> El binario `atlas` es **single-file**, sin Node/Python, así que se copia tal cual.

---

## 4. Config del agente para Harbor

`tools/harbor_atlas/run-harbor.ps1` ya tiene los defaults. Edítalos si hace falta:

| Param         | Default                               | Nota                                                                                     |
| ------------- | ------------------------------------- | ---------------------------------------------------------------------------------------- |
| `-Dataset`    | `terminal-bench@2.0`                  | confirma en §1.2                                                                         |
| `-Model`      | `anthropic/claude-opus-4-1`           | Harbor necesita un `provider/model`; Atlas además lee su propio `ATLAS_LLM_*` del `.env` |
| `-Agent`      | `harbor_atlas.atlas_agent:AtlasAgent` | el adaptador de este repo                                                                |
| `-Concurrent` | `4`                                   | baja a `1` si hay rate limits                                                            |

El adaptador ejecuta dentro del contenedor:

```bash
atlas --profile harbor mission new --force "<instruction>"   # entiende + lockea
atlas --profile harbor plan     "<mission_id>"               # Planning Engine
atlas --profile harbor execute  "<mission_id>" --coding --apply --root .   # loop real
```

> **Importante:** el `--profile harbor` debe existir y tener el deployment del endpoint. Créalo
> una vez: `atlas profile new harbor` y deja el `.env` accesible en el contenedor (§3).

---

## 5. Corrida

```powershell
# Smoke: 1-3 tareas, 1 concurrente
.\tools\harbor_atlas\run-harbor.ps1 -Concurrent 1

# Corrida completa
.\tools\harbor_atlas\run-harbor.ps1 -Concurrent 4
```

O directamente:

```bash
PYTHONPATH=tools uv run harbor run \
  --dataset terminal-bench@2.0 \
  --agent harbor_atlas.atlas_agent:AtlasAgent \
  --model <provider/model> \
  --n-concurrent 4
```

Los resultados quedan en `jobs/<job-id>/`:

```
jobs/<job-id>/
├── config.json
├── result.json          ← JobResult (lo que importa Atlas)
└── <trial-name>/result.json   ← TrialResult (por tarea)
```

---

## 6. Ingestión del baseline en Atlas

```powershell
atlas eval import jobs/<job-id>        # parsea JobResult/TrialResult → eval_runs/eval_cases
atlas eval report                      # detalle por caso + failure kinds
atlas eval metrics                     # pass_rate, tokens, $, model_reliability
atlas eval report <run-id>             # una corrida concreta
```

El importador (`eval/harbor.rs`) lee el esquema real: `JobResult`/`TrialResult`,
`agent_result`/`step_results[].agent_result` y `compute_token_cost_totals()`
(`n_input_tokens`, `n_cache_tokens`, `n_output_tokens`, `cost_usd`). Si el esquema cambia,
`atlas eval import` lo dirá con un error de campo; ajusta el parser con el fixture real.

---

## 7. Publicar el número

Añade la fila a `tools/harbor_atlas/README.md` (tabla harness×modelo) y a `README.md
§Evaluation baseline`:

| Harness            | Modelo           | Tareas | pass_rate | tokens | coste USD | fecha      |
| ------------------ | ---------------- | ------ | --------- | ------ | --------- | ---------- |
| terminal-bench@2.0 | <provider/model> | N      | 0.xx      | …      | …         | YYYY-MM-DD |

---

## 8. CI opt-in

`.github/workflows/eval-gate.yml` (ya existe) corre los **golden tasks offline**. Para el
baseline externo, añade un workflow **manual/scheduled** (`workflow_dispatch` + `schedule`)
con runners que tengan Docker, o publica los números en el README. **No** lo pongas como
gate de PR: Docker + creds no deben bloquear cada cambio.

---

## 9. Troubleshooting (fallos típicos reales)

| Síntoma                                                     | Causa                                   | Fix                                                                                                  |
| ----------------------------------------------------------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `cascade exhausted (NoFallbackConfigured) after 0 attempts` | el modelo del step no tiene deployment  | ya arreglado: `execute` hace override al deployment desplegado; verifica `--profile harbor` + `.env` |
| `FOREIGN KEY constraint failed` en `model_invocations`      | modelo runtime ausente en `models`      | ya arreglado (upsert en `record_model_invocation`)                                                   |
| `diff has no files` a repetición                            | modelo sin contexto / step de research  | ya arreglado: `WorkspaceContext` + skip honesto de steps sin diff                                    |
| El contenedor no alcanza el endpoint                        | red/firewall o modelo local no expuesto | usa HTTPS público o `host.docker.internal`                                                           |
| `atlas: command not found` en el contenedor                 | binario no copiado al PATH              | §3 (mount o COPY)                                                                                    |
| `docker: command not found`                                 | Docker no instalado                     | §0                                                                                                   |
| El modelo alucina rutas de fichero                          | prompt sin árbol del repo               | ya cubierto: el prompt incluye el file tree                                                          |
| Rate limit del provider                                     | muchas tareas concurrentes              | `-Concurrent 1`, o añade fallback a otro provider en `.env`                                          |

---

## 10. Qué demuestra cada cosa

- **Sin Docker (ya hecho):** el harness LLM funciona end-to-end contra un modelo real
  (`route → Diff → validate → repair → apply`). Ver commits `0d5cee7`, `0390f49`.
- **Con Docker (este runbook):** un **número comparable** (pass_rate/coste) frente a un
  benchmark público, que es lo que convierte a Atlas en medible y comparable (F32), y
  permite la comparativa antes/después por capacidad (F38).
