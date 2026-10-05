# Domain pack — `coding`

> Operator guide for the default software-engineering domain pack (RFC 64).

## What it is

The default pack. It does not specialise the core; it declares the routing the
generic engines already provide for source projects.

## Manifest (`src-tauri/src/domain/packs/coding.toml`)

| Field                | Value                                                                            |
| -------------------- | -------------------------------------------------------------------------------- |
| `detect`             | `Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`, `*.rs`, `*.ts`, `*.py` |
| `engines.coding`     | `default` (LSP + tests + Biome/tsc)                                              |
| `engines.validation` | `tests.pass`, `lint.clean`                                                       |
| `skills.bundled`     | `prompt-clarify`, `opencode-test`                                                |
| `tools.lateral.open` | — (no external binary)                                                           |
| `artifacts.types`    | `diff`, `test_report`                                                            |
| `policy.sandbox`     | `local`                                                                          |

## Usage

```bash
atlas domain list
atlas domain use coding
```

Project Map matching is automatic; no activation call is required.
