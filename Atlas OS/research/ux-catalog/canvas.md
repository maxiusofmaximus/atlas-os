# ux-catalog / canvas (canvas de nodos)

Fichas de referentes con **primario = canvas de nodos / grafo**. Procedencia como en `terminal.md`.
(Zeta nota: **Cate** tiene su ficha completa en `ade-orquestador.md`.)

---

## Flowise — canvas de agentes/LLM — prioridad A — [Os]

- **URL (en vivo):** https://docs.flowiseai.com/ [Os]
- **Funcionalidades clave:** "**Visual editor**" de orquestación que soporta modelos open/propietary, **expressions, custom code, branching/looping/routing logic**; conecta +100 fuentes/tools/vector DBs/memorias; **MCP client/server nodes** (tool listing, SSE, auth); API + JS/Python SDK + CLI [Os].
- **Layout y navegación:** canvas de flujo por nodos con nodos de MCP, memoria, RAG; **template marketplace** y componentes reutilizables [Os].
- **Estados y feedback:** "**Monitoring: Execution logs, visual debugging**, external log streaming" [Os].
- **Aprobaciones / HITL:** "Safety & Control: input moderation & output post-processing"; security controls **RBAC/SSO/rate limit** [Os].
- **Atajos de teclado:** [P]. **Onboarding:** template marketplace + embedded/share chatbot [Os]. **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **canvas con visual debugging (execution logs por nodo)** + branching/looping/routing y **MCP nodes**; evitar [P].

## Dify — canvas de agentes/workflows — prioridad A — [Os parcial]

- **URL (en vivo):** https://docs.dify.ai/ [Os]
- **Funcionalidades clave:** "open-source platform for building AI applications. Create **agents, agentic workflows, and chatbots** that draw on your own data, then **publish them as web apps or integrate them through APIs**" [Os].
- **Layout y navegación:** editor de **agentic workflows** por nodos (canvas) [Os].
- **Estados y feedback:** [P] (trazas por nodo no citadas en la home de docs).
- **Aprobaciones / HITL / Atajos / Accesibilidad:** [P]. **Onboarding:** publish web/API [Os].
- **Adoptar / Evitar para Atlas:** adoptar **canvas de workflow + publish web/API**; evitar [P] hasta abrir `/en/guides/workflow`.

## Langfuse — agent graph (Aggregated↔Expanded) — prioridad A — [Os]

- **URL (en vivo):** https://langfuse.com/docs/observability/features/agent-graphs [Os]
- **Funcionalidades clave:** "The same trace can be drawn two ways. Switch between them with the **Aggregated / Expanded toggle** in the top-left corner of the graph; **your choice is remembered across traces**" [Os].
- **Layout y navegación:** toggle en la esquina superior izquierda; un nodo por **step-name** (Aggregated, default) vs por **call** (Expanded) [Os].
- **Estados y feedback:** **Aggregated** dibuja **loops como ciclos** (edges que vuelven); **Expanded** los **desenrolla en un DAG** en orden de ejecución; "Aggregated to understand structure, Expanded to walk through a single run" [Os].
- **Aprobaciones / HITL:** [P]. **Atajos:** toggle de UI (no tecla citada) [P]. **Onboarding:** [P]. **Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **Aggregated↔Expanded con preferencia recordada** para el canvas/grafo (RFC 66 §10); evitar [P].

## n8n — canvas de nodos (+ log acoplado) — prioridad A — [Os parcial]

- **URL (en vivo):** https://docs.n8n.io/ (índice) [Os] + `docs/design/CONSENSUS_AUDIT.md` §1.3 #7 [Link]
- **Funcionalidades clave:** canvas de workflow por nodos; `docs.n8n.io` es el índice de la doc [Os].
- **Layout y navegación:** canvas con **log de ejecución acoplado abajo** y **selección bidireccional sincronizada** (canvas ↔ log) [Link].
- **Estados y feedback:** **auto-expand del grupo con error** al fallar; ejecución histórica conserva la agrupación [Link]. [P] en la home indexada.
- **Aprobaciones / HITL / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** adoptar **canvas↔log sync con log docked** (lo más fuerte de la categoría, RFC 66 §10); evitar [P].
- **Nota:** ya auditado en `CONSENSUS_AUDIT` §1.3 #7; esta ficha referencia ese [Os].

> **Cate** — canvas infinito: ficha completa en `ade-orquestador.md` §Cate [Os]. No se duplica aquí.
