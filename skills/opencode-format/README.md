# opencode-format — script module (RFC 29 §3.D Code)

This skill is invoked before the validation stage. It formats only the touched files.

## Inputs

- `file_paths` (string[]) — files touched by the current diff.
- `formatter` (string, optional, default `auto`) — `cargo-fmt`, `prettier`, or `auto`.

## Outputs

Formatter applied to `file_paths`; no semantic changes.

## Behavior

1. Resolve `formatter=auto` to `cargo fmt` for Rust paths and `prettier` for TS/Svelte paths.
2. Format only `file_paths`; never format the whole repo.
3. If the formatter is missing, report `ExternalToolMissing` instead of failing silently.
4. Leave the diff ready for `opencode-test`.

---

> This skill description is the formal contract. Phase 1 implements it as a prompt skill inside `skills/opencode-format/` (see RFC 06).
