<script lang="ts">
  import type { Snippet } from 'svelte';

  let { visible, onclose, width, children }: {
    visible: boolean;
    onclose: () => void;
    width?: string;
    children?: Snippet;
  } = $props();

  function handleBackdropClick() {
    onclose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onclose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if visible}
  <div class="modal-overlay">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal-backdrop" onclick={handleBackdropClick}></div>
    <div class="modal-card" style:width>
      {#if children}
        {@render children()}
      {/if}
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    z-index: 2000;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.25);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }

  .modal-card {
    position: relative;
    width: 320px;
    background: var(--bg-primary);
    border-radius: var(--radius-lg);
    padding: 24px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12), 0 0 0 1px var(--border-subtle);
    animation: modalFadeIn 0.15s ease;
  }

  @keyframes modalFadeIn {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
