# 29 - Genspark AI Integration

> Audit comparativo de Genspark AI como orquestador de agentes comerciales de 2025-2026 — `genspark.ai` (MainFunc, $100M Series A Mar 2025) — frente a Atlas OS. Cubre **Genspark Claw** (AI employee persistente multi-canal), **Genspark AI Workspace 6.0** (super-app web), y **Super Agent / MoA** (multi-agent orchestration). Mapea las funcionalidades diferenciales, los motivos de incoporación, una interpretación de su UI, y 6 brechas concretas (A–F) que Atlas OS puede cerrar adoptando los patrónes de Genspark. Las brechas son el input de Phase 2+; este RFC es el spec para dirigir trabajo futuro.

---

## 1. Contexto y motivación

Atlas OS auditó 4 orquestadores CLI-first en RFC 27 (`tmux-orchestrator`, `orca`, `herdr`, `traycer`). El campo de "AI workspace" comercial se ha movido en paralelo: Genspark pasó de AI Search (Jun 2024) a Super App con $100M Series A (Mar 2025) y ahora a **AI Workspace 6.0** (Jul 2026). Es el competidor comercial más completo en la categoría "AI employee / agentic workspace" — worth un audit separado porque toca un segmento distinto a orca/herdr: no es CLI-first, es **cloud-first multi-canal**.

Antes de cerrar Phase 2 (LLM-driven), hacemos un audit comparativo para:

1. **Validar la pivot de Atlas OS de "editor con IA" a "plataforma para agentes"** — Genspark ya no se vende como editor.
2. **Identificar principios de "AI Employee" que Atlas OS no ha articulado** — multi-canal, cloud persistence, "knows you".
3. **Catalogar brechas** específicas donde Atlas OS está detrás del estado del arte comercial y priorizarlas para Phase 2+.

Las fuentes auditadas (Jul–Ago 2026):

| Producto | URL | Stars/Users | Lanzamiento | Filosofía |
|---|---|---|---|---|
| `Genspark AI Workspace 6.0` | `genspark.ai` | $100M Series A, 5M+ users (Apr 2025), 9 localized sites | Jul 2026 | "Your all-in-one AI workspace" — super-app |
| `Genspark Claw` | `genspark.ai/genspark-claw` | Desktop app + 6 canales, $9.99/mo intro | Q3 2026 | "Your First AI Employee" — cloud computer |
| `Super Agent / MoA` | blog `genspark-moa`, `genspark-multiagent-orchestration` | Mixture-of-Agents v2 (Feb 2025) + Multi-Agent Orchestration shipped (Aug 2025) | Dec 2024–Aug 2025 | "Less Control, More Tools" — agentic scale |
| `Genspark AI Developer` | blog `genspark-ai-developer` | L4 autonomous coding agent | Aug 2025 | "Lets Everyone Build" — no-code agentic dev |
| `GenOffice` | blog `genoffice-open-source-ai-office` | First open-source AI Office Suite | Aug 2026 | OSS office suite |

---

## 2. Las 6 funcionalidades fundacionales de Genspark

Comparando los 4 productos, hay 6 invariantes que se repiten. No son features, son propiedades estructurales que cualquier "AI workspace" maduro exhibe en 2026.

### §2.1 AI Employee — agente persistente "que vive"

**Citado en:** Genspark Claw ("Your own Cloud Computer", "knows you, remembers everything"), blog `genspark-super-agent` (Apr 2025).

El agente runtime vive en la nube, no en el device del usuario. Sobrevive a cierre de laptop. Mantiene memory cross-session — "learns your context, preferences, and your habits". El operador puede messagearlo desde cualquier canal y sigue tractando.

- Genspark Claw: cloud computer dedicado por usuario, accessible from WhatsApp/LINE/Slack/Teams/Telegram/Discord.
- Genspark Super Agent (Apr 2025): "personalized — your AI, now tailored for you".
- Microsoft collaboration (Nov 2025): "bring AI agents to billions of knowledge users worldwide" — Agent365 integration.

**Implicación:** Atlas OS RFC 25 §3.2 dice "servidor axum WS sobrevive webview crashes" — pero el runtime **muere cuando se cierra el desktop Tauri**. No hay cloud persistence. **Esta es una brecha estructural (§3.A).**

### §2.2 Multi-canal — el agente habla donde el usuario ya está

**Citado en:** Genspark Claw (6 canales), Genspark AI Secretary (Google Suite, Jun 2025), Genspark AI Phone Call (May 2025, "AI Now Calls Personal Phones"), Genspark AI Meeting Notes (Apple Watch, Aug 2025).

El agente no exige abrir su app — se acciona desde el canal donde el operador ya trabaja. 6+ canales soportados:

- WhatsApp, LINE, Slack, Microsoft Teams, Telegram, Discord (Claw).
- Google Suite (Secretary, Jun 2025).
- Apple Watch + iOS + Android (Meeting Notes, Aug 2025).
- Voz — Phone Call (May 2025).

**Implicación:** Atlas OS HUD (RFC 24 §16) menciona "mobile + remote access" pero NO hay gateway multi-canal. El operador sólo puede steer via HUD web (OIDC). **Brecha B (§3).**

### §2.3 "Knows you" — memory personalizada y user modeling

**Citado en:** Genspark Claw ("Knows you, remembers everything"), `genspark-super-agent-personalized` (Apr 2025), `genspark-hub` (Oct 2025, "Dedicated Space and Memory for Every Project").

Genspark no es stateless entre sesiones. Cada usuario tiene:

- **Personalized Super Agent**: tailored al usuario individual.
- **Genspark Hub** (Oct 2025): "dedicated space and memory for every project" — project-scoped memory.
- **Genspark Hub → Workspace 6.0 → Second Brain** (Jul 2026): evolución a "Second Brain" feature en la nav actual.

**Implicación:** Atlas OS RFC 09 (Vector Knowledge) + RFC 16 (Learning Engine) tienen tiered memory — pero es user-agnostic. No hay **user modeling** (Honcho-style, citado en Hermes RFC 22 §1). **Brecha C (§3).**

### §2.4 Super-app — un workspace, 80+ herramientas integradas

**Citado en:** `genspark.ai/tools` (80+ tools listed), blog `genoffice` (Aug 2026, "First Open-Source AI Office Suite").

Genspark consolidó 80+ tools individuales en un sidebar unificado, organizados por categoría:

| Categoría | # tools | Ejemplos |
|---|---|---|
| Writing & Content | 18 | AI Writer, AI Document Generator, AI Summarizer, AI Fact Checker |
| Design & Visuals | 18 | AI Image Generator, AI UI Generator, Figma to Code, AI Storyboard |
| AI Models | 6 | GPT Image 2, Nano Banana, Claude Sonnet 5, Grok 4.5, Seedream 5 |
| Audio & Video | 11 | AI Video Generator, AI Podcast Generator, AI Voice Cloning, Image to Video |
| Business & Productivity | 33 | AI Dashboard Generator, AI CRM Builder, AI Database Builder, AI Task Manager, AI App Builder, AI Website Builder, AI Flowchart Generator, Sankey Diagram Generator |
| Products | 4 | Claw (AI Employee), Speakly, GenClipboard, GenTerminal |

**Implicación:** Atlas OS Skills (RFC 06) es un marketplace con firma obligatoria — pero el catálogo es user-installed, no bundled. La filosofía de Genspark es **opposite**: 80+ herramientas out-of-the-box, el usuario no tiene que descubrirlas. **Brecha D (§3) — diferente estrategia de discovery.**

### §2.5 Mixture-of-Agents (MoA) — multi-model as default, not opt-in

**Citado en:** `genspark-mixture-of-agents` (Dec 2024, "World's First MoA System"), `genspark-moa-v2` (Feb 2026, "Top Performance with Gemini 2.0"), `genspark-moa-powered-search` (Feb 2025), blog `genspark-multiagent-orchestration` (Aug 2025).

Genspark introdujo MoA en Dec 2024 como default en sus productos consumer. MoA v2 (Feb 2026) fuels Search, Chat, Images, Translation. El blog `genspark-less-control-more-tools` (Jul 2025, CTO Kay) articula la tesis:

> "Less Control, More Tools: How Genspark Built a Super Agent That Scales"

Multi-agent orchestration shipped en Aug 2025: `Multi-Agent Orchestration: Shipped. On-Demand Ephemeral Agent Creation: Next`. **Ephemeral agent creation** (spawn transient agents on-demand, no persistent state) es el next frontier identificado por el CTO.

**Implicación:** Atlas OS RFC 04 §3 define `Aggregation` (Single/MajorityVote/MoA/Council/SelfRefine/Reflexion) — pero reserved for `ExecutionMode::HighStakes` (RFC 19). No es default. Genspark lo usa para todo. **Brecha E (§3) — estrategia de uso, no capability.**

### §2.6 AI Developer — L4 autonomous coding agent

**Citado en:** `genspark-ai-developer` (Aug 2025, "The L4 Autonomous Coding Agent That Lets Everyone Build").

Genspark lanzó un **L4 autonomous coding agent** — clasificación L4 del framework de autonomía de agentes (L1 = suggestion, L2 = assistance, L3 = supervised execution, L4 = autonomous). "Lets Everyone Build": marketing explícito como democratizador —non-technical users building apps. Casos de uso documentados en blog:

- "How This Educator Built 13 Apps, Websites, and Simulations Without Writing a Single Line of Code" (Jul 2026).
- "How a Marketing Consultant Used Genspark AI to Build Software He Couldn't Have Imagined Writing" (Jul 2026).
- "How One CIO Built 25+ Internal Tools Without Developers" (May 2026).

**Implicación:** Atlas OS AUTONOMOUS mode (RFC 21 §3) ya existe — pero el marketing es "developer tool", no "no-code democratizer". Genspark captura un mercado que Atlas OS no. **Brecha F (§3) — narrativa, no arquitectura.**

---

## 3. Las 6 brechas contra los fundamentos

Las 6 brechas concretas que el audit identifica. Cada una está entrada como `(prioridad, esfuerzo estimado, principal beneficiario)`. Las primeras 3 son estructurales; las últimas 3 son de superficie o narrativa.

### §3.A — Cloud persistence "AI Employee" runtime **(P0, L, Kernel)**

**Patrón:** §2.1 AI Employee.
**Status:** RFC 25 §3.2 — axum WS server sobrevive webview crashes, pero el proceso Rust muere cuando Tauri desktop se cierra. No hay modo `opencode serve` que viva en un VPS headless.

**Propuesta:** Añadir `opencode serve` (sub-comando CLI RFC 08) que arranque el Kernel Bus + HUD WS server como daemon, sin webview. El operador desde su desktop/mobile se conecta via SSH tunnel o Tailscale al servidor. Esto extiende el principio §2.3 de RFC 27 (remote attach) al extremo lógico: el runtime es cloud-first, no desktop-first. Reutiliza la técnica de Hermes ("VPS de 5$") sin su adopción obligatoria — desktop-only sigue siendo válido como `opencode desktop`.

**INSPIRACIÓN:** Hermes (RFC 22 §1 — "VPS idle-casi-gratis"), Genspark Claw ("Your own Cloud Computer"). Atlas OS ya cumplió §2.3 remote attach parcial (webview crash survives); este es el siguiente paso.

### §3.B — Gateway multi-canal (Steer from anywhere) **(P1, M, HUD)**

**Patrón:** §2.2 multi-canal.
**Status:** RFC 24 §16 menciona "web responsive + push notifications + mobile review queue" pero NO hay gateway a Telegram/Discord/Slack/Teams/WhatsApp. El operador debe abrir el HUD web.

**Propuesta:** Implementar `opencode channel <platform>` (sub-comando CLI) que registra un bot OAuth en el canal del operador y mapea steer/approval requests a messages. Patrones:

- `approval.request` event → DraftsBot envía a Slack con botones `[APR] [DENY] [STEER]`.
- `cost.threshold.crossed` → DM Telegram al owner con `/pause`.
- `mission.consolidated` → Notion-style card posted en channel.

Stack: `teloxide` (Telegram, MIT), `serenity` (Discord, MIT), `slack-morphism` (Slack, MIT) — todas pure Rust, single-binary safe (RFC 25 §11). Feature-gated `multi-channel` default off.

**Riesgo:** Cada canal tiene rate limits distintos. Mitigación: cola por canal con backpressure (memo del patrón §2.4 de RFC 27).

### §3.C — User modeling (Honcho-style) **(P1, M, Vector KB)**

**Patrón:** §2.3 "Knows you".
**Status:** RFC 09 tiered memory + RFC 16 learning persisten facts pero son **user-agnostic**. La `mission_consolidated` (RFC 23) no diferencia ¿qué sabe el usuario? del ¿qué sabe el sistema?.

**Propuesta:** Añadir tabla SQLite `user_profile` (M20+) con:

```sql
CREATE TABLE IF NOT EXISTS user_profile (
  user_id TEXT PRIMARY KEY,
  preferences JSON NOT NULL,        -- {favorite_models, coding_style, …}
  knowledge_state JSON NOT NULL,    -- {domains_known, gaps_identified}
  interaction_history_summary TEXT, -- summary of past interactions
  updated_at INTEGER NOT NULL
);
```

El Prompt Understanding Pipeline (RFC 23 §2 paso 2) consulta `user_profile` para resolver `user_knowledge_gap` (C6) — si el usuario "no tiene manera o no la conoce" pero el sistema tiene `gap_identified` registrado, ofrecer mentor mode + atajos automáticamente. Inspiration: Genspark "Second Brain" feature + Honcho (Hermes RFC 22 §1).

### §3.D — Bundled skills catalog (60+ tools out-of-the-box) **(P2, M, Skills)**

**Patrón:** §2.4 super-app.
**Status:** RFC 06 Skills es un marketplace user-installed. El operador debe descubrir + instalar cada skill. Genspark bundles 80+ tools por categoría.

**Propuesta:** Ship un **catalogCurated** de ~30 skills más comunes bundled en el binario (con feature flag `bundled-skills` default on, configurable off para builds server-only):

| Categoría bundled | Skills (ejemplo) |
|---|---|
| Code | `opencode-fix`, `opencode-restart`, `opencode-format`, `opencode-test`, `opencode-refactor-extract-method` |
| Docs | `opencode-spec-show`, `opencode-doc-from-code`, `opencode-changelog-from-commits` |
| Research | `opencode-research-query`, `opencode-arxiv-lookup` (vía Firecrawl §E) |
| UI | `opencode-figma-to-code` (Genspark tiene esto como tool), `opencode-ui-from-screenshot` |

Las bundled skills están firmadas con la misma SHA-256 + minisign del marketplace (RFC 18) pero bundled en `assets/bundled-skills/`.

**Riesgo:** Tamaño binario. Cada skill ~5-50KB. 30 skills suman ~1MB. Aceptable dentro del budget 30-45MB (RFC 20).

### §3.E — MoA as default (cambiar narrativa) **(P2, S, Orchestrator)**

**Patrón:** §2.5 MoA.
**Status:** RFC 04 §3 `AggregationPolicy::MoA` reservado para `ExecutionMode::HighStakes`. Default es `Single`. Genspark usa MoA para todo.

**Propuesta:** NO habilitar MoA global por defecto — cost risk real (G11 cost guard es crítico, ver RFC 22 §10). Pero **cambiar narrativa**: documentar en RFC 04 §3 que `MoA` es "the Genspark default" y `Single` es "our cost-aware default" — y ofrecer `AggregationMode::Auto` experimental que decide por task difficulty (reader de Phase 2.3). Mantener cost guard (G11) como invariant. Inspiration: CTO Kay blog "Less Control, More Tools".

### §3.F — Narrativa "non-technical builder" (L4 democrat) **(P2, S, Vision)**

**Patrón:** §2.6 AI Developer L4.
**Status:** RFC 00 Vision y RFC 17 UI están narrados como "developer tool". Genspark captura el segmento "non-technical builder" (educator, marketer, CIO sin devs — los 3 user stories del blog).

**Propuesta:** Añadir a RFC 17 §11 "Modo noob" una variante **"Modo builder"** — el `modo_uso = architect` (RFC 23 §8) se marketing como "L4 autonomous". La narrativa: Atlas OS = "L4 autonomous coding for technical AND non-technical operators". Blog evidence (los 3 user stories) justifica que esto es un segmento real, no aspiracional. No requiere código nuevo — re-empaquetado de narrativa en HUD y docs.

---

## 4. Interpretación de la UI de Genspark

### §4.1 Layout maestro observado (genspark.ai + Claw page)

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Top Bar:  [Genspark]  Try Free  English ▾                                   │
├────────────────────────────────────────────────────────────────────────────┤
│ Hero (chat-first):                                                          │
│   "From a question to a live dashboard in seconds"                          │
│   [Try it now]  [Maybe later]                                               │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  Workspace 6.0 sidebar (visible en app):                                    │
│                                                                            │
│    New [Home] [Skills] [Second Brain] [More ▾]                             │
│                                                                            │
│    Products:                                                               │
│     ▸ Genspark Claw (AI Employee)                                          │
│     ▸ Speakly (voice)                                                      │
│     ▸ GenClipboard                                                         │
│     ▸ GenTerminal                                                         │
│                                                                            │
│    Tools (80+):  Writing(18)  Design(18)  Models(6)  Audio(11)  Biz(33)   │
│                                                                            │
│    Global:  Korea · Japan · Brasil · France · Italia · Español            │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│ Footer: Download App Store · Google Play · Blog · Comparisons · Business    │
└────────────────────────────────────────────────────────────────────────────┘
```

### §4.2 Patrones UI identificados

| Patrón | Genspark | OpenCode HUD (RFC 24) |
|---|---|---|
| **Chat-first hero** | La home es un prompt box gigante + CTA "Try Free" | HUD es sidebar+views+jerarquía (`+ New Mission` está pequeño en sidebar) |
| **Sidebar por categoría** | Tools agrupados por categoría (6 grupos) | Sidebar recursively árbol Mission→Objective→Task |
| **Nav global (i18n)** | 9 locales Selector en top bar | Sin i18n explícito en RFC 17/24 |
| **Mobile apps nativas** | iOS + Android apps dedicadas | Responsive web only |
| **Comparisons landing** | `/alternative` página SEO vs Manus/ChatGPT/Replit/Lovable/Sora/Midjourney/Wix/Squarespace/Webflow/Figma | Sin comparativa pública RFC |
| **Chat como control universal** | "Don't Type, Just Speak, Work Gets Done" (Workspace 2.0 tagline, Jan 2026) | HUD tiene chat steer pero views son lo principal |
| **Second Brain (memoria)** | New en nav — feature destacada | Memoria tiered es implícita, no expuesta |
| **Super Agent mascot** | Claw tiene mascot visual (verde) | Sin mascot (profesional, no friendly) |

### §4.3 Lo que OpenCode puede tomar (sin copiar)

| Tomar |justificación | RFC touched |
|---|---|---|
| **Hero con prompt box prominente** | Primer item del HUD ahora es la sidebar. Un hero chat-first mejora onboarding. | 24 §1 layout maestro |
| **`Second Brain` como view concreta** | Exponer tiered memory como view persistente (no sólo infra). | 24 §2 views selector (+1 view) |
| **`Comparisons` landing pública**| Página SEO `/compare/{tool}` para adopción. | 20 Roadmap Fase 10 |
| **i18n explícito** | 9 locales es excesivo pero ≥3 (EN/ES/PT) en Phase 3. | 17 §6 + 24 §16 |
| **Mascot opcional** | NO tomar — OpenCode es professional tool. Mascot distrae del multi-agent metaphor. | (no tomar) |

### §4.4 Lo que OpenCode NO debe copiar

- **Cloud-only model**: Genspark Claw es `$9.99/mo` SaaS. OpenCode es single-binary open-source (RFC 25 §11). Cloud persistence (§3.A) es opt-in, no assumption.
- **80+ tools bundled**:§3.D propone 30, no 80. Genspark tiene un equipo full-time por categoría; OpenCode no.
- **Mascot visual**: profesional tool con HUD Mission Control — mascot rompe contrato profesional.
- **MoA global default**: cost risk real, mantener cost guard (G11).

---

## 5. Cross-references entre brechas y RFCs existentes

| Brecha | RFC principal | RFCs colaterales | Migración schema |
|---|---|---|---|
| A (cloud serve) | 25 Stack, 08 CLI | 02 Kernel, 24 HUD | (no schema, runtime) |
| B (multi-canal) | 24 HUD | 02 Kernel Bus, 18 Security | M20 `channel_configs` |
| C (user modeling) | 09 Vector KB | 16 Learning, 23 Prompt | M20 `user_profile` |
| D (bundled skills) | 06 Skills | 18 Security | (FS only, no schema) |
| E (MoA narrativa) | 04 Orchestrator | 21 Execution Modes, 22 Research | (no schema, doc) |
| F (modo builder) | 17 UI, 00 Vision | 23 Prompt, 21 Modes | (no schema, narrative) |

---

## 6. Priorización por dependencia

```
C (user modeling) ─┐
D (bundled skills) ─┼─► Phase 2.1 (LLM-driven básico, ~sprints)
F (modo builder)   ─┘

A (cloud serve)   ──► Phase 2.2 (multi-agent + remote, ~sprints)
B (multi-canal)   ──► Phase 3 (mobile + outbound gateway, ~sprints)

E (MoA narrativa) ──► doc-only, anytime
```

**Orden recomendado de implementación (por return/effort):**

1. **F (modo builder)**: Effort S, narrativa only. Quick win — captura segmento non-technical sin código nuevo.
2. **E (MoA narrativa)**: Effort S, doc only. Clarifica la posición competitiva vs Genspark.
3. **D (bundled skills)**: Effort M, mejora onboarding masivo. 30 skills out-of-the-box.
4. **C (user modeling)**: Effort M, mejora Prompt Understanding (C6) sustancialmente.
5. **A (cloud serve)**: Effort L, desbloquea todo el ecosistema "AI Employee" — abre mercado server.
6. **B (multi-canal)**: Effort M, deferred hasta post-cloud-serve (depende de §3.A paraSplunk -grade uptime).

---

## 7. Principios no cubiertos por las brechas — ya cumplidos o importados

| Principio | Implementado en | Cumple |
|---|---|---|
| §2.1 Cloud persistence (parcial) | `25 §3.2` axum survives webview crash | ✓ (parcial — falta cloud-first) |
| §2.3 Memory tiers | `09` Vector KB + `16` Learning | ✓ (falta user modeling, brecha C) |
| MoA capability | `04 §3 Aggregation::MoA` | ✓ (falta narrativa default, brecha E) |
| L4 autonomous mode | `21 §3 AUTONOMOUS` | ✓ (falta narrativa non-technical, brecha F) |
| Skills marketplace | `06` + firma `18` | ✓ (falta bundled default, brecha D) |
| Spec-first | `23` Prompt Under. | ✓ |
| Anti-infinite-loop | `19` DoomLoopDetector | ✓ |
| Hash-chained audit | `24 §10` | ✓ |
| Demos over diffs | `24 §9` | ✓ |

---

## 8. Status de este RFC

- **Versión:** 1.0 (audit completo, Ago 2026).
- **Tipo:** Informativo + priorización. No introduce APIs nuevas inmediatamente.
- **Cambio relacionado:** Phase 1.5 (commits `db25379`–`8d26528`) sienta la base técnica sobre la que las brechas A–F se implementarán.
- **Cierre de brechas:** Cada brecha se cierra con un PR etiquetado `feat(genspark-RFC29-§X): ...`. Quando las 6 estén cerradas, este RFC pasa a status: **implemented** y se mueve al apéndice histórico.
- **Fuente primaria:** `genspark.ai` (fetched Ago 2026), Genspark blog (60+ posts Jun 2024–Aug 2026), Genspark Claw landing page.

---

## 9. Fuentes de auditoría

Sitios y blog posts consultados vía webfetch (Ago 2026):

- Home: `https://www.genspark.ai/`
- Genspark Claw: `https://www.genspark.ai/genspark-claw` — "Your First AI Employee" + cloud computer + 6 canales.
- Comparisons: `https://www.genspark.ai/alternative` — 12 comparativas vs Manus/ChatGPT/Canva/Gamma/Replit/Lovable/Sora/Midjourney/Grok/Wix/Squarespace/Webflow/Figma.
- Blog (60+ posts):
  - `genspark-intro` (Jun 2024): "Welcome to Genspark, the AI Agentic Engine".
  - `genspark-mixture-of-agents` (Dec 18, 2024): "World's First MoA System".
  - `genspark-autopilot-agent` (Sep 18, 2024): "World's First Asynchronous AI Agent".
  - `genspark-super-agent` (Apr 2, 2025): "Meet Genspark Super Agent".
  - `genspark-kill-aisearch` (Apr 4, 2025): "Why I Killed Our AI Search Product With 5 Million Users".
  - `genspark-series-a-funding-and-ios-app` (Mar 3, 2025): $100M Series A.
  - `genspark-super-agent-personalized` (Apr 28, 2025): "Super Agent Becomes Personalized".
  - `genspark-less-control-more-tools` (Jul 29, 2025, CTO Kay): "How Genspark Built a Super Agent That Scales".
  - `genspark-multiagent-orchestration` (Aug 1, 2025, CTO Kay): "Multi-Agent Orchestration: Shipped. On-Demand Ephemeral Agent Creation: Next".
  - `genspark-ai-developer` (Aug 14, 2025): "L4 Autonomous Coding Agent That Lets Everyone Build".
  - `genspark-ai-meeting-notes` (Aug 12, 2025): Apple Watch first.
  - `genspark-microsoft-agent365` (Nov 18, 2025): Microsoft collaboration.
  - `genspark-ai-workspace-and-series-b-funding` (Nov 20, 2025): Workspace 1.0.
  - `genspark-hub` (Oct 29, 2025): "Dedicated Space and Memory for Every Project".
  - `genspark-ai-workspace-2` (Jan 28, 2026): "Don't Type, Just Speak, Work Gets Done".
  - `genspark-ai-workspace-3` (Mar 12, 2026): "Your First AI Employee".
  - `genspark-ai-workspace-4` (Apr 8, 2026): "Your AI Employee, Now Everywhere".
  - `genspark-ai-workspace-6` (Jul 20, 2026): Workspace 6.0 (current).
  - `genoffice` (Aug 3, 2026): "First Open-Source AI Office Suite".
  - User stories: `entrepreneur-builds-ai-chief-of-staff-no-tech-background`, `how-this-educator-built-apps`, `marketing-consultant-used-genspark`, `ficofi-cio-ai-internal-tools`.

Atlas OS RFC cross-references:

- §2.1 → 25 Stack §3.2, 08 CLI, 24 HUD §16
- §2.2 → 24 HUD §16, 02 Kernel Bus, 18 Security
- §2.3 → 09 Vector KB, 16 Learning, 23 Prompt
- §2.4 → 06 Skills, 18 Security
- §2.5 → 04 Orchestrator §3, 21 Execution Modes, 22 Research §10
- §2.6 → 17 UI, 00 Vision, 23 Prompt, 21 Modes

---

> **Apego a la directriz AGENTS.md §6** «No adding módulos nuevos de monolithic features. New engines go in their own RFC first». Este RFC introduce 6 frentes como direcciones, no como code. Cada frente se implementa con su propio sub-RFC o PR etiquetado según se priorice. **No se añaden crates nuevas en este RFC** — las crates citadas (`teloxide`, `serenity`, `slack-morphism` para brecha B) son propuestas para sus propios sub-RFCs posteriores con audit single-binary-safety (RFC 25 §11) completo.
