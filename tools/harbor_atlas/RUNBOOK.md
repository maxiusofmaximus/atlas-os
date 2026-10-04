# Runbook — Baseline externo de Atlas OS (F32 / F38)

Guía paso a paso para correr Atlas como **agente bajo Harbor** (Terminal-Bench 2.0 /
SWE-bench Verified) y **ingerir el número** en el store de evaluación.

> **Estado verificado (2026-10-03):** el camino **Docker vía WSL2** está **montado y
> probado**: Harbor 0.23.0 corre el adapter `oracle` contra Terminal-Bench 2 y saca
> **mean ≈ 0.88** con trials reales `reward=1.0`. Este runbook es el procedimiento que
> se ejecutó, no una teoría. Ver §1 (un solo comando instala todo).

---

## 1. Setup de Docker en WSL2 (verificado, por comandos)

Windows no necesita Docker Desktop: **WSL2 + Docker Engine dentro** basta.

```powershell
# 1.1 Instalar una distro WSL2 (una vez; requiere admin para el componente)
wsl --install -d Ubuntu-24.04 --no-launch

# 1.2 Verificar
wsl -l -v          # Ubuntu-24.04  Version 2
```

Dentro de Ubuntu (los scripts del repo lo automatizan):

```bash
wsl -d Ubuntu-24.04
# Docker Engine (docker.io de Ubuntu)
sudo apt-get update -qq && sudo apt-get install -y docker.io
# Docker Compose v2 (docker.io NO lo trae; Harbor lo necesita)
sudo mkdir -p /usr/local/lib/docker/cli-plugins
sudo curl -sSL "https://github.com/docker/compose/releases/latest/download/docker-compose-linux-x86_64" \
  -o /usr/local/lib/docker/cli-plugins/docker-compose && sudo chmod +x $_
# Arrancar el daemon
sudo dockerd >/var/log/dockerd.log 2>&1 &
docker run --rm hello-world        # sanity
docker compose version             # v5.x
```

> Los scripts `tools/harbor_atlas/wsl_setup.sh`, `wsl_compose.sh` y `wsl_smoke.sh`
> automatizan exactamente estos pasos.

---

## 2. Instalar Harbor y verificar

```bash
curl -LsSf https://astral.sh/uv/install.sh | sh    # uv/uvx
export PATH="$HOME/.local/bin:$PATH"
uv tool install harbor                             # harbor 0.23.0
harbor --version
```

**Smoke obligatorio sin gastar tokens** (agente `oracle` = solver de referencia):

```bash
cd /mnt/c/Users/Max/Desktop/atlas-os
harbor run -d terminal-bench/terminal-bench-2 -a oracle --n-concurrent 4
# -> mean ~0.88; 89 tareas; results en jobs/<job-id>/result.json
```

Si `oracle` no saca ~0.8+, el problema es Docker/Harbor, **no** Atlas.

---

## 3. Endpoint de modelo

El adapter de Atlas lee su endpoint de `.env` (`ATLAS_LLM_BASE_URL`, `ATLAS_LLM_MODEL`,
`NVIDIA_API_KEY`). Verifica **desde el host** antes de Docker:

```powershell
$e=@{}; Get-Content .env | % { if($_ -match '^\s*([A-Z_]+)\s*=\s*(.*)$'){$e[$matches[1]]=$matches[2].Trim()} }
Invoke-RestMethod "$($e.ATLAS_LLM_BASE_URL)/chat/completions" -Method Post `
  -Headers @{Authorization="Bearer $($e.NVIDIA_API_KEY)";'Content-Type'='application/json'} `
  -Body (@{model=$e.ATLAS_LLM_MODEL;messages=@(@{role='user';content='ping'});max_tokens=5}|ConvertTo-Json -Depth 5)
```

> El **contenedor** debe alcanzar el endpoint: NIM/Groq/Cerebras son HTTPS públicos
> (basta salida a red en WSL). Para Ollama/vLLM local usa `host.docker.internal`.

---

## 4. Binario `atlas` en la imagen (Linux ELF)

```bash
# Compila el binario Linux dentro de WSL (no en Windows)
cd /mnt/c/Users/Max/Desktop/atlas-os
cargo build --release --manifest-path src-tauri/Cargo.toml --bin atlas
# -> src-tauri/target/release/atlas  (ELF)
```

Opción A (bind-mount en la config de entorno de Harbor):

```yaml
environment:
  mounts:
    - {
        type: bind,
        source: /mnt/c/Users/Max/Desktop/atlas-os/src-tauri/target/release/atlas,
        target: /usr/local/bin/atlas,
        read_only: true,
      }
    - {
        type: bind,
        source: /mnt/c/Users/Max/Desktop/atlas-os/.env,
        target: /work/.env,
        read_only: true,
      }
```

Opción B (`Dockerfile` de la tarea): `COPY atlas /usr/local/bin/atlas && chmod +x …`.

> `atlas` es single-binary (sin Node/Python), se copia tal cual.

---

## 5. Corrida con el adapter de Atlas

```powershell
.\tools\harbor_atlas\run-harbor.ps1 -Concurrent 4
```

o dentro de WSL:

```bash
cd /mnt/c/Users/Max/Desktop/atlas-os
PYTHONPATH=tools harbor run \
  -d terminal-bench/terminal-bench-2 \
  --agent harbor_atlas.atlas_agent:AtlasAgent \
  --model <provider/model> \
  --n-concurrent 4
```

El adapter ejecuta por tarea:

```bash
atlas --profile harbor mission new --force "<instruction>"
atlas --profile harbor plan     "<mission_id>"
atlas --profile harbor execute  --coding --apply --root . "<mission_id>"
```

> **Nota de persistencia WSL:** un `harbor run` lanzado con `nohup &` muere al cerrar la
> sesión WSL. Lánzalo en una sesión viva, o con `wsl --exec`/`tmux`, o desde Windows con
> `wsl -d Ubuntu-24.04 -- <cmd>` y espera. Para runs largos: `tmux new -s harbor`.

---

## 6. Ingestión del baseline en Atlas

```powershell
atlas eval import jobs/<job-id>     # JobResult/TrialResult -> eval_runs/eval_cases
atlas eval report                   # detalle + failure kinds
atlas eval metrics                  # pass_rate, tokens, $, model_reliability
```

El importador (`eval/harbor.rs`) lee el esquema real: `JobResult` (claves
`id`, `started_at`, `finished_at`, `n_total_trials`, `stats`) y `TrialResult`
(`agent_result`/`step_results[].agent_result`, `compute_token_cost_totals()`).

---

## 7. Publicar el número

Tabla en `tools/harbor_atlas/README.md` + `README.md §Evaluation baseline`:

| Harness                         | Modelo           | Tareas | pass_rate | tokens | coste USD | fecha      |
| ------------------------------- | ---------------- | ------ | --------- | ------ | --------- | ---------- |
| terminal-bench/terminal-bench-2 | <provider/model> | N      | 0.xx      | …      | …         | YYYY-MM-DD |

---

## 8. Troubleshooting (fallos reales ya resueltos)

| Síntoma                                                     | Causa                          | Fix                                                      |
| ----------------------------------------------------------- | ------------------------------ | -------------------------------------------------------- |
| `docker: command not found`                                 | sin distro WSL                 | `wsl --install -d Ubuntu-24.04` (§1)                     |
| `unknown flag: --project-name`                              | Compose **v1**, no v2          | instalar plugin v2 (§1)                                  |
| `Cannot connect to the Docker daemon`                       | dockerd no arrancado           | `sudo dockerd &` (§1)                                    |
| `cascade exhausted (NoFallbackConfigured) after 0 attempts` | modelo del step sin deployment | ya arreglado (`execute` hace override) + perfil `harbor` |
| `FOREIGN KEY constraint failed` en `model_invocations`      | modelo runtime ausente         | ya arreglado (upsert)                                    |
| `diff has no files` repetido                                | modelo sin contexto            | ya arreglado: `WorkspaceContext` + skip                  |
| `atlas: command not found` en contenedor                    | binario no en PATH             | §4                                                       |
| Contenedor no alcanza el endpoint                           | red                            | HTTPS público o `host.docker.internal`                   |
| El run muere al cerrar la terminal                          | `nohup` no sobrevive WSL       | usa `tmux` (§5)                                          |

---

## 9. Qué demuestra cada cosa

- **Sin Docker:** harness LLM real (`route → Diff → validate → repair → apply`) vs un
  modelo real. Commits `0d5cee7`, `0390f49`.
- **Con Docker (este runbook):** un **número comparable** (pass_rate/coste) vs un benchmark
  público (F32), y la comparativa antes/después por capacidad (F38). **El harness Docker ya
  está montado y verificado con `oracle` (mean ≈ 0.88).**
