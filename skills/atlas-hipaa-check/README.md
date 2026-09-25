# atlas-hipaa-check — HIPAA Safeguards actionable checklist (RFC 18 §12)

Applies the HIPAA Administrative, Physical and Technical Safeguards (45 CFR Part
160 / Part 164) to a diff that handles protected health information (PHI). Each
rule is `When` (trigger in the diff) → `Then` (required action). Failing rules
block the change until fixed or explicitly waived by a human reviewer (RFC 18 §12).

> Labels: [exact] = safeguard quoted verbatim from the cited 45 CFR section;
> [heuristic] = Atlas interpretation of what to do about it in this repo.

## Inputs

- `diff` (string) — the change under review.
- `phi_classes` (string, optional) — PHI categories the diff touches.

## Rules

| #   | Rule (When → Then)                                                                                                                                                                                                | Priority |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| 1   | [exact] §164.502(b) `minimum necessary` — When the diff reads PHI → Then request only the fields the task needs; no `SELECT *` on PHI stores.                                                                     | 95       |
| 2   | [exact] §164.308(a)(1) `security management process` — When the diff adds a PHI flow → Then risk-analyse it first; unanalysed PHI handling = request changes.                                                     | 90       |
| 3   | [exact] §164.312(a)(1) `access control` — When the diff exposes PHI → Then unique user IDs, least privilege, automatic logoff; deny-by-default.                                                                   | 95       |
| 4   | [exact] §164.312(b) `audit controls` — When the diff creates, updates or discloses PHI → Then audit-log actor + action + result to the Journal (RFC 18 §8).                                                       | 90       |
| 5   | [exact] §164.312(c)(1) `integrity` — When the diff writes PHI → Then protect against improper alteration: checksums, transactions, no silent overwrites.                                                          | 90       |
| 6   | [exact] §164.312(e)(1) `transmission security` — When the diff moves PHI over a network → Then encrypt in transit; no PHI in URLs, logs or error messages.                                                        | 95       |
| 7   | [exact] §164.310 `physical safeguards` — When the diff changes backups, media or device handling → Then cover disposal, re-use and backup with a documented procedure.                                            | 75       |
| 8   | [exact] §§164.400–414 breach notification — When the diff could cause unsecured-PHI acquisition, access, use or disclosure → Then require an incident path: assess, log, notify per the Breach Notification Rule. | 95       |
| 9   | [exact] §164.514 de-identification — When the diff claims data is de-identified → Then apply Safe Harbor removal or Expert Determination; re-identification risk = still PHI.                                     | 85       |
| 10  | [heuristic] §164.308(a)(3)/(4) workforce + information access — When the diff widens who can see PHI → Then role-based access with documented authorization; agents get the minimum role.                         | 80       |

## Outputs

Pass/fail per rule with the offending hunk quoted, or `waived:<rule>` with reviewer name.
Escalate waivers to a human (RFC 18 §12) — the agent never waives its own findings.

---

> This skill description is the formal contract. Phase 7 implements it as a prompt
> skill inside `skills/atlas-hipaa-check/` (see RFC 06).
