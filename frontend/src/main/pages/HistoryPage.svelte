<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '$lib/stores/i18n.svelte';
  import { showConfirm } from '$lib/stores/ui.svelte';
  import * as settingsStore from '$lib/stores/settings.svelte';
  import {
    getHistory,
    getHistoryStoragePath,
    clearAllHistory,
    exportHistoryAudio,
    deleteHistoryEntry,
    getAppIcon,
  } from '$lib/api';
  import { RETENTION_OPTIONS } from '$lib/constants';
  import Select from '$lib/components/Select.svelte';
  import HistoryDetailModal from '../components/HistoryDetailModal.svelte';
  import type { HistoryEntry } from '$lib/types';

  let entries = $state<HistoryEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let storagePath = $state('-');

  // Detail modal
  let detailVisible = $state(false);
  let detailEntry = $state<HistoryEntry | null>(null);

  // App icon cache: bundle_id → data URI
  let iconCache = $state<Record<string, string>>({});

  async function resolveIcons(items: HistoryEntry[]) {
    const bundleIds = [...new Set(items.map((e) => e.bundle_id).filter(Boolean))];
    const missing = bundleIds.filter((bid) => !iconCache[bid]);
    if (missing.length === 0) return;

    const results = await Promise.allSettled(
      missing.map(async (bid) => {
        const uri = await getAppIcon(bid);
        return { bid, uri };
      })
    );

    const newCache = { ...iconCache };
    let updated = false;
    for (const r of results) {
      if (r.status === 'fulfilled') {
        newCache[r.value.bid] = r.value.uri;
        updated = true;
      }
    }
    if (updated) {
      iconCache = newCache; // single reactive update
    }
  }

  $effect(() => {
    if (entries.length > 0) {
      resolveIcons(entries);
    }
  });

  // Context menu
  let openMenuId = $state<string | null>(null);

  // Retention
  let retentionDays = $derived(settingsStore.getSettings().history_retention_days);

  // Grouped entries
  interface DateGroup {
    label: string;
    entries: HistoryEntry[];
  }

  let groups = $derived.by(() => {
    if (!entries || entries.length === 0) return [];

    const result: DateGroup[] = [];
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);

    const groupMap = new Map<string, HistoryEntry[]>();
    const labelOrder: string[] = [];

    for (const entry of entries) {
      const d = new Date(entry.timestamp);
      const day = new Date(d);
      day.setHours(0, 0, 0, 0);

      let label: string;
      if (day.getTime() === today.getTime()) {
        label = t('history.today');
      } else if (day.getTime() === yesterday.getTime()) {
        label = t('history.yesterday');
      } else {
        label = d.toLocaleDateString(undefined, {
          year: 'numeric',
          month: '2-digit',
          day: '2-digit',
        });
      }

      if (!groupMap.has(label)) {
        groupMap.set(label, []);
        labelOrder.push(label);
      }
      groupMap.get(label)!.push(entry);
    }

    for (const label of labelOrder) {
      result.push({ label, entries: groupMap.get(label)! });
    }

    return result;
  });

  // Retention select options
  let retentionOptions = $derived(
    RETENTION_OPTIONS.map((opt) => ({
      value: String(opt.value),
      label: t(opt.labelKey),
    })),
  );

  onMount(async () => {
    await loadHistory();
    await loadStoragePath();
  });

  async function loadHistory() {
    loading = true;
    error = null;
    try {
      entries = await getHistory();
    } catch (e) {
      console.error('Failed to load history:', e);
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function loadStoragePath() {
    try {
      storagePath = (await getHistoryStoragePath()) || '-';
    } catch (e) {
      console.error('Failed to load storage path:', e);
    }
  }

  async function handleRetentionChange(value: string) {
    const days = parseInt(value, 10);
    settingsStore.setHistoryRetention(days);
    await settingsStore.save();
  }

  function revealStoragePath() {
    if (storagePath && storagePath !== '-') {
      navigator.clipboard.writeText(storagePath).catch(() => {});
      // Brief flash
      const el = document.querySelector('.history-storage-path') as HTMLElement | null;
      if (el) {
        el.style.borderColor = 'var(--accent-green)';
        setTimeout(() => {
          el.style.borderColor = '';
        }, 800);
      }
    }
  }

  function handleClearAll() {
    showConfirm(t('history.clearAll'), t('history.clearAllConfirm'), t('history.clearAll'), async () => {
      try {
        await clearAllHistory();
        await loadHistory();
      } catch (e) {
        console.error('Failed to clear history:', e);
      }
    });
  }

  function openDetail(entry: HistoryEntry) {
    detailEntry = entry;
    detailVisible = true;
  }

  function closeDetail() {
    detailVisible = false;
    detailEntry = null;
  }

  function handleDetailDelete(_id: string) {
    // Reload history after deletion
    loadHistory();
  }

  function formatTime(timestamp: number): string {
    const d = new Date(timestamp);
    return d.toLocaleTimeString(undefined, {
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    });
  }

  function toggleMenu(event: MouseEvent, id: string) {
    event.stopPropagation();
    if (openMenuId === id) {
      openMenuId = null;
    } else {
      openMenuId = id;
    }
  }

  function closeMenus() {
    openMenuId = null;
  }

  async function handleExportAudio(event: MouseEvent, id: string) {
    event.stopPropagation();
    openMenuId = null;
    try {
      await exportHistoryAudio(id);
    } catch (e) {
      console.error('Failed to export audio:', e);
    }
  }

  async function handleDeleteEntry(event: MouseEvent, id: string) {
    event.stopPropagation();
    openMenuId = null;
    try {
      await deleteHistoryEntry(id);
      await loadHistory();
    } catch (e) {
      console.error('Failed to delete history entry:', e);
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="page" onclick={closeMenus}>
  <h1 class="page-title">{t('history.title')}</h1>

  <!-- Privacy card -->
  <div class="history-privacy-card">
    <span class="history-privacy-icon">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>
      </svg>
    </span>
    <span class="history-privacy-text">{t('history.privacyNote')}</span>
  </div>

  <!-- Settings -->
  <div class="history-settings">
    <div class="history-setting-row">
      <div>
        <div class="history-setting-label">{t('history.retention')}</div>
        <div class="history-setting-desc">{t('history.retentionDesc')}</div>
      </div>
      <Select options={retentionOptions} value={String(retentionDays)} onchange={handleRetentionChange} />
    </div>
    <div class="history-setting-row">
      <div>
        <div class="history-setting-label">{t('history.storageLoc')}</div>
        <div class="history-setting-desc">{t('history.storageLocDesc')}</div>
      </div>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="history-storage-path"
        title="Click to copy path"
        onclick={revealStoragePath}
      >
        {storagePath}
      </div>
    </div>
    <div class="history-setting-row">
      <div>
        <div class="history-setting-label">{t('history.clearAll')}</div>
        <div class="history-setting-desc">{t('history.clearAllDesc')}</div>
      </div>
      <button class="reset-btn" onclick={handleClearAll}>{t('history.clearAll')}</button>
    </div>
  </div>

  <div class="history-divider"></div>

  <!-- History list -->
  {#if loading}
    <div class="history-empty">
      <div class="history-empty-icon">
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
        </svg>
      </div>
      <div class="history-empty-text">Loading...</div>
    </div>
  {:else if error}
    <div class="history-empty">
      <div class="history-empty-icon">
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
        </svg>
      </div>
      <div class="history-empty-text">{t('history.errorTitle')}</div>
      <div class="history-empty-hint">{error}</div>
    </div>
  {:else if entries.length === 0}
    <div class="history-empty">
      <div class="history-empty-icon">
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" y1="19" x2="12" y2="22"/>
        </svg>
      </div>
      <div class="history-empty-text">{t('history.emptyTitle')}</div>
      <div class="history-empty-hint">{t('history.emptyHint')}</div>
    </div>
  {:else}
    {#each groups as group}
      <div class="history-date-group">
        <div class="history-date-header">{group.label}</div>
        {#each group.entries as item (item.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="history-entry clickable"
            onclick={() => openDetail(item)}
          >
            <div class="history-time">{formatTime(item.timestamp)}</div>
            <div class="history-content">
              <div class="history-text">{item.text}</div>
              <div class="history-meta">
                {#if item.app_name}
                  <span class="history-meta-item">
                    {#if iconCache[item.bundle_id]}
                      <img class="history-app-icon" src={iconCache[item.bundle_id]} alt="" width="14" height="14" />
                    {/if}
                    {item.app_name}
                  </span>
                {/if}
                <span class="history-meta-item">{item.duration_secs.toFixed(1)}s</span>
                <span class="history-meta-item">STT: {item.stt_model}</span>
                {#if item.polish_model !== 'None'}
                  <span class="history-meta-item">Polish: {item.polish_model}</span>
                {/if}
              </div>
            </div>
            <button
              class="history-menu-btn"
              onclick={(e) => toggleMenu(e, item.id)}
              title="More"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                <circle cx="12" cy="5" r="2"/><circle cx="12" cy="12" r="2"/><circle cx="12" cy="19" r="2"/>
              </svg>
            </button>
            {#if openMenuId === item.id}
              <div class="history-menu visible">
                {#if item.has_audio}
                  <button class="history-menu-item" onclick={(e) => handleExportAudio(e, item.id)}>
                    {t('history.downloadAudio')}
                  </button>
                {/if}
                <button
                  class="history-menu-item destructive"
                  onclick={(e) => handleDeleteEntry(e, item.id)}
                >
                  {t('history.delete')}
                </button>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/each}
  {/if}
</div>

<!-- Detail modal -->
<HistoryDetailModal
  visible={detailVisible}
  entry={detailEntry}
  onclose={closeDetail}
  ondelete={handleDetailDelete}
/>

<style>
  .page-title {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.3px;
    color: var(--text-primary);
    margin-bottom: 24px;
  }

  /* ── Privacy card ── */
  .history-privacy-card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    background: rgba(52, 199, 89, 0.06);
    border: 1px solid rgba(52, 199, 89, 0.15);
    border-radius: var(--radius-md);
    margin-bottom: 24px;
  }

  .history-privacy-icon {
    flex-shrink: 0;
    color: var(--accent-green);
    display: flex;
    align-items: center;
  }

  .history-privacy-text {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  /* ── Settings section ── */
  .history-settings {
    margin-bottom: 24px;
    padding: 16px;
    background: var(--bg-sidebar);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .history-setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .history-setting-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .history-setting-desc {
    font-size: 11px;
    color: var(--text-tertiary);
    margin-top: 2px;
  }

  .history-storage-path {
    font-size: 11px;
    color: var(--text-tertiary);
    word-break: break-all;
    font-family: 'SF Mono', 'Menlo', monospace;
    background: var(--bg-primary);
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-divider);
    cursor: pointer;
    flex-shrink: 1;
    min-width: 0;
    transition: border-color 0.15s ease;
  }

  .history-storage-path:hover {
    border-color: rgba(0, 0, 0, 0.15);
  }

  .reset-btn {
    -webkit-app-region: no-drag;
    app-region: no-drag;
    padding: 6px 14px;
    border: 1px solid rgba(255, 59, 48, 0.3);
    border-radius: var(--radius-sm);
    background: rgba(255, 59, 48, 0.06);
    color: #ff3b30;
    font-family: 'Inter', sans-serif;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .reset-btn:hover {
    background: rgba(255, 59, 48, 0.12);
    border-color: rgba(255, 59, 48, 0.4);
  }

  .history-divider {
    height: 1px;
    background: var(--border-divider);
    margin-bottom: 20px;
  }

  /* ── Date groups ── */
  .history-date-group {
    margin-bottom: 20px;
  }

  .history-date-header {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-tertiary);
    margin-bottom: 8px;
    padding-left: 4px;
  }

  /* ── Entries ── */
  .history-entry {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 12px;
    border-radius: var(--radius-md);
    transition: background 0.15s ease;
    position: relative;
  }

  .history-entry:hover {
    background: var(--bg-hover);
  }

  .history-entry.clickable {
    cursor: pointer;
  }

  .history-time {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-tertiary);
    min-width: 44px;
    flex-shrink: 0;
    padding-top: 1px;
    font-variant-numeric: tabular-nums;
  }

  .history-content {
    flex: 1;
    min-width: 0;
  }

  .history-text {
    font-size: 14px;
    color: var(--text-primary);
    line-height: 1.45;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .history-meta {
    font-size: 11px;
    color: var(--text-tertiary);
    margin-top: 4px;
    display: flex;
    gap: 8px;
  }

  .history-meta-item {
    display: flex;
    align-items: center;
    gap: 3px;
  }

  .history-app-icon {
    border-radius: 3px;
    flex-shrink: 0;
  }

  /* ── Context menu ── */
  .history-menu-btn {
    -webkit-app-region: no-drag;
    app-region: no-drag;
    background: none;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 4px 6px;
    border-radius: var(--radius-sm);
    opacity: 0;
    transition: all 0.15s ease;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .history-entry:hover .history-menu-btn {
    opacity: 1;
  }

  .history-menu-btn:hover {
    background: var(--bg-active);
    color: var(--text-primary);
  }

  .history-menu {
    display: none;
    position: absolute;
    right: 12px;
    top: 40px;
    background: var(--bg-primary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
    z-index: 100;
    min-width: 140px;
    padding: 4px;
  }

  .history-menu.visible {
    display: block;
  }

  .history-menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    font-size: 13px;
    color: var(--text-primary);
    cursor: pointer;
    border-radius: var(--radius-sm);
    border: none;
    background: none;
    width: 100%;
    text-align: left;
    font-family: 'Inter', sans-serif;
    transition: background 0.1s ease;
  }

  .history-menu-item:hover {
    background: var(--bg-hover);
  }

  .history-menu-item.destructive {
    color: #ff3b30;
  }

  .history-menu-item.destructive:hover {
    background: rgba(255, 59, 48, 0.08);
  }

  /* ── Empty / Error state ── */
  .history-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    color: var(--text-tertiary);
    text-align: center;
  }

  .history-empty-icon {
    margin-bottom: 12px;
    color: var(--text-tertiary);
    opacity: 0.5;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .history-empty-text {
    font-size: 14px;
    font-weight: 500;
    margin-bottom: 4px;
    color: var(--text-secondary);
  }

  .history-empty-hint {
    font-size: 12px;
  }
</style>
