# opencode-refactor-extract-method — script module (RFC 29 §3.D Code)

This skill is invoked when a block needs a name. It extracts the selection into a function preserving behavior.

## Inputs

- `file_path` (string) — file containing the block.
- `start_line` (number) — first line of the block (1-indexed).
- `end_line` (number) — last line of the block (inclusive).
- `function_name` (string) — name for the extracted function.

## Outputs

Two-hunk diff: new function definition plus call-site replacement, with callers updated.

## Behavior

1. Infer inputs/outputs of the block from data flow; refuse when side effects cannot be preserved.
2. Insert the new function adjacent to the call site following repo conventions.
3. Update all in-file callers that duplicate the block.
4. Hand the diff to `opencode-test` before reporting done.

---

> This skill description is the formal contract. Phase 1 implements it as a prompt skill inside `skills/opencode-refactor-extract-method/` (see RFC 06).
