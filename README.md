<div align="center">

<table>
<tr>
<td><img src="readme-media/logo_booknamer.png" alt="BookNamer" width="140" /></td>
<td align="left">
<h1 style="margin:0">BookNamer</h1>
<p style="margin:0.25em 0 0"><strong>Renommage & métadonnées · EPUB / PDF</strong><br/>Desktop Tauri · Plugin Obsidian</p>
</td>
</tr>
</table>

<a href="https://atelier.atechnologie.fr/" title="l'Atelier – Association de fabrication de livres et d'outils de recherche"><img src="readme-media/logoasso.jpg" alt="l'Atelier" width="200" /></a>  
<sub>Développé par <a href="https://atelier.atechnologie.fr/">l'Atelier</a> — fabrication de livres et outils de recherche (EHESS)</sub>

<p>
🇫🇷 <a href="README.md"><b>Français</b></a> ·
🇬🇧 <a href="README.en.md">English</a> ·
🇩🇪 <a href="README.de.md">Deutsch</a> ·
🇪🇸 <a href="README.es.md">Español</a>
</p>

<p>
<a href="https://atelier.atechnologie.fr/"><img src="https://img.shields.io/badge/🌐_l'Atelier-atelier.atechnologie.fr-2d5016?style=for-the-badge" alt="Site l'Atelier" /></a>
<a href="https://github.com/Morglaf/BookNamer"><img src="https://img.shields.io/badge/📦_Dépôt-GitHub-181717?style=for-the-badge&logo=github" alt="Dépôt GitHub" /></a>
<a href="https://obsidian.md/plugins?search=BookNamer#"><img src="https://img.shields.io/badge/⬇️_Plugin_Obsidian-BookNamer-7c3aed?style=for-the-badge&logo=obsidian&logoColor=white" alt="Plugin Obsidian BookNamer" /></a>
</p>

</div>

---

## 📸 Aperçu

| Desktop | Plugin Obsidian |
| :---: | :---: |
| <img src="readme-media/screen1.jpg" alt="Desktop" width="400" /> | <img src="readme-media/screen2.jpg" alt="Plugin Obsidian" width="400" /> |

---
## 📖 À propos

Deux outils complémentaires pour le renommage, l’enrichissement et la normalisation des fichiers de livres et d’articles (`.epub`, `.pdf`) :

1. **`bib-renamer-desktop`** : Application Tauri v2 / Rust native pour Desktop (Linux, macOS, Windows). Haute performance, accès direct au système de fichiers local, traitement par batch avec interface Svelte 5.
2. **Plugin Obsidian `booknamer` (BookNamer)** : TypeScript, API Obsidian v1.5+, orienté voûte locale, compatible Android, iOS et Desktop. Sources dans `obsidian-bib-renamer/`.

---

## ⬇️ Plugin Obsidian (BookNamer)

**ID catalogue :** `booknamer`

### Installation (catalogue communautaire)

1. Ouvrir **Paramètres → Plugins communautaires → Parcourir**
2. Rechercher **BookNamer** et installer

> Si le plugin n’apparaît pas encore dans le catalogue, utilisez BRAT en fallback (voir ci-dessous).

### Installation beta (BRAT)

1. Installer **BRAT** : [Obsidian — BRAT](https://obsidian.md/plugins?search=BRAT#)
2. Ajouter ce dépôt avec *« Add Beta plugin »* :  
   `https://github.com/Morglaf/BookNamer`

### Usage réseau (disclosure)

Le plugin enrichit les métadonnées via Internet lorsque vous lancez un renommage. Aucune télémétrie. Services contactés :

| Service | Pourquoi |
| --- | --- |
| [Open Library](https://openlibrary.org/) | Recherche / fiche ISBN et titre+auteur |
| [BNF (SRU)](https://catalogue.bnf.fr/) | Catalogue francophone par ISBN / texte |
| Amazon.fr / Amazon.com | Complément de métadonnées par ISBN (pages produit HTML) |
| [Crossref](https://www.crossref.org/) | Résolution DOI et recherche bibliographique |
| [Google Books](https://developers.google.com/books) | Recherche de volumes par ISBN (clé API optionnelle) |

Les fichiers traités restent dans votre vault Obsidian ; le plugin n’accède pas au système de fichiers hors vault.

### Build local du plugin

```bash
cd obsidian-bib-renamer
npm install
npm run build
```

Le dossier plugin dans le vault doit s’appeler `booknamer` (égal à l’`id` du manifest).

---

## 🏗 Architecture & stratégie métadonnées

L’extraction ne repose pas uniquement sur le nom de fichier brut. Elle suit la cascade suivante :

1. **Extraction de métadonnées internes** :
   - EPUB : analyse du `content.opf` (balises Dublin Core : `<dc:title>`, `<dc:creator>`, `<dc:identifier>`, `<dc:date>`, `<dc:publisher>`).
   - PDF : analyse du dictionnaire `/Info` (`/Title`, `/Author`, `/CreationDate`, etc.) et extraction textuelle.
2. **Parsing heuristique du nom brut** :
   - Nettoyage des hashes MD5/SHA (ex: LibGen / Anna's Archive).
   - Détection des identifiants (`ISBN-13`, `ISBN-10`, `DOI`, année).
3. **Recherche API externe (Enrichissement)** :
   - Si **ISBN** présent :
     1. **OpenLibrary** (`search.json` + `api/books`) (gratuit sans clé)
     2. **BNF** (Bibliothèque Nationale de France, catalogue officiel francophone SRU) (gratuit sans clé)
     3. **Amazon** (Recherche directe par ISBN-10/13 Amazon.fr / Amazon.com)
     4. **Google Books** (recherche de volumes avec clé optionnelle)
   - Si **DOI** présent : **Crossref API** (gratuit sans clé).
   - Si identifiant absent : Recherche textuelle automatique sur OpenLibrary/Crossref via `Titre + Auteur`.
4. **Validation utilisateur (Human-in-the-loop)** :
   - Modal interactive présentant le nom source, les métadonnées identifiées, le résultat proposé et les alternatives d’API.
   - Édition directe des champs si nécessaire.

---

## 1. `bib-renamer-desktop` (Tauri v2 + Svelte 5)

### Structure

- `src-tauri/` : Cœur Rust haute performance
  - `parsers/epub.rs` : Parser EPUB via `epub` crate
  - `parsers/pdf.rs` : Parser PDF via `lopdf`
  - `heuristics.rs` : Regex compilées une seule fois (`once_cell`)
  - `api.rs` : Client HTTP async `reqwest` (OpenLibrary, Crossref, Google Books)
  - `template.rs` : Moteur de templates et sanitisation multi-OS
  - `commands.rs` : Commandes Tauri exposées au frontend
- `src/` : Interface utilisateur Svelte 5
  - `lib/Queue.svelte` : File d’attente interactive avec statuts visuels
  - `lib/ConfirmModal.svelte` : Modal de validation et d’édition en direct
  - `lib/TemplateEditor.svelte` : Gestionnaire de modèles sauvegardables
  - `routes/+page.svelte` : Orchestrateur principal & Drag & Drop

### Développement & lancement

```bash
cd bib-renamer-desktop
npm install
npm run tauri dev
```

### Installer Windows (Desktop)

Téléchargez la dernière release GitHub : fichier `BookNamer_X.Y.Z_x64-setup.exe` (NSIS).

### Build local de l’installeur

```bash
cd bib-renamer-desktop
npm install
npm run tauri build
```

L’installeur est généré dans `bib-renamer-desktop/src-tauri/target/release/bundle/nsis/`.

---

## 2. Plugin Obsidian `booknamer` (sources dans `obsidian-bib-renamer/`)

### Structure

- `src/main.ts` : Point d’entrée du plugin, commandes et menus contextuels
- `src/parsers/epub.ts` : Parser EPUB léger avec `jszip` et `DOMParser`
- `src/parsers/pdf.ts` : Parser PDF binaire compatible mobile
- `src/api/` : Requêtes via Obsidian `requestUrl` (CORS-friendly sur Android/iOS)
- `src/modal.ts` : Modal native Obsidian avec prévisualisation et création de notes
- `src/settings.ts` : Onglet de configuration des templates et des sources API

### Fonctionnalités clés

- Menu contextuel sur les fichiers `.epub` et `.pdf` dans l’explorateur de voûte.
- Commande globale : `Renommer et enrichir le fichier sélectionné`.
- Renommage sûr via `app.vault.rename()`.
- **Option « Créer la note de lecture associée »** : génère une fiche de lecture Markdown avec frontmatter YAML complet et lien vers le fichier.

---

## 🧩 Variables de template disponibles

- `{auteur}` : Premier auteur formaté (`Nom, Prénom`)
- `{titre}` : Titre du livre ou de l’article
- `{date}` : Année de publication
- `{editeur}` : Maison d’édition
- `{isbn}` : Numéro ISBN
- `{doi}` : Identifiant DOI
- `{extension}` : Extension du fichier (`epub`, `pdf`)

---

## 🌐 Liens

- 🌐 [l’Atelier](https://atelier.atechnologie.fr/)
- 📦 [Dépôt GitHub](https://github.com/Morglaf/BookNamer)
- 🔌 Plugin id : `booknamer`
