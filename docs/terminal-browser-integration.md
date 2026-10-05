# terminal-browser integration

> **Operator-facing guide** for pairing [`zenbu-labs/terminal-browser`](https://github.com/zenbu-labs/terminal-browser) (MIT) with Atlas OS. terminal-browser is a **real Chromium browser drawn inside the terminal** via the [kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/). Atlas OS treats it as an **external, opt-in, lateral** process — it is **never bundled** (RFC 28 §I, RFC 25 §11, AGENTS.md §6).

This document is the install recipe referenced by RFC 28 §I, mirroring the §B pattern of [`docs/intelligent-terminal-integration.md`](./intelligent-terminal-integration.md). Atlas OS adds **no crates, no feature flags, and no migrations** for this integration.

---

## 1. What it is

terminal-browser renders pixels produced by a real browser (Chromium, via Electron's offscreen-rendering API) into your terminal using the kitty graphics protocol, sending only the changed regions per frame. The surrounding browser chrome is a Rust graphics engine with a React custom renderer; keyboard/mouse/trackpad events are read from the terminal (plus a background Swift helper on macOS for OS-level input) and replayed as synthetic events to Chromium.

Why pair it with Atlas OS:

- **Agent and website share one terminal tab.** Keep `atlas` and the browser side by side.
- **Web capability for the agent.** `terminal-browser action` is an `agent-browser`-compatible CLI for driving open browsers.
- **Preview Atlas artifacts.** Open HTML plans/reports (RFC 12 plans, RFC 14 validation reports, diffs) in a split pane next to the agent.
- **SSH previews.** Preview websites running on remote machines without a local browser.

---

## 2. Requirements

- **A terminal implementing the kitty graphics protocol and kitty unicode placeholders.** Verified upstream: `ghostty`, `kitty`, `cmux`, and many libghostty-based terminals (`supacode`, …); VS Code's terminal is also cited. If the terminal lacks these features the TUI may be garbled.
- **macOS or Linux** for the primary install path (curl / Homebrew).
- **Windows requires WSL.** Windows terminals with kitty-graphics support are limited; upstream lists [`noctty.com`](https://noctty.com/) as tested working inside Windows.
- **Multiplexers may break graphics.** Multiplexers rewrite terminal output and can corrupt graphics commands. tmux works with terminal-browser directly (not yet through the Claude Code plugin); `herdr` has poor performance via that plugin. Other multiplexers are untested upstream.

---

## 3. Install / upgrade

```bash
# curl (macOS & Linux)
curl -fsSL https://terminal-browser.sh/install | bash

# Homebrew (macOS & Linux)
brew install terminal-browser
```

Upgrade to the latest version:

```bash
terminal-browser upgrade
```

Verify the binary is on `PATH`:

```bash
terminal-browser --help
```

---

## 4. Basic usage

```bash
terminal-browser                 # launch the browser
terminal-browser open <url>      # open the browser at a URL
terminal-browser --split right   # open in a split pane to the right
terminal-browser open --ssh <user@host> <url>   # route requests through a remote server
terminal-browser ls              # list open browsers
terminal-browser action          # agent-browser-compatible CLI for open terminal-browsers
terminal-browser upgrade         # upgrade to the latest version
```

---

## 5. Pairing with Atlas OS

Nothing special is required — terminal-browser is an ordinary external CLI. The integration is "same terminal, adjacent panes":

```bash
# Terminal tab / tmux pane 1 — the agent
opencode mission new "add a settings page"

# Terminal tab / tmux pane 2 — the live artifact
terminal-browser open ./atlas-plan.html --split right
```

- **Preview Atlas artifacts.** Generate an HTML plan or validation report and open it with `terminal-browser open`.
- **Remote dev server.** `terminal-browser open --ssh user@host http://localhost:3000`.
- **Split next to the agent.** `terminal-browser open <url> --split right` places the browser beside the agent pane.

> **Atlas OS subcommands (RFC 28 §I, implemented):**
> `atlas browser probe` (read-only capability check), `atlas browser open <url> [--split right]`, `atlas browser ls`, `atlas browser action -- <args>`. These shell out to terminal-browser; `open`/`ls`/`action` exit non-zero with clear remediation text when the binary is absent — never a silent no-op.

---

## 6. Agent access

`terminal-browser action` is an agent-browser-compatible CLI that lets an agent interact with open terminal-browsers. Atlas OS wraps this (RFC 28 §I item 5) with the typed `BrowserAction { Navigate, Click, Type, Snapshot }` and the `atlas browser action` passthrough, so the Orchestrator can "use the web" as a lateral tool. The `agent-browser` contract is not pinned yet (item 7): args are forwarded verbatim, never invented.

Reference pattern (upstream, **not** an Atlas OS dependency): the [Claude Code plugin](https://github.com/zenbu-labs/terminal-browser/tree/main/claude-code-plugin) exposes a `/browser` command and a `Browser.open/close` plugin API. It is useful as a design reference for "agent opens/closes the web surface", not as an install requirement here.

---

## 7. Windows / WSL

Install inside WSL, then use a kitty-graphics-capable terminal on Windows. Upstream lists [`noctty.com`](https://noctty.com/) as tested. Treat Windows support as experimental.

---

## 8. Troubleshooting

| Symptom                                                 | Likely cause                                                                   | Fix                                                                                     |
| ------------------------------------------------------- | ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------- |
| Garbled TUI / no pixels                                 | Terminal lacks kitty graphics protocol or kitty unicode placeholders           | Use `ghostty`, `kitty`, `cmux`, or a libghostty-based terminal                          |
| Graphics break under a multiplexer                      | Multiplexer rewrites terminal output                                           | Run without the multiplexer, or use tmux (direct usage works; plugin path does not yet) |
| Mouse position slightly off                             | Terminal does not report pixel coordinates (common in Claude Code plugin path) | Expected limitation; use the CLI directly                                               |
| OS prompts "terminal wants to access the local network" | The plugin makes fetch requests to a local HTTP server to talk to the CLI      | Allow once; expected for the plugin path only                                           |
| Cannot resize the browser as desired                    | Minimum width enforced by the host UI                                          | Use a wider pane                                                                        |

---

## 9. Uninstall

```bash
# Homebrew
brew uninstall terminal-browser

# curl-installed: remove the binary from wherever the installer placed it
# (typically ~/.local/bin or /usr/local/bin), then remove any shell PATH entry it added.
```

Because Atlas OS never bundles terminal-browser, uninstalling it has **no effect** on the Atlas OS build.

---

## 10. References

- RFC 28 §I — external tool integration spec (`Atlas OS/28 - External Tool Integration.md`).
- RFC 25 §11 — single-binary distribution invariant (why terminal-browser is never bundled).
- RFC 18 — calling an agent-accessible browser is a `SensitiveAction` (opt-in).
- RFC 21 — confirm policy for `HUMAN_IN_LOOP` / `AUTOPILOT`.
- RFC 24 §16 — HUD / external preview surfaces.
- Upstream — <https://github.com/zenbu-labs/terminal-browser> (MIT).
