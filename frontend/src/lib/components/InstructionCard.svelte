<script lang="ts">
  import type { Snippet } from 'svelte';

  let { icon, children }: {
    icon: string;
    children?: Snippet;
  } = $props();
</script>

<div class="instruction-card">
  <span class="instruction-icon">{@html icon}</span>
  <span class="instruction-text">
    {#if children}
      {@render children()}
    {/if}
  </span>
</div>

<style>
  .instruction-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 18px;
    background: #f0f7ff;
    border-radius: var(--radius-md);
    margin-bottom: 20px;
  }

  .instruction-icon {
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .instruction-text {
    font-size: 13px;
    color: var(--text-primary);
    line-height: 1.8;
  }

  .instruction-card :global(kbd) {
    min-width: 22px;
    height: 22px;
    padding: 0 5px;
    font-size: 11px;
    vertical-align: middle;
    margin: 0 1px;
  }
</style>
