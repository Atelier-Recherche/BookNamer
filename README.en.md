<div align="center" id="head">


<table>
<tr>
<td><img src="readme-media/logo_booknamer.png" alt="BookNamer" width="140" /></td>
<td align="left">
<h1 style="margin:0">BookNamer</h1>
<p style="margin:0.25em 0 0"><strong>Rename & metadata · EPUB / PDF</strong><br/>Tauri desktop · Obsidian plugin</p>
</td>
</tr>
</table>

<a href="https://atelier.atechnologie.fr/" title="l'Atelier – book-making and research tools association"><img src="https://raw.githubusercontent.com/Atelier-Recherche/.github/main/profile/logo.jpg" alt="l'Atelier" width="200" /></a>  
<sub>Developed by <a href="https://atelier.atechnologie.fr/">l'Atelier</a> — book-making and research tools (EHESS)</sub>

<p>
🇫🇷 <a href="README.md">Français</a> ·
🇬🇧 <a href="README.en.md"><b>English</b></a> ·
🇩🇪 <a href="README.de.md">Deutsch</a> ·
🇪🇸 <a href="README.es.md">Español</a>
</p>

<p>
<a href="https://atelier.atechnologie.fr/"><img src="https://img.shields.io/badge/🌐_l'Atelier-atelier.atechnologie.fr-2d5016?style=for-the-badge" alt="l'Atelier website" /></a>
<a href="https://github.com/Atelier-Recherche/BookNamer"><img src="https://img.shields.io/badge/📦_Repository-GitHub-181717?style=for-the-badge&logo=github" alt="GitHub repository" /></a>
<a href="https://obsidian.md/plugins?search=BRAT#"><img src="https://img.shields.io/badge/⬇️_Obsidian_Plugin-BRAT-7c3aed?style=for-the-badge&logo=obsidian&logoColor=white" alt="Install plugin via BRAT" /></a>
</p>

</div>

---

## 📸 Preview

| Desktop | Obsidian plugin |
| :---: | :---: |
| <img src="readme-media/screen1.jpg" alt="Desktop" width="400" /> | <img src="readme-media/screen2.jpg" alt="Obsidian plugin" width="400" /> |

---
## 📖 About

Two complementary tools for renaming, enriching and normalizing book and article files (`.epub`, `.pdf`):

1. **`bib-renamer-desktop`**: Native Tauri v2 / Rust desktop app (Linux, macOS, Windows). High performance, direct local filesystem access, batch processing with a Svelte 5 UI.
2. **`obsidian-bib-renamer`**: Obsidian plugin (TypeScript, Obsidian API v1.5+) aimed at the local vault, compatible with Android, iOS and Desktop.

---

## ⬇️ Obsidian plugin (BRAT)

1. 🔌 Install **BRAT**: [Obsidian — BRAT](https://obsidian.md/plugins?search=BRAT#)
2. ➕ Add this repository with *« Add Beta plugin »*:  
   `https://github.com/Atelier-Recherche/BookNamer`

> 💡 Our plugins may be pending approval on the Obsidian catalog; BRAT lets you try them now. See also 🌐 [l’Atelier](https://atelier.atechnologie.fr/).

### Local plugin build

```bash
cd obsidian-bib-renamer
npm install
npm run build
```

---

## 🏗 Architecture & metadata strategy

Extraction does not rely on the raw filename alone. It follows this cascade:

1. **Internal metadata extraction**:
   - EPUB: parse `content.opf` (Dublin Core tags: `<dc:title>`, `<dc:creator>`, `<dc:identifier>`, `<dc:date>`, `<dc:publisher>`).
   - PDF: parse the `/Info` dictionary (`/Title`, `/Author`, `/CreationDate`, etc.) and text extraction.
2. **Heuristic parsing of the raw name**:
   - Clean MD5/SHA hashes (e.g. LibGen / Anna's Archive).
   - Detect identifiers (`ISBN-13`, `ISBN-10`, `DOI`, year).
3. **External API lookup (enrichment)**:
   - If **ISBN** is present:
     1. **OpenLibrary** (`search.json` + `api/books`) (free, no key)
     2. **BNF** (Bibliothèque Nationale de France, official francophone SRU catalog) (free, no key)
     3. **Amazon** (direct ISBN-10/13 search on Amazon.fr / Amazon.com)
     4. **Google Books** (volume search with optional key)
   - If **DOI** is present: **Crossref API** (free, no key).
   - If no identifier: automatic text search on OpenLibrary/Crossref via `Title + Author`.
4. **User validation (human-in-the-loop)**:
   - Interactive modal showing the source name, identified metadata, proposed result and API alternatives.
   - Direct field editing when needed.

---

## 1. `bib-renamer-desktop` (Tauri v2 + Svelte 5)

### Structure

- `src-tauri/`: high-performance Rust core
  - `parsers/epub.rs`: EPUB parser via `epub` crate
  - `parsers/pdf.rs`: PDF parser via `lopdf`
  - `heuristics.rs`: regex compiled once (`once_cell`)
  - `api.rs`: async HTTP client `reqwest` (OpenLibrary, Crossref, Google Books)
  - `template.rs`: template engine and multi-OS sanitization
  - `commands.rs`: Tauri commands exposed to the frontend
- `src/`: Svelte 5 UI
  - `lib/Queue.svelte`: interactive queue with visual statuses
  - `lib/ConfirmModal.svelte`: live validation and editing modal
  - `lib/TemplateEditor.svelte`: savable template manager
  - `routes/+page.svelte`: main orchestrator & drag and drop

### Development & launch

```bash
cd bib-renamer-desktop
npm install
npm run tauri dev
```

### Windows installer (Desktop)

Download the latest GitHub release: `BookNamer_X.Y.Z_x64-setup.exe` (NSIS).

### Local installer build

```bash
cd bib-renamer-desktop
npm install
npm run tauri build
```

The installer is generated in `bib-renamer-desktop/src-tauri/target/release/bundle/nsis/`.

---

## 2. `obsidian-bib-renamer` (Obsidian plugin)

### Structure

- `src/main.ts`: plugin entry point, commands and context menus
- `src/parsers/epub.ts`: lightweight EPUB parser with `jszip` and `DOMParser`
- `src/parsers/pdf.ts`: binary PDF parser, mobile-compatible
- `src/api/`: requests via Obsidian `requestUrl` (CORS-friendly on Android/iOS)
- `src/modal.ts`: native Obsidian modal with preview and note creation
- `src/settings.ts`: settings tab for templates and API sources

### Key features

- Context menu on `.epub` and `.pdf` files in the vault explorer.
- Global command: `Renommer et enrichir le fichier sélectionné`.
- Safe rename via `app.vault.rename()`.
- **“Create associated reading note” option**: generates a Markdown reading sheet with full YAML frontmatter and a link to the file.

---

## 🧩 Available template variables

- `{auteur}`: first author formatted (`Last, First`)
- `{titre}`: book or article title
- `{date}`: publication year
- `{editeur}`: publisher
- `{isbn}`: ISBN number
- `{doi}`: DOI identifier
- `{extension}`: file extension (`epub`, `pdf`)

---

## 🌐 Links

- 🌐 [l’Atelier](https://atelier.atechnologie.fr/)
- 📦 [GitHub repository](https://github.com/Atelier-Recherche/BookNamer)
