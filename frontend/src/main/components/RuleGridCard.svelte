<script lang="ts">
  import type { PromptRule } from '$lib/types';
  import { t } from '$lib/stores/i18n.svelte';
  import { getRuleIcon } from '$lib/constants';

  let {
    rule,
    index,
    onEdit,
    onDelete,
    onToggle,
  }: {
    rule: PromptRule;
    index: number;
    onEdit: () => void;
    onDelete: () => void;
    onToggle: () => void;
  } = $props();

  const icon = $derived(getRuleIcon(rule));
  const name = $derived(rule.name || t('settings.polish.untitledRule'));
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="grid-tile"
  class:disabled={!rule.enabled}
  onclick={onEdit}
>
  <!-- Hover actions -->
  <div class="tile-actions">
    <button
      class="tile-action-btn"
      onclick={(e: MouseEvent) => { e.stopPropagation(); onToggle(); }}
    >
      {rule.enabled ? t('settings.polish.disableRule') : t('settings.polish.enableRule')}
    </button>
    <button
      class="tile-action-btn tile-action-delete"
      onclick={(e: MouseEvent) => { e.stopPropagation(); onDelete(); }}
    >
      {t('settings.polish.deleteRule')}
    </button>
  </div>

  <span class="grid-tile-icon">{@html icon}</span>
  <span class="grid-tile-name">{name}</span>
</div>

<style>
  .grid-tile {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 14px 8px;
    border-radius: var(--radius-md);
    background: var(--bg-sidebar);
    cursor: pointer;
    transition: all 0.15s ease;
    border: 2px solid transparent;
    min-height: 90px;
  }

  .grid-tile:hover {
    background: var(--bg-hover);
    border-color: var(--accent-blue);
  }

  .grid-tile.disabled {
    opacity: 0.4;
    filter: grayscale(0.6);
  }

  .grid-tile.disabled:hover {
    opacity: 0.7;
  }

  .grid-tile-icon {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .grid-tile-icon :global(svg) {
    width: 32px;
    height: 32px;
  }

  .grid-tile-name {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-primary);
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
    line-height: 1.3;
  }

  /* ── Hover action buttons ── */
  .tile-actions {
    position: absolute;
    top: 4px;
    left: 4px;
    right: 4px;
    display: flex;
    justify-content: center;
    gap: 4px;
    opacity: 0;
    transition: opacity 0.12s ease;
    pointer-events: none;
  }

  .grid-tile:hover .tile-actions {
    opacity: 1;
    pointer-events: auto;
  }

  .tile-action-btn {
    -webkit-app-region: no-drag;
    app-region: no-drag;
    padding: 2px 6px;
    border: none;
    border-radius: 4px;
    background: rgba(0, 0, 0, 0.55);
    color: #fff;
    font-family: 'Inter', sans-serif;
    font-size: 10px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.12s ease;
    white-space: nowrap;
    line-height: 1.4;
  }

  .tile-action-btn:hover {
    background: rgba(0, 0, 0, 0.75);
  }

  .tile-action-delete:hover {
    background: rgba(255, 59, 48, 0.85);
  }
</style>
