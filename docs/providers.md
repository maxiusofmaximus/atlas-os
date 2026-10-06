# Providers — conectar un modelo

Atlas llama a **cualquier endpoint OpenAI-compatible**. El backend de una
ejecución sale de **variables de entorno**; la **clave** de un proveedor vive en
el **llavero del SO** (nunca en `.env`, archivos ni logs) y se resuelve
`env var → keychain` (`orchestrator::client::resolve_api_key`).

## Variables de runtime

| Variable                | Ejemplo                         | Qué es                                                                                                                      |
| ----------------------- | ------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `ATLAS_LLM_BASE_URL`    | `https://opencode.ai/zen/go/v1` | Base OpenAI-compatible (sin `/chat/completions`)                                                                            |
| `ATLAS_LLM_MODEL`       | `deepseek-v4.1-flash`           | Id del modelo                                                                                                               |
| `ATLAS_LLM_API_KEY_ENV` | `OPENCODE_GO_KEY`               | (Opcional) slot de la clave. Para `opencode.ai` se **autodetecta** (`/go/` → `OPENCODE_GO_KEY`, resto → `OPENCODE_ZEN_KEY`) |

La clave se guarda desde la **vista Settings** de la app o con
`atlas secrets set <slot>`. Ver RFC 25 §3.10.

## OpenCode Go (suscripción)

Base: **`https://opencode.ai/zen/go/v1`** (compatible OpenAI). Pega tu clave en
el slot **`OPENCODE_GO_KEY`** (Settings → Provider API keys).

Modelos usados para el **benchmark** (RFC 63):

- `deepseek-v4.1-flash`
- `mimo-v2.6-flash`
- `muse-spark-1.3-contributor` (**free**)

El catálogo en vivo: `GET https://opencode.ai/zen/go/v1/models`.

> **Requisitos de Go (ya aplicados):** Go exige `x-opencode-session` (un id
> **estable por conversación**, para routing y prompt-cache) y un **User-Agent**
> propio (no genérico de librería). Atlas envía ambos automáticamente para las
> bases `opencode.ai`.
>
> **Protocolo por modelo:** `deepseek-*`, `mimo-*`, `glm-*`, `kimi-*`, `longcat-*`
> usan `/chat/completions` (lo que usa Atlas). `grok-*`/`gpt-*-luna` usan
> `/responses`; `minimax-*` usa `/messages`; `muse-spark-*` **no** acepta
> `/chat/completions` (`ModelProtocolUnsupported`). Para el benchmark usa
> **`deepseek-v4.1-flash`** o **`mimo-v2.6-flash`**.

```powershell
$env:ATLAS_LLM_BASE_URL = "https://opencode.ai/zen/go/v1"
$env:ATLAS_LLM_MODEL    = "deepseek-v4.1-flash"   # o mimo-v2.6-flash / muse-spark-1.3-contributor
atlas agent --verify --root . --goal "..."
```

## OpenCode Zen (pay-as-you-go)

Base `https://opencode.ai/zen/v1` → slot `OPENCODE_ZEN_KEY`. Mismo protocolo.

## Otros proveedores

- **NVIDIA / Cerebras / Groq / OpenAI**: define la base con `ATLAS_LLM_BASE_URL`
  y expón `NVIDIA_API_KEY` / `CEREBRAS_API_KEY` / `GROQ_API_KEY` / `OPENAI_API_KEY`
  (el slot se detecta por la base). También puedes forzar el slot con
  `ATLAS_LLM_API_KEY_ENV`.
- **Local** (Ollama, Strata, vLLM…): base `http://127.0.0.1:<puerto>/v1`; la clave
  puede quedar vacía.
