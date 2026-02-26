<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { t } from '$lib/stores/i18n.svelte';
  import {
    getSttConfig,
    setSttMode,
    setSttCloudProvider,
    setSttCloudApiKey,
    setSttCloudEndpoint,
    setSttCloudModelId,
    setSttCloudLanguage,
    saveStt,
  } from '$lib/stores/settings.svelte';
  import {
    checkModelStatus,
    downloadModel,
    onModelDownloadProgress,
    saveApiKey,
    getApiKey,
  } from '$lib/api';
  import type { SttMode, DownloadProgress } from '$lib/types';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import SettingRow from '$lib/components/SettingRow.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import CloudConfigPanel from '$lib/components/CloudConfigPanel.svelte';

  // ── STT model status ──

  let modelExists = $state(false);
  let isDownloading = $state(false);
  let downloadPercent = $state(0);
  let downloadedBytes = $state(0);
  let totalBytes = $state(0);
  let downloadError = $state(false);
  let unlisten: UnlistenFn | null = null;

  let sttModeOptions = $derived([
    { value: 'local', label: t('settings.stt.modeLocal') },
    { value: 'cloud', label: t('settings.stt.modeCloud') },
  ]);

  let sttConfig = $derived(getSttConfig());

  let downloadBtnLabel = $derived.by(() => {
    if (modelExists) return t('settings.polish.downloaded');
    if (isDownloading) return t('settings.polish.downloading');
    if (downloadError) return t('settings.polish.retry');
    return t('settings.polish.download');
  });

  let downloadBtnDisabled = $derived(modelExists || isDownloading);

  function formatBytes(bytes: number): string {
    const mb = bytes / 1048576;
    return mb.toFixed(0) + ' MB';
  }

  let downloadLabel = $derived(
    isDownloading ? Math.round(downloadPercent) + '%' : ''
  );

  let downloadSublabel = $derived(
    isDownloading ? formatBytes(downloadedBytes) + ' / ' + formatBytes(totalBytes) : ''
  );

  async function checkStatus() {
    try {
      const status = await checkModelStatus();
      modelExists = status.model_exists;
    } catch (e) {
      console.error('Failed to check STT model status:', e);
    }
  }

  async function startDownload() {
    isDownloading = true;
    downloadError = false;
    downloadPercent = 0;
    downloadedBytes = 0;
    totalBytes = 0;

    if (unlisten) {
      unlisten();
      unlisten = null;
    }

    unlisten = await onModelDownloadProgress((d: DownloadProgress) => {
      if (d.status === 'downloading') {
        const pct = d.downloaded && d.total ? (d.downloaded / d.total) * 100 : 0;
        downloadPercent = Math.min(pct, 100);
        downloadedBytes = d.downloaded ?? 0;
        totalBytes = d.total ?? 0;
      } else if (d.status === 'complete') {
        modelExists = true;
        isDownloading = false;
        if (unlisten) { unlisten(); unlisten = null; }
      } else if (d.status === 'error') {
        isDownloading = false;
        downloadError = true;
        console.error('STT model download error:', d.message);
        if (unlisten) { unlisten(); unlisten = null; }
      }
    });

    try {
      await downloadModel();
    } catch (e) {
      isDownloading = false;
      downloadError = true;
      console.error('Failed to start STT model download:', e);
    }
  }

  // ── Mode change ──

  function onModeChange(value: string) {
    setSttMode(value as SttMode);
    saveStt();
  }

  // ── Cloud config change ──

  async function onCloudChange() {
    // Save API key to keychain
    const provider = getSttConfig().cloud.provider;
    const apiKey = getSttConfig().cloud.api_key;
    try {
      await saveApiKey('stt_' + provider, apiKey);
    } catch (e) {
      console.error('Failed to save STT API key to keychain:', e);
    }
    await saveStt();
  }

  // ── Cloud config bindings ──
  // Use $state (not $derived) so CloudConfigPanel can write back via bind:
  let cloudProvider = $state(getSttConfig().cloud.provider);
  let cloudApiKey = $state(getSttConfig().cloud.api_key);
  let cloudEndpoint = $state(getSttConfig().cloud.endpoint);
  let cloudModelId = $state(getSttConfig().cloud.model_id);
  let cloudLanguage = $state(getSttConfig().cloud.language);

  // Sync from store when settings are reloaded externally
  $effect(() => {
    const cfg = getSttConfig();
    cloudProvider = cfg.cloud.provider;
    cloudApiKey = cfg.cloud.api_key;
    cloudEndpoint = cfg.cloud.endpoint;
    cloudModelId = cfg.cloud.model_id;
    cloudLanguage = cfg.cloud.language;
  });

  onMount(() => {
    checkStatus();
  });

  onDestroy(() => {
    if (unlisten) { unlisten(); unlisten = null; }
  });
</script>

<div class="section">
  <div class="section-header">
    <span class="section-icon">
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/>
        <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
        <line x1="12" x2="12" y1="19" y2="22"/>
      </svg>
    </span>
    <span class="section-title">{t('settings.stt')}</span>
  </div>

  <!-- Mode selector -->
  <SettingRow name={t('settings.stt.mode')} desc={t('settings.stt.modeDesc')}>
    <SegmentedControl
      options={sttModeOptions}
      value={sttConfig.mode}
      onchange={onModeChange}
    />
  </SettingRow>

  <!-- Local panel -->
  {#if sttConfig.mode === 'local'}
    <div class="sub-settings">
      <div class="polish-model-card">
        <div class="polish-model-header">
          <div>
            <div class="polish-model-name">Whisper large-v3-turbo-zh-TW</div>
            <div class="polish-model-size">~1.5 GB</div>
          </div>
          <button
            class="polish-download-btn"
            class:downloaded={modelExists}
            disabled={downloadBtnDisabled}
            onclick={startDownload}
          >
            {downloadBtnLabel}
          </button>
        </div>
        {#if isDownloading}
          <ProgressBar
            percent={downloadPercent}
            label={downloadLabel}
            sublabel={downloadSublabel}
            shimmer
          />
        {/if}
      </div>
    </div>
  {/if}

  <!-- Cloud panel -->
  {#if sttConfig.mode === 'cloud'}
    <div class="sub-settings">
      <CloudConfigPanel
        type="stt"
        bind:provider={cloudProvider}
        bind:apiKey={cloudApiKey}
        bind:endpoint={cloudEndpoint}
        bind:modelId={cloudModelId}
        bind:language={cloudLanguage}
        onchange={async () => {
          setSttCloudProvider(cloudProvider as any);
          setSttCloudApiKey(cloudApiKey);
          setSttCloudEndpoint(cloudEndpoint);
          setSttCloudModelId(cloudModelId);
          setSttCloudLanguage(cloudLanguage);
          await onCloudChange();
        }}
      />
    </div>
  {/if}
</div>

<style>
  .section {
    margin-bottom: 28px;
  }

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

  .sub-settings {
    display: flex;
    flex-direction: column;
    gap: 16px;
    margin-top: 16px;
    padding-left: 24px;
  }

  .polish-model-card {
    padding: 0;
    background: none;
    border-radius: 0;
  }

  .polish-model-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .polish-model-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .polish-model-size {
    font-size: 12px;
    color: var(--text-tertiary);
  }

  .polish-download-btn {
    -webkit-app-region: no-drag;
    app-region: no-drag;
    padding: 6px 14px;
    border: none;
    border-radius: var(--radius-sm);
    background: var(--accent-blue);
    color: #ffffff;
    font-family: 'Inter', sans-serif;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .polish-download-btn:hover {
    background: #0066d6;
  }

  .polish-download-btn.downloaded {
    background: var(--accent-green);
    cursor: default;
  }

  .polish-download-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
