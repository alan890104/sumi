<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    icon: Snippet;
  }

  let { title, icon }: Props = $props();
</script>

<div class="section-header">
  <span class="section-icon">
    {@render icon()}
  </span>
  <span class="section-title">{title}</span>
</div>

<style>
  .section-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 16px;
  }

  .section-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    color: var(--text-secondary);
  }

  .section-icon :global(svg) {
    width: 18px;
    height: 18px;
    display: block;
  }

  .section-title {
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    color: var(--text-secondary);
  }
</style>
