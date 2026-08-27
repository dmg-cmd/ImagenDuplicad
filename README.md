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

## ✨ Características principales

- 🔍 **Doble motor de detección**:
  - **Exactos (SHA-256)**: Identificación inmediata e infalible de duplicados idénticos byte a byte.
  - **Probables (EXIF)**: Detección por cámara, dimensiones y fecha de toma, ideal para fotos recodificadas.
  - **Similares (dHash)**: Búsqueda visual perceptual para encontrar imágenes idénticas aunque varíe el peso o resolución (con umbrales: Estricta, Normal, Laxa, Muy laxa).
- 📁 **Gestión y vista por carpetas**:
  - Resumen del espacio recuperable por carpeta.
  - **Revisión individual**: Filtra y revisa únicamente los duplicados pertenecientes a una carpeta específica.
  - **Abrir en explorador**: Acceso directo para abrir cualquier carpeta en el explorador de archivos de tu sistema operativo.
- 🖼️ **Visor y comparador avanzado**:
  - Vista previa a tamaño completo con navegación por teclado (`←`, `→`, `Esc`).
  - Comparador lado a lado, modo cortina interactiva y mapa de diferencias visuales (diff).
- 🗂️ **Organización flexible**:
  - **Conservar la mejor**: Elige automáticamente la de mayor resolución o calidad.
  - **Mover conservadas**: Opción para trasladar las mejores fotos a una carpeta destino organizada.
  - **Carpetas excluidas**: Lista negra de directorios que no deben ser analizados.
- 🗑️ **Borrado seguro y registro**:
  - Envío a papelera de reciclaje o borrado permanente.
  - **Historial**: Registro en archivo CSV de todas las operaciones realizadas.
- 🌓 **Tema claro y oscuro**: Interfaz moderna adaptable a tus preferencias.

---

## Guía de uso

### 1. Escanear una carpeta

Abre la app y pulsa **"Escanear carpeta"**. Se analizan recursivamente todas las imágenes (JPEG, PNG, WebP, GIF, TIFF, BMP). Opcionalmente puedes activar la casilla **"Buscar similares"** o gestionar **"Carpetas excluidas"**.

### 2. Revisar los grupos o explorar por carpetas

- **Por grupos**:
  - 🔵 **Grupo exacto**: copias idénticas byte a byte.
  - 🟠 **Grupo probable**: misma cámara, fecha y dimensiones (o similares visualmente).
- **Por carpetas**:
  - Consulta qué carpetas ocupan más espacio en duplicados.
  - Haz clic en **"🔍 Revisar"** en cualquier carpeta para enfocarte solo en sus duplicados.
  - Haz clic en **"📂 Abrir"** para ver la carpeta en el Explorador de Windows/Finder/gestor de archivos.

### 3. Verificar antes de borrar

- Haz clic en cualquier miniatura para verla en pantalla completa.
- Usa la herramienta de **Comparar** para analizar diferencias con cortina o mapa de calor.

### 4. Seleccionar, mover o borrar

- Marca manualmente las imágenes que deseas eliminar o pulsa **"Conservar la mejor"**.
- Si deseas organizar tus fotos limpias, activa **"Mover conservadas"** e indica la carpeta de destino.
- Elige **"Borrar a papelera"** (seguro y recuperable) o **"Borrar permanente"**.
- Consulta el registro de acciones desde el botón **"📄 Historial"**.

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
