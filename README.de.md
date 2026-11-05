<div align="center">

<table>
<tr>
<td><img src="readme-media/logo_booknamer.png" alt="BookNamer" width="140" /></td>
<td align="left">
<h1 style="margin:0">BookNamer</h1>
<p style="margin:0.25em 0 0"><strong>Umbenennen & Metadaten · EPUB / PDF</strong><br/>Tauri-Desktop · Obsidian-Plugin</p>
</td>
</tr>
</table>

<a href="https://atelier.atechnologie.fr/" title="l'Atelier – Verein für Buchherstellung und Forschungswerkzeuge"><img src="readme-media/logoasso.jpg" alt="l'Atelier" width="200" /></a>  
<sub>Entwickelt von <a href="https://atelier.atechnologie.fr/">l'Atelier</a> — Buchherstellung und Forschungswerkzeuge (EHESS)</sub>

<p>
🇫🇷 <a href="README.md">Français</a> ·
🇬🇧 <a href="README.en.md">English</a> ·
🇩🇪 <a href="README.de.md"><b>Deutsch</b></a> ·
🇪🇸 <a href="README.es.md">Español</a>
</p>

<p>
<a href="https://atelier.atechnologie.fr/"><img src="https://img.shields.io/badge/🌐_l'Atelier-atelier.atechnologie.fr-2d5016?style=for-the-badge" alt="Website l'Atelier" /></a>
<a href="https://github.com/Atelier-Recherche/BookNamer"><img src="https://img.shields.io/badge/📦_Repository-GitHub-181717?style=for-the-badge&logo=github" alt="GitHub-Repository" /></a>
<a href="https://obsidian.md/plugins?search=BRAT#"><img src="https://img.shields.io/badge/⬇️_Obsidian_Plugin-BRAT-7c3aed?style=for-the-badge&logo=obsidian&logoColor=white" alt="Plugin über BRAT installieren" /></a>
</p>

</div>

---

## 📸 Vorschau

| Desktop | Obsidian-Plugin |
| :---: | :---: |
| <img src="readme-media/screen1.jpg" alt="Desktop" width="400" /> | <img src="readme-media/screen2.jpg" alt="Obsidian-Plugin" width="400" /> |

---
## 📖 Über

Zwei ergänzende Werkzeuge zum Umbenennen, Anreichern und Normalisieren von Buch- und Artikeldateien (`.epub`, `.pdf`):

1. **`bib-renamer-desktop`**: Native Tauri-v2-/Rust-Desktop-App (Linux, macOS, Windows). Hohe Leistung, direkter lokaler Dateizugriff, Batch-Verarbeitung mit Svelte-5-Oberfläche.
2. **`obsidian-bib-renamer`**: Obsidian-Plugin (TypeScript, Obsidian-API v1.5+) für den lokalen Vault, kompatibel mit Android, iOS und Desktop.

---

## ⬇️ Obsidian-Plugin (BRAT)

1. 🔌 **BRAT** installieren: [Obsidian — BRAT](https://obsidian.md/plugins?search=BRAT#)
2. ➕ Dieses Repository mit *« Add Beta plugin »* hinzufügen:  
   `https://github.com/Atelier-Recherche/BookNamer`

> 💡 Unsere Plugins können noch auf Freigabe im Obsidian-Katalog warten; BRAT ermöglicht Tests sofort. Siehe auch 🌐 [l’Atelier](https://atelier.atechnologie.fr/).

### Lokaler Plugin-Build

```bash
cd obsidian-bib-renamer
npm install
npm run build
```

---

## 🏗 Architektur & Metadatenstrategie

Die Extraktion stützt sich nicht nur auf den Rohdateinamen. Sie folgt dieser Kaskade:

1. **Interne Metadatenextraktion**:
   - EPUB: Analyse von `content.opf` (Dublin-Core-Tags: `<dc:title>`, `<dc:creator>`, `<dc:identifier>`, `<dc:date>`, `<dc:publisher>`).
   - PDF: Analyse des `/Info`-Wörterbuchs (`/Title`, `/Author`, `/CreationDate` usw.) und Textextraktion.
2. **Heuristische Analyse des Rohnamens**:
   - Bereinigung von MD5-/SHA-Hashes (z. B. LibGen / Anna's Archive).
   - Erkennung von Identifikatoren (`ISBN-13`, `ISBN-10`, `DOI`, Jahr).
3. **Externe API-Suche (Anreicherung)**:
   - Bei vorhandener **ISBN**:
     1. **OpenLibrary** (`search.json` + `api/books`) (kostenlos ohne Schlüssel)
     2. **BNF** (Bibliothèque Nationale de France, offizieller frankophoner SRU-Katalog) (kostenlos ohne Schlüssel)
     3. **Amazon** (direkte ISBN-10/13-Suche auf Amazon.fr / Amazon.com)
     4. **Google Books** (Volumensuche mit optionalem Schlüssel)
   - Bei vorhandener **DOI**: **Crossref API** (kostenlos ohne Schlüssel).
   - Ohne Identifikator: automatische Textsuche auf OpenLibrary/Crossref via `Titel + Autor`.
4. **Benutzerbestätigung (Human-in-the-loop)**:
   - Interaktives Modal mit Quellname, erkannten Metadaten, Vorschlag und API-Alternativen.
   - Direkte Feldbearbeitung bei Bedarf.

---

## 1. `bib-renamer-desktop` (Tauri v2 + Svelte 5)

### Struktur

- `src-tauri/`: leistungsfähiger Rust-Kern
  - `parsers/epub.rs`: EPUB-Parser via `epub`-Crate
  - `parsers/pdf.rs`: PDF-Parser via `lopdf`
  - `heuristics.rs`: einmal kompilierte Regex (`once_cell`)
  - `api.rs`: asynchroner HTTP-Client `reqwest` (OpenLibrary, Crossref, Google Books)
  - `template.rs`: Template-Engine und Multi-OS-Bereinigung
  - `commands.rs`: an das Frontend exponierte Tauri-Befehle
- `src/`: Svelte-5-Oberfläche
  - `lib/Queue.svelte`: interaktive Warteschlange mit Statusanzeige
  - `lib/ConfirmModal.svelte`: Live-Validierungs- und Bearbeitungsmodal
  - `lib/TemplateEditor.svelte`: speicherbarer Vorlagenmanager
  - `routes/+page.svelte`: Hauptorchestrator & Drag & Drop

### Entwicklung & Start

```bash
cd bib-renamer-desktop
npm install
npm run tauri dev
```

### Windows-Installer (Desktop)

Laden Sie die neueste GitHub-Release herunter: `BookNamer_X.Y.Z_x64-setup.exe` (NSIS).

### Lokaler Installer-Build

```bash
cd bib-renamer-desktop
npm install
npm run tauri build
```

Der Installer wird in `bib-renamer-desktop/src-tauri/target/release/bundle/nsis/` erzeugt.

---

## 2. `obsidian-bib-renamer` (Obsidian-Plugin)

### Struktur

- `src/main.ts`: Plugin-Einstieg, Befehle und Kontextmenüs
- `src/parsers/epub.ts`: leichter EPUB-Parser mit `jszip` und `DOMParser`
- `src/parsers/pdf.ts`: binärer PDF-Parser, mobilkompatibel
- `src/api/`: Anfragen via Obsidian `requestUrl` (CORS-freundlich auf Android/iOS)
- `src/modal.ts`: natives Obsidian-Modal mit Vorschau und Notizerstellung
- `src/settings.ts`: Einstellungsreiter für Templates und API-Quellen

### Kernfunktionen

- Kontextmenü für `.epub`- und `.pdf`-Dateien im Vault-Explorer.
- Globaler Befehl: `Renommer et enrichir le fichier sélectionné`.
- Sicheres Umbenennen via `app.vault.rename()`.
- **Option „Zugehörige Lese-Notiz erstellen“**: erzeugt ein Markdown-Leseblatt mit vollständigem YAML-Frontmatter und Link zur Datei.

---

## 🧩 Verfügbare Template-Variablen

- `{auteur}`: erster Autor formatiert (`Nachname, Vorname`)
- `{titre}`: Titel des Buchs oder Artikels
- `{date}`: Erscheinungsjahr
- `{editeur}`: Verlag
- `{isbn}`: ISBN-Nummer
- `{doi}`: DOI-Kennung
- `{extension}`: Dateierweiterung (`epub`, `pdf`)

---

## 🌐 Links

- 🌐 [l’Atelier](https://atelier.atechnologie.fr/)
- 📦 [GitHub-Repository](https://github.com/Atelier-Recherche/BookNamer)
