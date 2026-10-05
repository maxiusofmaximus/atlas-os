# 07 - MCP

Model Context Protocol — soporte nativo, pero **seguro**. La crítica pública contra OpenClaw es que ejecuta MCPs sin sandbox ni verificación. Atlas OS resuelve eso.

---

## 1. Soporte MCP

Atlas OS implementa el protocolo MCP estándar (Anthropic / comunidad). Cualquier MCP server compatible puede configurarse como tool provider.

### Registro
```jsonc
{
  "mcpServers": {
    "context7": {
      "command": "npx",
      "args": ["-y", "@upstash/context7"],
      "trusted": true,
      "sandbox": "vuOnly",
      "allowed_tools": ["get_library_docs", "search_docs"]
    },
    "playwright": {
      "command": "npx",
      "args": ["-y", "@playwright/mcp"],
      "sandbox": "container",
      "allowed_tools": ["navigate", "click", "snapshot"]
    }
  }
}
```

Cada servidor declara:
- **trusted** (¿firma verificada?),
- **sandbox** (¿en qué aislamiento se ejecuta?),
- **allowed_tools** (lista blanca de herramientas expuestas).

## 2. Sandboxing

Tres modelos de aislamiento:

### `none`
Solo para servidores del propio sistema operativo del editor (núcleo). Uso interno.

### `vuOnly` (Virtualized User)
Proceso en el mismo host pero con filesystem virtual (capa FUSE / WASM / WSL en Windows). Sin acceso a credenciales ni red saliente salvo explicit allowlist.

### `container`
Contenedor Docker/Podman aislado, red restringida, mounts readonly.

### `wasm` (futuro)
Servidor MCP empaquetado en WASM, sin sistema de archivos salvo FS virtual explícitamente otorgada.

La política decide el sandbox automáticamente:
- sin firma → `container` o `wasm`.
- firma conocida → `vuOnly`.
- núcleo → `none`.

## 3. Verificación de paquetes (cadena de suministro)

Antes de instalar un MCP nuevo, el sistema:
- lo descarga desde el repositorio declarado;
- valida el checksum publicado;
- comprueba Socket supply-chain telemetry del paquete;
- registra el hash en el Journal.

Si el hash no coincide → bloqueado.

## 4. allowlist de tools

Cada MCP expone tools; Atlas OS no las habilita por defecto. Solo las listadas en `allowed_tools` se hacen accesibles al agente. Esto evita que un MCP malicioso exponga, por ejemplo, `fs.delete` o `shell.exec` sin que el usuario lo consienta.

## 5. Rutas críticas

- **Context7**:_SUPPLY de documentación oficial → alimentar Research Engine (prioridad alta).
- **Playwright (snapshot compacto)**: para Validation Engine E2E; preferimos fork estilo Playwright CLI con IDs `[E21]`, `[E26]` que reduce 70% de tokens.
- **Filesystem local**: restringido a la carpeta del workspace abierto.
- **Memory / Vector store**: interno, no se expone a MCP externo.
- **GitHub MCP**: para issues, PRs, Discussions (alimentar Research / CI).

## 6. Asociación de MCPs a Skills y motores

Las tools expuestas por un MCP se indexan en el Skill Graph como skills `auto_generated: true`, `verified: true`. Se asocian al motor por su dominio (Investigación, Validación, Edición).

## 7. Política de ejecución

| Tipo de tool | Aprobación por defecto |
|---|---|
| Lectura | Auto |
| Búsqueda de docs | Auto |
| Navegación web en sandbox | Auto |
| Escritura en filesystem workspace | Confirm |
| Escritura fuera del workspace | Forbidden |
| Ejecución de shell | Confirm + sandbox container |
| Red saliente libre | Forbidden |
| Acceso a secrets | Forbidden |

Los niveles los define el usuario y `18 - Security.md`.

## 8. Telemetría

Cada invocación MCP registra:
- servidor, tool, argumentos, respuesta, latencia,
- sandbox usado,
- resultado (success/fail),
- `confidence_delta` que aportó al razonamiento.

## 9. Rotación de tools

Si una tool MCP repite fallos o baja el Confidence, el Kernel la desactiva temporalmente y avisa al Learning Engine para que ajuste su `priority`.

---

## 10. Fase 29 — estado de implementación (MCP runtime)

**Status: ✅ Fase 29.0 (client core) implementado.** El stub de 23 líneas (`cli/commands/mcp.rs`) se reemplaza por un runtime real. Sin crates nuevas (`tokio::process` + `serde_json` + `thiserror`, ya en el árbol).

Módulo `src-tauri/src/mcp/`:

- **`config.rs`** — registry dual-shape: el formato opencode-compatible (`.opencode/mcp.json`, el que el HUD ya lee) y el formato RFC 07 (`mcp.json`, `mcpServers`). `command` acepta string (RFC 07) o argv (opencode); `allowed_tools` se preserva (§4). Precedencia perfil → repo; fichero ausente = "sin servidores", nunca una lista inventada.
- **`protocol.rs`** — JSON-RPC 2.0 (request/response/notification) + `initialize`/`tools/list`/`tools/call` (payloads puros, testables sin servidor).
- **`client.rs`** — transporte stdio (spawn + `kill_on_drop`) con timeout por petición; el bucle ignora notificaciones y respuestas de otros ids; **en Windows los shims `npx`/`pnpm` (`.cmd`) se enrutan por `cmd /C`** (CreateProcess no ejecuta `.cmd` directo). `tools_call_checked` hace cumplir `allowed_tools` **antes** de enviar bytes.

CLI: `atlas mcp list | add | remove | probe | call` (reemplaza el stub).

**Verificación (medida):**
- 35 tests unitarios (`mcp::`) con transporte en memoria + timeout + allowlist; `cargo clippy --all-targets -- -D warnings` limpio.
- **End-to-end en vivo**: `atlas mcp probe context7` hizo spawn real de `pnpm dlx @upstash/context7-mcp@3.2.4`, handshake y `tools/list` (2 tools); `atlas mcp call context7 resolve-library-id --args '{"libraryName":"react","query":"hooks"}'` devolvió datos reales de Context7. La allowlist rechazó `query-docs` (no listada) sin tocar la red.

**Pendiente (Fase 29.1+):** enforcement de sandbox (§2), verificación de cadena de suministro (§3), puente al `ToolRegistry` de RFC 63 (las tools MCP como tools del agente), telemetría (§8) y rotación (§9). El endpoint `GET /hud/mcp` (RFC 65 §10) sigue siendo la lectura honesta del catálogo.
