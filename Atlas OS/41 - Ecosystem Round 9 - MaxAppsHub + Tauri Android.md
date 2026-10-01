# 41 - Ecosystem Round 9 Audit — MaxAppsHub + Tauri Android companion

Auditoría del launcher Android del operador (MaxAppsHub) + el build Android de Atlas OS (Tauri 2). Verificada vía GitHub API + toolchain local (Sep 2026). Extiende RFC 38 (artemis) y la Fase 11 (mobile testing).

## 1. Contexto y motivación

El operador quiere **probar la app Atlas en Android**: "primero debe estar en github y que esté en un launcher que actualice la app como en este repo mio: maxiusofmaximus/MaxAppsHub". El flujo completo: Atlas OS APK (Tauri Android) → GitHub Releases → MaxAppsHub instala/actualiza → artemis automatiza el testing.

## 2. maxiusofmaximus/MaxAppsHub — launcher Android (VERIFICADO, proyecto propio del operador)

`github.com/maxiusofmaximus/MaxAppsHub` (GitHub API: Kotlin, 8 commits, default branch `main`, actualizado 2026-05-03; Gradle + Jetpack Compose):

*Launcher de Android diseñado para simplificar la gestión y actualización de tus aplicaciones APK alojadas en GitHub.*

### 2.1 Cómo gestiona apps (`data/AppRegistry.kt`)

```kotlin
data class ManagedApp(
    val name: String,           // nombre visible
    val packageId: String,      // applicationId Android (com.flashcards.quimica)
    val githubOwner: String,    // maxiusofmaximus
    val githubRepo: String,     // repo con los Releases APK
    val icon: String,
    val description: String,
    val accentColorHex: Long    // color de acento (0xFF6366F1)
)
```

Apps gestionadas hoy (Fase 14): "Flashcards Química" (`com.flashcards.quimica`) + "Atlas OS" (`com.opencode_os.app`, repo `atlas-os`, commit MaxAppsHub `b131415`). **Nota física validada**: el applicationId Android real es `com.opencode_os.app` (guion bajo — Tauri sustituye el guion del identifier al generar el package; Android no admite guiones) y el `<queries>` del manifest debe incluirlo — sin ambos, el launcher muestra "No instalada" (`NameNotFoundException` por filtrado de visibilidad Android 11+). El repo `atlas-os` es **público** (necesario: el launcher consulta `releases/latest` sin token; privado = HTTP 404).

### 2.2 Estados + mecanismo

- **Actualizaciones Automáticas**: conecta con la API de GitHub (Releases del repo de la app) para verificar nuevas versiones.
- Estados: No instalada (descarga+instala) / Actualizada (abre) / Desactualizada (botón "Actualizar").
- Descarga via gestor de descargas de Android + instalador del sistema + guías de permisos (apps desconocidas).
- v1.0.2: **APK firmado digitalmente** (fix "Paquete no válido"), `<queries>` para visibilidad Android 11+, `User-Agent` en las peticiones.

## 3. Toolchain Android verificado (Sep 2026)

| Componente | Estado |
|---|---|
| **JDK 17** | ✅ `C:\Program Files\Eclipse Adoptium\jdk-17.0.20.101-hotspot` (requiere `JAVA_HOME` set para Gradle — no está en PATH) |
| **Android SDK** | ✅ `C:\Users\Max\AppData\Local\Android\Sdk` — platforms `android-36` + NDK `29.0.14206865` + build-tools `34/36` |
| **Rust targets** | ✅ `aarch64-linux-android` + `armv7-linux-androideabi` + `x86_64-linux-android` instalados |
| **ADB** | ✅ `platform-tools` (dispositivo TECNO KI7 conectado por depuración inalámbrica — RFC 38 §3 validación) |
| **artemis** | ✅ clonado `C:\Users\Max\artemis` + `uv` en PATH (vía hermes) |

## 4. El flujo completo (Fase 14 — mobile companion + MaxAppsHub)

```
Atlas OS (Tauri 2 + SvelteKit CSR)
   └─ tauri android build --apk  →  APK firmado
        └─ GitHub Release (repo atlas-os)  →  APK en Releases
             └─ MaxAppsHub (AppRegistry entry)  →  instala/actualiza en el Android
                  └─ artemis (`atlas mobile run`)  →  automatiza el testing de la app
```

- **Sin apps Kotlin nuevas**: Tauri 2 soporta Android nativamente — el mismo HUD SvelteKit CSR se empaqueta como APK.
- **applicationId**: `com.opencode-os.app` (identifier `tauri.conf.json`, se mantiene) → package Android real `com.opencode_os.app` (Tauri sustituye guion por guion bajo). MaxAppsHub lo detecta con ese packageId + entrada `<queries>`.
- **Firma**: debug keystore para el MVP (el launcher instala apps de fuentes desconocidas con guía); release keystore cuando pase a producción.
- **artemis testing**: la app Atlas Android es una app Android normal — artemis la prueba (`atlas mobile run --task "abre Atlas OS y verifica el HUD"`).

## 5. Priorización por dependencia

```
tauri android init + build (14.0) ─► APK firmado
   └─ gh release (14.1) ─► APK en GitHub Releases
        └─ MaxAppsHub AppRegistry entry (14.2) ─► launcher gestiona la app
             └─ validación física: launcher + artemis testing
```

**Orden recomendado:**
1. **14.0 Tauri Android build** (M46): `tauri android init` + applicationId + build APK (JAVA_HOME + ANDROID_HOME).
2. **14.1 GitHub Release** (M47): `gh release create` con el APK.
3. **14.2 MaxAppsHub entry** (M48): `ManagedApp` en `AppRegistry.kt` (repo MaxAppsHub — mismas credenciales GH del operador) + validación física end-to-end.

## 6. Restricciones y boundary rules

- **Tauri Android build**: NO añade crates (el gen/android es generado; el applicationId es config). El primer build puede ser lento (NDK + Gradle daemon).
- **MaxAppsHub es repo del operador** — el cambio en `AppRegistry.kt` se hace con las mismas credenciales GH (owner maxiusofmaximus); si el push falla, se entrega el diff al operador.
- **Keystore**: debug keystore para el MVP; jamás commitear keystores de release al repo (el debug.keystore de MaxAppsHub ya está en su repo — es del launcher, no de Atlas).
- La app Atlas Android usa el MISMO Journal/perfil del desktop vía el HUD (RFC 24 — remote accesible); el dispositivo necesita acceso al host (LAN/wireless).

## 7. Status de este RFC

- **Versión:** 1.2 (cierre Fase 14 + fixes de detección, Oct 2026).
- **Tipo:** Informativo + plan de Fase 14 (research/40 v2 extension) — **Fase 14 COMPLETA y VALIDADA físicamente**.
- **Método:** GitHub API (`MaxAppsHub` + `AppRegistry.kt` raw) + toolchain local verificado (JDK/SDK/NDK/rustup/ADB) + build Android real (Tauri 2 CLI 2.12, HOME fix, git2 muerto eliminado) + instalación inalámbrica + artemis testing end-to-end.
- **Cierre (Oct 2026)**: 2 defectos de detección corregidos y validados en el TECNO KI7 — (1) packageId `com.opencode_os.app` + `<queries>` en `AppRegistry.kt`/manifest de MaxAppsHub (commit `12cd65a`: sin ambos, "No instalada" por NameNotFoundException/visibilidad), (2) repo `atlas-os` hecho público (sin token el launcher obtenía HTTP 404 de `releases/latest`). Tras los fixes: "Instalada 0.1.0" + "Última v0.1.0-android" sin error, sin crashes. artemis completó "Abre la app Atlas OS" (3 steps, ~106s) — los fallos previos (~340s sin steps) eran el serial mDNS con espacios/paréntesis rompiendo el device discovery de artemis; con serial convencional `host:port` (`adb connect`) funciona. Pendiente release-side: tag `v0.1.0-android` ≠ versionName `0.1.0` → el launcher muestra "Actualizar" aunque el APK instalado sea el del release (requiere nueva release o tag `v0.1.0` — Phase 15).

## 8. Fuentes de auditoría

- `https://github.com/maxiusofmaximus/MaxAppsHub` — README + AppRegistry.kt (verificación)
- `https://api.github.com/repos/maxiusofmaximus/MaxAppsHub/git/trees/main?recursive=1` — estructura del repo
- Toolchain local: `rustup target list --installed`, Android SDK dirs, Eclipse Adoptium JDK 17
