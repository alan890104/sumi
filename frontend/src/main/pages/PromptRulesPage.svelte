<script lang="ts">
  import { onMount } from 'svelte';
  import type { PromptRule } from '$lib/types';
  import { t } from '$lib/stores/i18n.svelte';
  import {
    getCurrentRules,
    setCurrentRules,
    savePolish,
    getPolishConfig,
  } from '$lib/stores/settings.svelte';
  import {
    getExpandedRuleIndex,
    setExpandedRuleIndex,
    toggleRuleExpand,
  } from '$lib/stores/ui.svelte';
  import { getDefaultPrompt, getDefaultPromptRules } from '$lib/api';
  import RuleCard from '../components/RuleCard.svelte';
  import RuleEditorModal from '../components/RuleEditorModal.svelte';

  let basePrompt = $state('');
  let editorVisible = $state(false);
  let editingIndex = $state(-1);

  const rules = $derived(getCurrentRules());
  const expandedIndex = $derived(getExpandedRuleIndex());

  // Build sorted rules with original indices
  const sortedRules = $derived.by(() => {
    const r = rules;
    if (!r || r.length === 0) return [];
    return r
      .map((rule, i) => ({ rule, origIndex: i }))
      .sort((a, b) => {
        const nameA = (a.rule.name || '').toLowerCase();
        const nameB = (b.rule.name || '').toLowerCase();
        return nameA.localeCompare(nameB);
      });
  });

  onMount(async () => {
    await initDefaultRules();
    await loadBasePrompt();
  });

  async function initDefaultRules() {
    const currentRules = getCurrentRules();
    if (!currentRules || currentRules.length === 0) {
      try {
        const defaults = await getDefaultPromptRules();
        if (defaults && defaults.length > 0) {
          setCurrentRules(defaults);
          await savePolish();
        }
      } catch (e) {
        console.error('Failed to load default prompt rules:', e);
      }
    }
  }

  async function loadBasePrompt() {
    try {
      basePrompt = await getDefaultPrompt();
    } catch (e) {
      console.error('Failed to load base prompt:', e);
    }
  }

  function openEditor(index: number) {
    editingIndex = index;
    editorVisible = true;
  }

  function closeEditor() {
    editorVisible = false;
    editingIndex = -1;
  }

  async function handleSave(rule: PromptRule) {
    const currentRules = getCurrentRules().slice();
    if (editingIndex >= 0) {
      currentRules[editingIndex] = rule;
    } else {
      currentRules.push(rule);
    }
    setCurrentRules(currentRules);
    closeEditor();
    await savePolish();
  }

  async function handleDelete(index: number) {
    const currentRules = getCurrentRules().slice();
    currentRules.splice(index, 1);
    setCurrentRules(currentRules);
    // Reset expanded if deleted rule was expanded
    if (expandedIndex === index) {
      setExpandedRuleIndex(-1);
    }
    await savePolish();
  }

  async function handleToggle(index: number) {
    const currentRules = getCurrentRules().slice();
    currentRules[index] = { ...currentRules[index], enabled: !currentRules[index].enabled };
    setCurrentRules(currentRules);
    await savePolish();
  }
</script>

<div class="page">
  <h1 class="page-title">{t('promptRules.title')}</h1>
  <div class="prompt-rules-desc">{t('promptRules.desc')}</div>

  <div class="prompt-rules-header">
    <span></span>
    <button class="add-rule-btn" onclick={() => openEditor(-1)}>{t('settings.polish.addRule')}</button>
  </div>

  <!-- Base Prompt Card -->
  {#if basePrompt}
    <div class="default-prompt-card">
      <div class="prompt-rule-top">
        <span class="prompt-rule-name">
          <span class="default-prompt-badge">{t('promptRules.basePrompt')}</span>
        </span>
      </div>
      <div class="default-prompt-desc">{t('promptRules.basePromptDesc')}</div>
      <div class="prompt-rule-prompt">{basePrompt}</div>
    </div>
  {/if}

  <!-- Rule List -->
  <div class="prompt-rules-list">
    {#if sortedRules.length === 0}
      <div class="prompt-rules-empty">{t('settings.polish.noRules')}</div>
    {:else}
      {#each sortedRules as { rule, origIndex } (origIndex)}
        <RuleCard
          {rule}
          index={origIndex}
          expanded={expandedIndex === origIndex}
          onToggleExpand={() => toggleRuleExpand(origIndex)}
          onEdit={() => openEditor(origIndex)}
          onDelete={() => handleDelete(origIndex)}
          onToggle={() => handleToggle(origIndex)}
        />
      {/each}
    {/if}
  </div>
</div>

<!-- Rule Editor Modal -->
<RuleEditorModal
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

  .prompt-rules-desc {
    font-size: 12px;
    color: var(--text-secondary);
    margin-bottom: 16px;
  }

  .prompt-rules-header {
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

  .default-prompt-card {
    background: var(--bg-sidebar);
    border-radius: var(--radius-md);
    padding: 12px;
    margin-bottom: 12px;
    border: 1px solid rgba(59, 130, 246, 0.2);
  }

  .prompt-rule-top {
    margin-bottom: 4px;
  }

  .prompt-rule-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .default-prompt-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 4px;
    background: rgba(59, 130, 246, 0.1);
    color: var(--accent-blue);
    font-size: 11px;
    font-weight: 600;
  }

  .default-prompt-desc {
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .prompt-rule-prompt {
    font-size: 11px;
    color: var(--text-tertiary);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .prompt-rules-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .prompt-rules-empty {
    font-size: 12px;
    color: var(--text-tertiary);
    text-align: center;
    padding: 20px 12px;
    background: var(--bg-sidebar);
    border-radius: var(--radius-md);
  }
</style>
