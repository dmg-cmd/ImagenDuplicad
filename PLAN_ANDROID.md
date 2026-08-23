# Plan de migración a Android — Estado actual

> Documento de trabajo para retomar la adaptación móvil desde cualquier máquina.
> Última actualización: al cerrar la versión 1.0.0 de escritorio.

## Objetivo

Compilar **Imagen Duplicada** para Android (APK directo / sideload), con funcionalidad
esencial: escanear galería/carpetas → ver grupos → conservar la mejor → **borrar duplicados**.

Distribución elegida: **APK directo** (no Play Store), lo que permite pedir permiso
`MANAGE_EXTERNAL_STORAGE` sin restricciones.

Alcance v1 Android (**esencial primero**): escaneo, grupos exactos/probables, borrado,
historial CSV, tema oscuro. Fuera de alcance por ahora: comparador táctil completo,
excluir carpetas, mover conservadas, vista por carpetas.

---

## Estado del trabajo

### ✅ Hecho

1. **Proyecto base compatible con Tauri mobile**: `crate-type` ya incluye
   `staticlib` y `cdylib` en `src-tauri/Cargo.toml`.
2. **`cargo tauri android init` ejecutado con éxito** — generó `src-tauri/gen/android`
   (carpeta gitignoreada; se regenera con el comando).
3. **Crate `trash` condicionado** — no compila para Android. Cambios ya commiteados:
   - `src-tauri/Cargo.toml`: `trash = "5"` movido a
     `[target.'cfg(not(target_os = "android"))'.dependencies]`
   - `src-tauri/src/commands.rs`: nueva función `eliminar_archivo()` con dos versiones:
     - Desktop (`#[cfg(not(target_os = "android"))]`): usa `trash::delete` como antes.
     - Android (`#[cfg(target_os = "android")]`): "papelera" = carpeta
       `papelera/` dentro del directorio de datos de la app, renombrando con
       sufijo `(2)`, `(3)`... ante colisiones; fallback copiar+borrar entre
       sistemas de archivos.
4. Escritorio sigue compilando y los 13 tests pasan después de estos cambios.

### ❌ Problema pendiente: error 134 en Gradle

- El build Rust para `aarch64-linux-android` **compila bien**.
- Al armar el APK, Gradle ejecuta la tarea `rustBuildArm64Release`, que llama a
  `npm run -- tauri android android-studio-script`; ese proceso muere con
  **código 134 (SIGABRT)**.
- Causas probables (en orden):
  1. **Falta de RAM** durante el build release: `[profile.release]` usa
     `lto = true` + `opt-level = "s"` (muy pesado).
  2. Problema del CLI de Tauri invocado desde Gradle.
- Mitigación propuesta si vuelve a pasar:
  - Probar `cargo build --target aarch64-linux-android --lib --release` manual
    para aislar si falla cargo solo.
  - Desactivar LTO (globalmente o probar `lto = false` / `thin-lto`) y reintentar.
  - Compilar una sola arquitectura (`--target aarch64`) en vez de las 4.

---

## Toolchain instalado en la máquina Linux

| Componente | Ubicación |
|---|---|
| NDK r27c | `~/Android/Sdk/ndk/android-ndk-r27c` |
| SDK Android | `~/Android/Sdk` (build-tools 34–37, plataformas 33–36.1) |
| Targets Rust Android | instalados vía rustup |
| JDK 17 | `/usr/lib/jvm/java-17-openjdk-amd64` |
| Variables de entorno | al final de `~/.bashrc`: `ANDROID_HOME`, `NDK_HOME`, `JAVA_HOME` |

Comando de build usado:

```bash
export ANDROID_HOME="$HOME/Android/Sdk"
export NDK_HOME="$ANDROID_HOME/ndk/android-ndk-r27c"
export JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64
npx tauri android build --target aarch64
```

---

## Pasos pendientes (orden sugerido)

### Fase A — Lograr el APK
1. Resolver el error 134 (ver arriba). Opción recomendada: **workflow de GitHub
   Actions** que compile el APK en la nube (máquinas con más RAM, nada que
   instalar localmente). Ver sección Workflow abajo.
2. Si se hace local (Windows): instalar JDK 17+, Android Studio (trae SDK+NDK),
   rustup targets Android y definir `ANDROID_HOME`, `NDK_HOME`, `JAVA_HOME`.

### Fase B — Backend adaptado
3. Miniaturas: reemplazar `std::env::temp_dir()` en
   `src-tauri/src/thumbnails.rs::cache_dir()` por el directorio de caché de la app
   cuando se corra en Android (requiere pasar `AppHandle` o usar una ruta fija
   tipo `/data/data/com.imagenduplicada.app/cache`).
4. `abrir_historial` (`commands.rs`): los comandos `xdg-open`/`explorer`/`open`
   no existen en Android. Ocultar el botón "📄 Historial" en móvil
   (detectar plataforma en el frontend con
   `navigator.userAgent` o compilar con flag). El CSV igual se genera en datos
   de la app.
5. Comandos nuevos: `tiene_acceso_archivos()` (verifica si puede leer
   `/storage/emulated/0/DCIM`) y abrir ajustes de Android para conceder el
   permiso "Todos los archivos".

### Fase C — Permisos y acceso a la galería
6. En `src-tauri/gen/android/app/src/main/AndroidManifest.xml` agregar:
   ```xml
   <uses-permission android:name="android.permission.READ_MEDIA_IMAGES" />
   <uses-permission android:name="android.permission.MANAGE_EXTERNAL_STORAGE" />
   ```
   Nota: `gen/` se regenera con `tauri android init`; si se regenera, volver a
   agregar los permisos (o committear `gen/` completo).
7. UI de primer uso: botón *"Conceder acceso a tus imágenes"* que abre
   Ajustes → Todos los archivos (intent `ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION`).
8. En lugar del selector de carpetas de PC: accesos rápidos predefinidos:
   `/storage/emulated/0/DCIM`, `/storage/emulated/0/Pictures`,
   `/storage/emulated/0/Download`, WhatsApp Images.

### Fase D — UI esencial táctil
9. CSS responsive (grilla de miniaturas, topbar compacto, botones grandes).
10. Visor de imágenes con swipe para navegar.
11. Flujo principal: escanear → grupos → seleccionar / conservar mejor → borrar.

### Fase E — Release
12. Job Android en `.github/workflows/build.yml` que genere el APK firmado con
    debug keystore y lo publique en Releases junto a los instaladores de PC.

---

## Esqueleto del workflow Android (GitHub Actions)

```yaml
android:
  runs-on: ubuntu-22.04
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-node@v4
      with: { node-version: 20 }
    - uses: actions/setup-java@v4
      with: { distribution: temurin, java-version: 17 }
    - uses: dtolnay/rust-toolchain@stable
      with:
        targets: aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
    - run: sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libssl-dev librsvg2-dev patchelf build-essential
    - run: |
        wget -q https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip -O ct.zip
        unzip -q ct.zip -d $HOME/android-sdk/cmdline-tools
        mv $HOME/android-sdk/cmdline-tools/cmdline-tools $HOME/android-sdk/cmdline-tools/latest
        yes | $HOME/android-sdk/cmdline-tools/latest/bin/sdkmanager --sdk_root=$HOME/android-sdk --licenses
        $HOME/android-sdk/cmdline-tools/latest/bin/sdkmanager --sdk_root=$HOME/android-sdk "ndk;27.2.12479018" "platforms;android-34" "build-tools;34.0.0"
        echo "ANDROID_HOME=$HOME/android-sdk" >> $GITHUB_ENV
        echo "NDK_HOME=$HOME/android-sdk/ndk/27.2.12479018" >> $GITHUB_ENV
    - run: npm ci
    - run: npx tauri android init   # si gen/ no está committeado
    - run: npx tauri android build --target aarch64 --apk
    - uses: actions/upload-artifact@v4
      with:
        name: android-apk
        path: src-tauri/gen/android/app/build/outputs/apk/**/*.apk
```

Ajustar la versión exacta del NDK (`27.2.12479018`) a la que se use
(r27c corresponde a esa versión).

---

## Decisiones ya tomadas (para no re-discutir)

- Distribución: **APK directo**, no Play Store.
- Alcance v1: **funcionalidad esencial** (escanear/grupos/borrar/historial/tema).
- Papelera en Android: **carpeta interna de la app** (no hay papelera del sistema).
- Permisos: **MANAGE_EXTERNAL_STORAGE** (válido fuera de Play Store).
- Versión de escritorio publicada: **1.0.0** (estable, no tocar hasta tener el APK).

## Referencias rápidas

- Arquitectura del proyecto: ver `AGENTS.md`
- Contrato IPC: structs en `src-tauri/src/lib.rs` ↔ `src/types.ts`
- Release desktop: tag `v1.0.0` en GitHub Releases
