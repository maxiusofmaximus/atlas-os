# ux-catalog / observabilidad

Fichas de referentes prioridad A con **primario = observabilidad / evals / gateway**.
Procedencia como en `terminal.md`. Los ya auditados en `CONSENSUS_AUDIT.md` no se repiten (enlaces al final).

> **Pasada 4:** fichas subidas a [Os] real abriendo la doc oficial en vivo (URL citada). El bundle `TruLens / Comet Opik / Lunary / W&B Weave` se **desdobló** en una ficha por producto.

---

## LangSmith — observabilidad LLM — prioridad A — [Os]

- **URL (en vivo):** https://docs.smith.langchain.com/observability [Os]
- **Funcionalidades clave:** "full visibility into your LLM application: from individual traces to production-wide performance metrics. Traces are the record of what your agents did in production." Integraciones "including OpenAI, Anthropic, CrewAI, Vercel AI SDK, Pydantic AI, and more" [Os].
- **Layout y navegación:** **vistas de traza conmutables** — "Inspect threads and runs in the Messages, Turns, and Details views." [Os].
- **Estados y feedback:** "Configure automations — Automate workflows with rules, webhooks, and online evaluations." + "Find and fix failures with Engine — Automatically detect recurring issues in your traces, diagnose their root cause." [Os].
- **Aprobaciones / HITL:** "Collect feedback — Annotate outputs and gather user feedback using **queues** or **inline annotation**." (revisión humana por cola o en línea) [Os].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** "Platform setup … choose between cloud, hybrid, or self-hosted. All options include observability, evaluation, prompt engineering, and deployment." [Os].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) **vistas conmutables del mismo run** (Messages/Turns/Details) — análogo al `Aggregated↔Expanded` de Langfuse; candidato directo para la Timeline/Audit de RFC 24/65; (b) **colas de anotación HITL** (queue + inline) como modelo de la ApprovalQueue; (c) **automations** (reglas/webhooks) para el cost/health de RFC 24.
- **Evitar para Atlas:** [P] (la doc no describe límites de UX a evitar).

---

## Arize Phoenix — observabilidad + evals — prioridad A — [Os]

- **URL (en vivo):** https://docs.arize.com/phoenix [Os]
- **Funcionalidades clave:** tracing, evaluación, ingeniería de prompts, datasets & experimentos [Os]. Ingesta por **OpenTelemetry (OTLP)** con auto-instrumentación para frameworks (LlamaIndex, LangChain, DSPy, Mastra, Vercel AI SDK) y providers [Os].
- **Layout y navegación:** UI con **trazas paso a paso** ("see what happened during a run, step by step: model calls, retrieval, tool use, custom logic"); evals en la UI con **scores sobre traces & spans** (evaluadores LLM, checks de código, etiquetas humanas) [Os].
- **Estados y feedback:** "where time is spent" por span; identificar fallos/regresiones [Os].
- **Aprobaciones / HITL:** **human labels** como fuente de evaluación (HITL analítico, no de ejecución) [Os].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** auto-instrumentación; comparativa Phoenix vs Arize AX (OSS vs managed) [Os].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) **traza paso a paso** como objeto central (model calls / retrieval / tool use) — el "execution log" que RFC 66 pide sincronizar con el canvas; (b) **scoring sobre spans** (LLM/código/humano) como base del health/eval de RFC 24/65.
- **Evitar para Atlas:** [P].

---

## Helicone — gateway + observabilidad — prioridad A — [Os]

- **URL (en vivo):** https://docs.helicone.ai/ [Os]
- **Funcionalidades clave:** **AI Gateway OpenAI-compatible** con 100+ modelos (OpenAI, Anthropic, Vertex, Groq…) y **logging/observabilidad + fallbacks automáticos** [Os].
- **Layout y navegación:** cuenta → API key → requests con logging automático; el panel de plataforma explica retos comunes [Os]. Vistas concretas del dashboard **[P]**.
- **Estados y feedback:** fallbacks integrados (failover transparente) [Os].
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** sign-up + "complete the onboarding flow" + generar API key [Os]. Onboarding explícito como flujo de producto [Os].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) **gateway + observabilidad acoplados** (fallbacks + logging en el mismo punto) — coincide con RFC 04 cascade + RFC 28 §H reset-window cards; (b) onboarding como flujo guiado (sign-up → key → primera request).
- **Evitar para Atlas:** [P].

---

## Braintrust — observabilidad de agentes — prioridad A — [Os]

- **URL (en vivo):** https://www.braintrust.dev/docs [Os]
- **Funcionalidades clave:** "the **active** observability platform for **instrumenting, understanding, and improving agents**. By actively applying intelligence to agent traces and automatically surfacing the most critical patterns, Braintrust gives teams the visibility to understand how agents behave in production" [Os].
- **Layout y navegación:** quickstarts "**Log your first trace**" y "**Run your first eval**"; "Follow Braintrust's structured **workflow** to build, evaluate, and improve AI applications" [Os].
- **Estados y feedback:** "**Evaluate** — Test changes with **experiments**"; evals "with **playgrounds and evals**" [Os].
- **Aprobaciones / HITL:** "**Annotate** — Add **human feedback** and build **datasets**" [Os].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** sección "**Quickstarts**" (log trace / run eval) [Os].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) framing **"active"** (de observar a *mejorar*, no solo logs) para el Health/Eval de RFC 24/65; (b) **Annotate → dataset** como el patrón de convertir feedback humano en datos de mejora; (c) "Loop thread" (un **agente** que investiga tus datos y construye scorers/dashboards) como idea de meta-agente de observabilidad.
- **Evitar para Atlas:** [P].

---

## MLflow — observabilidad / evals — prioridad A — [Os]

- **URL (en vivo):** https://mlflow.org/docs/latest/genai/ [Os]
- **Funcionalidades clave:** "the largest open source **AI engineering platform for agents and LLMs** … debug, evaluate, monitor, and optimize production-quality AI applications" [Os].
- **Layout y navegación:** **tracing** "captures your app's **entire execution, including prompts, retrievals and tool calls**"; SDK "OpenTelemetry-compatible … helps avoid vendor lock-in" [Os].
- **Estados y feedback:** "**Evaluation & Monitoring** — Stop manual testing with **LLM judges and custom metrics**. Systematically evaluate every change" [Os].
- **Aprobaciones / HITL:** [P] (usa LLM-as-a-judge, no revisión humana explícita en la landing).
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** "**Observability Quickstart** … sending your first trace"; "**Evaluations Quickstart** — preparing a dataset, configuring a scorer, and running your first evaluation" [Os].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) separar en la UX **"desarrollo de agente"** (trazas + eval) como bloque propio; (b) **OpenTelemetry-compatible** como criterio de no-lock-in (Atlas ya es local-first).
- **Evitar para Atlas:** [P].

---

## AgentOps — observabilidad de agentes — prioridad A — [Os]

- **URL (en vivo):** https://docs.agentops.ai/ [Os]
- **Funcionalidades clave:** "Observability and monitoring for your AI agents and LLM apps. And we do it all in just **two lines of code**" [Os].
- **Layout y navegación:** "the **AgentOps Dashboard** … visualize your agents' behavior … each execution of your program is **recorded as a session**" [Os].
- **Estados y feedback:** "automatically instrument your code and start **tracking traces**"; trazas custom con `@trace(name=…, tags=[…])` [Os].
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** `agentops.init(<API KEY>)` — "in just two lines of code" [Os].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) **fricción mínima de instrumentación** (2 líneas) como meta de DX; (b) **sesión como unidad** del dashboard (paralelo al `agent_run` de RFC 63).
- **Evitar para Atlas:** [P].

---

## Comet Opik — observabilidad + optimización — prioridad A — [Os]

- **URL (en vivo):** https://www.comet.com/docs/opik/ [Os]
- **Funcionalidades clave:** "Open-Source LLM **Observability & Optimization**"; "**Log traces** — Record every LLM call, tool invocation, and agent step. Debug failures, track token costs" [Os].
- **Layout y navegación:** sección "**Explore by feature**" (Quickstart, MCP Server, Log traces, Evaluate, Optimize, Manage prompts, Self-host); "the **project dashboard**" [Os].
- **Estados y feedback:** "**Track quality in production** — online evaluation rules that **automatically score** incoming traces, and monitor **feedback scores, latency, cost, and error rates** from the project dashboard" [Os].
- **Aprobaciones / HITL:** "Build test suites from your traces … add test cases **through the UI or SDK**" [Os].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** "**Quickstart** — Get Opik running with your existing AI stack in minutes" [Os].
- **Accesibilidad:** [P].
- **Bonus (MCP):** "**MCP Server** — Connect Claude Code, Cursor, VS Code Copilot, Codex, or **opencode** directly to your Opik workspace. Read traces, score outputs, and run experiments from chat — **no UI required**" [Os] — relevante para la DX de Atlas (docs/tooling vía MCP).
- **Adoptar para Atlas:** (a) **project dashboard** con feedback/latency/cost/error juntos (mapea el Cost+Health de RFC 24/65); (b) **MCP server de la propia herramienta** como patrón de DX.
- **Evitar para Atlas:** [P].

---

## W&B Weave — observabilidad + evals — prioridad A — [Os]

- **URL (en vivo):** https://weave-docs.wandb.ai/ [Os]
- **Funcionalidades clave:** "W&B Weave is an observability and evaluation platform that helps you **track, evaluate, and improve your agents and LLM applications**" [Os].
- **Layout y navegación:** "Send **OpenTelemetry spans** to the **Agents view**" (vista dedicada de agentes) [Os].
- **Estados y feedback:** "**Get started evaluating your app** — build an evaluation pipeline using **Weave scorers** to test and track your application's performance" [Os].
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** "**Quickstart: Instrument and trace functions** — tracing a basic call to an LLM and reviewing the data" [Os].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) **"Agents view"** como vista de primer nivel (no mezclar con logs genéricos); (b) **OTLP** como transporte neutro de trazas.
- **Evitar para Atlas:** [P].

---

## Lunary — observabilidad de chatbots — prioridad A — [Os parcial]

- **URL (en vivo):** https://docs.lunary.ai/ [Os]
- **Funcionalidades clave:** "**Lunary** is a platform for developers of AI chatbots and other LLM-powered applications" [Os].
- **Estados y feedback:** "**Observability** — Monitor and debug your LLM calls and agents"; "**Chats** — Track chatbot conversations and **user feedback**" [Os].
- **Layout/navegación · Aprobaciones · Atajos · Onboarding · Accesibilidad:** [P] (el index no detalla UI; <4 campos concretos esta sesión).
- **Adoptar para Atlas:** (a) **Chats con feedback de usuario** como vista (mapea el AgentConsole + feedback de RFC 24); **[P]** el resto.
- **Evitar para Atlas:** [P].

---

## TruLens — observabilidad / evals — prioridad A — [Os]

- **URL (en vivo):** https://github.com/truera/trulens (README) [Os] — la home `trulens.org` no era convertible (error del conversor); el README sí.
- **Funcionalidades clave:** "TruLens **finds where your agent fails and where you can cut cost without losing quality**. Open source, **OpenTelemetry-native**"; tracing OTEL de "every function call, LLM generation, retrieval, and tool invocation"; **"Seven purpose-built evaluators for agentic systems"** (LogicalConsistency, ExecutionEfficiency, PlanAdherence, PlanQuality, ToolSelection, ToolCalling, ToolQuality); **MCP support** (span type `MCP` captura tool name/arguments/output/latency) [Os].
- **Layout y navegación:** **"trace waterfall"** (traza por paso) + **"leaderboard"** para comparar versiones de la app ("Scores, latency and cost per app version") [Os].
- **Estados y feedback:** "**Latency, inputs, outputs, tokens and cost, recorded per step**"; "TruLens judges are graded against **human annotations**" (95% de errores de agente capturados con Agent GPA) [Os].
- **Aprobaciones / HITL:** evaluación anclada a **anotaciones humanas** (grading de judges contra humanos) [Os].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** "**Quick Usage** — Walk through how to instrument and evaluate a RAG built from scratch" + Colab quickstart [Os].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) **evaluadores específicos de agentes** (PlanAdherence, ToolSelection, ExecutionEfficiency) como taxonomía del Health/Eval de RFC 24/65 — más rica que "pass/fail"; (b) **trace waterfall + leaderboard** (comparar versiones) como vistas gemelas de Timeline/Health; (c) span **MCP** nativo (captura tool/args/output/latency) — encaja con el bridge MCP de RFC 07.
- **Evitar para Atlas:** [P].

---

## OpenRouter (Fusion / Sakana Fugu) — gateway LLM — prioridad A — [Os]

- **URL (en vivo):** https://openrouter.ai/docs/overview/models (+ https://openrouter.ai/) [Os]
- **Funcionalidades clave:** "**The Unified Interface For Every Model**. Better prices, better uptime, no subscriptions"; métricas de home "**600T+ Monthly Tokens · 10M+ Global Users · 80+ Providers · 500+ Models**" [Os]. **Fusion** (con Sakana Fugu) como *fusión de modelos* sigue **no expuesta como producto con UI verificable** [P] (probable feature experimental).
- **Layout y navegación:** "**Explore and browse 400+ models and providers**"; "**Discover Models**"; lista de modelos con `supported_parameters` (tools, reasoning, structured_outputs…) [Os].
- **Estados y feedback (routing):** "**Auto Router and model fallbacks**"; "**Higher Availability** — Reliable AI models via our distributed infrastructure. **Fall back to other providers when one goes down**" [Os].
- **Aprobaciones / HITL:** [P].
- **Atajos de teclado:** [P].
- **Onboarding / vacío / error:** "**Get your API key** — Create an API key and start making requests. **Fully OpenAI compatible**" [Os].
- **Accesibilidad:** [P].
- **Adoptar para Atlas:** (a) **gateway unificado multi-modelo** como capa de entrada del Model Orchestrator (RFC 04); (b) **model fallbacks** + "Auto Router" como modelo del cascade (RFC 04) y del cost/uptime del Health; (c) `supported_parameters` como metadatos por modelo (capability mask de RFC 04).
- **Evitar para Atlas:** [P].

---

## Cobertura [P] de esta categoría

Prioridad A sin ficha [Os] en vivo (ver `_pending-A.md`): **OpenTelemetry GenAI**, **Qdrant**, y el detalle del resto del bundle `Helicone / Portkey / LiteLLM / OpenRouter / ZenMux` (Helicone y OpenRouter ya fichados [Os]). **TruLens** cerrado [Os] (README GitHub) en la pasada 4.

## Enlaces a referentes ya auditados (no se repiten)

- **Langfuse** (trace tree + agent graph, `Aggregated↔Expanded`, MCP con HITL): `docs/design/CONSENSUS_AUDIT.md` §1.3 #5, §2.6, §2.9.
- **Grafana** (paneles/Explore/alerting): `CONSENSUS_AUDIT.md` §1.3 #17.
