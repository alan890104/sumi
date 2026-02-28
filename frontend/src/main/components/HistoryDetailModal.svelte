<script lang="ts">
  import { t } from '$lib/stores/i18n.svelte';
  import { showConfirm } from '$lib/stores/ui.svelte';
  import { exportHistoryAudio, deleteHistoryEntry } from '$lib/api';
  import { iconUri } from '$lib/stores/iconCache.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import type { HistoryEntry } from '$lib/types';

  let {
    visible,
    entry,
    onclose,
    ondelete,
  }: {
    visible: boolean;
    entry: HistoryEntry | null;
    onclose: () => void;
    ondelete?: (id: string) => void;
  } = $props();

  let exporting = $state(false);
  let exportDone = $state(false);
  let appIconUri = $derived(entry?.bundle_id ? iconUri(entry.bundle_id) : undefined);

  let formattedTime = $derived.by(() => {
    if (!entry) return '';
    const d = new Date(entry.timestamp);
    return d.toLocaleString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    });
  });

  async function handleDownloadAudio() {
    if (!entry || exporting) return;
    exporting = true;
    try {
      await exportHistoryAudio(entry.id);
      exportDone = true;
      setTimeout(() => { exportDone = false; }, 2000);
    } catch (e) {
      console.error('Failed to export audio:', e);
    } finally {
      exporting = false;
    }
  }

  function handleDelete() {
    if (!entry) return;
    const id = entry.id;
    showConfirm(
      t('history.delete'),
      t('history.clearAllConfirm'),
      t('history.delete'),
      async () => {
        try {
          await deleteHistoryEntry(id);
          ondelete?.(id);
          onclose();
        } catch (e) {
          console.error('Failed to delete history entry:', e);
        }
      },
    );
  }
</script>

<Modal {visible} {onclose} width="440px">
  {#if entry}
    <!-- Header -->
    <div class="hd-header">
      <h2 class="hd-title">{t('history.detailTitle')}</h2>
      <button class="hd-close" onclick={onclose} aria-label="Close">
        <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
          <path d="M1 1l12 12M13 1L1 13"/>
        </svg>
      </button>
    </div>

    <!-- Unified comparison card -->
    <div class="hd-card">
      <div class="hd-cell">
        <span class="hd-tag">{t('history.before')}</span>
        <p class="hd-text">{entry.raw_text}</p>
      </div>
      <div class="hd-divider"></div>
      <div class="hd-cell">
        <span class="hd-tag polished">{t('history.after')}</span>
        <p class="hd-text">{entry.text}</p>
      </div>
    </div>

    <!-- Meta -->
    <div class="hd-meta">
      <div class="hd-meta-row">
        <span class="hd-meta-label">{t('history.metaDuration')}</span>
        <span class="hd-meta-value">{entry.duration_secs.toFixed(1)}s</span>
      </div>
      <div class="hd-meta-row">
        <span class="hd-meta-label">{t('history.metaStt')}</span>
        <span class="hd-meta-value">{entry.stt_model}</span>
      </div>
      {#if entry.polish_model !== 'None'}
        <div class="hd-meta-row">
          <span class="hd-meta-label">{t('history.metaPolish')}</span>
          <span class="hd-meta-value">{entry.polish_model}</span>
        </div>
      {/if}
      <div class="hd-meta-row">
        <span class="hd-meta-label">{t('history.metaTime')}</span>
        <span class="hd-meta-value">{formattedTime}</span>
      </div>
      {#if entry.app_name}
        <div class="hd-meta-row">
          <span class="hd-meta-label">{t('history.metaApp')}</span>
          <span class="hd-meta-value hd-meta-app">
            {#if appIconUri}
              <img class="hd-app-icon" src={appIconUri} alt="" width="16" height="16" />
            {/if}
            {entry.app_name}
          </span>
        </div>
      {/if}
      {#if entry.chars_per_sec > 0}
        <div class="hd-meta-row">
          <span class="hd-meta-label">{t('history.metaCharsPerSec')}</span>
          <span class="hd-meta-value">{entry.chars_per_sec.toFixed(1)} chars/s</span>
        </div>
      {/if}
    </div>

    <!-- Actions -->
    <div class="hd-actions">
      {#if entry.has_audio}
        <button
          class="hd-btn"
          class:success={exportDone}
          onclick={handleDownloadAudio}
          disabled={exporting || exportDone}
        >
          {#if exportDone}
            <svg class="hd-btn-icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            {t('history.exportDone')}
          {:else if exporting}
            <span class="hd-btn-spinner"></span>
            {t('history.exporting')}
          {:else}
            {t('history.downloadAudio')}
          {/if}
        </button>
      {/if}
      <button class="hd-btn danger" onclick={handleDelete}>
        {t('history.delete')}
      </button>
    </div>
  {/if}
</Modal>

<style>
  /* ── Header ── */
  .hd-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 16px;
  }

  .hd-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .hd-close {
    width: 26px;
    height: 26px;
    border: none;
    background: var(--bg-hover);
    border-radius: 50%;
    color: var(--text-tertiary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .hd-close:hover {
    background: var(--bg-active);
    color: var(--text-primary);
  }

  /* ── Comparison card ── */
  .hd-card {
    border: 1px solid var(--border-divider);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .hd-cell {
    padding: 12px 14px;
  }

  .hd-divider {
    height: 1px;
    background: var(--border-divider);
  }

  .hd-tag {
    display: inline-block;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.4px;
    color: var(--text-tertiary);
    background: var(--bg-sidebar);
    padding: 2px 6px;
    border-radius: 4px;
    margin-bottom: 8px;
  }

  .hd-tag.polished {
    color: var(--accent-blue);
    background: rgba(0, 122, 255, 0.07);
  }

  .hd-text {
    font-size: 14px;
    color: var(--text-primary);
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
    -webkit-user-select: text;
    user-select: text;
  }

  /* ── Meta ── */
  .hd-meta {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px 16px;
    margin-top: 14px;
    padding: 10px 14px;
    background: var(--bg-sidebar);
    border-radius: var(--radius-md);
  }

  .hd-meta-row {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .hd-meta-label {
    font-size: 10px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--text-tertiary);
  }

  .hd-meta-value {
    font-size: 12px;
    color: var(--text-primary);
    font-weight: 500;
  }

  .hd-meta-app {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .hd-app-icon {
    border-radius: 3px;
    flex-shrink: 0;
  }

  /* ── Actions ── */
  .hd-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }

  .hd-btn {
    -webkit-app-region: no-drag;
    app-region: no-drag;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    font-family: 'Inter', sans-serif;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
    border: none;
    background: var(--bg-sidebar);
    color: var(--text-secondary);
  }

  .hd-btn:hover {
    background: var(--bg-active);
    color: var(--text-primary);
  }

  .hd-btn:disabled {
    cursor: default;
  }

  .hd-btn.success {
    background: rgba(52, 199, 89, 0.1);
    color: var(--accent-green);
  }

  .hd-btn-icon {
    flex-shrink: 0;
  }

  .hd-btn-spinner {
    width: 12px;
    height: 12px;
    border: 1.5px solid var(--border-divider);
    border-top-color: var(--text-secondary);
    border-radius: 50%;
    animation: hd-spin 0.6s linear infinite;
    flex-shrink: 0;
  }

  @keyframes hd-spin {
    to { transform: rotate(360deg); }
  }

  .hd-btn.danger {
    color: #ff3b30;
  }

  .hd-btn.danger:hover {
    background: rgba(255, 59, 48, 0.08);
    color: #ff3b30;
  }
</style>
