# BookNamer (2026)

Deux outils complémentaires pour le renommage, l'enrichissement et la normalisation des fichiers de livres et d'articles (`.epub`, `.pdf`) :

1. **`bib-renamer-desktop`** : Application Tauri v2 / Rust native pour Desktop (Linux, macOS, Windows). Haute performance, accès direct au système de fichiers local, traitement par batch avec interface Svelte 5.
2. **`obsidian-bib-renamer`** : Plugin Obsidian (TypeScript, API Obsidian v1.5+) orienté voûte locale, compatible Android, iOS et Desktop.

---

## Architecture & Stratégie Métadonnées

L'extraction ne repose pas uniquement sur le nom de fichier brut. Elle suit la cascade suivante :

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
   - Modal interactive présentant le nom source, les métadonnées identifiées, le résultat proposé et les alternatives d'API.
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
  - `lib/Queue.svelte` : File d'attente interactive avec statuts visuels
  - `lib/ConfirmModal.svelte` : Modal de validation et d'édition en direct
  - `lib/TemplateEditor.svelte` : Gestionnaire de modèles sauvegardables
  - `routes/+page.svelte` : Orchestrateur principal & Drag & Drop

### Développement & Lancement
```bash
cd bib-renamer-desktop
npm install
npm run tauri dev
```

---

## 2. `obsidian-bib-renamer` (Plugin Obsidian)

### Structure
- `src/main.ts` : Point d'entrée du plugin, commandes et menus contextuels
- `src/parsers/epub.ts` : Parser EPUB léger avec `jszip` et `DOMParser`
- `src/parsers/pdf.ts` : Parser PDF binaire compatible mobile
- `src/api/` : Requêtes via Obsidian `requestUrl` (CORS-friendly sur Android/iOS)
- `src/modal.ts` : Modal native Obsidian avec prévisualisation et création de notes
- `src/settings.ts` : Onglet de configuration des templates et des sources API

### Fonctionnalités Clés
- Menu contextuel sur les fichiers `.epub` et `.pdf` dans l'explorateur de voûte.
- Commande globale : `Renommer et enrichir le fichier sélectionné`.
- Renommage sûr via `app.vault.rename()`.
- **Option "Créer la note de lecture associée"** : génère une fiche de lecture Markdown avec frontmatter YAML complet et lien vers le fichier.

### Build
```bash
cd obsidian-bib-renamer
npm install
npm run build
```

---

## Variables de Template Disponibles

- `{auteur}` : Premier auteur formaté (`Nom, Prénom`)
- `{titre}` : Titre du livre ou de l'article
- `{date}` : Année de publication
- `{editeur}` : Maison d'édition
- `{isbn}` : Numéro ISBN
- `{doi}` : Identifiant DOI
- `{extension}` : Extension du fichier (`epub`, `pdf`)

---

## Installer Windows (Desktop)

Téléchargez la dernière release GitHub : fichier `BookNamer_X.Y.Z_x64-setup.exe` (NSIS).

### Build local de l'installeur

```bash
cd bib-renamer-desktop
npm install
npm run tauri build
```

L'installeur est généré dans `bib-renamer-desktop/src-tauri/target/release/bundle/nsis/`.

