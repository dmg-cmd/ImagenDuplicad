# Imagen Duplicada

Detector de imágenes duplicadas para escritorio (Windows, Linux y macOS).
Compara por hash **SHA-256** (copias exactas) y metadatos **EXIF** (probables duplicados recodificados).

> Guía completa y detallada: [`docs/README.md`](docs/README.md)

## 📥 Descargar

Instaladores listos para usar (sin necesidad de programar nada):

**👉 [Descargar la última versión](https://github.com/dmg-cmd/ImagenDuplicad/releases/latest)**

| Sistema | Archivo |
|---|---|
| Windows | `Imagen.Duplicada_*_x64-setup.exe` |
| Linux | `.deb` (Debian/Ubuntu) o `.AppImage` (cualquier distro) |
| macOS | `.dmg` |

En esa página, despliega la sección **Assets** y descarga el archivo de tu sistema operativo.

> También puedes probar las compilaciones más recientes (sin publicar) en la pestaña [Actions](https://github.com/dmg-cmd/ImagenDuplicad/actions): entra a la última ejecución y baja a **Artifacts** (requiere iniciar sesión en GitHub).

---

## Guía de uso

### 1. Escanear una carpeta

Abre la app y pulsa **"Escanear carpeta"**. Se analizan todas las imágenes de la carpeta y sus subcarpetas (JPEG, PNG, WebP, GIF, TIFF, BMP).

### 2. Revisar los grupos

La app agrupa las duplicadas encontradas:

- 🔵 **Grupo exacto**: copias idénticas byte a byte.
- 🟠 **Grupo probable**: misma cámara, fecha y dimensiones, pero archivo distinto (recodificadas).

Cada imagen muestra miniatura, nombre, dimensiones, peso y fecha.

### 3. Verificar antes de borrar

Haz clic en una miniatura para ver la imagen a tamaño completo (navega con `←` `→`, sale con `Esc`).

### 4. Seleccionar y borrar

- Marca las que quieras eliminar, o usa **"Conservar la mejor"** para dejar automáticamente solo la de mayor resolución.
- **"Borrar seleccionadas"** las envía a la papelera (recuperables).
- **"Borrar permanentemente"** las elimina sin vuelta atrás.

---

## Instalación desde el código fuente

Requisitos: [Rust](https://rustup.rs) y Node.js 18+.

```bash
git clone <repositorio>
cd ImagenDuplicad
npm install
npm run tauri dev     # modo desarrollo
npx tauri build       # generar instalador
```

Dependencias extra:

- **Linux:** `sudo apt install -y libwebkit2gtk-4.1-dev libssl-dev librsvg2-dev patchelf build-essential`
- **Windows:** WebView2 Runtime (preinstalado en Windows 10/11)
- **macOS:** `xcode-select --install`

Los instaladores se generan en `src-tauri/target/release/bundle/`.

---

## Compilación automática

Este repo incluye un workflow de GitHub Actions (`.github/workflows/build.yml`) que compila instaladores para Windows, Linux y macOS en cada push a `main`. Los artefactos quedan disponibles en la pestaña **Actions** del repositorio.
