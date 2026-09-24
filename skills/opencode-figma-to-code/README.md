# opencode-figma-to-code — script module (RFC 29 §3.D UI)

This skill is invoked to implement a Figma export. It converts a node export into an accessible Svelte component.

## Inputs

- `node_export_path` (string) — path to the Figma node JSON/export.
- `component_name` (string) — PascalCase name for the generated component.

## Outputs

Svelte component with Tailwind classes plus extracted design tokens.

## Behavior

1. Parse the node export; map frames to layout, text to type scale, fills to tokens.
2. Emit accessible markup (labels, focus order, contrast-safe defaults).
3. Never schedule alongside `opencode-ui-from-screenshot` on the same objective (conflict, RFC 06 §8).
4. Hand the result to `opencode-test` where snapshot coverage exists.

---

> This skill description is the formal contract. Phase 1 implements it as a prompt skill inside `skills/opencode-figma-to-code/` (see RFC 06).
