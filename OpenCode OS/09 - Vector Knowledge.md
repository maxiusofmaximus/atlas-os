# 09 - Vector Knowledge

La **memoria semántica** del sistema. Permite que capacidades de inferencia funcionen a *pleno pulmón y sin demora*, con velocidad y sostenibilidad.

---

## 1. Qué se vectoriza

Cualquier unidad de conocimiento que el agente deba encontrar rápido:

| Origen | Unidad indexada |
|---|---|
| Skills | una skill completa + metadatos |
| MCPs | cada tool expuesta |
| CLIs | cada comando registrado |
| Journal | cada checkpoint / decisión |
| TODO / Mission | cada objetivo y subobjetivo |
| Node del proyecto | cada archivo, función, módulo (embeddings locales) |
| Docs del proyecto | README, ADRs, RFCs, CHANGELOG |
| Investigaciones pasadas | cada Research Run |
| Aprendizajes | cada regla nueva del Learning Engine |

## 2. Estructura

```
VectorKnowledgeBase
├── store: Qdrant | SQLite-vec | FAISS (local) | cloud (extensible)
├── embedding_model: default local (e.g. nomic-embed-text via Ollama)
├── dimensions: 768
├── collections:
│    - skills
│    - tools (mcp + cli)
│    - journal_entries
│    - code_chunks
│    - docs
│    - research_runs
│    - learnings
├── metric: cosine
└── payload_schema: { kind, engine, ts, source_id, summary }
```

## 3. Recuperación con scoring

```
Query: "validar accesibilidad en este componente"
   ↓
Embeddings de la query
   ↓
top-K por colección con score
   ↓
Hybrid: cosine score + priority + freshness + confidence
   ↓
Resultados al Planner / Reasoning
```

## 4. Velocidad

- Embedding local por defecto para no pagar latencia.
- Hot-cache en RAM (`max 200MB`) para consultas repetidas.
- Re-embedding diferido: cuando una skill cambia, se marca `stale` y se reindexa en idle.
- Compresión cuantizada (int8) para matrices grandes.

## 5. Sostenibilidad

- Tamizado frecuente: entradas con `priority=0` y sin uso en 30 días se purgan.
- Compresión de skills similares colabora (ver `06 - Skills.md`).
- El Learning Engine aprende qué consultas son frecuentes → preload predictivo en caliente.

## 6. Privacidad

- Para proyectos privados, todo el Vector KB es **local**, encriptado en reposo (ver `18 - Security.md`).
- En modo mixto puede usarse un proveedor cloud de embeddings, pero el usuario lo aprueba y nunca se envían datos del proyecto (solo descripciones de skills).

## 7. Indexación en segundo plano

El Context Engine dispara:
- Indexación de git diffs al abrir el workspace.
- Reindexación de archivos cambiados en cada guardado (de forma diferida).
- Asociación de embeddings a entradas del Journal.

## 8. Consulta desde otros motores

API canónica:
```
VK.recall(query: string, k: int, filters: Filter) -> Hit[]
VK.fetch(id: string) -> Entry
VK.upsert(entry: Entry) -> id
VK.stats() -> { collections, vectors, cache_hits }
```

Cualquier motor la usa. Por ejemplo el Planning Engine hace consultas semánticas al Journal para no repetir errores del pasado.
