# opencode-arxiv-lookup — script module (RFC 29 §3.D Research, via Firecrawl RFC 28 §E)

This skill is invoked when a claim needs paper evidence. It fetches the arXiv abstract plus cited claims.

## Inputs

- `arxiv_id` (string) — e.g. `2406.18665`.
- `question` (string, optional) — narrows which claims are extracted.

## Outputs

Abstract plus claim list with section-level citations.

## Behavior

1. Fetch via the Firecrawl facade when the `firecrawl` feature is on; plain fetch otherwise.
2. Quote the abstract verbatim; extract only claims present in the paper text.
3. Chain to `opencode-research-query` when the paper alone cannot settle the topic.
4. Cache the result per the feasibility TTL pattern (RFC 10 §11.6) to avoid re-fetching.

---

> This skill description is the formal contract. Phase 1 implements it as a prompt skill inside `skills/opencode-arxiv-lookup/` (see RFC 06).
