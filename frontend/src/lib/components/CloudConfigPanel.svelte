<script lang="ts">
  import { t } from '$lib/stores/i18n.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { getApiKey } from '$lib/api';
  import {
    CLOUD_PROVIDERS,
    STT_CLOUD_PROVIDERS,
    CLOUD_PROVIDER_LABELS,
    STT_PROVIDER_LABELS,
    STT_LANGUAGES,
  } from '$lib/constants';
  import type { CloudProvider, SttProvider } from '$lib/types';

  let {
    type,
    provider = $bindable(),
    apiKey = $bindable(),
    endpoint = $bindable(),
    modelId = $bindable(),
    language = $bindable(''),
    onchange,
  }: {
    type: 'stt' | 'polish';
    provider: string;
    apiKey: string;
    endpoint: string;
    modelId: string;
    language?: string;
    onchange: () => void;
  } = $props();

  // Derive provider options based on type
  let providerOptions = $derived(
    type === 'stt'
      ? (Object.keys(STT_PROVIDER_LABELS) as SttProvider[]).map((key) => ({
          value: key,
          label: STT_PROVIDER_LABELS[key],
        }))
      : (Object.keys(CLOUD_PROVIDER_LABELS) as CloudProvider[]).map((key) => ({
          value: key,
          label: CLOUD_PROVIDER_LABELS[key],
        }))
  );

  // Derive provider metadata
  let providerMeta = $derived(
    type === 'stt'
      ? STT_CLOUD_PROVIDERS[provider as SttProvider]
      : CLOUD_PROVIDERS[provider as CloudProvider]
  );

  // Derive API key URL
  let apiKeyUrl = $derived(providerMeta?.apiKeyUrl ?? null);

  // STT-specific: whether provider is Azure
  let isAzure = $derived(type === 'stt' && provider === 'azure');

  // Whether to show the endpoint field
  let showEndpoint = $derived(provider === 'custom' || (type === 'stt' && provider === 'azure'));

  // Whether to show endpoint as "Region" (Azure) or "Endpoint"
  let endpointLabel = $derived(isAzure ? t('settings.stt.azureRegion') : t('settings.stt.endpoint'));
  let endpointPlaceholder = $derived(
    isAzure
      ? 'eastus'
      : type === 'stt'
        ? 'https://api.example.com/v1/audio/transcriptions'
        : 'https://api.example.com/v1/chat/completions'
  );

  // Polish-specific: model options from provider
  let polishModels = $derived(
    type === 'polish'
      ? (CLOUD_PROVIDERS[provider as CloudProvider]?.models ?? [])
      : []
  );

  // Whether the currently selected modelId is a known preset
  let isKnownModel = $derived(
    polishModels.length > 0 && polishModels.some((m) => m.id === modelId)
  );

  // Track whether "Custom..." is selected in the model dropdown
  let showCustomModelInput = $derived(
    type === 'polish' &&
    polishModels.length > 0 &&
    !isKnownModel &&
    modelId !== ''
  );

  // For the model select value: if the current modelId is known, use it; otherwise '__custom__'
  let modelSelectValue = $derived(
    isKnownModel ? modelId : (polishModels.length > 0 ? '__custom__' : modelId)
  );

  // STT-specific: model info (single model per STT provider)
  let sttModel = $derived(
    type === 'stt'
      ? (STT_CLOUD_PROVIDERS[provider as SttProvider]?.model ?? null)
      : null
  );

  // Whether to show model row
  let showModelRow = $derived(
    type === 'stt'
      ? sttModel !== null
      : polishModels.length > 0 || provider === 'custom' || provider === 'open_router' || provider === 'open_ai' || provider === 'gemini'
  );

  async function onProviderChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    provider = target.value;

    // Reset endpoint when switching away from custom/azure
    if (provider !== 'custom' && provider !== 'azure') {
      endpoint = '';
    }

    // Reset model to first available for this provider
    if (type === 'polish') {
      const meta = CLOUD_PROVIDERS[provider as CloudProvider];
      if (meta && meta.models.length > 0) {
        modelId = meta.models[0].id;
      } else {
        modelId = '';
      }
    } else {
      const meta = STT_CLOUD_PROVIDERS[provider as SttProvider];
      modelId = meta?.model?.id ?? '';
    }

    // Load API key from keychain for the new provider
    const keychainKey = type === 'stt' ? 'stt_' + provider : provider;
    try {
      apiKey = await getApiKey(keychainKey);
    } catch {
      apiKey = '';
    }

    onchange();
  }

  function onApiKeyInput(e: Event) {
    const target = e.target as HTMLInputElement;
    apiKey = target.value;
    onchange();
  }

  function onEndpointInput(e: Event) {
    const target = e.target as HTMLInputElement;
    endpoint = target.value;
    onchange();
  }

  function onModelSelectChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    if (target.value === '__custom__') {
      modelId = '';
    } else {
      modelId = target.value;
    }
    onchange();
  }

  function onCustomModelInput(e: Event) {
    const target = e.target as HTMLInputElement;
    modelId = target.value;
    onchange();
  }

  function onLanguageChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    language = target.value;
    onchange();
  }

  async function openApiKeyPage() {
    if (apiKeyUrl) {
      try {
        await openUrl(apiKeyUrl);
      } catch {
        window.open(apiKeyUrl, '_blank');
      }
    }
  }
</script>

<div class="cloud-config">
  <!-- Provider -->
  <div class="cloud-row">
    <div class="setting-info">
      <div class="setting-name sub-name">
        {type === 'stt' ? t('settings.stt.provider') : t('settings.polish.provider')}
      </div>
    </div>
    <select class="cloud-select" value={provider} onchange={onProviderChange}>
      {#each providerOptions as opt}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
  </div>

  <!-- API Key -->
  <div class="cloud-row">
    <div class="setting-info">
      <div class="setting-name sub-name">
        {type === 'stt' ? t('settings.stt.apiKey') : t('settings.polish.apiKey')}
      </div>
    </div>
    <div class="api-key-wrap">
      <input
        type="password"
        class="cloud-input"
        value={apiKey}
        placeholder="sk-..."
        oninput={onApiKeyInput}
      />
      {#if apiKeyUrl}
        <button class="provider-link-btn" onclick={openApiKeyPage} title="Get API Key">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>
            <polyline points="15 3 21 3 21 9"/>
            <line x1="10" y1="14" x2="21" y2="3"/>
          </svg>
        </button>
      {/if}
    </div>
  </div>

  <!-- Endpoint / Azure Region (conditional) -->
  {#if showEndpoint}
    <div class="cloud-row">
      <div class="setting-info">
        <div class="setting-name sub-name">{endpointLabel}</div>
      </div>
      <input
        type="text"
        class="cloud-input"
        value={endpoint}
        placeholder={endpointPlaceholder}
        oninput={onEndpointInput}
      />
    </div>
  {/if}

  <!-- Model (STT: single read-only model; Polish: dropdown with Custom option) -->
  {#if type === 'stt' && showModelRow}
    <div class="cloud-row">
      <div class="setting-info">
        <div class="setting-name sub-name">{t('settings.stt.cloudModel')}</div>
      </div>
      <select class="cloud-select" value={sttModel?.id ?? ''} disabled>
        {#if sttModel}
          <option value={sttModel.id}>{sttModel.name}</option>
        {/if}
      </select>
    </div>
  {/if}

  {#if type === 'polish' && polishModels.length > 0}
    <div class="cloud-row">
      <div class="setting-info">
        <div class="setting-name sub-name">{t('settings.polish.cloudModel')}</div>
      </div>
      <select class="cloud-select" value={modelSelectValue} onchange={onModelSelectChange}>
        {#each polishModels as model}
          <option value={model.id}>{model.name}</option>
        {/each}
        <option value="__custom__">Custom...</option>
      </select>
    </div>
  {/if}

  {#if type === 'polish' && (polishModels.length === 0 || showCustomModelInput)}
    <div class="cloud-row">
      <div class="setting-info">
        <div class="setting-name sub-name">{t('settings.polish.modelId')}</div>
      </div>
      <input
        type="text"
        class="cloud-input"
        value={modelId}
        placeholder="google/gemma-3n-e2b-it:free"
        oninput={onCustomModelInput}
      />
    </div>
  {/if}

  <!-- Language (STT only) -->
  {#if type === 'stt'}
    <div class="cloud-row">
      <div class="setting-info">
        <div class="setting-name sub-name">{t('settings.stt.language')}</div>
      </div>
      <select class="cloud-select" value={language} onchange={onLanguageChange}>
        {#each STT_LANGUAGES as lang}
          <option value={lang.value}>{lang.label}</option>
        {/each}
      </select>
    </div>
  {/if}
</div>

<style>
  .cloud-config {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .cloud-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 2px 0 2px 8px;
  }

  .cloud-row .setting-info {
    min-width: 90px;
    flex-shrink: 0;
  }

  .setting-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .sub-name {
    padding-left: 12px;
    font-size: 13px;
  }

  .cloud-row :global(.cloud-input),
  .cloud-row :global(.cloud-select) {
    flex: 1;
    max-width: 280px;
  }

  .cloud-input {
    width: 100%;
    padding: 7px 12px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-family: 'Inter', -apple-system, sans-serif;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    background: var(--bg-primary);
    outline: none;
    transition: border-color 0.15s ease, background-color 0.15s ease;
    flex: 1;
    max-width: 280px;
  }

  .cloud-input:hover {
    border-color: rgba(0, 0, 0, 0.15);
    background-color: var(--bg-sidebar);
  }

  .cloud-input:focus {
    border-color: var(--accent-blue);
    background-color: var(--bg-primary);
  }

  .cloud-input::placeholder {
    color: var(--text-tertiary);
  }

  .cloud-select {
    padding: 7px 28px 7px 12px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-family: 'Inter', -apple-system, sans-serif;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    background: var(--bg-primary);
    outline: none;
    cursor: pointer;
    appearance: none;
    -webkit-appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg width='10' height='6' viewBox='0 0 10 6' fill='none' xmlns='http://www.w3.org/2000/svg'%3E%3Cpath d='M1 1L5 5L9 1' stroke='%236e6e73' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 10px center;
    min-width: 160px;
    flex: 1;
    max-width: 280px;
    transition: border-color 0.15s ease, background-color 0.15s ease;
  }

  .cloud-select:hover {
    border-color: rgba(0, 0, 0, 0.15);
    background-color: var(--bg-sidebar);
  }

  .cloud-select:focus {
    outline: none;
    border-color: var(--accent-blue);
  }

  .cloud-select:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .api-key-wrap {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    max-width: 280px;
  }

  .api-key-wrap .cloud-input {
    max-width: none;
  }

  .provider-link-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    flex-shrink: 0;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--text-secondary);
    cursor: pointer;
    transition: border-color 0.15s ease, color 0.15s ease, background-color 0.15s ease;
  }

  .provider-link-btn:hover {
    border-color: var(--accent-blue);
    color: var(--accent-blue);
    background-color: var(--bg-sidebar);
  }
</style>
