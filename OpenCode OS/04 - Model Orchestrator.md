# 04 - Model Orchestrator

El motor que decide **qué cerebro** piensa cada paso. Es el pilar de la diferenciación frente a editores mono-modelo.

> No son mejores porque tengan un mejor prompt. Son mejores porque saben **cuándo cambiar de cerebro**.

---

## 1. Registro de modelos

El orquestador mantiene un registry de modelos. Cada entrada:

```json
{
  "id": "claude-sonnet-4.5",
  "provider": "anthropic",
  "protocol": "anthropic",            // gemini | anthropic | openai | bedrock | ollama | custom
  "kind": "remote",
  "tier": "frontier",
  "context_window": 200000,
  "capabilities": ["text", "vision", "function_calling", "image_generation"],
  "strengths": ["reasoning", "architecture", "long_context"],
  "weaknesses": ["cost", "rate_limits"],
  "cost_per_1k_tokens_in": 0.003,
  "cost_per_1k_tokens_out": 0.015,
  "latency_ms_p50": 900,
  "available": true,
  "credentials_env": "ANTHROPIC_API_KEY",
  "api_keys": ["ANTHROPIC_API_KEY", "ANTHROPIC_API_KEY_2"],   // multi-key pooling
  "runtime_options": {                                         // agent-defined runtime knobs
    "thought_level": ["minimal", "default", "extended"],
    "modes": ["code", "plan", "review"]
  },
  "capability_tags": ["reasoning", "long_context", "vision"]
}
```

> **Lecciones integradas de AionUI** (`iOfficeAI/AionUi`, Apache-2.0, 29.3k★): AionUi no rutea automáticamente por complejidad/costo/latencia — delega al usuario la selección manual de modelo por conversación o por *assistant preset*. OpenCode OS convierte ese routing en un motor automático, que es **la invención diferencial #1**. Hemos incorporado de AionUi: el *capability tag system* (`text|vision|function_calling|image_generation`), el *protocol auto-detection*, el *multi-key rotation con blacklist de 90s* (ver §3) y las *runtime options* (thought level / mode) que algunos backends exponen.

### Proveedores soportados por defecto
| Proveedor | Tier | Tipo |
|---|---|---|
| Anthropic (Claude) | Frontier / de pago | Remoto |
| OpenAI / o3 | Frontier / de pago | Remoto |
| Google AI (Gemini) | gratuito + de pago | Remoto |
| DeepSeek | pagado | Remoto |
| Mistral | gratuito + pagado | Remoto |
| OpenRouter (agregador) | página de cada modelo | Agregador |
| GitHub Models | gratuito | Remoto |
| Nvidia NIM | gratuito | Remoto |
| Cerebras | gratuito / ultra rápido | Remoto |
| Groq | gratuito / ultra rápido | Remoto |
| Sambanova | gratuito / ultra rápido | Remoto |
| Cloudflare Workers AI | gratuito | Edge |
| HuggingFace Inference | gratuito | Remoto |
| LM Studio / Ollama / llama-server | Local (7B–70B) | Local |
| Kimi K2.7 (en swarm) | Experimental | Remoto |

### Registry extensible
El usuario puede añadir proveedores de pago o cualquier otro endpoint compatible.

## 2. Política de selección

El orquestador analiza la tarea y ejecuta:

```
Analiza la tarea
   ↓
Clasifica (dominio + riesgo + tamaño)
   ↓
Candidatos compatibles
   ↓
Filtra por restricciones del usuario
   (modo local-only / free-only / mixto)
   ↓
Estima cost / tiempo / confidence esperado
   ↓
Escoge el mejor cerebro
   ↓
Verifica resultado
   ↓
Si falla → escoge otro modelo
   ↓
Fusiona respuestas si procede
   ↓
Devuelve una sola decisión
```

### Mapa de afinidad (por defecto, configurable)

| Tipo de tarea | Modelo sugerido |
|---|---|
| Arquitectura / diseño | Claude (long context + reasoning) |
| Refactor gigante | Gemini (gran ventana) |
| Backend TypeScript | GPT |
| Bug difícil | o3 / DeepSeek (reasoning) |
| Búsqueda masiva | DeepSeek |
| Razonamiento matemático / lógica | DeepSeek |
| Comparación / votación | enjambre + votación |
| Comparación semántica | Qwen |
| Generación rápida y barata | Cerebras / Groq / Sambanova |
| Offline / privacidad | Local 7B–70B en GPU del usuario |
| Documentación / resumen | Local gratuito o Mistral |

## 3. Fusión y votación

Para decisiones críticas el orquestador puede:

1. Llamar a **N modelos** en paralelo (por ejemplo Claude, Gemini, GPT, DeepSeek).
2. Recoger sus respuestas estructuradas.
3. Aplicar uno de:
   - **Votación mayoritaria** con peso por Confidence.
   - **Fusión** (mezcla de outputs si son compatibles).
   - **Debate** (un modelo critica al otro, iteran, finalmente emiten).
4. Devolver **una** decisión al caller.

### Ejemplo
```
Decisión: ¿Strategy, Factory o Context para el patrón de estado?
  ↓
   Claude      → Strategy (Conf 0.91)
   Gemini      → Strategy (Conf 0.88)
   GPT         → Factory  (Conf 0.62)
   DeepSeek    → Strategy (Conf 0.90)
  ↓
Votación ponderada → Strategy
Confidence fuse → 0.89
```

## 4. Fail-over

- Si un modelo devuelve timeout / rate-limit / hallucination detectada, se marca `available: false` temporalmente.
- Se reintenta con el siguiente candidato.
- Tras 3 fallos consecutivos se escapa hacia arriba (usuario o Planning Engine) para replanificar.

## 5. Restricciones de modo

El orquestador **nunca** debe usar un modelo fuera del sub-modo activo:
- sub-modo `local-only` ⇒ filtra `kind == local`.
- sub-modo `free-only` ⇒ filtra `tier in {free, free_tier}`.
- sub-modo `mixto` ⇒ acepta cualquier `tier`, respetando el presupuesto definido.

## 6. Presupuesto y límites

El usuario define:
- `max_tokens_per_minute`
- `max_cost_per_hour`
- `max_concurrent_remote_calls`
- `max_local_concurrency` (en función de RAM / VRAM libres)

El orquestador aplica back-pressure y cola si se superan.

## 7. Routing de inferencia por skill

El Skill Graph almacena `compatible_models` y `recommended_models`. El orquestador **prioriza** los recomendados al disparar una skill. Las Skills de diseño favorecen Gemini o Claude; las skills de tests priorizan modelos rápidos; las de lógica priorizan razonadores.

## 8. Telemetría

Cada invocación registra en el Journal:
- modelo, proveedor, latencia, tokens, coste,
- prompt template usada, skill disparadora,
- Confidence emitido, validación posterior,
- `was_correct` (booleano determinado por Learning Engine).

Esto retroalimenta la afinidad del orquestador con el tiempo → el sistema **aprende** qué modelo se porta mejor para cada clase de tarea en cada proyecto.
