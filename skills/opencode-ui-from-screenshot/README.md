# opencode-ui-from-screenshot — script module (RFC 29 §3.D UI)

This skill is invoked to clone a UI section visually. It rebuilds the screenshot as an accessible Svelte component.

## Inputs

- `image_path` (string) — path to the screenshot.
- `component_name` (string) — PascalCase name for the generated component.

## Outputs

Svelte component with Tailwind classes matching the screenshot layout and tokens.

## Behavior

1. Match layout, spacing scale, and type scale observed in the screenshot; reuse repo tokens first.
2. Emit accessible markup (labels, focus order, contrast-safe defaults).
3. Never schedule alongside `opencode-figma-to-code` on the same objective (conflict, RFC 06 §8).
4. Flag any unreadable region as `unverified` instead of guessing content.

---

> This skill description is the formal contract. Phase 1 implements it as a prompt skill inside `skills/opencode-ui-from-screenshot/` (see RFC 06).
