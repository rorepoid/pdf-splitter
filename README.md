# PDF Splitter & Compressor (Rust)

Herramienta de alto rendimiento para dividir archivos PDF, comprimirlos y generar imágenes web (WebP) optimizadas. Diseñada para ser portable y funcionar tanto en Linux como en Windows (incluyendo versiones Legacy 32-bit).

## Funcionalidades

1.  **Split:** Divide archivos PDF multipágina en archivos PDF individuales.
2.  **Organización:** Crea automáticamente una estructura de carpetas basada en la fecha del nombre del archivo (ej. `1982/01/30/lima/pages/`).
3.  **Compresión:** Utiliza **Ghostscript** para reducir drásticamente el tamaño de los PDFs generados.
4.  **Imágenes:** Genera una versión completa (`.webp`) y un thumbnail (`_thumb.webp`) redimensionado proporcionalmente (ancho 310px).
5.  **Paralelismo:** Procesa múltiples archivos PDF simultáneamente aprovechando todos los núcleos del CPU.

## Requerimientos Técnicos

### Para Ejecutar (Usuario Final)

*   **Sistema Operativo:** Linux (x64) o Windows (x86/x64).
*   **Ghostscript (Opcional pero Recomendado):**
    *   **Linux:** Tener `gs` instalado (`sudo apt install ghostscript`).
    *   **Windows:**
        1.  Ve a [ArtifexSoftware/ghostpdl-downloads](https://github.com/ArtifexSoftware/ghostpdl-downloads/releases/latest).
        2.  Descarga el instalador según tu arquitectura:
            *   Para 32-bit: **`gs10060w32.exe`**
            *   Para 64-bit: **`gs10060w64.exe`**
        3.  Ejecuta el instalador (puedes instalarlo temporalmente).
        4.  Ve a la ruta de instalación (ej. `C:\Program Files\gs\gs10.06.0\bin`).
        5.  Copia estos archivos a la carpeta de `pdf_splitter.exe`:
            *   32-bit: **`gswin32c.exe`** y **`gsdll32.dll`**.
            *   64-bit: **`gswin64c.exe`** y **`gsdll64.dll`**.
    *   *Nota:* Si no se encuentra Ghostscript, la aplicación funcionará pero no comprimirá los PDFs ni generará imágenes.

### Para Compilar (Desarrollador)

*   **Rust:** Última versión estable (`rustup`).
*   **Compilador C (Linux):** `build-essential`.
*   **Cross-Compilation (para Windows desde Linux):** `mingw-w64`.

## Instalación y Compilación

### 1. Compilar para Linux (Nativo)
```bash
cargo build --release
# El binario estará en: target/release/pdf_splitter
```

### 2. Compilar para Windows 32-bit (Desde Linux)
Requiere `mingw-w64` instalado.
```bash
rustup target add i686-pc-windows-gnu
cargo build --release --target i686-pc-windows-gnu
# El binario estará en: target/i686-pc-windows-gnu/release/pdf_splitter.exe
```

### 3. Compilar desde Windows (Nativo)
Si ya estás en una máquina Windows con Rust instalado:

```powershell
cargo build --release
# El binario estará en: target\release\pdf_splitter.exe
```

## Uso

La herramienta funciona por línea de comandos (CLI).

### Argumentos

*   `--path <RUTA>`: (Requerido) Ruta al archivo PDF o carpeta con PDFs a procesar.
*   `--output-dir <RUTA>`: (Opcional) Carpeta raíz donde se guardarán los resultados. Por defecto: `output`.

### Ejemplos

**1. Procesar una carpeta de PDFs y guardar en "resultados":**
```bash
./pdf_splitter --path "documentos/enero" --output-dir "resultados"
```

**2. Procesar un solo archivo (Linux):**
```bash
./pdf_splitter --path "REPLIM010182.pdf"
```

**3. Ejecutar en Windows:**
```cmd
pdf_splitter.exe --path "C:\Escaneos\Enero" --output-dir "Z:\Procesados"
```

## Estructura de Salida

Si procesas `REPLIM200182.pdf` (Fecha: 20 de Enero, 1982), se generará:

```text
output_dir/
└── 1982/
    └── 01/
        └── 20/
            └── lima/
                └── pages/
                    ├── 01.pdf            (Original extraído)
                    ├── 01_compress.pdf   (Comprimido con GS)
                    ├── 01.webp           (Imagen Full HD)
                    ├── 01_thumb.webp     (Thumbnail 310px ancho)
                    ├── 02.pdf
                    └── ...
```

## Notas sobre Ghostscript
La herramienta busca el binario de Ghostscript en el siguiente orden:
1.  **Windows:** `gswin32c.exe` (local), `gswin64c.exe` (local), `gswin32c` (PATH).
2.  **Linux:** `./gs` (local), `gs` (PATH).

Asegúrese de tener el binario accesible para habilitar la compresión y generación de imágenes.
