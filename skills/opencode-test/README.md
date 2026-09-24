# opencode-test — script module (RFC 29 §3.D Code)

This skill is invoked after every code diff. It runs the fastest covering test slice.

## Inputs

- `diff_id` (string) — the persisted diff in the Journal.
- `timeout_seconds` (number, optional, default 120)

## Outputs

Pass/fail with the failing assertion quoted verbatim (see RFC 14 Validation Engine).

## Behavior

1. Map the diff paths to the nearest test files (`*.test.ts` / `#[cfg(test)]`).
2. Run only that slice (`vitest related` / `cargo test <filter>`), never the full suite unless nothing maps.
3. On failure, hand the report to `opencode-fix`; on pass, mark the validation stage green.
4. Persist the result so the audit trail shows what was executed.

---

> This skill description is the formal contract. Phase 1 implements it as a prompt skill inside `skills/opencode-test/` (see RFC 06).
