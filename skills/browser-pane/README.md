# browser-pane

Bundled, opt-in skill (RFC 28 §I item 4). Teaches the Planner to surface an
HTML artifact next to the agent in a live terminal pane instead of asking the
operator to switch windows.

## When to use it

Pick this when a step produces a browser-viewable artifact and the operator is
already in a kitty-graphics terminal (ghostty, kitty, WezTerm):

- a **plan** rendered by RFC 12 (`atlas plan` HTML export),
- a **validation report** from RFC 14 (`atlas audit --json` → HTML),
- a **diff** or preview page from the Coding engine.

## How it works

The skill emits a lateral shell-out — Atlas OS never bundles the browser:

```bash
atlas browser probe                       # is terminal-browser installed? is the terminal capable?
atlas browser open ./plan.html --split right
atlas browser open https://example.com/vs/1234
```

`browser open` spawns the operator-owned `terminal-browser` (MIT, Electron/
Chromium) detached, then publishes `artifact_preview_opened` on the Kernel Bus
so the HUD/journal record what was previewed (RFC 28 §I item 6).

## Guardrails

- **Security (RFC 18):** a full Chromium engine is a new surface. The capability
  is opt-in; treat agent-driven `action` calls as a `SensitiveAction` and require
  confirmation in `HUMAN_IN_LOOP`/`AUTOPILOT` (RFC 21).
- Never accept URLs from unsigned skills without the SensitiveActions policy.
- The external process does not inherit keychain secrets or `Profile` data.
- Without the binary, `open` exits non-zero with the install recipe — never a
  silent no-op.

See `docs/terminal-browser-integration.md` for the operator install guide.
