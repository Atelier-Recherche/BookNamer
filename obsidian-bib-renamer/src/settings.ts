import { App, PluginSettingTab, Setting } from 'obsidian';
import type BibRenamerPlugin from './main';
import { NamingTemplate, TEMPLATE_VARIABLES, DEFAULT_TEMPLATES } from './types';

export class BibRenamerSettingTab extends PluginSettingTab {
  plugin: BibRenamerPlugin;

  constructor(app: App, plugin: BibRenamerPlugin) {
    super(app, plugin);
    this.plugin = plugin;
  }

  display(): void {
    const { containerEl } = this;
    containerEl.empty();

    containerEl.createEl('h2', { text: 'Paramètres Bib Renamer' });

    // ─── Section Templates ───────────────────────────────────────────────
    containerEl.createEl('h3', { text: 'Modèles de renommage (Templates)' });

    const activeTemplate = this.plugin.settings.templates.find(
      (t) => t.id === this.plugin.settings.activeTemplateId
    ) ?? this.plugin.settings.templates[0];

    new Setting(containerEl)
      .setName('Modèle actif')
      .setDesc('Modèle appliqué par défaut lors du renommage')
      .addDropdown((dropdown) => {
        this.plugin.settings.templates.forEach((tpl) => {
          dropdown.addOption(tpl.id, tpl.name);
        });
        dropdown.setValue(activeTemplate.id);
        dropdown.onChange(async (val) => {
          this.plugin.settings.activeTemplateId = val;
          await this.plugin.saveSettings();
          this.display();
        });
      });

    // Edit pattern of active template
    new Setting(containerEl)
      .setName(`Motif du modèle : ${activeTemplate.name}`)
      .setDesc('Variables : {auteur-nom}, {auteur-prenom}, {auteur}, {titre}, {date}, {editeur}, {isbn}, {doi}, {extension}')
      .addText((text) => {
        text
          .setPlaceholder('{auteur-nom} {auteur-prenom} - {titre} ({date}) [{editeur}]')
          .setValue(activeTemplate.pattern)
          .onChange(async (val) => {
            activeTemplate.pattern = val;
            await this.plugin.saveSettings();
          });
        text.inputEl.style.width = '100%';
        text.inputEl.style.fontFamily = 'var(--font-monospace)';
      });

    // Add new custom template
    let newTemplateName = '';
    new Setting(containerEl)
      .setName('Ajouter un nouveau modèle')
      .addText((text) => {
        text.setPlaceholder('Ex: Format Archives').onChange((val) => {
          newTemplateName = val;
        });
      })
      .addButton((btn) => {
        btn.setButtonText('+ Ajouter').onClick(async () => {
          if (!newTemplateName.trim()) return;
          const newTpl: NamingTemplate = {
            id: `custom_${Date.now()}`,
            name: newTemplateName.trim(),
            pattern: '{auteur-nom} {auteur-prenom} - {titre} ({date})',
            is_default: false,
          };
          this.plugin.settings.templates.push(newTpl);
          this.plugin.settings.activeTemplateId = newTpl.id;
          await this.plugin.saveSettings();
          this.display();
        });
      });

    // Reset templates button
    new Setting(containerEl)
      .setName('Réinitialiser les modèles par défaut')
      .addButton((btn) => {
        btn.setButtonText('Réinitialiser').onClick(async () => {
          this.plugin.settings.templates = JSON.parse(JSON.stringify(DEFAULT_TEMPLATES));
          this.plugin.settings.activeTemplateId = 'default';
          await this.plugin.saveSettings();
          this.display();
        });
      });

    // ─── Section APIs ────────────────────────────────────────────────────
    containerEl.createEl('h3', { text: 'Sources de métadonnées & APIs' });

    new Setting(containerEl)
      .setName('OpenLibrary')
      .setDesc('Recherche par ISBN & recherche bibliographique (gratuit, sans clé)')
      .addToggle((toggle) => {
        toggle
          .setValue(this.plugin.settings.enableOpenLibrary)
          .onChange(async (val) => {
            this.plugin.settings.enableOpenLibrary = val;
            await this.plugin.saveSettings();
          });
      });

    new Setting(containerEl)
      .setName('BNF (Bibliothèque Nationale de France)')
      .setDesc('Catalogue officiel français pour tous les ISBN francophones (gratuit, sans clé)')
      .addToggle((toggle) => {
        toggle
          .setValue(this.plugin.settings.enableBnf)
          .onChange(async (val) => {
            this.plugin.settings.enableBnf = val;
            await this.plugin.saveSettings();
          });
      });

    new Setting(containerEl)
      .setName('Amazon')
      .setDesc('Recherche de fiches produits par ISBN (Amazon.fr / Amazon.com)')
      .addToggle((toggle) => {
        toggle
          .setValue(this.plugin.settings.enableAmazon)
          .onChange(async (val) => {
            this.plugin.settings.enableAmazon = val;
            await this.plugin.saveSettings();
          });
      });

    new Setting(containerEl)
      .setName('Crossref')
      .setDesc('Recherche par DOI & recherche d’articles académiques (gratuit, sans clé)')
      .addToggle((toggle) => {
        toggle
          .setValue(this.plugin.settings.enableCrossref)
          .onChange(async (val) => {
            this.plugin.settings.enableCrossref = val;
            await this.plugin.saveSettings();
          });
      });

    new Setting(containerEl)
      .setName('Google Books')
      .setDesc('Recherche Google Books (clé API optionnelle)')
      .addToggle((toggle) => {
        toggle
          .setValue(this.plugin.settings.enableGoogleBooks)
          .onChange(async (val) => {
            this.plugin.settings.enableGoogleBooks = val;
            await this.plugin.saveSettings();
            this.display();
          });
      });

    if (this.plugin.settings.enableGoogleBooks) {
      new Setting(containerEl)
        .setName('Clé API Google Books')
        .setDesc('Clé API Google Cloud Console')
        .addText((text) => {
          text
            .setPlaceholder('AIza...')
            .setValue(this.plugin.settings.googleBooksApiKey)
            .onChange(async (val) => {
              this.plugin.settings.googleBooksApiKey = val;
              await this.plugin.saveSettings();
            });
          text.inputEl.type = 'password';
        });
    }

    // ─── Section Notes de Lecture ─────────────────────────────────────────
    containerEl.createEl('h3', { text: 'Notes de lecture associées' });

    new Setting(containerEl)
      .setName('Dossier des notes de lecture')
      .setDesc('Dossier de destination (laisser vide pour créer à côté du fichier)')
      .addText((text) => {
        text
          .setPlaceholder('Notes/Lectures')
          .setValue(this.plugin.settings.readingNotesFolder)
          .onChange(async (val) => {
            this.plugin.settings.readingNotesFolder = val.trim();
            await this.plugin.saveSettings();
          });
      });
  }
}
