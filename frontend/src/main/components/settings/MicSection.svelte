<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { t } from '$lib/stores/i18n.svelte';
  import SectionHeader from '$lib/components/SectionHeader.svelte';
  import { getMicStatus } from '$lib/api';
  import type { MicStatus } from '$lib/types';
  import SettingRow from '$lib/components/SettingRow.svelte';
  import Select from '$lib/components/Select.svelte';

  const POLL_INTERVAL = 3000;

  let micStatus = $state<MicStatus | null>(null);
  let selectedDevice = $state('auto');
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  let deviceName = $derived.by(() => {
    if (!micStatus) return 'Detecting...';
    if (!micStatus.connected) return t('settings.mic.noMic');
    return micStatus.default_device ?? 'Unknown';
  });

  let deviceDesc = $derived.by(() => {
    if (!micStatus) return '';
    if (!micStatus.connected) return t('settings.mic.noMicDesc');
    const n = micStatus.devices.length;
    const s = n === 1 ? '' : 's';
    return t('settings.mic.devicesAvailable', { n: String(n), s });
  });

  let isConnected = $derived(micStatus?.connected ?? false);

  let deviceOptions = $derived.by(() => {
    const opts = [{ value: 'auto', label: t('settings.mic.auto') }];
    if (micStatus) {
      for (const device of micStatus.devices) {
        opts.push({ value: device, label: device });
      }
    }
    return opts;
  });

  async function loadMicStatus() {
    try {
      micStatus = await getMicStatus();
    } catch (e) {
      console.error('Failed to get mic status:', e);
    }
  }

  function onDeviceChange(value: string) {
    selectedDevice = value;
    // Device selection is informational in this app --
    // the backend always uses the default input device
  }

  function startPolling() {
    stopPolling();
    loadMicStatus();
    pollTimer = setInterval(loadMicStatus, POLL_INTERVAL);
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  function handleVisibility() {
    if (document.hidden) {
      stopPolling();
    } else {
      startPolling();
    }
  }

  onMount(() => {
    startPolling();
    document.addEventListener('visibilitychange', handleVisibility);
  });

  onDestroy(() => {
    stopPolling();
    document.removeEventListener('visibilitychange', handleVisibility);
  });
</script>

<div class="section">
  <SectionHeader title={t('settings.mic')}>
    {#snippet icon()}
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <rect x="9" y="2" width="6" height="11" rx="3"/>
        <path d="M5 10a7 7 0 0 0 14 0"/>
        <line x1="12" y1="19" x2="12" y2="22"/>
        <line x1="8" y1="22" x2="16" y2="22"/>
      </svg>
    {/snippet}
  </SectionHeader>
  <div class="setting-row">
    <div class="setting-info">
      <div class="setting-name" class:disconnected={!isConnected}>{deviceName}</div>
      {#if deviceDesc}
        <div class="setting-desc">{deviceDesc}</div>
      {/if}
    </div>
    <div class="mic-status">
      <span class="mic-dot" class:disconnected={!isConnected}></span>
      <Select
        options={deviceOptions}
        value={selectedDevice}
        onchange={onDeviceChange}
      />
    </div>
  </div>
</div>

<style>
  .section {
    margin-bottom: 32px;
  }


  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 0;
    gap: 16px;
  }

  .setting-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .setting-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .setting-name.disconnected {
    color: #ff3b30;
  }

  .setting-desc {
    font-size: 12px;
    color: var(--text-secondary);
    margin-top: 2px;
  }

  .mic-status {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .mic-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent-green);
    flex-shrink: 0;
  }

  .mic-dot.disconnected {
    background: #ff3b30;
  }
</style>
