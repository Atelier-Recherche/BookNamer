<script lang="ts">
  import type { NamingTemplate } from '$lib/types';
  import { TEMPLATE_VARIABLES, DEFAULT_CONFIG } from '$lib/types';

  interface Props {
    templates: NamingTemplate[];
    activeId: string;
    onUpdate: (templates: NamingTemplate[], activeId: string) => void;
  }

  let { templates, activeId, onUpdate }: Props = $props();

  let editingId = $state<string | null>(null);
  let editPattern = $state('');
  let editName = $state('');
  let newTemplateName = $state('');

  function startEdit(tpl: NamingTemplate) {
    editingId = tpl.id;
    editPattern = tpl.pattern;
    editName = tpl.name;
  }

  function saveEdit() {
    if (!editingId) return;
    const updated = templates.map(t =>
      t.id === editingId
        ? { ...t, name: editName, pattern: editPattern }
        : t
    );
    onUpdate(updated, activeId);
    editingId = null;
  }

  function cancelEdit() {
    editingId = null;
  }

  function deleteTemplate(id: string) {
    if (templates.length <= 1) return; // Garder au moins un
    const filtered = templates.filter(t => t.id !== id);
    const newActive = id === activeId ? filtered[0].id : activeId;
    onUpdate(filtered, newActive);
  }

  function setActive(id: string) {
    onUpdate(templates, id);
  }

  function addTemplate() {
    if (!newTemplateName.trim()) return;
    const id = `custom_${Date.now()}`;
    const tpl: NamingTemplate = {
      id,
      name: newTemplateName.trim(),
      pattern: '{auteur-nom} {auteur-prenom} - {titre} ({date})',
      is_default: false,
    };
    onUpdate([...templates, tpl], activeId);
    newTemplateName = '';
    startEdit(tpl);
  }

  function insertVariable(v: string) {
    if (editingId) {
      editPattern = editPattern + v;
    }
  }

  function resetToDefaults() {
    const defaults = DEFAULT_CONFIG.templates;
    onUpdate(defaults, 'default');
    editingId = null;
  }
</script>

<div class="template-editor">
  <div style="display:flex; align-items:center; gap:0.75rem; margin-bottom:1rem;">
    <h3>Templates de nommage</h3>
    <button class="btn btn-ghost btn-sm" onclick={resetToDefaults} id="btn-reset-templates">
      Réinitialiser
    </button>
  </div>

  <!-- Liste des templates -->
  <div class="template-list">
    {#each templates as tpl (tpl.id)}
      <div class="template-item {activeId === tpl.id ? 'active' : ''} {editingId === tpl.id ? 'editing' : ''}">

        {#if editingId === tpl.id}
          <!-- Mode édition -->
          <div class="template-edit-form">
            <div class="form-group">
              <label for="tpl-name-{tpl.id}">Nom du template</label>
              <input id="tpl-name-{tpl.id}" type="text" bind:value={editName} />
            </div>
            <div class="form-group">
              <label for="tpl-pattern-{tpl.id}">Pattern</label>
              <input
                id="tpl-pattern-{tpl.id}"
                type="text"
                bind:value={editPattern}
                style="font-family:var(--font-mono); font-size:0.82rem;"
                placeholder="{auteur} - {titre} ({date})"
              />
              <!-- Chips variables -->
              <div class="var-chips" style="margin-top:0.4rem;">
                {#each TEMPLATE_VARIABLES as v}
                  <button
                    class="var-chip"
                    onclick={() => insertVariable(v.name)}
                    title={v.description}
                  >
                    {v.name}
                  </button>
                {/each}
              </div>
            </div>
            <!-- Prévisualisation -->
            <div class="tpl-preview">
              <span style="color:var(--text-muted); font-size:0.72rem;">Aperçu :</span>
              <code style="color:var(--accent-light);">
                {editPattern
                  .replace('{auteur}', 'Blumenberg, Hans')
                  .replace('{titre}', 'La vérité nue')
                  .replace('{date}', '2022')
                  .replace('{editeur}', 'Seuil')
                  .replace('{isbn}', '9782021478006')
                  .replace('{doi}', '')
                  .replace('{extension}', 'epub')
                  .replace(/\[\s*\]/g, '')
                  .replace(/\s{2,}/g, ' ')
                  .trim()}.epub
              </code>
            </div>
            <div style="display:flex; gap:0.5rem;">
              <button class="btn btn-primary btn-sm" onclick={saveEdit} id="btn-save-tpl-{tpl.id}">
                Sauvegarder
              </button>
              <button class="btn btn-ghost btn-sm" onclick={cancelEdit}>
                Annuler
              </button>
            </div>
          </div>

        {:else}
          <!-- Mode affichage -->
          <div class="template-display" onclick={() => setActive(tpl.id)} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && setActive(tpl.id)}>
            <div style="display:flex; align-items:center; gap:0.5rem; flex:1; min-width:0;">
              {#if activeId === tpl.id}
                <span style="color:var(--accent); font-size:0.9rem; flex-shrink:0;">●</span>
              {:else}
                <span style="color:var(--border); font-size:0.9rem; flex-shrink:0;">○</span>
              {/if}
              <div style="min-width:0; flex:1;">
                <div style="font-size:0.85rem; font-weight:500;">{tpl.name}</div>
                <div style="font-family:var(--font-mono); font-size:0.72rem; color:var(--text-muted); overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">
                  {tpl.pattern}
                </div>
              </div>
            </div>
            <div class="template-actions" onclick={(e) => e.stopPropagation()} role="presentation">
              <button class="btn btn-ghost btn-icon btn-sm" onclick={() => startEdit(tpl)} title="Modifier" id="btn-edit-tpl-{tpl.id}">
                ✎
              </button>
              {#if !tpl.is_default && templates.length > 1}
                <button class="btn btn-ghost btn-icon btn-sm" onclick={() => deleteTemplate(tpl.id)} title="Supprimer" style="color:var(--error);">
                  ✕
                </button>
              {/if}
            </div>
          </div>
        {/if}

      </div>
    {/each}
  </div>

  <!-- Ajouter un template -->
  <div class="add-template">
    <input
      type="text"
      bind:value={newTemplateName}
      placeholder="Nom du nouveau template…"
      onkeydown={(e) => e.key === 'Enter' && addTemplate()}
    />
    <button class="btn btn-secondary btn-sm" onclick={addTemplate} id="btn-add-template" disabled={!newTemplateName.trim()}>
      + Ajouter
    </button>
  </div>
</div>

<style>
  .template-editor {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .template-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .template-item {
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    overflow: hidden;
    transition: border-color var(--transition-fast);
  }

  .template-item.active {
    border-color: var(--accent)60;
    background: var(--accent-glow);
  }

  .template-item.editing {
    border-color: var(--accent);
  }

  .template-display {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.65rem 0.75rem;
    cursor: pointer;
    transition: background var(--transition-fast);
    border-radius: var(--radius-md);
  }

  .template-display:hover { background: var(--bg-elevated); }

  .template-edit-form {
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    background: var(--bg-elevated);
  }

  .template-actions {
    display: flex;
    gap: 0.2rem;
    flex-shrink: 0;
  }

  .tpl-preview {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    padding: 0.5rem 0.75rem;
    background: var(--bg-surface);
    border-radius: var(--radius-sm);
    font-size: 0.8rem;
    word-break: break-all;
  }

  .add-template {
    display: flex;
    gap: 0.5rem;
  }

  .add-template input {
    flex: 1;
  }
</style>
