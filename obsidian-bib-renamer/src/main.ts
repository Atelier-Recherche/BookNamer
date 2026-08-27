import { Plugin, TFile, TAbstractFile, Menu, Notice, Editor } from 'obsidian';
import { DEFAULT_SETTINGS, PluginSettings, BookMetadata } from './types';
import { BibRenamerSettingTab } from './settings';
import { parseFilename } from './heuristics';
import { extractEpubMetadata } from './parsers/epub';
import { extractPdfMetadata } from './parsers/pdf';
import { enrichMetadata } from './api';
import { BibRenamerModal } from './modal';

export default class BibRenamerPlugin extends Plugin {
  settings: PluginSettings = DEFAULT_SETTINGS;

  async onload() {
    await this.loadSettings();

    this.addSettingTab(new BibRenamerSettingTab(this.app, this));

    // Global command: Rename and enrich active file
    this.addCommand({
      id: 'rename-current-file',
      name: 'Renommer et enrichir le fichier sélectionné',
      checkCallback: (checking: boolean) => {
        const activeFile = this.app.workspace.getActiveFile();
        if (activeFile && this.isSupportedExtension(activeFile.extension)) {
          if (!checking) {
            this.processFile(activeFile);
          }
          return true;
        }
        return false;
      },
    });

    // File context menu (File Explorer & Link context menu)
    this.registerEvent(
      this.app.workspace.on('file-menu', (menu: Menu, file: TAbstractFile) => {
        if (file instanceof TFile && this.isSupportedExtension(file.extension)) {
          menu.addItem((item) => {
            item
              .setTitle('Enrichir et renommer le document 📚')
              .setIcon('book-open')
              .onClick(() => {
                this.processFile(file);
              });
          });
        }
      })
    );
  }

  isSupportedExtension(ext: string): boolean {
    const clean = ext.toLowerCase().replace(/^\.+/, '');
    return clean === 'epub' || clean === 'pdf';
  }

  async processFile(file: TFile) {
    const notice = new Notice(`Extraction des métadonnées pour ${file.name}...`, 0);

    try {
      // 1. Read binary content using mobile-safe Obsidian API
      const arrayBuffer = await this.app.vault.readBinary(file);

      // 2. Extract internal metadata based on format
      let metadata: BookMetadata = {
        title: null,
        authors: [],
        date: null,
        publisher: null,
        isbn: null,
        doi: null,
        source: 'unknown',
        confidence: 'low',
      };

      const ext = file.extension.toLowerCase();
      if (ext === 'epub') {
        try {
          metadata = await extractEpubMetadata(arrayBuffer);
        } catch (e) {
          console.warn('[obsidian-bib-renamer] EPUB internal parser warning:', e);
        }
      } else if (ext === 'pdf') {
        try {
          metadata = extractPdfMetadata(arrayBuffer);
        } catch (e) {
          console.warn('[obsidian-bib-renamer] PDF internal parser warning:', e);
        }
      }

      // 3. Parse filename heuristics for missing fields (ISBN, DOI, Year, Parts)
      const parsedName = parseFilename(file.name);
      if (!metadata.isbn && parsedName.isbn) {
        metadata.isbn = parsedName.isbn;
      }
      if (!metadata.doi && parsedName.doi) {
        metadata.doi = parsedName.doi;
      }
      if (!metadata.date && parsedName.year) {
        metadata.date = parsedName.year;
      }
      if (!metadata.title && parsedName.parts.length > 0) {
        metadata.title = parsedName.parts[0];
      }

      // 4. API Enrichment
      notice.setMessage(`Recherche API bibliographique (${metadata.isbn ? 'ISBN' : metadata.doi ? 'DOI' : 'Titre'})...`);
      const enrichedResults = await enrichMetadata(metadata, this.settings);

      let finalMainMeta = metadata;
      let alternatives: BookMetadata[] = [];

      if (enrichedResults.length > 0) {
        finalMainMeta = enrichedResults[0];
        alternatives = enrichedResults.slice(1);
      }

      notice.hide();

      // 5. Open confirmation modal
      const activeTemplate = this.settings.templates.find(
        (t) => t.id === this.settings.activeTemplateId
      ) ?? this.settings.templates[0];

      new BibRenamerModal(
        this.app,
        file,
        finalMainMeta,
        alternatives,
        activeTemplate,
        this.settings,
        (renamedFile, noteFile) => {
          // If note was created, open it in the active leaf
          if (noteFile) {
            this.app.workspace.getLeaf(false).openFile(noteFile);
          }
        }
      ).open();
    } catch (err: any) {
      notice.hide();
      console.error('[obsidian-bib-renamer] Error processing file:', err);
      new Notice(`Erreur de traitement : ${err.message || err}`);
    }
  }

  async loadSettings() {
    this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
  }

  async saveSettings() {
    await this.saveData(this.settings);
  }
}
