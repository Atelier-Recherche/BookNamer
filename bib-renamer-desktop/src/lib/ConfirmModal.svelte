<script lang="ts">
  import type { QueueEntry, BookMetadata, NamingTemplate, AppConfig } from '$lib/types';
  import { getStatusLabel, getStatusClass, getConfidenceLabel, getSourceLabel } from '$lib/types';
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    entry: QueueEntry;
    template: NamingTemplate;
    config: AppConfig;
    onConfirm: (entry: QueueEntry) => void;
    onCancel: () => void;
  }

  let { entry, template, config, onConfirm, onCancel }: Props = $props();

  // Copie éditable des métadonnées synchronisée sur entry
  let meta = $state<BookMetadata>({
    title: null,
    authors: [],
    date: null,
    publisher: null,
    isbn: null,
    doi: null,
    source: 'unknown',
    confidence: 'low',
  });

  let authorsString = $state('');
  let selectedAlt = $state(-1); // -1 = résultat principal

  // Recherche en ligne
  let searchQuery = $state('');
  let isSearching = $state(false);
  let searchResults = $state<BookMetadata[]>([]);
  let searchError = $state<string | null>(null);

  $effect(() => {
    meta = {
      title: entry.metadata?.title ?? null,
      authors: [...(entry.metadata?.authors ?? [])],
      date: entry.metadata?.date ?? null,
      publisher: entry.metadata?.publisher ?? null,
      isbn: entry.metadata?.isbn ?? null,
      doi: entry.metadata?.doi ?? null,
      source: entry.metadata?.source ?? 'unknown',
      confidence: entry.metadata?.confidence ?? 'low',
    };
    authorsString = (entry.metadata?.authors ?? []).join('; ');
    selectedAlt = -1;

    // Pré-remplir la requête de recherche avec titre / auteur ou nom de fichier
    const t = entry.metadata?.title || '';
    const rawAuthor = entry.metadata?.authors?.[0] || '';
    const a = rawAuthor.includes(',')
      ? rawAuthor.split(',').map(p => p.trim()).filter(Boolean).reverse().join(' ')
      : rawAuthor;
    searchQuery = (t || a) ? `${t} ${a}`.trim() : entry.file_name.replace(/\.[^/.]+$/, '').replace(/--|-|_/g, ' ');
    searchResults = [];
    searchError = null;
  });

  // Prévisualisation du nom — lit explicitement chaque champ pour suivre les edits
  let previewName = $derived.by(() => {
    const authors = authorsString.split(';').map(s => s.trim()).filter(Boolean);
    return computePreview(
      {
        title: meta.title,
        authors,
        date: meta.date,
        publisher: meta.publisher,
        isbn: meta.isbn,
        doi: meta.doi,
        source: meta.source,
        confidence: meta.confidence,
      },
      template,
      entry.extension,
    );
  });

  function computePreview(m: BookMetadata, tpl: NamingTemplate, ext: string): string {
    const rawAuteur = m.authors[0] ?? 'Auteur inconnu';
    const { nom, prenom } = m.authors[0]
      ? parseAuthorParts(m.authors[0])
      : { nom: 'Auteur inconnu', prenom: '' };
    const titre = m.title ?? 'Sans titre';
    const date = m.date ?? 's.d.';
    const editeur = m.publisher ?? '';
    const isbn = m.isbn ?? '';
    const doi = m.doi ?? '';

    let result = tpl.pattern
      .replaceAll('{auteur-nom}', nom)
      .replaceAll('{auteur-prenom}', prenom)
      .replaceAll('{auteur}', rawAuteur)
      .replaceAll('{titre}', titre)
      .replaceAll('{date}', date)
      .replaceAll('{extension}', ext);

    result = replaceOptional(result, '{editeur}', editeur);
    result = replaceOptional(result, '{isbn}', isbn);
    result = replaceOptional(result, '{doi}', doi);

    // Nettoyage crochets vides / espaces
    result = result.replace(/\[\s*\]|\(\s*\)/g, '').replace(/\s{2,}/g, ' ').trim();

    // Sanitisation caractères interdits
    result = result.replace(/[/\\:*?"<>|]/g, '').trim();

    return `${result}.${ext}`;
  }

  function replaceOptional(tpl: string, variable: string, value: string): string {
    if (!value) {
      return tpl
        .replaceAll(`[${variable}]`, '')
        .replaceAll(`(${variable})`, '')
        .replaceAll(variable, '');
    }
    return tpl.replaceAll(variable, value);
  }

  /** Découpe « Nom, Prénom » ou « Prénom Nom ». */
  function parseAuthorParts(name: string): { nom: string; prenom: string } {
    const trimmed = name.trim();
    if (!trimmed) return { nom: '', prenom: '' };
    if (trimmed.includes(',')) {
      const [nom, ...rest] = trimmed.split(',');
      return { nom: nom.trim(), prenom: rest.join(',').trim() };
    }
    const parts = trimmed.split(/\s+/);
    if (parts.length === 1) return { nom: parts[0], prenom: '' };
    return { nom: parts[parts.length - 1], prenom: parts.slice(0, -1).join(' ') };
  }

  function applyAlternative(idx: number) {
    const alt = idx === -1 ? entry.metadata : entry.alternative_results[idx];
    if (!alt) return;
    applyMetadata(alt);
    selectedAlt = idx;
  }

  function applyMetadata(newMeta: BookMetadata) {
    meta.title = newMeta.title;
    meta.authors = [...newMeta.authors];
    authorsString = newMeta.authors.join('; ');
    meta.date = newMeta.date;
    meta.publisher = newMeta.publisher;
    meta.isbn = newMeta.isbn;
    meta.doi = newMeta.doi;
    meta.source = newMeta.source;
    meta.confidence = newMeta.confidence;
  }

  async function handleSearchOnline() {
    if (!searchQuery.trim() && !meta.isbn?.trim()) return;
    isSearching = true;
    searchError = null;

    try {
      const authorClean = authorsString
        .split(';')
        .map(s => s.trim())
        .filter(Boolean)
        .map(a => a.includes(',') ? a.split(',').map(p => p.trim()).reverse().join(' ') : a)
        .join(' ');

      // Si la requête est encore le préremplissage titre+auteur, normaliser "Nom, Prénom"
      let query = searchQuery.trim();
      if (!query && meta.title) {
        query = `${meta.title} ${authorClean}`.trim();
      }

      const results: BookMetadata[] = await invoke('search_online', {
        query,
        config,
        isbn: meta.isbn?.trim() || null,
      });

      searchResults = results;
      if (results.length === 0) {
        searchError = 'Aucun résultat trouvé pour cette recherche.';
      }
    } catch (e: any) {
      console.error('search_online error:', e);
      searchError = `Erreur lors de la recherche : ${e}`;
    } finally {
      isSearching = false;
    }
  }

  function handleAuthorsChange() {
    meta.authors = authorsString.split(';').map(s => s.trim()).filter(Boolean);
  }

  function handleConfirm() {
    // Synchroniser les auteurs depuis le champ texte juste avant confirm
    const authors = authorsString.split(';').map(s => s.trim()).filter(Boolean);
    meta.authors = authors;

    const updatedEntry: QueueEntry = {
      ...entry,
      metadata: {
        title: meta.title?.trim() || null,
        authors,
        date: meta.date?.trim() || null,
        publisher: meta.publisher?.trim() || null,
        isbn: meta.isbn?.trim() || null,
        doi: meta.doi?.trim() || null,
        source: meta.source,
        confidence: meta.confidence,
      },
      proposed_name: previewName,
      status: 'confirmed',
    };
    onConfirm(updatedEntry);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onCancel();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Backdrop -->
<div
  class="modal-backdrop fade-in"
  onclick={onCancel}
  onkeydown={(e) => e.key === 'Escape' && onCancel()}
  role="button"
  tabindex="-1"
  aria-label="Fermer la modal"
>
  <!-- Modal -->
  <div
    class="modal animate-in"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
    aria-label="Confirmation du renommage"
    tabindex="0"
  >

    <!-- Header -->
    <div class="modal-header">
      <div style="flex:1; min-width:0;">
        <h2 style="margin-bottom:0.3rem;">Confirmation & Enrichissement</h2>
        <div style="display:flex; align-items:center; gap:0.5rem; flex-wrap:wrap;">
          <span class="badge {getStatusClass(entry.status)}">{getStatusLabel(entry.status)}</span>
          <span class="source-tag {meta.source}">{getSourceLabel(meta.source)}</span>
          <div class="confidence">
            <div class="confidence-bar {meta.confidence}">
              <div class="seg"></div>
              <div class="seg"></div>
              <div class="seg"></div>
            </div>
            <span style="color:var(--text-muted); font-size:0.75rem;">
              Confiance : {getConfidenceLabel(meta.confidence)}
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- Body -->
    <div class="modal-body">

      <!-- Nom original / proposé -->
      <div class="name-comparison">
        <div class="name-box">
          <div class="name-box-label">Fichier source</div>
          <div class="name-box-value">{entry.file_name}</div>
        </div>
        <div class="name-box proposed">
          <div class="name-box-label">✦ Nouveau nom</div>
          <div class="name-box-value">{previewName}</div>
        </div>
      </div>

      <!-- ─── Barre de Recherche en Ligne (Titre / Auteur / Mots-clés) ─── -->
      <div class="card-elevated" style="padding:0.75rem 1rem; border-color:var(--accent-glow);">
        <label for="search-input" style="display:block; margin-bottom:0.4rem; color:var(--text-accent);">
          🔍 Relancer la recherche en ligne (Titre / Auteur / ISBN)
        </label>
        <div style="display:flex; gap:0.5rem;">
          <input
            id="search-input"
            type="text"
            bind:value={searchQuery}
            placeholder="Ex: La vérité nue Hans Blumenberg..."
            onkeydown={(e) => e.key === 'Enter' && handleSearchOnline()}
          />
          <button
            class="btn btn-primary btn-sm"
            onclick={handleSearchOnline}
            disabled={isSearching || (!searchQuery.trim() && !meta.isbn?.trim())}
          >
            {#if isSearching}
              <span class="spinner" style="width:12px;height:12px;"></span>
              Recherche…
            {:else}
              Rechercher
            {/if}
          </button>
        </div>

        {#if searchError}
          <div style="font-size:0.78rem; color:var(--error); margin-top:0.4rem;">
            {searchError}
          </div>
        {/if}

        <!-- Résultats de la recherche en ligne -->
        {#if searchResults.length > 0}
          <div style="margin-top:0.75rem; display:flex; flex-direction:column; gap:0.4rem; max-height:160px; overflow-y:auto;">
            <div style="font-size:0.72rem; text-transform:uppercase; color:var(--text-muted); font-weight:600;">
              {searchResults.length} résultat(s) trouvé(s) — Cliquez pour appliquer :
            </div>
            {#each searchResults as res, i}
              <div
                class="search-result-item"
                onclick={() => applyMetadata(res)}
                role="button"
                tabindex="0"
                onkeydown={(e) => e.key === 'Enter' && applyMetadata(res)}
              >
                <div style="display:flex; align-items:center; justify-content:space-between; gap:0.5rem;">
                  <span style="font-weight:600; font-size:0.82rem; color:var(--text-primary);">
                    {res.title ?? 'Sans titre'}
                  </span>
                  <span class="source-tag {res.source}">{getSourceLabel(res.source)}</span>
                </div>
                <div style="font-size:0.75rem; color:var(--text-secondary); margin-top:2px;">
                  {res.authors.join(', ') || 'Auteur inconnu'} {res.date ? `(${res.date})` : ''} {res.publisher ? `— ${res.publisher}` : ''}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Alternatives API automatiques -->
      {#if entry.alternative_results.length > 0}
        <div>
          <div style="font-size:0.8rem; font-weight:500; color:var(--text-secondary); text-transform:uppercase; margin-bottom:0.4rem;">
            Alternatives issues de l'extraction initiale
          </div>
          <div style="display:flex; gap:0.4rem; flex-wrap:wrap;">
            <button
              class="btn btn-sm {selectedAlt === -1 ? 'btn-primary' : 'btn-secondary'}"
              onclick={() => applyAlternative(-1)}
            >
              Principal ({getSourceLabel(entry.metadata?.source ?? 'unknown')})
            </button>
            {#each entry.alternative_results as alt, i}
              <button
                class="btn btn-sm {selectedAlt === i ? 'btn-primary' : 'btn-secondary'}"
                onclick={() => applyAlternative(i)}
              >
                {getSourceLabel(alt.source)} #{i + 1}
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <hr />

      <!-- Champs éditables -->
      <div style="display:grid; grid-template-columns:1fr 1fr; gap:0.75rem;">
        <div class="form-group" style="grid-column:1/-1;">
          <label for="field-titre">Titre</label>
          <input
            id="field-titre"
            type="text"
            value={meta.title ?? ''}
            oninput={(e) => { meta.title = e.currentTarget.value || null; }}
            placeholder="Titre du livre / article"
          />
        </div>

        <div class="form-group" style="grid-column:1/-1;">
          <label for="field-auteur">Auteur(s) <span style="font-weight:400; text-transform:none;">(séparés par ;)</span></label>
          <input
            id="field-auteur"
            type="text"
            bind:value={authorsString}
            oninput={handleAuthorsChange}
            placeholder="Prénom Nom; Prénom2 Nom2"
          />
        </div>

        <div class="form-group">
          <label for="field-date">Année</label>
          <input
            id="field-date"
            type="text"
            value={meta.date ?? ''}
            oninput={(e) => { meta.date = e.currentTarget.value || null; }}
            placeholder="2022"
          />
        </div>

        <div class="form-group">
          <label for="field-editeur">Éditeur</label>
          <input
            id="field-editeur"
            type="text"
            value={meta.publisher ?? ''}
            oninput={(e) => { meta.publisher = e.currentTarget.value || null; }}
            placeholder="Seuil, Gallimard…"
          />
        </div>

        <div class="form-group">
          <label for="field-isbn">ISBN</label>
          <input
            id="field-isbn"
            type="text"
            value={meta.isbn ?? ''}
            oninput={(e) => { meta.isbn = e.currentTarget.value || null; }}
            placeholder="9780000000000"
            style="font-family:var(--font-mono);"
          />
        </div>

        <div class="form-group">
          <label for="field-doi">DOI</label>
          <input
            id="field-doi"
            type="text"
            value={meta.doi ?? ''}
            oninput={(e) => { meta.doi = e.currentTarget.value || null; }}
            placeholder="10.xxxx/xxxx"
            style="font-family:var(--font-mono);"
          />
        </div>
      </div>

    </div>

    <!-- Footer -->
    <div class="modal-footer">
      <button class="btn btn-ghost" onclick={onCancel} id="btn-cancel">
        Annuler
      </button>
      <button class="btn btn-primary btn-lg" onclick={handleConfirm} id="btn-confirm">
        ✓ Confirmer (↵)
      </button>
    </div>

  </div>
</div>

<style>
  .search-result-item {
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 0.5rem 0.75rem;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .search-result-item:hover,
  .search-result-item:focus-visible {
    border-color: var(--accent);
    background: var(--bg-hover);
    outline: none;
  }
</style>
