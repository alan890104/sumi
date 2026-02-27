<script lang="ts">
  import type { Snippet } from 'svelte';

  let { name, desc, sub, children }: {
    name: string;
    desc?: string;
    sub?: boolean;
    children?: Snippet;
  } = $props();
</script>

<div class="setting-row" class:sub-row={sub}>
  <div class="setting-info">
    <div class="setting-name">{name}</div>
    {#if desc}
      <div class="setting-desc">{desc}</div>
    {/if}
  </div>
  {#if children}
    {@render children()}
  {/if}
</div>

<style>
  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 0;
    gap: 16px;
  }

  .setting-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .setting-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .setting-desc {
    font-size: 13px;
    color: var(--text-secondary);
    margin-top: 2px;
  }

  .sub-row {
    padding: 2px 0 2px 24px;
  }
</style>
