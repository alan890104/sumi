<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '$lib/stores/i18n.svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';

  type UpdateState = 'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'error' | 'upToDate';

  let version = $state('');
  let updateState = $state<UpdateState>('idle');
  let updateVersion = $state('');
  let downloadPercent = $state(0);
  let pendingUpdate = $state<Awaited<ReturnType<typeof check>> | null>(null);

  onMount(async () => {
    try {
      version = await getVersion();
    } catch {
      version = '0.0.0';
    }
  });

  async function checkForUpdates() {
    updateState = 'checking';
    try {
      const update = await check();
      if (update) {
        pendingUpdate = update;
        updateVersion = update.version;
        updateState = 'available';
      } else {
        updateState = 'upToDate';
      }
    } catch (e) {
      console.error('Update check failed:', e);
      updateState = 'error';
    }
  }

  async function downloadAndInstall() {
    if (!pendingUpdate) return;
    updateState = 'downloading';
    downloadPercent = 0;

    try {
      let downloaded = 0;
      let contentLength = 0;

      await pendingUpdate.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength || 0;
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              downloadPercent = Math.min(100, Math.round((downloaded / contentLength) * 100));
            }
            break;
          case 'Finished':
            break;
        }
      });

      updateState = 'ready';
    } catch (e) {
      console.error('Update download failed:', e);
      updateState = 'error';
    }
  }

  async function handleRestart() {
    await relaunch();
  }

  function handleButtonClick() {
    switch (updateState) {
      case 'idle':
      case 'upToDate':
        checkForUpdates();
        break;
      case 'available':
        downloadAndInstall();
        break;
      case 'ready':
        handleRestart();
        break;
      case 'error':
        checkForUpdates();
        break;
    }
  }

  let buttonText = $derived.by(() => {
    switch (updateState) {
      case 'idle':
        return t('about.checkUpdate');
      case 'checking':
        return t('about.checking');
      case 'available':
        return t('about.download');
      case 'downloading':
        return t('about.downloading', { percent: String(downloadPercent) });
      case 'ready':
        return t('about.restartNow');
      case 'error':
        return t('about.retry');
      case 'upToDate':
        return t('about.checkUpdate');
      default:
        return t('about.checkUpdate');
    }
  });

  let buttonDisabled = $derived(updateState === 'checking' || updateState === 'downloading');
  let buttonPrimary = $derived(updateState === 'available' || updateState === 'ready');
  let showButton = $derived(updateState !== 'downloading');
</script>

<div class="page">
  <h1 class="page-title">{t('about.title')}</h1>

  <div class="about-content">
    <div class="about-icon">
      <img src="icon.png" alt="Sumi" />
    </div>
    <div class="about-name">Sumi</div>
    <div class="about-version">{t('about.version', { version })}</div>
    <div class="about-desc">{t('about.desc')}</div>

    <div class="about-update">
      {#if showButton}
        <button
          class="about-update-btn"
          class:primary={buttonPrimary}
          disabled={buttonDisabled}
          onclick={handleButtonClick}
        >
          {buttonText}
        </button>
      {/if}

      {#if updateState === 'checking'}
        <div class="about-update-status">{t('about.checking')}</div>
      {:else if updateState === 'available'}
        <div class="about-update-status">
          {t('about.updateAvailable', { version: updateVersion })}
        </div>
      {:else if updateState === 'downloading'}
        <div class="about-update-status">
          {t('about.downloading', { percent: String(downloadPercent) })}
        </div>
        <div class="about-update-progress">
          <div class="about-update-progress-bar" style="width: {downloadPercent}%"></div>
        </div>
      {:else if updateState === 'ready'}
        <div class="about-update-status">{t('about.readyToRestart')}</div>
        <div class="about-update-note">{t('about.gatekeeperNote')}</div>
      {:else if updateState === 'error'}
        <div class="about-update-status error">{t('about.updateError')}</div>
      {:else if updateState === 'upToDate'}
        <div class="about-update-status">{t('about.upToDate')}</div>
      {/if}
    </div>
  </div>
</div>

<style>
  .page-title {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.3px;
    color: var(--text-primary);
    margin-bottom: 24px;
  }

  .about-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding-top: 40px;
    gap: 6px;
  }

  .about-icon {
    width: 72px;
    height: 72px;
    border-radius: 18px;
    overflow: hidden;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
    margin-bottom: 12px;
  }

  .about-icon img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .about-name {
    font-size: 20px;
    font-weight: 700;
    letter-spacing: -0.3px;
    color: var(--text-primary);
  }

  .about-version {
    font-size: 13px;
    color: var(--text-tertiary);
    margin-bottom: 12px;
  }

  .about-desc {
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.5;
    max-width: 320px;
  }

  .about-update {
    margin-top: 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }

  .about-update-btn {
    -webkit-app-region: no-drag;
    app-region: no-drag;
    padding: 7px 18px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    background: var(--bg-primary);
    color: var(--accent-blue);
    font-family: 'Inter', sans-serif;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .about-update-btn:hover {
    background: var(--bg-hover);
  }

  .about-update-btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .about-update-btn.primary {
    background: var(--accent-blue);
    color: #ffffff;
    border-color: var(--accent-blue);
  }

  .about-update-btn.primary:hover {
    filter: brightness(1.1);
  }

  .about-update-status {
    font-size: 13px;
    color: var(--text-secondary);
  }

  .about-update-status.error {
    color: #ff3b30;
  }

  .about-update-progress {
    width: 200px;
    height: 4px;
    border-radius: 2px;
    background: var(--bg-active);
    overflow: hidden;
  }

  .about-update-progress-bar {
    height: 100%;
    background: var(--accent-blue);
    border-radius: 2px;
    transition: width 0.2s ease;
    width: 0%;
  }

  .about-update-note {
    font-size: 11px;
    color: var(--text-tertiary);
    max-width: 300px;
    text-align: center;
    line-height: 1.4;
  }
</style>
