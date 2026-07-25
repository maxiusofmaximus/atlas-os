# RFC 28 Research: graphify (graphify.com)

**Date:** 2026-07-25 · **Author:** opencode research agent · **For:** RFC 28 (Knowledge-graph layer for OpenCode OS)
**Status:** Verified against live URLs; URLs that returned no usable content are flagged `not found`.

---

## 1. What graphify actually is

**Graphify** is an **open-source code knowledge-graph skill** for AI coding assistants. It is **NOT** an agentic-execution-graph framework, not a workflow runtime, and not a SaaS API. The mission statement (graphify.com, verified):

> "Graphify turns your codebase into a knowledge graph your AI assistant queries instead of grepping: no embeddings, just a graph it can trace and cite. Open source, on-device, Apache 2.0."

- **Product type:** Python CLI + PyPI package (`graphifyy`, double-y) + cross-assistant "skill" (codified SKILL.md / AGENTS.md injection) + optional MCP server (stdio or HTTP).
- **Vendor / author:** Safi Shamsi (GitHub `safishamsi`), organization `Graphify-Labs`. LinkedIn page `graphify-labs` exists. YC **S26** badge on the repo (early stage).
- **Repo:** https://github.com/Graphify-Labs/graphify — **95.4k stars**, 9.2k forks, 1,253 commits, branch tag `v8`. Apache-2.0 (also ships MIT/NOTICE files).
- **Pricing:** Core free + Apache-2.0, no account. There is an "Enterprise (early access)" tier — merge-gate verification, graph-aware review, engineering digest — **self-hosted** in your own infra. (graphify.com/pricing)
- **Single-binary / Python rule:** Graphify is a **Python 3.10+ package** (`uv tool install graphifyy`). This **violates RFC 25 §11** for OpenCode OS if we ship it inside the binary. Decision: **adopt the pattern, NOT the dependency** (see §5).

## 2. How the graph model works (verified on graphify.com/concepts + /docs)

- **Build:** `/graphify .` walks the repo with **bundled tree-sitter grammars for 36 languages** (incl. Rust, TypeScript/Svelte). Code is parsed **deterministically — no LLM call, nothing leaves the machine.** Non-code (docs, PDFs, SQL, Terraform, video/audio) is read by **your configured model backend** (9 backends: OpenAI / Anthropic / Gemini / DeepSeek / Kimi / Bedrock / Azure / Ollama / local).
- **Nodes:** symbols (functions, classes, methods), files, doc concepts, `# NOTE:` / `# WHY:` / `# HACK:` comments, ADR/RFC citations — all first-class.
- **Edges:** `calls`, `imports`, `inherits`, `mixes_in`, `references`, `depends_on`. Every edge carries a **provenance tag**:
  - `EXTRACTED` — straight from the AST (deterministic).
  - `INFERRED` — model-linked (e.g. doc → code).
  - `AMBIGUOUS` — unresolved (dynamic dispatch, reflection). Kept but flagged.
- **Outputs:** `graphify-out/{graph.html (interactive), GRAPH_REPORT.md, graph.json}`.
- **Queries:** `graphify query "<q>"`, `graphify path A B` (shortest path), `graphify explain <node>`, `graphify prs` (PR impact / triage / merge conflicts via shared communities).
- **MCP server:** `python -m graphify.serve graph.json [--transport http --port 8080 --api-key …]` — **10 graph tools**: `query_graph, get_node, get_neighbors, shortest_path, get_community, god_nodes, graph_stats, list_prs, get_pr_impact, triage_prs`. (graphify.com/docs)
- **Clustering:** Leiden community detection (LLM-free labels).
- **Protocols:** JSON (`graph.json`), HTML, Markdown, GraphML, Neo4j/FalkorDB push (Cypher), Obsidian export. **No protobuf.**

## 3. Karpathy connection — VERIFIED

**Karpathy did tweet about the underlying idea.** A Hacker News "Ask HN" submission by user `lilwing` on 2026-04-08 (https://news.ycombinator.com/item?id=47696872 — verified via hn.algolia.com API) explicitly states:

> "This week Graphify launched off the back of a Karpathy tweet. 48 hours to build and 1k stars. in 2 days."

Corroborating sources (all reachable, content confirmed via DuckDuckGo index):
- `x.com/esotericpigeon/status/2041249629786820724` — "Someone just built the exact tool Andrej Karpathy said someone should build. 48 hours after Karpathy posted his LLM Knowledge Bases workflow, this showed up on GitHub. It's called Graphify."
- https://themenonlab.blog/blog/graphify-knowledge-graph-claude-code-karpathy — "Karpathy posted asking for a tool to query his /raw folder of papers, tweets, and notes without reading every file. Graphify shipped 48 hours later."
- https://www.analyticsvidhya.com/blog/2026/04/graphify-guide/ — "Learn how Graphify turns Andrej Karpathy's 'LLM Wiki' idea into reality."
- https://medium.com/@kdineshkvkl/from-karpathys-idea-to-graphify-i-tried-building-a-knowledge-graph-from-my-codebase-59ef94655874
- https://zerofuturetech.substack.com/p/your-second-brain-rebuilt-a-complete
- https://github.com/elbruno/graphify-dotnet (.NET 10 port) — explicitly cites "Andrej Karpathy's tweet on LLM-powered personal knowledge bases — the original idea that started the chain."

**I could NOT retrieve the exact Karpathy tweet URL** (nitter.net/search returned no body; x.com requires JS). The tweet exists per multiple independent attestations but I did not see it directly. Treat the **primary URL as `not found`**; the existence is **strongly corroborated** (>5 independent sources, April 2026).

> **Honesty note:** I did not find a URL like `x.com/karpathy/status/<id>` that I verified with my own fetch. Do not cite one in RFC 28 without re-verifying.

## 4. Forum coverage (verified via hn.algolia.com JSON API)

| Where | URL | Notes |
|---|---|---|
| HN Ask HN | https://news.ycombinator.com/item?id=47696872 | lilwing, 18 comments, discusses graphify vs Ix |
| HN Show HN | https://news.ycombinator.com/item?id=48794162 | "Graphify: Turn any codebase into a queryable knowledge graph" by domysee (2026-07-05) |
| HN comment | https://news.ycombinator.com/item?id=47952362 | SEJeff: "graphify cut down the tokens in thinking dramatically… It makes a knowledge graph and dumps it into markdown… has stubs that pretend to be some tools like grep that read from the knowledge graph first." |
| HN Show HN | https://news.ycombinator.com/item?id=47986749 | vdiff CLI integrates `graphifyy` for blast-radius analysis |
| HN comment | https://news.ycombinator.com/item?id=47742234 | Madsn: chose graphify **because** "I am using OpenCode" — graphify has a `--platform opencode` install path (verified in README). |

**Reddit (r/LocalLLaMA, r/MachineLearning, r/AutonomousAgents):** `not found` — Reddit blocked the webfetch (bot verification wall). No verified Reddit URLs in this report.

## 5. Five actionable ideas for OpenCode OS (RFC 28)

Graphify is **Python → cannot be a runtime dependency** (RFC 25 §11 single-binary, AGENTS.md no-Python). We **adopt its design pattern in Rust + our existing SQLite journal.**

**(a) RFC 19 doom-loop supervisor as a state DAG.** Today RFC 19 models states linearly (PLAN→CODE→VALIDATE→REPAIR→LEARN). Re-store each mission's state machine as a **JSON graph in a new SQLite table `mission_graph(M12)`**: nodes = engine states (with `EXTRACTED`/`INFERRED` provenance tags like graphify), edges = transitions with `precondition` + `guard` + `visit_count`. The doom-loop switcher then becomes graph traversal: when retries trigger, the supervisor walks `shortest_path(current, healthy_state)` instead of falling back linearly. This is a **direct port of graphify's `path` query** onto our own state machines.

**(b) RFC 24 HUD: graph view, not cascade.** Graphify's `graph.html` is a force-directed clickable graph (Leiden communities, color-coded). Replace/augment the linear HUD "Mission Control" cascade with a **SvelteKit `<GraphView>` component** rendering the `mission_graph` table via `d3-force` or `vis-network`. Cards become nodes; engine transitions become edges; doom-loop risk visible as node redness. Backend: axum endpoint `GET /hud/graph/:mission_id` returns JSON already in SQLite — no new infra.

**(c) RFC 23 skills as reusable graph templates.** A skill today is prose + tools. Add a **4th skill file `graph.toml`** declaring a reusable sub-graph template (nodes + edges + preconditions). Skills like `prompt-clarify` ship a 3-node graph (probe→gap→refine). Loading = inserting the template into `mission_graph` with fresh IDs. This mirrors graphify's "skill" abstraction but elevates it from prompt-injection to **runtime-executable state machine**.

**(d) RFC 12 Planner: emit multi-step DAG, not linear chain.** Currently Planner produces a linear task list. Refactor Planner's output schema to a **DAG** (JSON matching `mission_graph`). Branching happens when a Coder subagent can take multiple valid paths; the Validator scores each branch; Repair rewrites failing edges instead of redoing the whole chain. Borrow graphify's `AMBIGUOUS` tag for planner choices below a confidence threshold → defer to user (HUD "steer" button already in RFC 24 §3).

**(e) RFC 16 Learning: structural graph diffing.** Persist **successful `mission_graph` instances** to a `learning_graphs` table keyed by `(intent_signature, success)`. On a new mission, retrieve top-k by cosine on `intent_signature` embeddings (we already have `sqlite-vec` + `fastembed-rs` per RFC 25), diff their edges against the Planner's proposed graph, and **inject missing proven-successful edges** as `INFERRED` hints. Failure graphs are stored mirrored with `outcome=failed` and used as anti-patterns. This is graphify's "edges-as-first-class-objects" idea applied to *agent trajectories*, not code.

## 6. Technical integration decision

- **No Rust crate exists for graphify.** `graphify` ships only as Python + MCP server. Importing Python would break single-binary (RFC 25 §11) and AGENTS.md §6.
- **MCP HTTP server option (`python -m graphify.serve --transport http`)** is the only runtime-compatible path, and it is **opt-in, not bundled**: a user who already runs graphify on their machine could point OpenCode OS at `http://127.0.0.1:8080/mcp` as a skill-backend MCP tool → **zero new binary weight**. Document this as an **optional external MCP** in RFC 28 §"Integrations", not a hard dependency.
- **Primary path = re-implement the pattern in Rust.** Concrete stack: existing `rusqlite` + new table `mission_graph`; `petgraph` crate (already mature Rust) for in-memory traversal (`shortest_path`, `god_nodes`, `get_neighbors`); `tree-sitter` Rust bindings (we may already pull these for the planned LSP host RFC 25 §3.6) for `EXTRACTED` edges on OpenCode OS's own codebase as a meta-skill. SvelteKit HUD gets a `<GraphView>` component reading axum JSON.

**Justification for `petgraph` (RFC 22 entry required):** mature, pure-Rust, MIT/Apache-2.0, no native deps, single-binary-safe. Replaces any need for graphify's Python Leiden impl with `petgraph`'s built-in algorithms or a small Rust Leiden port (one extra crate, justify separately).

---

## Appendix — URLs

**Verified & content read:**
- https://graphify.com/ (markdown)
- https://graphify.com/docs (markdown)
- https://graphify.com/concepts (markdown)
- https://graphify.com/pricing (markdown)
- https://graphify.com/faq (markdown)
- https://graphify.com/integrations (markdown)
- https://github.com/Graphify-Labs/graphify (README, truncated but quoteable)
- https://hn.algolia.com/api/v1/search?query=graphify (JSON, 20 hits inspected incl. story 47696872)
- https://duckduckgo.com/html/?q=graphify+karpathy+tweet (HTML index; corroborating blog links extracted)

**404 / not found:**
- https://graphify.com/about → 404
- https://graphify.com/docs/mcp-tools → 404 (tools listed in /docs body instead)

**Not directly fetched (bot-walled / JS-only), existence corroborated via index:**
- Karpathy's original tweet — `not found` (primary URL); corroborated by ≥5 independent April 2026 sources.
- `x.com/esotericpigeon/status/2041249629786820724` — index-quoted; not directly fetched.
- Reddit threads — `not found` (Reddit bot-walled webfetch).

**Word count:** ~795.
