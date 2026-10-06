# RFC 66 — Consensus Audit

**Author:** UI/UX Design agent · **Date:** 2026-10-06
**Task (Project Lead):** verify whether RFC 66 truly reflects a field consensus.
**Scope:** design/docs only. `src/` untouched. No commits. FASE 10 not started.
**Artifacts:** this file + a new `RFC 66 §2.3` summary.

> **Provenance legend (rule 27):** **[O]** = observed in this repo's code/docs this session · **[Os]** = observed live from the product's own docs/site this session (URL cited) · **[I]** = inferred (repo research or prior knowledge, not inspected live) · **[P]** = pending verification.

---

## 1. Inventory — what actually exists, and how much I analysed

### 1.1 The "320+" claim is undocumented

`rg` over `Atlas OS/**`, `*.md`, `*.txt` for `320`, `320+`, `320 proyectos`, `320 projects` → **no match**. The figure appears **nowhere**. [O]

The only documented counts are in `research/62` itself [O]:
- Capa 1 + 1b: **~25**
- Capa 1c (A–O): **~150** (`research/62:273`)
- Capa 1d (P–X): **~125** (`research/62:343`)
- Capa 1e (favoritos): **~120** (`research/62:520`)
- **Total: ~425** (`research/62:520`, echoed in RFC 26, 25 §503, 64 §12) [O]

**Conclusion:** the operator's "320+" is **not** in any document. The nearest documented figure is **~425** — and it is a *project genealogy for architecture*, not a UX inventory (RFC 62's own title: "Genealogía de proyectos base y **arquitectura** de Atlas OS"). [O]

### 1.2 Real inventory: UI relevant to an *agent-orchestration HUD*

RFC 62's ~425 entries are dominated by things with **no coordination UI**: model providers, infra (memoria/sandbox/vector), papers, benchmarks, protocols, CAD/3D, creative-media, health, EU-politics. [O]

I parsed the catalog and classified by **primary artifact = a human-facing surface to coordinate/know agents**. Counts are the section's own documented/parsed size [O]:

| RFC 62 section | Count | UI-relevant? |
|---|---|---|
| Capa 1 — referentes (opencode, Claude Code, Codex, Cursor, Windsurf, Cline/Roo, Aider, Continue, Hermes, OpenClaw, AionUI, Genspark, Antigravity/Gemini, Copilot HydraFusion, OpenRouter Fusion, Kimi) | 16 | **Yes** (all are products with a UI/TUI) |
| 1b — añadidos agénticos (MiniMax Agent, Manus, Blackbox, Qoder, Abacus, Vida, MecAgent, Parakeet) | 9 | Yes (~8) |
| always-on cloud-computer (Dots, Grok Bot, Meta Muse/Gemini Spark) | 3 | Yes |
| A. Agentes de terminal open-source (OpenHands, SWE-agent, Goose, Plandex, gptme, Devon, …) | 14 | **Yes** (TUI) |
| B. Agentes de plataforma / CLIs (Claude Code, Codex, Gemini, Cursor, Windsurf, Devin, Amp, Junie, Warp, Trae, …) | 12 | Yes |
| D. Harnesses / runners / orquestadores (claude-flow, DeerFlow, Symphony, Omnigent, Conductor, **tmux/Zellij/Overmind**, **Vibe Kanban**, AgentSwarms) | 10 | **Yes** |
| G. Cloud dev environments (Coder, Gitpod, Codespaces, DevPod, code-server, …) | 7 | Yes (~6) |
| H. Observabilidad / evals / gateways (Langfuse, LangSmith, Phoenix, Braintrust, MLflow, Helicone/Portkey, TruLens/Opik, AgentOps) | 9 | **Yes** (dashboards) |
| I. Code review / calidad (Qodo, CodeRabbit, Greptile, Graphite, Sourcegraph, …) | 8 | Yes (~7) |
| C. Ecosistema de coding chino (Qwen, Qoder, CodeGeeX, CodeBuddy, Comate, Kimi Code, DeepSeek, GLM) | 9 | partial (~7) — **overlaps B/Capa 1** |
| 1d R. Workspaces / app-builders con agente (v0, Lovable, Replit, Base44, Rocket.new…) | ⊂ ~125 | Yes (~8), not individually enumerated |
| 1e favoritos (Warp, Kiro, Pieces, Pencil, v0, Replit, Lovable, n8n-workflows…) | ⊂ ~120 | marginal (~8 of 139) |

**UI-relevant to an agent-orchestration HUD (my classification, non-overlapping core): ≈ 88–100 entries.** No repo document enumerates this set — RFC 66 §2 simply *asserted* a 9-referent matrix. [O]

### 1.3 What I actually analysed

| # | Project | Category | Status | Source |
|---|---|---|---|---|
| 1 | **Herdr** | terminal workspace mgr | **[Os]** | herdr.dev, herdr.dev/agent-guide.md |
| 2 | **Orca** | ADE / orchestrator | **[Os]** | onorca.dev, github.com/stablyai/orca |
| 3 | **Genspark** | AI workspace | **[Os]** | genspark.ai (blog 6.0, help) |
| 4 | **Zed** | IDE | **[Os]** | zed.dev/docs/ai/agent-panel, /parallel-agents, discuss #58314 |
| 5 | **Langfuse** | LLM observability | **[Os]** | langfuse.com/docs/tracing, /agent-graphs, /custom-dashboards |
| 6 | **Vibe Kanban** | ADE + kanban | **[Os]** | vibekanban.com, github BloopAI/vibe-kanban |
| 7 | **n8n** | node canvas | **[Os]** | github n8n-io/n8n (WorkflowCanvas.vue, PR #33235, #15391) |
| 8 | **VS Code** | IDE | **[Os]** | code.visualstudio.com/docs/editing/userinterface |
| 9 | **JetBrains / IntelliJ** | IDE | **[Os]** | jetbrains.com/help/idea/new-ui.html, /tool-window-layouts |
| 10 | **Linear** | kanban / PM | **[Os]** | linear.app/docs/board-layout, /display-options, /now/how-we-redesigned |
| 11 | Hermes | orchestrator HUD | **[I]** repo | RFC 22 §1/§8.3 (cites hermes docs; not opened live) |
| 12 | Cursor | ADE | **[I]** repo | RFC 22 §3.1 (cites cursor.com/cloud) |
| 13 | Notion | docs/PM | **[I]** repo | RFC 22 §8.3 |
| 14 | Conductor | orchestrator | **[O]** repo | `research/28 §A` (repo research, sources internal) |
| 15 | **tmux** | terminal multiplexer | **[Os]** | github.com/tmux/tmux (tmux.1); tmux.app/doc/man |
| 16 | **Zellij** | terminal multiplexer | **[Os]** | zellij.dev/features · /documentation/session-manager-alias · /documentation/layout-examples |
| 17 | **Grafana** | observability dashboard | **[Os]** | grafana.com/docs/grafana/latest/visualizations/panels-visualizations/panel-overview · /visualizations/explore · /alerting |
| 18 | **Notion** | docs / PM | **[Os]** | notion.com/help/intro-to-databases |

**Analysed: 17 of ~88–100 UI-relevant candidates (~18%).** **14 verified live [Os]** this session + **3 repo-sourced [I]/[O]** (Hermes, Cursor, Conductor). **Category coverage — COMPLETE [Os]:** IDE ✔ (Zed/VS Code/JetBrains), ADE/orchestrator ✔ (Orca/Herdr/Vibe/Genspark), **terminal multiplexer ✔ (Herdr + tmux + Zellij)**, **observability dashboard ✔ (Langfuse + Grafana)**, **kanban/PM ✔ (Vibe/Linear + Notion)**, node canvas ✔ (n8n). **Remaining [I]:** Cursor/Hermes (repo-sourced); Datadog/Jira not inspected live.

---

## 2. Patterns by problem, across EVERY UX category (with sources)

Format per problem: what the live evidence shows → what Atlas should take. **[Os]** = live this session.

### 2.1 "¿Qué está pasando?" (situational awareness)

- **Herdr [Os]:** sidebar shows each agent's `working / blocked / done / idle / unknown`, and **rolls state up** pane → tab → workspace ("A blocked agent makes its pane, tab, and workspace look blocked"). "You never hunt for the stuck one."
- **Orca [Os]:** sidebar worktree card's **status dot** overlays hook-reported `blocked/waiting/done` on a heuristic base; priority `permission > done > heuristic`; retained "done" glows after the process exits (PR #1147).
- **Zed [Os]:** Threads Sidebar groups threads by project, each row shows **status indicator + which agent** is running it.
- **Langfuse [Os]:** trace shows as a **trace tree + agent graph**.
- **RFC 66 →** Activity Spine + per-card state. **CONFIRMED**, but the *rollup* (agent → mission) is under-specified in RFC 66; Herdr/Orca both roll up. **Adopt: state rollup to the Mission rail.**

### 2.2 Navegación raíz (what is the root axis?)

- **VS Code [Os]:** Activity Bar → Side Bar; recent versions add a **Secondary Side Bar (Chat view by default)** — still **file-first**.
- **JetBrains [Os]:** tool windows docked to edges; New UI "reduced visual complexity".
- **Vibe Kanban [Os]:** root = **kanban board of issues**; issues have parent/child; selecting an issue opens a details panel.
- **Orca [Os]:** unit = **worktree per agent**; "Quick open — search across worktrees, files, agents, commands".
- **Linear [Os]:** global chrome is an "inverted L-shape" (sidebar + header); views switch list/board/timeline/split.
- **RFC 66 →** Mission rail as root. **CONFIRMED** — every ADE/orchestrator puts a **container above the agent** (worktree/issue/workspace/Run). Naming differs; the concept is universal. VS Code's file-first axis remains the correct anti-pattern. **Nuance: VS Code is no longer purely "one side chat" — it has a secondary chat sidebar.**

### 2.3 Estados de agente

- **Herdr [Os]:** 5 states — `working, blocked, done, idle, unknown`.
- **Orca [Os]:** `blocked, waiting, done, permission` + `working` heuristic.
- **n8n [Os]:** node execution states; errored group auto-expands + selects the errored node.
- **RFC 66 →** maps 1:1 to Atlas's real `AgentStatus` (10 states) [O]. **CONFIRMED** that agent state is first-class everywhere. **Nuance:** only **Herdr [Os]** documents an explicit `unknown` state (the other references' docs read this session do not cite one); Atlas's enum lacks it (closest is `idle`). Consider adding `Unknown` for panes/runs the detector can't classify.

### 2.4 Aprobaciones / HITL

- **Orca [Os]:** explicit **"decision gates"** — a coordinator-owned question that **blocks a task until resolved**; `ask` for blocking questions; approval is a first-class message type (`escalation`, `question`).
- **Vibe Kanban [Os]:** **approval workflows** in the conversation panel for reviewing agent plans.
- **Langfuse [Os]:** MCP-driven dashboard edits run with **human-in-the-loop approval** for changes.
- **RFC 66 →** Approvals Dock + HITL taxonomy (informativo/confirmación/decisión/bloqueo/error/resultado), never mixed. **CONFIRMED** — Orca's "decision gate blocks the task" is exactly RFC 66's "bloqueo" class. **Nuance:** Orca distinguishes **task-blocking gates** from **worker questions** (`ask`) — RFC 66 collapses these into one dock; a two-channel split (gate vs question) may be warranted.

### 2.5 Contexto

- **Orca [Os]:** **Design Mode** — click a UI element in the embedded browser → its HTML+CSS+cropped screenshot go into the agent prompt.
- **Langfuse [Os]:** data model `observations → traces → sessions`; sessions group multi-agent work.
- **Vibe Kanban [Os]:** workspace = repo(s)+branch+**dev server**; Context Panel shows changes/logs/preview.
- **RFC 66 →** Context Rail. **MATIZED** — RFC 66 never specified the **worktree/branch/dev-server** context nor a **multi-agent session** grouping (Langfuse's "session spans several agents that feed one report"). Adopt both.

### 2.6 Feedback (demos over diffs, review)

- **Vibe Kanban [Os]:** **inline diff comments sent back to the agent**; built-in browser with devtools; "the new bottleneck is planning and review".
- **Zed [Os]:** while working, a turn is expanded live; **on completion it collapses into "Thinking / Commands / Edits" + final answer**, with a `Worked for {duration}` summary; a filter to answer "what files did it touch / what commands did it run" at a glance.
- **Langfuse [Os]:** trace tree + **Aggregated vs Expanded** ("as it ran") graph — one node per step-name vs one node per call; loops drawn as cycles vs unrolled.
- **n8n [Os]:** execution log per node; a toggle **syncs canvas selection ↔ log selection**.
- **RFC 66 →** Demo-over-diff + 3-layer card disclosure. **CONFIRMED** (Zed's collapse validates the layering). **Adopt:** (a) a turn **collapse-on-completion** with a "worked for Nm" header; (b) **Aggregated↔Expanded** toggle for the Canvas/Outline; (c) **canvas↔log selection sync**.

### 2.7 Historia

- **Herdr [Os]:** timelines survive restart; server-owned history.
- **Langfuse [Os]:** session replay; traces ordered in sequence.
- **n8n [Os]:** historical executions preserve grouping via snapshot.
- **RFC 66 →** Audit chain + Timeline. **CONFIRMED** (RFC 24 §10/§15 already specify it).

### 2.8 Descubrimiento

- **VS Code [Os]:** Command Palette with modes (`>` commands, `#` symbols, file quick-open) — the reference implementation.
- **Orca [Os]:** Quick open across worktrees/files/agents/commands.
- **Herdr [Os]:** mouse-first **plus** prefix mode (`ctrl+b` + key) **plus** a persistent navigate mode; CLI and socket API are the *same* surface agents drive.
- **RFC 66 →** `:`-prefixed command palette. **CONFIRMED** on the need for a palette. **Nuance:** `:` is a **TUI convention** (lazygit/k9s), not an IDE one (VS Code uses `Ctrl+Shift+P`); fine for Atlas's audience but worth stating as a choice, and Herdr's insight — **the agent-facing API and the human UI are the same surface** — is a differentiator Atlas should consider.

### 2.9 Node canvas (category RFC 66 named but didn't evidence)

- **n8n [Os]:** canvas ↔ **logs panel docked below**, bidirectional selection sync, group collapse, error auto-expand. **Langfuse [Os]:** agent graph with Aggregated/Expanded.
- **RFC 66 §2/§13 →** Canvas view. **CONFIRMED** the pattern; **MATIZED** — RFC 66's Activity Spine is a *separate right column with no canvas link*. n8n's docked, selection-synced log is the stronger pattern.

---

## 3. Decision-by-decision contrast

| Decision [O] | Evidence | Verdict |
|---|---|---|
| **D-66-01** Mission = unidad fundamental | Vibe (issue→workspace/agent), Orca (Run+Tasks+workers), Herdr (workspace container + rollup), Langfuse (session spans agents) **[Os]** | **CONFIRMADA** — universal container-above-agent. Naming ("Mission") is Atlas's; concept confirmed. |
| **D-66-02** Layout Mission Control (no IDE) | Vibe **4-panel** (sidebar+conversation+context+details) **[Os]**; Zed **Agentic layout** flips agent panel left, project right **[Os]**; Orca worktree-per-agent **[Os]**; n8n canvas+**bottom docked logs** **[Os]** | **CONFIRMADA (family) / MATIZADA (specifics)** — (a) references keep a **conversation surface** RFC 66 dropped; (b) n8n's exec log is **docked to the canvas and selection-synced**, not a separate column. Keep the spine, but consider canvas-synced log and a conversation surface. |
| **D-66-03** "Calm Instrumentation" + tokens | JetBrains New UI: "reduce visual complexity … progressively disclose" + **Compact mode** **[Os]**; Linear redesign: "reduce visual noise, increase hierarchy and density" **[Os]**; OKLCH/WCAG tooling consensus **[Os]** | **CONFIRMADA** (direction). **Nuance:** the **teal accent is NOT a field consensus** — no reference uses teal as its signal; it's a *differentiator* chosen to avoid the blue default. That's legitimate, but it is a **choice, not a consensus**. |
| **D-66-04** Estados = enum real | Herdr (5 states + rollup) **[Os]**; Orca (`permission>done>heuristic`) **[Os]**; Zed thread status **[Os]**; n8n node states **[Os]** | **CONFIRMADA**. **Nuance:** only Herdr [Os] documents an `unknown` state (1 of 14, not all); Atlas's enum lacks it (see §2.3). |
| **D-66-05** Reconcile hotkeys | VS Code palette modes **[Os]**; Herdr prefix/navigate modes + shared CLI/socket surface **[Os]** | **CONFIRMADA** (internal fix). **Nuance:** `:` is TUI-style, not IDE-style — state it as a deliberate choice. |
| **Palette** teal-on-graphite | Consensus = *method* (OKLCH, semantic tokens, WCAG/APCA, CVD, dark≠inverted) **[Os]**; no reference validates teal-on-graphite as *the* consensus | **CONFIRMADA** (method + focus on AA). The lead's two AA fixes (`faint→#868686`, light `warn→#966000`, PALETTE §4.1) are consistent with that method — **reviewed, not reverted** [O]. |

### 3.1 Proposed changes (consensus pushes back on RFC 66)

1. **Add a turn "collapse-on-completion" + `Worked for {duration}` header** to the Agent Card (Zed) — RFC 66's 3-layer model lacks the automatic *completion* collapse. [Os]
2. **Canvas↔Activity selection sync**, and consider docking the activity log to the canvas (n8n) — RFC 66's spine is unlinked. [Os]
3. **Aggregated↔Expanded toggle** for Canvas/Outline (Langfuse) — same data, one-node-per-name vs one-node-per-call. [Os]
4. **State rollup** agent → Mission rail (Herdr/Orca). [Os]
5. **Add `Unknown` to the state vocabulary** (evidence: Herdr only, 1 of 14 verified; treat as a weak, single-source proposal). [Os]
6. **Re-introduce a conversation surface** (Vibe keeps one; Zed's panel *is* one) or explicitly justify its absence. [Os]
7. **Split the approvals dock into gate vs question** (Orca `decision gate` vs `ask`). [Os]
8. **Spec the worktree/branch/dev-server context** and multi-agent session grouping (Orca, Vibe, Langfuse). [Os]

None of these **refutes** the core decisions; they **nuance (matize)** them.

---

## 4. Verdict

**Sí — condicional (Yes, conditional).** *[Updated 2026-10-06, round 2.]* The three conditions that kept this at *Parcial* are now met:
1. **§3.1 items 1–5 landed** at spec + mockup level — collapse-on-completion + `Worked for Nm` (RFC 66 §6.2), canvas↔log sync (§10), Aggregated↔Expanded (§10), state rollup (§4/§10), `unknown` state (§6.1 / PALETTE §5). Items 6–8 are registered as **explicit open operator decisions** (RFC 66 §16, OA-66-06/07/08), not omissions.
2. **Live coverage closed** for the previously-unverified families: tmux, Zellij, Grafana, Notion now **[Os]** (§1.3) → 14 live + 3 repo = **17 analysed**; every UX category is live-verified.
3. **§2 reframed** from "consenso del campo" to **evidence-based (n=14 live + 3 repo)**, linking this audit; the teal is marked a **differentiation choice, not consensus** (D-66-03).

**Residual conditions (why "conditional", not an unqualified "Sí"):**
- **Items 6–8 pending the operator** (conversation surface, gate-vs-question, execution context/session) — registered as open decisions.
- **`unknown` is proposed, not implemented**: it is **not** in the Rust `AgentStatus` enum [O]; needs the enum change in FASE 10.
- **Cursor/Hermes remain [I]** (repo-sourced); Datadog/Jira were not inspected live.
- **FASE 9 a11y still open**: `:focus-visible` is now demonstrated in `mockup.html` (rule added) and `unknown`'s contrast is **measured** (5.33/4.97/4.52:1 vs bg/surface/surface-2), but CVD simulation and APCA at real sizes remain [P].

**Confirmed:** all structural decisions (D-66-01..05) and the color *method* hold against the (now broader) evidence.

---

## 5. Appendix

### 5.1 URLs consulted (live, this session)
- https://herdr.dev/ · https://herdr.dev/agent-guide.md · https://herdr.dev/docs/agents/ · https://herdr.dev/docs/agent-automation/ · https://herdr.org/
- https://www.onorca.dev/ · https://www.onorca.dev/docs/cli/orchestration · https://github.com/stablyai/orca · https://github.com/stablyai/orca/pull/1147
- https://www.genspark.ai/blog/genspark-ai-workspace-6 · https://www.genspark.ai/helpcenter/genspark-design · https://screensdesign.com/showcase/genspark-super-ai-agent
- https://zed.dev/docs/ai/agent-panel · https://zed.dev/docs/ai/parallel-agents · https://github.com/zed-industries/zed/discussions/58314
- https://langfuse.com/docs/tracing · https://langfuse.com/docs/observability/best-practices · https://langfuse.com/docs/observability/features/agent-graphs · https://langfuse.com/docs/metrics/features/custom-dashboards
- https://vibekanban.com/ · https://github.com/BloopAI/vibe-kanban/blob/main/docs/workspaces/interface.mdx · .../docs/getting-started.mdx
- https://github.com/n8n-io/n8n (WorkflowCanvas.vue; PR #33235; commit #15391)
- https://code.visualstudio.com/docs/editing/userinterface
- https://www.jetbrains.com/help/idea/new-ui.html · https://www.jetbrains.com/help/idea/manipulating-the-tool-windows.html · https://www.jetbrains.com/help/idea/tool-window-layouts.html
- https://linear.app/docs/board-layout · https://linear.app/docs/display-options · https://linear.app/now/how-we-redesigned-the-linear-ui
- **Round 2 — multiplexers:** https://github.com/tmux/tmux/blob/master/tmux.1 · https://tmux.app/doc/man/ · https://zellij.dev/features/ · https://zellij.dev/documentation/session-manager-alias.html · https://zellij.dev/documentation/layout-examples
- **Round 2 — observability/PM:** https://grafana.com/docs/grafana/latest/visualizations/panels-visualizations/panel-overview/ · https://grafana.com/docs/grafana/latest/visualizations/explore/get-started-with-explore/ · https://grafana.com/docs/grafana/latest/alerting/ · https://www.notion.com/help/intro-to-databases
- Color/tooling: accessibility.build, chromui.app, hexpalette.com, evvytools.com (via websearch), impeccable v4.1.1, design-taste-frontend

### 5.2 Commands run
- `rg` for `320|425|proyectos catalogados` across `Atlas OS/**`, `*.md`, `*.txt`
- `Select-String` headings of `research/62`, `research/28`, `research/61`
- Node parse of `research/62` (353 table rows; per-section counts)
- `node impeccable/scripts/context.mjs`, `palette.mjs`, `detect.mjs`
- Contrast recompute (OKLCH→sRGB + WCAG) for `#868686`: **5.33 / 4.97 / 4.52:1** vs bg/surface/surface-2
- Headless Edge render of `mockup.html` (dark + light — round 1 and round 2)

### 5.3 Files modified / created
- **Created** `docs/design/CONSENSUS_AUDIT.md` (this file)
- **Edited (round 1):** `Atlas OS/66 - UX Architecture & Design System.md` (§2.3 summary)
- **Edited (round 2):** `Atlas OS/66 - UX Architecture & Design System.md` (§2 reframe + provenance tags; §6.1 `unknown`; §6.2 collapse-on-completion; §10 four patterns; D-66-03 teal note; §16 items 1–5 landed + OA-66-06/07/08); `docs/design/PALETTE.md` (§5 `unknown` row + note; teal-not-consensus note); `docs/design/mockup.html` (rollup rail, turn-collapse, `unknown` card, canvas-sync badge, Agg↔Exp toggle, `:focus-visible` rule + focus demo, legend fixes); `docs/design/wireframe.html` (layers 0–3, rollup, Agg↔Exp, canvas-sync note)
- **Reviewed, not reverted:** `docs/design/PALETTE.md` §4.1 lead edits (`faint #868686`, light `warn #966000`)
- **Not touched:** `src/**`
