# atlas-owasp-check — OWASP Top 10 actionable checklist (RFC 18 §12)

Applies the OWASP Top 10 (2021) to a diff touching user-critical code. Each rule is
`When` (trigger in the diff) → `Then` (required action). Failing rules block the
change until fixed or explicitly waived by a human reviewer (RFC 18 §12).

> Labels: [exact] = category name quoted verbatim from OWASP Top 10 2021;
> [heuristic] = Atlas interpretation of what to do about it in this repo.

## Inputs

- `diff` (string) — the change under review.
- `scope` (string, optional) — `web` (default), `api`, `infra`.

## Rules

| # | Rule (When → Then) | Priority |
|---|---|---|
| 1 | [exact] `A01:2021-Broken Access Control` — When the diff adds a route, handler or object lookup → Then deny-by-default, check ownership server-side, test with a second account. | 95 |
| 2 | [exact] `A02:2021-Cryptographic Failures` — When the diff touches secrets, hashes or TLS → Then no plaintext secrets, modern KDF/AEAD only, secrets via vault (RFC 18 §3). | 95 |
| 3 | [exact] `A03:2021-Injection` — When the diff builds SQL, shell, LDAP or template strings → Then parameterize / allowlist-encode; never concatenate untrusted input. | 95 |
| 4 | [exact] `A04:2021-Insecure Design` — When the diff adds a trust boundary (auth, payments, uploads) → Then threat-model it first; missing abuse cases = request changes. | 80 |
| 5 | [exact] `A05:2021-Security Misconfiguration` — When the diff changes defaults, CORS, headers or error pages → Then secure defaults, no stack traces to clients, minimal CORS. | 80 |
| 6 | [exact] `A06:2021-Vulnerable and Outdated Components` — When the diff adds/bumps a dependency → Then run the supply-chain gate (`atlas security gate`); known CVE = block. | 85 |
| 7 | [exact] `A07:2021-Identification and Authentication Failures` — When the diff touches login, session or reset flows → Then rate-limit, constant-time compare, short-lived sessions. | 90 |
| 8 | [exact] `A08:2021-Software and Data Integrity Failures` — When the diff adds unsigned updates, plugins or CI artifacts → Then verify checksum/signature before load (RFC 18 §5). | 85 |
| 9 | [exact] `A09:2021-Security Logging and Monitoring Failures` — When the diff adds auth, payment or admin actions → Then audit-log actor + action + result to the Journal (RFC 18 §8). | 75 |
| 10 | [exact] `A10:2021-Server-Side Request Forgery (SSRF)` — When the diff fetches a user-supplied URL → Then allowlist host/scheme, no internal-network access, timeout + cap. | 90 |

## Outputs

Pass/fail per rule with the offending hunk quoted, or `waived:<rule>` with reviewer name.
Escalate waivers to a human (RFC 18 §12) — the agent never waives its own findings.

---

> This skill description is the formal contract. Phase 7 implements it as a prompt
> skill inside `skills/atlas-owasp-check/` (see RFC 06).
