# Microsoft Intelligent Terminal integration

> **MS trademark notice.** Per Microsoft trademark policy, Atlas OS user-facing surfaces call this feature **"agent pane integration"** — never "Intelligent Terminal". The rights in the Microsoft upstream (`microsoft/intelligent-terminal`, MIT) belong to Microsoft; we ship against the public ACP v1 wire protocol and publish this install guide for operators who already have the host installed.

This document captures the **operator-facing** install recipe (RFC 28 §B item 7): how to point IT's `Settings.json` at the `atlas` binary as a delegate-agent, how Atlas OS auto-detects IT lift-off via `WT_COM_CLSID`, and how to troubleshoot discovery when the pane does not render the OpenCode slash-command menu.

The Rust-side contract lives in `src-tauri/src/acp/` and `src-tauri/src/journal/agent_events.rs`; the wire-level spec is RFC 28 §B and the OSC 9001 envelope is mirrored at `src-tauri/specs/osc-9001.md`.

---

## 1. Prerequisites

1. **Atlas OS** built with the `acp-server` feature:

   ```powershell
   pnpm install
   pnpm tauri:build -- --features acp-server
   ```

   The produced `atlas` binary advertises ACP `initialize` / `session/new` / `session/prompt` / `session/set_mode` / `$/cancel_request` only when the feature is compiled in — without it the binary is unchanged and the rest of this guide is a no-op.

2. **Microsoft Intelligent Terminal** 0.1.1+ running on Windows 10 build 19041+ (or Windows 11). Older IT releases lack the `wtcli` subcommand surface that the `agent_session_events` worker consumes.

3. **`atlas` on `PATH`** — IT auto-discovers the ACP host process by shelling out to `atlas` (see `microsoft/intelligent-terminal` README §"Get Started"). Easiest:

   ```powershell
   $binDir = "$env:LOCALAPPDATA\AtlasOS\bin"
   New-Item -ItemType Directory -Force $binDir | Out-Null
   Copy-Item .\src-tauri\target\release\opencode.exe $binDir
   $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
   if ($userPath -notlike "*$binDir*") {
       [Environment]::SetEnvironmentVariable('Path', "$userPath;$binDir", 'User')
   }
   ```

   Open a fresh terminal and verify `opencode --version` returns the build version before wiring IT.

---

## 2. Register `atlas` as the IT delegate-agent (Alt+Shift+B)

IT's `delegate-agent` (Alt+Shift+B / Alt+Shift+/) is wired through the **Agent Configuration** portion of IT's `Settings.json`. Open IT `Settings` → `Agent` and add a new registered agent, or edit `%LOCALAPPDATA%\Packages\Microsoft.IntelligentTerminal_*\LocalState\settings.json` (the MSIX variant) directly:

```jsonc
{
  "agents": {
    "opencode": {
      "command": "opencode",
      "args": [],
      // Atlas OS auto-detects IT via WT_COM_CLSID and swaps into ACP
      // JSON-RPC stdio loop. No extra argv needed.
      "icon": "⚡",
      "displayName": "Atlas OS"
    }
  },
  "delegateAgent": "opencode"
}
```

After saving, restart IT. Alt+Shift+B in any pane now spawns `atlas`, which in turn:

1. Detects the `WT_COM_CLSID` env var that IT seeds before launching the delegate.
2. Invokes `atlas_os::acp::run_server()` (RFC 28 §B item 4) instead of the clap dispatcher.
3. Responds to IT's `initialize` with `agentInfo.name = "atlas"` + `title = "Atlas OS"` + `loadSession = false`.
4. After `session/new`, streams `available_commands_update` with the six OpenCode slash commands (`/atlas mission new`, `/atlas fork`, `/atlas resume`, `/atlas exec step`, `/atlas fix [hint]`, `/atlas restart`).
5. Honours `session/set_mode` by parsing the ACP mode tag onto `crate::acp::mode_mapping::SupervisorState` and emitting `currentModeUpdate` (RFC 19 §6.1.2).

---

## 3. Built-in slash commands

Available everywhere IT can show an ACP slash-command menu:

| Command                                  | Behaviour                                                                                                              |
| ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `/atlas mission new <prompt>`            | Delegates to `atlas mission new <prompt>` (RFC 25 §3.9); new mission is journaled in M1–M12; pane renders plan id. |
| `/atlas fork`                             | Delegates to `atlas fork`; opens a new branch sharing the consolidated mission prompt under a fresh mission id.    |
| `/atlas resume`                           | Delegates to `atlas resume`; runs the latest checkpoint of the current mission.                                    |
| `/atlas exec step <id> <step>`            | Delegates to `atlas exec step`; single-step Coding→Validation→Repair cycle for the given plan id and step id.      |
| `/atlas fix [hint]`                      | Captures the active pane scrollback via `wtcli active-pane` + `wtcli capture-pane --last-prompt`, routes it to the Repair engine (RFC 15). |
| `/atlas restart`                          | Calls `session/close` + `session/new` with the same cwd — clean-slate restart of the ACP session.                     |

Phase 1.5d ships the catalogue + handler wiring. The actual hill-climbing host loop (i.e., the LLM-driven Phase 2 work that turns scrollback into a Repair-input diff) arrives in a later phase; until then `/atlas fix` records the operator intent in the Journal and surfaces a `\u201cPhase 1.5d: agent host loop not wired.\u201d` message — never a silent no-op.

---

## 4. `agent_session_events` worker (Channel 2)

IT re-broadcasts OSC 9001 envelopes (and `wtcli send-event` calls) to every `wtcli listen --event "agent.*" --json` subscriber. RFC 28 §B item 5 ships a Rust worker (`src-tauri/src/acp/listen_worker.rs`) that:

1. Spawns `wtcli listen --event "agent.*" --json` as a subprocess.
2. Parses each JSON-line envelope into `AgentEventEnvelope`.
3. UPSERTs into `agent_session_events` (M14, see `src-tauri/src/journal/agent_events.rs`) with `ts`, `pane_id`, `event_type`, `agent`, `task_id`, `payload_json` (verbatim).

In Phase 1.5d the actual subprocess spawn is a typed stub returning `SpawnError::StubNotWired` — the production shell-out unlocks alongside the Phase 2 host-loop effort. The envelope parser + persistence decision are wired now so the Journal tail schema is fixed before the subprocess lands.

---

## 5. Troubleshooting `WT_COM_CLSID` discovery

The auto-detection relies on the env var IT sets when it spawns the delegate agent. If `atlas` boots straight into the CLI without launching the ACP host loop:

1. **Verify the env var is set inside the pane IT spawned.** In the IT pane:

   ```powershell
   echo $env:WT_COM_CLSID
   ```

   Empty / `$null` means IT did not mark this pane as an agent pane. Reopen IT `Settings` → `Agent` and confirm the agent block matches the JSONC above (the `delegateAgent` line is what flips the env).

2. **If `#1` returns a CLSID but the pane shows `atlas --help` output**, the binary was compiled without `--features acp-server`. Rebuild with the flag (see §1 step 1). You can verify the feature at runtime:

   ```powershell
   opencode --features acp-server detect
   ```

   (Wire command pending Phase 2; for today the signal is `should_run_acp_server()` in `src-tauri/src/cli/bin/opencode.rs` returning true only when the feature is active.)

3. **Manual override for local ACP smoke tests.** Set `ATLAS_ACP_FORCE=1` before `atlas` and pipe in a JSON-RPC request on stdin:

   ```powershell
   $env:ATLAS_ACP_FORCE = "1"
   opencode
   ```

   The binary now serves the ACP host loop regardless of `WT_COM_CLSID`. IT itself is not involved, so this is only useful for manual upstream-client tests (e.g. running `agent-client-protocol`'s example clients). The `ATLAS_ACP_FORCE` env var is documented for operators; it is not used by any production flow.

4. **Pane telemetry missing in the HUD card.** The HUD Mission Control reads `agent_session_events` (M14) — the table is always created (cheap; lets HUD read pane telemetry on non-Windows hosts too) but it is only populated when the `wtcli listen` worker is wired (Phase 2). On Windows Phase 1.5d the pane-telemetry card will be empty until Phase 2 — this is expected and tracked as RFC 28 §B items 5+8.

5. **Performance / hanging panes.** If IT stalls after a `/atlas fix` slash command, the ACP host loop is waiting for a Repair run that has not been wired into the loop yet — this is the Phase 1.5d "host loop not wired" refusal path. Type `/atlas restart` to recover. The behaviour is auditable as a `StopReason::Refusal` in the Journal `prompt_verdicts` tail.

---

## 6. Uninstall

1. Remove the agent block from IT `Settings.json`.
2. Reopen IT; the version mismatch prompts IT to delete its per-pane hook cache for the orphaned agent.
3. Optionally `Set-Content $PROFILE` to remove the `AtlasOS\bin` entry from `$env:PATH`.

A re-registration prompts IT to refresh the hook cache — idempotent per RFC 28 §B source item IT-001.

---

## 7. References

- RFC 28 §B — external tool integration spec (`Atlas OS/28 - External Tool Integration.md`).
- RFC 19 §6.1.2 — `session/set_mode` override of SupervisorState.
- RFC 04 §9 — ACP server as a third Orchestrator frontend (next to CLI and HUD).
- `src-tauri/src/acp/{mod, mode_mapping, commands, delegate, listen_worker}.rs` — Rust implementation.
- `src-tauri/src/journal/agent_events.rs` — M14 persistence API.
- `src-tauri/specs/osc-9001.md` — verbatim copy of `microsoft/intelligent-terminal/doc/specs/llm-agent-event-integration.md` (MIT).
- `agent-client-protocol` crate v2.0.0 — `https://crates.io/crates/agent-client-protocol`.
- Upstream IT docs — `https://github.com/microsoft/intelligent-terminal`.
