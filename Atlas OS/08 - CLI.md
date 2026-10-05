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
| `terminal-browser` | browser pane (lateral, kitty graphics — RFC 28 §I) | Coding / Context |
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

## 7. Familia `atlas browser` (terminal-browser lateral, RFC 28 §I)

Shell-out (nunca bundling; ver RFC 25 §11). Sin el binario en `PATH`, `open`/`ls`/`action` terminan con exit != 0 y la receta de install (nunca un no-op silencioso):

```bash
atlas browser probe                       # ¿instalado? versión + ¿terminal con kitty graphics?
atlas browser open https://example.com    # delega en `terminal-browser open`
atlas browser open ./plan.html --split right
atlas browser ls                          # delega en `terminal-browser ls`
atlas browser action -- navigate https://example.com   # passthrough agent-browser (contrato NO pinneado)
```

`open` publica `artifact_preview_opened` en el Journal (RFC 28 §I item 6). La capacidad web del agente se clasifica como `SensitiveAction` (RFC 18, opt-in). El binario es `terminal-browser` (MIT) o `$ATLAS_TERMINAL_BROWSER_BIN`.

## 8. Familia `atlas mcp` (runtime MCP, RFC 07)

Runtime MCP real (Fase 29.0): registry dual-shape (`.opencode/mcp.json` opencode + `mcp.json` RFC 07) y protocolo stdio JSON-RPC. `probe`/`call` spawnean el servidor; `call` aplica `allowed_tools` (RFC 07 §4) **antes** de enviar bytes.

```bash
atlas mcp list                                   # servidores configurados (sólo lectura)
atlas mcp add context7 --command pnpm --arg dlx --arg @upstash/context7-mcp@3.2.4 --allow resolve-library-id
atlas mcp probe context7                         # spawn + handshake + tools/list
atlas mcp call context7 resolve-library-id --args '{"libraryName":"react","query":"hooks"}'
atlas mcp remove context7
```

En Windows los shims `npx`/`pnpm` (`.cmd`) se enrutan por `cmd /C` (CreateProcess no ejecuta `.cmd` directo). Sandbox/supply-chain/ToolRegistry bridge: Fase 29.1+.

## 9. Familia `atlas serve` (daemon headless, RFC 29 §3.A)

Arranca el Kernel Bus + HUD axum server **sin webview**, para que el runtime sobreviva al cierre del desktop (postura "AI Employee"). Bind loopback por defecto; expón sólo tras túnel autenticado (SSH / Tailscale / Cloudflare).

```bash
atlas serve                              # 127.0.0.1:<puerto efímero>
atlas serve --host 127.0.0.1 --port 8787
atlas hud                                # imprime la URL del daemon activo
```

Requiere la feature `hud` (default on). Un bind no-loopback imprime un aviso (RFC 18) y exige el bearer remoto (`atlas hud --rotate-token`).
