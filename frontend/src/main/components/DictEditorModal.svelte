<script lang="ts">
  import type { DictionaryEntry } from '$lib/types';
  import { t } from '$lib/stores/i18n.svelte';
  import { getDictionary } from '$lib/stores/settings.svelte';

  let {
    visible,
    editIndex,
    onclose,
    onsave,
  }: {
    visible: boolean;
    editIndex: number;
    onclose: () => void;
    onsave: (entry: DictionaryEntry) => void;
  } = $props();

  let term = $state('');
  let termInput: HTMLInputElement | undefined = $state();

  const title = $derived(
    editIndex >= 0 ? t('dictionary.editEntry') : t('dictionary.addEntry')
  );

  // Populate form when modal opens
  $effect(() => {
    if (visible) {
      if (editIndex >= 0) {
        const dict = getDictionary();
        const entry = dict.entries[editIndex];
        if (entry) {
          term = entry.term || '';
        }
      } else {
        term = '';
      }
    }
  });

  function handleSave() {
    if (!term.trim()) {
      termInput?.focus();
      return;
    }

    const entry: DictionaryEntry = {
      term: term.trim(),
      enabled: true,
    };

    // Preserve enabled state when editing
    if (editIndex >= 0) {
      const dict = getDictionary();
      if (dict.entries[editIndex]) {
        entry.enabled = dict.entries[editIndex].enabled;
      }
    }

    onsave(entry);
  }

  function handleBackdropClick() {
    onclose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onclose();
    }
  }
</script>

<svelte:window onkeydown={visible ? handleKeydown : undefined} />

{#if visible}
  <div class="rule-editor-overlay">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="rule-editor-backdrop" onclick={handleBackdropClick}></div>
    <div class="rule-editor-card">
      <div class="rule-editor-title">{title}</div>

      <div class="rule-editor-field">
        <div class="rule-editor-label">{t('dictionary.term')}</div>
        <input
          type="text"
          class="rule-editor-input"
          bind:this={termInput}
          bind:value={term}
          placeholder={t('dictionary.termPlaceholder')}
        />
      </div>

      <div class="rule-editor-actions">
        <button class="rule-editor-cancel" onclick={onclose}>{t('dictionary.cancel')}</button>
        <button class="rule-editor-save" onclick={handleSave}>{t('dictionary.save')}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .rule-editor-overlay {
    position: fixed;
    inset: 0;
    z-index: 2000;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .rule-editor-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.25);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }

  .rule-editor-card {
    position: relative;
    width: 420px;
    max-height: 80vh;
    overflow-y: auto;
    background: var(--bg-primary);
    border-radius: var(--radius-lg);
    padding: 24px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12), 0 0 0 1px var(--border-subtle);
    animation: dictEditorFadeIn 0.15s ease;
  }

  @keyframes dictEditorFadeIn {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .rule-editor-title {
    font-size: 15px;
    font-weight: 600;
    margin-bottom: 16px;
  }

  .rule-editor-field {
    margin-bottom: 14px;
  }

  .rule-editor-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .rule-editor-input {
    width: 100%;
    padding: 8px 10px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-family: 'Inter', sans-serif;
    font-size: 13px;
    outline: none;
    transition: border-color 0.15s ease;
    box-sizing: border-box;
  }

  .rule-editor-input:focus {
    border-color: var(--accent-blue);
  }

  .rule-editor-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 18px;
  }

  .rule-editor-actions button {
    -webkit-app-region: no-drag;
    app-region: no-drag;
    padding: 7px 18px;
    border-radius: var(--radius-sm);
    font-family: 'Inter', sans-serif;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    border: none;
  }

  .rule-editor-cancel {
    background: var(--bg-hover);
    color: var(--text-secondary);
  }

  .rule-editor-cancel:hover {
    background: var(--bg-active);
    color: var(--text-primary);
  }

  .rule-editor-save {
    background: var(--accent-blue);
    color: #fff;
  }

  .rule-editor-save:hover {
    filter: brightness(1.1);
  }
</style>
