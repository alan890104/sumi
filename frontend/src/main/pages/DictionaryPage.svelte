<script lang="ts">
  import type { DictionaryEntry } from '$lib/types';
  import { t } from '$lib/stores/i18n.svelte';
  import {
    getDictionary,
    setDictionaryEnabled,
    savePolish,
  } from '$lib/stores/settings.svelte';
  import Toggle from '$lib/components/Toggle.svelte';
  import DictEditorModal from '../components/DictEditorModal.svelte';

  let editorVisible = $state(false);
  let editingIndex = $state(-1);

  const dictionary = $derived(getDictionary());
  const entries = $derived(dictionary.entries);
  const dictEnabled = $derived(dictionary.enabled);

  function openEditor(index: number) {
    editingIndex = index;
    editorVisible = true;
  }

  function closeEditor() {
    editorVisible = false;
    editingIndex = -1;
  }

  async function handleToggleDict(checked: boolean) {
    setDictionaryEnabled(checked);
    await savePolish();
  }

  async function handleSave(entry: DictionaryEntry) {
    const dict = getDictionary();
    if (editingIndex >= 0) {
      dict.entries[editingIndex] = entry;
    } else {
      dict.entries.push(entry);
    }
    closeEditor();
    await savePolish();
  }

  async function handleDelete(index: number) {
    const dict = getDictionary();
    dict.entries.splice(index, 1);
    await savePolish();
  }

  async function handleToggleEntry(index: number) {
    const dict = getDictionary();
    dict.entries[index] = { ...dict.entries[index], enabled: !dict.entries[index].enabled };
    await savePolish();
  }
</script>

<div class="page">
  <h1 class="page-title">{t('dictionary.title')}</h1>
  <div class="dictionary-desc">{t('dictionary.desc')}</div>

  <div class="dictionary-toggle-row">
    <div>
      <div class="dictionary-toggle-label">{t('dictionary.toggle')}</div>
      <div class="dictionary-toggle-desc">{t('dictionary.toggleDesc')}</div>
    </div>
    <Toggle checked={dictEnabled} onchange={handleToggleDict} />
  </div>

  <div class="dictionary-header">
    <span></span>
    <button class="add-rule-btn" onclick={() => openEditor(-1)}>{t('dictionary.addEntry')}</button>
  </div>

  <div class="dictionary-list">
    {#if !entries || entries.length === 0}
      <div class="dictionary-empty">
        <div>{t('dictionary.emptyTitle')}</div>
        <div class="dictionary-empty-hint">{t('dictionary.emptyHint')}</div>
      </div>
    {:else}
      {#each entries as entry, i (i)}
        <div class="dictionary-card" class:disabled={!entry.enabled}>
          <div class="dictionary-card-top">
            <span class="dictionary-card-terms">{entry.term}</span>
            <div class="dictionary-card-actions">
              <button onclick={() => handleToggleEntry(i)}>
                {entry.enabled ? t('dictionary.disable') : t('dictionary.enable')}
              </button>
              <button class="icon-btn" onclick={() => openEditor(i)} title="Edit">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
                </svg>
              </button>
              <button class="icon-btn danger" onclick={() => handleDelete(i)} title="Delete">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
                </svg>
              </button>
            </div>
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

<!-- Dictionary Editor Modal -->
<DictEditorModal
  visible={editorVisible}
  editIndex={editingIndex}
  onclose={closeEditor}
  onsave={handleSave}
/>

<style>
  .page-title {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.3px;
    color: var(--text-primary);
    margin-bottom: 24px;
  }

  .dictionary-desc {
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 16px;
  }

  .dictionary-toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px;
    background: var(--bg-sidebar);
    border-radius: var(--radius-md);
    margin-bottom: 16px;
  }

  .dictionary-toggle-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .dictionary-toggle-desc {
    font-size: 11px;
    color: var(--text-secondary);
    margin-top: 2px;
  }

  .dictionary-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .add-rule-btn {
    -webkit-app-region: no-drag;
    app-region: no-drag;
    padding: 4px 12px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--accent-blue);
    font-family: 'Inter', sans-serif;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .add-rule-btn:hover {
    background: var(--bg-sidebar);
    border-color: rgba(0, 0, 0, 0.15);
  }

  .dictionary-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .dictionary-empty {
    font-size: 12px;
    color: var(--text-tertiary);
    text-align: center;
    padding: 20px 12px;
    background: var(--bg-sidebar);
    border-radius: var(--radius-md);
  }

  .dictionary-empty-hint {
    font-size: 11px;
    color: var(--text-tertiary);
    margin-top: 4px;
  }

  .dictionary-card {
    background: var(--bg-sidebar);
    border-radius: var(--radius-md);
    padding: 12px;
    transition: opacity 0.15s ease;
  }

  .dictionary-card.disabled {
    opacity: 0.5;
  }

  .dictionary-card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .dictionary-card-terms {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .dictionary-card-actions {
    display: flex;
    gap: 4px;
  }

  .dictionary-card-actions button {
    -webkit-app-region: no-drag;
    app-region: no-drag;
    padding: 4px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
    transition: all 0.15s ease;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .dictionary-card-actions button:hover {
    background: var(--bg-hover);
    color: var(--text-secondary);
  }

  .dictionary-card-actions button.danger:hover {
    color: #ff3b30;
    background: rgba(255, 59, 48, 0.06);
  }
</style>
