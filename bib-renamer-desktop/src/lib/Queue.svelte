<script lang="ts">
  import type { QueueEntry } from '$lib/types';
  import { getStatusLabel, getStatusClass } from '$lib/types';
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    entries: QueueEntry[];
    onSelectEntry: (entry: QueueEntry) => void;
    onClearAll: () => void;
  }

  let { entries, onSelectEntry, onClearAll }: Props = $props();

  // Statistiques de la queue
  let stats = $derived({
    total: entries.length,
    pending: entries.filter(e => e.status === 'pending').length,
    done: entries.filter(e => e.status === 'renamed').length,
    errors: entries.filter(e => typeof e.status === 'object').length,
    needsReview: entries.filter(e => e.status === 'needs_review').length,
  });

  function getExtClass(ext: string): string {
    return ext === 'epub' ? 'epub-tag' : 'pdf-tag';
  }

  function isClickable(entry: QueueEntry): boolean {
    return entry.status === 'metadata_found' ||
           entry.status === 'needs_review' ||
           entry.status === 'confirmed';
  }

  function hasError(entry: QueueEntry): boolean {
    return typeof entry.status === 'object';
  }

  function getErrorMessage(entry: QueueEntry): string {
    if (typeof entry.status === 'object' && 'error' in entry.status) {
      return entry.status.error;
    }
    return '';
  }
</script>

<div class="queue-panel">
  <!-- Stats header -->
  <div class="queue-stats">
    <div class="stat">
      <span class="stat-value">{stats.total}</span>
      <span class="stat-label">Fichiers</span>
    </div>
    <div class="stat">
      <span class="stat-value" style="color:var(--success)">{stats.done}</span>
      <span class="stat-label">Renommés</span>
    </div>
    <div class="stat">
      <span class="stat-value" style="color:var(--warning)">{stats.needsReview}</span>
      <span class="stat-label">À valider</span>
    </div>
    {#if stats.errors > 0}
      <div class="stat">
        <span class="stat-value" style="color:var(--error)">{stats.errors}</span>
        <span class="stat-label">Erreurs</span>
      </div>
    {/if}
    {#if stats.total > 0}
      <button class="btn btn-ghost btn-sm" onclick={onClearAll} style="margin-left:auto;" id="btn-clear-queue">
        Vider
      </button>
    {/if}
  </div>

  <!-- Progress bar -->
  {#if stats.total > 0}
    <div class="progress-track">
      <div
        class="progress-fill"
        style="width: {Math.round((stats.done / stats.total) * 100)}%"
      ></div>
    </div>
  {/if}

  <!-- Liste des fichiers -->
  <div class="queue-list">
    {#if entries.length === 0}
      <div class="empty-state" style="padding:2rem 1rem;">
        <div class="empty-state-icon">📂</div>
        <p style="font-size:0.82rem;">Glissez des fichiers EPUB/PDF<br>ou cliquez "Ajouter"</p>
      </div>
    {:else}
      {#each entries as entry (entry.id)}
        <div
          class="queue-item {isClickable(entry) ? 'clickable' : ''} {hasError(entry) ? 'has-error' : ''}"
          onclick={() => isClickable(entry) && onSelectEntry(entry)}
          role="row"
          tabindex="0"
          onkeydown={(e) => e.key === 'Enter' && isClickable(entry) && onSelectEntry(entry)}
          title={hasError(entry) ? getErrorMessage(entry) : entry.file_name}
        >
          <!-- Extension badge -->
          <span class="queue-item-ext {getExtClass(entry.extension)}">
            {entry.extension.toUpperCase()}
          </span>

          <!-- Nom du fichier -->
          <div class="queue-item-info">
            <div class="queue-item-name">
              {#if entry.status === 'renamed'}
                <span style="color:var(--accent-light);">{entry.file_name}</span>
              {:else}
                {entry.file_name}
              {/if}
            </div>
            {#if entry.metadata?.title}
              <div class="queue-item-meta">{entry.metadata.title}</div>
            {/if}
            {#if hasError(entry)}
              <div class="queue-item-error">{getErrorMessage(entry)}</div>
            {/if}
          </div>

          <!-- Statut -->
          <span class="badge {getStatusClass(entry.status)}">
            {#if entry.status === 'extracting'}
              <span class="dot">●</span>
            {/if}
            {#if entry.status === 'extracting'}
              <span class="spinner" style="width:10px;height:10px;border-width:1.5px;"></span>
            {/if}
            {getStatusLabel(entry.status)}
          </span>

          <!-- Flèche si cliquable -->
          {#if isClickable(entry)}
            <span style="color:var(--text-muted); font-size:0.75rem; flex-shrink:0;">›</span>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .queue-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .queue-stats {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .stat {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .stat-value {
    font-size: 1.1rem;
    font-weight: 700;
    line-height: 1;
    font-family: var(--font-mono);
  }

  .stat-label {
    font-size: 0.68rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .progress-track {
    height: 2px;
    background: var(--border);
    flex-shrink: 0;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), var(--accent-light));
    transition: width 0.4s ease;
    border-radius: 0 2px 2px 0;
  }

  .queue-list {
    flex: 1;
    overflow-y: auto;
  }

  .queue-item {
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    align-items: center;
    gap: 0.6rem;
    padding: 0.6rem 0.875rem;
    border-bottom: 1px solid var(--border);
    transition: background var(--transition-fast);
    cursor: default;
    min-height: 48px;
  }

  .queue-item.clickable {
    cursor: pointer;
  }

  .queue-item.clickable:hover,
  .queue-item.clickable:focus-visible {
    background: var(--bg-elevated);
    outline: none;
  }

  .queue-item.has-error {
    background: var(--error-bg);
  }

  .queue-item:last-child { border-bottom: none; }

  .queue-item-info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .queue-item-name {
    font-size: 0.8rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-primary);
  }

  .queue-item-meta {
    font-size: 0.72rem;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .queue-item-error {
    font-size: 0.7rem;
    color: var(--error);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Extension badges */
  .epub-tag {
    background: #7c3aed20;
    color: #a78bfa;
    border-color: #7c3aed30;
  }

  .pdf-tag {
    background: #dc262620;
    color: #fca5a5;
    border-color: #dc262630;
  }
</style>
