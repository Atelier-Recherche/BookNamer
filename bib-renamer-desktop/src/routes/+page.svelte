<script lang="ts">
  import '../app.css';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import Queue from '$lib/Queue.svelte';
  import ConfirmModal from '$lib/ConfirmModal.svelte';
  import TemplateEditor from '$lib/TemplateEditor.svelte';
  import type { QueueEntry, AppConfig, NamingTemplate } from '$lib/types';
  import { DEFAULT_CONFIG } from '$lib/types';
  import { onMount } from 'svelte';

  // ─── État global ──────────────────────────────────────────────────────────
  let queue = $state<QueueEntry[]>([]);
  let config = $state<AppConfig>({ ...DEFAULT_CONFIG });
  let selectedEntry = $state<QueueEntry | null>(null);
  let activeTab = $state<'queue' | 'templates' | 'settings'>('queue');
  let isDragOver = $state(false);
  let isProcessing = $state(false);

  // Template actif
  let activeTemplate = $derived(
    config.templates.find(t => t.id === config.active_template_id)
    ?? config.templates[0]
  );

  // ─── Lifecycle ────────────────────────────────────────────────────────────
  onMount(() => {
    // 1. Charger la config
    invoke('load_config')
      .then((savedConfig: any) => {
        if (savedConfig) config = savedConfig;
      })
      .catch((e) => {
        console.warn('load_config error (using defaults):', e);
        config = { ...DEFAULT_CONFIG };
      });

    // 2. Écouter le Drag & Drop natif Tauri (fichiers/dossiers depuis l'explorateur OS)
    let unlistenDragDrop: (() => void) | null = null;
    getCurrentWebviewWindow()
      .onDragDropEvent((event) => {
        if (event.payload.type === 'over') {
          isDragOver = true;
        } else if (event.payload.type === 'drop') {
          isDragOver = false;
          if (event.payload.paths && event.payload.paths.length > 0) {
            addPaths(event.payload.paths);
          }
        } else {
          isDragOver = false;
        }
      })
      .then((unlisten) => {
        unlistenDragDrop = unlisten;
      })
      .catch((err) => {
        console.error('Error attaching onDragDropEvent listener:', err);
      });

    return () => {
      if (unlistenDragDrop) unlistenDragDrop();
    };
  });

  // ─── Drag & Drop Fallback HTML5 ───────────────────────────────────────────
  function onDragOver(e: DragEvent) {
    e.preventDefault();
    isDragOver = true;
  }

  function onDragLeave() {
    isDragOver = false;
  }

  async function onDrop(e: DragEvent) {
    e.preventDefault();
    isDragOver = false;
  }

  // ─── Ajout de fichiers via dialog ─────────────────────────────────────────
  async function openFileDialog() {
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: 'Livres & Articles', extensions: ['epub', 'pdf'] }],
        directory: false,
      });
      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      await addPaths(paths);
    } catch (err) {
      console.error('openFileDialog error:', err);
      alert(`Erreur lors de l'ouverture des fichiers: ${err}`);
    }
  }

  async function openFolderDialog() {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected as string];
      await addPaths(paths);
    } catch (err) {
      console.error('openFolderDialog error:', err);
      alert(`Erreur lors de l'ouverture du dossier: ${err}`);
    }
  }

  async function addPaths(paths: string[]) {
    for (const p of paths) {
      try {
        const entries: QueueEntry[] = await invoke('scan_path', { path: p });
        // Éviter les doublons
        const existingIds = new Set(queue.map(e => e.id));
        const newEntries = entries.filter(e => !existingIds.has(e.id));
        queue = [...queue, ...newEntries];
      } catch (err) {
        console.error('scan_path error:', err);
      }
    }
    // Lancer le traitement automatique
    await processQueue();
  }

  // ─── Traitement de la queue ───────────────────────────────────────────────
  async function processQueue() {
    if (isProcessing) return;
    isProcessing = true;

    const pending = queue.filter(e => e.status === 'pending');

    for (const entry of pending) {
      // Mise à jour statut
      updateEntry({ ...entry, status: 'extracting' });

      try {
        // 1. Extraction métadonnées
        let updated: QueueEntry = await invoke('extract_metadata', { entry });
        updateEntry(updated);

        // 2. Enrichissement API
        updated = await invoke('enrich_entry', { entry: updated, config });
        updateEntry(updated);

        // 3. Génération du nom proposé
        if (updated.metadata) {
          updated = await invoke('generate_proposed_name', {
            entry: updated,
            template: activeTemplate,
          });
          updateEntry(updated);
        }

      } catch (err) {
        updateEntry({
          ...entry,
          status: { error: String(err) },
        });
      }
    }

    isProcessing = false;
  }

  function updateEntry(updated: QueueEntry) {
    queue = queue.map(e => e.id === updated.id ? updated : e);
  }

  // ─── Confirmation / Renommage ─────────────────────────────────────────────
  function handleSelectEntry(entry: QueueEntry) {
    selectedEntry = entry;
  }

  async function handleConfirm(entry: QueueEntry) {
    selectedEntry = null;
    updateEntry({ ...entry, status: 'confirmed' });

    try {
      const renamed: QueueEntry = await invoke('rename_file', { entry });
      updateEntry(renamed);
    } catch (err) {
      updateEntry({ ...entry, status: { error: String(err) } });
    }
  }

  function handleCancel() {
    selectedEntry = null;
  }

  // ─── Actions batch ────────────────────────────────────────────────────────
  async function confirmAll() {
    const toConfirm = queue.filter(
      e => e.status === 'metadata_found' || e.status === 'confirmed'
    );
    for (const entry of toConfirm) {
      if (!entry.proposed_name) continue;
      try {
        const renamed: QueueEntry = await invoke('rename_file', { entry });
        updateEntry(renamed);
      } catch (err) {
        updateEntry({ ...entry, status: { error: String(err) } });
      }
    }
  }

  function clearQueue() {
    queue = [];
  }

  // ─── Config ───────────────────────────────────────────────────────────────
  function handleTemplatesUpdate(templates: NamingTemplate[], activeId: string) {
    config = { ...config, templates, active_template_id: activeId };
    saveConfig();
  }

  async function saveConfig() {
    try {
      await invoke('save_config', { config });
    } catch (err) {
      console.error('save_config error:', err);
    }
  }

  // Statistiques pour le bouton "Tout confirmer"
  let readyCount = $derived(
    queue.filter(e => e.status === 'metadata_found' && e.proposed_name).length
  );
</script>

<div class="app-shell">
  <!-- ─── Header ────────────────────────────────────────────────────────── -->
  <header class="app-header">
    <div class="logo">
      <div class="logo-icon">📚</div>
      <span>BookNamer</span>
    </div>

    <div class="tabs" style="margin-left:1.5rem;">
      <button class="tab {activeTab === 'queue' ? 'active' : ''}" onclick={() => activeTab = 'queue'} id="tab-queue">
        File d'attente {queue.length > 0 ? `(${queue.length})` : ''}
      </button>
      <button class="tab {activeTab === 'templates' ? 'active' : ''}" onclick={() => activeTab = 'templates'} id="tab-templates">
        Templates
      </button>
      <button class="tab {activeTab === 'settings' ? 'active' : ''}" onclick={() => activeTab = 'settings'} id="tab-settings">
        Paramètres
      </button>
    </div>

    <div class="header-actions">
      {#if readyCount > 0}
        <button class="btn btn-primary btn-sm" onclick={confirmAll} id="btn-confirm-all">
          ✓ Tout renommer ({readyCount})
        </button>
      {/if}
      <button class="btn btn-secondary btn-sm" onclick={openFolderDialog} id="btn-open-folder">
        📁 Dossier
      </button>
      <button class="btn btn-primary btn-sm" onclick={openFileDialog} id="btn-add-files">
        + Ajouter
      </button>
    </div>
  </header>

  <!-- ─── Sidebar (Queue) ──────────────────────────────────────────────── -->
  <aside class="app-sidebar">
    <Queue
      entries={queue}
      onSelectEntry={handleSelectEntry}
      onClearAll={clearQueue}
    />
  </aside>

  <!-- ─── Main Content ─────────────────────────────────────────────────── -->
  <main class="app-main">
    {#if activeTab === 'queue'}
      <!-- Drop Zone -->
      <div
        class="drop-zone {isDragOver ? 'drag-over' : ''}"
        ondragover={onDragOver}
        ondragleave={onDragLeave}
        ondrop={onDrop}
        onclick={openFileDialog}
        role="button"
        tabindex="0"
        onkeydown={(e) => e.key === 'Enter' && openFileDialog()}
        id="drop-zone"
        aria-label="Zone de dépôt de fichiers EPUB et PDF"
      >
        <div style="position:relative; z-index:1;">
          <div style="font-size:3rem; margin-bottom:0.75rem;">
            {isDragOver ? '📂' : '📚'}
          </div>
          <h2 style="margin-bottom:0.5rem; color:var(--text-primary);">
            {isDragOver ? 'Relâchez pour ajouter' : 'Glissez vos fichiers ici'}
          </h2>
          <p style="color:var(--text-secondary); font-size:0.85rem;">
            Fichiers <code>.epub</code> et <code>.pdf</code> — drag & drop ou cliquez pour parcourir
          </p>
          <div style="margin-top:1.25rem; display:flex; gap:0.75rem; justify-content:center;">
            <button
              class="btn btn-primary"
              onclick={(e) => { e.stopPropagation(); openFileDialog(); }}
              id="btn-drop-files"
            >
              Choisir des fichiers
            </button>
            <button
              class="btn btn-secondary"
              onclick={(e) => { e.stopPropagation(); openFolderDialog(); }}
              id="btn-drop-folder"
            >
              Choisir un dossier
            </button>
          </div>
        </div>
      </div>

      <!-- Statut du traitement -->
      {#if isProcessing}
        <div class="card-elevated animate-in" style="margin-top:1rem; padding:0.875rem 1rem; display:flex; align-items:center; gap:0.75rem;">
          <div class="spinner"></div>
          <span style="font-size:0.85rem;">Extraction et enrichissement en cours…</span>
        </div>
      {/if}

      <!-- Résumé des métadonnées du fichier sélectionné dans la queue -->
      {#if queue.length > 0}
        <div style="margin-top:1.5rem;">
          <div style="display:flex; align-items:center; justify-content:space-between; margin-bottom:0.75rem;">
            <h2>Template actif</h2>
            <button class="btn btn-ghost btn-sm" onclick={() => activeTab = 'templates'}>
              Modifier →
            </button>
          </div>
          <div class="card-elevated" style="padding:0.875rem 1rem;">
            <div style="font-size:0.78rem; color:var(--text-muted); margin-bottom:0.3rem;">
              {activeTemplate.name}
            </div>
            <code style="font-size:0.85rem; color:var(--accent-light);">
              {activeTemplate.pattern}.{'{extension}'}
            </code>
            <div class="var-chips" style="margin-top:0.6rem;">
              {#each activeTemplate.pattern.match(/\{[^}]+\}/g) ?? [] as v}
                <span class="var-chip" style="cursor:default;">{v}</span>
              {/each}
            </div>
          </div>
        </div>
      {/if}

    {:else if activeTab === 'templates'}
      <div style="max-width:680px;">
        <TemplateEditor
          templates={config.templates}
          activeId={config.active_template_id}
          onUpdate={handleTemplatesUpdate}
        />
      </div>

    {:else if activeTab === 'settings'}
      <div style="max-width:560px;" class="animate-in">
        <h2 style="margin-bottom:1.25rem;">Paramètres</h2>

        <div class="card" style="padding:1.25rem;">
          <h3 style="margin-bottom:1rem;">Sources API</h3>
          <div class="settings-section">

            <div class="settings-row">
              <div>
                <div style="font-size:0.875rem; font-weight:500;">OpenLibrary</div>
                <div style="font-size:0.78rem; color:var(--text-muted);">Livres via ISBN — gratuit, sans clé</div>
              </div>
              <label class="toggle">
                <input type="checkbox" bind:checked={config.enabled_sources.openlibrary} onchange={saveConfig} />
                <span class="toggle-slider"></span>
              </label>
            </div>

            <hr />

            <div class="settings-row">
              <div>
                <div style="font-size:0.875rem; font-weight:500;">Crossref</div>
                <div style="font-size:0.78rem; color:var(--text-muted);">Articles via DOI — gratuit, sans clé</div>
              </div>
              <label class="toggle">
                <input type="checkbox" bind:checked={config.enabled_sources.crossref} onchange={saveConfig} />
                <span class="toggle-slider"></span>
              </label>
            </div>

            <div class="settings-row">
              <div>
                <div style="font-size:0.875rem; font-weight:500;">BNF (Bibliothèque Nationale de France)</div>
                <div style="font-size:0.78rem; color:var(--text-muted);">Livres francophones via ISBN — gratuit, sans clé</div>
              </div>
              <label class="toggle">
                <input type="checkbox" bind:checked={config.enabled_sources.bnf} onchange={saveConfig} />
                <span class="toggle-slider"></span>
              </label>
            </div>

            <hr />

            <div class="settings-row">
              <div>
                <div style="font-size:0.875rem; font-weight:500;">Amazon (Recherche Produit)</div>
                <div style="font-size:0.78rem; color:var(--text-muted);">Recherche rapide par ISBN (Amazon.fr / Amazon.com)</div>
              </div>
              <label class="toggle">
                <input type="checkbox" bind:checked={config.enabled_sources.amazon} onchange={saveConfig} />
                <span class="toggle-slider"></span>
              </label>
            </div>

            <hr />

            <div class="settings-row">
              <div>
                <div style="font-size:0.875rem; font-weight:500;">Google Books</div>
                <div style="font-size:0.78rem; color:var(--text-muted);">Recherche Google Books — clé API optionnelle</div>
              </div>
              <label class="toggle">
                <input type="checkbox" bind:checked={config.enabled_sources.google_books} onchange={saveConfig} />
                <span class="toggle-slider"></span>
              </label>
            </div>

            {#if config.enabled_sources.google_books}
              <div class="form-group animate-in" style="margin-top:0.25rem;">
                <label for="gbooks-key">Clé API Google Books</label>
                <input
                  id="gbooks-key"
                  type="text"
                  bind:value={config.google_books_api_key}
                  placeholder="AIza..."
                  onchange={saveConfig}
                  style="font-family:var(--font-mono);"
                />
              </div>
            {/if}

          </div>
        </div>

        <div class="card" style="padding:1.25rem; margin-top:1rem;">
          <h3 style="margin-bottom:0.5rem;">À propos</h3>
          <p style="font-size:0.82rem; color:var(--text-secondary); line-height:1.6;">
            BookNamer enrichit automatiquement vos fichiers EPUB et PDF via OpenLibrary, Crossref et Google Books,
            puis les renomme selon vos templates personnalisés.
          </p>
          <div style="margin-top:0.75rem; font-size:0.78rem; color:var(--text-muted); font-family:var(--font-mono);">
            v0.1.0 · Tauri v2 · Svelte 5
          </div>
        </div>
      </div>
    {/if}
  </main>
</div>

<!-- ─── Modal de confirmation ─────────────────────────────────────────── -->
{#if selectedEntry}
  <ConfirmModal
    entry={selectedEntry}
    template={activeTemplate}
    config={config}
    onConfirm={handleConfirm}
    onCancel={handleCancel}
  />
{/if}
