# Domain pack — `creative-media`

> Operator guide for the 3D / asset / render domain pack (RFC 64).

## What it is

Routes 3D asset work (Blender / glTF / FBX) through the creative skill set.
Blender is a lateral DCC tool; Mixar (agent-in-Blender) is the closest
reference for this pattern (RFC 64 §1).

## Manifest (`src-tauri/src/domain/packs/creative-media.toml`)

| Field | Value |
|---|---|
| `detect` | `*.blend`, `*.fbx`, `*.gltf`, `*.glb`, `*.obj`, `blender` |
| `engines.validation` | `asset.renders`, `export.gltf` |
| `skills.bundled` | `creative.img-to-threejs`, `creative.asset-review` |
| `mcp.servers` | `blender-mcp` |
| `tools.lateral.open` | `blender` |
| `artifacts.types` | `blend`, `gltf`, `render.png`, `preview.mp4` |
| `policy.sandbox` | `container` |

## Lateral tools

```bash
atlas domain probe creative-media
atlas domain guide creative-media blender
```
