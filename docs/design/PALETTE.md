# Atlas OS — Color System & Palette (RFC 66 Addendum A)

**Date:** 2026-10-06 · **Status:** Proposed [R] · **Companion to:** `Atlas OS/66 - UX Architecture & Design System.md`
**Artifacts:** `docs/design/mockup.html` (hi-fi), `docs/design/wireframe.html` (lo-fi)

> This document answers the operator's ask: *"research the best color consensus; use the `impeccable` and `frontend-design` skills."*
> Every hex below was **computed from OKLCH** (script `oklch→sRGB`) and its **WCAG contrast ratio measured** — no hand-picked guesses (rule 27).

---

## 1. Research consensus (what the field agrees on)

Sources consulted: `impeccable` (v4.1.1) color strategy + `design-taste-frontend` §4.2/§8 (dark-mode protocol, accent/consistency locks), plus live reference tooling (Accessible Palette Studio, ChromUI, HexPalette, EvvyTools token generators).

The convergent best practice across all of them:

| Consensus principle | Implication for Atlas OS |
|---|---|
| Build scales in **OKLCH** (perceptually uniform), export sRGB | Tokens authored in OKLCH; shipped as hex/`oklch()`. |
| **Semantic tokens**, not raw swatches (`bg/surface/border/text/muted/primary/focus/success/warning/error/info`) | One closed token set; components never use raw color. |
| Validate with **WCAG 2.2** (normative) + **APCA** (perceptual) | WCAG ratios measured (§4); APCA noted as review aid. |
| **Dark ≠ inverted light.** Dark needs *lower* saturation + elevated surfaces | Separate dark/light token blocks; state colors desaturated on dark. |
| **Never color alone** — pair with icon/shape/text (CVD) | State = color **+ spine + icon + label** (see §5). |
| **Focus ring ≥ 3:1** (WCAG 1.4.11); text ≥ 4.5:1 (AA), 7:1 target | `--a-focus` = primary, 4.17:1 vs bg; body text 16.3:1. |
| **No pure `#000`/`#fff`** as surfaces | Graphite `#0d0d0d` / off-white `#ffffff` for light. |
| **One accent, locked** — no AI-purple/neon defaults | Single teal signal; semantic roles are the *only* other colors. |

---

## 2. Design read & mood

**Reading this as:** an *operate*-mode Mission Control for technical operators coordinating a swarm, with a **calm-instrumentation** language, leaning toward a bespoke token system (no third-party DS).

**Mood sentence:** *"an instrument panel at night — a graphite room, one teal signal light, everything else reads as precision."*

**Strategy:** *Restrained* — tinted-free graphite neutrals carry the surface; the teal accent is used for **human attention / interactivity** only; the rest of the color budget is spent on a **closed semantic vocabulary** for agent/task state.

> **Not a field consensus.** The teal accent is a **differentiation choice**, not a measured convention — none of the products verified in `docs/design/CONSENSUS_AUDIT.md` uses teal as its signal. What *is* consensus is the **method** (OKLCH scales, semantic tokens, WCAG 2.2/APCA, CVD cues, dark≠inverted); the hue is Atlas's. See RFC 66 D-66-03.

**Why not the incumbent (GitHub Primer)?** The current code hardcodes `#0d1117/#161b22/#58a6ff/...` **[O]** — it is literally GitHub's palette, which is the "clone" anti-pattern (brief §14). Atlas OS's identity must come from its *function*: a mission console, not a code host. Hence: **pure graphite neutrals (zero hue) + one teal signal**, distinct from VS Code `#007ACC` and GitHub `#58A6FF`.

---

## 3. Palette (OKLCH → hex)

Neutrals are **pure graphite, chroma 0** (no hidden tint). Semantic roles sit at lightness tuned for the dark surface.

### 3.1 Dark (default)

| Token | OKLCH | Hex | Role |
|---|---|---|---|
| `--a-bg` | `0.16 0 0` | `#0d0d0d` | App background |
| `--a-surface` | `0.20 0 0` | `#161616` | Panel / card |
| `--a-surface-2` | `0.24 0 0` | `#1f1f1f` | Elevated / hover |
| `--a-border` | `0.28 0 0` | `#292929` | Hairline divider (decorative) |
| `--a-border-strong` | `0.37 0 0` | `#404040` | Decorative emphasis |
| `--a-border-ui` | `0.56 0 0` | `#747474` | **Control boundary (≥3:1)** |
| `--a-text` | `0.94 0 0` | `#ebebeb` | Primary text |
| `--a-text-muted` | `0.73 0 0` | `#a8a8a8` | Secondary |
| `--a-text-faint` | `≈0.62 0 0` | `#868686` | Tertiary / placeholder |
| `--a-primary` | `0.80 0.115 192` | `#4fd5d1` | **Signal (interaction/attention)** |
| `--a-info` | `0.74 0.12 245` | `#65b2f1` | Running / reading / planning |
| `--a-ok` | `0.78 0.14 150` | `#6fd087` | Success / approved |
| `--a-warn` | `0.82 0.13 80` | `#f0ba59` | Paused / reviewing / cost |
| `--a-err` | `0.72 0.17 22` | `#fd7273` | Error / doom_loop |
| `--a-violet` | `0.75 0.12 295` | `#b39ef2` | Research / fork / learning |
| `--a-focus` | = `--a-primary` | `#4fd5d1` | Focus ring |

### 3.2 Light

| Token | OKLCH | Hex |
|---|---|---|
| `--a-bg` | `1 0 0` | `#ffffff` |
| `--a-surface` | `0.972 0 0` | `#f6f6f6` |
| `--a-text` | `0.22 0 0` | `#1b1b1b` |
| `--a-text-muted` | `0.49 0 0` | `#606060` |
| `--a-text-faint` | `0.60 0 0` | `#808080` |
| `--a-border-ui` | `0.62 0 0` | `#868686` |
| `--a-primary` | `0.52 0.12 192` | `#007d7b` |
| `--a-info` | `0.50 0.13 245` | `#0068a7` |
| `--a-ok` | `0.50 0.14 150` | `#007834` |
| `--a-warn` | `≈0.53 0.12 72` | `#966000` |
| `--a-err` | `0.53 0.19 25` | `#c2272d` |
| `--a-violet` | `0.48 0.15 295` | `#6646a8` |

---

## 4. Contrast validation (measured, not asserted)

**On dark `--a-bg` (target: text ≥4.5 AA, 7 preferred; UI ≥3):**

| Pair | Ratio | Verdict |
|---|---|---|
| text / bg | **16.28:1** | AAA |
| muted / bg | **8.12:1** | AAA |
| faint / bg | **5.34:1** (`#868686`, after the §4.1 fix) | AA |
| primary / bg | 10.88:1 | AAA |
| info | 8.51:1 · ok 10.23:1 · warn 10.99:1 · err 7.25:1 · violet 8.42:1 | all AAA |
| border-ui / bg | **4.17:1** | ✅ ≥3 (1.4.11) |
| button label (bg) on primary | 10.88:1 | AAA |

**On light `--a-bg:#ffffff`:** text 17.31:1 · muted 6.26:1 · primary 4.96:1 · info 5.93 · ok 5.61 · warn 5.29 (`#966000`, after the §4.1 fix) · err 5.83 · violet 6.98 · white-on-primary 4.96:1. All ≥ AA. `faint` = 3.95:1 → **placeholder/non-essential only** [P].

**CVD note:** err(22°)/ok(150°)/warn(80°) separate by hue *and* ~0.05–0.10 lightness; still, state is never color-only (§5). Tritanopia flattens warn↔info — the mandatory icon+label pairing covers it. [P] spot-check with the `accessibility-audit` skill + a CVD simulator before ship.

---

### 4.1 FASE 9 re-measurement (2026-10-06, independent script, all three surfaces)

Recomputed from the hex values (WCAG 2.x relative luminance + APCA-W3 0.0.98G). Section 4 holds to ±0.05 on `--a-bg`. Measuring on the other surfaces exposes three gaps:

| Finding | Measured | Impact | Required action |
|---|---|---|---|
| Dark `faint` on `surface-2` | 4.29:1 (AA needs 4.5) | Fails AA on elevated/hover surfaces | **Applied:** `faint` → `#868686` (5.34 / 4.97 / 4.53 on bg / surface / surface-2) |
| Light `warn` on `surface` | 4.40:1 (AA needs 4.5) | Warn text fails AA on `#f6f6f6` cards | **Applied:** light `warn` → `#966000` (5.29 on bg, 4.89 on surface) |
| Light `faint` on `bg` / `surface` | 3.95 / 3.65:1 | Already flagged [P] | Placeholder / non-essential only; never state labels |

APCA advisory (Lc ≥ 60 for body-size text): dark `muted` Lc 55, `err` 50, `info` 57, `violet` 56 and `faint` 35 fall short at small sizes. They pass WCAG AA, so they stay valid for large or bold text. Small state labels (≤14px) must use `--a-text` plus the glyph cue from §5, with the color on the glyph only. Light theme is Lc 73-84 for all semantic roles except `faint` (67).

Verified in `mockup.html`: `prefers-reduced-motion` disables `.pulse`; `lang="en"` set. Still open: CVD simulation and a keyboard focus-visible audit on the hi-fi mockup (no `:focus-visible` rule found there, so the focus ring is not yet demonstrated).

---

## 5. State mapping (closed vocabulary → real enums)

No component invents a color. `AgentStatus` **[O]** maps 1:1:

| Status | Token | + cue (never color-only) |
|---|---|---|
| `queued` | `faint` | hollow dot |
| `reading`/`planning`/`coding` | `info` | filled dot + label |
| `reviewing` | `warn` | half dot + label |
| `idle` | `text-muted` | ring dot |
| `paused` | `warn` | pause glyph |
| `doom_loop` | `err` | ⚠ glyph + pulse |
| `error` | `err` | ✖ glyph |
| `success` | `ok` | ✓ glyph |
| `unknown` | `faint` | `?` glyph (present, unclassifiable) |

`Confidence`: High→`ok`, Medium→`warn`, Low→`violet`, Block→`err`.
`ApprovalDecisionKind`: Apr→`ok`, Deny→`err`, Steer→`info`, Fork→`violet`.

**`unknown` (new — CONSENSUS_AUDIT §3.1-5):** only Herdr [Os] documents an explicit `unknown` state (1 of 14 verified; the other references' docs read this session do not cite one) — a single-source proposal, justified mainly by Atlas's detector being able to fail to classify. Map it to `--a-text-faint` **`#868686`** — **measured** 5.33 / 4.97 / 4.52:1 vs `bg` / `surface` / `surface-2`, **AA on all three** (reuses an existing token → **no new hex**). It shares `faint` with `queued`; they are distinguished by **glyph** (`?` vs hollow dot) per the color+glyph+label rule.

---

## 6. What changed vs. the incumbent

| | Incumbent **[O]** | RFC 66 Addendum A **[R]** |
|---|---|---|
| Neutrals | GitHub blue-grey (`#0d1117`, `#161b22`) | Pure **graphite** (chroma 0) |
| Accent | `#58a6ff` (GitHub blue) | **Teal `#4fd5d1`** — distinct signal |
| Authoring | 30 hardcoded hex × component | **OKLCH-authored, ~18 semantic tokens** |
| Validation | none recorded | **WCAG measured (§4) + CVD cues** |
| Dark mode | shades of one palette | independent light tokens, AA-verified |

---

## 7. Open items

- [P] Bake `docs/design/tokens.css` as the canonical source and diff against `impeccable`'s detector once implemented (FASE 10).
- [P] Bundle a distinctive mono (e.g. JetBrains Mono) vs. system mono — currently **system stack** to honor the no-new-dependency boundary (RFC 66 §9.3). *Decision pending operator.*
- [P] APCA Lc pass for body/chrome at production sizes.
