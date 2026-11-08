<div align="center" id="head">


<table>
<tr>
<td><img src="readme-media/logo_booknamer.png" alt="BookNamer" width="140" /></td>
<td align="left">
<h1 style="margin:0">BookNamer</h1>
<p style="margin:0.25em 0 0"><strong>Renombrado y metadatos · EPUB / PDF</strong><br/>Escritorio Tauri · Plugin Obsidian</p>
</td>
</tr>
</table>

<a href="https://atelier.atechnologie.fr/" title="l'Atelier – asociación de fabricación de libros y herramientas de investigación"><img src="https://raw.githubusercontent.com/Atelier-Recherche/.github/main/profile/logo.jpg" alt="l'Atelier" width="200" /></a>  
<sub>Desarrollado por <a href="https://atelier.atechnologie.fr/">l'Atelier</a> — fabricación de libros y herramientas de investigación (EHESS)</sub>

<p>
🇫🇷 <a href="README.md">Français</a> ·
🇬🇧 <a href="README.en.md">English</a> ·
🇩🇪 <a href="README.de.md">Deutsch</a> ·
🇪🇸 <a href="README.es.md"><b>Español</b></a>
</p>

<p>
<a href="https://atelier.atechnologie.fr/"><img src="https://img.shields.io/badge/🌐_l'Atelier-atelier.atechnologie.fr-2d5016?style=for-the-badge" alt="Sitio l'Atelier" /></a>
<a href="https://github.com/Atelier-Recherche/BookNamer"><img src="https://img.shields.io/badge/📦_Repositorio-GitHub-181717?style=for-the-badge&logo=github" alt="Repositorio GitHub" /></a>
<a href="https://obsidian.md/plugins?search=BRAT#"><img src="https://img.shields.io/badge/⬇️_Plugin_Obsidian-BRAT-7c3aed?style=for-the-badge&logo=obsidian&logoColor=white" alt="Instalar el plugin vía BRAT" /></a>
</p>

</div>

---

## 📸 Vista previa

| Escritorio | Plugin Obsidian |
| :---: | :---: |
| <img src="readme-media/screen1.jpg" alt="Escritorio" width="400" /> | <img src="readme-media/screen2.jpg" alt="Plugin Obsidian" width="400" /> |

---
## 📖 Acerca de

Dos herramientas complementarias para renombrar, enriquecer y normalizar archivos de libros y artículos (`.epub`, `.pdf`):

1. **`bib-renamer-desktop`**: Aplicación de escritorio nativa Tauri v2 / Rust (Linux, macOS, Windows). Alto rendimiento, acceso directo al sistema de archivos local, procesamiento por lotes con interfaz Svelte 5.
2. **`obsidian-bib-renamer`**: Plugin Obsidian (TypeScript, API Obsidian v1.5+) orientado a la bóveda local, compatible con Android, iOS y escritorio.

---

## ⬇️ Plugin Obsidian (BRAT)

1. 🔌 Instalar **BRAT**: [Obsidian — BRAT](https://obsidian.md/plugins?search=BRAT#)
2. ➕ Añadir este repositorio con la opción *« Add Beta plugin »*:  
   `https://github.com/Atelier-Recherche/BookNamer`

> 💡 Nuestros plugins pueden estar pendientes de validación en el catálogo Obsidian; BRAT permite probarlos ya. Ver también 🌐 [l’Atelier](https://atelier.atechnologie.fr/).

### Build local del plugin

```bash
cd obsidian-bib-renamer
npm install
npm run build
```

---

## 🏗 Arquitectura y estrategia de metadatos

La extracción no se basa solo en el nombre de archivo bruto. Sigue esta cascada:

1. **Extracción de metadatos internos**:
   - EPUB: análisis de `content.opf` (etiquetas Dublin Core: `<dc:title>`, `<dc:creator>`, `<dc:identifier>`, `<dc:date>`, `<dc:publisher>`).
   - PDF: análisis del diccionario `/Info` (`/Title`, `/Author`, `/CreationDate`, etc.) y extracción textual.
2. **Parsing heurístico del nombre bruto**:
   - Limpieza de hashes MD5/SHA (p. ej. LibGen / Anna's Archive).
   - Detección de identificadores (`ISBN-13`, `ISBN-10`, `DOI`, año).
3. **Búsqueda API externa (enriquecimiento)**:
   - Si hay **ISBN**:
     1. **OpenLibrary** (`search.json` + `api/books`) (gratis sin clave)
     2. **BNF** (Bibliothèque Nationale de France, catálogo oficial francófono SRU) (gratis sin clave)
     3. **Amazon** (búsqueda directa por ISBN-10/13 en Amazon.fr / Amazon.com)
     4. **Google Books** (búsqueda de volúmenes con clave opcional)
   - Si hay **DOI**: **Crossref API** (gratis sin clave).
   - Sin identificador: búsqueda textual automática en OpenLibrary/Crossref vía `Título + Autor`.
4. **Validación del usuario (human-in-the-loop)**:
   - Modal interactivo con el nombre de origen, metadatos identificados, resultado propuesto y alternativas de API.
   - Edición directa de campos si es necesario.

---

## 1. `bib-renamer-desktop` (Tauri v2 + Svelte 5)

### Estructura

- `src-tauri/`: núcleo Rust de alto rendimiento
  - `parsers/epub.rs`: parser EPUB vía crate `epub`
  - `parsers/pdf.rs`: parser PDF vía `lopdf`
  - `heuristics.rs`: regex compiladas una sola vez (`once_cell`)
  - `api.rs`: cliente HTTP async `reqwest` (OpenLibrary, Crossref, Google Books)
  - `template.rs`: motor de plantillas y sanitización multi-OS
  - `commands.rs`: comandos Tauri expuestos al frontend
- `src/`: interfaz Svelte 5
  - `lib/Queue.svelte`: cola interactiva con estados visuales
  - `lib/ConfirmModal.svelte`: modal de validación y edición en vivo
  - `lib/TemplateEditor.svelte`: gestor de plantillas guardables
  - `routes/+page.svelte`: orquestador principal y drag & drop

### Desarrollo y lanzamiento

```bash
cd bib-renamer-desktop
npm install
npm run tauri dev
```

### Instalador Windows (escritorio)

Descargue la última release de GitHub: `BookNamer_X.Y.Z_x64-setup.exe` (NSIS).

### Build local del instalador

```bash
cd bib-renamer-desktop
npm install
npm run tauri build
```

El instalador se genera en `bib-renamer-desktop/src-tauri/target/release/bundle/nsis/`.

---

## 2. `obsidian-bib-renamer` (Plugin Obsidian)

### Estructura

- `src/main.ts`: punto de entrada del plugin, comandos y menús contextuales
- `src/parsers/epub.ts`: parser EPUB ligero con `jszip` y `DOMParser`
- `src/parsers/pdf.ts`: parser PDF binario compatible con móvil
- `src/api/`: peticiones vía Obsidian `requestUrl` (CORS-friendly en Android/iOS)
- `src/modal.ts`: modal nativo Obsidian con vista previa y creación de notas
- `src/settings.ts`: pestaña de configuración de plantillas y fuentes API

### Funciones clave

- Menú contextual sobre archivos `.epub` y `.pdf` en el explorador de la bóveda.
- Comando global: `Renommer et enrichir le fichier sélectionné`.
- Renombrado seguro vía `app.vault.rename()`.
- **Opción «Crear la nota de lectura asociada»**: genera una ficha de lectura Markdown con frontmatter YAML completo y enlace al archivo.

---

## 🧩 Variables de plantilla disponibles

- `{auteur}`: primer autor formateado (`Apellido, Nombre`)
- `{titre}`: título del libro o artículo
- `{date}`: año de publicación
- `{editeur}`: editorial
- `{isbn}`: número ISBN
- `{doi}`: identificador DOI
- `{extension}`: extensión del archivo (`epub`, `pdf`)

---

## 🌐 Enlaces

- 🌐 [l’Atelier](https://atelier.atechnologie.fr/)
- 📦 [Repositorio GitHub](https://github.com/Atelier-Recherche/BookNamer)
