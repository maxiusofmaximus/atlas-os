# atlas-gdpr-check — GDPR actionable checklist (RFC 18 §12)

Applies GDPR duties to a diff that handles personal data. Each rule is `When`
(trigger in the diff) → `Then` (required action). Failing rules block the change
until fixed or explicitly waived by a human reviewer (RFC 18 §12).

> Labels: [exact] = duty quoted verbatim from the cited GDPR article;
> [heuristic] = Atlas interpretation of what to do about it in this repo.

## Inputs

- `diff` (string) — the change under review.
- `data_classes` (string, optional) — personal-data categories the diff touches.

## Rules

| #   | Rule (When → Then)                                                                                                                                                                                                                 | Priority |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------- |
| 1   | [exact] Art. 5(1)(a) `lawfulness, fairness and transparency` — When the diff collects new personal data → Then record the lawful basis and surface it to the user before collection.                                               | 90       |
| 2   | [exact] Art. 5(1)(b) `purpose limitation` — When the diff reuses personal data for a new purpose → Then block reuse unless the new purpose is compatible and documented.                                                           | 90       |
| 3   | [exact] Art. 5(1)(c) `data minimisation` — When the diff adds a personal-data field → Then justify necessity; drop anything merely convenient.                                                                                     | 85       |
| 4   | [exact] Art. 5(1)(e) `storage limitation` — When the diff persists personal data → Then set a retention deadline and a deletion job; no indefinite retention.                                                                      | 85       |
| 5   | [exact] Art. 5(1)(f) `integrity and confidentiality` — When the diff stores or transmits personal data → Then encrypt in transit and at rest, least-privilege access only.                                                         | 95       |
| 6   | [exact] Art. 25 `data protection by design and by default` — When the diff adds a feature handling personal data → Then ship the most private default; opt-in, never opt-out, for extras.                                          | 85       |
| 7   | [exact] Art. 32 `security of processing` — When the diff touches auth, backups or restores → Then cover confidentiality, integrity, availability and regular testing.                                                              | 90       |
| 8   | [exact] Art. 33(1) breach notification `without undue delay and, where feasible, not later than 72 hours` — When the diff could leak personal data → Then require an incident path: detect, log, notify the DPO within the window. | 95       |
| 9   | [heuristic] Arts. 12–22 data-subject rights — When the diff stores personal data → Then expose access, rectification, erasure and export; erasure must cascade.                                                                    | 80       |
| 10  | [heuristic] Art. 28 processors — When the diff sends personal data to a third party (model API, SaaS) → Then require a processing agreement and log the transfer.                                                                  | 80       |

## Outputs

Pass/fail per rule with the offending hunk quoted, or `waived:<rule>` with reviewer name.
Escalate waivers to a human (RFC 18 §12) — the agent never waives its own findings.

---

> This skill description is the formal contract. Phase 7 implements it as a prompt
> skill inside `skills/atlas-gdpr-check/` (see RFC 06).
