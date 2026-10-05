# Domain pack — `mobile`

> Operator guide for the Android / mobile domain pack (RFC 64; RFC 38 artemis).

## What it is

Routes Android work through the generic engines and the mobile skill set.
`artemis` (RFC 38) is a **lateral** Python/uv tool, never bundled.

## Manifest (`src-tauri/src/domain/packs/mobile.toml`)

| Field                | Value                                                                                         |
| -------------------- | --------------------------------------------------------------------------------------------- |
| `detect`             | `AndroidManifest.xml`, `build.gradle`, `build.gradle.kts`, `*.kt`, `*.apk`, `tauri.conf.json` |
| `engines.validation` | `gradle.builds`, `apk.signed`                                                                 |
| `skills.bundled`     | `mobile.reproduce-bug`, `mobile.write-tests`                                                  |
| `tools.lateral.open` | `artemis`                                                                                     |
| `artifacts.types`    | `apk`, `screenshot`, `logcat`                                                                 |
| `policy.sandbox`     | `container`                                                                                   |

## Lateral tools

```bash
atlas domain probe mobile
atlas domain guide mobile artemis
```
