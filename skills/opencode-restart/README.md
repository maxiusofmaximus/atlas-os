# opencode-restart — script module (RFC 29 §3.D Code)

This skill is invoked when a run stalls or the Execution Supervisor raises `doom_loop`. It restarts from the last clean checkpoint.

## Inputs

- `mission_id` (string) — the Mission to restart.
- `checkpoint_id` (string, optional) — defaults to the last clean checkpoint in the Journal.

## Outputs

New run id bound to the same `mission_id`, resumed after `checkpoint_id`.

## Behavior

1. Read the checkpoint list for `mission_id` from the Journal.
2. Refuse the restart while an unacknowledged `doom_loop` hard-deny is open (RFC 19).
3. Resume after `checkpoint_id`; never replay already-applied diffs (idempotency_key preserved).
4. Emit `kernel.event(MissionResumed)` so the HUD re-renders.

---

> This skill description is the formal contract. Phase 1 implements it as a prompt skill inside `skills/opencode-restart/` (see RFC 06).
