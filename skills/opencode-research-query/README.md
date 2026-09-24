# opencode-research-query — script module (RFC 29 §3.D Research)

This skill is invoked before critical decisions. It runs a collective web query and returns scored consensus.

## Inputs

- `topic` (string) — the question to research.
- `min_sources` (number, optional, default 3)

## Outputs

Consensus summary with per-dimension scores plus source list (see RFC 10 §5/§7).

## Behavior

1. Fan out to docs gateway, community, academic, and official sources (RFC 10 §5).
2. Apply the fail-safe: fewer than `min_sources` yields `needing_human`, never a confident answer.
3. Score consensus per dimension and keep the top reference per dimension.
4. Persist the run so `atlas research query` can re-render the YAML report.

---

> This skill description is the formal contract. Phase 1 implements it as a prompt skill inside `skills/opencode-research-query/` (see RFC 06).
