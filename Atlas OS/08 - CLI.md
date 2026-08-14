# 08 - CLI

Soporte de **CLIs externas** como tools de agente, no como simples comandos shell. Mismo modelo de seguridad que MCPs, mismas reglas de Skill Graph.

---

## 1. Ejemplos de CLIs integrados por defecto

| CLI | Uso | Motor asociado |
|---|---|---|
| `biome` | lint + format | Validation |
| `knip` | dead code | Validation |
| `playwright` | E2E | Validation |
| `tsc --noEmit` | type check | Validation |
| `lefthook` | git hooks | Validation |
| `snyk` | audit deps | Validation / Security |
| `socket` | supply chain | Security |
| `pulumi` / `tofu` | IaC apply | Coding (IaC sub-role) |
| `docker` | Testcontainers | Validation |
| `turbopack`/`vite` | build | Validation |
| `tree-sitter` | AST parsing | Context Engine |
| `semgrep` | reglas custom | Validation / Security |
| `codeql` | deep vuln | Security |
| `dependency-cruiser` | límites de módulo | Validation |
| `gh` | GitHub (issues, PR) | Research / CI |
| `git` | versionado | todas |

Cualquier CLI del usuario puede registrarse como tool.

## 2. Registro

```yaml
cli:
  name: biome
  command: ["biome", "check", "--apply"]
  cwd_mode: workspace
  sandbox: vuOnly
  allowed_args_regex:
    - "^--"
    - "^src/.*\\.tsx?$"
  blocked_args_regex:
    - "\\|\\|"
    - ";"
    - "&"
  env_vars_pass: []
  env_vars_block: ["AWS_*", "OPENAI_API_KEY", "*_TOKEN"]
  timeout_ms: 30000
  approve: auto   # auto | confirm | forbidden
```

Argumentos peligrosos (`&&`, `||`, `;`) se bloquean por defecto para evitar shell injection desde un agente.

## 3. Modo compacto de salida

El orchestrator prioriza CLIs que devuelven **salida estructurada / compacta**. Por eso elegimos:
- **Playwright con `[E21]`** en vez de volcar el DOM.
- **Biome JSON** en vez de logs superficiales.
- **Tree-sitter AST JSON** en vez de texto.

Cualquier CLI con mas de ~1000 tokens de salida se **resume** automáticamente con un modelo barato antes de pasar al agente principal. Esto evita saturar la ventana.

## 4. Reglas anti-alucinación

- El agente no puede ejecutar un CLI que no esté registrado.
- Cualquier comando sensible (network, fs write, shell) requiere approval rule.
- Tokens de argumentos se validan contra `allowed_args_regex`.

## 5. API del CLI engine

```
CLIEngine.run(command_id, args) -> { stdout, stderr, code, parsed, tokens }
CLIEngine.suggest(command_id, args) -> { safe: boolean, reason: string }
CLIEngine.register(manifest) -> id
CLIEngine.tear_down(id)
```

Toda ejecución queda en el Journal.

## 6. Profiling de costes

El CLI engine computa tiempo total consumido por CLI en una mission. Esto permite que el Learning Engine sugiera alternativas más rápidas (p. ej. `biome` en vez de `eslint+prettier`).
