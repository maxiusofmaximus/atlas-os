# opencode-doc-from-code — script module (RFC 29 §3.D Docs)

This skill is invoked to document real APIs. It generates docs strictly from signatures present in the repo.

## Inputs

- `file_paths` (string[]) — source files to document.
- `format` (string, optional, default `markdown`)

## Outputs

Markdown API reference with signatures plus one minimal example per export.

## Behavior

1. Extract exports via Tree-sitter/AST where available, text scan otherwise.
2. Quote the real signature for every entry; flag anything unresolved as `unverified`.
3. Never invent endpoints, params, or return shapes (anti-hallucination filter, RFC 18 §11).
4. One runnable example per export, derived from existing tests when present.

---

> This skill description is the formal contract. Phase 1 implements it as a prompt skill inside `skills/opencode-doc-from-code/` (see RFC 06).
