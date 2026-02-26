<script lang="ts">
  import { hotkeyToParts } from '$lib/constants';

  let { hotkey, size = 'normal' }: {
    hotkey: string;
    size?: 'normal' | 'large';
  } = $props();

  let parts = $derived(hotkeyToParts(hotkey));
</script>

<div class="keycaps" class:large={size === 'large'}>
  {#each parts as part, i}
    <kbd class:accent={i === parts.length - 1}>{part}</kbd>
  {/each}
</div>

<style>
  .keycaps {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 34px;
    height: 34px;
    padding: 0 10px;
    font-family: 'Inter', -apple-system, sans-serif;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    background: #fff;
    border: 1px solid rgba(0, 0, 0, 0.15);
    border-bottom: 2px solid rgba(0, 0, 0, 0.15);
    border-radius: var(--radius-sm);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.06);
  }

  kbd.accent {
    background: var(--bg-sidebar);
    border-color: rgba(0, 0, 0, 0.12);
    border-bottom-color: rgba(0, 0, 0, 0.18);
    color: var(--accent-blue);
    font-weight: 700;
  }

  .large kbd {
    min-width: 56px;
    height: 56px;
    font-size: 22px;
  }
</style>
