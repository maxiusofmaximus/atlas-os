# Grill Me — script module (plan 30 §A.3, Phase 3 sub-fase 3.5)

This skill is invoked before `Plan.lock`. It stress-tests the plan with systematic questioning across every decision-tree branch until shared understanding is reached (ported from `mattpocock/skills → grill-me`).

## Inputs

- `plan_id` (string) — the persisted `Plan.plan_id` in the Journal.
- `max_questions` (number, optional, default 10)

## Outputs

`GrillQuestion[]` rendered to the operator (see `planning::grill::GrillReport`).

## Behavior

1. Read `plan_id` from the Journal.
2. Run the mechanical pass (`planning::grill::grill_plan`): one question per unresolved decision — confidence below 0.7, live blockers, breaking impact, objectives without verifiable criteria, empty steps, high risk, research steps without an attached ResearchRun.
3. Present blocking questions first (`blocks_lock=true`); `Plan.lock` is refused while any remains.
4. Advisory questions (`blocks_lock=false`) guide the strong model to raise `Plan.confidence` without gating the lock.
5. Persist the answered questions with the plan so the audit trail shows why the lock was granted.

---

> This skill description is the formal contract. Phase 1 implements the mechanical pass in Rust (`planning::grill`); the strong model owns the follow-up prose.
