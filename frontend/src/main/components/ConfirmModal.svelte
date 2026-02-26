<script lang="ts">
  import { t } from '$lib/stores/i18n.svelte';
  import { getConfirm, hideConfirm } from '$lib/stores/ui.svelte';

  function handleOk() {
    const c = getConfirm();
    if (c.onConfirm) c.onConfirm();
    hideConfirm();
  }
</script>

{#if getConfirm().visible}
  <div class="confirm-overlay">
    <div class="confirm-backdrop" onclick={hideConfirm} role="presentation"></div>
    <div class="confirm-card">
      <div class="confirm-title">{getConfirm().title}</div>
      <div class="confirm-message">{getConfirm().message}</div>
      <div class="confirm-actions">
        <button class="confirm-cancel" onclick={hideConfirm}>{t('confirm.cancel')}</button>
        <button class="confirm-destructive" onclick={handleOk}>{getConfirm().okLabel}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .confirm-overlay {
    position: fixed;
    inset: 0;
    z-index: 2000;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .confirm-backdrop {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.25);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
  }

  .confirm-card {
    position: relative;
    width: 320px;
    background: var(--bg-primary);
    border-radius: var(--radius-lg);
    padding: 24px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12), 0 0 0 1px var(--border-subtle);
    animation: confirmFadeIn 0.15s ease;
  }

  @keyframes confirmFadeIn {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .confirm-title {
    font-size: 15px;
    font-weight: 600;
    margin-bottom: 8px;
  }

  .confirm-message {
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin-bottom: 20px;
  }

  .confirm-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .confirm-actions button {
    padding: 7px 16px;
    border-radius: var(--radius-sm);
    font-family: 'Inter', sans-serif;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    border: 1px solid var(--border-subtle);
    transition: all 0.15s ease;
  }

  .confirm-cancel {
    background: var(--bg-primary);
    color: var(--text-secondary);
  }

  .confirm-cancel:hover {
    background: var(--bg-sidebar);
    color: var(--text-primary);
  }

  .confirm-destructive {
    background: #ff3b30;
    color: #fff;
    border-color: #ff3b30;
  }

  .confirm-destructive:hover {
    background: #e0352b;
    border-color: #e0352b;
  }
</style>
