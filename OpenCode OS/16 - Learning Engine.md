# 16 - Learning Engine

El motor que hace que **el editor mejore solo, no solo el proyecto**. Cada error produce una regla nueva y un patrón cacheado para nunca repetirlo.

---

## 1. Funciones

```
Learning Engine
   ├── detecta errores repetitivos
   ├── genera nuevas reglas
   ├── ajusta prompts
   ├── mejora estrategias
   └── registra conocimiento
```

## 2. Reflection Loop (canónico)

```
Bug
   ↓
Root Cause
   ↓
Qué regla faltó
   ↓
Nueva regla (draft)
   ↓
Validación (Voto + Tests)
   ↓
Guardar Harness
   ↓
Nunca repetir
```

Cada regla tiene lifecycle:
- draft (priority 0, verified false),
- candidate (priority 30, verified by reviewers),
- active (priority 60+, verified true, runs N veces correctas),
- deprecated.

## 3. Tipos de aprendizaje

### Por error
Errores reparados por Repair producen reglas:
```yaml
rule:
  id: r-2026-07-04-001
  when:
    stage: validation.biome
    pattern: "no-unused-vars"
    lang: typescript
  then:
    skill: react-doctor
    diff_hint: "remove unused export"
  confidence: 0.91
  evidence: ["rr-001-ref"]
```

### Por usuario
Si el usuario hace override de una skill (elige "USA ESTA y no esta") el diferencia de signals:
- disminuye `priority` de la skill rechazada,
- empuja embeddings de la preferida.

### Por performance
Si un modelo en particular acierta el 90% en un dominio, el Model Orchestrator lo prioriza para ese rol (entropía del uso).

### Por consenso social
Research Run Reports exitosos generan notas de Collective Engineering Intelligence (CEI), cacheadas en el Vector KB.

### Por grafo de misión exitoso (RFC 28 §C — structural graph diffing)

Persistimos **instancias exitosas de `mission_graph`** en la tabla M15 `learning_graphs` (`src-tauri/src/journal/schema.rs`), con key `(intent_signature, success)` y embedding de `intent_signature` generado por `fastembed-rs` (RFC 25 §3.5). En una nueva mission:

1. retrieve top-k graphs por cosine en `intent_signature` sobre `sqlite-vec` (RFC 09).
2. diff sus edges contra el DAG propuesto por el Planner (RFC 12 §3.1).
3. **inyectar edges faltantes demostrados-exitosos** como hints `INFERRED` (tags provenance = `INFERRED` en `mission_graph_nodes`).
4. Grafos de fallo se persisten mirrored con `outcome=failed` y sirven como anti-patterns (alertas al Planner para evitar la misma topología).

Esto importa graphify's *"edges-as-first-class-objects"* a **trayectorias de agentes**, no código fuente. Detrás de feature `dag_mode` (default off) hasta validar el contrato contra runs reales. Setup visual en RFC 24 §13 Graph View (RFC 28 §C item 7).

## 4. Auto-reglas del harness

El Learning Engine escribe:
- `.opencode/rules/*.yaml` (reglas scoped al proyecto),
- `~/.opencode/rules/*.yaml` (reglas usuario),
- efectos `--inMemoryRules` persistidos también.

Estas reglas no son texto libre: son **JSON/YAML parseable por el Reasoning Engine y la Validation Engine**. A futuro convertibles a Semgrep rules reales.

## 5. Compresión de Skills

El Learning Engine orquesta la compresión de Skills (`06 - Skills.md`):
- detecta pares con similitud alta,
- propone fusión,
- emite diff de Skill Graph,
- escapa al usuario si `verified=false`.

El editor **mejora solo**.

## 6. Ajuste de prompts

El Learning Engine ajusta templates:
- si una plantilla de prompt repite fallos, el engine la reescribe y la guarda como `prompt_template version+1`.
- mantiene off-line el diff en el Journal para auditar.

## 7. Métricas

`was_correct`, `was_blocked`, `tests_passed`, `models_used`, `approved_by_user`. Los dashboards del Command Center los pintan.

## 8. Politica de seguridad de aprendizaje

Reglas auto-generadas no se aplican a rutas críticas (security, deploy) sin verificación. Si tocan Security Layer, requieren aprobación manual.

## 9. Reflection Engine (alias dedicado)

Cuando el meta-razonamiento marque self-doubt, ese evento se convierte en input explícito al Learning Engine para "registrarlo como una lección".
