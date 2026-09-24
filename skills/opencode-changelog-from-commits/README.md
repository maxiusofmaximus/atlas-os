# opencode-changelog-from-commits — script module (RFC 29 §3.D Docs)

This skill is invoked at release time. It drafts the changelog from Conventional Commits since the last tag.

## Inputs

- `since_tag` (string, optional) — defaults to the latest git tag.
- `range` (string, optional, default `HEAD`)

## Outputs

Changelog draft grouped by `feat` / `fix` / `docs`, one line per commit.

## Behavior

1. List commits in range via `git log`; parse Conventional Commits prefixes.
2. Group by `feat`, `fix`, `docs`; drop `chore` noise unless it changes behavior.
3. Quote the short SHA per line so every entry is traceable.
4. Never commit or tag; output the draft for operator approval only.

---

> This skill description is the formal contract. Phase 1 implements it as a prompt skill inside `skills/opencode-changelog-from-commits/` (see RFC 06).
