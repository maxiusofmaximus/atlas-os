# Domain pack — `media-gen`

> Operator guide for the media-generation domain pack (RFC 64).

## What it is

Routes video/image/audio generation. Provider calls go through the existing
facade (RFC 04); `ffmpeg` is the lateral local tool for probing/remuxing. No
provider SDK is bundled.

## Manifest (`src-tauri/src/domain/packs/media-gen.toml`)

| Field | Value |
|---|---|
| `detect` | `*.mp4`, `*.mov`, `*.webm`, `generated/`, `media/`, `*.png`, `*.jpg` |
| `engines.validation` | `media.exists`, `codec.playable` |
| `skills.bundled` | `media.prompt-to-clip`, `media.asset-review` |
| `tools.lateral.open` | `ffmpeg` |
| `artifacts.types` | `mp4`, `png`, `wav`, `preview.mp4` |
| `policy.sandbox` | `local` (network egress is `Confirm`, RFC 18) |

## Lateral tools

```bash
atlas domain probe media-gen
atlas domain guide media-gen ffmpeg
```
