# 18 - Security

Extensible como Hermes o OpenClaw, pero **seguro**. Esta es la crítica que se le hace a OpenClaw y aquí se resuelve.

---

## 1. Principios

- **Sandbox por defecto**. Cualquier MCP, CLI, plugin o skill externa se ejecuta aislada (`07 - MCP.md`, `08 - CLI.md`).
- **Allowlist, no denylist**. Lo no explicitamente permitido está prohibido.
- **Secretos nunca en claro**. Sistema de secretería obligatorio.
- **Firma de skills cerradas**..skills empaquetadas requieren signature verificable.
- **Auditoría permanente**. Cada acción sensible se persiste.
- **Falla hacia lo seguro** en todo edge case.

## 2. Política de aprobaciones

| Acción | Default | Override por |
|---|---|---|
| Lectura FS workspace | Auto | — |
| Escritura FS workspace | Confirm | usuario |
| Escritura fuera de workspace | Forbidden | — |
| Salida de red saliente | Forbidden | usuario |
| Shell exec interactive | Forbidden | — |
| Shell exec sandbox | Confirm | — |
| Instalar skill abierta | Confirm | — |
| Instalar skill cerrada sin firma | Forbidden | — |
| Aplicar Pulumi `apply` | Confirm | — |
| Enviar secretos por red | Forbidden | — |
| Acceder `.env`, secrets.json, `*TOKEN*` | Forbidden | usuario scope |
| Borrar archivo > 50KB | Confirm | — |
| Push git | Confirm | — |

## 3. Secretos

Se cargan vía vault del SO (Keychain/Spanner) o un vault interno sqlite cifrado. Nunca se inyectan en prompts al agente. Para modelos que requieren API keys, el Orchestrator inyecta vía backend sin exponer el texto al agente.

## 4. Cadena de suministro

Antes de instalar un package (npm, pip, go mod, cargo crate…):
- Socket telemetría: typosquatting, postinstall scripts, env-var access.
- Snyk CVE check.
- Comparación con checksum firmado.
- Snapshot a Vector KB.

Si fallo: bloquear + sugerir patch.

## 5. Firmas de Skills

Skills cerradas incluyen manifest con:
```yaml
skill: my-closed-skill
version: 1.0.0
signer: vendor-x
signature: <ed25519 detached>
public_key: <embedded>
```
El sistema valida la firma antes de carga. Sin firma OK → `Forbidden`.

## 6. Sandbox levels

- `none` → solo núcleo interno.
- `vuOnly` → FS virtual, sin creds, red restringida.
- `container` → Docker/Podman con red `none` salvo allowlist.
- `wasm` → sandbox Wasm (futuro).

## 7. Approvals workflow

Toda acción sensible entra a la cola:
```
Approvals [
  { who:agentX, action:"npm install lodash@4.17.21", risk:low, sandbox:container }
]
```
El usuario aprueba desde UI o web. Approvals `/approve all` batch para tareas largas.

## 8. Auditoría

Journal incluye:
- actor (agente/usuario/modelo),
- acción,
- código de retorno,
- diff,
- chain-of-thought del reasoning que la generó,
- result.

Logs exportables: JSONL y/o compatible OTel.

## 9. Privacidad por defecto

- Vector KB local, encrypted-at-rest (libsodium / SQLCipher).
- Modo mixto nunca envía código del usuario a embeddings cloud sin permiso.
- Modo local-only nunca toca red saliente excepto allowlist permanente de sources docs públicas.

## 10. Anti-prompt-injection

Si una skill/MCP/devuelve texto con instrucciones tipo "ignore previous instructions", el Reasoning Engine lo detecta y lo marca como prompt-injection attempt.

## 11. Protección anti-alucinación

- Socket: evita paquetes inventados.
- Prisma/SQLAlchemy/etc.: evita columnas inventadas.
- tRPC/ts-rest: evita endpoints inventados.
- Knip/dead-code: evita código muerto.
- Biome/Ruff/clippy: evita malas prácticas.
- MSW: evita asumir que la API responde perfecto.
- Tree-sitter: el agente entiende AST, no solo texto plano.
- Semgrep/CodeQL: reglas custom profundas.

Todo eso pasa por un filter chain obligatorio.

## 12. Compliance

- PR targeting change of secrets/deps/security → normalmente requiere review de humano.
- En código de usuario crítico (HIPAA/GDPR/PCI) se aplican skills específicas (reglas OWASP, GDPR skill, etc.).

## 13. Telemetría de seguridad

Dashboard con: top call attempts, CPU/RAM anomalies, blocked actions, MCP sanc-cli violations, reputation changes de proveedores.
