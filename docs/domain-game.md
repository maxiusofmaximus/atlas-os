# Domain pack — `game`

> Operator guide for the video-games domain pack (RFC 64).

## What it is

Routes asset/scene work through the generic engines with asset-level
validation. Engines (Godot/Bevy/Unity) are lateral.

## Manifest (`src-tauri/src/domain/packs/game.toml`)

| Field                | Value                                                                                |
| -------------------- | ------------------------------------------------------------------------------------ |
| `detect`             | `project.godot`, `*.tscn`, `*.gdscript`, `Assets/`, `ProjectSettings/`, `*.uproject` |
| `engines.validation` | `assets.valid`, `scene.parses`                                                       |
| `skills.bundled`     | `game.scene-review`, `game.asset-check`                                              |
| `tools.lateral.open` | `godot`                                                                              |
| `artifacts.types`    | `scene`, `asset`, `build`                                                            |
| `policy.sandbox`     | `container`                                                                          |

## Lateral tools

```bash
atlas domain probe game
atlas domain open game godot -- --version
```
