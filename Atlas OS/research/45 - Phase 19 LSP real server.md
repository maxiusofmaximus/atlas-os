# 45 - Phase 19: LSP real server (v1 mínimo)

## Objetivo

Eliminar el stub del LSP host (213 LOC, sólo sleep) y sustituirlo por un servidor
real mínimo que los editores ACP/LSP puedan consumir para probar la presencia del
backend de Atlas OS.

## Implementación

- Nuevo `lsp/server.rs` con `AtlasLspBackend: tower_lsp::LanguageServer`:
  - `initialize` devuelve `ServerCapabilities` con hover provider simple,
    `TextDocumentSyncKind::FULL`, y `server_info` (name "atlas-lsp", version CARGO_PKG_VERSION).
  - `initialized` publica log via client info logger.
  - `shutdown` ok.
  - `hover` devuelve una markdown estable (`Atlas OS LSP host — version X…`).
  - `did_open` publica diagnostics vacías (no rompe a los clientes que esperan publish).
- `lsp::host::serve` queda como entry: si `ATLAS_LSP_STDIO` está definido o stdout
  no es terminal, monta `serve_stdio` (stdin/stdout JSON-RPC MCP); si no, "park"
  sin bloquear el webview (comportamiento desktop preservado).

## Verificación

- `cargo test --lib lsp` → **9 passed; 0 failed** (incluye `initialize_advertises_hover_and_full_sync`
  y `hover_payload_carries_version`).
- `cargo check` default OK.
- `cargo clippy --all-targets -- -D warnings`: OK.
- `cargo fmt --check`: OK.

## Regresión

- La trait `LanguageServer` estaba implementada para `Backend` del tower-lsp example,
  no en `atlas-os`. Stub park preservado para el desktop; comportamiento de
  `lib.rs`/`main.rs` sin cambios.

## Fuera de alcance

- Proxy de lsp multiplexer (los language servers reales dependen de la config del operador).
- Diagnostics cargadas desde `journal::AstSymbolRow`: surface existe, se imputa en
  Phase 20 junto al refactor del módulo gigante.
