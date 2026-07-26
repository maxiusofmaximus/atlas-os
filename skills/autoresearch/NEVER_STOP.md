<!-- Extract from karpathy/autoresearch program.md (MIT License).
     Source: https://github.com/karpathy/autoresearch/blob/master/program.md
     Copyright (c) 2025-2026 Andrej Karpathy.
     This file quotes the two governing guards ("NEVER STOP" + "rewind sparingly")
     verbatim so they can be cited by other OpenCode OS skills (RFC 23 §7) without
     having to re-quote the full program.md. -->

# NEVER STOP — Autoresearch autonomy guards

> Two guards, extracted verbatim from `karpathy/autoresearch/program.md`, govern every autonomous hill-climbing loop in OpenCode OS (RFC 28 §A).

## NEVER STOP

> **NEVER STOP**: Once the experiment loop has begun (after the initial setup), do NOT pause to ask the human if you should continue. Do NOT ask "should I keep going?" or "is this a good stopping point?". The human might be asleep, or gone from a computer and expects you to continue working *indefinitely* until you are manually stopped. You are autonomous. If you run out of ideas, think harder — read papers referenced in the code, re-read the in-scope files for new angles, try combining previous near-misses, try more radical architectural changes. The loop runs until the human interrupts you, period.

## Rewind sparingly, if ever

> If you feel like you're getting stuck in some way, you can rewind but you should probably do this very very sparingly (if ever).

## Crash discipline

> If a run crashes (OOM, or a bug, or etc.), use your judgment: If it's something dumb and easy to fix (e.g. a typo, a missing import), fix it and re-run. If the idea itself is fundamentally broken, just skip it, log "crash" as the status in the tsv, and move on.

## Timeout gravity

> Each experiment should take ~5 minutes total (+ a few seconds for startup and eval overhead). If a run exceeds 10 minutes, kill it and treat it as a failure (discard and revert).

## Pacing model

> As an example use case, a user might leave you running while they sleep. If each experiment takes you ~5 minutes then you can run approx 12/hour, for a total of about 100 over the duration of the average human sleep. The user then wakes up to experimental results, all completed by you while they slept!

---

## How OpenCode OS translates these guards (RFC 28 §A)

These guards are **not** pluggable prompt engineering for the agent's LLM — that would be brittle. They map to hard Rust invariants in `src-tauri/src/supervisor/loop.rs::Autoresearch`:

| Guard in `program.md`                          | Rust invariant in OpenCode OS supervisor                                                |
|---|---|
| NEVER STOP                                     | No user-input `await` between candidates; the loop runs unconditionally until `Mission.options.max_steps` or `timebox_seconds` hit, regardless of LLM idle. |
| Rewind sparingly                               | `git reset --hard` is only permitted after `metric_after >= metric_baseline_at_step`. Reverted candidates are persisted to `autoresearch_candidates.kept=0` for audit; streak detection: 3 consecutive reverts → `outcome=plateau` abort (doom-loop guard, RFC 19). |
| Crash discipline                               | Typos/NaN/etc. score 0.0 → discard; persisted to DB as `kept=0`; loop continues unchanged. |
| Timeout gravity                                | `tokio::time::timeout(Duration::from_secs(timebox_seconds))` per candidate; 10-minute wall-clock ceiling enforced. |
| Pacing model                                   | Used only for HUD telemetry estimate, not for agent behavior. `AutoresearchCard.svelte` shows ETA based on observed per-step time. |

The autonomous stance is enforced by code, not by prompt. The LLM proposes ideas; the supervisor decides `keep`/`discard` numerically.
