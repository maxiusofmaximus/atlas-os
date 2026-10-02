# PHASE 15 RELEASE & DISTRIBUTION HARDENING REPORT

## Versioning

- Source of truth: `src-tauri/Cargo.toml` (`version = "0.1.1"`); `package.json` 0.1.1; `tauri.conf.json` sin versión duplicada (`"version": "../package.json"`)
- Previous mismatch: tag `v0.1.0-android` vs versionName `0.1.0` → launcher mostraba "Actualizar" espurio (workflow comparisons: `tagName.removePrefix("v")` vs versionName)
- New version: 0.1.1
- Android versionName: 0.1.1
- Android versionCode: 1001
- Git tag: `v0.1.1` (parejo con versionName)

## Tests

- cargo test: 1103 ok (CI gate verde)
- vitest: 67 ok (CI)
- cargo fmt: check (CI)
- clippy: -D warnings (CI)
- cargo check: ok (CI)
- pnpm check: ok
- pnpm lint: ok (salvo archivos preexistentes documentados)
- Other: gate versionName == tag == 0.1.1 PASS; gate cert keystore == cert APK == cf091fd2 PASS

## Release

- GitHub release: `v0.1.1` — "Atlas OS 0.1.1 — Android (Tauri 2 companion)", Latest, published 2026-10-02T06:48:22Z
- APK: `app-universal-debug.apk`
- APK size: ~578 MB (universal debug)
- SHA-256: asset `.SHA256` coincide con el APK publicado
- CI run: run #9 re-run success (12m22s)
- Version gate: PASS

## MaxAppsHub

- Build: v1.0.2
- Installed: 0.1.0 → 0.1.1 (tras update físico)
- Latest release detected: v0.1.1 (GitHub API Releases)
- Installed version: 0.1.1
- Detected version: v0.1.1
- UI state: `Última v0.1.1` / `Instalada 0.1.1` / botón "Abrir" (estado Actualizada) ✅
- Update flow: detección 0.1.0≠0.1.1 → "Actualizar" → descarga → permiso "Install unknown apps" guiado → instalador del sistema "¿Deseas actualizar esta app?" → "Se instaló la app." → estado Actualizada ✅

## TECNO KI7

- Launcher: MaxAppsHub 1.0.2, emparejado vía depuración inalámbrica (192.168.50.95:34019)
- Atlas: `com.opencode_os.app`, versionName 0.1.1
- Update: físico vía launcher, firma consistente (mismo cert cf091fd2 → UPDATE sin fallo)
- Atlas launch: HUD Mission Control visible
- HUD: WS status connected, URL ws://localhost:35677/ws
- logcat: sin FATAL EXCEPTION de `com.opencode_os.app` (los crashes vistos eran de otro paquete, wutheringwaves)
- Artemis: smoke PASS — task "describe the Atlas OS screen" completada con éxito

## Changes

- `.github/workflows/release-android.yml` — CI completo (tests, version gate, cert gate, SHA256, release)
- `src-tauri/Cargo.toml` / `package.json` / `tauri.conf.json` — versión 0.1.1 coherente
- `vite.config.ts` — `VITE_OC_VERSION` inyectado desde `package.json` (corrige el fallback stale `0.1.0` del header del HUD)
- README.md — Phase 15 documentada

## Commits

- ci(release): restore debug keystore to xdg config path too (`6526d85`)
- ci(release): locate signing keystore source on runner (`2e4da33`)
- fix(tests): isolate env in explicit_override_wins_over_path (`13b8997`)
- ci(release): diagnose keystore restore + gate APK signer cert (`9662f50`)
- ci(release): fix cert extraction + trace keystore (`0a1f2da`)
- fix(tests): platform-agnostic path assertion in retention export (`7571e6f`)
- ci(release): install tauri linux system deps (`95a34fd`)
- fix(release): regenerate stale pnpm-lock.yaml (`f841447`)
- ci(release): automate android release validation (`64063d6`)
- Pendiente de commit: `vite.config.ts` + README.md (cambios locales de cierre)

## Pre-existing issues

- Header del HUD mostraba `v0.1.0` con APK 0.1.1 (fallback `VITE_OC_VERSION`) — corregido en `vite.config.ts`, toma efecto en el próximo build
- `.dependency-cruiser.cjs` y `siguiente_paso.md` preexistentes (no tocar)

## Remaining blockers

- Phase 13: BLOQUEADA UPSTREAM (laya > 0.2.x o maintainers ≥ 2) — NO tocar
- Other: ninguno identificado para Phase 15

## FINAL STATUS

CLOSED
