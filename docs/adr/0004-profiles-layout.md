# ADR 0004 — Profiles: per-worktree state under `~/.opencode/profiles/<id>/`

- **Status**: Accepted
- **Date**: 2026-07-20
- **Decision owner**: RFC 25 §4, RFC 22 §1
- **Supersedes**: —
- **Superseded by**: —

## Context

Atlas OS must run multiple parallel agent workspaces on the same machine
without:

1. Cross-contaminating SQLite journals (each profile is its own audit log).
2. Sharing credentials between unrelated work (personal vs. contractor vs.
   research-repo).
3. Losing track of "what's the active profile" when the user switches the
   desktop app and the headless CLI in the same hour.

The Hermes pattern (RFC 22 §1) calls this a _profile_: a directory holding
the journal, skills cache, embeddings cache, model credentials, and HUD
port pointer for exactly one workspace.

## Decision

A profile maps 1-to-1 to a directory:

```
~/.opencode/
  profiles/
    <id>/                # ProfileId::new(id)
      journal.sqlite     # the SQLite database
      skills/            # installed skills (skill.toml + mailbox)
      embeddings/        # sqlite-vec cache
      hud_port.txt       # last bound HUD port (recovered by the CLI)
      current            # symlink target set by `atlas profile switch`
  current                # text file naming the active ProfileId
```

- The active profile pointer lives at `~/.opencode/current`
  (`profiles::current()` reads it; `profiles::set_current()` writes it).
  CLI `--profile` overrides at invocation time; otherwise the persisted
  pointer wins.
- `AppState::switch_profile(new)` is the only path that swaps the live
  profile: it re-opens the journal and atomic-swaps both RwLocks.
- Default profile id is `default`.

## Consequences

- **No `~/.ocahs`-style monolith**: each profile is fully independent.
  Deleting a profile is `rm -rf ~/.opencode/profiles/<id>`.
- **CLI/desktop coherence**: the desktop shell and `atlas` CLI see the
  same active profile because they both read the same `~/.opencode/current`.
- **Concurrent desktop launches in different profiles**: each desktop
  process points at a different `hud_port.txt` and the browser picks the
  right HUD URL via `hud_url:` IPC.
- **No profile namespacing at the OS level**: the profile id is a string,
  not a UUID — it's a human-readable handle. Future Phase 7+ (RFC 21)
  will layer P2P sync on top.

## Alternatives rejected

- **Single shared journal for all profiles**: contradicts RFC 22 §1 (no
  cross-work-tree contamination).
- **Profile ids as UUIDs**: worse UX; users switch `atlas profile switch
acme-research`, not GUID gibberish.
- **Store state in the worktree itself (`.opencode/` in the repo)**: makes
  the worktree a single-tenant artefact and breaks "agent supervises five
  repos". Profiles live in `~/.opencode`, worktrees stay portable.
