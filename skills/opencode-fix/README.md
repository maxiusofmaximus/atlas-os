# opencode-fix — script module (RFC 29 §3.D Code)

This skill is invoked on `validation.failed`. It diagnoses the failing check and proposes the minimal single-hunk fix.

## Inputs

- `validation_report_id` (string) — the persisted validation report in the Journal.
- `max_hunks` (number, optional, default 1)

## Outputs

Single-hunk diff proposal with the failing assertion quoted (see RFC 13 Coding Engine).

## Behavior

1. Read `validation_report_id` from the Journal.
2. Reproduce the failing assertion locally (never assume the error text is accurate).
3. Propose the smallest diff that turns the check green; no refactors in the same hunk.
4. If the fix needs more than `max_hunks`, escalate to `opencode-refactor-extract-method` instead.
5. Persist the proposal so the audit trail shows why the fix was accepted.

---

> This skill description is the formal contract. Phase 1 implements it as a prompt skill inside `skills/opencode-fix/` (see RFC 06).
