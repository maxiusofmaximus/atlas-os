# 23 - Prompt Understanding & Refinement

> RFC que resuelve el problema más caro de la IA de código: **el usuario es tonto** (o, dicho formalmente: underspecified, hypothetical, role-overloaded, capacity-hallucinated, lost-in-the-middle, low-information). Antes de ejecutar nada, un pipeline reconstruye la intención tras el prompt crudo, mide el confidence y solo entonces desbloquea la planificación.

---

## 0. Por qué este RFC existe

En todos los RFCs anteriores asumíamos que la *Mission* del usuario llegaba comprensible. Eso es falso en el 80% de los casos reales observados en producción. El usuario:

- **Resume la idea en su cabeza** y solo suelta pistas parciales ("hazlo mejor", "fix the bug", "go").
- **Presupone cosas** (un framework, un stack, una convención) sin decirlas.
- **Imagina que está volando por la galaxia** y pide capacidad imposible con las tools disponibles.
- **Cree que la IA puede construir algo que se escapa de sus manos** — por ejemplo, *"construir una casa con una impresora 3D que casi nadie conoce"* — pero no sabe cómo conseguir instalación / validarlo.
- **No tiene la manera o no la conoce** por gap de conocimiento del dominio.

Cualquier agente que actúe igualmente alucinará capacidad, tiempos o existencia de herramientas. **Ésta es la fuente #1 de alucinaciones en editores con IA**.

> *"LLMs struggle with open-domain queries exhibiting underspecified or uncertain expressions."* — Wang et al., ACL 2026 (ODUTQA-MDC, https://arxiv.org/abs/2604.10159)

Este RFC inserta un **Prompt Understanding Pipeline** entre el usuario y el Planning Engine (`12 - Planning Engine.md`). Es la primera línea de defensa contra la alucinación.

---

## 1. Anti-patrones del usuario — catálogo cerrado

Cada categorìa tiene un `gap_type` y una `clarification_strategy`. El detector del pipeline emite uno o más `gap_type` por prompt crudo.

| # | Anti-patrón | `gap_type` | Ejemplo | Strategy |
|---|---|---|---|---|
| C1 | Grandiosidad técnica imposible | `capacity_hallucination` | "volar por la galaxia" | Pedir confirmación de alcance + Research Engine |
| C2 | Herramienta / dependencia edge-case | `unknown_tool_dependency` | "impresora 3D que casi nadie conoce" | `probe_feasibility` del Research Engine |
| C3 | Instrucción subespecificada | `underspecified` | "Resume la idea en su cabeza" | K=3 preguntas STORM |
| C4 | Supuesto implícito | `implicit_assumption` | Sileasume un framework | Lista explicita: "Asumo X. ¿Correcto?" |
| C5 | Capacity overreach | `capacity_overreach` | "Construye algo que se escapa de manos" | Verbalizar límites; pedir aceptar Y |
| C6 | Gap de conocimiento del usuario | `user_knowledge_gap` | "No tiene manera o no la conoce" | Modo mentor + atajos |
| C7 | Low information prompt | `low_information_prompt` | "fix", "go", "ok" | Pasar al `ask` mode y exigir elección A/B/C |
| C8 | Lost in the middle (Liu et al. 2023) | `lost_in_the_middle` | Prompt largo con clave al medio | Reordenar contexto: verdict al inicio y al final |
| C9 | Role overload | `role_overload` | "Actúa como astronauta+abogado+cocinero" | Pedir elección de un rol |
| C10 | Omnisciencia asumida | `model_omniscience_assumption` | "Como IA ya sabes..." | Atribuir al Research Engine |

Mapeo `gap_type → resolution path`:

```
CAPACITY_HALLUCINATION        → Research Engine verify feasibility + ask to confirm scope
UNKNOWN_TOOL_DEPENDENCY       → Research Engine probe_feasibility (registry + stars + releases)
UNDERSPECIFIED                → generate K=3 clarification questions (STORM)
IMPLICIT_ASSUMPTION           → list assumptions, ask user to confirm
CAPACITY_OVERREACH            → verbalize limits, ask "do you accept Y?"
USER_KNOWLEDGE_GAP            → mentor mode + offer shortcuts
LOW_INFORMATION_PROMPT        → refuse to proceed without A/B/C answer
LOST_IN_THE_MIDDLE            → reorder context (verdict al inicio y al final)
ROLE_OVERLOAD                 → ask to pick one role
MODEL_OMNISCIENCE_ASSUMPTION  → route to Research Engine (RFC 10)
```

---

## 2. Pipeline (9 pasos)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  USER PROMPT (raw)                                                  │
│      │                                                                        │
│      ▼                                                                        │
│  1) Capture                  store raw + timestamp + session + mode            │
│      │                                                                        │
│      ▼                                                                        │
│  2) Parse Intent             NLU: intent, keys, NER, domain, tech,            │
│      │                       action, scope, implicit_signals                  │
│      │                       ToT K=3 hipótesis rankedas                       │
│      ▼                                                                        │
│  3) Detect Ambiguities       gap_type[] por categoría C-table                │
│      │                                                                        │
│      ▼                                                                        │
│  4) Search Similar Missions  Vector KB / Journal embedding lookup            │
│      │                       returns: similarity, outcome, reflexion          │
│      ▼                                                                        │
│  5) Generate Clarification   K=3, STORM multi-perspectiva                    │
│      │                       (developer/qa/ops/security)                      │
│      ▼                                                                        │
│  6) Auto-Resolve (optional)  Architecture Memory lookup                      │
│      │                       e.g. "fix the bug" → último test rojo           │
│      ▼                                                                        │
│  7) Consolidated Mission     Self-Refine 2 iter: criticar → reescribir       │
│      │                       + PublicUnderstandingVerdict                    │
│      ▼                                                                        │
│  8) Loop with user           si confidence < threshold → pulsar K preguntas  │
│      │                       y regenerar verdict sobre las respuestas         │
│      ▼                                                                        │
│  9) Locked Mission          Persist al Journal → Planning Engine (RFC 12)        │
└──────────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Justificación académica de cada paso

- **Self-Refine** (Madaan et al. 2023, https://arxiv.org/abs/2303.17651) — paso 7.
- **Reflexion** (Shinn et al. 2023, https://arxiv.org/abs/2303.11366) — paso 4 recupera memorias verbales de fallos pasados por similarity.
- **Tree of Thoughts** (Yao et al. 2023, https://arxiv.org/abs/2305.10601, código https://github.com/princeton-nlp/tree-of-thought-llm) — paso 2.
- **RePrompt** (Chen et al. 2024, https://arxiv.org/abs/2406.11132) — permite ajustar prompts futuros basado en feedback intermedio.
- **ODUTQA-MDC** (Wang et al. ACL 2026, https://arxiv.org/abs/2604.10159) — paper bisagra entre ambiguous-input y clarification-loop; toma su etiquetado fino para medir underspecification internamente.
- **LLM-Modulo** (Kambhampati et al. ICML 2024, https://arxiv.org/abs/2402.01817) — afirma que los LLM no pueden planificar solos; exige verifier externo. Por eso detrás del veredicto hay checkers deterministas (`probe_feasibility`, reachability checker, knowledge-gap detector).
- **LLM-as-judge** (Zheng et al. 2023, https://arxiv.org/abs/2306.05685) — confidence del verdict se juzga por rubric (HIGH/MEDIUM/LOW/BLOCK) con orden randomizado.
- **Lost in the Middle** (Liu et al. 2023, https://arxiv.org/abs/2307.03172) — `verdict` y `mission` se colocan al **inicio y al final** del prompt enviado al Planning Engine, nunca al medio.
- **STORM** (Shao et al. NAACL 2024, https://arxiv.org/abs/2402.14207) — generacióon de clarification preguntas desde 3 perspectivas distintas (no 3 desde la misma).
- **Survey of Agent Architectures** (Masterman 2024, https://arxiv.org/abs/2404.11584) — confirma orden fases: planificación > ejecución > reflexión; nuestra pipeline encaja antes de la planificación.

### 2.2 Verificadores externos (LLM-Modulo)

El LLM solo no basta. El pipeline siempre llama:

- **reachability checker** — ¿existe una trayectoria de tools-capabilities que puede lograr lo pedido en este proyecto?
- **knowledge-gap detector** — ¿qué entidades nombradas (NER) NO están en el `Project Map` / `Architecture Memory`?
- **probe_feasibility** (delegado al Research Engine, ver RFC 10) — ¿existen paquetes/releases/stars para los tools nombrados? ¿Hay papers que validen capacity_hallucination?

---

## 3. Estructura de datos — `PublicUnderstandingVerdict`

Persistido en el Journal con cada prompt crudo.

```typescript
type GapType =
  | 'capacity_hallucination'
  | 'unknown_tool_dependency'
  | 'underspecified'
  | 'implicit_assumption'
  | 'capacity_overreach'
  | 'user_knowledge_gap'
  | 'low_information_prompt'
  | 'lost_in_the_middle'
  | 'role_overload'
  | 'model_omniscience_assumption';

type ConfidenceLevel = 'HIGH' | 'MEDIUM' | 'LOW' | 'BLOCK';

interface ClarificationQuestion {
  id: string;
  perspective: 'developer' | 'qa' | 'ops' | 'security' | 'user';
  question: string;
  justification: string;           // cita el gap que la disparó
  format: 'multiple_choice' | 'text' | 'file_url' | 'tool_install_url';
  options?: string[];
  default_answer?: string;          // si step 6 lo rellena desde Architecture Memory
  auto_resolved: boolean;
  source?: { memory_id?: string; journal_id?: string };
}

interface PublicUnderstandingVerdict {
  verdict_id: string;
  session_id: string;
  raw_prompt: string;
  timestamp: string;               // ISO 8601

  // step 2 — parse
  intent: string;                   // 1 frase, lo que creemos que el usuario quiere
  intent_hypotheses: {              // ToT
    rank: number;
    text: string;
    feasibility_score: number;     // 0..1 por LLM-as-judge
    rejection_reason?: string;
  }[];
  keys: string[];
  named_entities: { text: string; type: string; in_kb: boolean }[];
  domain: 'frontend' | 'backend' | 'devops' | 'data' | 'ml' | 'docs' | 'unknown';
  technology: string[];
  desired_action: 'create' | 'modify' | 'debug' | 'explain' | 'test' | 'research' | 'refactor' | 'unknown';
  scope: 'file' | 'module' | 'feature' | 'project' | 'unknown';
  implicit_signals: string[];       // tokens: "should", "I think", urgencia

  // step 3 — gaps
  gaps: {
    type: GapType;
    evidence: string;               // cita literal del prompt del usuario
    severity: 'low' | 'med' | 'high' | 'blocker';
    auto_resolvable: boolean;
  }[];

  // step 4 — similar missions
  similar_missions: {
    journal_id: string;
    similarity: number;             // cosine 0..1
    outcome: 'success' | 'failed' | 'abandoned' | 'refined';
    reflexion?: string;
  }[];

  // step 5+6 — clarification
  clarification_questions: ClarificationQuestion[];

  // step 7 — verdict
  confidence: ConfidenceLevel;
  confidence_rubric: {
    intent_clarity: number;       // 0..1
    scope_clarity: number;
    feasibility_clarity: number;
    context_clarity: number;
  };
  observations: string[];
  recommended_mode: 'ask' | 'architect' | 'code' | 'context';

  // provenance
  model_id: string;
  judge_model_id?: string;
  elapsed_ms: number;
}
```

---

## 4. Estructura de datos — `MissionConsolidated`

Input formal del Planning Engine. El Planning **no** acepta `locked=false`.

```typescript
interface MissionConsolidated {
  mission_id: string;
  verdict_id: string;
  generated_at: string;

  mission_statement: string;         // 1-3 frases, voz activa
  success_criteria: string[];        // condiciones observables
  non_goals: string[];               // exclusiones para evitar scope creep

  accepted_assumptions: {
    assumption: string;
    user_confirmed: boolean;
    auto_resolved: boolean;
    source: 'architecture_memory' | 'journal' | 'user_answer';
  }[];

  // forward hints
  suggested_mode: 'ask' | 'architect' | 'code' | 'context';
  suggested_tooling: string[];
  forbidden_actions: string[];       // p.ej. "do not delete migrations"
  requires_research_first: boolean;  // route to RFC 10 antes de planificar
  research_queries?: string[];

  // lock state
  locked: boolean;
  locked_at?: string;
  locked_by: 'user' | 'auto_threshold';

  planning_session_id?: string;
  execution_session_id?: string;
}
```

---

## 5. Política de confidence

| Confidence | Acción | Efecto en Execution Mode (`21`) |
|---|---|---|
| `HIGH ≥ 0.85` | Pasa directo a Planning | `code` mode permitido |
| `0.6 ≤ MEDIUM < 0.85` | Pasa pero fuerza `suggested_mode=architect` (dual-model de Aider) | `architect` mode |
| `0.3 ≤ LOW < 0.6` | Bloquea avance. Pregunta K=1. | `ask` mode |
| `< 0.3 o BLOCK` | Bloqueo duro. Pregunta K=3. No bypass sin respuesta. | `ask` mode (no bypass) |

El valor del rubric (`confidence_rubric`)ажд componentales juzgada por **LLM-as-judge con rubric fijo + orden randomizado** para evitar position bias (cf. Zheng 2023).

---

## 6. Métricas

- **re-prompt rate within 60s** — proxy directo de underspecification (idea tomada de Copilot Chat telemetry).
- **clarification questions asked per session** — distribución por `gap_type`.
- **auto-resolve hit rate** — cuando Architecture Memory rellenó y el usuario aceptó.
- **distribution of gap_types** — para ajustar el detector.
- **velocity HIGH** — % de prompts que llegan a HIGH sin intervention. North star.

Cada `verdict` se persiste en Journal con `was_correct` (determinado por Learning Engine con el outcome final). Sirve para entrenar futuros clasificadores de `gap_type`.

---

## 7. Comando `/refine` y skill `prompt-clarify`

### 7.1 Comando `/refine`

```
/refine [--force] [--dry-run] [--max-questions K] [--perspective p1,p2,...]

--force           sobrescribe el `locked=false` y manda al Planning igualmente.
--dry-run         produce verdict + mission pero no envía. Útil para depurar.
--max-questions K límite de clarification questions (default 3).
--perspective     lista de perspectivas para STORM (default: developer,qa,ops).
```

### 7.2 Skill `prompt-clarify`

Skill ubicada en `~/.opencode/skills/prompt-clarify/`. Se activa automáticamente cuando `verdict.confidence < HIGH`. Hace exposes en la UI:

- snippet con las K preguntas pre-llenadas,
- button "Aceptar todas las auto-resoluciones",
- toggle "Modo mentor" (avisa al detector de `user_knowledge_gap` para mostrar texto didáctico).

### 7.3 Skills como graph templates (RFC 28 §C — behind `dag_mode`)

Cuando la mission tiene `dag_mode=true`, una skill puede declarar un **4º archivo** `graph.toml` junto a su `skill.toml`/`program.md`/`tools.json`. Formato:

```toml
# ~/.opencode/skills/<skill>/graph.toml
nodes = [
  { id = "probe",     kind = "engine_state", label = "PromptUnd degap"     },
  { id = "gap",       kind = "engine_state", label = "Classify gap_type"   },
  { id = "refine",    kind = "engine_state", label = "Refine prompt"       },
]
edges = [
  { from = "probe",  to = "gap",    kind = "transitions_to", precondition = "verdict.confidence < HIGH" },
  { from = "gap",     to = "refine", kind = "transitions_to", guard = "gap_type != user_knowledge_gap" },
]
```

Loader `src-tauri/src/skills/loader.rs` inserta el template en M15 `mission_graph_nodes`/`edges` con IDs fresh (prefijo `{skill_id}-{mission_id}-`). Tags provenance = `INFERRED` porque son spec de skill, no ingest AST (que sería `EXTRACTED`). Skills que ya existen (`prompt-clarify`, autoresearch `program.md`, etc.) podrán `graph.toml`-ar su state machine cuando el DAG mode estabilice en Phase 2; en Phase 1.5c esto es **sólo spec** — el loader es RFC 28 §C item 5 (pending).

---

## 8. Modos de uso (mapeo Aider-style)

| Modo | Confidence requerido | Comportamiento |
|---|---|---|
| `ask` (readOnly) | LOW/BLOCK | Modo default. Bloqueado por question loop. No edita archivos. |
| `architect` (dual-model) | MEDIUM | Architect model propone, editor model ejecuta (patrón Aider `/architect`). |
| `code` (default executor) | HIGH o `locked=true` | Ejecuta normalmente. |
| `context` (auto-expand) | `lost_in_the_middle` detectado | Reordena contexto antes de enviar al Planning. |

Comandos análogos a Aider (https://aider.chat/docs/usage/commands.html): `/ask`, `/architect`, `/code`, `/context`, `/reasoning-effort low|medium|high`.

---

## 9. Modos de ejecución vs. modo de uso

No confundir:
- **Execution Modes** (`21 - Execution Modes.md`): cuán autónomo es el agente. (`MANUAL_CLASSIC`, `HUMAN_IN_LOOP`, `AUTOPILOT`, `AUTONOMOUS`).
- **Modos de uso** (este RFC, §8): cómo interpretar/registrar el intent. (`ask`, `architect`, `code`, `context`).

Son ortogonales. El Planning Engine puede estar en `AUTONOMOUS` (resource mode) pero con `modoe de uso = architect` (architect propone, editor ejecuta) si `confidence = MEDIUM`.

---

## 10. Outputs hacia otros RFCs

- `02 - Agent Operating System.md` — el Kernel debe route todo `task.received` por el Prompt Understanding Pipeline antes de planner.
- `10 - Research Engine.md` — añadir capability `probe_feasibility` consumida por el paso 6.
- `12 - Planning Engine.md` — input becomes `MissionConsolidated`, no `raw prompt`.
- `21 - Execution Modes.md` — el `ask`/`architect`/`code`/`context` se vuelve un hint para el Reasoning Engine.
- `24 - HUD Mission Control.md` — `verdict.confidence` se visualiza como badge en cada Mission/subagent.
- `19 - Execution Supervisor.md` — `goal_drift` recovery incorpora el `intent` original del verdict como ancla.

---

## 11. Limitaciones explicitas

- El Pipeline **no substituye al LLM-Modulo** (Kambhampati 2024). Es una capa previa. El LLM sigue sin poder planificar solo.
- Caso de "construir casa con impresora 3D edge-case" requiere `probe_feasibility` que apunte a search en arxiv/openreview/academic sources — no solo npm/pip. Hay que añadir fuentes.
- `confidence = HIGH` **no garantiza éxito**; solo garantiza que **entendimos** lo que el usuario pidió. La ejecución puede fallar igual.
- Si el usuario rechaza K veces el `consolidated mission`, se escala al humano con etiqueta `user_intent_unresolvable` — y no se procede.

---

## 12. Estado

- Status: Draft v1
- Depends on: `02`, `10`, `12`, `14`, `19`, `21`, `24`
- Cubre explícitamente el pedido del usuario de "suponer que el usuario es tonto y refinar el prompt".
