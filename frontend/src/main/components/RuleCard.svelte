<script lang="ts">
  import type { PromptRule } from '$lib/types';
  import { t } from '$lib/stores/i18n.svelte';
  import { getRuleIcon } from '$lib/constants';

  let {
    rule,
    index,
    expanded,
    onToggleExpand,
    onEdit,
    onDelete,
    onToggle,
  }: {
    rule: PromptRule;
    index: number;
    expanded: boolean;
    onToggleExpand: () => void;
    onEdit: () => void;
    onDelete: () => void;
    onToggle: () => void;
  } = $props();

  const icon = $derived(getRuleIcon(rule));
  const name = $derived(rule.name || t('settings.polish.untitledRule'));
  const matchLabel = $derived(
    rule.match_type === 'app_name'
      ? t('settings.polish.matchAppName')
      : rule.match_type === 'bundle_id'
        ? t('settings.polish.matchBundleId')
        : t('settings.polish.matchUrl')
  );
  const toggleLabel = $derived(
    rule.enabled ? t('settings.polish.disableRule') : t('settings.polish.enableRule')
  );
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="prompt-rule-card" class:disabled={!rule.enabled} class:expanded={expanded}>
  <div class="prompt-rule-header" onclick={onToggleExpand}>
    <span class="prompt-rule-icon">{@html icon}</span>
    <span class="prompt-rule-header-name">{name}</span>
    <span class="prompt-rule-chevron">
      <svg width="10" height="10" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5l10 7-10 7z"/></svg>
    </span>
  </div>
  <div class="prompt-rule-body">
    <div class="prompt-rule-body-inner">
      <div class="prompt-rule-match">{matchLabel}: {rule.match_value}</div>
      <div class="prompt-rule-prompt"><strong>{t('settings.polish.rulePrompt')}:</strong> {rule.prompt || ''}</div>
      <div class="prompt-rule-actions">
        <button onclick={(e: MouseEvent) => { e.stopPropagation(); onToggle(); }}>{toggleLabel}</button>
        <button class="icon-btn" onclick={(e: MouseEvent) => { e.stopPropagation(); onEdit(); }} title={t('settings.polish.editRule')}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
          </svg>
        </button>
        <button class="icon-btn" onclick={(e: MouseEvent) => { e.stopPropagation(); onDelete(); }} title={t('settings.polish.deleteRule')}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .prompt-rule-card {
    background: var(--bg-sidebar);
    border-radius: var(--radius-md);
    transition: opacity 0.15s ease;
  }

  .prompt-rule-card.disabled .prompt-rule-header-name {
    opacity: 0.5;
  }

  .prompt-rule-header {
    display: flex;
    cursor: pointer;
    align-items: center;
    padding: 10px 12px;
    gap: 8px;
    border-radius: var(--radius-md);
    transition: background 0.15s ease;
  }

  .prompt-rule-header:hover {
    background: var(--bg-hover);
  }

  .prompt-rule-icon {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .prompt-rule-icon :global(svg) {
    width: 14px;
    height: 14px;
  }

  .prompt-rule-header-name {
    flex: 1;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .prompt-rule-chevron {
    flex-shrink: 0;
    color: var(--text-tertiary);
    transition: transform 0.2s ease;
    display: flex;
    align-items: center;
  }

  .prompt-rule-card.expanded .prompt-rule-chevron {
    transform: rotate(90deg);
  }

  .prompt-rule-body {
    max-height: 0;
    overflow: hidden;
    transition: max-height 0.2s ease;
  }

  .prompt-rule-card.expanded .prompt-rule-body {
    max-height: 500px;
  }

  .prompt-rule-body-inner {
    padding: 0 12px 12px;
  }

  .prompt-rule-match {
    font-size: 11px;
    color: var(--text-secondary);
    margin-bottom: 4px;
  }

  .prompt-rule-prompt {
    font-size: 11px;
    color: var(--text-tertiary);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .prompt-rule-actions {
    display: flex;
    gap: 4px;
    margin-top: 8px;
  }

  .prompt-rule-actions button {
    -webkit-app-region: no-drag;
    app-region: no-drag;
    padding: 4px 8px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-tertiary);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .prompt-rule-actions button:hover {
    background: var(--bg-hover);
    color: var(--text-secondary);
  }

  .prompt-rule-actions button.icon-btn {
    padding: 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
</style>
