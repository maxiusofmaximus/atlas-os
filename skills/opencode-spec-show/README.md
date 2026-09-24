# opencode-spec-show — script module (RFC 29 §3.D Docs)

This skill is invoked when the operator asks what the mission agreed. It renders the locked spec readably.

## Inputs

- `mission_id` (string) — the Mission whose spec is shown.

## Outputs

Summary of `MissionConsolidated` plus Plan milestones with confidence (see RFC 23 §4, RFC 12 §3).

## Behavior

1. Read `MissionConsolidated` and the locked Plan for `mission_id` from the Journal.
2. Render goals, non-goals, milestones, and open clarification questions in order.
3. Never invent scope beyond the persisted rows; quote the Journal.
4. Link each milestone to its verification criterion.

---

> This skill description is the formal contract. Phase 1 implements it as a prompt skill inside `skills/opencode-spec-show/` (see RFC 06).
