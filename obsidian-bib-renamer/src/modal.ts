import { App, Modal, Setting, TFile, Notice } from 'obsidian';
import { BookMetadata, NamingTemplate, PluginSettings } from './types';
import { applyTemplate } from './template';

export class BibRenamerModal extends Modal {
  private file: TFile;
  private metadata: BookMetadata;
  private alternatives: BookMetadata[];
  private template: NamingTemplate;
  private settings: PluginSettings;
  private onComplete: (renamedFile: TFile, noteFile?: TFile) => void;

  private editableMetadata: BookMetadata;
  private authorsInputText: string;
  private selectedAlternativeIndex: number = -1;
  private previewEl!: HTMLElement;

  constructor(
    app: App,
    file: TFile,
    metadata: BookMetadata,
    alternatives: BookMetadata[],
    template: NamingTemplate,
    settings: PluginSettings,
    onComplete: (renamedFile: TFile, noteFile?: TFile) => void
  ) {
    super(app);
    this.file = file;
    this.metadata = metadata;
    this.alternatives = alternatives;
    this.template = template;
    this.settings = settings;
    this.onComplete = onComplete;

    this.editableMetadata = { ...metadata, authors: [...metadata.authors] };
    this.authorsInputText = this.editableMetadata.authors.join('; ');
  }

  onOpen() {
    const { contentEl } = this;
    contentEl.empty();
    contentEl.addClass('bib-renamer-modal');

    // Title
    contentEl.createEl('h2', { text: 'Enrichissement & Renommage Bibliographique' });

    // Source Info Container
    const infoContainer = contentEl.createDiv({ cls: 'bib-info-container' });
    infoContainer.style.display = 'flex';
    infoContainer.style.gap = '8px';
    infoContainer.style.alignItems = 'center';
    infoContainer.style.marginBottom = '12px';

    const sourceBadge = infoContainer.createSpan({
      cls: `bib-source-badge ${this.editableMetadata.source}`,
      text: `Source: ${this.editableMetadata.source.toUpperCase()}`,
    });
    sourceBadge.style.padding = '2px 8px';
    sourceBadge.style.borderRadius = '4px';
    sourceBadge.style.fontSize = '0.8em';
    sourceBadge.style.background = 'var(--background-secondary-alt)';

    const confidenceBadge = infoContainer.createSpan({
      text: `Confiance: ${this.editableMetadata.confidence.toUpperCase()}`,
    });
    confidenceBadge.style.fontSize = '0.8em';
    confidenceBadge.style.color = 'var(--text-muted)';

    // Source vs Proposed Name Preview Box
    const previewBox = contentEl.createDiv({ cls: 'bib-preview-box' });
    previewBox.style.background = 'var(--background-secondary)';
    previewBox.style.padding = '12px';
    previewBox.style.borderRadius = '8px';
    previewBox.style.marginBottom = '16px';
    previewBox.style.border = '1px solid var(--background-modifier-border)';

    const originalRow = previewBox.createDiv();
    originalRow.style.fontSize = '0.85em';
    originalRow.style.color = 'var(--text-muted)';
    originalRow.createSpan({ text: 'Fichier source: ' });
    originalRow.createEl('code', { text: this.file.name });

    const proposedRow = previewBox.createDiv();
    proposedRow.style.marginTop = '6px';
    proposedRow.style.fontWeight = 'bold';
    proposedRow.createSpan({ text: '✦ Nouveau nom: ' });
    this.previewEl = proposedRow.createEl('code', { text: this.getProposedName() });
    this.previewEl.style.color = 'var(--text-accent)';

    // Alternative API results selector
    if (this.alternatives.length > 0) {
      new Setting(contentEl)
        .setName('Sources alternatives trouvées')
        .setDesc('Choisir un autre résultat de recherche API')
        .addDropdown((dropdown) => {
          dropdown.addOption('-1', `Principal (${this.metadata.source})`);
          this.alternatives.forEach((alt, idx) => {
            const label = `${alt.source} - ${alt.title ?? 'Sans titre'} (${alt.date ?? 's.d.'})`;
            dropdown.addOption(String(idx), label.substring(0, 45));
          });
          dropdown.setValue(String(this.selectedAlternativeIndex));
          dropdown.onChange((val) => {
            const idx = parseInt(val, 10);
            this.selectedAlternativeIndex = idx;
            const chosen = idx === -1 ? this.metadata : this.alternatives[idx];
            this.editableMetadata = { ...chosen, authors: [...chosen.authors] };
            this.authorsInputText = this.editableMetadata.authors.join('; ');
            this.onOpen(); // Re-render fields
          });
        });
    }

    // Editable fields
    new Setting(contentEl)
      .setName('Titre')
      .addText((text) => {
        text
          .setPlaceholder('Titre du document')
          .setValue(this.editableMetadata.title ?? '')
          .onChange((val) => {
            this.editableMetadata.title = val.trim() || null;
            this.updatePreview();
          });
        text.inputEl.style.width = '100%';
      });

    new Setting(contentEl)
      .setName('Auteur(s)')
      .setDesc('Séparer les auteurs par un point-virgule (;)')
      .addText((text) => {
        text
          .setPlaceholder('Nom Prénom; Auteur 2')
          .setValue(this.authorsInputText)
          .onChange((val) => {
            this.authorsInputText = val;
            this.editableMetadata.authors = val
              .split(';')
              .map((s) => s.trim())
              .filter(Boolean);
            this.updatePreview();
          });
        text.inputEl.style.width = '100%';
      });

    new Setting(contentEl)
      .setName('Date / Année')
      .addText((text) => {
        text
          .setPlaceholder('2024')
          .setValue(this.editableMetadata.date ?? '')
          .onChange((val) => {
            this.editableMetadata.date = val.trim() || null;
            this.updatePreview();
          });
      });

    new Setting(contentEl)
      .setName('Éditeur')
      .addText((text) => {
        text
          .setPlaceholder('Maison d’édition')
          .setValue(this.editableMetadata.publisher ?? '')
          .onChange((val) => {
            this.editableMetadata.publisher = val.trim() || null;
            this.updatePreview();
          });
      });

    new Setting(contentEl)
      .setName('ISBN')
      .addText((text) => {
        text
          .setPlaceholder('978...')
          .setValue(this.editableMetadata.isbn ?? '')
          .onChange((val) => {
            this.editableMetadata.isbn = val.trim() || null;
            this.updatePreview();
          });
      });

    new Setting(contentEl)
      .setName('DOI')
      .addText((text) => {
        text
          .setPlaceholder('10.xxxx/...')
          .setValue(this.editableMetadata.doi ?? '')
          .onChange((val) => {
            this.editableMetadata.doi = val.trim() || null;
            this.updatePreview();
          });
      });

    // Action buttons container
    const buttonContainer = contentEl.createDiv({ cls: 'bib-modal-buttons' });
    buttonContainer.style.display = 'flex';
    buttonContainer.style.justifyContent = 'flex-end';
    buttonContainer.style.gap = '10px';
    buttonContainer.style.marginTop = '20px';

    const cancelBtn = buttonContainer.createEl('button', { text: 'Annuler' });
    cancelBtn.onclick = () => this.close();

    const noteBtn = buttonContainer.createEl('button', {
      cls: 'mod-cta',
      text: 'Renommer + Créer la note 📝',
    });
    noteBtn.onclick = async () => {
      await this.executeRename(true);
    };

    const confirmBtn = buttonContainer.createEl('button', {
      cls: 'mod-cta',
      text: 'Renommer ✓',
    });
    confirmBtn.onclick = async () => {
      await this.executeRename(false);
    };
  }

  private getProposedName(): string {
    return applyTemplate(this.template, this.editableMetadata, this.file.extension);
  }

  private updatePreview() {
    if (this.previewEl) {
      this.previewEl.setText(this.getProposedName());
    }
  }

  private async executeRename(createNote: boolean) {
    const newName = this.getProposedName();
    const parentPath = this.file.parent?.path ?? '';
    const newPath = parentPath === '' || parentPath === '/'
      ? newName
      : `${parentPath}/${newName}`;

    try {
      // 1. Rename the binary book/article file in Vault
      if (this.file.path !== newPath) {
        await this.app.vault.rename(this.file, newPath);
        new Notice(`Fichier renommé en : ${newName}`);
      }

      let noteFile: TFile | undefined;

      // 2. Create Reading Note if requested
      if (createNote) {
        noteFile = await this.createReadingNote(newName);
      }

      this.close();
      this.onComplete(this.file, noteFile);
    } catch (err: any) {
      console.error('[obsidian-bib-renamer] Error renaming file:', err);
      new Notice(`Erreur lors du renommage : ${err.message || err}`);
    }
  }

  private async createReadingNote(bookFilename: string): Promise<TFile | undefined> {
    const title = this.editableMetadata.title ?? 'Sans titre';
    const authorsStr = this.editableMetadata.authors.join(', ');
    const stem = bookFilename.substring(0, bookFilename.lastIndexOf('.'));
    const noteName = `${stem}.md`;

    const folder = this.settings.readingNotesFolder?.trim() || (this.file.parent?.path ?? '');
    const notePath = folder === '' || folder === '/'
      ? noteName
      : `${folder}/${noteName}`;

    // Generate Frontmatter YAML
    const yaml = [
      '---',
      `title: "${title.replace(/"/g, '\\"')}"`,
      `authors: [${this.editableMetadata.authors.map((a) => `"${a.replace(/"/g, '\\"')}"`).join(', ')}]`,
      `date: ${this.editableMetadata.date ? `"${this.editableMetadata.date}"` : 'null'}`,
      `publisher: ${this.editableMetadata.publisher ? `"${this.editableMetadata.publisher.replace(/"/g, '\\"')}"` : 'null'}`,
      `isbn: ${this.editableMetadata.isbn ? `"${this.editableMetadata.isbn}"` : 'null'}`,
      `doi: ${this.editableMetadata.doi ? `"${this.editableMetadata.doi}"` : 'null'}`,
      `file: "[[${this.file.name}]]"`,
      'tags: [lecture, bibliographie]',
      'status: à lire',
      '---',
      '',
      `# ${title}`,
      '',
      `**Auteur(s)** : ${authorsStr || 'Inconnu'}  `,
      `**Publication** : ${this.editableMetadata.date || 's.d.'} ${this.editableMetadata.publisher ? `— ${this.editableMetadata.publisher}` : ''}  `,
      `**Fichier source** : [[${this.file.name}]]  `,
      '',
      '## Résumé & Idées clés',
      '',
      '- ',
      '',
      '## Citations & Notes',
      '',
      '> ',
      '',
    ].join('\n');

    try {
      const existing = this.app.vault.getAbstractFileByPath(notePath);
      if (existing instanceof TFile) {
        new Notice(`La note existe déjà : ${notePath}`);
        return existing;
      }

      const created = await this.app.vault.create(notePath, yaml);
      new Notice(`Note de lecture créée : ${noteName}`);
      return created;
    } catch (err: any) {
      console.error('[obsidian-bib-renamer] Error creating reading note:', err);
      new Notice(`Erreur création de note : ${err.message || err}`);
      return undefined;
    }
  }

  onClose() {
    const { contentEl } = this;
    contentEl.empty();
  }
}
