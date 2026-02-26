<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { t } from '$lib/stores/i18n.svelte';
  import {
    getPolishConfig,
    setPolishEnabled,
    setPolishMode,
    setPolishModel,
    setPolishReasoning,
    setPolishCloudProvider,
    setPolishCloudApiKey,
    setPolishCloudEndpoint,
    setPolishCloudModelId,
    savePolish,
  } from '$lib/stores/settings.svelte';
  import {
    checkLlmModelStatus,
    downloadLlmModel,
    onLlmModelDownloadProgress,
    saveApiKey,
  } from '$lib/api';
  import type { PolishMode, PolishModel, DownloadProgress } from '$lib/types';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import SettingRow from '$lib/components/SettingRow.svelte';
  import Toggle from '$lib/components/Toggle.svelte';
  import SegmentedControl from '$lib/components/SegmentedControl.svelte';
  import ProgressBar from '$lib/components/ProgressBar.svelte';
  import CloudConfigPanel from '$lib/components/CloudConfigPanel.svelte';

  // ── LLM model status ──

  let llmModelExists = $state(false);
  let isLlmDownloading = $state(false);
  let llmDownloadPercent = $state(0);
  let llmDownloadedBytes = $state(0);
  let llmTotalBytes = $state(0);
  let llmDownloadError = $state(false);
  let llmDownloadingModel = $state<string | null>(null);
  let unlisten: UnlistenFn | null = null;

  let polishConfig = $derived(getPolishConfig());

  let modeOptions = $derived([
    { value: 'local', label: t('settings.polish.modeLocal') },
    { value: 'cloud', label: t('settings.polish.modeCloud') },
  ]);

  let modelOptions = [
    { value: 'llama_taiwan', label: 'Llama 3 Taiwan' },
    { value: 'qwen25', label: 'Qwen 2.5' },
  ];

  let currentModelName = $derived(
    polishConfig.model === 'llama_taiwan' ? 'Llama 3 Taiwan 8B' : 'Qwen 2.5 7B'
  );

  let currentModelSize = $derived('Q4_K_M (~4.6 GB)');

  let downloadBtnLabel = $derived.by(() => {
    if (llmModelExists) return t('settings.polish.downloaded');
    if (isLlmDownloading && llmDownloadingModel === polishConfig.model)
      return t('settings.polish.downloading');
    if (llmDownloadError) return t('settings.polish.retry');
    return t('settings.polish.download');
  });

  let downloadBtnDisabled = $derived(
    llmModelExists || (isLlmDownloading && llmDownloadingModel === polishConfig.model)
  );

  function formatBytes(bytes: number): string {
    const mb = bytes / 1048576;
    return mb.toFixed(0) + ' MB';
  }

  let downloadLabel = $derived(
    isLlmDownloading && llmDownloadingModel === polishConfig.model
      ? Math.round(llmDownloadPercent) + '%'
      : ''
  );

  let downloadSublabel = $derived(
    isLlmDownloading && llmDownloadingModel === polishConfig.model
      ? formatBytes(llmDownloadedBytes) + ' / ' + formatBytes(llmTotalBytes)
      : ''
  );

  let showDownloadProgress = $derived(
    isLlmDownloading && llmDownloadingModel === polishConfig.model
  );

  async function checkStatus() {
    try {
      const status = await checkLlmModelStatus();
      llmModelExists = status.model_exists;
    } catch (e) {
      console.error('Failed to check LLM model status:', e);
    }
  }

  async function startLlmDownload() {
    isLlmDownloading = true;
    llmDownloadError = false;
    llmDownloadPercent = 0;
    llmDownloadedBytes = 0;
    llmTotalBytes = 0;
    llmDownloadingModel = polishConfig.model;

    if (unlisten) {
      unlisten();
      unlisten = null;
    }

    unlisten = await onLlmModelDownloadProgress((d: DownloadProgress) => {
      if (d.status === 'downloading') {
        const pct = d.downloaded && d.total ? (d.downloaded / d.total) * 100 : 0;
        if (polishConfig.model === llmDownloadingModel) {
          llmDownloadPercent = Math.min(pct, 100);
          llmDownloadedBytes = d.downloaded ?? 0;
          llmTotalBytes = d.total ?? 0;
        }
      } else if (d.status === 'complete') {
        const completedModel = llmDownloadingModel;
        llmDownloadingModel = null;
        isLlmDownloading = false;
        if (polishConfig.model === completedModel) {
          llmModelExists = true;
        }
        if (unlisten) { unlisten(); unlisten = null; }
      } else if (d.status === 'error') {
        llmDownloadingModel = null;
        isLlmDownloading = false;
        llmDownloadError = true;
        console.error('LLM download error:', d.message);
        if (unlisten) { unlisten(); unlisten = null; }
      }
    });

    try {
      await downloadLlmModel();
    } catch (e) {
      llmDownloadingModel = null;
      isLlmDownloading = false;
      llmDownloadError = true;
      console.error('Failed to start LLM download:', e);
    }
  }

  // ── Event handlers ──

  function onTogglePolish(checked: boolean) {
    setPolishEnabled(checked);
    savePolish();
  }

  function onToggleReasoning(checked: boolean) {
    setPolishReasoning(checked);
    savePolish();
  }

  function onModeChange(value: string) {
    setPolishMode(value as PolishMode);
    savePolish();
  }

  function onModelChange(value: string) {
    setPolishModel(value as PolishModel);
    savePolish();

    // If switching to the model that's downloading, show progress
    if (llmDownloadingModel && llmDownloadingModel === value) {
      // Progress will be shown automatically via the derived state
    } else {
      // Re-check status for the newly selected model
      checkStatus();
    }
  }

  // ── Cloud config ──
  // Use $state (not $derived) so CloudConfigPanel can write back via bind:
  let cloudProvider = $state(getPolishConfig().cloud.provider);
  let cloudApiKey = $state(getPolishConfig().cloud.api_key);
  let cloudEndpoint = $state(getPolishConfig().cloud.endpoint);
  let cloudModelId = $state(getPolishConfig().cloud.model_id);

  // Sync from store when settings are reloaded externally
  $effect(() => {
    const cfg = getPolishConfig();
    cloudProvider = cfg.cloud.provider;
    cloudApiKey = cfg.cloud.api_key;
    cloudEndpoint = cfg.cloud.endpoint;
    cloudModelId = cfg.cloud.model_id;
  });

  async function onCloudChange() {
    const provider = cloudProvider;
    const apiKey = cloudApiKey;
    setPolishCloudProvider(provider as any);
    setPolishCloudApiKey(apiKey);
    setPolishCloudEndpoint(cloudEndpoint);
    setPolishCloudModelId(cloudModelId);
    try {
      await saveApiKey(provider, apiKey);
    } catch (e) {
      console.error('Failed to save polish API key to keychain:', e);
    }
    await savePolish();
  }

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
        <path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .963 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.963 0z"/>
        <path d="M20 3v4"/>
        <path d="M22 5h-4"/>
      </svg>
    </span>
    <span class="section-title">{t('settings.polish')}</span>
  </div>

  <!-- Polish toggle -->
  <SettingRow name={t('settings.polish.toggle')} desc={t('settings.polish.toggleDesc')}>
    <Toggle checked={polishConfig.enabled} onchange={onTogglePolish} />
  </SettingRow>

  <!-- Sub-settings (visible when enabled) -->
  {#if polishConfig.enabled}
    <div class="sub-settings">
      <!-- Mode selector -->
      <SettingRow name={t('settings.polish.mode')} sub>
        <SegmentedControl
          options={modeOptions}
          value={polishConfig.mode}
          onchange={onModeChange}
        />
      </SettingRow>

      <!-- Reasoning toggle -->
      <SettingRow
        name={t('settings.polish.reasoning')}
        desc={t('settings.polish.reasoningDesc')}
        sub
      >
        <Toggle checked={polishConfig.reasoning} onchange={onToggleReasoning} />
      </SettingRow>

      <!-- Local panel -->
      {#if polishConfig.mode === 'local'}
        <div class="local-panel">
          <!-- Model selector -->
          <SettingRow name={t('settings.polish.model')} sub>
            <SegmentedControl
              options={modelOptions}
              value={polishConfig.model}
              onchange={onModelChange}
            />
          </SettingRow>

          <!-- Model download card -->
          <div class="polish-model-card">
            <div class="polish-model-header">
              <div>
                <div class="polish-model-name">{currentModelName}</div>
                <div class="polish-model-size">{currentModelSize}</div>
              </div>
              <button
                class="polish-download-btn"
                class:downloaded={llmModelExists}
                disabled={downloadBtnDisabled}
                onclick={startLlmDownload}
              >
                {downloadBtnLabel}
              </button>
            </div>
            {#if showDownloadProgress}
              <ProgressBar
                percent={llmDownloadPercent}
                label={downloadLabel}
                sublabel={downloadSublabel}
                shimmer
              />
            {/if}
          </div>
        </div>
      {/if}

      <!-- Cloud panel -->
      {#if polishConfig.mode === 'cloud'}
        <div class="cloud-panel">
          <CloudConfigPanel
            type="polish"
            bind:provider={cloudProvider}
            bind:apiKey={cloudApiKey}
            bind:endpoint={cloudEndpoint}
            bind:modelId={cloudModelId}
            onchange={onCloudChange}
          />
        </div>
      {/if}

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
    gap: 12px;
    margin-top: 12px;
  }

  .local-panel,
  .cloud-panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
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
